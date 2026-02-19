package rtparam

type SearchChatModelsParam struct {
	Name     string `json:"name" swaggertype:"string" example:"My GPT-4"`
	Provider string `json:"provider" swaggertype:"string" example:"openai"`
	Model    string `json:"model" swaggertype:"string" example:"gpt-4o"`
	BaseURL  string `json:"base_url" swaggertype:"string" example:"https://api.openai.com/v1"`
} // @name SearchChatModelsParam

type CreateChatModelParam struct {
	Name        string  `json:"name" swaggertype:"string" example:"My GPT-4" binding:"required"`
	Provider    string  `json:"provider" swaggertype:"string" example:"openai" binding:"required"`
	Model       string  `json:"model" swaggertype:"string" example:"gpt-4o" binding:"required"`
	BaseURL     string  `json:"base_url" swaggertype:"string" example:"https://api.openai.com/v1"`
	ApiKey      string  `json:"api_key" swaggertype:"string" example:"sk-proj-..." binding:"required"`
	MaxTokens   int     `json:"max_tokens" swaggertype:"integer" example:"4096"`
	Temperature float64 `json:"temperature" swaggertype:"number" example:"0.2"`
} // @name CreateChatModelParam

type UpdateChatModelParam struct {
	Name        string   `json:"name" swaggertype:"string" example:"My GPT-4"`
	Provider    string   `json:"provider" swaggertype:"string" example:"openai"`
	Model       string   `json:"model" swaggertype:"string" example:"gpt-4o"`
	BaseURL     string   `json:"base_url" swaggertype:"string" example:"https://api.openai.com/v1"`
	ApiKey      string   `json:"api_key" swaggertype:"string" example:"sk-proj-..."`
	MaxTokens   int      `json:"max_tokens" swaggertype:"integer" example:"4096"`
	Temperature *float64 `json:"temperature" swaggertype:"number" example:"0.2"`
} // @name UpdateChatModelParam
