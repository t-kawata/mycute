package types

import "go.uber.org/zap"

// CuberConfig は、Cuberサービスの初期化に必要な設定を保持する構造体です。
// データベースの配置場所とLLMプロバイダーの接続情報を含みます。
type CuberConfig struct {
	// データベースファイルを格納するディレクトリのパス
	DBDirPath string

	// LadybugDB Configuration
	LadybugDBDatabasePath string // Path to LadybugDB database file (if different from default)

	// Memify設定
	MemifyMaxCharsForBulkProcess int // デフォルト: 50000
	MemifyBatchOverlapPercent    int // デフォルト: 20
	MemifyBatchMinChars          int // デフォルト: 5000

	// Metacognition Configuration
	MetaSimilarityThresholdUnknown         float64 // Unknown解決の類似度閾値 (Default: 0.3)
	MetaSimilarityThresholdReflection      float64 // Self-Reflectionの関連情報閾値 (Default: 0.5)
	MetaSimilarityThresholdCrystallization float64 // 知識結晶化のクラスタリング閾値 (Default: 0.8)
	MetaSearchLimitUnknown                 int     // Unknown解決時の検索数 (Default: 5)
	MetaSearchLimitReflectionChunk         int     // Self-Reflection時のチャンク検索数 (Default: 3)
	MetaSearchLimitReflectionRule          int     // Self-Reflection時のルール検索数 (Default: 3)
	MetaCrystallizationMinCluster          int     // 知識結晶化の最小クラスタサイズ (Default: 2)

	// Storage Configuration
	S3UseLocal                bool   // trueならローカルストレージを使用
	S3LocalDir                string // ローカル保存先ディレクトリ (例: "data/files")
	S3DLDir                   string // s3client が Down() した時に使用するローカル保存先ディレクトリ (例: "data/files")
	StorageIdleTimeoutMinutes int    // ストレージのアイドルタイムアウト（分）

	// S3 Cleanup Configuration
	S3CleanupIntervalMinutes int // クリーンアップ実行間隔（分） (Default: 60)
	S3RetentionHours         int // ファイル保持期間（時間） (Default: 24)

	// AWS S3 Configuration (S3UseLocal=falseの場合に使用)
	S3AccessKey string
	S3SecretKey string
	S3Region    string
	S3Bucket    string
	S3Endpoint  string // S3互換ストレージのエンドポイント (例: MinIO)

	// Graph Metabolism Configuration
	GraphMetabolismAlpha           float64 // 強化学習率 (Default: 0.2)
	GraphMetabolismDelta           float64 // 減衰ペナルティ率 (Default: 0.3)
	GraphMetabolismPruneThreshold  float64 // 淘汰閾値 (Default: 0.1)
	GraphPruningGracePeriodMinutes int     // 孤立ノード削除猶予期間 (Default: 60)

	// Logger
	Logger *zap.Logger
}

// CognifyConfig は、cognifyの設定を表す構造体です。
type CognifyConfig struct {
	ChunkSize    int // チャンクのサイズとなる文字数（トークン数でカウントするとユーザーが使いにくいのでやめた）
	ChunkOverlap int // チャンクのオーバーラップとなる文字数（トークン数でカウントするとユーザーが使いにくいのでやめた）
}

type QueryConfig struct {
	QueryType   QueryType // 検索タイプ
	SummaryTopk int       // 要約の上位k件を取得
	ChunkTopk   int       // チャンクの上位k件を取得
	EntityTopk  int       // エンティティの上位k件を対象にグラフを取得
}

// MemifyConfig は、Memify処理のオプション設定を保持します。
type MemifyConfig struct {
	// RulesNodeSetName はルールセットの名前です。
	// デフォルト: "coding_agent_rules"
	// RecursiveDepth ... (updated below)
	RulesNodeSetName string
	// RecursiveDepth は、Memifyを再帰的に実行する深さを指定します。
	//
	// 値の動作:
	//   - 0: 再帰なし（Memifyを1回のみ実行）。通常のユースケースではこれで十分です。
	//   - 1以上: 指定した深さまでMemifyを繰り返し実行します。
	//     各反復で知識グラフが拡張され、より深い洞察や高次のルール抽出が期待できます。
	//
	// 注意: 再帰実行は処理時間とトークン消費量が増加します。
	// Unknown解決（Phase A）後に実行される Phase B の反復回数に対応します。
	RecursiveDepth     int
	PrioritizeUnknowns bool // Unknownの解決を優先するか（デフォルト: true）
}

// EmbeddingModelConfig は埋め込みモデルの設定を保持します。
type EmbeddingModelConfig struct {
	Provider  string `json:"provider"`
	Model     string `json:"model"`
	Dimension uint   `json:"dimension"`
	BaseURL   string `json:"base_url,omitempty"`
	ApiKey    string `json:"-"` // JSON出力(Export)には含めない
}
