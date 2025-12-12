// Package chunking は、テキストをトークンベースでチャンク分割するタスクを提供します。
// 日本語と英語の両方に対応し、Kagome（日本語形態素解析）とTiktoken（トークンカウント）を使用します。
package chunking

import (
	"context"
	"fmt"
	"os"
	"regexp"
	"strings"
	"unicode/utf8"

	"github.com/t-kawata/mycute/lib/common"
	"github.com/t-kawata/mycute/pkg/cuber/pipeline"
	"github.com/t-kawata/mycute/pkg/cuber/storage"
	"github.com/t-kawata/mycute/pkg/cuber/types"
	"github.com/t-kawata/mycute/pkg/s3client"

	"github.com/google/uuid"
	"github.com/ikawaha/kagome-dict/ipa"
	"github.com/ikawaha/kagome/v2/tokenizer"
)

var (
	// 日本語と英語の句読点と改行2個以上で文を分割するための正規表現。すべての改行コードに対応：CRLF、LF、CR。
	SplitSentencesRegexp = regexp.MustCompile(`[。！？.!?]\s*|(?:\r\n|\r|\n){2,}`)
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
func (t *ChunkingTask) Run(ctx context.Context, input any) (any, types.TokenUsage, error) {
	var totalUsage types.TokenUsage
	dataList, ok := input.([]*storage.Data)
	if !ok {
		return nil, totalUsage, fmt.Errorf("Chunking: Expected []*storage.Data input, got %T", input)
	}
	var allChunks []*storage.Chunk
	for _, data := range dataList {
		// ファイルを取得（S3ならダウンロード、ローカルならパス解決）
		localPath, err := t.s3Client.Down(data.RawDataLocation)
		if err != nil {
			return nil, totalUsage, fmt.Errorf("Chunking: Failed to download file %s: %w", data.RawDataLocation, err)
		}
		// 取得したローカルパスから読み込み
		content, err := os.ReadFile(*localPath)
		if err != nil {
			return nil, totalUsage, fmt.Errorf("Chunking: Failed to read file %s: %w", *localPath, err)
		}
		text := string(content)
		// ドキュメントを作成
		docID := uuid.New().String()
		doc := &storage.Document{
			ID:          docID,
			MemoryGroup: data.MemoryGroup, // パーティション
			DataID:      data.ID,
			Text:        text,
			MetaData:    map[string]any{"source": data.Name},
		}
		// ドキュメントを保存（チャンクの外部キー制約のため）
		if err := t.VectorStorage.SaveDocument(ctx, doc); err != nil {
			return nil, totalUsage, fmt.Errorf("Chunking: Failed to save document for %s: %w", data.Name, err)
		}
		// テキストをチャンク化
		chunks, chunkUsage, err := t.chunkText(text, docID, data.MemoryGroup)
		totalUsage.Add(chunkUsage)
		if err != nil {
			return nil, totalUsage, fmt.Errorf("Chunking: Failed to chunk text for %s: %w", data.Name, err)
		}
		allChunks = append(allChunks, chunks...)
		fmt.Printf("Chunking: Chunked file %s into %d chunks\n", data.Name, len(chunks))
	}
	return allChunks, totalUsage, nil
}

// chunkText は、テキストを文字数ベースでチャンクに分割します。
// 重要：文（sentence）を最小単位とし、文の途中で分割することはありません。
// 1. 文単位に分割（splitSentences）
// 2. 文字数をカウントしながら、文単位でチャンクを構築
// 3. オーバーラップを考慮して前のチャンクの末尾の文を次のチャンクの先頭に含める
// 4. 各チャンクのembeddingを生成
func (t *ChunkingTask) chunkText(text string, documentID string, memoryGroup string) ([]*storage.Chunk, types.TokenUsage, error) {
	var usage types.TokenUsage
	// 文単位に分割（文の途中で切れることを防ぐ）
	sentences := splitSentences(text)
	var chunks []*storage.Chunk
	// 現在構築中のチャンクに含まれる文のリスト
	var currentChunk []string
	// 現在のチャンクの累計文字数
	currentChars := 0
	// オーバーラップ用：前回確定したチャンクの文リスト
	var previousChunkSentences []string
	for _, sentence := range sentences {
		// 文の文字数をカウント（Unicodeのルーン数で正確にカウント）
		sentenceChars := utf8.RuneCountInString(sentence)
		// 現在のチャンクにこの文を追加するとサイズを超える場合
		if currentChars+sentenceChars > t.ChunkSize && len(currentChunk) > 0 {
			// 現在のチャンクを確定（embeddingを生成してchunksに追加）
			if err := t.finalizeChunk(&currentChunk, &currentChars, &previousChunkSentences,
				&chunks, &usage, memoryGroup, documentID); err != nil {
				return nil, usage, err
			}
			// オーバーラップ分（前のチャンクの末尾の文）を新しいチャンクの先頭に追加
			t.addOverlap(&currentChunk, &currentChars, previousChunkSentences)
		}
		// 文を現在のチャンクに追加
		// 注意：1つの文がChunkSizeを超えていても、文は分割せずそのまま1つのチャンクとする
		currentChunk = append(currentChunk, sentence)
		currentChars += sentenceChars
	}
	// 最後のチャンクを確定
	if len(currentChunk) > 0 {
		if err := t.finalizeChunk(&currentChunk, &currentChars, &previousChunkSentences,
			&chunks, &usage, memoryGroup, documentID); err != nil {
			return nil, usage, err
		}
	}
	return chunks, usage, nil
}

// finalizeChunk は現在のチャンクを確定してembeddingを生成し、chunksリストに追加します
func (t *ChunkingTask) finalizeChunk(
	currentChunk *[]string,
	currentChars *int,
	previousChunkSentences *[]string,
	chunks *[]*storage.Chunk,
	usage *types.TokenUsage,
	memoryGroup string,
	documentID string,
) error {
	// チャンクのテキストを結合
	chunkText := strings.Join(*currentChunk, "")
	// embeddingを生成
	embedding, u, err := t.Embedder.EmbedQuery(context.Background(), chunkText)
	usage.Add(u)
	if err != nil {
		return fmt.Errorf("Chunking: Failed to generate embedding: %w", err)
	}
	// チャンクをリストに追加
	*chunks = append(*chunks, &storage.Chunk{
		ID:          *common.GenUUID(),
		MemoryGroup: memoryGroup,
		DocumentID:  documentID,
		Text:        chunkText,
		ChunkIndex:  len(*chunks),
		Embedding:   embedding,
	})
	// 次のオーバーラップ用に現在のチャンクの文を保存
	*previousChunkSentences = make([]string, len(*currentChunk))
	copy(*previousChunkSentences, *currentChunk)
	// 現在のチャンクをリセット
	*currentChunk = []string{}
	*currentChars = 0
	return nil
}

// addOverlap は前のチャンクから ChunkOverlap 分の文字数になるまで、
// 末尾の文を取得して新しいチャンクの先頭に追加します。
// これにより、チャンク境界での文脈の連続性を保ちます。
// 重要：文単位で追加するため、実際のオーバーラップ文字数は ChunkOverlap を超える場合があります。
func (t *ChunkingTask) addOverlap(
	currentChunk *[]string,
	currentChars *int,
	previousChunkSentences []string,
) {
	// オーバーラップサイズが0または前のチャンクがない場合はスキップ
	if t.ChunkOverlap <= 0 || len(previousChunkSentences) == 0 {
		return
	}
	// 前のチャンクの末尾から、ChunkOverlap分の文字数になるまで文を取得
	var overlapSentences []string
	overlapChars := 0
	// 後ろから順に文を追加していき、ChunkOverlap文字数に達するまで続ける
	for i := len(previousChunkSentences) - 1; i >= 0; i-- {
		sentence := previousChunkSentences[i]
		sentenceChars := utf8.RuneCountInString(sentence)
		// オーバーラップサイズを超える場合
		// ただし、まだ1文も追加していない場合は、少なくとも1文は追加する
		if overlapChars+sentenceChars > t.ChunkOverlap && len(overlapSentences) > 0 {
			break
		}
		// 文を先頭に追加（後ろから取得しているので逆順に）
		overlapSentences = append([]string{sentence}, overlapSentences...)
		overlapChars += sentenceChars
		// 少なくとも1文は追加したので、ChunkOverlap以上になったら終了
		if overlapChars >= t.ChunkOverlap {
			break
		}
	}
	// オーバーラップ分の文を新しいチャンクの先頭に追加
	*currentChunk = append(overlapSentences, *currentChunk...)
	*currentChars += overlapChars
}

// splitSentences は、日本語と英語の句読点で文を分割します。
func splitSentences(text string) []string {
	var sentences []string
	lastIndex := 0
	matches := SplitSentencesRegexp.FindAllStringIndex(text, -1)
	for _, match := range matches {
		end := match[1]
		sentence := strings.TrimSpace(text[lastIndex:end])
		if sentence != "" {
			sentences = append(sentences, sentence)
		}
		lastIndex = end
	}
	if lastIndex < len(text) {
		remaining := strings.TrimSpace(text[lastIndex:])
		if remaining != "" {
			sentences = append(sentences, remaining)
		}
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
