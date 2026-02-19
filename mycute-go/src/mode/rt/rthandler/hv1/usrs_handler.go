package hv1

import (
	"net/http"

	"github.com/gin-gonic/gin"
	"github.com/t-kawata/mycute/enum/usrtype"
	"github.com/t-kawata/mycute/mode/rt/rtbl"
	"github.com/t-kawata/mycute/mode/rt/rtreq"
	"github.com/t-kawata/mycute/mode/rt/rtutil"
)

// @Tags v1 User
// @Router /v1/usrs/auth/{apx_id}/{vdr_id} [get]
// @Summary 認証を行い、tokenを返す。ロールや権限も一緒に返す。
// @Description ### 総則
// @Description - X-Key での認証時は、X-Key を入れ、apx_id=0、vdr_id=0、email & password はダミーを入力
// @Description - APX として認証する場合、apx_id=0、vdr_id=0、email & password は当該APXのもの
// @Description - VDR として認証する場合、apx_id=所属ApxID、vdr_id=0、email & password は当該VDRのもの
// @Description - USR として認証する場合、apx_id=所属ApxID、vdr_id=所属VdrID、email & password は当該USRのもの
// @Description - expire は hour で指定すること
// @Description ### スタッフについて
// @Description - USRは、VDR の権限により、VDRのスタッフになることができる
// @Description - スタッフとしての立場を与えられた USRは、その後、スタッフとしての token のみを取得できる
// @Description - スタッフ token を使用した場合、システム内で常に VDR として振る舞うことになる
// @Description - その場合、全ての操作は当該 VDR が行ったものと同一の結果となる
// @Description - 行った操作が、どのスタッフによるものか記録したい場合は、token payload 内の usr_id で記録できる
// @Description - システム内部においては、ju.StaffID がそれにあたる
// @Description ### 注意
// @Description - スタッフであるかどうかの確認は、tokenの取得のタイミングで1度だけ行われる
// @Description - 取得した token が、スタッフであるか否かを示す唯一の証明書である
// @Description - 当該 USR が真にスタッフであるかを問わず、システムは token によってのみスタッフか否かを判断する
// @Description - つまり、スタッフ token を取得後、VDR により当該 USR がスタッフ権限を剥奪されたとしても、当該 token の expire までは、そのスタッフ token は有効である
// @Accept application/json
// @Param X-Key header string false "key" example(??????????)
// @Param apx_id path int true "apx_id"
// @Param vdr_id path int true "vdr_id"
// @Param email query string true "email"
// @Param password query string true "password"
// @Param expire query number true "expire" example(24)
// @Success 200 {object} AuthUsrRes{errors=[]int}
// @Failure 400 {object} ErrRes
// @Failure 401 {object} ErrRes
// @Failure 403 {object} ErrRes
// @Failure 404 {object} ErrRes
func AuthUsr(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr) {
	if req, res, ok, hasKey, isValidKey := rtreq.AuthUsrReqBind(c, u); ok {
		if !hasKey {
			rtbl.AuthUsrByParam(c, u, ju, &req, &res)
			return
		}
		if isValidKey {
			rtbl.AuthUsrByKey(c, u, ju, &req, &res)
			return
		}
		c.JSON(http.StatusUnauthorized, res)
	} else {
		rtbl.BadRequest(c, &res)
	}
}

// @Tags v1 User
// @Router /v1/usrs/search [post]
// @Summary ユーザを検索する。
// @Description - Keyは全てのユーザを検索できる
// @Description - Apxは配下のVdr以下の全てのユーザを検索できる
// @Description - Vdrは、配下の全てのユーザを検索できる
// @Description - Usr は使用できない
// @Accept application/json
// @Param Authorization header string true "token" example(Bearer ??????????)
// @Param json body SearchUsrParam true "json"
// @Success 200 {object} SearchUsrsRes{errors=[]int}
// @Failure 400 {object} ErrRes
// @Failure 401 {object} ErrRes
// @Failure 403 {object} ErrRes
// @Failure 404 {object} ErrRes
func SearchUsrs(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr) {
	if rtbl.RejectUsr(c, u, ju, []usrtype.UsrType{usrtype.USR}) {
		return
	}
	if req, res, ok := rtreq.SearchUsrsReqBind(c, u); ok {
		rtbl.SearchUsrs(c, u, ju, &req, &res)
	} else {
		rtbl.BadRequest(c, &res)
	}
}

// @Tags v1 User
// @Router /v1/usrs/{usr_id} [get]
// @Summary ユーザ情報を1件取得する。
// @Description - Keyは全てのユーザを取得できる
// @Description - Apxは配下のVdr以下の全てのユーザを取得できる
// @Description - Vdrは、配下の全てのユーザを取得できる
// @Description - Usr は使用できない
// @Accept application/json
// @Param Authorization header string true "token" example(Bearer ??????????)
// @Param usr_id path int true "ユーザID"
// @Success 200 {object} GetUsrRes{errors=[]int}
// @Failure 400 {object} ErrRes
// @Failure 401 {object} ErrRes
// @Failure 403 {object} ErrRes
// @Failure 404 {object} ErrRes
func GetUsr(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr) {
	if rtbl.RejectUsr(c, u, ju, []usrtype.UsrType{usrtype.USR}) {
		return
	}
	if req, res, ok := rtreq.GetUsrReqBind(c, u); ok {
		rtbl.GetUsr(c, u, ju, &req, &res)
	} else {
		rtbl.BadRequest(c, &res)
	}
}

// @Tags v1 User
// @Router /v1/usrs/ [post]
// @Summary ユーザを作成する。
// @Description - Key で取得した token では Apx のみを作成できる
// @Description - Apx で取得した token では Vdr のみを作成できる
// @Description - Vdr で取得した token では Usr のみを作成できる
// @Description - Usr は Usr を作れない
// @Description ### パラメータについて
// @Description - type: 1: 法人, 2: 個人 (VDR作成時は無視される)
// @Description - VDR作成時以外にVDR用項目を送信するとエラーとなる
// @Description - 法人作成時以外に法人用項目を送信するとエラーとなる
// @Description ### name について
// @Description - type=2 (個人) の場合、姓名の間にスペース（半角・全角問わず）が必須
// @Description - 全角スペースは半角スペースに変換され、連続するスペースは1つにまとめられる
// @Accept application/json
// @Param Authorization header string true "token" example(Bearer ??????????)
// @Param json body CreateUsrParam true "json"
// @Success 200 {object} CreateUsrRes{errors=[]int}
// @Failure 400 {object} ErrRes
// @Failure 401 {object} ErrRes
// @Failure 403 {object} ErrRes
// @Failure 404 {object} ErrRes
func CreateUsr(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr) {
	if rtbl.RejectUsr(c, u, ju, []usrtype.UsrType{usrtype.USR}) {
		return
	}
	if req, res, ok := rtreq.CreateUsrReqBind(c, u, ju); ok {
		rtbl.CreateUsr(c, u, ju, &req, &res)
	} else {
		rtbl.BadRequest(c, &res)
	}
}

// @Tags v1 User
// @Router /v1/usrs/{usr_id} [patch]
// @Summary ユーザ情報を更新する。
// @Description - Keyは安全の為、更新権限を持たない
// @Description - Apxは配下のVdr以下の全てのユーザを更新できる
// @Description - Vdrは、配下の全てのユーザを更新できる
// @Description - Usrは使用できない
// @Description ### パラメータについて
// @Description - type: 1: 法人, 2: 個人 (更新時は変更不可の場合がある)
// @Description - VDR以外にVDR用項目を送信するとエラーとなる
// @Description - 法人以外に法人用項目を送信するとエラーとなる
// @Description ### name について
// @Description - type=2 (個人) の場合、姓名の間にスペース（半角・全角問わず）が必須
// @Description - 全角スペースは半角スペースに変換され、連続するスペースは1つにまとめられる
// @Accept application/json
// @Param Authorization header string true "token" example(Bearer ??????????)
// @Param usr_id path int true "ユーザID"
// @Param json body UpdateUsrParam true "json"
// @Success 200 {object} UpdateUsrRes{errors=[]int}
// @Failure 400 {object} ErrRes
// @Failure 401 {object} ErrRes
// @Failure 403 {object} ErrRes
// @Failure 404 {object} ErrRes
func UpdateUsr(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr) {
	if rtbl.RejectUsr(c, u, ju, []usrtype.UsrType{usrtype.KEY, usrtype.USR}) {
		return
	}
	if req, res, ok := rtreq.UpdateUsrReqBind(c, u, ju); ok {
		rtbl.UpdateUsr(c, u, ju, &req, &res)
	} else {
		rtbl.BadRequest(c, &res)
	}
}

// @Tags v1 User
// @Router /v1/usrs/{usr_id} [delete]
// @Summary ユーザを削除する。
// @Description - Keyは安全の為、削除権限を持たない
// @Description - Apxは配下のVdrのみを削除できる
// @Description - Vdrは、配下の全てのユーザを削除できる
// @Description - Usrは使用できない
// @Accept application/json
// @Param Authorization header string true "token" example(Bearer ??????????)
// @Param usr_id path int true "ユーザID"
// @Success 200 {object} DeleteUsrRes{errors=[]int}
// @Failure 400 {object} ErrRes
// @Failure 401 {object} ErrRes
// @Failure 403 {object} ErrRes
// @Failure 404 {object} ErrRes
func DeleteUsr(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr) {
	if rtbl.RejectUsr(c, u, ju, []usrtype.UsrType{usrtype.KEY, usrtype.USR}) {
		return
	}
	if req, res, ok := rtreq.DeleteUsrReqBind(c, u); ok {
		rtbl.DeleteUsr(c, u, ju, &req, &res)
	} else {
		rtbl.BadRequest(c, &res)
	}
}

// @Tags v1 User
// @Router /v1/usrs/{usr_id}/hire [patch]
// @Summary ユーザにベンダーのスタッフとしての権限を与える。
// @Description - Vdr によってのみ使用できる
// @Description - スタッフは Vdr として振る舞うため、同様に使用できる
// @Accept application/json
// @Param Authorization header string true "token" example(Bearer ??????????)
// @Param usr_id path int true "ユーザID"
// @Success 200 {object} HireUsrRes{errors=[]int}
// @Failure 400 {object} ErrRes
// @Failure 401 {object} ErrRes
// @Failure 403 {object} ErrRes
// @Failure 404 {object} ErrRes
func HireUsr(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr) {
	if rtbl.RejectUsr(c, u, ju, []usrtype.UsrType{usrtype.KEY, usrtype.APX, usrtype.USR}) {
		return
	}
	if req, res, ok := rtreq.HireUsrReqBind(c, u); ok {
		rtbl.HireUsr(c, u, ju, &req, &res)
	} else {
		rtbl.BadRequest(c, &res)
	}
}

// @Tags v1 User
// @Router /v1/usrs/{usr_id}/hire [delete]
// @Summary ユーザからベンダーのスタッフとしての権限を剥奪する。
// @Description - Vdr によってのみ使用できる
// @Description - スタッフは Vdr として振る舞うため、同様に使用できる
// @Accept application/json
// @Param Authorization header string true "token" example(Bearer ??????????)
// @Param usr_id path int true "ユーザID"
// @Success 200 {object} DehireUsrRes{errors=[]int}
// @Failure 400 {object} ErrRes
// @Failure 401 {object} ErrRes
// @Failure 403 {object} ErrRes
// @Failure 404 {object} ErrRes
func DehireUsr(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr) {
	if rtbl.RejectUsr(c, u, ju, []usrtype.UsrType{usrtype.KEY, usrtype.APX, usrtype.USR}) {
		return
	}
	if req, res, ok := rtreq.DehireUsrReqBind(c, u); ok {
		rtbl.DehireUsr(c, u, ju, &req, &res)
	} else {
		rtbl.BadRequest(c, &res)
	}
}
