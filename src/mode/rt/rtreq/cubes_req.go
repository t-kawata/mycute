package rtreq

import (
	"github.com/gin-gonic/gin"
	"github.com/t-kawata/mycute/mode/rt/rtres"
	"github.com/t-kawata/mycute/mode/rt/rtutil"
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
	CubeID      uint   `json:"cube_id" binding:"required"`
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
	CubeID      uint    `form:"cube_id" binding:"required"`
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
