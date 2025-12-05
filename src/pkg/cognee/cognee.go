// Package cognee は、Cognee サービスのコアとなるパッケージです。
// このパッケージは、データの取り込み(Add)、知識グラフ化(Cognify)、検索(Search)の
// 3つの主要機能を提供します。
package cognee

import (
	"context"
	"database/sql"
	_ "embed"
	"fmt"
	"os"
	"path/filepath"
	"runtime"

	"mycute/pkg/cognee/db/cozodb"
	duckdbRepo "mycute/pkg/cognee/db/duckdb"
	"mycute/pkg/cognee/pipeline"
	"mycute/pkg/cognee/storage"
	"mycute/pkg/cognee/tasks/chunking"
	"mycute/pkg/cognee/tasks/graph"
	"mycute/pkg/cognee/tasks/ingestion"
	storageTaskPkg "mycute/pkg/cognee/tasks/storage"
	"mycute/pkg/cognee/tasks/summarization"
	"mycute/pkg/cognee/tools/search"

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

	// Completion (テキスト生成) LLM の設定
	CompletionAPIKey  string // APIキー（必須）
	CompletionBaseURL string // ベースURL（オプション、Bifrostプロキシ等で使用）
	CompletionModel   string // モデル名（オプション、例: gpt-4o）

	// Embeddings (ベクトル化) LLM の設定
	EmbeddingsAPIKey  string // APIキー（必須）
	EmbeddingsBaseURL string // ベースURL（オプション、Bifrostプロキシ等で使用）
	EmbeddingsModel   string // モデル名（オプション、例: text-embedding-3-small）
}

// CogneeService は、Cogneeの主要な機能を提供するサービス構造体です。
// データベース接続とLLMクライアントを内部で保持し、ライフサイクルを管理します。
type CogneeService struct {
	VectorStorage storage.VectorStorage // ベクトルストレージ（DuckDB）
	GraphStorage  storage.GraphStorage  // グラフストレージ（CozoDB）
	Embedder      storage.Embedder      // テキストのベクトル化を行うEmbedder
	LLM           llms.Model            // テキスト生成を行うLLM
}

// NewCogneeService は、CogneeServiceの新しいインスタンスを作成します。
// この関数は以下の処理を順番に実行します：
//  1. DuckDBとCozoDBのファイルパスを構築
//  2. DuckDBを初期化し、VSS拡張をロード、スキーマを適用
//  3. CozoDBを初期化し、スキーマを適用
//  4. Embeddings用のLLMクライアントを初期化
//  5. Completion用のLLMクライアントを初期化
//
// エラーが発生した場合は、それまでに開いたリソースをクリーンアップしてからエラーを返します。
func NewCogneeService(config CogneeConfig) (*CogneeService, error) {
	// データベースファイルのフルパスを構築
	// 例: DBDirPath="/data", DBName="cognee" の場合
	//     duckDBPath="/data/cognee.duckdb"
	//     cozoDBPath="/data/cognee.cozodb"
	duckDBPath := filepath.Join(config.DBDirPath, config.DBName+".duckdb")
	cozoDBPath := filepath.Join(config.DBDirPath, config.DBName+".cozodb")

	// ========================================
	// 1. DuckDB の初期化
	// ========================================
	// DuckDBへの接続を開く
	// access_mode=READ_WRITE: 読み書き両方を許可
	// hnsw_enable_experimental_persistence=true: HNSW インデックスの永続化を有効化
	duckDBConn, err := sql.Open("duckdb", fmt.Sprintf("%s?access_mode=READ_WRITE&hnsw_enable_experimental_persistence=true", duckDBPath))
	if err != nil {
		return nil, fmt.Errorf("failed to open DuckDB: %w", err)
	}

	// VSS（Vector Similarity Search）拡張をロード
	// この拡張により、ベクトル検索機能が利用可能になります
	if err := loadDuckDBExtension(duckDBConn); err != nil {
		duckDBConn.Close() // エラー時はリソースをクリーンアップ
		return nil, fmt.Errorf("failed to load VSS extension: %w", err)
	}

	// データベーススキーマを適用
	// 埋め込まれたschema.sqlを実行してテーブルを作成します
	if _, err := duckDBConn.Exec(duckDBSchema); err != nil {
		duckDBConn.Close() // エラー時はリソースをクリーンアップ
		return nil, fmt.Errorf("failed to apply DuckDB schema: %w", err)
	}

	// DuckDBStorageインスタンスを作成
	// このインスタンスを通じてDuckDBへのデータ操作を行います
	duckStorage := duckdbRepo.NewDuckDBStorage(duckDBConn)

	// ========================================
	// 2. CozoDB の初期化
	// ========================================
	// CozoDBへの接続を開く
	// "rocksdb": RocksDBバックエンドを使用（永続化対応）
	cozodbInstance, err := cozo.New("rocksdb", cozoDBPath, nil)
	if err != nil {
		duckDBConn.Close() // エラー時は既に開いたDuckDBをクリーンアップ
		return nil, fmt.Errorf("failed to open CozoDB: %w", err)
	}

	// CozoStorageインスタンスを作成
	cozoStorage := cozodb.NewCozoStorage(&cozodbInstance)

	// グラフデータベースのスキーマを適用
	// nodesとedgesのリレーションを作成します
	if err := cozoStorage.EnsureSchema(context.Background()); err != nil {
		cozodbInstance.Close() // エラー時はCozoDBをクリーンアップ
		duckDBConn.Close()     // エラー時はDuckDBもクリーンアップ
		return nil, fmt.Errorf("failed to apply CozoDB schema: %w", err)
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
		cozodbInstance.Close() // エラー時はCozoDBをクリーンアップ
		duckDBConn.Close()     // エラー時はDuckDBをクリーンアップ
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
		cozodbInstance.Close() // エラー時はCozoDBをクリーンアップ
		duckDBConn.Close()     // エラー時はDuckDBをクリーンアップ
		return nil, fmt.Errorf("failed to initialize Completion LLM: %w", err)
	}

	// ========================================
	// CogneeServiceインスタンスを返す
	// ========================================
	return &CogneeService{
		VectorStorage: duckStorage,   // ベクトルストレージ
		GraphStorage:  cozoStorage,   // グラフストレージ
		Embedder:      embedder,      // Embedder
		LLM:           completionLLM, // Completion LLM
	}, nil
}

// Close は、CogneeServiceが保持するリソースを解放します。
// VectorStorageとGraphStorageの両方をクローズし、エラーがあれば収集して返します。
// defer service.Close() のように使用することで、リソースリークを防ぎます。
func (s *CogneeService) Close() error {
	var errs []error

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
	ingestTask := ingestion.NewIngestTask(s.VectorStorage, groupID)

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
	chunkingTask, err := chunking.NewChunkingTask(1024, 20, s.VectorStorage, s.Embedder)
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
