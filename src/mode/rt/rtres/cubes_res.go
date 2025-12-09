package rtres

import (
	"github.com/t-kawata/mycute/lib/common"
	"github.com/t-kawata/mycute/model"
)

// CreateCubeResData はCube作成レスポンスのデータ部分です。
type CreateCubeResData struct {
	UUID string `json:"uuid" swaggertype:"string" example:"550e8400-e29b-41d4-a716-446655440000"`
} // @name CreateCubeResData

// CreateCubeRes はCube作成レスポンスです。
type CreateCubeRes struct {
	Data   CreateCubeResData `json:"data"`
	Errors []Err             `json:"errors"`
} // @name CreateCubeRes

type AbsorbCubeResData struct {
	InputTokens  int64 `json:"input_tokens"`
	OutputTokens int64 `json:"output_tokens"`
	AbsorbLimit  int   `json:"absorb_limit"`
} // @name AbsorbCubeResData

type AbsorbCubeRes struct {
	Data   *AbsorbCubeResData `json:"data"`
	Errors []Err              `json:"errors"`
} // @name AbsorbCubeRes

// GetCubeResData はCube詳細レスポンスのデータ部分です。
type GetCubeResData struct {
	ID          uint   `json:"id" swaggertype:"integer" example:"1"`
	UUID        string `json:"uuid" swaggertype:"string" example:"550e8400-e29b-41d4-a716-446655440000"`
	UsrID       uint   `json:"usr_id" swaggertype:"integer" example:"1"`
	Name        string `json:"name" swaggertype:"string" example:"My Cube"`
	Description string `json:"description" swaggertype:"string" example:"Knowledge base"`
	ExpireAt    string `json:"expire_at,omitempty" swaggertype:"string" format:"date-time"`
	ApxID       uint   `json:"apx_id" swaggertype:"integer" example:"1"`
	VdrID       uint   `json:"vdr_id" swaggertype:"integer" example:"1"`
	CreatedAt   string `json:"created_at" swaggertype:"string" format:"date-time" example:"2025-12-09T11:20:57"`
	UpdatedAt   string `json:"updated_at" swaggertype:"string" format:"date-time" example:"2025-12-09T11:20:57"`
} // @name GetCubeResData

// Of は model.Cube から GetCubeResData のポインタに変換します。
func (d *GetCubeResData) Of(m *model.Cube) *GetCubeResData {
	expireStr := ""
	if m.ExpireAt != nil {
		expireStr = common.ParseDatetimeToStr(m.ExpireAt)
	}
	data := GetCubeResData{
		ID:          m.ID,
		UUID:        m.UUID,
		UsrID:       m.UsrID,
		Name:        m.Name,
		Description: m.Description,
		ExpireAt:    expireStr,
		ApxID:       m.ApxID,
		VdrID:       m.VdrID,
		CreatedAt:   common.ParseDatetimeToStr(&m.CreatedAt),
		UpdatedAt:   common.ParseDatetimeToStr(&m.UpdatedAt),
	}
	return &data
}

// GetCubeRes はCube取得レスポンスです。
type GetCubeRes struct {
	Data   GetCubeResData `json:"data"`
	Errors []Err          `json:"errors"`
} // @name GetCubeRes
