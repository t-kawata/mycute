// Package ingestion は、ファイルをCuberシステムに取り込むタスクを提供します。
// このタスクは、ファイルのメタデータを計算し、LadybugDBに保存します。
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

	"go.uber.org/zap"

	"github.com/t-kawata/mycute/pkg/cuber/pipeline"
	"github.com/t-kawata/mycute/pkg/cuber/storage"
	"github.com/t-kawata/mycute/pkg/cuber/types"
	"github.com/t-kawata/mycute/pkg/cuber/utils"
	"github.com/t-kawata/mycute/pkg/s3client"

	"github.com/google/uuid"
	"github.com/t-kawata/mycute/lib/eventbus"
	"github.com/t-kawata/mycute/pkg/cuber/event"
)

// IngestTask は、ファイル取り込みタスクを表します。
// このタスクは、ファイルのハッシュを計算し、重複チェックを行い、
// メタデータをLadybugDBに保存します。
type IngestTask struct {
	vectorStorage storage.VectorStorage // ベクトルストレージ（LadybugDB）
	memoryGroup   string                // メモリーグループ（パーティション識別子）
	s3Client      *s3client.S3Client    // S3クライアント
	Logger        *zap.Logger
	EventBus      *eventbus.EventBus
}

// NewIngestTask は、新しいIngestTaskを作成します。
// 引数:
//   - vectorStorage: ベクトルストレージ
//   - memoryGroup: メモリーグループ（"user-dataset"形式）
//   - s3Client: S3クライアント
//
// 返り値:
//   - *IngestTask: 新しいIngestTaskインスタンス
func NewIngestTask(vectorStorage storage.VectorStorage, memoryGroup string, s3Client *s3client.S3Client, l *zap.Logger, eb *eventbus.EventBus) *IngestTask {
	return &IngestTask{
		vectorStorage: vectorStorage,
		memoryGroup:   memoryGroup,
		s3Client:      s3Client,
		Logger:        l,
		EventBus:      eb,
	}
}

// インターフェース実装の確認
var _ pipeline.Task = (*IngestTask)(nil)

// generateDeterministicID は、コンテンツハッシュとメモリーグループから決定論的なIDを生成します。
// この関数により、同じファイルを再度取り込んでも同じIDが生成されます。
//
// 引数:
//   - contentHash: ファイルのコンテンツハッシュ（SHA-256）
//   - memoryGroup: メモリーグループ
//
// 返り値:
//   - string: 決定論的に生成されたUUID
func generateDeterministicID(contentHash string, memoryGroup string) string {
	// Cuber Ingestion用の名前空間UUID
	namespace := uuid.NameSpaceOID
	// コンテンツハッシュとメモリーグループを結合してUUIDを生成
	return uuid.NewSHA1(namespace, []byte(contentHash+memoryGroup)).String()
}

// Run は、ファイル取り込みタスクを実行します。
// この関数は以下の処理を行います：
//  1. 各ファイルのハッシュを計算
//  2. 重複チェック（既に取り込まれているかを確認）
//  3. ファイルをストレージ（ローカル/S3）に保存
//  4. メタデータを作成
//  5. LadybugDBに保存
//
// 引数:
//   - ctx: コンテキスト
//   - input: ファイルパスのリスト（[]string）
//
// 返り値:
//   - any: 取り込まれたデータのリスト（[]*storage.Data）
//   - types.TokenUsage: トークン使用量（Ingestでは0）
//   - error: エラーが発生した場合
func (t *IngestTask) Run(ctx context.Context, input any) (any, types.TokenUsage, error) {
	var usage types.TokenUsage // Empty
	// 入力の型チェック
	filePaths, ok := input.([]string)
	if !ok {
		return nil, usage, fmt.Errorf("Ingest: Expected []string input, got %T", input)
	}
	fileCount := len(filePaths)
	utils.LogInfo(t.Logger, "IngestTask: Starting ingestion", zap.Int("file_count", fileCount), zap.String("group", t.memoryGroup))
	skippedCount := 0
	var dataList []*storage.Data
	// 各ファイルを処理
	for _, path := range filePaths {
		baseName := filepath.Base(path)
		// Emit Add File Start
		eventbus.Emit(t.EventBus, string(event.EVENT_ABSORB_ADD_FILE_START), event.AbsorbAddFileStartPayload{
			BasePayload: event.NewBasePayload(t.memoryGroup), // CubeID unavailable in task context, left empty or passed? Assuming empty is fine as BasePayload handles timestamp.
			FileName:    baseName,
		})

		// ========================================
		// 1. ハッシュとメタデータの計算
		// ========================================
		hash, err := calculateFileHash(path)
		if err != nil {
			return nil, usage, fmt.Errorf("Ingest: Failed to calculate hash for %s: %w", path, err)
		}

		var fileSize int64
		fileInfo, statErr := os.Stat(path)
		if statErr == nil {
			fileSize = fileInfo.Size()
		}

		utils.LogDebug(t.Logger, "IngestTask: Calculated hash", zap.String("path", path), zap.String("hash", hash))
		// ========================================
		// 2. 重複チェック
		// ========================================
		// 決定論的IDにより、ON CONFLICTで重複が処理されますが、
		// 再処理をスキップするために存在チェックを行います
		if t.vectorStorage.Exists(ctx, hash, t.memoryGroup) {
			skippedCount++
			utils.LogDebug(t.Logger, "IngestTask: Skipping duplicate file", zap.String("path", path), zap.String("hash", hash))
			// 既存データのIDを決定論的に再生成
			id := generateDeterministicID(hash, t.memoryGroup)
			// 最小限のデータオブジェクトを作成して返す
			data := &storage.Data{ID: id, MemoryGroup: t.memoryGroup, ContentHash: hash, Name: filepath.Base(path)}
			dataList = append(dataList, data)

			// Emit Add File End (Skipped)
			eventbus.Emit(t.EventBus, string(event.EVENT_ABSORB_ADD_FILE_END), event.AbsorbAddFileEndPayload{
				BasePayload: event.NewBasePayload(t.memoryGroup),
				FileName:    baseName,
				Size:        fileSize,
			})
			continue
		}
		// ファイルの存在確認 (Already checked above for size, but strict check here)
		if statErr != nil {
			return nil, usage, fmt.Errorf("Ingest: Failed to stat file %s: %w", path, statErr)
		}

		utils.LogDebug(t.Logger, "IngestTask: Processing file", zap.String("path", path), zap.Int64("size", fileInfo.Size()))
		// ========================================
		// 3. ファイルのアップロード/保存
		// ========================================
		// S3Client.Up は、ローカルモードなら指定ディレクトリにコピー、S3モードならアップロードを行い、
		// 保存先のキー（相対パス）を返します。
		storageKey, err := t.s3Client.Up(path)
		if err != nil {
			return nil, usage, fmt.Errorf("Ingest: Failed to upload file %s: %w", path, err)
		}
		utils.LogDebug(t.Logger, "IngestTask: Uploaded file", zap.String("key", *storageKey))
		// ========================================
		// 4. データオブジェクトの作成
		// ========================================
		// 決定論的IDを生成
		dataID := generateDeterministicID(hash, t.memoryGroup)
		data := &storage.Data{
			ID:              dataID,
			MemoryGroup:     t.memoryGroup, // パーティションID
			Name:            filepath.Base(path),
			Extension:       filepath.Ext(path),
			ContentHash:     hash,
			RawDataLocation: *storageKey, // 保存された場所のキーを記録
			CreatedAt:       time.Now(),
		}
		// ========================================
		// 5. LadybugDBに保存
		// ========================================
		if err := t.vectorStorage.SaveData(ctx, data); err != nil {
			return nil, usage, fmt.Errorf("Ingest: Failed to save data %s: %w", data.Name, err)
		}
		dataList = append(dataList, data)
		utils.LogDebug(t.Logger, "IngestTask: Ingested file", zap.String("name", data.Name), zap.String("id", data.ID))

		// Emit Add File End
		eventbus.Emit(t.EventBus, string(event.EVENT_ABSORB_ADD_FILE_END), event.AbsorbAddFileEndPayload{
			BasePayload: event.NewBasePayload(t.memoryGroup),
			FileName:    baseName,
			Size:        fileSize,
		})
	}
	if fileCount == skippedCount { // 全件スキップされた場合は、全て重複データであるとしてエラーで返す
		return nil, usage, fmt.Errorf("Ingest: All data or files are duplicates.")
	}
	return dataList, usage, nil
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
