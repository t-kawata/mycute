package hv1

import (
	"github.com/gin-gonic/gin"
	"github.com/t-kawata/mycute/enum/usrtype"
	"github.com/t-kawata/mycute/mode/rt/rtbl"
	"github.com/t-kawata/mycute/mode/rt/rtreq"
	"github.com/t-kawata/mycute/mode/rt/rtutil"
)

// @Tags v1 Cube
// @Router /v1/cubes/create [post]
// @Summary 新しいCubeを作成する。
// @Description - USR によってのみ使用できる
// @Description - 作成者は「神権限 (Limit = 0: 無制限)」を持つ
// @Description - Cube は知識ベースとして機能し、Absorb/Memify/Search を通じて利用される
// @Accept application/json
// @Param Authorization header string true "token" example(Bearer ??????????)
// @Param json body CreateCubeParam true "json"
// @Success 200 {object} CreateCubeRes{errors=[]int}
// @Failure 400 {object} ErrRes
// @Failure 401 {object} ErrRes
// @Failure 403 {object} ErrRes
// @Failure 500 {object} ErrRes
func CreateCube(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr) {
	if rtbl.RejectUsr(c, u, ju, []usrtype.UsrType{usrtype.KEY, usrtype.APX, usrtype.VDR}) {
		return
	}
	if req, res, ok := rtreq.CreateCubeReqBind(c, u); ok {
		rtbl.CreateCube(c, u, ju, &req, &res)
	} else {
		rtbl.BadRequest(c, &res)
	}
}

// @Tags v1 Cube
// @Router /v1/cubes/absorb [put]
// @Summary コンテンツを取り込む (吸着)
// @Description - USR によってのみ使用できる
// @Description - Cube に知識を追加する
// @Description - 実行には AbsorbLimit に残数が必要
// @Accept application/json
// @Param Authorization header string true "token" example(Bearer ??????????)
// @Param json body AbsorbCubeReq true "json"
// @Success 200 {object} AbsorbCubeRes{errors=[]int}
// @Failure 400 {object} ErrRes
// @Failure 401 {object} ErrRes
// @Failure 403 {object} ErrRes
// @Failure 404 {object} ErrRes
// @Failure 500 {object} ErrRes
func AbsorbCube(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr) {
	if rtbl.RejectUsr(c, u, ju, []usrtype.UsrType{usrtype.KEY, usrtype.APX, usrtype.VDR}) {
		return
	}
	if req, res, ok := rtreq.AbsorbCubeReqBind(c, u); ok {
		rtbl.AbsorbCube(c, u, ju, &req, &res)
	} else {
		rtbl.BadRequest(c, &res)
	}
}

// @Tags v1 Cube
// @Router /v1/cubes/stats [get]
// @Summary Cube の統計情報を取得する。
// @Description - USR によってのみ使用できる
// @Description - MemoryGroup 別の使用量と貢献者情報を返す
// @Description - AllowStats が false の場合は 403
// @Accept application/json
// @Param Authorization header string true "token" example(Bearer ??????????)
// @Param cube_id query uint true "Cube ID"
// @Param memory_group query string false "特定の MemoryGroup に絞る (オプション)"
// @Success 200 {object} StatsCubeRes{errors=[]int}
// @Failure 400 {object} ErrRes
// @Failure 401 {object} ErrRes
// @Failure 403 {object} ErrRes
// @Failure 404 {object} ErrRes
// @Failure 500 {object} ErrRes
func StatsCube(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr) {
	if rtbl.RejectUsr(c, u, ju, []usrtype.UsrType{usrtype.KEY, usrtype.APX, usrtype.VDR}) {
		return
	}
	if req, res, ok := rtreq.StatsCubeReqBind(c, u); ok {
		rtbl.StatsCube(c, u, ju, &req, &res)
	} else {
		rtbl.BadRequest(c, &res)
	}
}
