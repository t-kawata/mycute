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
// @Param json body AbsorbCubeParam true "json"
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

// @Tags v1 Cube
// @Router /v1/cubes/export [get]
// @Summary Cube をエクスポートする。
// @Description - USR によってのみ使用できる
// @Description - Cube を .cube ファイルとしてダウンロードする
// @Description - 実行には ExportLimit に残数が必要
// @Description - 成功すると Zip ファイル (application/zip) が返却される
// @Accept application/json
// @Param Authorization header string true "token" example(Bearer ??????????)
// @Param cube_id query uint true "Cube ID"
// @Success 200 {file} file "cube_uuid.cube"
// @Failure 400 {object} ErrRes
// @Failure 401 {object} ErrRes
// @Failure 403 {object} ErrRes
// @Failure 404 {object} ErrRes
// @Failure 500 {object} ErrRes
func ExportCube(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr) {
	if rtbl.RejectUsr(c, u, ju, []usrtype.UsrType{usrtype.KEY, usrtype.APX, usrtype.VDR}) {
		return
	}
	if req, res, ok := rtreq.ExportCubeReqBind(c, u); ok {
		if buffer, fileName, ok := rtbl.ExportCube(c, u, ju, &req, &res); ok {
			c.Header("Content-Disposition", "attachment; filename="+fileName)
			c.Data(200, "application/octet-stream", buffer.Bytes())
		}
	} else {
		rtbl.BadRequest(c, &res)
	}
}

// @Tags v1 Cube
// @Router /v1/cubes/genkey [post]
// @Summary 鍵を発行する
// @Description - USR によってのみ使用できる
// @Description - Exportされた.cubeファイルをアップロードして鍵を発行
// @Description - 発行される鍵には権限と有効期限が含まれる
// @Accept multipart/form-data
// @Param Authorization header string true "token" example(Bearer ??????????)
// @Param file formData file true ".cube file"
// @Param permissions formData string true "CubePermission JSON"
// @Param expire_at formData string false "有効期限 (ISO8601)"
// @Success 200 {object} GenKeyCubeRes{errors=[]int}
// @Failure 400 {object} ErrRes
// @Failure 401 {object} ErrRes
// @Failure 403 {object} ErrRes
// @Failure 404 {object} ErrRes
// @Failure 500 {object} ErrRes
func GenKeyCube(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr) {
	if rtbl.RejectUsr(c, u, ju, []usrtype.UsrType{usrtype.KEY, usrtype.APX, usrtype.VDR}) {
		return
	}
	if req, res, ok := rtreq.GenKeyCubeReqBind(c, u); ok {
		rtbl.GenKeyCube(c, u, ju, &req, &res)
	} else {
		rtbl.BadRequest(c, &res)
	}
}

// @Tags v1 Cube
// @Router /v1/cubes/import [post]
// @Summary Cubeをインポートする
// @Description - USR によってのみ使用できる
// @Description - .cubeファイルと鍵を使用してCubeをインポート
// @Description - 鍵に含まれる権限と有効期限が適用される
// @Accept multipart/form-data
// @Param Authorization header string true "token" example(Bearer ??????????)
// @Param file formData file true ".cube file"
// @Param key formData string true "鍵文字列"
// @Param name formData string true "新しいCube名"
// @Param description formData string false "説明"
// @Success 200 {object} ImportCubeRes{errors=[]int}
// @Failure 400 {object} ErrRes
// @Failure 401 {object} ErrRes
// @Failure 403 {object} ErrRes
// @Failure 404 {object} ErrRes
// @Failure 500 {object} ErrRes
func ImportCube(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr) {
	if rtbl.RejectUsr(c, u, ju, []usrtype.UsrType{usrtype.KEY, usrtype.APX, usrtype.VDR}) {
		return
	}
	if req, res, ok := rtreq.ImportCubeReqBind(c, u); ok {
		rtbl.ImportCube(c, u, ju, &req, &res)
	} else {
		rtbl.BadRequest(c, &res)
	}
}

// @Tags v1 Cube
// @Router /v1/cubes/rekey [post]
// @Summary Cubeの権限を更新する (ReKey)
// @Description - USR によってのみ使用できる
// @Description - 新しい鍵を使用して権限と有効期限を更新
// @Description - ReKey対象のCubeはImportされたものである必要がある
// @Accept application/json
// @Param Authorization header string true "token" example(Bearer ??????????)
// @Param json body ReKeyCubeParam true "json"
// @Success 200 {object} ReKeyCubeRes{errors=[]int}
// @Failure 400 {object} ErrRes
// @Failure 401 {object} ErrRes
// @Failure 403 {object} ErrRes
// @Failure 404 {object} ErrRes
// @Failure 500 {object} ErrRes
func ReKeyCube(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr) {
	if rtbl.RejectUsr(c, u, ju, []usrtype.UsrType{usrtype.KEY, usrtype.APX, usrtype.VDR}) {
		return
	}
	if req, res, ok := rtreq.ReKeyCubeReqBind(c, u); ok {
		rtbl.ReKeyCube(c, u, ju, &req, &res)
	} else {
		rtbl.BadRequest(c, &res)
	}
}
