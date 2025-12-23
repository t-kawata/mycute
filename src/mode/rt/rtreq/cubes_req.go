package rtreq

import (
	"encoding/json"

	"fmt"

	"github.com/gin-gonic/gin"
	"github.com/t-kawata/mycute/lib/common"
	"github.com/t-kawata/mycute/mode/rt/rtres"
	"github.com/t-kawata/mycute/mode/rt/rtutil"
	"github.com/t-kawata/mycute/model"
	"github.com/t-kawata/mycute/pkg/cuber/types"
	"github.com/t-kawata/mycute/pkg/cuber/validator"
)

type SearchCubesReq struct {
	Name        string `json:"name" binding:"max=50"`
	Description string `json:"description" binding:"max=255"`
	Limit       uint16 `json:"limit" binding:"gte=1,lte=25"`
	Offset      uint16 `json:"offset" binding:"gte=0"`
}

func SearchCubesReqBind(c *gin.Context, u *rtutil.RtUtil) (SearchCubesReq, rtres.SearchCubesRes, bool) {
	ok := true
	req := SearchCubesReq{}
	res := rtres.SearchCubesRes{Errors: []rtres.Err{}}
	if err := c.ShouldBindJSON(&req); err != nil {
		res.Errors = u.GetValidationErrs(err)
		ok = false
	}
	return req, res, ok
}

type GetCubeReq struct {
	ID uint `form:"cube_id" binding:"required,gte=1"`
}

func GetCubeReqBind(c *gin.Context, u *rtutil.RtUtil) (GetCubeReq, rtres.GetCubeRes, bool) {
	ok := true
	req := GetCubeReq{ID: common.StrToUint(c.Param("cube_id"))}
	res := rtres.GetCubeRes{Errors: []rtres.Err{}}
	if err := c.ShouldBind(&req); err != nil {
		res.Errors = u.GetValidationErrs(err)
		ok = false
	}
	return req, res, ok
}

type CreateCubeReq struct {
	Name               string `json:"name" binding:"required,max=50"`
	Description        string `json:"description" binding:"max=255"`
	EmbeddingProvider  string `json:"embedding_provider" binding:"required,max=50"`
	EmbeddingModel     string `json:"embedding_model" binding:"required,max=100"`
	EmbeddingDimension uint   `json:"embedding_dimension" binding:"required,gte=1"` // 0は不可
	EmbeddingApiKey    string `json:"embedding_api_key" binding:"required"`
	EmbeddingBaseURL   string `json:"embedding_base_url" binding:"omitempty,max=255"`
}

func CreateCubeReqBind(c *gin.Context, u *rtutil.RtUtil) (CreateCubeReq, rtres.CreateCubeRes, bool) {
	ok := true
	req := CreateCubeReq{}
	res := rtres.CreateCubeRes{Errors: []rtres.Err{}}
	if err := c.ShouldBindJSON(&req); err != nil {
		res.Errors = u.GetValidationErrs(err)
		ok = false
		return req, res, ok
	}
	// 埋め込みモデル関連のバリデーション実行
	if err := validator.ValidateEmbeddingConfig(req.EmbeddingProvider, req.EmbeddingModel, req.EmbeddingDimension); err != nil {
		res.Errors = append(res.Errors, rtres.Err{Field: "embedding_provider", Message: fmt.Sprintf("Invalid embedding configuration: %s", err.Error())})
		res.Errors = append(res.Errors, rtres.Err{Field: "embedding_model", Message: fmt.Sprintf("Invalid embedding configuration: %s", err.Error())})
		res.Errors = append(res.Errors, rtres.Err{Field: "embedding_dimension", Message: fmt.Sprintf("Invalid embedding configuration: %s", err.Error())})
		ok = false
	}
	// Live Test Embedding
	if ok {
		// Only run if basic validation passed
		embConfig := types.EmbeddingModelConfig{
			Provider:  req.EmbeddingProvider,
			Model:     req.EmbeddingModel,
			Dimension: req.EmbeddingDimension,
			BaseURL:   req.EmbeddingBaseURL,
			ApiKey:    req.EmbeddingApiKey,
		}

		if err := u.CuberService.VerifyEmbeddingConfiguration(c.Request.Context(), embConfig); err != nil {
			res.Errors = append(res.Errors, rtres.Err{Field: "embedding_config", Message: fmt.Sprintf("Live embedding verification failed: %s", err.Error())})
			ok = false
		}
	}
	return req, res, ok
}

type AbsorbCubeReq struct {
	CubeID                     uint    `json:"cube_id" binding:"required,gte=1"`
	MemoryGroup                string  `json:"memory_group" binding:"required,max=64"`
	Content                    string  `json:"content" binding:"required"`
	ChunkSize                  int     `json:"chunk_size" binding:"gte=25"`
	ChunkOverlap               int     `json:"chunk_overlap" binding:"gte=0"`
	ChatModelID                uint    `json:"chat_model_id" binding:"required,gte=1"`
	Stream                     bool    `json:"stream" binding:""`
	IsEn                       bool    `json:"is_en"`                                                   // true=English, false=Japanese (default)
	HalfLifeDays               float64 `json:"half_life_days" binding:"omitempty,gte=1"`                // 価値が半減する日数 (デフォルト: 30)
	PruneThreshold             float64 `json:"prune_threshold" binding:"omitempty,gte=0,lte=1"`         // 削除対象となるThickness閾値 (デフォルト: 0.1)
	MinSurvivalProtectionHours float64 `json:"min_survival_protection_hours" binding:"omitempty,gte=0"` // 新規知識の最低生存保護期間 (デフォルト: 72時間)
	MdlKNeighbors              int     `json:"mdl_k_neighbors" binding:"omitempty,gte=1"`               // MDL判定時の近傍ノード数 (デフォルト: 5)
}

func AbsorbCubeReqBind(c *gin.Context, u *rtutil.RtUtil) (AbsorbCubeReq, rtres.AbsorbCubeRes, bool) {
	ok := true
	req := AbsorbCubeReq{}
	res := rtres.AbsorbCubeRes{Errors: []rtres.Err{}}
	if err := c.ShouldBindJSON(&req); err != nil {
		res.Errors = u.GetValidationErrs(err)
		ok = false
	}
	return req, res, ok
}

type ExportCubeReq struct {
	CubeID uint `form:"cube_id" binding:"required,gte=1"`
}

func ExportCubeReqBind(c *gin.Context, u *rtutil.RtUtil) (ExportCubeReq, rtres.ExportCubeRes, bool) {
	ok := true
	req := ExportCubeReq{}
	res := rtres.ExportCubeRes{Errors: []rtres.Err{}}
	if err := c.ShouldBindQuery(&req); err != nil {
		res.Errors = u.GetValidationErrs(err)
		ok = false
	}
	return req, res, ok
}

type GenKeyCubeReq struct {
	Permissions model.CubePermissions `json:"permissions"`
	ExpireAt    *string               `json:"expire_at"` // ISO8601 or YYYY-MM-DD...
}

// GenKeyCubeReqBind binds multipart form data for GenKey API
// - file: .cube file (handled in BL)
// - permissions: JSON string form field
// - expire_at: optional string form field
func GenKeyCubeReqBind(c *gin.Context, u *rtutil.RtUtil) (GenKeyCubeReq, rtres.GenKeyCubeRes, bool) {
	req := GenKeyCubeReq{}
	res := rtres.GenKeyCubeRes{Errors: []rtres.Err{}}
	// Parse permissions from form field as JSON string
	permStr := c.PostForm("permissions")
	if permStr != "" {
		if err := json.Unmarshal([]byte(permStr), &req.Permissions); err != nil {
			res.Errors = append(res.Errors, rtres.Err{Field: "permissions", Message: "Invalid JSON format"})
			return req, res, false
		}
	} else {
		res.Errors = append(res.Errors, rtres.Err{Field: "permissions", Message: "Required field"})
		return req, res, false
	}
	// Parse expire_at from form field
	expireStr := c.PostForm("expire_at")
	if expireStr != "" {
		req.ExpireAt = &expireStr
	}

	return req, res, true
}

type ImportCubeReq struct {
	Key             string `json:"key" binding:"required"`
	Name            string `json:"name" binding:"required,max=50"`
	Description     string `json:"description" binding:"required,max=255"`
	EmbeddingApiKey string `json:"embedding_api_key" binding:"required"`
}

// ImportCubeReqBind binds multipart form data for Import API
// - file: .cube file
// - key: key string
// - name: new cube name
// - description: optional description
// - embedding_api_key: embedding API key
func ImportCubeReqBind(c *gin.Context, u *rtutil.RtUtil) (ImportCubeReq, rtres.ImportCubeRes, bool) {
	req := ImportCubeReq{}
	res := rtres.ImportCubeRes{Errors: []rtres.Err{}}
	// Parse key from form field
	keyStr := c.PostForm("key")
	if keyStr == "" {
		res.Errors = append(res.Errors, rtres.Err{Field: "key", Message: "Required field."})
		return req, res, false
	}
	req.Key = keyStr
	// Parse name from form field
	nameStr := c.PostForm("name")
	if nameStr == "" {
		res.Errors = append(res.Errors, rtres.Err{Field: "name", Message: "Required field."})
		return req, res, false
	}
	req.Name = nameStr
	// Parse description from form field
	descStr := c.PostForm("description")
	if descStr == "" {
		res.Errors = append(res.Errors, rtres.Err{Field: "description", Message: "Required field."})
		return req, res, false
	}
	req.Description = descStr
	// Parse embedding_api_key from form field
	embKeyStr := c.PostForm("embedding_api_key")
	if embKeyStr == "" {
		res.Errors = append(res.Errors, rtres.Err{Field: "embedding_api_key", Message: "Required field."})
		return req, res, false
	}
	req.EmbeddingApiKey = embKeyStr
	return req, res, true
}

type ReKeyCubeReq struct {
	CubeID uint   `json:"cube_id" binding:"required,gte=1"`
	Key    string `json:"key" binding:"required"`
}

func ReKeyCubeReqBind(c *gin.Context, u *rtutil.RtUtil) (ReKeyCubeReq, rtres.ReKeyCubeRes, bool) {
	ok := true
	req := ReKeyCubeReq{}
	res := rtres.ReKeyCubeRes{Errors: []rtres.Err{}}
	if err := c.ShouldBindJSON(&req); err != nil {
		res.Errors = u.GetValidationErrs(err)
		ok = false
	}
	return req, res, ok
}

type QueryCubeReq struct {
	CubeID                  uint    `json:"cube_id" binding:"required,gte=1"`
	MemoryGroup             string  `json:"memory_group" binding:"required,max=64"`
	Text                    string  `json:"text" binding:"required"`
	Type                    uint8   `json:"type" binding:"required,gte=1,lte=11"`                      // 検索タイプ
	SummaryTopk             int     `json:"summary_topk" binding:"omitempty,gte=0"`                    // 要約文の上位k件を取得
	ChunkTopk               int     `json:"chunk_topk" binding:"omitempty,gte=0"`                      // チャンクの上位k件を取得
	EntityTopk              int     `json:"entity_topk" binding:"omitempty,gte=0"`                     // エンティティの上位k件を対象にグラフを取得
	FtsType                 uint8   `json:"fts_type" binding:"omitempty,gte=0,lte=2"`                  // FTSレイヤー: 0=nouns, 1=nouns_verbs, 2=all
	FtsTopk                 int     `json:"fts_topk" binding:"omitempty,gte=0"`                        // FTS拡張Top-K (0=disabled)
	ThicknessThreshold      float64 `json:"thickness_threshold" binding:"omitempty,gte=0,lte=1"`       // エッジ足切り閾値 (デフォルト: 0.3)
	ConflictResolutionStage uint8   `json:"conflict_resolution_stage" binding:"omitempty,gte=0,lte=2"` // 矛盾解決ステージ: 0=なし, 1=Stage1のみ, 2=Stage1+2
	ChatModelID             uint    `json:"chat_model_id" binding:"required,gte=1"`
	Stream                  bool    `json:"stream" binding:""`
	IsEn                    bool    `json:"is_en"` // true=English, false=Japanese (default)
}

func QueryCubeReqBind(c *gin.Context, u *rtutil.RtUtil) (QueryCubeReq, rtres.QueryCubeRes, bool) {
	ok := true
	req := QueryCubeReq{}
	res := rtres.QueryCubeRes{Errors: []rtres.Err{}}
	if err := c.ShouldBindJSON(&req); err != nil {
		res.Errors = u.GetValidationErrs(err)
		ok = false
	}
	return req, res, ok
}

type MemifyCubeReq struct {
	CubeID                  uint   `json:"cube_id" binding:"required,gte=1"`
	MemoryGroup             string `json:"memory_group" binding:"required,max=64"`
	Epochs                  int    `json:"epochs" binding:"omitempty,gte=0"`
	PrioritizeUnknowns      bool   `json:"prioritize_unknowns" binding:"boolean"`
	ConflictResolutionStage uint8  `json:"conflict_resolution_stage"` // 0=none, 1=stage1, 2=stage1+2
	ChatModelID             uint   `json:"chat_model_id" binding:"required,gte=1"`
	Stream                  bool   `json:"stream" binding:""`
	IsEn                    bool   `json:"is_en"` // true=English, false=Japanese (default)
}

func MemifyCubeReqBind(c *gin.Context, u *rtutil.RtUtil) (MemifyCubeReq, rtres.MemifyCubeRes, bool) {
	ok := true
	req := MemifyCubeReq{}
	res := rtres.MemifyCubeRes{Errors: []rtres.Err{}}
	if err := c.ShouldBindJSON(&req); err != nil {
		res.Errors = u.GetValidationErrs(err)
		ok = false
	}
	return req, res, ok
}

type DeleteCubeReq struct {
	CubeID uint `form:"cube_id" binding:"required,gte=1"`
}

func DeleteCubeReqBind(c *gin.Context, u *rtutil.RtUtil) (DeleteCubeReq, rtres.DeleteCubeRes, bool) {
	ok := true
	req := DeleteCubeReq{}
	res := rtres.DeleteCubeRes{Errors: []rtres.Err{}}
	if err := c.ShouldBindQuery(&req); err != nil {
		res.Errors = u.GetValidationErrs(err)
		ok = false
	}
	return req, res, ok
}
