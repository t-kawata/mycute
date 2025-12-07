// Package storage は、Cogneeシステムで使用されるストレージインターフェースと
// データ構造を定義します。このパッケージは、ベクトルストレージ（KuzuDB）と
// グラフストレージ（KuzuDB）の抽象化を提供します。
package storage

import (
	"context"
	"time"
)

// Data は、取り込まれたファイルのメタデータを表します。
// このデータは、KuzuDBのdataテーブルに保存されます。
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
// このデータは、KuzuDBのdocumentsテーブルに保存されます。
type Document struct {
	ID       string         `json:"id"`       // ドキュメントの一意識別子
	GroupID  string         `json:"group_id"` // グループID（パーティション分離用）
	DataID   string         `json:"data_id"`  // 親データへの外部キー
	Text     string         `json:"text"`     // ドキュメントのテキスト内容
	MetaData map[string]any `json:"metadata"` // メタデータ（JSON形式）
}

// Chunk は、ドキュメントを分割したチャンクを表します。
// このデータは、KuzuDBのchunksテーブルとvectorsテーブルに保存されます。
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
// このインターフェースは、KuzuDBStorageによって実装されます。
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

	// ========================================
	// Embedding取得操作 (Phase-09追加)
	// ========================================

	// GetEmbeddingByID は、指定されたIDのEmbeddingをvectorsテーブルから取得します。
	// この関数は、既にDBに保存されているEmbeddingを再利用する際に使用します。
	// API呼び出しを削減し、処理効率を向上させます。
	//
	// 引数:
	//   - ctx: コンテキスト
	//   - collectionName: コレクション名（例: "Rule_text", "Entity_name", "Chunk"）
	//   - id: ノードID
	//   - groupID: グループID（パーティション分離用）
	//
	// 返り値:
	//   - []float32: Embedding配列（見つからない場合はnil）
	//   - error: エラーが発生した場合
	GetEmbeddingByID(ctx context.Context, collectionName, id, groupID string) ([]float32, error)

	// GetEmbeddingsByIDs は、複数IDのEmbeddingを一括取得します。
	// バッチ処理で効率的にEmbeddingを取得する際に使用します。
	// 個別のGetEmbeddingByIDを繰り返すよりも、1回のクエリで取得する方が効率的です。
	//
	// 引数:
	//   - ctx: コンテキスト
	//   - collectionName: コレクション名
	//   - ids: ノードIDのスライス
	//   - groupID: グループID
	//
	// 返り値:
	//   - map[string][]float32: IDをキーとしたEmbeddingのマップ（見つからないIDは含まれない）
	//   - error: エラーが発生した場合
	GetEmbeddingsByIDs(ctx context.Context, collectionName string, ids []string, groupID string) (map[string][]float32, error)

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
// このデータは、KuzuDBのnodesテーブルに保存されます。
type Node struct {
	ID         string         `json:"id"`         // ノードの一意識別子
	GroupID    string         `json:"group_id"`   // グループID（パーティション分離用）
	Type       string         `json:"type"`       // ノードのタイプ（例: "Person", "Organization"）
	Properties map[string]any `json:"properties"` // ノードの属性（JSON形式）
}

// Edge は、知識グラフのエッジ（関係）を表します。
// このデータは、KuzuDBのedgesテーブルに保存されます。
type Edge struct {
	SourceID   string         `json:"source_id"`  // ソースノードのID
	TargetID   string         `json:"target_id"`  // ターゲットノードのID
	GroupID    string         `json:"group_id"`   // グループID（パーティション分離用）
	Type       string         `json:"type"`       // エッジのタイプ（例: "WORKS_AT", "LOCATED_IN"）
	Properties map[string]any `json:"properties"` // エッジの属性（JSON形式）
	Weight     float64        `json:"weight"`     // [NEW] エッジの重み（0.0〜1.0）
	Confidence float64        `json:"confidence"` // [NEW] 信頼度（0.0〜1.0）
}

// Triplet は、ノード-エッジ-ノードの3つ組を表します。
// グラフトラバーサルの結果として使用されます。
type Triplet struct {
	Source *Node // ソースノード
	Edge   *Edge // エッジ
	Target *Node // ターゲットノード
}

// ChunkData は、ストリーミング取得されるチャンクデータを表します。
// MemoryFragment 全体をロードする代わりに、チャンク単位で処理することで
// メモリ使用量を最小限に抑えます。
type ChunkData struct {
	ID         string // チャンクID
	Text       string // チャンクのテキスト内容
	GroupID    string // グループID
	DocumentID string // 親ドキュメントID
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

	// StreamDocumentChunks は、DocumentChunk タイプのノードをストリーミングで取得します。
	// 全データをメモリにロードせず、イテレーター形式で1つずつ返します。
	// これにより、大規模グラフでもメモリ使用量を一定に保てます。
	//
	// 引数:
	//   - ctx: コンテキスト（キャンセル対応）
	//   - groupID: パーティション分離用のグループID
	//
	// 戻り値:
	//   - <-chan *ChunkData: チャンクデータのチャネル（読み取り専用）
	//   - <-chan error: エラーチャネル
	StreamDocumentChunks(ctx context.Context, groupID string) (<-chan *ChunkData, <-chan error)

	// GetDocumentChunkCount は、指定されたグループIDの DocumentChunk 数を取得します。
	// 進捗表示や処理見積もりに使用されます。
	GetDocumentChunkCount(ctx context.Context, groupID string) (int, error)

	// [NEW] 指定されたタイプのノードを取得
	GetNodesByType(ctx context.Context, nodeType string, groupID string) ([]*Node, error)

	// [NEW] 指定されたエッジタイプでターゲットに接続されたノードを取得
	GetNodesByEdge(ctx context.Context, targetID string, edgeType string, groupID string) ([]*Node, error)

	// [NEW] エッジの重みを更新
	UpdateEdgeWeight(ctx context.Context, sourceID, targetID, groupID string, weight float64) error

	// [NEW] エッジの重みと信頼度を更新
	UpdateEdgeMetrics(ctx context.Context, sourceID, targetID, groupID string, weight, confidence float64) error

	// [NEW] エッジを削除
	DeleteEdge(ctx context.Context, sourceID, targetID, groupID string) error

	// [NEW] ノードを削除
	DeleteNode(ctx context.Context, nodeID, groupID string) error

	// [NEW] 指定されたノードに接続されたエッジを取得
	GetEdgesByNode(ctx context.Context, nodeID string, groupID string) ([]*Edge, error)

	// ========================================
	// 効率化API (Phase-09追加)
	// ========================================

	// GetOrphanNodes は、エッジを持たない孤立ノードを取得します。
	// この関数は、グラフのガベージコレクション（Pruning）で不要ノードを特定する際に使用します。
	// 1回のクエリで全孤立ノードを取得することで、N+1問題を回避します。
	//
	// 引数:
	//   - ctx: コンテキスト
	//   - groupID: グループID
	//   - gracePeriod: 作成からこの期間内のノードは除外（誤削除防止）
	//
	// 返り値:
	//   - []*Node: 孤立ノードのスライス
	//   - error: エラーが発生した場合
	//
	// 孤立ノードの定義:
	//   - InDegree = 0（このノードをターゲットとするエッジがない）
	//   - OutDegree = 0（このノードをソースとするエッジがない）
	//
	// 前提条件:
	//   - ノードの作成日時は properties["created_at"] にRFC3339形式で格納されていること
	GetOrphanNodes(ctx context.Context, groupID string, gracePeriod time.Duration) ([]*Node, error)

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
