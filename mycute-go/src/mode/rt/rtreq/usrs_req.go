package rtreq

import (
	"github.com/gin-gonic/gin"
	"github.com/t-kawata/mycute/cutil"
	"github.com/t-kawata/mycute/enum/acctype"
	"github.com/t-kawata/mycute/enum/rterr"
	"github.com/t-kawata/mycute/lib/common"
	"github.com/t-kawata/mycute/mode/rt/rtres"
	"github.com/t-kawata/mycute/mode/rt/rtutil"
)

type AuthUsrReq struct {
	Key      string
	ApxID    string `binding:"number"`
	VdrID    string `binding:"number"`
	Email    string `form:"email" binding:"required,email,half"`
	Password string `form:"password" binding:"required,max=100"`
	Expire   uint   `form:"expire" binding:"required,gte=1"`
}

func AuthUsrReqBind(c *gin.Context, u *rtutil.RtUtil) (AuthUsrReq, rtres.AuthUsrRes, bool, bool, bool) {
	ok := true
	hasKey := false
	isValidKey := false
	key := c.Request.Header.Get("X-Key")
	req := AuthUsrReq{Key: key, ApxID: c.Param("apx_id"), VdrID: c.Param("vdr_id"), Expire: common.StrToUint(c.Query("expire"))}
	res := rtres.AuthUsrRes{Errors: []rtres.Err{}}
	if len(key) > 0 {
		hasKey = true
		if u.IsValidKey(key) {
			isValidKey = true
		} else {
			res.Errors = []rtres.Err{{Field: "key", Code: rterr.ValidKey.Code(), Message: rterr.ValidKey.Msg()}}
		}
		return req, res, ok, hasKey, isValidKey
	}
	if err := c.ShouldBind(&req); err != nil {
		res.Errors = u.GetValidationErrs(err)
		ok = false
	}
	return req, res, ok, hasKey, isValidKey
}

type SearchUsrsReq struct {
	Name   string `json:"name" binding:"max=50"`
	Email  string `json:"email" binding:"email,half,max=50"`
	BgnAt  string `json:"bgn_at" binding:"required,datetime"`
	EndAt  string `json:"end_at" binding:"required,datetime"`
	Limit  uint16 `json:"limit" binding:"gte=1,lte=25"`
	Offset uint16 `json:"offset" binding:"gte=0"`
}

func SearchUsrsReqBind(c *gin.Context, u *rtutil.RtUtil) (SearchUsrsReq, rtres.SearchUsrsRes, bool) {
	ok := true
	req := SearchUsrsReq{}
	res := rtres.SearchUsrsRes{Errors: []rtres.Err{}}
	if err := c.ShouldBindJSON(&req); err != nil {
		res.Errors = u.GetValidationErrs(err)
		ok = false
	}
	return req, res, ok
}

type GetUsrReq struct {
	ID uint `json:"usr_id" binding:"gte=1"`
}

func GetUsrReqBind(c *gin.Context, u *rtutil.RtUtil) (GetUsrReq, rtres.GetUsrRes, bool) {
	ok := true
	req := GetUsrReq{ID: common.StrToUint(c.Param("usr_id"))}
	res := rtres.GetUsrRes{Errors: []rtres.Err{}}
	if err := c.ShouldBind(&req); err != nil {
		res.Errors = u.GetValidationErrs(err)
		ok = false
	}
	return req, res, ok
}

type CreateUsrReq struct {
	Name     string `json:"name" binding:"required,max=50"`
	Email    string `json:"email" binding:"required,email,half,max=50"`
	Password string `json:"password" binding:"required,password"`
	BgnAt    string `json:"bgn_at" binding:"required,datetime"`
	EndAt    string `json:"end_at" binding:"required,datetime"`
	Type     uint8  `json:"type" binding:"omitempty,oneof=1 2"`
}

func CreateUsrReqBind(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr) (CreateUsrReq, rtres.CreateUsrRes, bool) {
	ok := true
	req := CreateUsrReq{}
	res := rtres.CreateUsrRes{Errors: []rtres.Err{}}
	if err := c.ShouldBindJSON(&req); err == nil {
		if !cutil.IsUniqueByUsrForCreateUsr(u.Logger, u.DB, ju, "email", &req.Email) {
			res.Errors = append(res.Errors, rtres.Err{Field: "email", Code: rterr.UniqueByUsr.Code(), Message: rterr.UniqueByUsr.Msg()})
		}

		if ju.IsFromKey() {
			// Creating APX
			// Key を使った登録の場合、name/email/password/bgn_at/end_at だけが必須でそれ以外は不要
			if req.Name == "" {
				res.Errors = append(res.Errors, rtres.Err{Field: "name", Code: rterr.Required.Code(), Message: rterr.Required.Msg()})
			}
			if req.Email == "" {
				res.Errors = append(res.Errors, rtres.Err{Field: "email", Code: rterr.Required.Code(), Message: rterr.Required.Msg()})
			}
			if req.Password == "" {
				res.Errors = append(res.Errors, rtres.Err{Field: "password", Code: rterr.Required.Code(), Message: rterr.Required.Msg()})
			}
			if req.BgnAt == "" {
				res.Errors = append(res.Errors, rtres.Err{Field: "bgn_at", Code: rterr.Required.Code(), Message: rterr.Required.Msg()})
			}
			if req.EndAt == "" {
				res.Errors = append(res.Errors, rtres.Err{Field: "end_at", Code: rterr.Required.Code(), Message: rterr.Required.Msg()})
			}
		} else if ju.IsApx() {
			// Creating VDR
		} else {
			// この場合は、Typeが必須
			if req.Type == 0 {
				res.Errors = append(res.Errors, rtres.Err{Field: "type", Code: rterr.Required.Code(), Message: rterr.Required.Msg()})
			}
			// Creating Usr (Corp or Indiv)
			if req.Type == acctype.CORP.Val() {
				// Creating Corp
			} else if req.Type == acctype.INDI.Val() {
				// Creating Indiv
			}
		}

		if len(res.Errors) > 0 {
			ok = false
		}
	} else {
		res.Errors = u.GetValidationErrs(err)
		ok = false
	}
	return req, res, ok
}

type UpdateUsrReq struct {
	ID       uint   `json:"id" binding:"gte=1"`
	Name     string `json:"name" binding:"max=50"`
	Email    string `json:"email" binding:"email,half,max=50"`
	Password string `json:"password" binding:"password"`
	BgnAt    string `json:"bgn_at" binding:"datetime"`
	EndAt    string `json:"end_at" binding:"datetime"`
	Type     uint8  `json:"type" binding:"oneof=1 2"`
}

func UpdateUsrReqBind(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr) (UpdateUsrReq, rtres.UpdateUsrRes, bool) {
	ok := true
	id := common.StrToUint(c.Param("usr_id"))
	req := UpdateUsrReq{ID: id}
	res := rtres.UpdateUsrRes{Errors: []rtres.Err{}}
	if err := c.ShouldBindJSON(&req); err == nil {
		if !cutil.IsUniqueByUsrForCreateUsr(u.Logger, u.DB, ju, "email", &req.Email) {
			res.Errors = append(res.Errors, rtres.Err{Field: "email", Code: rterr.UniqueByUsr.Code(), Message: rterr.UniqueByUsr.Msg()})
		}

		if ju.IsApx() {
			// Updating Vdr
		} else {
			// Updating Usr (Corp or Indiv)
			if req.Type == acctype.CORP.Val() {
				// Updating Corp
			} else if req.Type == acctype.INDI.Val() {
				// Updating Indiv
			}
		}

		if len(res.Errors) > 0 {
			ok = false
		}
	} else {
		res.Errors = u.GetValidationErrs(err)
		ok = false
	}
	return req, res, ok
}

type DeleteUsrReq struct {
	ID uint `json:"id" binding:"gte=1"`
}

func DeleteUsrReqBind(c *gin.Context, u *rtutil.RtUtil) (DeleteUsrReq, rtres.DeleteUsrRes, bool) {
	ok := true
	req := DeleteUsrReq{ID: common.StrToUint(c.Param("usr_id"))}
	res := rtres.DeleteUsrRes{Errors: []rtres.Err{}}
	if err := c.ShouldBind(&req); err != nil {
		res.Errors = u.GetValidationErrs(err)
		ok = false
	}
	return req, res, ok
}

type HireUsrReq struct {
	ID uint `json:"id" binding:"gte=1"`
}

func HireUsrReqBind(c *gin.Context, u *rtutil.RtUtil) (HireUsrReq, rtres.HireUsrRes, bool) {
	ok := true
	req := HireUsrReq{ID: common.StrToUint(c.Param("usr_id"))}
	res := rtres.HireUsrRes{Errors: []rtres.Err{}}
	if err := c.ShouldBind(&req); err != nil {
		res.Errors = u.GetValidationErrs(err)
		ok = false
	}
	return req, res, ok
}

type DehireUsrReq struct {
	ID uint `json:"id" binding:"gte=1"`
}

func DehireUsrReqBind(c *gin.Context, u *rtutil.RtUtil) (DehireUsrReq, rtres.DehireUsrRes, bool) {
	ok := true
	req := DehireUsrReq{ID: common.StrToUint(c.Param("usr_id"))}
	res := rtres.DehireUsrRes{Errors: []rtres.Err{}}
	if err := c.ShouldBind(&req); err != nil {
		res.Errors = u.GetValidationErrs(err)
		ok = false
	}
	return req, res, ok
}

type ActivateLnByUsrReq struct {
	UsrID uint   `json:"usr_id" binding:"gte=1"`
	LnID  uint   `json:"ln_id" binding:"gte=1"`
	BgnAt string `json:"bgn_at" binding:"datetime"`
	EndAt string `json:"end_at" binding:"datetime"`
}

func ActivateLnByUsrReqBind(c *gin.Context, u *rtutil.RtUtil) (ActivateLnByUsrReq, rtres.ActivateLnByUsrRes, bool) {
	ok := true
	req := ActivateLnByUsrReq{UsrID: common.StrToUint(c.Param("usr_id")), LnID: common.StrToUint(c.Param("ln_id"))}
	res := rtres.ActivateLnByUsrRes{Errors: []rtres.Err{}}
	if err := c.ShouldBindJSON(&req); err != nil {
		res.Errors = u.GetValidationErrs(err)
		ok = false
	}
	return req, res, ok
}

type DeactivateLnByUsrReq struct {
	UsrID uint `json:"usr_id" binding:"gte=1"`
	LnID  uint `json:"ln_id" binding:"gte=1"`
}

func DeactivateLnByUsrReqBind(c *gin.Context, u *rtutil.RtUtil) (DeactivateLnByUsrReq, rtres.DeactivateLnByUsrRes, bool) {
	ok := true
	req := DeactivateLnByUsrReq{UsrID: common.StrToUint(c.Param("usr_id")), LnID: common.StrToUint(c.Param("ln_id"))}
	res := rtres.DeactivateLnByUsrRes{Errors: []rtres.Err{}}
	if err := c.ShouldBind(&req); err != nil {
		res.Errors = u.GetValidationErrs(err)
		ok = false
	}
	return req, res, ok
}

type KickLnByUsrReq struct {
	UsrID uint `json:"usr_id" binding:"gte=1"`
	LnID  uint `json:"ln_id" binding:"gte=1"`
}

func KickLnByUsrReqBind(c *gin.Context, u *rtutil.RtUtil) (KickLnByUsrReq, rtres.KickLnByUsrRes, bool) {
	ok := true
	req := KickLnByUsrReq{UsrID: common.StrToUint(c.Param("usr_id")), LnID: common.StrToUint(c.Param("ln_id"))}
	res := rtres.KickLnByUsrRes{Errors: []rtres.Err{}}
	if err := c.ShouldBind(&req); err != nil {
		res.Errors = u.GetValidationErrs(err)
		ok = false
	}
	return req, res, ok
}
