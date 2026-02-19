package validator

import (
	"fmt"
	"strings"
)

// KnownModel は既知の埋め込みモデルの定義です。
type KnownModel struct {
	ProviderKeyword string // プロバイダー判定用キーワード (lowercase)
	ModelKeyword    string // モデル判定用キーワード (lowercase)
	Dimension       uint   // 固定次元数
}

// knownModels は docs/embeddings_models_complete_accurate.csv に基づく定義リストです。
var knownModels = []KnownModel{
	// OpenAI
	{ProviderKeyword: "openai", ModelKeyword: "text-embedding-3-small", Dimension: 1536},
	{ProviderKeyword: "openai", ModelKeyword: "text-embedding-3-large", Dimension: 3072},
	{ProviderKeyword: "openai", ModelKeyword: "text-embedding-ada-002", Dimension: 1536},
	// Mistral
	{ProviderKeyword: "mistral", ModelKeyword: "mistral-embed", Dimension: 1024},
	// Gemini (AI Studio / Vertex AI)
	{ProviderKeyword: "gemini", ModelKeyword: "gemini-embedding-001", Dimension: 3072}, // "AI Studio" or "Vertex AI" keys handled by "gemini"
	{ProviderKeyword: "gemini", ModelKeyword: "text-embedding-004", Dimension: 768},
	// DeepSeek
	{ProviderKeyword: "deepseek", ModelKeyword: "deepseek-embedding-v2", Dimension: 768},
	// Qwen
	{ProviderKeyword: "qwen", ModelKeyword: "qwen3-embedding-0.6b", Dimension: 1024},
	{ProviderKeyword: "qwen", ModelKeyword: "qwen3-embedding-4b", Dimension: 2560},
	{ProviderKeyword: "qwen", ModelKeyword: "qwen3-embedding-8b", Dimension: 4096},
}

// ValidateEmbeddingConfig は入力されたプロバイダー、モデル、次元数の妥当性を検証します。
func ValidateEmbeddingConfig(provider, model string, dimension uint) error {
	providerLower := strings.ToLower(provider)
	modelLower := strings.ToLower(model)
	// 明示的にサポートされていないプロバイダー/モデルのチェック
	if (strings.Contains(providerLower, "anthropic") && strings.Contains(modelLower, "claude")) ||
		(strings.Contains(providerLower, "xai") && strings.Contains(modelLower, "grok")) {
		return fmt.Errorf("Provider '%s' with model '%s' does not support embeddings (Chat only)", provider, model)
	}
	for _, km := range knownModels {
		// プロバイダーとモデル名の両方が含まれているか ("推定")
		if strings.Contains(providerLower, km.ProviderKeyword) && strings.Contains(modelLower, km.ModelKeyword) {
			// 固定次元モデルの場合 check exact match
			if dimension != km.Dimension {
				return fmt.Errorf("Invalid dimension %d for fixed model '%s' (Provider: %s). Expected: %d",
					dimension, model, provider, km.Dimension)
			}
			return nil // Valid (Matched and dimension correct)
		}
	}
	// リストにマッチしない場合は、未知のモデルとして許容する (将来のモデルやマイナーなモデルのため)
	return nil
}
