package chunking

import (
	"context"
	"fmt"
	"os"
	"regexp"
	"strings"

	"mycute/pkg/cognee/pipeline"
	"mycute/pkg/cognee/storage"

	"github.com/google/uuid"
	"github.com/ikawaha/kagome-dict/ipa"
	"github.com/ikawaha/kagome/v2/tokenizer"
	"github.com/pkoukk/tiktoken-go"
)

type ChunkingTask struct {
	ChunkSize     int
	ChunkOverlap  int
	Tokenizer     *tokenizer.Tokenizer
	VectorStorage storage.VectorStorage
	Embedder      storage.Embedder
}

func NewChunkingTask(chunkSize, chunkOverlap int, vectorStorage storage.VectorStorage, embedder storage.Embedder) (*ChunkingTask, error) {
	t, err := tokenizer.New(ipa.Dict(), tokenizer.OmitBosEos())
	if err != nil {
		return nil, fmt.Errorf("failed to initialize kagome tokenizer: %w", err)
	}
	return &ChunkingTask{
		ChunkSize:     chunkSize,
		ChunkOverlap:  chunkOverlap,
		Tokenizer:     t,
		VectorStorage: vectorStorage,
		Embedder:      embedder,
	}, nil
}

// Ensure interface implementation
var _ pipeline.Task = (*ChunkingTask)(nil)

func (t *ChunkingTask) Run(ctx context.Context, input any) (any, error) {
	dataList, ok := input.([]*storage.Data)
	if !ok {
		return nil, fmt.Errorf("expected []*storage.Data input, got %T", input)
	}

	var allChunks []*storage.Chunk

	for _, data := range dataList {
		// Read file content
		content, err := os.ReadFile(data.RawDataLocation)
		if err != nil {
			return nil, fmt.Errorf("failed to read file %s: %w", data.RawDataLocation, err)
		}
		text := string(content)

		// Create Document
		docID := uuid.New().String()
		doc := &storage.Document{
			ID:       docID,
			GroupID:  data.GroupID, // [NEW] Partitioning
			DataID:   data.ID,
			Text:     text,
			MetaData: map[string]interface{}{"source": data.Name},
		}

		// Save Document (to satisfy FK for chunks)
		if err := t.VectorStorage.SaveDocument(ctx, doc); err != nil {
			return nil, fmt.Errorf("failed to save document for %s: %w", data.Name, err)
		}

		// Chunk text
		chunks, err := t.chunkText(text, docID, data.GroupID) // [NEW] Pass GroupID
		if err != nil {
			return nil, fmt.Errorf("failed to chunk text for %s: %w", data.Name, err)
		}

		allChunks = append(allChunks, chunks...)
		fmt.Printf("Chunked file %s into %d chunks\n", data.Name, len(chunks))
	}

	return allChunks, nil
}

func (t *ChunkingTask) chunkText(text string, documentID string, groupID string) ([]*storage.Chunk, error) {
	// 1. Split into sentences
	sentences := splitSentences(text)

	// 2. Group sentences into chunks based on token count
	var chunks []*storage.Chunk
	var currentChunk []string
	currentTokens := 0

	tiktokenEncoding, err := tiktoken.GetEncoding("cl100k_base") // OpenAI default
	if err != nil {
		return nil, fmt.Errorf("failed to get tiktoken encoding: %w", err)
	}

	for _, sentence := range sentences {
		// Count tokens in sentence
		tokenCount := len(tiktokenEncoding.Encode(sentence, nil, nil))

		if tokenCount > t.ChunkSize {
			// Sentence is too long, split by words using Kagome
			words := t.splitByWords(sentence)
			for _, word := range words {
				wordTokens := len(tiktokenEncoding.Encode(word, nil, nil))
				if currentTokens+wordTokens > t.ChunkSize {
					// Finalize current chunk
					chunkText := strings.Join(currentChunk, "")
					// Generate Embedding
					embedding, err := t.Embedder.EmbedQuery(context.Background(), chunkText)
					if err != nil {
						return nil, fmt.Errorf("failed to generate embedding: %w", err)
					}

					chunks = append(chunks, &storage.Chunk{
						ID:         uuid.New().String(),
						GroupID:    groupID, // [NEW] Partitioning
						DocumentID: documentID,
						Text:       chunkText,
						ChunkIndex: len(chunks),
						Embedding:  embedding,
					})
					currentChunk = []string{}
					currentTokens = 0
				}
				currentChunk = append(currentChunk, word)
				currentTokens += wordTokens
			}
		} else {
			if currentTokens+tokenCount > t.ChunkSize {
				// Finalize current chunk
				chunkText := strings.Join(currentChunk, "")
				// Generate Embedding
				embedding, err := t.Embedder.EmbedQuery(context.Background(), chunkText)
				if err != nil {
					return nil, fmt.Errorf("failed to generate embedding: %w", err)
				}

				chunks = append(chunks, &storage.Chunk{
					ID:         uuid.New().String(),
					GroupID:    groupID, // [NEW] Partitioning
					DocumentID: documentID,
					Text:       chunkText,
					ChunkIndex: len(chunks),
					Embedding:  embedding,
				})
				currentChunk = []string{}
				currentTokens = 0
			}
			currentChunk = append(currentChunk, sentence)
			currentTokens += tokenCount
		}
	}

	// Add last chunk
	if len(currentChunk) > 0 {
		chunkText := strings.Join(currentChunk, "")
		// Generate Embedding
		embedding, err := t.Embedder.EmbedQuery(context.Background(), chunkText)
		if err != nil {
			return nil, fmt.Errorf("failed to generate embedding: %w", err)
		}

		chunks = append(chunks, &storage.Chunk{
			ID:         uuid.New().String(),
			GroupID:    groupID, // [NEW] Partitioning
			DocumentID: documentID,
			Text:       chunkText,
			ChunkIndex: len(chunks),
			Embedding:  embedding,
		})
	}

	return chunks, nil
}

func splitSentences(text string) []string {
	// Regex for Japanese and English punctuation
	// 。！？.!? followed by optional space/newline
	re := regexp.MustCompile(`([。！？.!?])\s*`)

	// Split keeps the delimiter? No, Split slices around it.
	// We want to keep the delimiter attached to the sentence.
	// We can use FindAllStringIndex.

	var sentences []string
	lastIndex := 0
	matches := re.FindAllStringIndex(text, -1)

	for _, match := range matches {
		// match[1] is the end of the delimiter
		end := match[1]
		sentence := text[lastIndex:end]
		sentences = append(sentences, sentence)
		lastIndex = end
	}

	// Add remaining text
	if lastIndex < len(text) {
		sentences = append(sentences, text[lastIndex:])
	}

	return sentences
}

func (t *ChunkingTask) splitByWords(text string) []string {
	tokens := t.Tokenizer.Tokenize(text)
	var words []string
	for _, token := range tokens {
		if token.Class == tokenizer.DUMMY {
			continue
		}
		words = append(words, token.Surface)
	}
	return words
}
