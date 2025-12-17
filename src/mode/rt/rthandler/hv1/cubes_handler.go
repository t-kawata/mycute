package hv1

import (
	"github.com/gin-gonic/gin"
	"github.com/t-kawata/mycute/enum/usrtype"
	"github.com/t-kawata/mycute/mode/rt/rtbl"
	"github.com/t-kawata/mycute/mode/rt/rtreq"
	"github.com/t-kawata/mycute/mode/rt/rtutil"
)

// @Tags v1 Cube
// @Router /v1/cubes/search [post]
// @Summary Cubeを検索
// @Description - USR によってのみ使用できる
// @Description - 条件に一致するCubeの詳細情報を一覧取得する
// @Param Authorization header string true "token" example(Bearer ??????????)
// @Param params body rtparam.SearchCubesParam true "Search Params"
// @Success 200 {object} rtres.SearchCubesRes "Success"
// @Failure 400 {object} rtres.ErrRes "Validation Error"
// @Failure 401 {object} rtres.ErrRes "Unauthorized"
// @Failure 500 {object} rtres.ErrRes "Internal Server Error"
func SearchCubes(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr) {
	if rtbl.RejectUsr(c, u, ju, []usrtype.UsrType{usrtype.KEY, usrtype.APX, usrtype.VDR}) {
		return
	}
	if req, res, ok := rtreq.SearchCubesReqBind(c, u); ok {
		rtbl.SearchCubes(c, u, ju, &req, &res)
	} else {
		rtbl.BadRequest(c, &res)
	}
}

// @Tags v1 Cube
// @Router /v1/cubes/get/{cube_id} [get]
// @Summary Cubeの詳細情報を取得
// @Description - USR によってのみ使用できる
// @Description - Cube の基本情報、統計、系譜情報を取得する
// @Accept application/json
// @Param Authorization header string true "token" example(Bearer ??????????)
// @Param cube_id path int true "Cube ID"
// @Success 200 {object} rtres.GetCubeRes "Success"
// @Failure 400 {object} rtres.ErrRes "Validation Error"
// @Failure 401 {object} rtres.ErrRes "Unauthorized"
// @Failure 404 {object} rtres.ErrRes "Not Found"
// @Failure 500 {object} rtres.ErrRes "Internal Server Error"
func GetCube(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr) {
	if rtbl.RejectUsr(c, u, ju, []usrtype.UsrType{usrtype.KEY, usrtype.APX, usrtype.VDR}) {
		return
	}
	if req, res, ok := rtreq.GetCubeReqBind(c, u); ok {
		rtbl.GetCube(c, u, ju, &req, &res)
	} else {
		rtbl.BadRequest(c, &res)
	}
}

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
// @Summary コンテンツを取り込む
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
// @Param embedding_api_key formData string true "インポートするCubeが要求する埋め込みモデル用API-KEY"
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

// @Tags v1 Cube
// @Router /v1/cubes/query [post]
// @Summary Cubeにクエリを実行する (Query)
// @Description - USR によってのみ使用できる
// @Description - 指定したCubeの知識を利用してクエリに回答する
// @Description - memory_groupで対象分野を指定
// @Description ---
// @Description ### クエリタイプ一覧
// @Description | ID | Type | Description |
// @Description |---|---|---|
// @Description | 1 | QUERY_TYPE_GET_GRAPH | 知識グラフ自体を取得 |
// @Description | 2 | QUERY_TYPE_GET_CHUNKS | ベクトル検索によりチャンクを取得 |
// @Description | 3 | QUERY_TYPE_GET_PRE_MADE_SUMMARIES | 事前に作成された要約リストを取得 |
// @Description | 4 | QUERY_TYPE_GET_GRAPH_AND_CHUNKS | 知識グラフとベクトル検索によるチャンクを取得 |
// @Description | 5 | QUERY_TYPE_GET_GRAPH_AND_PRE_MADE_SUMMARIES | 知識グラフと事前に作成された要約リストを取得 |
// @Description | 6 | QUERY_TYPE_GET_GRAPH_AND_CHUNKS_AND_PRE_MADE_SUMMARIES | 知識グラフとベクトル検索によるチャンクと事前に作成された要約リストを取得 |
// @Description | 7 | QUERY_TYPE_GET_GRAPH_EXPLANATION_EN | 知識グラフを英語で構造文変換して取得 |
// @Description | 8 | QUERY_TYPE_GET_GRAPH_EXPLANATION_JA | 知識グラフを日本語で構造文変換して取得 |
// @Description | 9 | QUERY_TYPE_GET_GRAPH_SUMMARY_EN | 知識グラフを英語で要約文変換して取得 |
// @Description | 10 | QUERY_TYPE_GET_GRAPH_SUMMARY_JA | 知識グラフを日本語で要約文変換して取得 |
// @Description | 11 | QUERY_TYPE_GET_GRAPH_SUMMARY_TO_ANSWER_EN | 知識グラフを、クエリにダイレクトに答えられる形式の英語要約文で取得 |
// @Description | 12 | QUERY_TYPE_GET_GRAPH_SUMMARY_TO_ANSWER_JA | 知識グラフを、クエリにダイレクトに答えられる形式の日本語要約文で取得 |
// @Description | 13 | QUERY_TYPE_ANSWER_BY_PRE_MADE_SUMMARIES_AND_GRAPH_SUMMARY_EN | 事前に作成された要約リストと、知識グラフ要約を用いて質問に英語で回答 |
// @Description | 14 | QUERY_TYPE_ANSWER_BY_PRE_MADE_SUMMARIES_AND_GRAPH_SUMMARY_JA | 事前に作成された要約リストと、知識グラフ要約を用いて質問に日本語で回答 |
// @Description | 15 | QUERY_TYPE_ANSWER_BY_CHUNKS_AND_GRAPH_SUMMARY_EN | ベクトル検索によるチャンクと知識グラフ要約を用いて質問に英語で回答 |
// @Description | 16 | QUERY_TYPE_ANSWER_BY_CHUNKS_AND_GRAPH_SUMMARY_JA | ベクトル検索によるチャンクと知識グラフ要約を用いて質問に日本語で回答 |
// @Param Authorization header string true "token" example(Bearer ??????????)
// @Param json body QueryCubeParam true "json"
// @Success 200 {object} QueryCubeRes{errors=[]int}
// @Failure 400 {object} ErrRes
// @Failure 401 {object} ErrRes
// @Failure 403 {object} ErrRes
// @Failure 404 {object} ErrRes
// @Failure 500 {object} ErrRes
func QueryCube(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr) {
	if rtbl.RejectUsr(c, u, ju, []usrtype.UsrType{usrtype.KEY, usrtype.APX, usrtype.VDR}) {
		return
	}
	if req, res, ok := rtreq.QueryCubeReqBind(c, u); ok {
		rtbl.QueryCube(c, u, ju, &req, &res)
	} else {
		rtbl.BadRequest(c, &res)
	}
}

// @Tags v1 Cube
// @Router /v1/cubes/memify [put]
// @Summary Cubeを自己強化する (Memify)
// @Description - USR によってのみ使用できる
// @Description - 指定したCubeの知識を強化・最適化する
// @Description - memory_groupで対象分野を指定
// @Accept application/json
// @Param Authorization header string true "token" example(Bearer ??????????)
// @Param json body MemifyCubeParam true "json"
// @Success 200 {object} MemifyCubeRes{errors=[]int}
// @Failure 400 {object} ErrRes
// @Failure 401 {object} ErrRes
// @Failure 403 {object} ErrRes
// @Failure 404 {object} ErrRes
// @Failure 500 {object} ErrRes
func MemifyCube(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr) {
	if rtbl.RejectUsr(c, u, ju, []usrtype.UsrType{usrtype.KEY, usrtype.APX, usrtype.VDR}) {
		return
	}
	if req, res, ok := rtreq.MemifyCubeReqBind(c, u); ok {
		rtbl.MemifyCube(c, u, ju, &req, &res)
	} else {
		rtbl.BadRequest(c, &res)
	}
}

// @Tags v1 Cube
// @Router /v1/cubes/delete [delete]
// @Summary Cubeを削除する (Delete)
// @Description - USR によってのみ使用できる
// @Description - 指定したCubeと関連データを完全に削除する
// @Param Authorization header string true "token" example(Bearer ??????????)
// @Param cube_id query int true "Cube ID"
// @Success 200 {object} DeleteCubeRes{errors=[]int}
// @Failure 400 {object} ErrRes
// @Failure 401 {object} ErrRes
// @Failure 403 {object} ErrRes
// @Failure 404 {object} ErrRes
// @Failure 500 {object} ErrRes
func DeleteCube(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr) {
	if rtbl.RejectUsr(c, u, ju, []usrtype.UsrType{usrtype.KEY, usrtype.APX, usrtype.VDR}) {
		return
	}
	if req, res, ok := rtreq.DeleteCubeReqBind(c, u); ok {
		rtbl.DeleteCube(c, u, ju, &req, &res)
	} else {
		rtbl.BadRequest(c, &res)
	}
}
