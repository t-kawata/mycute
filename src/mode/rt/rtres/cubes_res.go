package rtres

import (
	"github.com/t-kawata/mycute/lib/common"
	"github.com/t-kawata/mycute/model"
)

// CreateCubeResData はCube作成レスポンスのデータ部分です。
type CreateCubeResData struct {
	ID   uint   `json:"id" swaggertype:"integer" example:"1"`
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
	Data   AbsorbCubeResData `json:"data"`
	Errors []Err             `json:"errors"`
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

// ========================================
// Stats API Response Structs
// ========================================

// ModelStatRes はモデルごとの使用量です。
type ModelStatRes struct {
	ModelName    string `json:"model_name"`
	ActionType   string `json:"action_type"` // "training" or "search"
	InputTokens  int64  `json:"input_tokens"`
	OutputTokens int64  `json:"output_tokens"`
} // @name ModelStatRes

// ContributorRes は貢献者ごとの使用量です。
type ContributorRes struct {
	ContributorName string `json:"contributor_name"`
	ModelName       string `json:"model_name"`
	InputTokens     int64  `json:"input_tokens"`
	OutputTokens    int64  `json:"output_tokens"`
} // @name ContributorRes

// LineageRes は系譜情報です。
type LineageRes struct {
	UUID          string `json:"uuid"`
	Owner         string `json:"owner"`
	ExportedAt    int64  `json:"exported_at"`     // timestamp ms
	ExportedAtJST string `json:"exported_at_jst"` // YYYY-MM-DDThh:mm:ss (JST)
	Generation    int    `json:"generation"`
} // @name LineageRes

// MemoryGroupStatsRes はMemoryGroup別の統計です。
type MemoryGroupStatsRes struct {
	MemoryGroup  string           `json:"memory_group"`
	Stats        []ModelStatRes   `json:"stats"`
	Contributors []ContributorRes `json:"contributors"`
} // @name MemoryGroupStatsRes

// StatsCubeResData はCube統計レスポンスのデータ部分です。
type StatsCubeResData struct {
	MemoryGroups []MemoryGroupStatsRes `json:"memory_groups"`
	Lineage      []LineageRes          `json:"lineage"`
} // @name StatsCubeResData

// StatsCubeRes はCube統計レスポンスです。
type StatsCubeRes struct {
	Data   StatsCubeResData `json:"data"`
	Errors []Err            `json:"errors"`
} // @name StatsCubeRes

// ExportCubeRes はCubeエクスポートレスポンスです。
// 成功時はZipファイルがダウンロードされるため、このJSONはエラー時のみ返されます。
type ExportCubeRes struct {
	Errors []Err `json:"errors"`
} // @name ExportCubeRes

type GenKeyCubeResData struct {
	Key string `json:"key"`
} // @name GenKeyCubeResData

type GenKeyCubeRes struct {
	Data   GenKeyCubeResData `json:"data"`
	Errors []Err             `json:"errors"`
} // @name GenKeyCubeRes

type ImportCubeResData struct {
	ID   uint   `json:"id"`
	UUID string `json:"uuid"`
} // @name ImportCubeResData

type ImportCubeRes struct {
	Data   ImportCubeResData `json:"data"`
	Errors []Err             `json:"errors"`
} // @name ImportCubeRes

type ReKeyCubeRes struct {
	Errors []Err `json:"errors"`
} // @name ReKeyCubeRes
