package rtreq

import (
	"encoding/json"

	"github.com/gin-gonic/gin"
	"github.com/t-kawata/mycute/mode/rt/rtres"
	"github.com/t-kawata/mycute/mode/rt/rtutil"
	"github.com/t-kawata/mycute/model"
)

type CreateCubeReq struct {
	Name        string `json:"name" binding:"required,max=50"`
	Description string `json:"description" binding:"max=255"`
}

func CreateCubeReqBind(c *gin.Context, u *rtutil.RtUtil) (CreateCubeReq, rtres.CreateCubeRes, bool) {
	ok := true
	req := CreateCubeReq{}
	res := rtres.CreateCubeRes{Errors: []rtres.Err{}}
	if err := c.ShouldBindJSON(&req); err != nil {
		res.Errors = u.GetValidationErrs(err)
		ok = false
	}
	return req, res, ok
}

type AbsorbCubeReq struct {
	CubeID      uint   `json:"cube_id" binding:"required,min=1"`
	MemoryGroup string `json:"memory_group" binding:"required"`
	Content     string `json:"content" binding:"required"`
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

type StatsCubeReq struct {
	CubeID      uint    `form:"cube_id" binding:"required,min=1"`
	MemoryGroup *string `form:"memory_group"` // Optional: filter by specific memory group
}

func StatsCubeReqBind(c *gin.Context, u *rtutil.RtUtil) (StatsCubeReq, rtres.StatsCubeRes, bool) {
	ok := true
	req := StatsCubeReq{}
	res := rtres.StatsCubeRes{Errors: []rtres.Err{}}
	if err := c.ShouldBindQuery(&req); err != nil {
		res.Errors = u.GetValidationErrs(err)
		ok = false
	}
	return req, res, ok
}

type ExportCubeReq struct {
	CubeID uint `form:"cube_id" binding:"required,min=1"`
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
	Permissions model.CubePermission `json:"permissions"`
	ExpireAt    *string              `json:"expire_at"` // ISO8601 or YYYY-MM-DD...
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
	Key         string  `json:"key"`
	Name        string  `json:"name"`
	Description *string `json:"description"`
}

// ImportCubeReqBind binds multipart form data for Import API
// - file: .cube file
// - key: key string
// - name: new cube name
// - description: optional description
func ImportCubeReqBind(c *gin.Context, u *rtutil.RtUtil) (ImportCubeReq, rtres.ImportCubeRes, bool) {
	req := ImportCubeReq{}
	res := rtres.ImportCubeRes{Errors: []rtres.Err{}}

	// Parse key from form field
	keyStr := c.PostForm("key")
	if keyStr == "" {
		res.Errors = append(res.Errors, rtres.Err{Field: "key", Message: "Required field"})
		return req, res, false
	}
	req.Key = keyStr

	// Parse name from form field
	nameStr := c.PostForm("name")
	if nameStr == "" {
		res.Errors = append(res.Errors, rtres.Err{Field: "name", Message: "Required field"})
		return req, res, false
	}
	req.Name = nameStr

	// Parse description from form field (optional)
	descStr := c.PostForm("description")
	if descStr != "" {
		req.Description = &descStr
	}

	return req, res, true
}

type ReKeyCubeReq struct {
	CubeID uint   `json:"cube_id" binding:"required,min=1"`
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
