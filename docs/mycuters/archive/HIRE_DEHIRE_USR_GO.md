Go言語で実装されているユーザーの雇用/解雇に関連するコードを以下にスニペットとして貼り付けます。

src/mode/rt/rthandler/hv1/usrs_handler.go
```go
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
```

src/mode/rt/rtparam/usrs_param.go
```go
// なし
```

src/mode/rt/rtreq/usrs_req.go
```go
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
```

src/mode/rt/rtres/usrs_res.go
```go
type HireUsrResData struct {
} // @name HireUsrResData

type HireUsrRes struct {
	Data   HireUsrResData `json:"data"`
	Errors []Err          `json:"errors"`
} // @name HireUsrRes

type DehireUsrResData struct {
} // @name DehireUsrResData

type DehireUsrRes struct {
	Data   DehireUsrResData `json:"data"`
	Errors []Err            `json:"errors"`
} // @name DehireUsrRes
```

src/mode/rt/rtbl/usrs_bl.go
```go
func HireUsr(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.HireUsrReq, res *rtres.HireUsrRes) bool {
	usr := model.Usr{}
	ids := ju.IDs(true)
	r := u.DB.Select("`usrs`.`id`", "`usrs`.`is_staff`").Where(
		"`usrs`.`apx_id` = ? AND `usrs`.`vdr_id` = ? AND `usrs`.`id` = ? AND `usrs`.`is_staff` = 0",
		ids.ApxID,
		ids.VdrID,
		req.ID,
	).First(&usr)
	if r.Error != nil {
		return NotFound(c, res)
	}
	err := common.UpdateSingleTable(u.DB, "usrs", &usr, &struct{ IsStaff bool }{IsStaff: true})
	if err != nil {
		return InternalServerError(c, res)
	}
	data := rtres.HireUsrResData{}
	return OK(c, &data, res)
}

func DehireUsr(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.DehireUsrReq, res *rtres.DehireUsrRes) bool {
	usr := model.Usr{}
	ids := ju.IDs(true)
	r := u.DB.Select("`usrs`.`id`", "`usrs`.`is_staff`").Where(
		"`usrs`.`apx_id` = ? AND `usrs`.`vdr_id` = ? AND `usrs`.`id` = ? AND `usrs`.`is_staff` = 1",
		ids.ApxID,
		ids.VdrID,
		req.ID,
	).First(&usr)
	if r.Error != nil {
		return NotFound(c, res)
	}
	err := common.UpdateSingleTable(u.DB, "usrs", &usr, &struct{ IsStaff bool }{IsStaff: false})
	if err != nil {
		return InternalServerError(c, res)
	}
	data := rtres.DehireUsrResData{}
	return OK(c, &data, res)
}
```