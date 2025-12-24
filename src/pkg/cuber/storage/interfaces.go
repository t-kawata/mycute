// Package storage は、Cuberシステムで使用されるストレージインターフェースと
// データ構造を定義します。このパッケージは、ベクトルストレージ（LadybugDB）と
// グラフストレージ（LadybugDB）の抽象化を提供します。
package storage

import (
	"context"
	"time"

	"github.com/t-kawata/mycute/pkg/cuber/types"
)

// Data は、取り込まれたファイルのメタデータを表します。
// このデータは、LadybugDBのdataテーブルに保存されます。
type Data struct {
	ID                   string    `json:"id"`                     // データの一意識別子（UUID）
	MemoryGroup          string    `json:"memory_group"`           // メモリーグループ（"user-dataset"形式）でパーティション分離
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
// このデータは、LadybugDBのdocumentsテーブルに保存されます。
type Document struct {
	ID          string         `json:"id"`           // ドキュメントの一意識別子
	MemoryGroup string         `json:"memory_group"` // メモリーグループ（パーティション分離用）
	DataID      string         `json:"data_id"`      // 親データへの外部キー
	Text        string         `json:"text"`         // ドキュメントのテキスト内容
	MetaData    map[string]any `json:"metadata"`     // メタデータ（JSON形式）
}

// Chunk は、ドキュメントを分割したチャンクを表します。
// このデータは、LadybugDBのchunksテーブルとvectorsテーブルに保存されます。
type Chunk struct {
	ID          string    `json:"id"`           // チャンクの一意識別子
	MemoryGroup string    `json:"memory_group"` // メモリーグループ（パーティション分離用）
	DocumentID  string    `json:"document_id"`  // 親ドキュメントへの外部キー
	Text        string    `json:"text"`         // チャンクのテキスト内容
	Keywords    string    `json:"keywords"`     // FTS用キーワード（全内容語、スペース区切り）
	Nouns       string    `json:"nouns"`        // FTS用キーワード Layer 0: 名詞のみ
	NounsVerbs  string    `json:"nouns_verbs"`  // FTS用キーワード Layer 1: 名詞+動詞
	Embedding   []float32 `json:"embedding"`    // ベクトル表現（1536次元のfloat32配列）
	TokenCount  int       `json:"token_count"`  // トークン数
	ChunkIndex  int       `json:"chunk_index"`  // ドキュメント内でのチャンクの順序
}

// QueryResult は、ベクトル検索の結果を表します。
type QueryResult struct {
	ID         string  // 検索結果のID
	Text       string  // 検索結果のテキスト
	Distance   float64 // クエリとの類似度（コサイン類似度、-1〜1）
	Nouns      string  // FTS拡張用: チャンクから取り出した名詞キーワード
	NounsVerbs string  // FTS拡張用: チャンクから取り出した名詞+動詞キーワード
}

// VectorStorage は、ベクトルストレージの操作を定義するインターフェースです。
// このインターフェースは、LadybugDBStorageによって実装されます。
type VectorStorage interface {
	// ========================================
	// メタデータ操作
	// ========================================

	// SaveData は、ファイルのメタデータを保存します。
	SaveData(ctx context.Context, data *Data) error

	// Exists は、指定されたコンテンツハッシュとメモリーグループを持つデータが存在するかをチェックします。
	// memory_groupによる厳格なフィルタリングを行います。
	Exists(ctx context.Context, contentHash string, memoryGroup string) bool

	// GetDataByID は、IDとメモリーグループでデータを取得します。
	// memory_groupによる厳格なフィルタリングを行います。
	GetDataByID(ctx context.Context, id string, memoryGroup string) (*Data, error)

	// GetDataList は、指定されたメモリーグループに属するすべてのデータを取得します。
	GetDataList(ctx context.Context, memoryGroup string) ([]*Data, error)

	// ========================================
	// ベクトル操作
	// ========================================

	// SaveDocument は、ドキュメントを保存します。
	SaveDocument(ctx context.Context, document *Document) error

	// SaveChunk は、チャンクとそのベクトル表現を保存します。
	SaveChunk(ctx context.Context, chunk *Chunk) error

	// SaveEmbedding は、任意のテキストのベクトル表現を保存します。
	// tableName: テーブル名（例: "Entity", "Summary", "Rule"）
	SaveEmbedding(ctx context.Context, tableName types.TableName, id string, text string, vector []float32, memoryGroup string) error

	// Query は、ベクトル類似度検索を実行します。
	// tableName: 検索対象のテーブル
	// vector: クエリベクトル
	// k: 返す結果の最大数
	// memoryGroup: メモリーグループ（パーティション分離用）
	Query(ctx context.Context, tableName types.TableName, vector []float32, topk int, memoryGroup string) ([]*QueryResult, error)

	// FullTextSearch は、全文検索を実行します。
	// 検索クエリを形態素解析し、指定されたレイヤーのインデックスを使用して検索します。
	// tableName: 検索対象のテーブル（通常は Chunk）
	// query: 検索クエリ文字列
	// topk: 返す結果の最大数
	// memoryGroup: メモリーグループ（パーティション分離用）
	// isEn: true=英語、false=日本語
	// layer: 検索に使用するFTSレイヤー（nouns, nouns_verbs, all）
	FullTextSearch(ctx context.Context, tableName types.TableName, query string, topk int, memoryGroup string, isEn bool, layer types.FtsLayer) ([]*QueryResult, error)

	// ========================================
	// Embedding取得操作 (Phase-09追加)
	// ========================================

	// GetDocumentByID は、指定されたIDのDocumentを取得します。
	GetDocumentByID(ctx context.Context, id string, memoryGroup string) (*Document, error)

	// GetEmbeddingByID は、指定されたIDのEmbeddingをvectorsテーブルから取得します。
	// この関数は、既にDBに保存されているEmbeddingを再利用する際に使用します。
	// API呼び出しを削減し、処理効率を向上させます。
	//
	// 引数:
	//   - ctx: コンテキスト
	//   - tableName: テーブル名（例: "Rule", "Entity", "Chunk"）
	//   - id: ノードID
	//   - memoryGroup: メモリーグループ（パーティション分離用）
	//
	// 返り値:
	//   - []float32: Embedding配列（見つからない場合はnil）
	//   - error: エラーが発生した場合
	GetEmbeddingByID(ctx context.Context, tableName types.TableName, id string, memoryGroup string) ([]float32, error)

	// GetEmbeddingsByIDs は、複数IDのEmbeddingを一括取得します。
	// バッチ処理で効率的にEmbeddingを取得する際に使用します。
	// 個別のGetEmbeddingByIDを繰り返すよりも、1回のクエリで取得する方が効率的です。
	//
	// 引数:
	//   - ctx: コンテキスト
	//   - tableName: テーブル名
	//   - ids: ノードIDのスライス
	//   - memoryGroup: メモリーグループ
	//
	// 返り値:
	//   - map[string][]float32: IDをキーとしたEmbeddingのマップ（見つからないIDは含まれない）
	//   - error: エラーが発生した場合
	GetEmbeddingsByIDs(ctx context.Context, tableName types.TableName, ids []string, memoryGroup string) (map[string][]float32, error)

	// Transaction は与えられた関数をトランザクション内で実行します。
	Transaction(ctx context.Context, fn func(txCtx context.Context) error) error

	// Checkpoint は、WAL（Write-Ahead Log）をメインのデータベースファイルにマージします。
	Checkpoint() error

	// Close は、ストレージへの接続をクローズします。
	Close() error

	// IsOpen は、ストレージ接続が開いているかどうかを返します。
	IsOpen() bool
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
	//   - types.TokenUsage: トークン使用量
	//   - error: エラーが発生した場合
	EmbedQuery(ctx context.Context, text string) ([]float32, types.TokenUsage, error)
}

// Node は、知識グラフのノード（エンティティ）を表します。
// このデータは、LadybugDBのnodesテーブルに保存されます。
type Node struct {
	ID          string         `json:"id"`           // ノードの一意識別子
	MemoryGroup string         `json:"memory_group"` // メモリーグループ（パーティション分離用）
	Type        string         `json:"type"`         // ノードのタイプ（例: "Person", "Organization"）
	Properties  map[string]any `json:"properties"`   // ノードの属性（JSON形式）
}

// Edge は、知識グラフのエッジ（関係）を表します。
// このデータは、LadybugDBのedgesテーブルに保存されます。
type Edge struct {
	SourceID    string         `json:"source_id"`    // ソースノードのID
	TargetID    string         `json:"target_id"`    // ターゲットノードのID
	MemoryGroup string         `json:"memory_group"` // メモリーグループ（パーティション分離用）
	Type        string         `json:"type"`         // エッジのタイプ（例: "WORKS_AT", "LOCATED_IN"）
	Properties  map[string]any `json:"properties"`   // エッジの属性（JSON形式）
	Weight      float64        `json:"weight"`       // エッジの重み（0.0〜1.0）
	Confidence  float64        `json:"confidence"`   // 信頼度（0.0〜1.0）
	Unix        int64          `json:"unix"`         // 観測・更新時のUnixタイムスタンプ（ミリ秒）
	Thickness   float64        `json:"thickness"`    // 計算された Thickness 値（クエリ時に動的算出、Weight × Confidence × 時間減衰）
}

// Triple は、ノード-エッジ-ノードの3つ組を表します。
// グラフトラバーサルの結果として使用されます。
type Triple struct {
	Source *Node `json:"source"` // ソースノード
	Edge   *Edge `json:"edge"`   // エッジ
	Target *Node `json:"target"` // ターゲットノード
}

// ChunkData は、ストリーミング取得されるチャンクデータを表します。
// MemoryFragment 全体をロードする代わりに、チャンク単位で処理することで
// メモリ使用量を最小限に抑えます。
type ChunkData struct {
	ID          string // チャンクID
	Text        string // チャンクのテキスト内容
	MemoryGroup string // メモリーグループ
	DocumentID  string // 親ドキュメントID
}

// GraphStorage は、グラフストレージの操作を定義するインターフェースです。
// このインターフェースは、CozoStorageによって実装されます。
type GraphStorage interface {
	// AddNodes は、複数のノードをグラフに追加します。
	AddNodes(ctx context.Context, nodes []*Node) error

	// AddEdges は、複数のエッジをグラフに追加します。
	AddEdges(ctx context.Context, edges []*Edge) error

	// GetTriples は、指定されたノードIDに関連するトリプルを取得します。
	// memory_groupによる厳格なフィルタリングを行います。
	//
	// 注意:
	//   - nodeIDsは既にmemory_groupでフィルタリングされたベクトル検索結果から来ている可能性が高いですが、
	//     実装の一貫性と厳格なパーティション分離のため、ここでも明示的にmemory_groupでフィルタリングします
	//   - nodeIDsが空の場合は空のスライスを返します
	GetTriples(ctx context.Context, nodeIDs []string, memoryGroup string) ([]*Triple, error)

	// GetSourceNodeIDs は、エッジの発信元となるノードIDを重複なしでページング取得します。
	// Metabolism処理の大規模グラフ対応に使用します。
	//
	// 引数:
	//   - ctx: コンテキスト
	//   - memoryGroup: メモリーグループ
	//   - offset: スキップするノード数
	//   - limit: 取得するノード数
	//
	// 戻り値:
	//   - []string: ノードIDのスライス（ID順でソート済み）
	//   - error: エラー
	GetSourceNodeIDs(ctx context.Context, memoryGroup string, offset, limit int) ([]string, error)

	// GetTriplesBySourceIDs は、指定されたSourceノードIDに関連するトリプルを取得します。
	// ページング化されたMetabolism処理で使用します。
	//
	// 引数:
	//   - ctx: コンテキスト
	//   - sourceIDs: 発信元ノードIDのスライス
	//   - memoryGroup: メモリーグループ
	//
	// 戻り値:
	//   - []*Triple: トリプルのスライス
	//   - error: エラー
	GetTriplesBySourceIDs(ctx context.Context, sourceIDs []string, memoryGroup string) ([]*Triple, error)

	// StreamDocumentChunks は、DocumentChunk タイプのノードをストリーミングで取得します。
	// 全データをメモリにロードせず、イテレーター形式で1つずつ返します。
	// これにより、大規模グラフでもメモリ使用量を一定に保てます。
	//
	// 引数:
	//   - ctx: コンテキスト（キャンセル対応）
	//   - memoryGroup: パーティション分離用のメモリーグループ
	//
	// 戻り値:
	//   - <-chan *ChunkData: チャンクデータのチャネル（読み取り専用）
	//   - <-chan error: エラーチャネル
	StreamDocumentChunks(ctx context.Context, memoryGroup string) (<-chan *ChunkData, <-chan error)

	// GetDocumentChunkCount は、指定されたメモリーグループの DocumentChunk 数を取得します。
	// 進捗表示や処理見積もりに使用されます。
	GetDocumentChunkCount(ctx context.Context, memoryGroup string) (int, error)

	// 指定されたタイプのノードを取得
	GetNodesByType(ctx context.Context, nodeType string, memoryGroup string) ([]*Node, error)

	// 指定されたエッジタイプでターゲットに接続されたノードを取得
	GetNodesByEdge(ctx context.Context, targetID string, edgeType string, memoryGroup string) ([]*Node, error)

	// エッジの重みを更新
	UpdateEdgeWeight(ctx context.Context, sourceID, targetID, memoryGroup string, weight float64) error

	// エッジの重みと信頼度とタイムスタンプを更新
	UpdateEdgeMetrics(ctx context.Context, sourceID, targetID, memoryGroup string, weight, confidence float64, unix int64) error

	// エッジを削除
	// 注意: sourceID, edgeType, targetID の組み合わせで特定のエッジのみを削除します。
	DeleteEdge(ctx context.Context, sourceID, edgeType, targetID, memoryGroup string) error

	// ノードを削除
	DeleteNode(ctx context.Context, nodeID, memoryGroup string) error

	// 指定されたノードに接続されたエッジを取得
	GetEdgesByNode(ctx context.Context, nodeID string, memoryGroup string) ([]*Edge, error)

	// ========================================
	// 効率化API (Phase-09追加)
	// ========================================

	// GetOrphanNodes は、エッジを持たない孤立ノードを取得します。
	// この関数は、グラフのガベージテーブル（Pruning）で不要ノードを特定する際に使用します。
	// 1回のクエリで全孤立ノードを取得することで、N+1問題を回避します。
	//
	// 引数:
	//   - ctx: コンテキスト
	//   - memoryGroup: メモリーグループ
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
	GetOrphanNodes(ctx context.Context, memoryGroup string, gracePeriod time.Duration) ([]*Node, error)

	// GetWeaklyConnectedNodes は、接続エッジの全てが「弱い」と判断されるノードを取得します。
	// MDL Principle に基づくノード削除の候補を抽出する目的で使用されます。
	//
	// 引数:
	//   - ctx: コンテキスト
	//   - memoryGroup: メモリーグループ
	//   - thicknessThreshold: Thickness 閾値（Weight × Confidence がこの値以下のエッジを「弱い」と判定）
	//   - gracePeriod: この期間内に作成されたノードは除外（誤削除防止）
	//
	// 戻り値:
	//   - []*Node: 「弱い接続のみを持つ」ノードのスライス
	//   - error: エラー
	//
	// 対象ノードの条件:
	//   1. 1 本以上のエッジを持つ（完全孤立ではない）
	//   2. 接続している全てのエッジの Thickness (= Weight × Confidence) が thicknessThreshold 以下
	//   3. ノードの created_at が gracePeriod より古い
	GetWeaklyConnectedNodes(ctx context.Context, memoryGroup string, thicknessThreshold float64, gracePeriod time.Duration) ([]*Node, error)

	// EnsureSchema は、グラフデータベースのスキーマを作成します。
	EnsureSchema(ctx context.Context, config types.EmbeddingModelConfig) error

	// Transaction は与えられた関数をトランザクション内で実行します。
	Transaction(ctx context.Context, fn func(txCtx context.Context) error) error

	// Checkpoint は、WAL（Write-Ahead Log）をメインのデータベースファイルにマージします。
	Checkpoint() error

	// Close は、ストレージへの接続をクローズします。
	Close() error

	// IsOpen は、ストレージ接続が開いているかどうかを返します。
	IsOpen() bool

	// GetMaxUnix は、指定されたメモリーグループ内のエッジの最大Unixタイムスタンプを取得します。
	// 相対時間減衰の基準点として使用されます。
	GetMaxUnix(ctx context.Context, memoryGroup string) (int64, error)

	// GetMemoryGroupConfig は、指定されたメモリーグループの設定を取得します。
	// 存在しない場合はnilを返します。
	GetMemoryGroupConfig(ctx context.Context, memoryGroup string) (*MemoryGroupConfig, error)

	// UpsertMemoryGroup は、メモリーグループの設定を作成または更新します。
	// Absorbリクエスト時に動的にグループ設定を初期化・調整するために使用されます。
	UpsertMemoryGroup(ctx context.Context, config *MemoryGroupConfig) error
}

// MemoryGroupConfig は、メモリーグループごとの代謝パラメータを保持します。
// メモリーグループごとに最適化された知識のライフサイクル管理のための設定です。
type MemoryGroupConfig struct {
	ID                         string  `json:"id"`                            // メモリーグループ名（例: "project-a"）
	HalfLifeDays               float64 `json:"half_life_days"`                // 価値が半減する日数（デフォルト: 30）
	PruneThreshold             float64 `json:"prune_threshold"`               // 削除対象となるThickness閾値（デフォルト: 0.1）
	MinSurvivalProtectionHours float64 `json:"min_survival_protection_hours"` // 新規知識の最低生存保護期間（デフォルト: 72時間）
	MdlKNeighbors              int     `json:"mdl_k_neighbors"`               // MDL判定時の近傍ノード数（デフォルト: 5）
}

// GraphData は、ノードとエッジのテーブルを表します。
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
