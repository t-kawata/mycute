// Package cuber は、Cuber サービスのコアとなるパッケージです。
// このパッケージは、データの取り込み(Add)、知識グラフ化(Cognify)、検索(Search)の
// 3つの主要機能を提供します。
package cuber

import (
	"archive/zip"
	"bytes"
	"context"
	"errors"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"sync"
	"time"

	"github.com/cloudwego/eino/components/model"
	"github.com/cloudwego/eino/schema"
	"github.com/t-kawata/mycute/lib/eventbus"
	"github.com/t-kawata/mycute/pkg/cuber/db/kuzudb"
	"github.com/t-kawata/mycute/pkg/cuber/event"
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
	"github.com/t-kawata/mycute/pkg/cuber/utils"
	"github.com/t-kawata/mycute/pkg/s3client"
	"go.uber.org/zap"
)

type StorageSet struct {
	Vector     storage.VectorStorage // ベクトルストレージ（KuzuDB）
	Graph      storage.GraphStorage  // グラフストレージ（KuzuDB）
	LastUsedAt time.Time
	mu         sync.Mutex // 個別のStorageSetへのアクセス保護
}

// CuberService は、Cuberの主要な機能を提供するサービス構造体です。
// データベース接続とLLMクライアントを内部で保持し、ライフサイクルを管理します。
type CuberService struct {
	StorageMap map[string]*StorageSet // マップのキーは、model.Cube.UUID となる
	mu         sync.RWMutex           // StorageMapへのアクセス保護
	Config     types.CuberConfig      // 設定値を保持
	S3Client   *s3client.S3Client     // S3クライアント（ローカル/S3両対応）
	closeCh    chan struct{}          // サービス終了通知用チャネル
	Logger     *zap.Logger
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
func NewCuberService(config types.CuberConfig) (*CuberService, error) {
	// ========================================
	// 1. 設定のデフォルト値を適用
	// ========================================
	// Logger
	if config.Logger == nil {
		return nil, errors.New("Logger is nil.")
	}
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
	// 2. S3Client の初期化
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
	// 3. サービスインスタンスの作成
	// ========================================
	service := &CuberService{
		StorageMap: make(map[string]*StorageSet),
		Config:     config,
		S3Client:   s3Client,
		closeCh:    closeCh,
		Logger:     config.Logger,
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
			utils.LogWarn(service.Logger, "Failed to cleanup S3 download cache", zap.Error(err))
		}
		for {
			select {
			case <-service.closeCh:
				return // サービス終了時にループを抜ける
			case <-ticker.C:
				if err := service.S3Client.CleanupDownDir(retention); err != nil {
					utils.LogWarn(service.Logger, "Failed to cleanup S3 download cache", zap.Error(err))
				}
			}
		}
	}()
	// ========================================
	// 4. CuberServiceインスタンスを返す
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

// createTempEmbedder creates a temporary embedder instance for a specific operation.
func (s *CuberService) createTempEmbedder(ctx context.Context, config types.EmbeddingModelConfig) (storage.Embedder, error) {
	embConfig := providers.ProviderConfig{
		Type:      providers.ProviderType(config.Provider),
		APIKey:    config.ApiKey,
		BaseURL:   config.BaseURL,
		ModelName: config.Model,
	}
	einoRawEmb, err := providers.NewEmbedder(ctx, embConfig)
	if err != nil {
		return nil, fmt.Errorf("createTempEmbedder: failed to create raw embedder: %w", err)
	}
	return query.NewEinoEmbedderAdapter(einoRawEmb, config.Model), nil
}

// createTempChatModel creates a temporary chat model instance for a specific operation.
func (s *CuberService) createTempChatModel(ctx context.Context, config types.ChatModelConfig) (model.ToolCallingChatModel, error) {
	pConfig := providers.ProviderConfig{
		Type:        providers.ProviderType(config.Provider),
		APIKey:      config.ApiKey,
		BaseURL:     config.BaseURL,
		ModelName:   config.Model,
		MaxTokens:   config.MaxTokens,
		Temperature: config.Temperature,
	}
	return providers.NewChatModel(ctx, pConfig)
}

// VerifyChatModelConfiguration validates the chat model configuration by running a live test.
// It creates a temporary ChatModel and attempts to generate a response for a test prompt.
func (s *CuberService) VerifyChatModelConfiguration(ctx context.Context, config types.ChatModelConfig) error {
	// 1. Validate Provider Type (Basic check)
	if !providers.IsValidProviderType(providers.ProviderType(config.Provider)) {
		return fmt.Errorf("Invalid or unsupported provider: %s", config.Provider)
	}
	// 2. Create ChatModel
	chatModel, err := s.createTempChatModel(ctx, config)
	if err != nil {
		return fmt.Errorf("Failed to initialize chat model: %w", err)
	}
	// 3. Live Test
	// Generate a short response "Hello"
	msg := &schema.Message{
		Role:    schema.User,
		Content: `You must only respond with the word "Hello" and nothing else. No explanations, no additional text.`,
	}
	// Note: We use Generate, not Stream, for simple verification
	resp, err := chatModel.Generate(ctx, []*schema.Message{msg})
	if err != nil {
		return fmt.Errorf("Live chat model test failed: %w", err)
	}
	if resp == nil || resp.Content == "" {
		return fmt.Errorf("Live chat model test returned empty response")
	}
	return nil
}

// VerifyEmbeddingConfiguration validates the embedding configuration by running a live test.
// It creates a temporary embedder and attempts to embed a test string.
// It also checks if the returned dimension matches the expected dimension.
func (s *CuberService) VerifyEmbeddingConfiguration(ctx context.Context, config types.EmbeddingModelConfig) error {
	// 1. Validate Provider Type (Basic check)
	if !providers.IsValidProviderType(providers.ProviderType(config.Provider)) {
		return fmt.Errorf("Invalid or unsupported provider: %s", config.Provider)
	}
	// 2. Create Embedder
	embedder, err := s.createTempEmbedder(ctx, config)
	if err != nil {
		return fmt.Errorf("Failed to initialize embedder: %w", err)
	}
	// 3. Live Test
	vec, _, err := embedder.EmbedQuery(ctx, "こんにちは")
	if err != nil {
		return fmt.Errorf("Live embedding test failed: %w", err)
	}
	// 4. Dimension Check
	if len(vec) != int(config.Dimension) {
		return fmt.Errorf("Dimension mismatch: expected %d, got %d", config.Dimension, len(vec))
	}
	return nil
}

// GetOrOpenStorage retrieves an existing storage set or opens a new one for the given Cube DB file path.
// cubeUUID is derived from the file path (basename without extension).
func (s *CuberService) GetOrOpenStorage(cubeDbFilePath string, embeddingModelConfig types.EmbeddingModelConfig) (*StorageSet, error) {
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
	kuzuSt, err := kuzudb.NewKuzuDBStorage(cubeDbFilePath, s.Logger)
	if err != nil {
		return nil, fmt.Errorf("Failed to open KuzuDB at %s: %w", cubeDbFilePath, err)
	}
	// Ensure schema (lazy init)
	if err := kuzuSt.EnsureSchema(context.Background(), embeddingModelConfig); err != nil {
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
			utils.LogDebug(s.Logger, "Closing idle storage for Cube", zap.String("uuid", uuid), zap.Duration("idle_time", idleTime))
			// Close VectorStorage
			if st.Vector.IsOpen() {
				if err := st.Vector.Close(); err != nil {
					utils.LogWarn(s.Logger, "Error closing vector storage", zap.String("uuid", uuid), zap.Error(err))
				}
			}
			// Close GraphStorage
			if st.Graph.IsOpen() {
				if err := st.Graph.Close(); err != nil {
					utils.LogWarn(s.Logger, "Error closing graph storage", zap.String("uuid", uuid), zap.Error(err))
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
//   - embeddingModelConfig: 埋め込みモデル設定
//   - logger: ロガー
//
// 返り値:
//   - error: エラーが発生した場合
func CreateCubeDB(dbFilePath string, embeddingModelConfig types.EmbeddingModelConfig, logger *zap.Logger) error {
	// 親ディレクトリの作成
	parentDir := filepath.Dir(dbFilePath)
	if err := os.MkdirAll(parentDir, 0755); err != nil {
		return fmt.Errorf("CreateCubeDB: failed to create parent directory: %w", err)
	}
	// KuzuDB を初期化
	kuzuSt, err := kuzudb.NewKuzuDBStorage(dbFilePath, logger)
	if err != nil {
		return fmt.Errorf("CreateCubeDB: failed to create KuzuDBStorage: %w", err)
	}
	defer kuzuSt.Close()
	// スキーマ適用
	if err := kuzuSt.EnsureSchema(context.Background(), embeddingModelConfig); err != nil {
		return fmt.Errorf("CreateCubeDB: failed to apply schema: %w", err)
	}
	utils.LogInfo(logger, "Created new Cube", zap.String("path", dbFilePath))
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
//   - cognifyConfig: Cognify設定
//   - embeddingModelConfig: 埋め込みモデル設定
//   - chatModelConfig: チャットモデル設定
//
// 返り値:
//   - types.TokenUsage: トークン使用量
//   - error: エラーが発生した場合
func (s *CuberService) Absorb(
	ctx context.Context,
	eb *eventbus.EventBus,
	cubeDbFilePath string,
	memoryGroup string,
	filePaths []string,
	cognifyConfig types.CognifyConfig,
	embeddingModelConfig types.EmbeddingModelConfig,
	chatModelConfig types.ChatModelConfig,
	dataCh chan<- event.StreamEvent,
	isEn bool,
) (types.TokenUsage, error) {
	var totalUsage types.TokenUsage
	cubeUUID := getUUIDFromDBFilePath(cubeDbFilePath)

	// Register Events
	if dataCh != nil {
		event.RegisterAbsorbStreamer(eb, dataCh)
	}

	// Emit Absorb Start
	startPayload := event.AbsorbStartPayload{
		BasePayload: event.NewBasePayload(memoryGroup),
		FileCount:   len(filePaths),
	}
	eventbus.Emit(eb, string(event.EVENT_ABSORB_START), startPayload)

	// Ensure storage is ready
	_, err := s.GetOrOpenStorage(cubeDbFilePath, embeddingModelConfig)
	if err != nil {
		eventbus.Emit(eb, string(event.EVENT_ABSORB_ERROR), event.AbsorbErrorPayload{
			BasePayload: event.NewBasePayload(memoryGroup),
			Error:       err,
		})
		return totalUsage, fmt.Errorf("Absorb: Failed to open storage for cube %s: %w", cubeUUID, err)
	}
	// 1. ファイルの取り込み（add）
	usage1, err := s.add(ctx, eb, cubeDbFilePath, memoryGroup, filePaths, embeddingModelConfig)
	totalUsage.Add(usage1)
	if err != nil {
		eventbus.Emit(eb, string(event.EVENT_ABSORB_ERROR), event.AbsorbErrorPayload{
			BasePayload: event.NewBasePayload(memoryGroup),
			Error:       err,
		})
		return totalUsage, fmt.Errorf("Absorb: Add phase failed: %w", err)
	}
	// Create temp embedder
	embedder, err := s.createTempEmbedder(ctx, embeddingModelConfig)
	if err != nil {
		eventbus.Emit(eb, string(event.EVENT_ABSORB_ERROR), event.AbsorbErrorPayload{
			BasePayload: event.NewBasePayload(memoryGroup),
			Error:       err,
		})
		return totalUsage, fmt.Errorf("Absorb: Failed to create embedder: %w", err)
	}
	// Create temp chat model
	chatModel, err := s.createTempChatModel(ctx, chatModelConfig)
	if err != nil {
		eventbus.Emit(eb, string(event.EVENT_ABSORB_ERROR), event.AbsorbErrorPayload{
			BasePayload: event.NewBasePayload(memoryGroup),
			Error:       err,
		})
		return totalUsage, fmt.Errorf("Absorb: Failed to create chat model: %w", err)
	}
	// 2. 知識グラフの構築（cognify）
	usage2, err := s.cognify(ctx, eb, cubeDbFilePath, memoryGroup, cognifyConfig, embeddingModelConfig, embedder, chatModel, chatModelConfig.Model, isEn) // Passed embedder, chatModel, modelName
	totalUsage.Add(usage2)
	if err != nil {
		eventbus.Emit(eb, string(event.EVENT_ABSORB_ERROR), event.AbsorbErrorPayload{
			BasePayload: event.NewBasePayload(memoryGroup),
			Error:       err,
		})
		return totalUsage, fmt.Errorf("Absorb: Cognify phase failed: %w", err)
	}

	// Emit Absorb End
	eventbus.EmitSync(eb, string(event.EVENT_ABSORB_END), event.AbsorbEndPayload{
		BasePayload: event.NewBasePayload(memoryGroup),
		TotalTokens: totalUsage,
	})
	time.Sleep(150 * time.Millisecond) // Ensure event is processed before function return

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
func (s *CuberService) add(ctx context.Context, eb *eventbus.EventBus, cubeDbFilePath string, memoryGroup string, filePaths []string, embeddingModelConfig types.EmbeddingModelConfig) (types.TokenUsage, error) {
	var usage types.TokenUsage
	utils.LogDebug(s.Logger, "Add: Processing files", zap.String("group", memoryGroup), zap.Int("count", len(filePaths)))
	// Storage retrieval
	st, err := s.GetOrOpenStorage(cubeDbFilePath, embeddingModelConfig)
	if err != nil {
		return usage, fmt.Errorf("Add: Failed to get storage: %w", err)
	}
	// ========================================
	// 1. タスクの作成
	// ========================================
	// IngestTaskを作成
	// このタスクは、ファイルを読み込んでKuzuDBに保存します
	ingestTask := ingestion.NewIngestTask(st.Vector, memoryGroup, s.S3Client, s.Logger, eb)
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
	utils.LogDebug(s.Logger, "Add: Completed", zap.Int64("total_tokens", usage.InputTokens+usage.OutputTokens))
	return usage, nil
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
//   - embeddingModelConfig: EmbeddingModelConfig構造体
//   - embedder: Embedderインスタンス
//   - chatModel: LLMインスタンス
//   - modelName: モデル名
//
// 返り値:
//   - types.TokenUsage: トークン使用量
//   - error: エラーが発生した場合
func (s *CuberService) cognify(
	ctx context.Context,
	eb *eventbus.EventBus,
	cubeDbFilePath string,
	memoryGroup string,
	config types.CognifyConfig,
	embeddingModelConfig types.EmbeddingModelConfig,
	embedder storage.Embedder,
	chatModel model.ToolCallingChatModel,
	modelName string,
	isEn bool,
) (types.TokenUsage, error) {
	var usage types.TokenUsage
	utils.LogDebug(s.Logger, "Cognify: Starting pipeline", zap.String("group", memoryGroup), zap.String("model", modelName))
	// Storage retrieval
	st, err := s.GetOrOpenStorage(cubeDbFilePath, embeddingModelConfig)
	if err != nil {
		return usage, fmt.Errorf("Cognify: Failed to get storage: %w", err)
	}
	// ========================================
	// 1. タスクの初期化
	// ========================================
	// ChunkingTask: テキストをconfig.ChunkSize文字のチャンクに分割
	// config.ChunkOverlap文字のオーバーラップを設定
	chunkingTask, err := chunking.NewChunkingTask(config.ChunkSize, config.ChunkOverlap, st.Vector, embedder, s.S3Client, s.Logger, eb)
	if err != nil {
		return usage, fmt.Errorf("Cognify: Failed to initialize ChunkingTask: %w", err)
	}
	// GraphExtractionTask: LLMを使用してテキストからエンティティと関係を抽出
	graphTask := graph.NewGraphExtractionTask(chatModel, modelName, memoryGroup, s.Logger, eb, isEn)
	// StorageTask: チャンクとグラフをデータベースに保存
	storageTask := storageTaskPkg.NewStorageTask(st.Vector, st.Graph, embedder, memoryGroup, s.Logger, eb)
	// SummarizationTask: チャンクの要約を生成
	summarizationTask := summarization.NewSummarizationTask(st.Vector, chatModel, embedder, memoryGroup, modelName, s.Logger, eb)
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
		utils.LogDebug(s.Logger, "Cognify: No data to process", zap.String("group", memoryGroup))
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
				utils.LogWarn(s.Logger, "Failed to delete file", zap.String("location", data.RawDataLocation), zap.Error(err))
			} else {
				utils.LogDebug(s.Logger, "Deleted file", zap.String("location", data.RawDataLocation))
			}
		}
	}
	utils.LogDebug(s.Logger, "Cognify: Completed pipeline", zap.Int64("total_tokens", usage.InputTokens+usage.OutputTokens))
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
//   - text: クエリテキスト
//   - queryConfig: クエリ設定
//   - embeddingModelConfig: エンベディングモデル設定
//   - chatModelConfig: チャットモデル設定
//
// 返り値:
//   - string: クエリ結果
//   - types.TokenUsage: トークン使用量
//   - error: エラーが発生した場合
func (s *CuberService) Query(
	ctx context.Context,
	eb *eventbus.EventBus,
	cubeDbFilePath string,
	memoryGroup string,
	text string,
	queryConfig types.QueryConfig,
	embeddingModelConfig types.EmbeddingModelConfig,
	chatModelConfig types.ChatModelConfig,
	dataCh chan<- event.StreamEvent,
	isEn bool,
) (answer *string, chunks *string, summaries *string, graph *[]*storage.Triple, embedding *[]float32, usage types.TokenUsage, err error) {
	// Register Events
	if dataCh != nil {
		event.RegisterQueryStreamer(eb, dataCh)
	}

	st, err := s.GetOrOpenStorage(cubeDbFilePath, embeddingModelConfig)
	if err != nil {
		return nil, nil, nil, nil, nil, types.TokenUsage{}, fmt.Errorf("Query: Failed to get storage: %w", err)
	}
	utils.LogDebug(s.Logger, "Query: Executing", zap.String("cube", getUUIDFromDBFilePath(cubeDbFilePath)), zap.String("text", text))
	// ========================================
	// 1. 検索ツールの作成
	// ========================================
	// Create temp embedder
	embedder, err := s.createTempEmbedder(ctx, embeddingModelConfig)
	if err != nil {
		return nil, nil, nil, nil, nil, types.TokenUsage{}, fmt.Errorf("Query: Failed to create embedder: %w", err)
	}
	// Create temp chat model
	chatModel, err := s.createTempChatModel(ctx, chatModelConfig)
	if err != nil {
		return nil, nil, nil, nil, nil, types.TokenUsage{}, fmt.Errorf("Query: Failed to create chat model: %w", err)
	}
	// Create search tool with dynamic LLM
	queryTool := query.NewGraphCompletionTool(st.Vector, st.Graph, chatModel, embedder, memoryGroup, chatModelConfig.Model, eb)
	// ========================================
	// 2. クエリの実行
	// ========================================
	// クエリタイプに応じて適切な検索処理が実行されます
	return queryTool.Query(ctx, text, queryConfig)
}

// Memify は、既存の知識グラフに対して強化処理を適用します。
// 設定に応じて、Unknown解決（Phase A）と知識グラフ拡張（Phase B, 再帰的）を実行します。
//
// 引数:
//   - ctx: コンテキスト
//   - cubeDbFilePath: CubeのDBファイルパス
//   - memoryGroup: メモリグループ名（memory_groupとして検索対象を指定）
//   - memifyConfig: Memify設定
//   - embeddingModelConfig: エンベディングモデル設定
//   - chatModelConfig: チャットモデル設定
//
// 返り値:
//   - types.TokenUsage: トークン使用量
//   - error: エラーが発生した場合
func (s *CuberService) Memify(
	ctx context.Context,
	eb *eventbus.EventBus,
	cubeDbFilePath string,
	memoryGroup string,
	memifyConfig *types.MemifyConfig,
	embeddingModelConfig types.EmbeddingModelConfig,
	chatModelConfig types.ChatModelConfig,
	dataCh chan<- event.StreamEvent,
	isEn bool,
) (totalUsage types.TokenUsage, err error) {
	if memifyConfig == nil {
		memifyConfig = &types.MemifyConfig{RecursiveDepth: 0, PrioritizeUnknowns: true}
	}
	if memifyConfig.RulesNodeSetName == "" {
		memifyConfig.RulesNodeSetName = "coding_agent_rules"
	}

	// Register Events
	if dataCh != nil {
		event.RegisterMemifyStreamer(eb, dataCh)
	}

	utils.LogDebug(s.Logger, "Memify: Starting", zap.String("cube", getUUIDFromDBFilePath(cubeDbFilePath)), zap.String("mode", "unknowns/expansion"))

	// Emit Memify Start
	eventbus.Emit(eb, string(event.EVENT_MEMIFY_START), event.MemifyStartPayload{
		BasePayload: event.NewBasePayload(memoryGroup),
	})

	defer func() {
		if err != nil {
			eventbus.Emit(eb, string(event.EVENT_MEMIFY_ERROR), event.MemifyErrorPayload{
				BasePayload: event.NewBasePayload(memoryGroup),
				Error:       err,
			})
		} else {
			eventbus.EmitSync(eb, string(event.EVENT_MEMIFY_END), event.MemifyEndPayload{
				BasePayload: event.NewBasePayload(memoryGroup),
				TotalTokens: totalUsage,
			})
			time.Sleep(150 * time.Millisecond) // Ensure event is processed before function return
		}
	}()
	// Storage retrieval
	st, err := s.GetOrOpenStorage(cubeDbFilePath, embeddingModelConfig)
	if err != nil {
		return totalUsage, fmt.Errorf("Memify: Failed to get storage: %w", err)
	}
	// Create temp chat model
	chatModel, err := s.createTempChatModel(ctx, chatModelConfig)
	if err != nil {
		return totalUsage, fmt.Errorf("Memify: Failed to create chat model: %w", err)
	}
	// ========================================
	// Phase A: Unknown解決フェーズ (Priority High)
	// ========================================
	// Create temp embedder for Memify
	embedder, err := s.createTempEmbedder(ctx, embeddingModelConfig)
	if err != nil {
		return totalUsage, fmt.Errorf("Memify: Failed to create embedder: %w", err)
	}
	if memifyConfig.PrioritizeUnknowns {
		utils.LogDebug(s.Logger, "Memify: Starting Phase A (Unknown Resolution)", zap.String("group", memoryGroup))
		ignoranceManager := metacognition.NewIgnoranceManager(
			st.Vector, st.Graph, chatModel, embedder, memoryGroup,
			s.Config.MetaSimilarityThresholdUnknown,
			s.Config.MetaSearchLimitUnknown,
			chatModelConfig.Model,
			s.Logger,
		)

		// Emit Unknown Search Start
		eventbus.Emit(eb, string(event.EVENT_MEMIFY_UNKNOWN_SEARCH_START), event.MemifyUnknownSearchStartPayload{
			BasePayload: event.NewBasePayload(memoryGroup),
		})

		// 1. 未解決のUnknownを取得
		unknowns, err := ignoranceManager.GetUnresolvedUnknowns(ctx)

		// Emit Unknown Search End
		unknownCount := len(unknowns)
		eventbus.Emit(eb, string(event.EVENT_MEMIFY_UNKNOWN_SEARCH_END), event.MemifyUnknownSearchEndPayload{
			BasePayload:  event.NewBasePayload(memoryGroup),
			UnknownCount: unknownCount,
		})

		if err != nil {
			utils.LogWarn(s.Logger, "Failed to get unresolved unknowns", zap.Error(err))
		} else {
			for _, unknown := range unknowns {
				// Emit Unknown Item Start
				eventbus.Emit(eb, string(event.EVENT_MEMIFY_UNKNOWN_ITEM_START), event.MemifyUnknownItemStartPayload{
					BasePayload: event.NewBasePayload(memoryGroup),
					UnknownID:   unknown.ID,
				})

				// 2. 各Unknownについて、解決のための自問自答と検索を集中的に行う
				usage, err := s.attemptToResolveUnknown(ctx, st, unknown, memoryGroup, embedder, chatModel, chatModelConfig.Model, eb) // Pass eb
				totalUsage.Add(usage)

				// Emit Unknown Item End
				eventbus.Emit(eb, string(event.EVENT_MEMIFY_UNKNOWN_ITEM_END), event.MemifyUnknownItemEndPayload{
					BasePayload: event.NewBasePayload(memoryGroup),
					UnknownID:   unknown.ID,
				})

				if err != nil {
					utils.LogWarn(s.Logger, "Failed to resolve unknown", zap.String("id", unknown.ID), zap.Error(err))
				}
			}
		}
	}
	// ========================================
	// Phase B: 全体グラフ拡張フェーズ (Priority Normal)
	// ========================================
	utils.LogDebug(s.Logger, "Memify: Starting Phase B (Graph Expansion)")
	// 再帰的にコアロジックを実行
	// RecursiveDepth=0 の場合は1回のみ実行 (level 0 <= 0 for 1 iteration)
	for level := 0; level <= memifyConfig.RecursiveDepth; level++ {
		utils.LogDebug(s.Logger, "Memify: Recursive Level", zap.Int("level", level), zap.Int("max_depth", memifyConfig.RecursiveDepth))

		// Emit Expansion Loop Start
		eventbus.Emit(eb, string(event.EVENT_MEMIFY_EXPANSION_LOOP_START), event.MemifyExpansionLoopStartPayload{
			BasePayload: event.NewBasePayload(memoryGroup),
			Level:       level,
		})

		usage, err := s.executeMemifyCore(ctx, st, memoryGroup, memifyConfig, embedder, chatModel, chatModelConfig.Model, eb) // Pass eb
		totalUsage.Add(usage)

		// Emit Expansion Loop End
		eventbus.Emit(eb, string(event.EVENT_MEMIFY_EXPANSION_LOOP_END), event.MemifyExpansionLoopEndPayload{
			BasePayload: event.NewBasePayload(memoryGroup),
			Level:       level,
		})

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
	embedder storage.Embedder,
	chatModel model.ToolCallingChatModel,
	modelName string,
	eb *eventbus.EventBus,
) (types.TokenUsage, error) {
	var totalUsage types.TokenUsage
	// ルール抽出タスクを作成
	task := memify.NewRuleExtractionTask(
		st.Vector,
		st.Graph,
		chatModel,
		embedder,
		memoryGroup,
		rulesNodeSetName,
		modelName,
		s.Logger,
	)
	// 全テキストを1つのバッチとして処理
	// Emit BATCH_START (Index 1)
	if eb != nil {
		eventbus.Emit(eb, string(event.EVENT_MEMIFY_EXPANSION_BATCH_START), event.MemifyExpansionBatchStartPayload{
			BasePayload: event.NewBasePayload(memoryGroup),
			BatchIndex:  1,
		})
	}

	// Emit BATCH_PROCESS_START
	if eb != nil {
		eventbus.Emit(eb, string(event.EVENT_MEMIFY_EXPANSION_BATCH_PROCESS_START), event.MemifyExpansionBatchProcessStartPayload{
			BasePayload: event.NewBasePayload(memoryGroup),
		})
	}

	if usage, err := task.ProcessBatch(ctx, texts); err != nil {
		return usage, fmt.Errorf("Bulk processing failed: %w", err)
	} else {
		totalUsage.Add(usage)
	}

	// Emit BATCH_PROCESS_END
	if eb != nil {
		eventbus.Emit(eb, string(event.EVENT_MEMIFY_EXPANSION_BATCH_PROCESS_END), event.MemifyExpansionBatchProcessEndPayload{
			BasePayload: event.NewBasePayload(memoryGroup),
		})
	}

	// Emit BATCH_END (Index 1)
	if eb != nil {
		eventbus.Emit(eb, string(event.EVENT_MEMIFY_EXPANSION_BATCH_END), event.MemifyExpansionBatchEndPayload{
			BasePayload: event.NewBasePayload(memoryGroup),
			BatchIndex:  1,
		})
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
	embedder storage.Embedder,
	chatModel model.ToolCallingChatModel,
	modelName string,
	eb *eventbus.EventBus,
) (types.TokenUsage, error) {
	var totalUsage types.TokenUsage
	// ルール抽出タスクを作成
	task := memify.NewRuleExtractionTask(
		st.Vector,
		st.Graph,
		chatModel,
		embedder,
		memoryGroup,
		rulesNodeSetName,
		modelName,
		s.Logger,
	)
	// 1. 全テキストを結合
	combinedText := strings.Join(texts, "\n\n")
	// 2. 日本語自然境界 + オーバーラップで分割
	utils.LogDebug(s.Logger, "Memify [BATCH]: Processing", zap.Int("batch_size_chars", batchCharSize), zap.Int("overlap_percent", overlapPercent))
	batches := memify.SplitTextWithOverlap(combinedText, batchCharSize, overlapPercent)
	utils.LogDebug(s.Logger, "Memify [BATCH]: Split result", zap.Int("batches", len(batches)))
	// 3. 各バッチを処理
	for i, batch := range batches {
		utils.LogDebug(s.Logger, "Memify [BATCH]: Processing batch", zap.Int("index", i+1), zap.Int("total", len(batches)), zap.Int("chars", memify.CountUTF8Chars(batch)))

		// Emit BATCH_START
		if eb != nil {
			eventbus.Emit(eb, string(event.EVENT_MEMIFY_EXPANSION_BATCH_START), event.MemifyExpansionBatchStartPayload{
				BasePayload: event.NewBasePayload(memoryGroup),
				BatchIndex:  i + 1,
			})
		}

		// Emit BATCH_PROCESS_START
		if eb != nil {
			eventbus.Emit(eb, string(event.EVENT_MEMIFY_EXPANSION_BATCH_PROCESS_START), event.MemifyExpansionBatchProcessStartPayload{
				BasePayload: event.NewBasePayload(memoryGroup),
			})
		}

		// 1バッチ分のスライスを作成して渡す
		batchSlice := []string{batch}
		if usage, err := task.ProcessBatch(ctx, batchSlice); err != nil {
			return totalUsage, fmt.Errorf("Memify: Batch processing failed (batch %d): %w", i+1, err)
		} else {
			totalUsage.Add(usage)
		}

		// Emit BATCH_PROCESS_END
		if eb != nil {
			eventbus.Emit(eb, string(event.EVENT_MEMIFY_EXPANSION_BATCH_PROCESS_END), event.MemifyExpansionBatchProcessEndPayload{
				BasePayload: event.NewBasePayload(memoryGroup),
			})
		}

		// Emit BATCH_END
		if eb != nil {
			eventbus.Emit(eb, string(event.EVENT_MEMIFY_EXPANSION_BATCH_END), event.MemifyExpansionBatchEndPayload{
				BasePayload: event.NewBasePayload(memoryGroup),
				BatchIndex:  i + 1,
			})
		}
	}
	return totalUsage, nil
}

// RecursiveMemify は廃止されました。代わりに Memify を使用してください。
// config.RecursiveDepth を設定することで同様の動作を実現できます。
//
// executeMemifyCore は、Memifyのコアロジック（チャンク収集〜ルール抽出）を実行します。
// これは再帰ループの各反復で呼び出される1回分の処理単位です。
func (s *CuberService) executeMemifyCore(ctx context.Context, st *StorageSet, memoryGroup string, memifyConfig *types.MemifyConfig, embedder storage.Embedder, chatModel model.ToolCallingChatModel, modelName string, eb *eventbus.EventBus) (types.TokenUsage, error) {
	var totalUsage types.TokenUsage
	if memifyConfig.RulesNodeSetName == "" {
		memifyConfig.RulesNodeSetName = "coding_agent_rules"
	}
	utils.LogInfo(s.Logger, "Memify Core: Starting execution", zap.String("group", memoryGroup))
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
		utils.LogInfo(s.Logger, "Memify Core: No DocumentChunks found, skipping")
		return totalUsage, nil
	}
	// ========================================
	// 2. ハイブリッド処理の判定
	// ========================================
	totalCharCount := memify.CountTotalUTF8Chars(texts)
	maxCharsForBulk := s.Config.MemifyMaxCharsForBulkProcess
	utils.LogDebug(s.Logger, "Memify Core: Stats", zap.Int("total_chars", totalCharCount), zap.Int("threshold", maxCharsForBulk))
	if totalCharCount <= maxCharsForBulk {
		utils.LogInfo(s.Logger, "Memify Core: Using BULK processing")
		return s.memifyBulkProcess(ctx, st, texts, memoryGroup, memifyConfig.RulesNodeSetName, embedder, chatModel, modelName, eb) // Pass eb
	} else {
		utils.LogInfo(s.Logger, "Memify Core: Using BATCH processing")
		batchCharSize := max(maxCharsForBulk/5, s.Config.MemifyBatchMinChars)
		overlapPercent := max(s.Config.MemifyBatchOverlapPercent, 0)
		return s.memifyBatchProcess(ctx, st, texts, memoryGroup, memifyConfig.RulesNodeSetName, batchCharSize, overlapPercent, embedder, chatModel, modelName, eb) // Pass eb
	}
}

// attemptToResolveUnknown は、特定のUnknownを解決するためにリソースを集中させます。
func (s *CuberService) attemptToResolveUnknown(ctx context.Context, st *StorageSet, unknown *metacognition.Unknown, memoryGroup string, embedder storage.Embedder, chatModel model.ToolCallingChatModel, modelName string, eb *eventbus.EventBus) (types.TokenUsage, error) {
	var totalUsage types.TokenUsage
	// 1. SelfReflectionTask を初期化
	task := metacognition.NewSelfReflectionTask(
		st.Vector, st.Graph, chatModel, embedder, memoryGroup,
		s.Config.MetaSimilarityThresholdReflection,
		s.Config.MetaSearchLimitReflectionChunk,
		s.Config.MetaSearchLimitReflectionRule,
		s.Config.MetaSimilarityThresholdUnknown,
		s.Config.MetaSearchLimitUnknown,
		modelName,
		s.Logger,
		eb,
	)
	// 2. Unknownのテキストを「問い」として解決を試みる
	// SelfReflectionTask.TryAnswer を使用
	answered, insight, usage, err := task.TryAnswer(ctx, unknown.Text, unknown.ID) // Pass unknown.ID
	totalUsage.Add(usage)
	if err != nil {
		return totalUsage, fmt.Errorf("TryAnswer failed: %w", err)
	}
	if answered {
		utils.LogDebug(s.Logger, "Resolved Unknown", zap.String("text", unknown.Text), zap.String("insight", insight))
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
		utils.LogDebug(s.Logger, "Could not resolve Unknown", zap.String("text", unknown.Text))
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
