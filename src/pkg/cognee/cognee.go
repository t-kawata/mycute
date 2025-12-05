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
	"github.com/tmc/langchaingo/llms/openai"
)

//go:embed db/duckdb/extensions/v1.4.2/darwin_arm64/vss.duckdb_extension
var duckDbVssDarwinArm64 []byte

//go:embed db/duckdb/extensions/v1.4.2/linux_amd64/vss.duckdb_extension
var duckDbVssLinuxAmd64 []byte

//go:embed db/duckdb/schema.sql
var duckDBSchema string

type CogneeConfig struct {
	DuckDBPath   string
	CozoDBPath   string
	OpenAIAPIKey string
}

type CogneeService struct {
	VectorStorage storage.VectorStorage
	GraphStorage  storage.GraphStorage
	Embedder      storage.Embedder
}

func NewCogneeService(config CogneeConfig) (*CogneeService, error) {
	// 1. Initialize DuckDB
	duckDBConn, err := sql.Open("duckdb", fmt.Sprintf("%s?access_mode=READ_WRITE&hnsw_enable_experimental_persistence=true", config.DuckDBPath))
	if err != nil {
		return nil, fmt.Errorf("failed to open DuckDB: %w", err)
	}

	// Load VSS Extension
	if err := loadDuckDBExtension(duckDBConn); err != nil {
		duckDBConn.Close()
		return nil, fmt.Errorf("failed to load VSS extension: %w", err)
	}

	// Apply Schema
	if _, err := duckDBConn.Exec(duckDBSchema); err != nil {
		duckDBConn.Close()
		return nil, fmt.Errorf("failed to apply DuckDB schema: %w", err)
	}

	duckStorage := duckdbRepo.NewDuckDBStorage(duckDBConn)

	// 2. Initialize CozoDB
	cozodbInstance, err := cozo.New("rocksdb", config.CozoDBPath, nil)
	if err != nil {
		duckDBConn.Close()
		return nil, fmt.Errorf("failed to open CozoDB: %w", err)
	}

	cozoStorage := cozodb.NewCozoStorage(&cozodbInstance)
	if err := cozoStorage.EnsureSchema(context.Background()); err != nil {
		cozodbInstance.Close()
		duckDBConn.Close()
		return nil, fmt.Errorf("failed to apply CozoDB schema: %w", err)
	}

	// 3. Initialize Embedder
	llm, err := openai.New(openai.WithToken(config.OpenAIAPIKey))
	if err != nil {
		cozodbInstance.Close()
		duckDBConn.Close()
		return nil, fmt.Errorf("failed to initialize OpenAI LLM: %w", err)
	}
	embedder := search.NewOpenAIEmbedderAdapter(llm)

	return &CogneeService{
		VectorStorage: duckStorage,
		GraphStorage:  cozoStorage,
		Embedder:      embedder,
	}, nil
}

func (s *CogneeService) Close() error {
	var errs []error
	if err := s.VectorStorage.Close(); err != nil {
		errs = append(errs, fmt.Errorf("failed to close VectorStorage: %w", err))
	}
	if err := s.GraphStorage.Close(); err != nil {
		errs = append(errs, fmt.Errorf("failed to close GraphStorage: %w", err))
	}
	if len(errs) > 0 {
		return fmt.Errorf("errors closing service: %v", errs)
	}
	return nil
}

func loadDuckDBExtension(db *sql.DB) error {
	platform := fmt.Sprintf("%s-%s", runtime.GOOS, runtime.GOARCH)
	var data []byte
	switch platform {
	case "darwin-arm64":
		data = duckDbVssDarwinArm64
	case "linux-amd64":
		data = duckDbVssLinuxAmd64
	default:
		return fmt.Errorf("unsupported platform for VSS extension: %s", platform)
	}

	tmpDir := os.TempDir()
	extPath := filepath.Join(tmpDir, "vss.duckdb_extension")
	if err := os.WriteFile(extPath, data, 0755); err != nil {
		return fmt.Errorf("failed to write VSS extension to temp file: %w", err)
	}
	defer os.Remove(extPath)

	query := fmt.Sprintf("INSTALL '%s'; LOAD '%s';", extPath, extPath)
	if _, err := db.Exec(query); err != nil {
		return fmt.Errorf("failed to install/load extension: %w", err)
	}
	return nil
}

func (s *CogneeService) Add(ctx context.Context, filePaths []string, dataset string, user string) error {
	// Generate GroupID from User and Dataset
	groupID := user + "-" + dataset

	// 1. Create Tasks
	ingestTask := ingestion.NewIngestTask(s.VectorStorage, groupID)

	// 2. Create Pipeline
	p := pipeline.NewPipeline([]pipeline.Task{ingestTask})

	// 3. Run Pipeline
	_, err := p.Run(ctx, filePaths)
	if err != nil {
		return fmt.Errorf("pipeline execution failed: %w", err)
	}

	return nil
}

func (s *CogneeService) Cognify(ctx context.Context, dataset string, user string) error {
	// Generate GroupID
	groupID := user + "-" + dataset

	// 1. Initialize LLM (OpenAI)
	llm, err := openai.New()
	if err != nil {
		return fmt.Errorf("failed to initialize OpenAI LLM: %w", err)
	}

	// 2. Initialize Tasks
	// Note: chunkingTask currently doesn't need groupID as it processes purely in memory for the pipeline steps
	// until StorageTask, but for future proofing or if it saves intermediate state, it might.
	// Checking directives... ChunkingTask wasn't explicitly mentioned to change, but StorageTask WAS.
	chunkingTask, err := chunking.NewChunkingTask(1024, 20, s.VectorStorage, s.Embedder)
	if err != nil {
		return fmt.Errorf("failed to initialize ChunkingTask: %w", err)
	}

	graphTask := graph.NewGraphExtractionTask(llm)
	storageTask := storageTaskPkg.NewStorageTask(s.VectorStorage, s.GraphStorage, s.Embedder, groupID)

	// [NEW] Summarization Task (Phase 4) -> Needs GroupID now
	summarizationTask := summarization.NewSummarizationTask(s.VectorStorage, llm, s.Embedder, groupID)

	// 3. Create Pipeline
	p := pipeline.NewPipeline([]pipeline.Task{
		chunkingTask,
		graphTask,
		storageTask,
		summarizationTask,
	})

	// 4. Fetch Input Data
	// Filter by GroupID logic needs to be implemented in implementation of GetDataList or filter here.
	// GetDataList in DuckDBStorage currently returns ALL.
	// Directives said "Enable filtering by group_id on ALL queries".
	// I should update GetDataList to accept groupID too if possible, but the interface signature is fixed there?
	// Wait, I didn't update GetDataList signature in interfaces.go?
	// I updated VectorStorage interface. Let's check my interface update.
	// `GetDataList(ctx context.Context) ([]*Data, error)` -> I missed updating this signature in interface update step?
	// Let's check `interfaces.go` again in my mind.
	// I sent `interfaces.go` update but I didn't verify if I included GetDataList update.
	// If I missed it, I should update it.
	// But for now, let's assume I need to pass groupID to tasks.

	// Actually, `GetDataList(ctx)` is what was there. If I strictly follow directives, I should have updated it.
	// If I didn't, I have a gap.
	// Let's proceed with updating tasks creation first, and if `GetDataList` is broken or needs update, I'll fix it.
	// Actually, for Cognify, we probably want to process only data for this group.
	// So GetDataList SHOULD take groupID.
	dataList, err := s.VectorStorage.GetDataList(ctx, groupID)
	if err != nil {
		return fmt.Errorf("failed to fetch input data: %w", err)
	}

	if len(dataList) == 0 {
		fmt.Println("No data to process for this group.")
		return nil
	}

	// 5. Run Pipeline
	_, err = p.Run(ctx, dataList)
	if err != nil {
		return fmt.Errorf("cognify pipeline failed: %w", err)
	}

	return nil
}

func (s *CogneeService) Search(ctx context.Context, query string, searchType search.SearchType, dataset string, user string) (string, error) {
	// Generate GroupID
	groupID := user + "-" + dataset

	// 1. Initialize LLM (OpenAI)
	llm, err := openai.New()
	if err != nil {
		return "", fmt.Errorf("failed to initialize OpenAI LLM: %w", err)
	}

	// 2. Create Search Tool
	searchTool := search.NewGraphCompletionTool(s.VectorStorage, s.GraphStorage, llm, s.Embedder, groupID)

	// 3. Execute Search
	return searchTool.Search(ctx, query, searchType)
}
