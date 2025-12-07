// Package chunking は、テキストをトークンベースでチャンク分割するタスクを提供します。
// 日本語と英語の両方に対応し、Kagome（日本語形態素解析）とTiktoken（トークンカウント）を使用します。
package chunking

import (
	"context"
	"fmt"
	"os"
	"regexp"
	"strings"

	"mycute/pkg/cognee/pipeline"
	"mycute/pkg/cognee/storage"
	"mycute/pkg/s3client"

	"github.com/google/uuid"
	"github.com/ikawaha/kagome-dict/ipa"
	"github.com/ikawaha/kagome/v2/tokenizer"
	"github.com/pkoukk/tiktoken-go"
)

// ChunkingTask は、テキストをチャンクに分割するタスクです。
type ChunkingTask struct {
	ChunkSize     int                   // チャンクの最大トークン数
	ChunkOverlap  int                   // チャンク間のオーバーラップトークン数
	Tokenizer     *tokenizer.Tokenizer  // 日本語形態素解析器（Kagome）
	VectorStorage storage.VectorStorage // ベクトルストレージ
	Embedder      storage.Embedder      // Embedder
	s3Client      *s3client.S3Client    // S3クライアント
}

// NewChunkingTask は、新しいChunkingTaskを作成します。
func NewChunkingTask(chunkSize, chunkOverlap int, vectorStorage storage.VectorStorage, embedder storage.Embedder, s3Client *s3client.S3Client) (*ChunkingTask, error) {
	// Kagome形態素解析器を初期化
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
		s3Client:      s3Client,
	}, nil
}

var _ pipeline.Task = (*ChunkingTask)(nil)

// Run は、データリストからテキストを読み込み、チャンクに分割します。
func (t *ChunkingTask) Run(ctx context.Context, input any) (any, error) {
	dataList, ok := input.([]*storage.Data)
	if !ok {
		return nil, fmt.Errorf("expected []*storage.Data input, got %T", input)
	}

	var allChunks []*storage.Chunk

	for _, data := range dataList {
		// ファイルを取得（S3ならダウンロード、ローカルならパス解決）
		localPath, err := t.s3Client.Down(data.RawDataLocation)
		if err != nil {
			return nil, fmt.Errorf("failed to download file %s: %w", data.RawDataLocation, err)
		}

		// 取得したローカルパスから読み込み
		content, err := os.ReadFile(*localPath)
		if err != nil {
			return nil, fmt.Errorf("failed to read file %s: %w", *localPath, err)
		}
		text := string(content)

		// ドキュメントを作成
		docID := uuid.New().String()
		doc := &storage.Document{
			ID:       docID,
			GroupID:  data.GroupID, // パーティション
			DataID:   data.ID,
			Text:     text,
			MetaData: map[string]any{"source": data.Name},
		}

		// ドキュメントを保存（チャンクの外部キー制約のため）
		if err := t.VectorStorage.SaveDocument(ctx, doc); err != nil {
			return nil, fmt.Errorf("failed to save document for %s: %w", data.Name, err)
		}

		// テキストをチャンク化
		chunks, err := t.chunkText(text, docID, data.GroupID)
		if err != nil {
			return nil, fmt.Errorf("failed to chunk text for %s: %w", data.Name, err)
		}

		allChunks = append(allChunks, chunks...)
		fmt.Printf("Chunked file %s into %d chunks\n", data.Name, len(chunks))
	}

	return allChunks, nil
}

// chunkText は、テキストをトークンベースでチャンクに分割します。
// 1. 文単位に分割
// 2. トークン数をカウントしながらチャンクを構築
// 3. 各チャンクのembeddingを生成
func (t *ChunkingTask) chunkText(text string, documentID string, groupID string) ([]*storage.Chunk, error) {
	// 文単位に分割
	sentences := splitSentences(text)

	var chunks []*storage.Chunk
	var currentChunk []string
	currentTokens := 0

	// Tiktokenエンコーディング（OpenAIのデフォルト）
	tiktokenEncoding, err := tiktoken.GetEncoding("cl100k_base")
	if err != nil {
		return nil, fmt.Errorf("failed to get tiktoken encoding: %w", err)
	}

	for _, sentence := range sentences {
		// 文のトークン数をカウント
		tokenCount := len(tiktokenEncoding.Encode(sentence, nil, nil))

		if tokenCount > t.ChunkSize {
			// 文が長すぎる場合、単語単位に分割
			words := t.splitByWords(sentence)
			for _, word := range words {
				wordTokens := len(tiktokenEncoding.Encode(word, nil, nil))
				if currentTokens+wordTokens > t.ChunkSize {
					// 現在のチャンクを確定
					chunkText := strings.Join(currentChunk, "")
					embedding, err := t.Embedder.EmbedQuery(context.Background(), chunkText)
					if err != nil {
						return nil, fmt.Errorf("failed to generate embedding: %w", err)
					}

					chunks = append(chunks, &storage.Chunk{
						ID:         uuid.New().String(),
						GroupID:    groupID,
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
				// 現在のチャンクを確定
				chunkText := strings.Join(currentChunk, "")
				embedding, err := t.Embedder.EmbedQuery(context.Background(), chunkText)
				if err != nil {
					return nil, fmt.Errorf("failed to generate embedding: %w", err)
				}

				chunks = append(chunks, &storage.Chunk{
					ID:         uuid.New().String(),
					GroupID:    groupID,
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

	// 最後のチャンクを追加
	if len(currentChunk) > 0 {
		chunkText := strings.Join(currentChunk, "")
		embedding, err := t.Embedder.EmbedQuery(context.Background(), chunkText)
		if err != nil {
			return nil, fmt.Errorf("failed to generate embedding: %w", err)
		}

		chunks = append(chunks, &storage.Chunk{
			ID:         uuid.New().String(),
			GroupID:    groupID,
			DocumentID: documentID,
			Text:       chunkText,
			ChunkIndex: len(chunks),
			Embedding:  embedding,
		})
	}

	return chunks, nil
}

// splitSentences は、日本語と英語の句読点で文を分割します。
func splitSentences(text string) []string {
	// 日本語と英語の句読点の正規表現
	re := regexp.MustCompile(`([。！？.!?])\s*`)

	var sentences []string
	lastIndex := 0
	matches := re.FindAllStringIndex(text, -1)

	for _, match := range matches {
		end := match[1]
		sentence := text[lastIndex:end]
		sentences = append(sentences, sentence)
		lastIndex = end
	}

	// 残りのテキストを追加
	if lastIndex < len(text) {
		sentences = append(sentences, text[lastIndex:])
	}

	return sentences
}

// splitByWords は、Kagomeを使用してテキストを単語単位に分割します。
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
