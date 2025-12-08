// Package ingestion は、ファイルをCuberシステムに取り込むタスクを提供します。
// このタスクは、ファイルのメタデータを計算し、KuzuDBに保存します。
package ingestion

import (
	"context"
	"crypto/sha256"
	"encoding/hex"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"time"

	"mycute/pkg/cuber/pipeline"
	"mycute/pkg/cuber/storage"
	"mycute/pkg/s3client"

	"github.com/google/uuid"
)

// IngestTask は、ファイル取り込みタスクを表します。
// このタスクは、ファイルのハッシュを計算し、重複チェックを行い、
// メタデータをKuzuDBに保存します。
type IngestTask struct {
	vectorStorage storage.VectorStorage // ベクトルストレージ（KuzuDB）
	groupID       string                // グループID（パーティション識別子）
	s3Client      *s3client.S3Client    // S3クライアント
}

// NewIngestTask は、新しいIngestTaskを作成します。
// 引数:
//   - vectorStorage: ベクトルストレージ
//   - groupID: グループID（"user-dataset"形式）
//   - s3Client: S3クライアント
//
// 返り値:
//   - *IngestTask: 新しいIngestTaskインスタンス
func NewIngestTask(vectorStorage storage.VectorStorage, groupID string, s3Client *s3client.S3Client) *IngestTask {
	return &IngestTask{
		vectorStorage: vectorStorage,
		groupID:       groupID,
		s3Client:      s3Client,
	}
}

// インターフェース実装の確認
var _ pipeline.Task = (*IngestTask)(nil)

// generateDeterministicID は、コンテンツハッシュとグループIDから決定論的なIDを生成します。
// この関数により、同じファイルを再度取り込んでも同じIDが生成されます。
//
// 引数:
//   - contentHash: ファイルのコンテンツハッシュ（SHA-256）
//   - groupID: グループID
//
// 返り値:
//   - string: 決定論的に生成されたUUID
func generateDeterministicID(contentHash string, groupID string) string {
	// Cuber Ingestion用の名前空間UUID
	namespace := uuid.NameSpaceOID
	// コンテンツハッシュとグループIDを結合してUUIDを生成
	return uuid.NewSHA1(namespace, []byte(contentHash+groupID)).String()
}

// Run は、ファイル取り込みタスクを実行します。
// この関数は以下の処理を行います：
//  1. 各ファイルのハッシュを計算
//  2. 重複チェック（既に取り込まれているかを確認）
//  3. ファイルをストレージ（ローカル/S3）に保存
//  4. メタデータを作成
//  5. KuzuDBに保存
//
// 引数:
//   - ctx: コンテキスト
//   - input: ファイルパスのリスト（[]string）
//
// 返り値:
//   - any: 取り込まれたデータのリスト（[]*storage.Data）
//   - error: エラーが発生した場合
func (t *IngestTask) Run(ctx context.Context, input any) (any, error) {
	// 入力の型チェック
	filePaths, ok := input.([]string)
	if !ok {
		return nil, fmt.Errorf("expected []string input, got %T", input)
	}

	var dataList []*storage.Data

	// 各ファイルを処理
	for _, path := range filePaths {
		// ========================================
		// 1. ハッシュとメタデータの計算
		// ========================================
		hash, err := calculateFileHash(path)
		if err != nil {
			return nil, fmt.Errorf("failed to calculate hash for %s: %w", path, err)
		}

		// ========================================
		// 2. 重複チェック
		// ========================================
		// 決定論的IDにより、ON CONFLICTで重複が処理されますが、
		// 再処理をスキップするために存在チェックを行います
		if t.vectorStorage.Exists(ctx, hash, t.groupID) {
			fmt.Printf("Skipping duplicate file: %s (hash: %s)\n", path, hash)
			// 既存データのIDを決定論的に再生成
			id := generateDeterministicID(hash, t.groupID)
			// 最小限のデータオブジェクトを作成して返す
			data := &storage.Data{ID: id, GroupID: t.groupID, ContentHash: hash, Name: filepath.Base(path)}
			dataList = append(dataList, data)
			continue
		}

		// ファイルの存在確認
		_, err = os.Stat(path)
		if err != nil {
			return nil, fmt.Errorf("failed to stat file %s: %w", path, err)
		}

		// ========================================
		// 3. ファイルのアップロード/保存
		// ========================================
		// S3Client.Up は、ローカルモードなら指定ディレクトリにコピー、S3モードならアップロードを行い、
		// 保存先のキー（相対パス）を返します。
		storageKey, err := t.s3Client.Up(path)
		if err != nil {
			return nil, fmt.Errorf("failed to upload file %s: %w", path, err)
		}

		// ========================================
		// 4. データオブジェクトの作成
		// ========================================
		// 決定論的IDを生成
		dataID := generateDeterministicID(hash, t.groupID)

		data := &storage.Data{
			ID:              dataID,
			GroupID:         t.groupID, // パーティションID
			Name:            filepath.Base(path),
			Extension:       filepath.Ext(path),
			ContentHash:     hash,
			RawDataLocation: *storageKey, // 保存された場所のキーを記録
			CreatedAt:       time.Now(),
		}

		// ========================================
		// 5. KuzuDBに保存
		// ========================================
		if err := t.vectorStorage.SaveData(ctx, data); err != nil {
			return nil, fmt.Errorf("failed to save data %s: %w", data.Name, err)
		}

		dataList = append(dataList, data)
		fmt.Printf("Ingested file: %s (id: %s)\n", data.Name, data.ID)
	}

	return dataList, nil
}

// calculateFileHash は、ファイルのSHA-256ハッシュを計算します。
// このハッシュは、ファイルの重複チェックと決定論的ID生成に使用されます。
//
// 引数:
//   - filePath: ハッシュを計算するファイルのパス
//
// 返り値:
//   - string: SHA-256ハッシュの16進数文字列
//   - error: ファイルの読み込みに失敗した場合
func calculateFileHash(filePath string) (string, error) {
	// ファイルを開く
	file, err := os.Open(filePath)
	if err != nil {
		return "", err
	}
	defer file.Close()

	// SHA-256ハッシュを計算
	hash := sha256.New()
	if _, err := io.Copy(hash, file); err != nil {
		return "", err
	}

	// ハッシュを16進数文字列に変換
	return hex.EncodeToString(hash.Sum(nil)), nil
}
