package rtres

import (
	"github.com/t-kawata/mycute/lib/common"
	"github.com/t-kawata/mycute/model"
)

type SearchCubesResCube struct {
	ID             uint                  `json:"id" swaggertype:"integer" example:"1"`
	UUID           string                `json:"uuid" swaggertype:"string" example:"550e8400-e29b-41d4-a716-446655440000"`
	Name           string                `json:"name" swaggertype:"string" example:"MyCube"`
	Description    string                `json:"description" swaggertype:"string" example:"This is my cube"`
	ExpireAt       string                `json:"expire_at" swaggertype:"string" format:"date-time" example:"2025-01-01T00:00:00"`
	Permissions    model.CubePermissions `json:"permissions" swaggertype:"string" example:"{}"`
	SourceExportID *uint                 `json:"source_export_id" swaggertype:"integer" example:"1"`
	ApxID          uint                  `json:"apx_id" swaggertype:"integer" example:"1"`
	VdrID          uint                  `json:"vdr_id" swaggertype:"integer" example:"1"`
	CreatedAt      string                `json:"created_at" swaggertype:"string" format:"date-time" example:"2025-01-01T00:00:00"`
	UpdatedAt      string                `json:"updated_at" swaggertype:"string" format:"date-time" example:"2025-01-01T00:00:00"`
}

type SearchCubesResData struct {
	Cube         SearchCubesResCube    `json:"cube"`
	Lineage      []LineageRes          `json:"lineage"`
	MemoryGroups []MemoryGroupStatsRes `json:"memory_groups"`
} // @name SearchCubesResData

type SearchCubesRes struct {
	Data   []SearchCubesResData `json:"data"`
	Errors []Err                `json:"errors"`
} // @name SearchCubesRes

type GetCubeResCube struct {
	ID             uint                  `json:"id" swaggertype:"integer" example:"1"`
	UUID           string                `json:"uuid" swaggertype:"string" example:"550e8400-e29b-41d4-a716-446655440000"`
	Name           string                `json:"name" swaggertype:"string" example:"MyCube"`
	Description    string                `json:"description" swaggertype:"string" example:"This is my cube"`
	ExpireAt       string                `json:"expire_at" swaggertype:"string" format:"date-time" example:"2025-01-01T00:00:00"`
	Permissions    model.CubePermissions `json:"permissions" swaggertype:"string" example:"{}"`
	SourceExportID *uint                 `json:"source_export_id" swaggertype:"integer" example:"1"`
	ApxID          uint                  `json:"apx_id" swaggertype:"integer" example:"1"`
	VdrID          uint                  `json:"vdr_id" swaggertype:"integer" example:"1"`
	CreatedAt      string                `json:"created_at" swaggertype:"string" format:"date-time" example:"2025-01-01T00:00:00"`
	UpdatedAt      string                `json:"updated_at" swaggertype:"string" format:"date-time" example:"2025-01-01T00:00:00"`
}

type GetCubeResData struct {
	Cube         GetCubeResCube        `json:"cube"`
	Lineage      []LineageRes          `json:"lineage"`
	MemoryGroups []MemoryGroupStatsRes `json:"memory_groups"`
} // @name GetCubeResData

func (d *GetCubeResData) Of(m *model.Cube, lineage *[]LineageRes, memoryGroups *[]MemoryGroupStatsRes) *GetCubeResData {
	permisions, err := common.ParseDatatypesJson[model.CubePermissions](&m.Permissions)
	if err != nil {
		return nil
	}
	data := GetCubeResData{}
	data.Cube = GetCubeResCube{
		ID:             m.ID,
		UUID:           m.UUID,
		Name:           m.Name,
		Description:    m.Description,
		ExpireAt:       common.ParseDatetimeToStr(m.ExpireAt),
		Permissions:    permisions,
		SourceExportID: m.SourceExportID,
		ApxID:          m.ApxID,
		VdrID:          m.VdrID,
		CreatedAt:      common.ParseDatetimeToStr(&m.CreatedAt),
		UpdatedAt:      common.ParseDatetimeToStr(&m.UpdatedAt),
	}
	data.Lineage = *lineage
	data.MemoryGroups = *memoryGroups
	return &data
}

type GetCubeRes struct {
	Data   GetCubeResData `json:"data"`
	Errors []Err          `json:"errors"`
} // @name GetCubeRes

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

type QueryCubeResData struct {
	Answer       string `json:"answer" swaggertype:"string" example:"契約違反の場合は..."`
	InputTokens  int64  `json:"input_tokens" swaggertype:"integer" example:"1500"`
	OutputTokens int64  `json:"output_tokens" swaggertype:"integer" example:"500"`
	QueryLimit   int    `json:"query_limit" swaggertype:"integer" example:"-1"`
} // @name QueryCubeResData

type QueryCubeRes struct {
	Data   QueryCubeResData `json:"data"`
	Errors []Err            `json:"errors"`
} // @name QueryCubeRes

type MemifyCubeResData struct {
	InputTokens  int64 `json:"input_tokens" swaggertype:"integer" example:"5000"`
	OutputTokens int64 `json:"output_tokens" swaggertype:"integer" example:"2000"`
	MemifyLimit  int   `json:"memify_limit" swaggertype:"integer" example:"-1"`
} // @name MemifyCubeResData

type MemifyCubeRes struct {
	Data   MemifyCubeResData `json:"data"`
	Errors []Err             `json:"errors"`
} // @name MemifyCubeRes

type DeleteCubeRes struct {
	Errors []Err `json:"errors"`
} // @name DeleteCubeRes
