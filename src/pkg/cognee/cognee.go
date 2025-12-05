package cognee

import (
	"context"
	"fmt"

	"github.com/t-kawata/mycute/pkg/cognee/pipeline"
	"github.com/t-kawata/mycute/pkg/cognee/storage"
	"github.com/t-kawata/mycute/pkg/cognee/tasks/chunking"
	"github.com/t-kawata/mycute/pkg/cognee/tasks/graph"
	"github.com/t-kawata/mycute/pkg/cognee/tasks/ingestion"
	storageTaskPkg "github.com/t-kawata/mycute/pkg/cognee/tasks/storage"
	"github.com/t-kawata/mycute/pkg/cognee/tools/search"
	"github.com/tmc/langchaingo/llms/openai"
)

type CogneeService struct {
	VectorStorage storage.VectorStorage
	GraphStorage  storage.GraphStorage
	Embedder      storage.Embedder
}

func NewCogneeService(vectorStorage storage.VectorStorage, graphStorage storage.GraphStorage, embedder storage.Embedder) *CogneeService {
	return &CogneeService{
		VectorStorage: vectorStorage,
		GraphStorage:  graphStorage,
		Embedder:      embedder,
	}
}

func (s *CogneeService) Add(ctx context.Context, filePaths []string, dataset string, user string) error {
	// 1. Create Tasks
	ingestTask := ingestion.NewIngestTask(s.VectorStorage)

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
	// 1. Initialize LLM (OpenAI)
	llm, err := openai.New()
	if err != nil {
		return fmt.Errorf("failed to initialize OpenAI LLM: %w", err)
	}

	// 2. Initialize Tasks
	chunkingTask, err := chunking.NewChunkingTask(1024, 20, s.VectorStorage, s.Embedder) // ChunkSize=1024, Overlap=20
	if err != nil {
		return fmt.Errorf("failed to initialize ChunkingTask: %w", err)
	}

	graphTask := graph.NewGraphExtractionTask(llm)
	storageTask := storageTaskPkg.NewStorageTask(s.VectorStorage, s.GraphStorage, s.Embedder)

	// 3. Create Pipeline
	p := pipeline.NewPipeline([]pipeline.Task{
		chunkingTask,
		graphTask,
		storageTask,
	})

	// 4. Fetch Input Data
	// For now, fetch ALL data. In real implementation, filter by dataset/user.
	dataList, err := s.VectorStorage.GetDataList(ctx)
	if err != nil {
		return fmt.Errorf("failed to fetch input data: %w", err)
	}
	if len(dataList) == 0 {
		fmt.Println("No data to process.")
		return nil
	}

	// 5. Run Pipeline
	_, err = p.Run(ctx, dataList)
	if err != nil {
		return fmt.Errorf("cognify pipeline failed: %w", err)
	}

	return nil
}

func (s *CogneeService) Search(ctx context.Context, query string, searchType SearchType, user string) (string, error) {
	// 1. Initialize LLM (OpenAI)
	llm, err := openai.New()
	if err != nil {
		return "", fmt.Errorf("failed to initialize OpenAI LLM: %w", err)
	}

	// 2. Create Search Tool
	searchTool := search.NewGraphCompletionTool(s.VectorStorage, s.GraphStorage, llm, s.Embedder)

	// 3. Execute Search
	return searchTool.Search(ctx, query)
}
