package rtres

import (
	"github.com/t-kawata/mycute/lib/common"
	"github.com/t-kawata/mycute/model"
)

type SearchChatModelsResData struct {
	ID          uint    `json:"id"`
	Name        string  `json:"name"`
	Provider    string  `json:"provider"`
	Model       string  `json:"model"`
	BaseURL     string  `json:"base_url"`
	MaxTokens   int     `json:"max_tokens"`
	Temperature float64 `json:"temperature"`
	CreatedAt   string  `json:"created_at"`
	UpdatedAt   string  `json:"updated_at"`
} // @name SearchChatModelsResData

func (d *SearchChatModelsResData) Of(ms *[]model.ChatModel) *[]SearchChatModelsResData {
	data := []SearchChatModelsResData{}
	for _, m := range *ms {
		data = append(data, SearchChatModelsResData{
			ID:          m.ID,
			Name:        m.Name,
			Provider:    m.Provider,
			Model:       m.Model,
			BaseURL:     m.BaseURL,
			MaxTokens:   m.MaxTokens,
			Temperature: m.Temperature,
			CreatedAt:   common.ParseDatetimeToStr(&m.CreatedAt),
			UpdatedAt:   common.ParseDatetimeToStr(&m.UpdatedAt),
		})
	}
	return &data
}

type SearchChatModelsRes struct {
	Data   []SearchChatModelsResData `json:"data"`
	Errors []Err                     `json:"errors"`
} // @name SearchChatModelsRes

type GetChatModelResData struct {
	ID          uint    `json:"id"`
	Name        string  `json:"name"`
	Provider    string  `json:"provider"`
	Model       string  `json:"model"`
	BaseURL     string  `json:"base_url"`
	MaxTokens   int     `json:"max_tokens"`
	Temperature float64 `json:"temperature"`
	CreatedAt   string  `json:"created_at"`
	UpdatedAt   string  `json:"updated_at"`
} // @name GetChatModelResData

func (d *GetChatModelResData) Of(m *model.ChatModel) *GetChatModelResData {
	data := GetChatModelResData{
		ID:          m.ID,
		Name:        m.Name,
		Provider:    m.Provider,
		Model:       m.Model,
		BaseURL:     m.BaseURL,
		MaxTokens:   m.MaxTokens,
		Temperature: m.Temperature,
		CreatedAt:   common.ParseDatetimeToStr(&m.CreatedAt),
		UpdatedAt:   common.ParseDatetimeToStr(&m.UpdatedAt),
	}
	return &data
}

type GetChatModelRes struct {
	Data   GetChatModelResData `json:"data"`
	Errors []Err               `json:"errors"`
} // @name GetChatModelRes

type CreateChatModelResData struct {
	ID uint `json:"id"`
} // @name CreateChatModelResData

type CreateChatModelRes struct {
	Data   CreateChatModelResData `json:"data"`
	Errors []Err                  `json:"errors"`
} // @name CreateChatModelRes

type UpdateChatModelResData struct {
} // @name UpdateChatModelResData

type UpdateChatModelRes struct {
	Data   UpdateChatModelResData `json:"data"`
	Errors []Err                  `json:"errors"`
} // @name UpdateChatModelRes

type DeleteChatModelResData struct {
} // @name DeleteChatModelResData

type DeleteChatModelRes struct {
	Data   DeleteChatModelResData `json:"data"`
	Errors []Err                  `json:"errors"`
} // @name DeleteChatModelRes
