// Package cognee は、Cognee サービスのコアとなるパッケージです。
// このパッケージは、データの取り込み(Add)、知識グラフ化(Cognify)、検索(Search)の
// 3つの主要機能を提供します。
package cognee

import (
	"context"
	"database/sql"
	_ "embed"
	"fmt"
	"log"
	"os"
	"path/filepath"
	"runtime"
	"strings"
	"time"

	"mycute/pkg/cognee/db/cozodb"
	duckdbRepo "mycute/pkg/cognee/db/duckdb"
	"mycute/pkg/cognee/db/kuzudb"
	"mycute/pkg/cognee/pipeline"
	"mycute/pkg/cognee/storage"
	"mycute/pkg/cognee/tasks/chunking"
	"mycute/pkg/cognee/tasks/graph"
	"mycute/pkg/cognee/tasks/ingestion"
	"mycute/pkg/cognee/tasks/memify"
	"mycute/pkg/cognee/tasks/metacognition"
	storageTaskPkg "mycute/pkg/cognee/tasks/storage"
	"mycute/pkg/cognee/tasks/summarization"
	"mycute/pkg/cognee/tools/search"
	"mycute/pkg/s3client"

	cozo "github.com/cozodb/cozo-lib-go"
	_ "github.com/duckdb/duckdb-go/v2"
	"github.com/tmc/langchaingo/llms"
	"github.com/tmc/langchaingo/llms/openai"
)

// DuckDB VSS拡張のバイナリデータ（Darwin ARM64版）
// ビルド時にバイナリファイルとして埋め込まれます
//
//go:embed db/duckdb/extensions/v1.4.2/darwin_arm64/vss.duckdb_extension
var duckDbVssDarwinArm64 []byte

// DuckDB VSS拡張のバイナリデータ（Linux AMD64版）
// ビルド時にバイナリファイルとして埋め込まれます
//
//go:embed db/duckdb/extensions/v1.4.2/linux_amd64/vss.duckdb_extension
var duckDbVssLinuxAmd64 []byte

// DuckDBのスキーマ定義SQL
// ビルド時にschema.sqlファイルの内容が埋め込まれます
//
//go:embed db/duckdb/schema.sql
var duckDBSchema string

// CogneeConfig は、Cogneeサービスの初期化に必要な設定を保持する構造体です。
// データベースの配置場所とLLMプロバイダーの接続情報を含みます。
type CogneeConfig struct {
	// データベースファイルを格納するディレクトリのパス
	DBDirPath string
	// データベースファイルのベース名（拡張子なし）
	// 実際には ${DBName}.duckdb と ${DBName}.cozodb が作成されます
	DBName string

	// Database Mode Configuration (Phase-10)
	DatabaseMode       string // "duckdb+cozodb" (default) or "kuzudb"
	KuzuDBDatabasePath string // Path to KuzuDB directory (if different from default)

	// Completion (テキスト生成) LLM の設定
	CompletionAPIKey  string // APIキー（必須）
	CompletionBaseURL string // ベースURL（オプション、Bifrostプロキシ等で使用）
	CompletionModel   string // モデル名（オプション、例: gpt-4o）

	// Embeddings (ベクトル化) LLM の設定
	EmbeddingsAPIKey  string // APIキー（必須）
	EmbeddingsBaseURL string // ベースURL（オプション、Bifrostプロキシ等で使用）
	EmbeddingsModel   string // モデル名（オプション、例: text-embedding-3-small）

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
	S3UseLocal  bool   // trueならローカルストレージを使用
	S3LocalPath string // ローカル保存先ディレクトリ (例: "data/files")

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
}

// CogneeService は、Cogneeの主要な機能を提供するサービス構造体です。
// データベース接続とLLMクライアントを内部で保持し、ライフサイクルを管理します。
type CogneeService struct {
	VectorStorage storage.VectorStorage // ベクトルストレージ（DuckDB）
	GraphStorage  storage.GraphStorage  // グラフストレージ（CozoDB）
	Embedder      storage.Embedder      // テキストのベクトル化を行うEmbedder
	LLM           llms.Model            // テキスト生成を行うLLM
	Config        CogneeConfig          // 設定値を保持
	S3Client      *s3client.S3Client    // S3クライアント（ローカル/S3両対応）
	closeCh       chan struct{}         // サービス終了通知用チャネル
}

// NewCogneeService は、CogneeServiceの新しいインスタンスを作成します。
// この関数は以下の処理を順番に実行します：
//  1. DuckDBとCozoDBのファイルパスを構築
//  2. DuckDBを初期化し、VSS拡張をロード、スキーマを適用
//  3. CozoDBを初期化し、スキーマを適用
//  4. Embeddings用のLLMクライアントを初期化
//  5. Completion用のLLMクライアントを初期化
//  6. S3Clientを初期化
//
// エラーが発生した場合は、それまでに開いたリソースをクリーンアップしてからエラーを返します。
func NewCogneeService(config CogneeConfig) (*CogneeService, error) {
	// ========================================
	// 0. 設定のデフォルト値を適用
	// ========================================
	// Memify設定
	if config.MemifyMaxCharsForBulkProcess == 0 {
		config.MemifyMaxCharsForBulkProcess = 50000
	}
	if config.MemifyBatchOverlapPercent == 0 {
		config.MemifyBatchOverlapPercent = 20
	}
	if config.MemifyBatchMinChars == 0 {
		config.MemifyBatchMinChars = 5000
	}

	// Metacognition設定
	if config.MetaSimilarityThresholdUnknown == 0 {
		config.MetaSimilarityThresholdUnknown = 0.3
	}
	if config.MetaSimilarityThresholdReflection == 0 {
		config.MetaSimilarityThresholdReflection = 0.5
	}
	if config.MetaSimilarityThresholdCrystallization == 0 {
		config.MetaSimilarityThresholdCrystallization = 0.8
	}
	if config.MetaSearchLimitUnknown == 0 {
		config.MetaSearchLimitUnknown = 5
	}
	if config.MetaSearchLimitReflectionChunk == 0 {
		config.MetaSearchLimitReflectionChunk = 3
	}
	if config.MetaSearchLimitReflectionRule == 0 {
		config.MetaSearchLimitReflectionRule = 3
	}
	if config.MetaCrystallizationMinCluster == 0 {
		config.MetaCrystallizationMinCluster = 2
	}

	// S3 Cleanup設定
	if config.S3CleanupIntervalMinutes == 0 {
		config.S3CleanupIntervalMinutes = 60
	}
	if config.S3RetentionHours == 0 {
		config.S3RetentionHours = 24
	}

	// Graph Metabolism設定
	if config.GraphMetabolismAlpha == 0 {
		config.GraphMetabolismAlpha = 0.2
	}
	if config.GraphMetabolismDelta == 0 {
		config.GraphMetabolismDelta = 0.3
	}
	if config.GraphMetabolismPruneThreshold == 0 {
		config.GraphMetabolismPruneThreshold = 0.1
	}
	if config.GraphPruningGracePeriodMinutes == 0 {
		config.GraphPruningGracePeriodMinutes = 60
	}

	// DatabaseMode のデフォルト設定
	if config.DatabaseMode == "" {
		config.DatabaseMode = "duckdb+cozodb"
	}

	// データベースインスタンスの変数を宣言
	var vectorStorage storage.VectorStorage
	var graphStorage storage.GraphStorage
	// クリーンアップ用のクロージャ
	var cleanupFunc func()

	switch config.DatabaseMode {
	case "kuzudb":
		// ========================================
		// 1. KuzuDB の初期化
		// ========================================
		// パス設定
		if config.KuzuDBDatabasePath == "" {
			config.KuzuDBDatabasePath = filepath.Join(config.DBDirPath, "kuzudb")
		}

		// 親ディレクトリの作成
		parentDir := filepath.Dir(config.KuzuDBDatabasePath)
		if err := os.MkdirAll(parentDir, 0755); err != nil {
			return nil, fmt.Errorf("failed to create KuzuDB parent directory: %w", err)
		}

		log.Printf("[Cognee] Initializing KuzuDB at %s", config.KuzuDBDatabasePath)
		kuzuSt, err := kuzudb.NewKuzuDBStorage(config.KuzuDBDatabasePath)
		if err != nil {
			return nil, fmt.Errorf("failed to create KuzuDBStorage: %w", err)
		}

		// スキーマ適用
		if err := kuzuSt.EnsureSchema(context.Background()); err != nil {
			kuzuSt.Close()
			return nil, fmt.Errorf("failed to apply KuzuDB schema: %w", err)
		}

		// インターフェースに割り当て (KuzuDBは両方を実装)
		vectorStorage = kuzuSt
		graphStorage = kuzuSt

		cleanupFunc = func() {
			kuzuSt.Close()
		}

	default: // "duckdb+cozodb" and others
		// ========================================
		// 1. DuckDB & CozoDB の初期化 (Legacy)
		// ========================================
		// データベースファイルのフルパスを構築
		duckDBPath := filepath.Join(config.DBDirPath, config.DBName+".duckdb")
		cozoDBPath := filepath.Join(config.DBDirPath, config.DBName+".cozodb")

		// DuckDBへの接続を開く
		duckDBConn, err := sql.Open("duckdb", fmt.Sprintf("%s?access_mode=READ_WRITE&hnsw_enable_experimental_persistence=true", duckDBPath))
		if err != nil {
			return nil, fmt.Errorf("failed to open DuckDB: %w", err)
		}

		// VSS拡張ロード
		if err := loadDuckDBExtension(duckDBConn); err != nil {
			duckDBConn.Close()
			return nil, fmt.Errorf("failed to load VSS extension: %w", err)
		}

		// スキーマ適用
		if _, err := duckDBConn.Exec(duckDBSchema); err != nil {
			duckDBConn.Close()
			return nil, fmt.Errorf("failed to apply DuckDB schema: %w", err)
		}

		// DuckDBStorageインスタンスを作成
		duckSt := duckdbRepo.NewDuckDBStorage(duckDBConn)

		// CozoDBへの接続を開く
		cozodbInstance, err := cozo.New("rocksdb", cozoDBPath, nil)
		if err != nil {
			duckDBConn.Close()
			return nil, fmt.Errorf("failed to open CozoDB: %w", err)
		}

		// CozoStorageインスタンスを作成
		cozoSt := cozodb.NewCozoStorage(&cozodbInstance)

		// スキーマ適用
		if err := cozoSt.EnsureSchema(context.Background()); err != nil {
			cozodbInstance.Close()
			duckDBConn.Close()
			return nil, fmt.Errorf("failed to apply CozoDB schema: %w", err)
		}

		vectorStorage = duckSt
		graphStorage = cozoSt

		cleanupFunc = func() {
			cozodbInstance.Close()
			duckDBConn.Close()
		}
	}

	// ========================================
	// 3. Embeddings LLM の初期化
	// ========================================
	// Embeddings用のLLMクライアントのオプションを構築
	// 最低限APIキーは必須です
	embeddingsOpts := []openai.Option{openai.WithToken(config.EmbeddingsAPIKey)}

	// BaseURLが指定されている場合は追加（Bifrostプロキシ等で使用）
	if config.EmbeddingsBaseURL != "" {
		embeddingsOpts = append(embeddingsOpts, openai.WithBaseURL(config.EmbeddingsBaseURL))
	}

	// モデル名が指定されている場合は追加
	if config.EmbeddingsModel != "" {
		embeddingsOpts = append(embeddingsOpts, openai.WithEmbeddingModel(config.EmbeddingsModel))
	}

	// Embeddings用のLLMクライアントを作成
	embeddingsLLM, err := openai.New(embeddingsOpts...)
	if err != nil {
		if cleanupFunc != nil {
			cleanupFunc()
		}
		return nil, fmt.Errorf("failed to initialize Embeddings LLM: %w", err)
	}

	// Embedderアダプターを作成
	// このアダプターを通じてテキストのベクトル化を行います
	embedder := search.NewOpenAIEmbedderAdapter(embeddingsLLM)

	// ========================================
	// 4. Completion LLM の初期化
	// ========================================
	// Completion用のLLMクライアントのオプションを構築
	completionOpts := []openai.Option{openai.WithToken(config.CompletionAPIKey)}

	// BaseURLが指定されている場合は追加
	if config.CompletionBaseURL != "" {
		completionOpts = append(completionOpts, openai.WithBaseURL(config.CompletionBaseURL))
	}

	// モデル名が指定されている場合は追加
	if config.CompletionModel != "" {
		completionOpts = append(completionOpts, openai.WithModel(config.CompletionModel))
	}

	// Completion用のLLMクライアントを作成
	completionLLM, err := openai.New(completionOpts...)
	if err != nil {
		if cleanupFunc != nil {
			cleanupFunc()
		}
		return nil, fmt.Errorf("failed to initialize Completion LLM: %w", err)
	}

	// ========================================
	// 5. S3Client の初期化
	// ========================================
	// ダウンロード用ディレクトリは一時ディレクトリまたはキャッシュディレクトリを指定
	downDir := filepath.Join(config.DBDirPath, "downloads")

	s3Client, err := s3client.NewS3Client(
		config.S3AccessKey,
		config.S3SecretKey,
		config.S3Region,
		config.S3Bucket,
		config.S3LocalPath, // アップロード先（ローカルモード時）
		downDir,            // ダウンロード先（キャッシュ）
		config.S3UseLocal,
	)
	if err != nil {
		if cleanupFunc != nil {
			cleanupFunc()
		}
		return nil, fmt.Errorf("failed to initialize S3Client: %w", err)
	}

	// サービス終了通知用チャネルの作成
	closeCh := make(chan struct{})

	// ========================================
	// 6. サービスインスタンスの作成
	// ========================================
	service := &CogneeService{
		VectorStorage: vectorStorage,
		GraphStorage:  graphStorage,
		Embedder:      embedder,
		LLM:           completionLLM,
		Config:        config,
		S3Client:      s3Client,
		closeCh:       closeCh,
	}

	// バックグラウンドでダウンロードキャッシュのクリーンアップを実行
	go func() {
		interval := time.Duration(config.S3CleanupIntervalMinutes) * time.Minute
		retention := time.Duration(config.S3RetentionHours) * time.Hour

		ticker := time.NewTicker(interval)
		defer ticker.Stop()

		// 初回実行
		if err := service.S3Client.CleanupDownDir(retention); err != nil {
			fmt.Printf("Warning: Failed to cleanup S3 download cache: %v\n", err)
		}

		for {
			select {
			case <-service.closeCh:
				return // サービス終了時にループを抜ける
			case <-ticker.C:
				if err := service.S3Client.CleanupDownDir(retention); err != nil {
					fmt.Printf("Warning: Failed to cleanup S3 download cache: %v\n", err)
				}
			}
		}
	}()

	// ========================================
	// CogneeServiceインスタンスを返す
	// ========================================
	return service, nil
}

// Close は、CogneeServiceが保持するリソースを解放します。
// VectorStorageとGraphStorageの両方をクローズし、エラーがあれば収集して返します。
// defer service.Close() のように使用することで、リソースリークを防ぎます。
func (s *CogneeService) Close() error {
	var errs []error

	// バックグラウンド処理を停止
	if s.closeCh != nil {
		close(s.closeCh)
	}

	// VectorStorage（DuckDB）をクローズ
	if err := s.VectorStorage.Close(); err != nil {
		errs = append(errs, fmt.Errorf("failed to close VectorStorage: %w", err))
	}

	// GraphStorage（CozoDB）をクローズ
	if err := s.GraphStorage.Close(); err != nil {
		errs = append(errs, fmt.Errorf("failed to close GraphStorage: %w", err))
	}

	// エラーがあれば全て返す
	if len(errs) > 0 {
		return fmt.Errorf("errors closing service: %v", errs)
	}
	return nil
}

// loadDuckDBExtension は、DuckDBにVSS拡張をロードします。
// この関数は以下の処理を行います：
//  1. 実行環境のプラットフォーム（OS + アーキテクチャ）を判定
//  2. 対応するVSS拡張バイナリを選択
//  3. 一時ファイルとして書き出し
//  4. DuckDBにINSTALLとLOADコマンドを実行
//  5. 一時ファイルを削除
func loadDuckDBExtension(db *sql.DB) error {
	// 実行環境のプラットフォームを取得（例: "darwin-arm64", "linux-amd64"）
	platform := fmt.Sprintf("%s-%s", runtime.GOOS, runtime.GOARCH)

	var data []byte
	// プラットフォームに応じて適切なバイナリを選択
	switch platform {
	case "darwin-arm64":
		data = duckDbVssDarwinArm64 // macOS ARM64用
	case "linux-amd64":
		data = duckDbVssLinuxAmd64 // Linux AMD64用
	default:
		// サポートされていないプラットフォームの場合はエラー
		return fmt.Errorf("unsupported platform for VSS extension: %s", platform)
	}

	// 一時ディレクトリのパスを取得
	tmpDir := os.TempDir()
	// 一時ファイルのフルパスを構築
	extPath := filepath.Join(tmpDir, "vss.duckdb_extension")

	// バイナリデータを一時ファイルとして書き出し
	// 0755: 実行権限を付与
	if err := os.WriteFile(extPath, data, 0755); err != nil {
		return fmt.Errorf("failed to write VSS extension to temp file: %w", err)
	}
	// 関数終了時に一時ファイルを削除
	defer os.Remove(extPath)

	// DuckDBにINSTALLとLOADコマンドを実行
	// INSTALL: 拡張を登録
	// LOAD: 拡張を読み込んで有効化
	query := fmt.Sprintf("INSTALL '%s'; LOAD '%s';", extPath, extPath)
	if _, err := db.Exec(query); err != nil {
		return fmt.Errorf("failed to install/load extension: %w", err)
	}
	return nil
}

// Add は、ファイルをCogneeシステムに取り込みます。
// この関数は以下の処理を行います：
//  1. ユーザーとデータセットからグループIDを生成
//  2. IngestTaskを作成
//  3. パイプラインを実行してファイルを取り込む
//
// 取り込まれたファイルは、DuckDBのdataテーブルに保存されます。
// group_idによって、ユーザーとデータセットごとにデータが分離されます。
func (s *CogneeService) Add(ctx context.Context, filePaths []string, dataset string, user string) error {
	// グループIDを生成（例: "user1-test_dataset"）
	// このIDによって、データがパーティション分割されます
	groupID := user + "-" + dataset

	// ========================================
	// 1. タスクの作成
	// ========================================
	// IngestTaskを作成
	// このタスクは、ファイルを読み込んでDuckDBに保存します
	ingestTask := ingestion.NewIngestTask(s.VectorStorage, groupID, s.S3Client)

	// ========================================
	// 2. パイプラインの作成
	// ========================================
	// 単一のタスクからなるパイプラインを作成
	p := pipeline.NewPipeline([]pipeline.Task{ingestTask})

	// ========================================
	// 3. パイプラインの実行
	// ========================================
	// ファイルパスのリストを入力としてパイプラインを実行
	_, err := p.Run(ctx, filePaths)
	if err != nil {
		return fmt.Errorf("pipeline execution failed: %w", err)
	}

	return nil
}

// Cognify は、取り込まれたデータを処理して知識グラフを構築します。
// この関数は以下の処理を行います：
//  1. ユーザーとデータセットからグループIDを生成
//  2. 各種タスク（チャンク化、グラフ抽出、ストレージ、要約）を作成
//  3. パイプラインを実行してデータを処理
//
// 処理の流れ：
//
//	Data → Chunking → Graph Extraction → Storage → Summarization
//
// 最終的に、チャンク、グラフ、要約がそれぞれDuckDBとCozoDBに保存されます。
func (s *CogneeService) Cognify(ctx context.Context, dataset string, user string) error {
	// グループIDを生成
	groupID := user + "-" + dataset

	// ========================================
	// 1. 事前初期化されたLLMを使用
	// ========================================
	// s.LLM は NewCogneeService で初期化済み

	// ========================================
	// 2. タスクの初期化
	// ========================================

	// ChunkingTask: テキストを1024トークンのチャンクに分割
	// 20トークンのオーバーラップを設定
	chunkingTask, err := chunking.NewChunkingTask(1024, 20, s.VectorStorage, s.Embedder, s.S3Client)
	if err != nil {
		return fmt.Errorf("failed to initialize ChunkingTask: %w", err)
	}

	// GraphExtractionTask: LLMを使用してテキストからエンティティと関係を抽出
	graphTask := graph.NewGraphExtractionTask(s.LLM)

	// StorageTask: チャンクとグラフをデータベースに保存
	storageTask := storageTaskPkg.NewStorageTask(s.VectorStorage, s.GraphStorage, s.Embedder, groupID)

	// SummarizationTask: チャンクの要約を生成（Phase 4で追加）
	summarizationTask := summarization.NewSummarizationTask(s.VectorStorage, s.LLM, s.Embedder, groupID)

	// ========================================
	// 3. パイプラインの作成
	// ========================================
	// 4つのタスクを順番に実行するパイプラインを作成
	p := pipeline.NewPipeline([]pipeline.Task{
		chunkingTask,      // 1. チャンク化
		graphTask,         // 2. グラフ抽出
		storageTask,       // 3. ストレージ
		summarizationTask, // 4. 要約
	})

	// ========================================
	// 4. 入力データの取得
	// ========================================
	// グループIDでフィルタリングされたデータリストを取得
	// このデータは Add() で取り込まれたものです
	dataList, err := s.VectorStorage.GetDataList(ctx, groupID)
	if err != nil {
		return fmt.Errorf("failed to fetch input data: %w", err)
	}

	// データが存在しない場合は処理をスキップ
	if len(dataList) == 0 {
		fmt.Println("No data to process for this group.")
		return nil
	}

	// ========================================
	// 5. パイプラインの実行
	// ========================================
	// データリストを入力としてパイプラインを実行
	_, err = p.Run(ctx, dataList)
	if err != nil {
		return fmt.Errorf("cognify pipeline failed: %w", err)
	}

	return nil
}

// Search は、クエリに基づいて知識グラフを検索し、回答を生成します。
// この関数は以下の処理を行います：
//  1. ユーザーとデータセットからグループIDを生成
//  2. GraphCompletionToolを作成
//  3. 検索を実行して回答を取得
//
// 検索タイプに応じて、以下の処理が行われます：
//   - SUMMARIES: 要約のみを検索
//   - GRAPH_SUMMARY_COMPLETION: グラフを検索して要約を生成
//   - GRAPH_COMPLETION: グラフとチャンクを検索して回答を生成
func (s *CogneeService) Search(ctx context.Context, query string, searchType search.SearchType, dataset string, user string) (string, error) {
	// グループIDを生成
	groupID := user + "-" + dataset

	// ========================================
	// 1. 検索ツールの作成
	// ========================================
	// 事前初期化されたLLM（s.LLM）を使用
	searchTool := search.NewGraphCompletionTool(s.VectorStorage, s.GraphStorage, s.LLM, s.Embedder, groupID)

	// ========================================
	// 2. 検索の実行
	// ========================================
	// 検索タイプに応じて適切な検索処理が実行されます
	return searchTool.Search(ctx, query, searchType)
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

// Memify は、既存の知識グラフに対して強化処理を適用します。
// 設定に応じて、Unknown解決（Phase A）と知識グラフ拡張（Phase B, 再帰的）を実行します。
//
// 引数:
//   - ctx: コンテキスト
//   - dataset: データセット名
//   - user: ユーザー名
//   - config: Memify設定（再帰深度やUnknown優先フラグを含む）
//
// 返り値:
//   - error: エラーが発生した場合
func (s *CogneeService) Memify(ctx context.Context, dataset string, user string, config *MemifyConfig) error {
	if config == nil {
		config = &MemifyConfig{RecursiveDepth: 0, PrioritizeUnknowns: true}
	}

	groupID := user + "-" + dataset
	if config.RulesNodeSetName == "" {
		config.RulesNodeSetName = "coding_agent_rules"
	}

	// ========================================
	// Phase A: Unknown解決フェーズ (Priority High)
	// ========================================
	if config.PrioritizeUnknowns {
		fmt.Println("Memify: Phase A - Prioritizing Unknown Resolution")
		ignoranceManager := metacognition.NewIgnoranceManager(
			s.VectorStorage, s.GraphStorage, s.LLM, s.Embedder, groupID,
			s.Config.MetaSimilarityThresholdUnknown,
			s.Config.MetaSearchLimitUnknown,
		)

		// 1. 未解決のUnknownを取得
		unknowns, err := ignoranceManager.GetUnresolvedUnknowns(ctx)
		if err != nil {
			fmt.Printf("Warning: Failed to get unresolved unknowns: %v\n", err)
		} else {
			for _, unknown := range unknowns {
				// 2. 各Unknownについて、解決のための自問自答と検索を集中的に行う
				if err := s.attemptToResolveUnknown(ctx, unknown, groupID); err != nil {
					fmt.Printf("Failed to resolve unknown %s: %v\n", unknown.ID, err)
				}
			}
		}
	}

	// ========================================
	// Phase B: 全体グラフ拡張フェーズ (Priority Normal)
	// ========================================
	fmt.Println("Memify: Phase B - Graph Expansion")

	// 再帰的にコアロジックを実行
	// RecursiveDepth=0 の場合は1回のみ実行 (level 0 <= 0 for 1 iteration)
	for level := 0; level <= config.RecursiveDepth; level++ {
		fmt.Printf("Memify: Level %d / %d\n", level, config.RecursiveDepth)

		if err := s.executeMemifyCore(ctx, dataset, user, config); err != nil {
			return fmt.Errorf("Memify execution failed at level %d: %w", level, err)
		}
	}

	return nil
}

// memifyBulkProcess は、全テキストを一括で処理します。
// Python版の memify と同等の精度を保証します。
func (s *CogneeService) memifyBulkProcess(
	ctx context.Context,
	texts []string,
	groupID string,
	rulesNodeSetName string,
) error {
	// ルール抽出タスクを作成
	task := memify.NewRuleExtractionTask(
		s.VectorStorage,
		s.GraphStorage,
		s.LLM,
		s.Embedder,
		groupID,
		rulesNodeSetName,
	)

	// 全テキストを1つのバッチとして処理
	if err := task.ProcessBatch(ctx, texts); err != nil {
		return fmt.Errorf("bulk processing failed: %w", err)
	}

	return nil
}

// memifyBatchProcess は、テキストをバッチ分割して処理します。
// 大規模データに対応し、メモリ使用量を抑制します。
func (s *CogneeService) memifyBatchProcess(
	ctx context.Context,
	texts []string,
	groupID string,
	rulesNodeSetName string,
	batchCharSize int,
	overlapPercent int,
) error {
	// ルール抽出タスクを作成
	task := memify.NewRuleExtractionTask(
		s.VectorStorage,
		s.GraphStorage,
		s.LLM,
		s.Embedder,
		groupID,
		rulesNodeSetName,
	)

	// 1. 全テキストを結合
	combinedText := strings.Join(texts, "\n\n")

	// 2. 日本語自然境界 + オーバーラップで分割
	fmt.Printf("Memify [BATCH]: Processing with batch size ~%d chars, overlap %d%%\n", batchCharSize, overlapPercent)
	batches := memify.SplitTextWithOverlap(combinedText, batchCharSize, overlapPercent)

	fmt.Printf("Memify [BATCH]: Split into %d batches with natural boundaries\n", len(batches))

	// 3. 各バッチを処理
	for i, batch := range batches {
		fmt.Printf("Memify [BATCH]: Processing batch %d/%d (%d chars)\n", i+1, len(batches), memify.CountUTF8Chars(batch))

		// 1バッチ分のスライスを作成して渡す
		batchSlice := []string{batch}
		if err := task.ProcessBatch(ctx, batchSlice); err != nil {
			return fmt.Errorf("batch processing failed (batch %d): %w", i+1, err)
		}
	}

	return nil
}

// RecursiveMemify は廃止されました。代わりに Memify を使用してください。
// config.RecursiveDepth を設定することで同様の動作を実現できます。
//
// executeMemifyCore は、Memifyのコアロジック（チャンク収集〜ルール抽出）を実行します。
// これは再帰ループの各反復で呼び出される1回分の処理単位です。
func (s *CogneeService) executeMemifyCore(ctx context.Context, dataset string, user string, config *MemifyConfig) error {
	groupID := user + "-" + dataset
	if config.RulesNodeSetName == "" {
		config.RulesNodeSetName = "coding_agent_rules"
	}

	fmt.Printf("Starting Memify Core Execution for group: %s\n", groupID)

	// ========================================
	// 1. DocumentChunk のテキストを収集
	// ========================================
	chunkChan, errChan := s.GraphStorage.StreamDocumentChunks(ctx, groupID)
	var texts []string

loop:
	for {
		select {
		case <-ctx.Done():
			return ctx.Err()
		case err := <-errChan:
			if err != nil {
				return fmt.Errorf("failed to stream chunks: %w", err)
			}
		case chunk, ok := <-chunkChan:
			if !ok {
				break loop
			}
			texts = append(texts, chunk.Text)
		}
	}

	if len(texts) == 0 {
		fmt.Println("Memify Core: No DocumentChunks found. Skipping.")
		return nil
	}

	// ========================================
	// 2. ハイブリッド処理の判定
	// ========================================
	totalCharCount := memify.CountTotalUTF8Chars(texts)
	maxCharsForBulk := s.Config.MemifyMaxCharsForBulkProcess

	fmt.Printf("Memify Core: Total chars: %d, Threshold: %d\n", totalCharCount, maxCharsForBulk)

	if totalCharCount <= maxCharsForBulk {
		fmt.Println("Memify Core: Using BULK processing")
		return s.memifyBulkProcess(ctx, texts, groupID, config.RulesNodeSetName)
	} else {
		fmt.Println("Memify Core: Using BATCH processing")
		batchCharSize := max(maxCharsForBulk/5, s.Config.MemifyBatchMinChars)
		overlapPercent := max(s.Config.MemifyBatchOverlapPercent, 0)
		return s.memifyBatchProcess(ctx, texts, groupID, config.RulesNodeSetName, batchCharSize, overlapPercent)
	}
}

// attemptToResolveUnknown は、特定のUnknownを解決するためにリソースを集中させます。
func (s *CogneeService) attemptToResolveUnknown(ctx context.Context, unknown *metacognition.Unknown, groupID string) error {
	// 1. SelfReflectionTask を初期化
	task := metacognition.NewSelfReflectionTask(
		s.VectorStorage, s.GraphStorage, s.LLM, s.Embedder, groupID,
		s.Config.MetaSimilarityThresholdReflection,
		s.Config.MetaSearchLimitReflectionChunk,
		s.Config.MetaSearchLimitReflectionRule,
		s.Config.MetaSimilarityThresholdUnknown,
		s.Config.MetaSearchLimitUnknown,
	)

	// 2. Unknownのテキストを「問い」として解決を試みる
	// SelfReflectionTask.TryAnswer を使用
	answered, insight, err := task.TryAnswer(ctx, unknown.Text)
	if err != nil {
		return fmt.Errorf("TryAnswer failed: %w", err)
	}

	if answered {
		fmt.Printf("Resolved Unknown: %s\nInsight: %s\n", unknown.Text, insight)
		// 3. 解決できた場合は Capability を登録し、Unknown を解決済みとする
		if err := task.IgnoranceManager.RegisterCapability(
			ctx,
			insight,
			[]string{"recursive_memify", "unknown_resolution"},
			[]string{""},
			[]string{"self_reflection"},
			[]string{unknown.ID}, // resolvedUnknownID
		); err != nil {
			return fmt.Errorf("failed to register capability: %w", err)
		}
	} else {
		fmt.Printf("Could not resolve Unknown: %s\n", unknown.Text)
	}

	return nil
}
