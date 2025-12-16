// Package cuber は、Cuber サービスのコアとなるパッケージです。
// このパッケージは、データの取り込み(Add)、知識グラフ化(Cognify)、検索(Search)の
// 3つの主要機能を提供します。
package cuber

import (
	"archive/zip"
	"bytes"
	"context"
	"fmt"
	"log"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"time"

	"github.com/cloudwego/eino/components/model"
	"github.com/t-kawata/mycute/pkg/cuber/db/kuzudb"
	"github.com/t-kawata/mycute/pkg/cuber/pipeline"
	"github.com/t-kawata/mycute/pkg/cuber/providers"
	"github.com/t-kawata/mycute/pkg/cuber/storage"
	"github.com/t-kawata/mycute/pkg/cuber/tasks/chunking"
	"github.com/t-kawata/mycute/pkg/cuber/tasks/graph"
	"github.com/t-kawata/mycute/pkg/cuber/tasks/ingestion"
	"github.com/t-kawata/mycute/pkg/cuber/tasks/memify"
	"github.com/t-kawata/mycute/pkg/cuber/tasks/metacognition"
	storageTaskPkg "github.com/t-kawata/mycute/pkg/cuber/tasks/storage"
	"github.com/t-kawata/mycute/pkg/cuber/tasks/summarization"
	"github.com/t-kawata/mycute/pkg/cuber/tools/query"
	"github.com/t-kawata/mycute/pkg/cuber/types"
	"github.com/t-kawata/mycute/pkg/s3client"
)

// CuberConfig は、Cuberサービスの初期化に必要な設定を保持する構造体です。
// データベースの配置場所とLLMプロバイダーの接続情報を含みます。
type CuberConfig struct {
	// データベースファイルを格納するディレクトリのパス
	DBDirPath string

	// KuzuDB Configuration
	KuzuDBDatabasePath string // Path to KuzuDB database file (if different from default)

	// Completion (テキスト生成) LLM の設定
	CompletionAPIKey    string // APIキー（必須）
	CompletionBaseURL   string // ベースURL（オプション、Bifrostプロキシ等で使用）
	CompletionModel     string // モデル名（オプション、例: gpt-4o）
	CompletionMaxTokens int    // 最大生成トークン数（0の場合はデフォルトを使用）

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
}

type StorageSet struct {
	Vector     storage.VectorStorage // ベクトルストレージ（KuzuDB）
	Graph      storage.GraphStorage  // グラフストレージ（KuzuDB）
	LastUsedAt time.Time
	mu         sync.Mutex // 個別のStorageSetへのアクセス保護
}

// CuberService は、Cuberの主要な機能を提供するサービス構造体です。
// データベース接続とLLMクライアントを内部で保持し、ライフサイクルを管理します。
type CuberService struct {
	StorageMap map[string]*StorageSet     // マップのキーは、model.Cube.UUID となる
	mu         sync.RWMutex               // StorageMapへのアクセス保護
	Embedder   storage.Embedder           // テキストのベクトル化を行うEmbedder
	LLM        model.ToolCallingChatModel // テキスト生成を行うLLM (Eino)
	Config     CuberConfig                // 設定値を保持
	S3Client   *s3client.S3Client         // S3クライアント（ローカル/S3両対応）
	closeCh    chan struct{}              // サービス終了通知用チャネル
}

// NewCuberService は、CuberServiceの新しいインスタンスを作成します。
// この関数は以下の処理を順番に実行します：
//  1. KuzuDBのファイルパスを構築
//  2. KuzuDBを初期化し、スキーマを適用
//  4. Embeddings用のLLMクライアントを初期化
//  5. Completion用のLLMクライアントを初期化
//  6. S3Clientを初期化
//
// エラーが発生した場合は、それまでに開いたリソースをクリーンアップしてからエラーを返します。
// NewCuberService は、CuberServiceの新しいインスタンスを作成します。
func NewCuberService(config CuberConfig) (*CuberService, error) {
	// ========================================
	// 0. 設定のデフォルト値を適用
	// ========================================
	// Storage Idle Timeout
	if config.StorageIdleTimeoutMinutes == 0 {
		config.StorageIdleTimeoutMinutes = 60
	}
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
	cleanupFunc := func() {
		// Nothing to close yet for LLMs, but if we opened files, close here.
	}
	// ========================================
	// 3. Embeddings LLM の初期化 (Eino Factory使用)
	// ========================================
	ctx := context.Background()
	embConfig := providers.ProviderConfig{
		Type:      providers.ProviderOpenAI, // TODO: Configから取得できるように拡張
		APIKey:    config.EmbeddingsAPIKey,
		BaseURL:   config.EmbeddingsBaseURL,
		ModelName: config.EmbeddingsModel,
	}

	einoRawEmb, err := providers.NewEmbedder(ctx, embConfig)
	if err != nil {
		cleanupFunc()
		return nil, fmt.Errorf("Failed to initialize Eino Embedder: %w", err)
	}
	// EinoEmbedderAdapterを作成
	// このアダプターを通じてテキストのベクトル化を行います
	embedder := query.NewEinoEmbedderAdapter(einoRawEmb, config.EmbeddingsModel)

	// ========================================
	// 4. Completion LLM の初期化 (Eino Factory使用)
	// ========================================
	chatConfig := providers.ProviderConfig{
		Type:      providers.ProviderOpenAI, // TODO: Configから取得できるように拡張
		APIKey:    config.CompletionAPIKey,
		BaseURL:   config.CompletionBaseURL,
		ModelName: config.CompletionModel,
		MaxTokens: config.CompletionMaxTokens,
	}

	chatModel, err := providers.NewChatModel(ctx, chatConfig)
	if err != nil {
		cleanupFunc()
		return nil, fmt.Errorf("Failed to initialize Eino ChatModel: %w", err)
	}

	// ========================================
	// 5. S3Client の初期化
	// ========================================
	s3Client, err := s3client.NewS3Client(
		config.S3AccessKey,
		config.S3SecretKey,
		config.S3Region,
		config.S3Bucket,
		config.S3LocalDir, // アップロード先（ローカルモード時）
		config.S3DLDir,    // ダウンロード先（キャッシュ）
		config.S3UseLocal,
	)
	if err != nil {
		cleanupFunc()
		return nil, fmt.Errorf("Failed to initialize S3Client: %w", err)
	}

	// サービス終了通知用チャネルの作成
	closeCh := make(chan struct{})

	// ========================================
	// 6. サービスインスタンスの作成
	// ========================================
	service := &CuberService{
		StorageMap: make(map[string]*StorageSet),
		Embedder:   embedder,
		LLM:        chatModel,
		Config:     config,
		S3Client:   s3Client,
		closeCh:    closeCh,
	}

	// Start StorageGC routine
	go service.startStorageGCRoutine()

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
	// CuberServiceインスタンスを返す
	// ========================================
	return service, nil
}

// Close は、CuberServiceが保持するリソースを解放します。
// VectorStorageとGraphStorageの両方をクローズし、エラーがあれば収集して返します。
// defer service.Close() のように使用することで、リソースリークを防ぎます。
func (s *CuberService) Close() error {
	var errs []error
	// バックグラウンド処理を停止
	if s.closeCh != nil {
		close(s.closeCh)
	}
	s.mu.Lock()
	defer s.mu.Unlock()

	for _, set := range s.StorageMap {
		if set.Vector.IsOpen() {
			if err := set.Vector.Close(); err != nil {
				errs = append(errs, fmt.Errorf("Failed to close VectorStorage: %w", err))
			}
		}
		if set.Graph.IsOpen() {
			if err := set.Graph.Close(); err != nil {
				errs = append(errs, fmt.Errorf("Failed to close GraphStorage: %w", err))
			}
		}
	}
	s.StorageMap = make(map[string]*StorageSet)
	if len(errs) > 0 {
		return fmt.Errorf("errors closing service: %v", errs)
	}
	return nil
}

// GetOrOpenStorage retrieves an existing storage set or opens a new one for the given Cube DB file path.
// cubeUUID is derived from the file path (basename without extension).
func (s *CuberService) GetOrOpenStorage(cubeDbFilePath string) (*StorageSet, error) {
	cubeUUID := getUUIDFromDBFilePath(cubeDbFilePath)
	s.mu.RLock()
	st, exists := s.StorageMap[cubeUUID]
	s.mu.RUnlock()
	if exists {
		st.mu.Lock()
		st.LastUsedAt = time.Now()
		st.mu.Unlock()
		return st, nil
	}
	s.mu.Lock()
	defer s.mu.Unlock()
	// Double check
	if st, exists = s.StorageMap[cubeUUID]; exists {
		st.mu.Lock()
		st.LastUsedAt = time.Now()
		st.mu.Unlock()
		return st, nil
	}
	// Open new storage
	// Ensure directory exists
	if err := os.MkdirAll(filepath.Dir(cubeDbFilePath), 0755); err != nil {
		return nil, fmt.Errorf("Failed to create db directory: %w", err)
	}
	// Initialize KuzuDB (Vector and Graph share the same DB path for now in this context,
	// assuming kuzudb.NewKuzuDBStorage returns an object implementing both interfaces or capable of both)
	// Note: The original code used kuzuSt for both. We assume NewKuzuDBStorage returns *KuzuDBStorage which implements both.
	kuzuSt, err := kuzudb.NewKuzuDBStorage(cubeDbFilePath)
	if err != nil {
		return nil, fmt.Errorf("Failed to open KuzuDB at %s: %w", cubeDbFilePath, err)
	}
	// Ensure schema (lazy init)
	if err := kuzuSt.EnsureSchema(context.Background()); err != nil {
		kuzuSt.Close()
		return nil, fmt.Errorf("Failed to ensure schema: %w", err)
	}
	newSet := &StorageSet{
		Vector:     kuzuSt,
		Graph:      kuzuSt,
		LastUsedAt: time.Now(),
	}
	s.StorageMap[cubeUUID] = newSet
	return newSet, nil
}

// startStorageGCRoutine periodically checks for idle storage connections and closes them.
func (s *CuberService) startStorageGCRoutine() {
	ticker := time.NewTicker(1 * time.Minute) // Check every minute
	defer ticker.Stop()
	for {
		select {
		case <-s.closeCh:
			return
		case <-ticker.C:
			s.cleanupIdleStorages()
		}
	}
}

// cleanupIdleStorages closes and removes storage sets that haven't been used for the configured duration.
func (s *CuberService) cleanupIdleStorages() {
	timeout := time.Duration(s.Config.StorageIdleTimeoutMinutes) * time.Minute
	now := time.Now()
	s.mu.Lock()
	defer s.mu.Unlock()
	for uuid, st := range s.StorageMap {
		st.mu.Lock()
		idleTime := now.Sub(st.LastUsedAt)
		st.mu.Unlock()
		if idleTime > timeout {
			fmt.Printf("[CuberService] Closing idle storage for Cube %s (Idle: %v)\n", uuid, idleTime)
			// Close VectorStorage
			if st.Vector.IsOpen() {
				if err := st.Vector.Close(); err != nil {
					fmt.Printf("[CuberService] Warning: Error closing vector storage for %s: %v\n", uuid, err)
				}
			}
			// Close GraphStorage
			if st.Graph.IsOpen() {
				if err := st.Graph.Close(); err != nil {
					fmt.Printf("[CuberService] Warning: Error closing graph storage for %s: %v\n", uuid, err)
				}
			}
			delete(s.StorageMap, uuid)
		}
	}
}

// CreateCubeDB は、新しい空の Cube データベースを初期化します。
// この関数は、指定されたパスに KuzuDB データベースを作成し、スキーマを適用します。
//
// 引数:
//   - dbFilePath: KuzuDB データベースのパス
//
// 返り値:
//   - error: エラーが発生した場合
func CreateCubeDB(dbFilePath string) error {
	// 親ディレクトリの作成
	parentDir := filepath.Dir(dbFilePath)
	if err := os.MkdirAll(parentDir, 0755); err != nil {
		return fmt.Errorf("CreateCubeDB: failed to create parent directory: %w", err)
	}
	// KuzuDB を初期化
	kuzuSt, err := kuzudb.NewKuzuDBStorage(dbFilePath)
	if err != nil {
		return fmt.Errorf("CreateCubeDB: failed to create KuzuDBStorage: %w", err)
	}
	defer kuzuSt.Close()
	// スキーマ適用
	if err := kuzuSt.EnsureSchema(context.Background()); err != nil {
		return fmt.Errorf("CreateCubeDB: failed to apply schema: %w", err)
	}
	log.Printf("[Cuber] Created new Cube at %s", dbFilePath)
	return nil
}

// getUUIDFromDBFilePath は、CubeのDBファイルパスからUUIDを抽出します。
// 例: "/path/to/apxID-vdrID-usrID/uuid.db" → "uuid"
func getUUIDFromDBFilePath(cubeDbFilePath string) string {
	baseName := filepath.Base(cubeDbFilePath)
	return strings.TrimSuffix(baseName, filepath.Ext(baseName))
}

// この関数は、従来のAdd → Cognifyの2ステップを1つの操作に統合します。
//
// 処理の流れ:
//  1. ファイルをS3/ローカルストレージに保存（add）
//  2. テキストをチャンク化し、グラフを抽出（cognify）
//  3. 知識グラフをKuzuDBに保存
//  4. 処理済みファイルを自動削除（クリーンアップ）
//
// 使用例:
//
//	svc.Absorb(ctx, "path/to/cube.db", "legal_expert", []string{"doc.txt"})
//	svc.Query(ctx, "path/to/cube.db", "legal_expert", search.SearchTypeGraphCompletion, "質問")
//
// 引数:
//   - ctx: コンテキスト
//   - cubeDbFilePath: CubeのDBファイルパス
//   - memoryGroup: メモリグループ名（KuzuDB内のmemory_groupとして使用される）
//   - filePaths: 取り込むファイルパスのリスト
//
// 返り値:
//   - types.TokenUsage: トークン使用量
//   - error: エラーが発生した場合
func (s *CuberService) Absorb(ctx context.Context, cubeDbFilePath string, memoryGroup string, filePaths []string, cognifyConfig CognifyConfig) (types.TokenUsage, error) {
	var totalUsage types.TokenUsage
	cubeUUID := getUUIDFromDBFilePath(cubeDbFilePath)
	// Ensure storage is ready
	_, err := s.GetOrOpenStorage(cubeDbFilePath)
	if err != nil {
		return totalUsage, fmt.Errorf("Absorb: Failed to open storage for cube %s: %w", cubeUUID, err)
	}
	// 1. ファイルの取り込み（add）
	usage1, err := s.add(ctx, cubeDbFilePath, memoryGroup, filePaths)
	totalUsage.Add(usage1)
	if err != nil {
		return totalUsage, fmt.Errorf("Absorb: Add phase failed: %w", err)
	}
	// 2. 知識グラフの構築（cognify）
	usage2, err := s.cognify(ctx, cubeDbFilePath, memoryGroup, cognifyConfig)
	totalUsage.Add(usage2)
	if err != nil {
		return totalUsage, fmt.Errorf("Absorb: Cognify phase failed: %w", err)
	}
	return totalUsage, nil
}

// add は、ファイルをCuberシステムに取り込む内部メソッドです。
// この関数は以下の処理を行います：
//  1. ユーザーとデータセットからメモリーグループを生成
//  2. IngestTaskを作成
//  3. パイプラインを実行してファイルを取り込む
//
// 取り込まれたファイルは、KuzuDBのdataテーブルに保存されます。
// memory_groupによって、ユーザーとデータセットごとにデータが分離されます。
//
// 注意: このメソッドはパッケージ内部で使用されます。外部からはAbsorbを使用してください。
//
// 引数:
//   - ctx: コンテキスト
//   - cubeDbFilePath: CubeのDBファイルパス
//   - memoryGroup: メモリグループ名（memory_groupとして使用）
//   - filePaths: 取り込むファイルパスのリスト
//
// 返り値:
//   - types.TokenUsage: トークン使用量
//   - error: エラーが発生した場合
func (s *CuberService) add(ctx context.Context, cubeDbFilePath string, memoryGroup string, filePaths []string) (types.TokenUsage, error) {
	var usage types.TokenUsage
	// Storage retrieval
	st, err := s.GetOrOpenStorage(cubeDbFilePath)
	if err != nil {
		return usage, fmt.Errorf("Add: Failed to get storage: %w", err)
	}
	// ========================================
	// 1. タスクの作成
	// ========================================
	// IngestTaskを作成
	// このタスクは、ファイルを読み込んでKuzuDBに保存します
	ingestTask := ingestion.NewIngestTask(st.Vector, memoryGroup, s.S3Client)
	// ========================================
	// 2. パイプラインの作成
	// ========================================
	// 単一のタスクからなるパイプラインを作成
	p := pipeline.NewPipeline([]pipeline.Task{ingestTask})
	// ========================================
	// 3. パイプラインの実行
	// ========================================
	// ファイルパスのリストを入力としてパイプラインを実行
	_, usage, err = p.Run(ctx, filePaths)
	if err != nil {
		return usage, fmt.Errorf("Add: Pipeline execution failed: %w", err)
	}
	return usage, nil
}

// CognifyConfig は、cognifyの設定を表す構造体です。
type CognifyConfig struct {
	ChunkSize    int // チャンクのサイズとなる文字数（トークン数でカウントするとユーザーが使いにくいのでやめた）
	ChunkOverlap int // チャンクのオーバーラップとなる文字数（トークン数でカウントするとユーザーが使いにくいのでやめた）
}

// cognify は、取り込まれたデータを処理して知識グラフを構築する内部メソッドです。
// この関数は以下の処理を行います：
//  1. ユーザーとデータセットからメモリーグループを生成
//  2. 各種タスク（チャンク化、グラフ抽出、ストレージ、要約）を作成
//  3. パイプラインを実行してデータを処理
//  4. 処理済みファイルをクリーンアップ
//
// 処理の流れ:
//
//	Data → Chunking → Graph Extraction → Storage → Summarization → Cleanup
//
// 最終的に、チャンク、グラフ、要約がKuzuDBに保存されます。
//
// 注意: このメソッドはパッケージ内部で使用されます。外部からはAbsorbを使用してください。
//
// 引数:
//   - ctx: コンテキスト
//   - cubeDbFilePath: CubeのDBファイルパス
//   - memoryGroup: メモリグループ名（memory_groupとして使用）
//   - config: CognifyConfig構造体
//
// 返り値:
//   - types.TokenUsage: トークン使用量
//   - error: エラーが発生した場合
func (s *CuberService) cognify(ctx context.Context, cubeDbFilePath string, memoryGroup string, config CognifyConfig) (types.TokenUsage, error) {
	var usage types.TokenUsage
	// Storage retrieval
	st, err := s.GetOrOpenStorage(cubeDbFilePath)
	if err != nil {
		return usage, fmt.Errorf("Cognify: Failed to get storage: %w", err)
	}
	// ========================================
	// 1. タスクの初期化
	// ========================================
	// ChunkingTask: テキストをconfig.ChunkSize文字のチャンクに分割
	// config.ChunkOverlap文字のオーバーラップを設定
	chunkingTask, err := chunking.NewChunkingTask(config.ChunkSize, config.ChunkOverlap, st.Vector, s.Embedder, s.S3Client)
	if err != nil {
		return usage, fmt.Errorf("Cognify: Failed to initialize ChunkingTask: %w", err)
	}
	// GraphExtractionTask: LLMを使用してテキストからエンティティと関係を抽出
	graphTask := graph.NewGraphExtractionTask(s.LLM, s.Config.CompletionModel, memoryGroup)
	// StorageTask: チャンクとグラフをデータベースに保存
	storageTask := storageTaskPkg.NewStorageTask(st.Vector, st.Graph, s.Embedder, memoryGroup)
	// SummarizationTask: チャンクの要約を生成
	summarizationTask := summarization.NewSummarizationTask(st.Vector, s.LLM, s.Embedder, memoryGroup, s.Config.CompletionModel)
	// ========================================
	// 2. パイプラインの作成
	// ========================================
	// 4つのタスクを順番に実行するパイプラインを作成
	p := pipeline.NewPipeline([]pipeline.Task{
		chunkingTask,      // 1. チャンク化
		graphTask,         // 2. グラフ抽出
		storageTask,       // 3. ストレージ
		summarizationTask, // 4. 要約
	})
	// ========================================
	// 3. 入力データの取得
	// ========================================
	// メモリーグループでフィルタリングされたデータリストを取得
	// このデータは Add() で取り込まれたものです
	dataList, err := st.Vector.GetDataList(ctx, memoryGroup)
	if err != nil {
		return usage, fmt.Errorf("Cognify: Failed to fetch input data: %w", err)
	}
	// データが存在しない場合は処理をスキップ
	if len(dataList) == 0 {
		fmt.Println("No data to process for this group.")
		return usage, nil
	}
	// ========================================
	// 4. パイプラインの実行
	// ========================================
	// データリストを入力としてパイプラインを実行
	_, usage, err = p.Run(ctx, dataList)
	if err != nil {
		return usage, fmt.Errorf("Cognify: Pipeline execution failed: %w", err)
	}
	// ========================================
	// 5. ファイルのクリーンアップ
	// ========================================
	// Cognify成功後、S3Clientで保存されたファイルを削除
	// これにより、再利用されないファイルがストレージに残るのを防ぐ
	for _, data := range dataList {
		// S3Clientで保存されたファイル（RawDataLocation）を削除
		// OriginalDataLocationは現在使用されていないため、チェック不要
		if data.RawDataLocation != "" {
			if err := s.S3Client.Del(data.RawDataLocation); err != nil {
				log.Printf("Warning: Failed to delete file %s: %v", data.RawDataLocation, err)
			} else {
				log.Printf("Deleted file: %s", data.RawDataLocation)
			}
		}
	}
	return usage, nil
}

// Query は、クエリ（質問）に基づいて知識グラフを検索し、回答を生成します。
//
// クエリタイプに応じて、以下の処理が行われます：
//   - SUMMARIES: 要約のみを検索
//   - GRAPH_SUMMARY_COMPLETION: グラフを検索して要約を生成
//   - GRAPH_COMPLETION: グラフとチャンクを検索して回答を生成
//
// 引数:
//   - ctx: コンテキスト
//   - cubeDbFilePath: CubeのDBファイルパス
//   - memoryGroup: メモリグループ名（memory_groupとして検索対象を指定）
//   - queryType: クエリタイプ
//   - text: クエリテキスト
//
// 返り値:
//   - string: クエリ結果
//   - types.TokenUsage: トークン使用量
//   - error: エラーが発生した場合
func (s *CuberService) Query(ctx context.Context, cubeDbFilePath string, memoryGroup string, text string, queryConfig query.QueryConfig) (answer *string, chunks *string, summaries *string, graph *[]*storage.Triple, embedding *[]float32, usage types.TokenUsage, err error) {
	st, err := s.GetOrOpenStorage(cubeDbFilePath)
	if err != nil {
		return nil, nil, nil, nil, nil, types.TokenUsage{}, fmt.Errorf("Query: Failed to get storage: %w", err)
	}
	// ========================================
	// 1. 検索ツールの作成
	// ========================================
	// 事前初期化されたLLM（s.LLM）を使用
	searchTool := query.NewGraphCompletionTool(st.Vector, st.Graph, s.LLM, s.Embedder, memoryGroup, s.Config.CompletionModel)
	// ========================================
	// 2. クエリの実行
	// ========================================
	// クエリタイプに応じて適切な検索処理が実行されます
	return searchTool.Query(ctx, text, queryConfig)
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
//   - cubeDbFilePath: CubeのDBパス
//   - memoryGroup: メモリグループ名（memory_groupとして使用）
//   - config: Memify設定（再帰深度やUnknown優先フラグを含む）
//
// 返り値:
//   - types.TokenUsage: トークン使用量
//   - error: エラーが発生した場合
func (s *CuberService) Memify(ctx context.Context, cubeDbFilePath string, memoryGroup string, config *MemifyConfig) (types.TokenUsage, error) {
	var totalUsage types.TokenUsage
	if config == nil {
		config = &MemifyConfig{RecursiveDepth: 0, PrioritizeUnknowns: true}
	}
	if config.RulesNodeSetName == "" {
		config.RulesNodeSetName = "coding_agent_rules"
	}
	// Storage retrieval
	st, err := s.GetOrOpenStorage(cubeDbFilePath)
	if err != nil {
		return totalUsage, fmt.Errorf("Memify: Failed to get storage: %w", err)
	}
	// ========================================
	// Phase A: Unknown解決フェーズ (Priority High)
	// ========================================
	if config.PrioritizeUnknowns {
		fmt.Println("Memify: Phase A - Prioritizing Unknown Resolution")
		ignoranceManager := metacognition.NewIgnoranceManager(
			st.Vector, st.Graph, s.LLM, s.Embedder, memoryGroup,
			s.Config.MetaSimilarityThresholdUnknown,
			s.Config.MetaSearchLimitUnknown,
			s.Config.CompletionModel,
		)
		// 1. 未解決のUnknownを取得
		unknowns, err := ignoranceManager.GetUnresolvedUnknowns(ctx)
		if err != nil {
			fmt.Printf("Warning: Failed to get unresolved unknowns: %v\n", err)
		} else {
			for _, unknown := range unknowns {
				// 2. 各Unknownについて、解決のための自問自答と検索を集中的に行う
				usage, err := s.attemptToResolveUnknown(ctx, st, unknown, memoryGroup)
				totalUsage.Add(usage)
				if err != nil {
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
		usage, err := s.executeMemifyCore(ctx, st, memoryGroup, config)
		totalUsage.Add(usage)
		if err != nil {
			return totalUsage, fmt.Errorf("Memify execution failed at level %d: %w", level, err)
		}
	}
	return totalUsage, nil
}

// memifyBulkProcess は、全テキストを一括で処理します。
// Python版の memify と同等の精度を保証します。
func (s *CuberService) memifyBulkProcess(
	ctx context.Context,
	st *StorageSet,
	texts []string,
	memoryGroup string,
	rulesNodeSetName string,
) (types.TokenUsage, error) {
	var totalUsage types.TokenUsage
	// ルール抽出タスクを作成
	task := memify.NewRuleExtractionTask(
		st.Vector,
		st.Graph,
		s.LLM,
		s.Embedder,
		memoryGroup,
		rulesNodeSetName,
		s.Config.CompletionModel,
	)
	// 全テキストを1つのバッチとして処理
	if usage, err := task.ProcessBatch(ctx, texts); err != nil {
		return usage, fmt.Errorf("Bulk processing failed: %w", err)
	} else {
		totalUsage.Add(usage)
	}
	return totalUsage, nil
}

// memifyBatchProcess は、テキストをバッチ分割して処理します。
// 大規模データに対応し、メモリ使用量を抑制します。
func (s *CuberService) memifyBatchProcess(
	ctx context.Context,
	st *StorageSet,
	texts []string,
	memoryGroup string,
	rulesNodeSetName string,
	batchCharSize int,
	overlapPercent int,
) (types.TokenUsage, error) {
	var totalUsage types.TokenUsage
	// ルール抽出タスクを作成
	task := memify.NewRuleExtractionTask(
		st.Vector,
		st.Graph,
		s.LLM,
		s.Embedder,
		memoryGroup,
		rulesNodeSetName,
		s.Config.CompletionModel,
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
		if usage, err := task.ProcessBatch(ctx, batchSlice); err != nil {
			return totalUsage, fmt.Errorf("Memify: Batch processing failed (batch %d): %w", i+1, err)
		} else {
			totalUsage.Add(usage)
		}
	}
	return totalUsage, nil
}

// RecursiveMemify は廃止されました。代わりに Memify を使用してください。
// config.RecursiveDepth を設定することで同様の動作を実現できます。
//
// executeMemifyCore は、Memifyのコアロジック（チャンク収集〜ルール抽出）を実行します。
// これは再帰ループの各反復で呼び出される1回分の処理単位です。
func (s *CuberService) executeMemifyCore(ctx context.Context, st *StorageSet, memoryGroup string, config *MemifyConfig) (types.TokenUsage, error) {
	var totalUsage types.TokenUsage
	if config.RulesNodeSetName == "" {
		config.RulesNodeSetName = "coding_agent_rules"
	}
	fmt.Printf("Starting Memify Core Execution for group: %s\n", memoryGroup)
	// ========================================
	// 1. DocumentChunk のテキストを収集
	// ========================================
	chunkChan, errChan := st.Graph.StreamDocumentChunks(ctx, memoryGroup)
	var texts []string
loop:
	for {
		select {
		case <-ctx.Done():
			return totalUsage, ctx.Err()
		case err := <-errChan:
			if err != nil {
				return totalUsage, fmt.Errorf("Memify: Failed to stream chunks: %w", err)
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
		return totalUsage, nil
	}
	// ========================================
	// 2. ハイブリッド処理の判定
	// ========================================
	totalCharCount := memify.CountTotalUTF8Chars(texts)
	maxCharsForBulk := s.Config.MemifyMaxCharsForBulkProcess
	fmt.Printf("Memify Core: Total chars: %d, Threshold: %d\n", totalCharCount, maxCharsForBulk)
	if totalCharCount <= maxCharsForBulk {
		fmt.Println("Memify Core: Using BULK processing")
		return s.memifyBulkProcess(ctx, st, texts, memoryGroup, config.RulesNodeSetName)
	} else {
		fmt.Println("Memify Core: Using BATCH processing")
		batchCharSize := max(maxCharsForBulk/5, s.Config.MemifyBatchMinChars)
		overlapPercent := max(s.Config.MemifyBatchOverlapPercent, 0)
		return s.memifyBatchProcess(ctx, st, texts, memoryGroup, config.RulesNodeSetName, batchCharSize, overlapPercent)
	}
}

// attemptToResolveUnknown は、特定のUnknownを解決するためにリソースを集中させます。
func (s *CuberService) attemptToResolveUnknown(ctx context.Context, st *StorageSet, unknown *metacognition.Unknown, memoryGroup string) (types.TokenUsage, error) {
	var totalUsage types.TokenUsage
	// 1. SelfReflectionTask を初期化
	task := metacognition.NewSelfReflectionTask(
		st.Vector, st.Graph, s.LLM, s.Embedder, memoryGroup,
		s.Config.MetaSimilarityThresholdReflection,
		s.Config.MetaSearchLimitReflectionChunk,
		s.Config.MetaSearchLimitReflectionRule,
		s.Config.MetaSimilarityThresholdUnknown,
		s.Config.MetaSearchLimitUnknown,
		s.Config.CompletionModel,
	)
	// 2. Unknownのテキストを「問い」として解決を試みる
	// SelfReflectionTask.TryAnswer を使用
	answered, insight, usage, err := task.TryAnswer(ctx, unknown.Text)
	totalUsage.Add(usage)
	if err != nil {
		return totalUsage, fmt.Errorf("TryAnswer failed: %w", err)
	}
	if answered {
		fmt.Printf("Resolved Unknown: %s\nInsight: %s\n", unknown.Text, insight)
		// 3. 解決できた場合は Capability を登録し、Unknown を解決済みとする
		if u, err := task.IgnoranceManager.RegisterCapability(
			ctx,
			insight,
			[]string{"recursive_memify", "unknown_resolution"},
			[]string{""},
			[]string{"self_reflection"},
			[]string{unknown.ID}, // resolvedUnknownID
		); err != nil {
			totalUsage.Add(u)
			return totalUsage, fmt.Errorf("Unknown Resolution: Failed to register capability: %w", err)
		} else {
			totalUsage.Add(u)
		}
	} else {
		fmt.Printf("Could not resolve Unknown: %s\n", unknown.Text)
	}
	return totalUsage, nil
}

// AddToZip adds a file with the given name and content to the zip archive.
func AddToZip(zw *zip.Writer, filename string, content []byte) error {
	w, err := zw.Create(filename)
	if err != nil {
		return err
	}
	_, err = w.Write(content)
	return err
}

// ExportCubeToZip creates a zip archive of the cube database and extra files.
func ExportCubeToZip(cubeDbFilePath string, extraFiles map[string][]byte) (*bytes.Buffer, error) {
	buf := new(bytes.Buffer)
	zw := zip.NewWriter(buf)
	defer zw.Close()
	// 1. Add extra files (metadata, stats, etc.)
	for filename, content := range extraFiles {
		if err := AddToZip(zw, filename, content); err != nil {
			return nil, fmt.Errorf("Failed to add %s to zip: %w", filename, err)
		}
	}
	// 2. Add KuzuDB database (single file)
	// cubeDbFilePath is the full path to the .db file (e.g., .../uuid.db)
	filename := filepath.Base(cubeDbFilePath)
	zipPath := filepath.Join("db", filename) // db/uuid.db
	data, err := os.ReadFile(cubeDbFilePath)
	if err != nil {
		return nil, fmt.Errorf("Failed to read KuzuDB file %s: %w", cubeDbFilePath, err)
	}
	if err := AddToZip(zw, zipPath, data); err != nil {
		return nil, fmt.Errorf("Failed to add %s to zip: %w", zipPath, err)
	}
	return buf, nil
}
