package rtstream

// Tokenize splits text into tokens by rune.
// size が 1 のときは従来どおり 1 文字ずつ分割します。
// size <= 0 のときは size=1 として扱います。
func Tokenize(text string, size int) []string {
	if text == "" {
		return []string{}
	}
	if size <= 0 {
		size = 1
	}
	runes := []rune(text)
	n := len(runes)
	// チャンク数を見積もってスライスを確保
	chunkCount := (n + size - 1) / size
	tokens := make([]string, 0, chunkCount)

	for i := 0; i < n; i += size {
		j := i + size
		if j > n {
			j = n
		}
		tokens = append(tokens, string(runes[i:j]))
	}
	return tokens
}

// TokenizeWithFallback is kept for compatibility but simplifies to Tokenize.
func TokenizeWithFallback(text string, size int) []string {
	if text == "" {
		return []string{}
	}
	// Simple fallback logic if needed, but Tokenize is robust.
	// We can check if Tokenize fails (unlikely for string manipulation)
	// For now, just use Tokenize or strings.Fields if desired?
	// The original logic used Fields if tiktoken failed.
	// Since we don't use tiktoken, we can always use Tokenize.
	return Tokenize(text, size)
}
