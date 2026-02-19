package hv1

import (
	"github.com/gin-gonic/gin"
	"github.com/t-kawata/mycute/enum/usrtype"
	"github.com/t-kawata/mycute/mode/rt/rtbl"
	"github.com/t-kawata/mycute/mode/rt/rtreq"
	"github.com/t-kawata/mycute/mode/rt/rtutil"
)

// @Tags v1 ChatModel
// @Router /v1/chat_models/search [post]
// @Summary ChatModelを検索
// @Description - USR によってのみ使用できる
// @Accept application/json
// @Produce application/json
// @Param Authorization header string true "token" example(Bearer ??????????)
// @Param params body rtparam.SearchChatModelsParam true "Search Params"
// @Success 200 {object} rtres.SearchChatModelsRes "Success"
// @Failure 400 {object} rtres.ErrRes "Validation Error"
// @Failure 401 {object} rtres.ErrRes "Unauthorized"
// @Failure 500 {object} rtres.ErrRes "Internal Server Error"
func SearchChatModels(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr) {
	if rtbl.RejectUsr(c, u, ju, []usrtype.UsrType{usrtype.KEY, usrtype.APX, usrtype.VDR}) { // USRのみ使用可能
		return
	}
	if req, res, ok := rtreq.SearchChatModelsReqBind(c, u); ok {
		rtbl.SearchChatModels(c, u, ju, &req, &res)
	} else {
		rtbl.BadRequest(c, &res)
	}
}

// @Tags v1 ChatModel
// @Router /v1/chat_models/{chat_model_id} [get]
// @Summary ChatModel詳細取得
// @Description - USR によってのみ使用できる
// @Produce application/json
// @Param Authorization header string true "token"
// @Param chat_model_id path int true "ChatModel ID"
// @Success 200 {object} rtres.GetChatModelRes
// @Failure 400 {object} rtres.ErrRes
// @Failure 401 {object} rtres.ErrRes
// @Failure 404 {object} rtres.ErrRes
// @Failure 500 {object} rtres.ErrRes
func GetChatModel(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr) {
	if rtbl.RejectUsr(c, u, ju, []usrtype.UsrType{usrtype.KEY, usrtype.APX, usrtype.VDR}) { // USRのみ使用可能
		return
	}
	if req, res, ok := rtreq.GetChatModelReqBind(c, u); ok {
		rtbl.GetChatModel(c, u, ju, &req, &res)
	} else {
		rtbl.BadRequest(c, &res)
	}
}

// @Tags v1 ChatModel
// @Router /v1/chat_models/ [post]
// @Summary ChatModel作成
// @Description - USR によってのみ使用できる
// @Accept application/json
// @Produce application/json
// @Param Authorization header string true "token"
// @Param json body rtparam.CreateChatModelParam true "json"
// @Success 200 {object} rtres.CreateChatModelRes
// @Failure 400 {object} rtres.ErrRes
// @Failure 401 {object} rtres.ErrRes
// @Failure 500 {object} rtres.ErrRes
func CreateChatModel(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr) {
	if rtbl.RejectUsr(c, u, ju, []usrtype.UsrType{usrtype.KEY, usrtype.APX, usrtype.VDR}) { // USRのみ使用可能
		return
	}
	if req, res, ok := rtreq.CreateChatModelReqBind(c, u); ok {
		rtbl.CreateChatModel(c, u, ju, &req, &res)
	} else {
		rtbl.BadRequest(c, &res)
	}
}

// @Tags v1 ChatModel
// @Router /v1/chat_models/{chat_model_id} [patch]
// @Summary ChatModel更新
// @Description - USR によってのみ使用できる
// @Accept application/json
// @Produce application/json
// @Param Authorization header string true "token"
// @Param chat_model_id path int true "ChatModel ID"
// @Param json body rtparam.UpdateChatModelParam true "json"
// @Success 200 {object} rtres.UpdateChatModelRes
// @Failure 400 {object} rtres.ErrRes
// @Failure 401 {object} rtres.ErrRes
// @Failure 404 {object} rtres.ErrRes
// @Failure 500 {object} rtres.ErrRes
func UpdateChatModel(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr) {
	if rtbl.RejectUsr(c, u, ju, []usrtype.UsrType{usrtype.KEY, usrtype.APX, usrtype.VDR}) { // USRのみ使用可能
		return
	}
	if req, res, ok := rtreq.UpdateChatModelReqBind(c, u, ju); ok {
		rtbl.UpdateChatModel(c, u, ju, &req, &res)
	} else {
		rtbl.BadRequest(c, &res)
	}
}

// @Tags v1 ChatModel
// @Router /v1/chat_models/{chat_model_id} [delete]
// @Summary ChatModel削除
// @Description - USR によってのみ使用できる
// @Produce application/json
// @Param Authorization header string true "token"
// @Param chat_model_id path int true "ChatModel ID"
// @Success 200 {object} rtres.DeleteChatModelRes
// @Failure 400 {object} rtres.ErrRes
// @Failure 401 {object} rtres.ErrRes
// @Failure 404 {object} rtres.ErrRes
// @Failure 500 {object} rtres.ErrRes
func DeleteChatModel(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr) {
	if rtbl.RejectUsr(c, u, ju, []usrtype.UsrType{usrtype.KEY, usrtype.APX, usrtype.VDR}) { // USRのみ使用可能
		return
	}
	if req, res, ok := rtreq.DeleteChatModelReqBind(c, u); ok {
		rtbl.DeleteChatModel(c, u, ju, &req, &res)
	} else {
		rtbl.BadRequest(c, &res)
	}
}
