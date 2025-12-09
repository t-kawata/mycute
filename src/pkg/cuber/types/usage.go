package types

// TokenUsage は LLM/Embedding のトークン使用量を記録します。
type TokenUsage struct {
	InputTokens  int64 `json:"prompt_tokens"`
	OutputTokens int64 `json:"completion_tokens"`
	// Details はモデルごとの使用量内訳を保持します。Keyはモデル名です。
	Details map[string]TokenUsage `json:"details,omitempty"`
}

// Add は他の Usage を加算します。
func (t *TokenUsage) Add(other TokenUsage) {
	t.InputTokens += other.InputTokens
	t.OutputTokens += other.OutputTokens

	if t.Details == nil {
		t.Details = make(map[string]TokenUsage)
	}
	for model, usage := range other.Details {
		if existing, ok := t.Details[model]; ok {
			existing.InputTokens += usage.InputTokens
			existing.OutputTokens += usage.OutputTokens
			t.Details[model] = existing
		} else {
			t.Details[model] = usage
		}
	}
}
