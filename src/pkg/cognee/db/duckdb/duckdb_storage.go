// Package duckdb は、DuckDBを使用したベクトルストレージの実装を提供します。
// DuckDBは、OLAP（分析処理）に特化したSQLデータベースで、VSS拡張により
// ベクトル検索機能を利用できます。
package duckdb

import (
	"context"
	"database/sql"
	"encoding/json"
	"fmt"

	"mycute/pkg/cognee/storage"
)

// DuckDBStorage は、DuckDBを使用したVectorStorageの実装です。
// このストレージは以下のテーブルを管理します：
//   - data: 取り込まれたファイルのメタデータ
//   - documents: ファイルから抽出されたドキュメント
//   - chunks: ドキュメントを分割したチャンク
//   - vectors: チャンクやエンティティのベクトル表現
type DuckDBStorage struct {
	db *sql.DB // DuckDBへの接続
}

// NewDuckDBStorage は、DuckDBStorageの新しいインスタンスを作成します。
// 引数:
//   - db: 既に開かれたDuckDBへの接続
//
// 返り値:
//   - *DuckDBStorage: 新しいDuckDBStorageインスタンス
func NewDuckDBStorage(db *sql.DB) *DuckDBStorage {
	return &DuckDBStorage{db: db}
}

// インターフェース実装の確認
// コンパイル時に、DuckDBStorageがstorage.VectorStorageインターフェースを
// 正しく実装しているかをチェックします
var _ storage.VectorStorage = (*DuckDBStorage)(nil)

// SaveData は、ファイルのメタデータをdataテーブルに保存します。
// この関数は以下の処理を行います：
//  1. データをINSERT
//  2. 既に存在する場合（group_id, idの組み合わせが重複）は UPDATE
//
// ON CONFLICT句により、同じファイルを再度取り込んでも安全に処理されます。
//
// 引数:
//   - ctx: コンテキスト（キャンセル処理等に使用）
//   - data: 保存するデータのメタデータ
//
// 返り値:
//   - error: エラーが発生した場合
func (s *DuckDBStorage) SaveData(ctx context.Context, data *storage.Data) error {
	// UPSERT（INSERT or UPDATE）クエリ
	// ON CONFLICT句により、既存データがあれば更新、なければ挿入
	query := `
		INSERT INTO data (id, group_id, name, raw_data_location, extension, content_hash, created_at)
		VALUES (?, ?, ?, ?, ?, ?, ?)
		ON CONFLICT (group_id, id) DO UPDATE SET
			name = excluded.name,
			raw_data_location = excluded.raw_data_location,
			extension = excluded.extension,
			content_hash = excluded.content_hash,
			created_at = excluded.created_at
	`
	_, err := s.db.ExecContext(ctx, query, data.ID, data.GroupID, data.Name, data.RawDataLocation, data.Extension, data.ContentHash, data.CreatedAt)
	return err
}

// Exists は、指定されたコンテンツハッシュを持つデータが存在するかをチェックします。
// group_idによるフィルタリングを強制することで、厳格なパーティション分離を実現します。
//
// 注意:
//   - content_hashだけでもユニークである可能性が高いですが、
//     実装の一貫性のため、group_idでもフィルタリングしています
//   - これにより、異なるグループ間でのデータ漏洩を確実に防ぎます
//
// 引数:
//   - ctx: コンテキスト
//   - contentHash: ファイルのコンテンツハッシュ（SHA-256等）
//   - groupID: グループID（"user-dataset"形式）
//
// 返り値:
//   - bool: データが存在する場合true
func (s *DuckDBStorage) Exists(ctx context.Context, contentHash string, groupID string) bool {
	var count int
	// content_hash と group_id の両方で検索
	query := `SELECT COUNT(*) FROM data WHERE content_hash = ? AND group_id = ?`
	err := s.db.QueryRowContext(ctx, query, contentHash, groupID).Scan(&count)
	if err != nil {
		// エラーが発生した場合は存在しないとみなす
		return false
	}
	return count > 0
}

// GetDataByID は、IDでデータを取得します。
// group_idによるフィルタリングを強制することで、厳格なパーティション分離を実現します。
//
// 注意:
//   - IDはグローバルにユニークである可能性が高いですが、
//     実装の一貫性のため、group_idでもフィルタリングしています
//   - これにより、すべてのデータベース操作で厳格なパーティショニングを維持します
//
// 引数:
//   - ctx: コンテキスト
//   - id: データのID
//   - groupID: グループID
//
// 返り値:
//   - *storage.Data: 取得したデータ
//   - error: データが見つからない場合やエラーが発生した場合
func (s *DuckDBStorage) GetDataByID(ctx context.Context, id string, groupID string) (*storage.Data, error) {
	// id と group_id の両方で検索
	// id::VARCHAR: UUIDをVARCHARにキャスト
	query := `SELECT id::VARCHAR, group_id, name, raw_data_location, extension, content_hash, created_at FROM data WHERE id = ? AND group_id = ?`
	row := s.db.QueryRowContext(ctx, query, id, groupID)

	var data storage.Data
	// 各カラムの値をData構造体にスキャン
	if err := row.Scan(&data.ID, &data.GroupID, &data.Name, &data.RawDataLocation, &data.Extension, &data.ContentHash, &data.CreatedAt); err != nil {
		return nil, err
	}
	return &data, nil
}

// GetDataList は、指定されたグループIDに属するすべてのデータを取得します。
// この関数は、Cognifyパイプラインで処理対象のデータを取得する際に使用されます。
//
// 引数:
//   - ctx: コンテキスト
//   - groupID: グループID（"user-dataset"形式）
//
// 返り値:
//   - []*storage.Data: データのリスト
//   - error: エラーが発生した場合
func (s *DuckDBStorage) GetDataList(ctx context.Context, groupID string) ([]*storage.Data, error) {
	// group_idでフィルタリングしてすべてのデータを取得
	query := `SELECT id::VARCHAR, group_id, name, raw_data_location, extension, content_hash, created_at FROM data WHERE group_id = ?`
	rows, err := s.db.QueryContext(ctx, query, groupID)
	if err != nil {
		return nil, err
	}
	defer rows.Close() // 関数終了時にrowsをクローズ

	var dataList []*storage.Data
	// 各行をData構造体に変換してリストに追加
	for rows.Next() {
		var data storage.Data
		if err := rows.Scan(&data.ID, &data.GroupID, &data.Name, &data.RawDataLocation, &data.Extension, &data.ContentHash, &data.CreatedAt); err != nil {
			return nil, err
		}
		dataList = append(dataList, &data)
	}
	return dataList, nil
}

// SaveDocument は、ドキュメントをdocumentsテーブルに保存します。
// ドキュメントは、ファイルから抽出されたテキストとメタデータを含みます。
//
// 引数:
//   - ctx: コンテキスト
//   - doc: 保存するドキュメント
//
// 返り値:
//   - error: エラーが発生した場合
func (s *DuckDBStorage) SaveDocument(ctx context.Context, doc *storage.Document) error {
	// UPSERT（INSERT or UPDATE）クエリ
	query := `
	INSERT INTO documents (id, group_id, data_id, text, metadata, created_at)
	VALUES (?, ?, ?, ?, ?, current_timestamp)
	ON CONFLICT (group_id, id) DO UPDATE SET
		text = excluded.text,
		metadata = excluded.metadata
	`
	// メタデータをJSON文字列に変換
	metaJSON, err := json.Marshal(doc.MetaData)
	if err != nil {
		return fmt.Errorf("failed to marshal metadata: %w", err)
	}

	// ドキュメントを保存
	if _, err := s.db.ExecContext(ctx, query, doc.ID, doc.GroupID, doc.DataID, doc.Text, string(metaJSON)); err != nil {
		return fmt.Errorf("failed to save document: %w", err)
	}
	return nil
}

// SaveChunk は、チャンクをchunksテーブルとvectorsテーブルに保存します。
// この関数は以下の処理を行います：
//  1. チャンクのテキストとメタデータをchunksテーブルに保存
//  2. チャンクのベクトル表現（embedding）が存在する場合、vectorsテーブルに保存
//
// 引数:
//   - ctx: コンテキスト
//   - chunk: 保存するチャンク
//
// 返り値:
//   - error: エラーが発生した場合
func (s *DuckDBStorage) SaveChunk(ctx context.Context, chunk *storage.Chunk) error {
	// ========================================
	// 1. チャンクをchunksテーブルに保存
	// ========================================
	chunkQuery := `
		INSERT INTO chunks (id, group_id, document_id, text, chunk_index, token_count, created_at)
		VALUES (?, ?, ?, ?, ?, ?, current_timestamp)
		ON CONFLICT (group_id, id) DO UPDATE SET
			text = excluded.text,
			chunk_index = excluded.chunk_index,
			token_count = excluded.token_count
	`
	_, err := s.db.ExecContext(ctx, chunkQuery, chunk.ID, chunk.GroupID, chunk.DocumentID, chunk.Text, chunk.ChunkIndex, chunk.TokenCount)
	if err != nil {
		return fmt.Errorf("failed to save chunk: %w", err)
	}

	// ========================================
	// 2. ベクトル（embedding）が存在する場合、vectorsテーブルに保存
	// ========================================
	if len(chunk.Embedding) > 0 {
		vectorQuery := `
			INSERT INTO vectors (id, group_id, collection_name, text, embedding)
			VALUES (?, ?, ?, ?, ?)
			ON CONFLICT (group_id, collection_name, id) DO UPDATE SET
				text = excluded.text,
				embedding = excluded.embedding
		`
		// コレクション名: "DocumentChunk_text"
		// これにより、チャンクのベクトルを検索時に識別できます
		collectionName := "DocumentChunk_text"
		_, err = s.db.ExecContext(ctx, vectorQuery, chunk.ID, chunk.GroupID, collectionName, chunk.Text, chunk.Embedding)
		if err != nil {
			return fmt.Errorf("failed to save vector: %w", err)
		}
	}

	return nil
}

// SaveEmbedding は、任意のテキストのベクトル表現をvectorsテーブルに保存します。
// この関数は、エンティティ名や要約など、チャンク以外のテキストのベクトル化に使用されます。
//
// 引数:
//   - ctx: コンテキスト
//   - collectionName: コレクション名（例: "Entity_name", "TextSummary_text"）
//   - id: ベクトルのID
//   - text: 元のテキスト
//   - vector: ベクトル表現（float32の配列）
//   - groupID: グループID
//
// 返り値:
//   - error: エラーが発生した場合
func (s *DuckDBStorage) SaveEmbedding(ctx context.Context, collectionName, id, text string, vector []float32, groupID string) error {
	// UPSERT（INSERT or UPDATE）クエリ
	// ON CONFLICT句のキーは (group_id, collection_name, id) の組み合わせ
	query := `
		INSERT INTO vectors (id, group_id, collection_name, text, embedding)
		VALUES (?, ?, ?, ?, ?)
		ON CONFLICT (group_id, collection_name, id) DO UPDATE SET
			text = excluded.text,
			embedding = excluded.embedding
	`
	_, err := s.db.ExecContext(ctx, query, id, groupID, collectionName, text, vector)
	if err != nil {
		return fmt.Errorf("failed to save embedding: %w", err)
	}
	return nil
}

// Search は、ベクトル類似度検索を実行します。
// この関数は以下の処理を行います：
//  1. クエリベクトルと各ベクトルのコサイン類似度を計算
//  2. 類似度の高い順にソート
//  3. 上位k件を返す
//
// DuckDBのVSS拡張により、効率的なベクトル検索が可能です。
//
// 引数:
//   - ctx: コンテキスト
//   - collectionName: 検索対象のコレクション名
//   - vector: クエリベクトル（float32の配列、次元数は1536）
//   - k: 返す結果の最大数
//   - groupID: グループID（パーティション分離のため）
//
// 返り値:
//   - []*storage.SearchResult: 検索結果のリスト（類似度の高い順）
//   - error: エラーが発生した場合
func (s *DuckDBStorage) Search(ctx context.Context, collectionName string, vector []float32, k int, groupID string) ([]*storage.SearchResult, error) {
	// ベクトル類似度検索クエリ
	// array_cosine_similarity: コサイン類似度を計算（-1〜1、1が最も類似）
	// ?::FLOAT[1536]: クエリベクトルを1536次元のFLOAT配列にキャスト
	query := `
		SELECT id, text, array_cosine_similarity(embedding, ?::FLOAT[1536]) as score
		FROM vectors
		WHERE collection_name = ? AND group_id = ?
		ORDER BY score DESC
		LIMIT ?
	`

	// クエリを実行
	rows, err := s.db.QueryContext(ctx, query, vector, collectionName, groupID, k)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var results []*storage.SearchResult
	// 各行を SearchResult 構造体に変換
	for rows.Next() {
		var res storage.SearchResult
		if err := rows.Scan(&res.ID, &res.Text, &res.Distance); err != nil {
			return nil, err
		}
		results = append(results, &res)
	}
	return results, nil
}

// Close は、DuckDBへの接続をクローズします。
// この関数は、CogneeService.Close() から呼び出されます。
//
// 返り値:
//   - error: クローズに失敗した場合
func (s *DuckDBStorage) Close() error {
	return s.db.Close()
}
