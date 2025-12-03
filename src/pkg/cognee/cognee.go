package cognee

import (
	"context"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"log"
	"os"
	"path/filepath"
	"strings"

	"github.com/google/uuid"
	"github.com/t-kawata/mycute/pkg/s3client"
	"github.com/tmc/langchaingo/llms"
	"github.com/tmc/langchaingo/textsplitter"
	"golang.org/x/sync/errgroup"
)

var (
	metadataStorage MetadataStorage
	vectorStorage   VectorStorage
	graphStorage    GraphStorage
)

func init() {
	metadataStorage = NewInMemoryMetadataStorage()
	vectorStorage = NewInMemoryVectorStorage()
	graphStorage = NewInMemoryGraphStorage()
}

// Add ingests files into the system.
func Add(ctx context.Context, filePaths []string, datasetName string, userID string) error {
	log.Printf("[DEBUG] Add: Start - Args: filePaths=%v, dataset=%s, user=%s", filePaths, datasetName, userID)

	// Initialize S3Client (Local mode for Phase 1)
	// Using dummy AWS creds as they are required by NewS3Client even for local mode
	client, err := s3client.NewS3Client(
		"dummy_access", "dummy_secret", "us-east-1", "dummy_bucket",
		".cognee_data/storage", ".cognee_data/downloads", true,
	)
	if err != nil {
		return fmt.Errorf("failed to initialize s3client: %w", err)
	}

	for _, path := range filePaths {
		// Handle directories by walking them
		info, err := os.Stat(path)
		if err != nil {
			log.Printf("[ERROR] Add: Failed to stat path %s: %v", path, err)
			continue
		}

		var filesToProcess []string
		if info.IsDir() {
			err := filepath.Walk(path, func(p string, info os.FileInfo, err error) error {
				if err != nil {
					return err
				}
				if !info.IsDir() {
					filesToProcess = append(filesToProcess, p)
				}
				return nil
			})
			if err != nil {
				log.Printf("[ERROR] Add: Failed to walk directory %s: %v", path, err)
				continue
			}
		} else {
			filesToProcess = append(filesToProcess, path)
		}

		for _, filePath := range filesToProcess {
			log.Printf("[INFO] Add: Processing file - %s", filePath)

			// 1. Save file using s3client
			savedPathPtr, err := client.Up(filePath)
			if err != nil {
				log.Printf("[ERROR] Add: Failed to upload file %s: %v", filePath, err)
				continue
			}
			savedPath := *savedPathPtr

			// 2. Read file content (for text extraction - simple read for Phase 1)
			// In a real scenario, we would use different loaders based on extension.
			contentBytes, err := os.ReadFile(filePath)
			if err != nil {
				log.Printf("[ERROR] Add: Failed to read file %s: %v", filePath, err)
				continue
			}
			content := string(contentBytes)
			log.Printf("[DEBUG] Add: Read content length: %d", len(content))

			// 3. Generate ID (Hash of content)
			hash := sha256.Sum256(contentBytes)
			contentHash := hex.EncodeToString(hash[:])
			dataID := uuid.New().String()

			// 4. Save Metadata
			data := &Data{
				ID:                   dataID,
				Name:                 filepath.Base(filePath),
				RawDataLocation:      savedPath, // In Phase 1, we treat the saved file as the raw data
				OriginalDataLocation: filePath,
				Extension:            filepath.Ext(filePath),
				MimeType:             "text/plain", // Simplified for Phase 1
				ContentHash:          contentHash,
				OwnerID:              userID,
			}

			if err := metadataStorage.SaveData(ctx, data); err != nil {
				log.Printf("[ERROR] Add: Failed to save metadata for %s: %v", filePath, err)
				continue
			}

			log.Printf("[INFO] Add: Data saved - ID: %s", dataID)
		}
	}

	log.Printf("[DEBUG] Add: Completed")
	return nil
}

// Cognify builds the knowledge graph from ingested data.
func Cognify(ctx context.Context, datasetName string, userID string) error {
	log.Printf("[DEBUG] Cognify: Start - Dataset: %s, User: %s", datasetName, userID)

	// 1. Initialize LLM, Embedder, and S3Client
	llmClient, err := NewLLMClient(ctx)
	if err != nil {
		return fmt.Errorf("failed to initialize LLM client: %w", err)
	}
	embedder, err := NewEmbedder(ctx)
	if err != nil {
		return fmt.Errorf("failed to initialize embedder: %w", err)
	}
	s3, err := s3client.NewS3Client(
		"dummy_access", "dummy_secret", "us-east-1", "dummy_bucket",
		".cognee_data/storage", ".cognee_data/downloads", true,
	)
	if err != nil {
		return fmt.Errorf("failed to initialize s3client: %w", err)
	}

	// 2. Retrieve unprocessed data (For Phase 1, we just iterate over all data in memory)
	// In a real scenario, we would filter by status.
	// We need to access the underlying map of the in-memory storage to iterate.
	// Since we can't easily iterate over the interface, we'll assume we process the data we just added.
	// For simplicity in this Phase 1 demo, we will re-process everything in the store.
	// Cast to concrete type to iterate (only for this in-memory demo)
	memStore, ok := metadataStorage.(*inMemoryMetadataStorage)
	if !ok {
		return fmt.Errorf("storage is not in-memory")
	}

	memStore.mu.RLock()
	var dataList []*Data
	for _, d := range memStore.data {
		dataList = append(dataList, d)
	}
	memStore.mu.RUnlock()

	for _, data := range dataList {
		log.Printf("[INFO] Cognify: Processing data - %s", data.Name)

		// Retrieve file using S3Client
		localPathPtr, err := s3.Down(data.RawDataLocation)
		if err != nil {
			log.Printf("[ERROR] Cognify: Failed to retrieve file %s: %v", data.RawDataLocation, err)
			continue
		}
		localPath := *localPathPtr

		// Read content
		contentBytes, err := os.ReadFile(localPath)
		if err != nil {
			log.Printf("[ERROR] Cognify: Failed to read file %s: %v", localPath, err)
			continue
		}
		text := string(contentBytes)

		// 3. Chunking
		splitter := textsplitter.NewRecursiveCharacter(
			textsplitter.WithChunkSize(4000),
			textsplitter.WithChunkOverlap(200),
			textsplitter.WithSeparators([]string{"\n\n", "\n", " ", ""}),
		)
		docs, err := textsplitter.CreateDocuments(splitter, []string{text}, []map[string]any{{}})
		if err != nil {
			return fmt.Errorf("failed to split text: %w", err)
		}

		var chunks []*Chunk
		for _, doc := range docs {
			chunk := &Chunk{
				ID:         uuid.New().String(),
				DocumentID: data.ID, // Linking directly to Data ID for simplicity in Phase 1
				Text:       doc.PageContent,
				TokenCount: len(strings.Split(doc.PageContent, " ")), // Simple estimation
			}
			chunks = append(chunks, chunk)
		}
		log.Printf("[INFO] Cognify: Created %d chunks", len(chunks))

		// 4. Generate Embeddings
		for _, chunk := range chunks {
			embeddings, err := embedder.EmbedDocuments(ctx, []string{chunk.Text})
			if err != nil {
				return fmt.Errorf("failed to generate embedding: %w", err)
			}
			chunk.Embedding = embeddings[0]
			if err := vectorStorage.SaveChunk(ctx, chunk); err != nil {
				return fmt.Errorf("failed to save chunk: %w", err)
			}
		}

		// 5. Graph Extraction (Parallel)
		nodes, edges, err := processChunksInParallel(ctx, chunks, llmClient)
		if err != nil {
			return fmt.Errorf("graph extraction failed: %w", err)
		}

		// 6. Save Graph Data
		for _, node := range nodes {
			if err := graphStorage.AddNode(ctx, node); err != nil {
				return fmt.Errorf("failed to save node: %w", err)
			}
		}
		for _, edge := range edges {
			if err := graphStorage.AddEdge(ctx, edge); err != nil {
				return fmt.Errorf("failed to save edge: %w", err)
			}
		}

		log.Printf("[INFO] Cognify: Graph saved - Nodes: %d, Edges: %d", len(nodes), len(edges))
	}

	log.Printf("[DEBUG] Cognify: Completed")
	return nil
}

// processChunksInParallel processes chunks concurrently to extract graph data.
func processChunksInParallel(ctx context.Context, chunks []*Chunk, llm llms.Model) ([]*Node, []*Edge, error) {
	g, ctx := errgroup.WithContext(ctx)
	g.SetLimit(10) // Limit concurrency

	nodes := make([][]*Node, len(chunks))
	edges := make([][]*Edge, len(chunks))

	for i, chunk := range chunks {
		i, chunk := i, chunk
		g.Go(func() error {
			// Check context cancellation
			select {
			case <-ctx.Done():
				return ctx.Err()
			default:
			}

			log.Printf("[INFO] Processing chunk %d/%d...", i+1, len(chunks))
			extractedNodes, extractedEdges, err := extractGraphFromChunk(ctx, chunk, llm)
			if err != nil {
				return fmt.Errorf("failed to process chunk %d: %w", i, err)
			}

			nodes[i] = extractedNodes
			edges[i] = extractedEdges
			return nil
		})
	}

	if err := g.Wait(); err != nil {
		return nil, nil, err
	}

	// Merge results
	var allNodes []*Node
	var allEdges []*Edge
	for i := range chunks {
		allNodes = append(allNodes, nodes[i]...)
		allEdges = append(allEdges, edges[i]...)
	}

	return allNodes, allEdges, nil
}

// extractGraphFromChunk extracts nodes and edges from a single chunk using LLM.
func extractGraphFromChunk(ctx context.Context, chunk *Chunk, llm llms.Model) ([]*Node, []*Edge, error) {
	prompt := fmt.Sprintf(`以下のテキストから、エンティティ（ノード）と関係性（エッジ）を抽出してください。

テキスト:
%s

出力形式（JSON）:
{
  "nodes": [
    {"id": "uuid形式", "type": "エンティティタイプ", "properties": {"name": "名前", ...}}
  ],
  "edges": [
    {"source_id": "uuid", "target_id": "uuid", "type": "関係性", "properties": {...}}
  ]
}
JSON以外のテキストは出力しないでください。`, chunk.Text)

	response, err := llms.GenerateFromSinglePrompt(ctx, llm, prompt)
	if err != nil {
		return nil, nil, err
	}

	jsonStr := extractJSON(response)
	if jsonStr == "" {
		return nil, nil, fmt.Errorf("valid JSON not found in response")
	}

	var result struct {
		Nodes []*Node `json:"nodes"`
		Edges []*Edge `json:"edges"`
	}
	if err := json.Unmarshal([]byte(jsonStr), &result); err != nil {
		return nil, nil, fmt.Errorf("JSON unmarshal failed: %w", err)
	}

	return result.Nodes, result.Edges, nil
}

// extractJSON extracts the JSON substring from a string.
func extractJSON(s string) string {
	start := strings.Index(s, "{")
	end := strings.LastIndex(s, "}")
	if start == -1 || end == -1 || start > end {
		return ""
	}
	return s[start : end+1]
}

// Search queries the knowledge graph.
func Search(ctx context.Context, query string, searchType SearchType, userID string) (string, error) {
	log.Printf("[DEBUG] Search: Start - Query: %s, Type: %s", query, searchType)

	// 1. Initialize LLM and Embedder
	llmClient, err := NewLLMClient(ctx)
	if err != nil {
		return "", fmt.Errorf("failed to initialize LLM client: %w", err)
	}
	embedder, err := NewEmbedder(ctx)
	if err != nil {
		return "", fmt.Errorf("failed to initialize embedder: %w", err)
	}

	// 2. Generate Query Embedding
	queryEmbeddings, err := embedder.EmbedDocuments(ctx, []string{query})
	if err != nil {
		return "", fmt.Errorf("failed to embed query: %w", err)
	}
	queryVector := queryEmbeddings[0]

	// 3. Vector Search (Find relevant chunks)
	relevantChunks, err := vectorStorage.Search(ctx, queryVector, 5) // Top 5
	if err != nil {
		return "", fmt.Errorf("vector search failed: %w", err)
	}
	log.Printf("[INFO] Search: Found %d relevant chunks", len(relevantChunks))

	// 4. Graph Search (Get context from related nodes)
	// For Phase 1, we simply use the chunk IDs or some heuristic to find nodes.
	// Since we don't have a direct mapping from Chunk -> Nodes in this simple implementation yet,
	// we will skip complex graph traversal for now and rely on the text content of the chunks.
	// However, to demonstrate GraphStorage usage, let's assume we had some node IDs.
	// In a real implementation, we would index nodes by vector or link chunks to nodes.
	// For this demo, we'll just use the chunk text as context.

	// 5. Construct Context
	var contextBuilder strings.Builder
	for _, chunk := range relevantChunks {
		contextBuilder.WriteString(chunk.Text)
		contextBuilder.WriteString("\n---\n")
	}
	contextText := contextBuilder.String()

	// 6. Generate Answer
	systemPrompt := `あなたは親切なAIアシスタントです。以下のコンテキスト情報のみに基づいて、ユーザーの質問に答えてください。コンテキストに答えがない場合は、「わかりません」と答えてください。`
	userPrompt := fmt.Sprintf("コンテキスト:\n%s\n\n質問: %s", contextText, query)

	messages := []llms.MessageContent{
		llms.TextParts(llms.ChatMessageTypeSystem, systemPrompt),
		llms.TextParts(llms.ChatMessageTypeHuman, userPrompt),
	}

	response, err := llmClient.GenerateContent(ctx, messages)
	if err != nil {
		return "", fmt.Errorf("failed to generate answer: %w", err)
	}

	answer := response.Choices[0].Content
	log.Printf("[DEBUG] Search: Completed")
	return answer, nil
}
