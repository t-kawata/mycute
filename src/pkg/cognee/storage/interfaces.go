// Package storage は、Cogneeシステムで使用されるストレージインターフェースと
// データ構造を定義します。このパッケージは、ベクトルストレージ（DuckDB）と
// グラフストレージ（CozoDB）の抽象化を提供します。
package storage

import (
	"context"
	"time"
)

// Data は、取り込まれたファイルのメタデータを表します。
// このデータは、DuckDBのdataテーブルに保存されます。
type Data struct {
	ID                   string    `json:"id"`                     // データの一意識別子（UUID）
	GroupID              string    `json:"group_id"`               // グループID（"user-dataset"形式）でパーティション分離
	Name                 string    `json:"name"`                   // ファイル名
	RawDataLocation      string    `json:"raw_data_location"`      // 変換後のテキストファイルのパス
	OriginalDataLocation string    `json:"original_data_location"` // 元のファイルのパス
	Extension            string    `json:"extension"`              // ファイル拡張子
	MimeType             string    `json:"mime_type"`              // MIMEタイプ
	ContentHash          string    `json:"content_hash"`           // ファイルのコンテンツハッシュ（SHA-256等）
	OwnerID              string    `json:"owner_id"`               // 所有者ID
	CreatedAt            time.Time `json:"created_at"`             // 作成日時
}

// Document は、ファイルから抽出されたドキュメントを表します。
// このデータは、DuckDBのdocumentsテーブルに保存されます。
type Document struct {
	ID       string                 `json:"id"`       // ドキュメントの一意識別子
	GroupID  string                 `json:"group_id"` // グループID（パーティション分離用）
	DataID   string                 `json:"data_id"`  // 親データへの外部キー
	Text     string                 `json:"text"`     // ドキュメントのテキスト内容
	MetaData map[string]interface{} `json:"metadata"` // メタデータ（JSON形式）
}

// Chunk は、ドキュメントを分割したチャンクを表します。
// このデータは、DuckDBのchunksテーブルとvectorsテーブルに保存されます。
type Chunk struct {
	ID         string    `json:"id"`          // チャンクの一意識別子
	GroupID    string    `json:"group_id"`    // グループID（パーティション分離用）
	DocumentID string    `json:"document_id"` // 親ドキュメントへの外部キー
	Text       string    `json:"text"`        // チャンクのテキスト内容
	Embedding  []float32 `json:"embedding"`   // ベクトル表現（1536次元のfloat32配列）
	TokenCount int       `json:"token_count"` // トークン数
	ChunkIndex int       `json:"chunk_index"` // ドキュメント内でのチャンクの順序
}

// SearchResult は、ベクトル検索の結果を表します。
type SearchResult struct {
	ID       string  // 検索結果のID
	Text     string  // 検索結果のテキスト
	Distance float64 // クエリとの類似度（コサイン類似度、-1〜1）
}

// VectorStorage は、ベクトルストレージの操作を定義するインターフェースです。
// このインターフェースは、DuckDBStorageによって実装されます。
type VectorStorage interface {
	// ========================================
	// メタデータ操作
	// ========================================

	// SaveData は、ファイルのメタデータを保存します。
	SaveData(ctx context.Context, data *Data) error

	// Exists は、指定されたコンテンツハッシュとグループIDを持つデータが存在するかをチェックします。
	// group_idによる厳格なフィルタリングを行います。
	Exists(ctx context.Context, contentHash string, groupID string) bool

	// GetDataByID は、IDとグループIDでデータを取得します。
	// group_idによる厳格なフィルタリングを行います。
	GetDataByID(ctx context.Context, id string, groupID string) (*Data, error)

	// GetDataList は、指定されたグループIDに属するすべてのデータを取得します。
	GetDataList(ctx context.Context, groupID string) ([]*Data, error)

	// ========================================
	// ベクトル操作
	// ========================================

	// SaveDocument は、ドキュメントを保存します。
	SaveDocument(ctx context.Context, document *Document) error

	// SaveChunk は、チャンクとそのベクトル表現を保存します。
	SaveChunk(ctx context.Context, chunk *Chunk) error

	// SaveEmbedding は、任意のテキストのベクトル表現を保存します。
	// collectionName: コレクション名（例: "Entity_name", "TextSummary_text"）
	SaveEmbedding(ctx context.Context, collectionName, id, text string, vector []float32, groupID string) error

	// Search は、ベクトル類似度検索を実行します。
	// collectionName: 検索対象のコレクション
	// vector: クエリベクトル
	// k: 返す結果の最大数
	// groupID: グループID（パーティション分離用）
	Search(ctx context.Context, collectionName string, vector []float32, k int, groupID string) ([]*SearchResult, error)

	// Close は、ストレージへの接続をクローズします。
	Close() error
}

// Embedder は、テキストをベクトル表現に変換する操作を定義するインターフェースです。
// このインターフェースは、OpenAIEmbedderAdapterによって実装されます。
type Embedder interface {
	// EmbedQuery は、テキストをベクトル表現に変換します。
	// 引数:
	//   - ctx: コンテキスト
	//   - text: ベクトル化するテキスト
	// 返り値:
	//   - []float32: ベクトル表現（1536次元）
	//   - error: エラーが発生した場合
	EmbedQuery(ctx context.Context, text string) ([]float32, error)
}

// Node は、知識グラフのノード（エンティティ）を表します。
// このデータは、CozoDBのnodesリレーションに保存されます。
type Node struct {
	ID         string                 `json:"id"`         // ノードの一意識別子
	GroupID    string                 `json:"group_id"`   // グループID（パーティション分離用）
	Type       string                 `json:"type"`       // ノードのタイプ（例: "Person", "Organization"）
	Properties map[string]interface{} `json:"properties"` // ノードの属性（JSON形式）
}

// Edge は、知識グラフのエッジ（関係）を表します。
// このデータは、CozoDBのedgesリレーションに保存されます。
type Edge struct {
	SourceID   string                 `json:"source_id"`  // ソースノードのID
	TargetID   string                 `json:"target_id"`  // ターゲットノードのID
	GroupID    string                 `json:"group_id"`   // グループID（パーティション分離用）
	Type       string                 `json:"type"`       // エッジのタイプ（例: "WORKS_AT", "LOCATED_IN"）
	Properties map[string]interface{} `json:"properties"` // エッジの属性（JSON形式）
}

// Triplet は、ノード-エッジ-ノードの3つ組を表します。
// グラフトラバーサルの結果として使用されます。
type Triplet struct {
	Source *Node // ソースノード
	Edge   *Edge // エッジ
	Target *Node // ターゲットノード
}

// GraphStorage は、グラフストレージの操作を定義するインターフェースです。
// このインターフェースは、CozoStorageによって実装されます。
type GraphStorage interface {
	// AddNodes は、複数のノードをグラフに追加します。
	AddNodes(ctx context.Context, nodes []*Node) error

	// AddEdges は、複数のエッジをグラフに追加します。
	AddEdges(ctx context.Context, edges []*Edge) error

	// GetTriplets は、指定されたノードIDに関連するトリプレットを取得します。
	// group_idによる厳格なフィルタリングを行います。
	//
	// 注意:
	//   - nodeIDsは既にgroup_idでフィルタリングされたベクトル検索結果から来ている可能性が高いですが、
	//     実装の一貫性と厳格なパーティション分離のため、ここでも明示的にgroup_idでフィルタリングします
	GetTriplets(ctx context.Context, nodeIDs []string, groupID string) ([]*Triplet, error)

	// EnsureSchema は、グラフデータベースのスキーマを作成します。
	EnsureSchema(ctx context.Context) error

	// Close は、ストレージへの接続をクローズします。
	Close() error
}

// GraphData は、ノードとエッジのコレクションを表します。
// グラフ抽出タスクの出力として使用されます。
type GraphData struct {
	Nodes []*Node `json:"nodes"` // ノードのリスト
	Edges []*Edge `json:"edges"` // エッジのリスト
}

// CognifyOutput は、Cognifyパイプラインの各ステップの出力を表します。
// パイプライン内でデータを受け渡すために使用されます。
type CognifyOutput struct {
	Chunks    []*Chunk   `json:"chunks"`     // チャンクのリスト
	GraphData *GraphData `json:"graph_data"` // グラフデータ
}
