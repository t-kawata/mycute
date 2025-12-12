package types

import "fmt"

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

// ExtractTokenUsage は LLM レスポンスの GenerationInfo からトークン使用量を抽出します。
// トークン使用量が取得できない場合、またはどちらかがゼロの場合は詳細なエラーを返します。
//
// 引数:
//   - info: LLMレスポンスのGenerationInfo (map[string]any)
//   - modelName: モデル名 (Details に記録)
//   - taskName: エラーメッセージに含めるタスク名（観測性向上のため）
//   - requireOutputTokens: true の場合、OutputTokens が 0 でもエラーにしない（Embedding 等）
//
// 返り値:
//   - TokenUsage: トークン使用量
//   - error: 抽出失敗またはゼロトークンの場合のエラー
func ExtractTokenUsage(info map[string]any, modelName string, taskName string, requireOutputTokens bool) (TokenUsage, error) {
	var u TokenUsage
	if info == nil {
		return u, fmt.Errorf("[%s] GenerationInfo is nil - LLM response did not include token usage metadata", taskName)
	}
	// langchaingoのバージョンによってキー名が異なる (snake_case or PascalCase)
	getInt := func(keys ...string) int64 {
		for _, k := range keys {
			if v, ok := info[k]; ok {
				if f, ok := v.(float64); ok {
					return int64(f)
				}
				if i, ok := v.(int); ok {
					return int64(i)
				}
				if i, ok := v.(int64); ok {
					return i
				}
			}
		}
		return 0
	}
	// 複数のキー名に対応: snake_case (OpenAI API) と PascalCase (langchaingo)
	u.InputTokens = getInt("prompt_tokens", "PromptTokens")
	u.OutputTokens = getInt("completion_tokens", "CompletionTokens")
	// バリデーション: InputTokens は常に必須
	if u.InputTokens == 0 {
		return u, fmt.Errorf("[%s] InputTokens is 0 - 'prompt_tokens'/'PromptTokens' not found in GenerationInfo. Available keys: %v", taskName, mapKeys(info))
	}
	// requireOutputTokens が true の場合、OutputTokens もチェック
	if requireOutputTokens && u.OutputTokens == 0 {
		return u, fmt.Errorf("[%s] OutputTokens is 0 - 'completion_tokens'/'CompletionTokens' not found in GenerationInfo. Available keys: %v", taskName, mapKeys(info))
	}
	// モデル名を Details に記録
	if modelName != "" {
		u.Details = map[string]TokenUsage{
			modelName: {InputTokens: u.InputTokens, OutputTokens: u.OutputTokens},
		}
	}
	return u, nil
}

// mapKeys は map のキー一覧を返します（デバッグ用）
func mapKeys(m map[string]any) []string {
	keys := make([]string, 0, len(m))
	for k := range m {
		keys = append(keys, k)
	}
	return keys
}
