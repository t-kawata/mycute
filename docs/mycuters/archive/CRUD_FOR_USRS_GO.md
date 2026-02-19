Go言語で実装されているユーザーのCRUDに関連するコードを以下にスニペットとして貼り付けます。

src/mode/rt/rthandler/hv1/usrs_handler.go
```go
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
// @Description - base_point: VDRのみ必須 (バッジ授与時に授与者である個人に付与される基本ポイント数)
// @Description - belong_rate: VDRのみ必須 (所属によるポイント割増率)
// @Description - max_works: VDRのみ必須 (Vdr内の個人が就労できる最大数)
// @Description - flush_fee_rate: VDRのみ必須 (現金プールを現金分配実行する時に、事務コストを賄うために Pool から引かれる割合)
// @Description - flush_days: 法人のみ必須 (現金プールを現金分配実行するためのサイクルとなる日数)
// @Description - rate: 法人のみ必須 (法人が、自分に所属するユーザーに対して付与する割増ポイント率)
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
// @Description - base_point: VDRのみ (バッジ授与時に授与者である個人に付与される基本ポイント数)
// @Description - belong_rate: VDRのみ (所属によるポイント割増率)
// @Description - max_works: VDRのみ (Vdr内の個人が就労できる最大数)
// @Description - flush_fee_rate: VDRのみ必須 (現金プールを現金分配実行する時に、事務コストを賄うために Pool から引かれる割合)
// @Description - flush_days: 法人のみ (現金プールを現金分配実行するためのサイクルとなる日数)
// @Description - rate: 法人のみ (法人が、自分に所属するユーザーに対して付与する割増ポイント率)
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
```

src/mode/rt/rtparam/usrs_param.go
```go
type SearchUsrParam struct {
	Name   string    `json:"name" swaggertype:"string" format:"" example:"User01"`
	Email  string    `json:"email" swaggertype:"string" format:"email" example:"sample@example.com"`
	BgnAt  time.Time `json:"bgn_at" swaggertype:"string" format:"date-time" example:"2023-01-01T00:00:00"`
	EndAt  time.Time `json:"end_at" swaggertype:"string" format:"date-time" example:"2100-12-31T23:59:59"`
	Limit  uint16    `json:"limit" swaggertype:"integer" format:"" example:"10"`
	Offset uint16    `json:"offset" swaggertype:"integer" format:"" example:"0"`
} // @name SearchUsrParam

type CreateUsrParam struct {
	Name         string    `json:"name" swaggertype:"string" format:"" example:"User01"`
	Email        string    `json:"email" swaggertype:"string" format:"email" example:"sample@example.com"`
	Password     string    `json:"password" swaggertype:"string" format:"password" example:"ta5!CAzQz8DjMydju?"`
	BgnAt        time.Time `json:"bgn_at" swaggertype:"string" format:"date-time" example:"2023-01-01T00:00:00"`
	EndAt        time.Time `json:"end_at" swaggertype:"string" format:"date-time" example:"2100-12-31T23:59:59"`
	Type         uint8     `json:"type" swaggertype:"integer" format:"" example:"1"`
	BasePoint    uint      `json:"base_point" swaggertype:"integer" format:"" example:"100"`
	BelongRate   float64   `json:"belong_rate" swaggertype:"number" format:"" example:"0.1"`
	MaxWorks     uint      `json:"max_works" swaggertype:"integer" format:"" example:"5"`
	FlushDays    uint      `json:"flush_days" swaggertype:"integer" format:"" example:"30"`
	Rate         float64   `json:"rate" swaggertype:"number" format:"" example:"0.05"`
	FlushFeeRate float64   `json:"flush_fee_rate" swaggertype:"number" format:"" example:"0.05"`
} // @name CreateUsrParam

type UpdateUsrParam struct {
	Name         string    `json:"name" swaggertype:"string" format:"" example:"User01"`
	Email        string    `json:"email" swaggertype:"string" format:"email" example:"sample@example.com"`
	Password     string    `json:"password" swaggertype:"string" format:"password" example:"ta5!CAzQz8DjMydju?"`
	BgnAt        time.Time `json:"bgn_at" swaggertype:"string" format:"date-time" example:"2023-01-01T00:00:00"`
	EndAt        time.Time `json:"end_at" swaggertype:"string" format:"date-time" example:"2100-12-31T23:59:59"`
	Type         uint8     `json:"type" swaggertype:"integer" format:"" example:"1"`
	BasePoint    uint      `json:"base_point" swaggertype:"integer" format:"" example:"100"`
	BelongRate   float64   `json:"belong_rate" swaggertype:"number" format:"" example:"0.1"`
	MaxWorks     uint      `json:"max_works" swaggertype:"integer" format:"" example:"5"`
	FlushDays    uint      `json:"flush_days" swaggertype:"integer" format:"" example:"30"`
	Rate         float64   `json:"rate" swaggertype:"number" format:"" example:"0.05"`
	FlushFeeRate float64   `json:"flush_fee_rate" swaggertype:"number" format:"" example:"0.05"`
} // @name UpdateUsrParam
```

src/mode/rt/rtreq/usrs_req.go
```go
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
	Name         string  `json:"name" binding:"required,max=50"`
	Email        string  `json:"email" binding:"required,email,half,max=50"`
	Password     string  `json:"password" binding:"required,password"`
	BgnAt        string  `json:"bgn_at" binding:"required,datetime"`
	EndAt        string  `json:"end_at" binding:"required,datetime"`
	Type         uint8   `json:"type" binding:"omitempty,oneof=1 2"`
	BasePoint    uint    `json:"base_point" binding:"gte=0"`
	BelongRate   float64 `json:"belong_rate" binding:"gte=0"`
	MaxWorks     uint    `json:"max_works" binding:"gte=0"`
	FlushDays    uint    `json:"flush_days" binding:"gte=0"`
	Rate         float64 `json:"rate" binding:"gte=0"`
	FlushFeeRate float64 `json:"flush_fee_rate" binding:"gte=0"`
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
			// 不要なものが入っていたらエラー
			if req.Type > 0 || req.BasePoint > 0 || req.BelongRate > 0 || req.MaxWorks > 0 || req.FlushDays > 0 || req.Rate > 0 || req.FlushFeeRate > 0 {
				res.Errors = append(res.Errors, rtres.Err{Field: "system", Code: rterr.BadRequest.Code(), Message: "These (type, base_point, belong_rate, max_works, flush_days, rate, flush_fee_rate) are unnecessary parameters."})
			}
		} else if ju.IsApx() {
			// Creating VDR
			if req.BasePoint == 0 {
				res.Errors = append(res.Errors, rtres.Err{Field: "base_point", Code: rterr.Required.Code(), Message: rterr.Required.Msg()})
			}
			if req.BelongRate == 0 {
				res.Errors = append(res.Errors, rtres.Err{Field: "belong_rate", Code: rterr.Required.Code(), Message: rterr.Required.Msg()})
			}
			if req.MaxWorks == 0 {
				res.Errors = append(res.Errors, rtres.Err{Field: "max_works", Code: rterr.Required.Code(), Message: rterr.Required.Msg()})
			}
			if req.FlushFeeRate == 0 {
				res.Errors = append(res.Errors, rtres.Err{Field: "flush_fee_rate", Code: rterr.Required.Code(), Message: rterr.Required.Msg()})
			}
			if req.FlushDays > 0 || req.Rate > 0 || req.Type > 0 {
				res.Errors = append(res.Errors, rtres.Err{Field: "system", Code: rterr.BadRequest.Code(), Message: "These (flush_days, rate, type) are unnecessary parameters."})
			}
		} else {
			// この場合は、Typeが必須
			if req.Type == 0 {
				res.Errors = append(res.Errors, rtres.Err{Field: "type", Code: rterr.Required.Code(), Message: rterr.Required.Msg()})
			}
			// Creating Usr (Corp or Indiv)
			if req.BasePoint > 0 || req.BelongRate > 0 || req.MaxWorks > 0 || req.FlushFeeRate > 0 {
				res.Errors = append(res.Errors, rtres.Err{Field: "system", Code: rterr.BadRequest.Code(), Message: "These (base_point, belong_rate, max_works, flush_fee_rate) are unnecessary parameters."})
			}

			if req.Type == acctype.CORP.Val() {
				// Creating Corp
				if req.FlushDays == 0 {
					res.Errors = append(res.Errors, rtres.Err{Field: "flush_days", Code: rterr.Required.Code(), Message: rterr.Required.Msg()})
				}
				if req.Rate == 0 {
					res.Errors = append(res.Errors, rtres.Err{Field: "rate", Code: rterr.Required.Code(), Message: rterr.Required.Msg()})
				}
			} else if req.Type == acctype.INDI.Val() {
				// Creating Indiv
				if req.FlushDays > 0 || req.Rate > 0 {
					res.Errors = append(res.Errors, rtres.Err{Field: "system", Code: rterr.BadRequest.Code(), Message: "These (flush_days, rate) are unnecessary parameters."})
				}
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
	ID           uint    `json:"id" binding:"gte=1"`
	Name         string  `json:"name" binding:"max=50"`
	Email        string  `json:"email" binding:"email,half,max=50"`
	Password     string  `json:"password" binding:"password"`
	BgnAt        string  `json:"bgn_at" binding:"datetime"`
	EndAt        string  `json:"end_at" binding:"datetime"`
	Type         uint8   `json:"type" binding:"oneof=1 2"`
	BasePoint    uint    `json:"base_point" binding:"gte=0"`
	BelongRate   float64 `json:"belong_rate" binding:"gte=0"`
	MaxWorks     uint    `json:"max_works" binding:"gte=0"`
	FlushDays    uint    `json:"flush_days" binding:"gte=0"`
	Rate         float64 `json:"rate" binding:"gte=0"`
	FlushFeeRate float64 `json:"flush_fee_rate" binding:"gte=0"`
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
			if req.FlushDays > 0 || req.Rate > 0 {
				res.Errors = append(res.Errors, rtres.Err{Field: "system", Code: rterr.BadRequest.Code(), Message: "These (flush_days, rate) are unnecessary parameters."})
			}
		} else {
			// Updating Usr (Corp or Indiv)
			if req.BasePoint > 0 || req.BelongRate > 0 || req.MaxWorks > 0 || req.FlushFeeRate > 0 {
				res.Errors = append(res.Errors, rtres.Err{Field: "system", Code: rterr.BadRequest.Code(), Message: "These (base_point, belong_rate, max_works, flush_fee_rate) are unnecessary parameters."})
			}

			if req.Type == acctype.CORP.Val() {
				// Updating Corp
				if req.FlushDays == 0 {
					res.Errors = append(res.Errors, rtres.Err{Field: "flush_days", Code: rterr.Required.Code(), Message: rterr.Required.Msg()})
				}
				if req.Rate == 0 {
					res.Errors = append(res.Errors, rtres.Err{Field: "rate", Code: rterr.Required.Code(), Message: rterr.Required.Msg()})
				}
			} else if req.Type == acctype.INDI.Val() {
				// Updating Indiv
				if req.FlushDays > 0 || req.Rate > 0 {
					res.Errors = append(res.Errors, rtres.Err{Field: "system", Code: rterr.BadRequest.Code(), Message: "These (flush_days, rate) are unnecessary parameters."})
				}
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
```

src/mode/rt/rtres/usrs_res.go
```go
type SearchUsrsResData struct {
	ID           uint    `json:"id" swaggertype:"integer" format:"" example:"1"`
	ApxID        uint    `json:"apx_id" swaggertype:"integer" format:"" example:"1"`
	VdrID        uint    `json:"vdr_id" swaggertype:"integer" format:"" example:"1"`
	Name         string  `json:"name" swaggertype:"string" format:"" example:"User01"`
	Email        string  `json:"email" swaggertype:"string" format:"email" example:"sample@example.com"`
	BgnAt        string  `json:"bgn_at" swaggertype:"string" format:"date-time" example:"2023-01-01T00:00:00"`
	EndAt        string  `json:"end_at" swaggertype:"string" format:"date-time" example:"2023-01-01T00:00:00"`
	Type         uint8   `json:"type" swaggertype:"integer" format:"" example:"1"`
	BasePoint    uint    `json:"base_point" swaggertype:"integer" format:"" example:"100"`
	BelongRate   float64 `json:"belong_rate" swaggertype:"number" format:"" example:"0.1"`
	MaxWorks     uint    `json:"max_works" swaggertype:"integer" format:"" example:"5"`
	FlushDays    uint    `json:"flush_days" swaggertype:"integer" format:"" example:"30"`
	Rate         float64 `json:"rate" swaggertype:"number" format:"" example:"0.05"`
	FlushFeeRate float64 `json:"flush_fee_rate" swaggertype:"number" format:"" example:"0.05"`
} // @name SearchUsrsResData

func (d *SearchUsrsResData) Of(usrs *[]model.Usr) *[]SearchUsrsResData {
	data := []SearchUsrsResData{}
	for _, u := range *usrs {
		aid := uint(0)
		vid := uint(0)
		if u.ApxID != nil {
			aid = *u.ApxID
		}
		if u.VdrID != nil {
			vid = *u.VdrID
		}
		data = append(data, SearchUsrsResData{
			ID:           u.ID,
			ApxID:        aid,
			VdrID:        vid,
			Name:         u.Name,
			Email:        u.Email,
			BgnAt:        common.ParseDatetimeToStr(&u.BgnAt),
			EndAt:        common.ParseDatetimeToStr(&u.EndAt),
			Type:         u.Type,
			BasePoint:    u.BasePoint,
			BelongRate:   u.BelongRate,
			MaxWorks:     u.MaxWorks,
			FlushDays:    u.FlushDays,
			Rate:         u.Rate,
			FlushFeeRate: u.FlushFeeRate,
		})
	}
	return &data
}

type SearchUsrsRes struct {
	Data   []SearchUsrsResData `json:"data"`
	Errors []Err               `json:"errors"`
} // @name SearchUsrsRes

type GetUsrResData struct {
	ID           uint    `json:"id" swaggertype:"integer" format:"" example:"1"`
	ApxID        uint    `json:"apx_id" swaggertype:"integer" format:"" example:"1"`
	VdrID        uint    `json:"vdr_id" swaggertype:"integer" format:"" example:"1"`
	Name         string  `json:"name" swaggertype:"string" format:"" example:"User01"`
	Email        string  `json:"email" swaggertype:"string" format:"email" example:"sample@example.com"`
	BgnAt        string  `json:"bgn_at" swaggertype:"string" format:"date-time" example:"2023-01-01T00:00:00"`
	EndAt        string  `json:"end_at" swaggertype:"string" format:"date-time" example:"2023-01-01T00:00:00"`
	Type         uint8   `json:"type" swaggertype:"integer" format:"" example:"1"`
	BasePoint    uint    `json:"base_point" swaggertype:"integer" format:"" example:"100"`
	BelongRate   float64 `json:"belong_rate" swaggertype:"number" format:"" example:"0.1"`
	MaxWorks     uint    `json:"max_works" swaggertype:"integer" format:"" example:"5"`
	FlushDays    uint    `json:"flush_days" swaggertype:"integer" format:"" example:"30"`
	Rate         float64 `json:"rate" swaggertype:"number" format:"" example:"0.05"`
	FlushFeeRate float64 `json:"flush_fee_rate" swaggertype:"number" format:"" example:"0.05"`
} // @name GetUsrResData

func (d *GetUsrResData) Of(u *model.Usr) *GetUsrResData {
	aid := uint(0)
	vid := uint(0)
	if u.ApxID != nil {
		aid = *u.ApxID
	}
	if u.VdrID != nil {
		vid = *u.VdrID
	}
	data := GetUsrResData{
		ID:           u.ID,
		ApxID:        aid,
		VdrID:        vid,
		Name:         u.Name,
		Email:        u.Email,
		BgnAt:        common.ParseDatetimeToStr(&u.BgnAt),
		EndAt:        common.ParseDatetimeToStr(&u.EndAt),
		Type:         u.Type,
		BasePoint:    u.BasePoint,
		BelongRate:   u.BelongRate,
		MaxWorks:     u.MaxWorks,
		FlushDays:    u.FlushDays,
		Rate:         u.Rate,
		FlushFeeRate: u.FlushFeeRate,
	}
	return &data
}

type GetUsrRes struct {
	Data   GetUsrResData `json:"data"`
	Errors []Err         `json:"errors"`
} // @name GetUsrRes

type CreateUsrResData struct {
	ID uint `json:"id"`
} // @name CreateUsrResData

type CreateUsrRes struct {
	Data   CreateUsrResData `json:"data"`
	Errors []Err            `json:"errors"`
} // @name CreateUsrRes

type UpdateUsrResData struct {
} // @name UpdateUsrResData

type UpdateUsrRes struct {
	Data   UpdateUsrResData `json:"data"`
	Errors []Err            `json:"errors"`
} // @name UpdateUsrRes

type DeleteUsrResData struct {
} // @name DeleteUsrResData

type DeleteUsrRes struct {
	Data   DeleteUsrResData `json:"data"`
	Errors []Err            `json:"errors"`
} // @name DeleteUsrRes
```

src/mode/rt/rtbl/usrs_bl.go
```go
func SearchUsrs(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.SearchUsrsReq, res *rtres.SearchUsrsRes) bool {
	usrs := []model.Usr{}
	r := restsql.SearchUsrs(u.DB, &usrs, ju.IDs(false), "u1", req, &[]string{"name", "email"}, nil, ju.IsUsr(), true)
	if r.Error != nil {
		return InternalServerError(c, res)
	}
	data := rtres.SearchUsrsResData{}
	return OK(c, data.Of(&usrs), res)
}

func GetUsr(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.GetUsrReq, res *rtres.GetUsrRes) bool {
	usr := model.Usr{}
	r := restsql.GetUsr(u.DB, &usr, ju.IDs(false), "u1", req, nil, nil, ju.IsUsr(), true)
	if r.Error != nil || usr.ID == 0 {
		return NotFound(c, res)
	}
	data := rtres.GetUsrResData{}
	return OK(c, data.Of(&usr), res)
}

func CreateUsr(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.CreateUsrReq, res *rtres.CreateUsrRes) bool {
	if ju.IsFromKey() {
		return createApxAsKey(c, u, ju, req, res) // Create Apx
	} else if ju.IsApx() {
		return createVdrAsApx(c, u, ju, req, res) // Create Vdr
	} else if ju.IsVdr() {
		return createUsrAsVdr(c, u, ju, req, res) // Create Usr as Vdr
	}
	return Forbidden(c, res)
}

func createApxAsKey(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.CreateUsrReq, res *rtres.CreateUsrRes) bool {
	usr, err := buildAndCreateUsr(c, u, ju, nil, req, res, common.RandStr(13), "apxpw")
	if err != nil {
		return InternalServerError(c, res)
	}
	data := rtres.CreateUsrResData{ID: usr.ID}
	return OK(c, &data, res)
}

func createVdrAsApx(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.CreateUsrReq, res *rtres.CreateUsrRes) bool {
	var usr *model.Usr
	err := u.DB.Transaction(func(tx *gorm.DB) error {
		var err error
		usr, err = buildAndCreateUsr(c, u, ju, tx, req, res, common.RandStr(13), "vdrpw")
		if err != nil {
			return err
		}
		pool := model.Pool{
			ApxID:    *usr.ApxID,
			VdrID:    usr.ID,
			Remain:   0,
			TotalIn:  0,
			TotalOut: 0,
		}
		if err := tx.Create(&pool).Error; err != nil {
			return err
		}
		return nil
	})

	if err != nil {
		return InternalServerError(c, res)
	}
	data := rtres.CreateUsrResData{ID: usr.ID}
	return OK(c, &data, res)
}

func createUsrAsVdr(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.CreateUsrReq, res *rtres.CreateUsrRes) bool {
	lastUsr := &model.Usr{}
	err := u.DB.Transaction(func(tx *gorm.DB) error {
		secret := common.RandStr(20)
		usr, errr := buildAndCreateUsr(c, u, ju, tx, req, res, common.RandStr(13), secret)
		if errr != nil {
			return errr
		}
		lastUsr = usr
		return nil
	})
	if err != nil {
		u.Logger.Warn(err.Error())
		return InternalServerError(c, res)
	}
	data := rtres.CreateUsrResData{ID: lastUsr.ID}
	return OK(c, &data, res)
}

func buildAndCreateUsr(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, tx *gorm.DB, req *rtreq.CreateUsrReq, res *rtres.CreateUsrRes, username string, secret string) (*model.Usr, error) {
	var (
		aid *uint = nil
		vid *uint = nil
	)
	if !ju.IsFromKey() {
		if ju.IsApx() {
			aid = ju.UsrID
		} else if ju.IsVdr() {
			aid = ju.ApxID
			vid = ju.UsrID
		}
	}
	bgnAt, _ := common.ParseStrToDatetime(&req.BgnAt)
	endAt, _ := common.ParseStrToDatetime(&req.EndAt)

	if req.Type == acctype.INDI.Val() {
		formattedName, err := common.FormatJapaneseName(req.Name)
		if err != nil {
			return nil, fmt.Errorf("invalid name format: %w", err)
		}
		req.Name = formattedName
	}

	usr := model.Usr{
		Name:     req.Name,
		Email:    req.Email,
		Password: u.HashPassword(req.Password),
		BgnAt:    bgnAt,
		EndAt:    endAt,
		ApxID:    aid,
		VdrID:    vid,
		Type:     req.Type,
	}
	if ju.IsApx() { // VDRを登録しているなら
		usr.BasePoint = req.BasePoint
		usr.BelongRate = req.BelongRate
		usr.MaxWorks = req.MaxWorks
		usr.FlushFeeRate = req.FlushFeeRate
	}
	if req.Type == acctype.CORP.Val() { // CORPを登録しているなら
		usr.FlushDays = req.FlushDays
		usr.Rate = req.Rate
	}
	var r *gorm.DB
	if tx != nil {
		r = tx.Create(&usr)
	} else {
		r = u.DB.Create(&usr)
	}
	return &usr, r.Error
}

func UpdateUsr(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.UpdateUsrReq, res *rtres.UpdateUsrRes) bool {
	usr := model.Usr{}
	r := restsql.GetUsr(u.DB, &usr, ju.IDs(false), "u1", &rtreq.GetUsrReq{ID: req.ID}, nil, nil, ju.IsUsr(), false)
	if r.Error != nil || usr.ID == 0 {
		return NotFound(c, res)
	}
	if len(req.Password) > 0 {
		req.Password = u.HashPassword(req.Password)
	}

	if usr.Type == acctype.INDI.Val() && len(req.Name) > 0 {
		formattedName, err := common.FormatJapaneseName(req.Name)
		if err != nil {
			return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("invalid name format: %s", err.Error()))
		}
		req.Name = formattedName
	}

	err := common.UpdateSingleTable(u.DB, "usrs", &usr, req)
	if err != nil {
		return InternalServerError(c, res)
	}
	data := rtres.UpdateUsrResData{}
	return OK(c, &data, res)
}

func DeleteUsr(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.DeleteUsrReq, res *rtres.DeleteUsrRes) bool {
	usr := model.Usr{}
	r := restsql.GetUsr(u.DB, &usr, ju.IDs(false), "u1", &rtreq.GetUsrReq{ID: req.ID}, nil, nil, ju.IsUsr(), false)
	if r.Error != nil || usr.ID == 0 {
		return NotFound(c, res)
	}
	if ju.IsApx() {
		err := DeleteVdrByApx(c, u, ju, req, res, &usr)
		if err != nil {
			return InternalServerErrorCustomMsg(c, res, err.Error())
		}
	} else if ju.IsVdr() || ju.IsUsr() {
		err := DeleteUsrByVdrOrUsr(c, u, ju, req, res, &usr)
		if err != nil {
			return InternalServerErrorCustomMsg(c, res, err.Error())
		}
	}
	data := rtres.DeleteUsrResData{}
	return OK(c, &data, res)
}

func DeleteVdrByApx(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.DeleteUsrReq, res *rtres.DeleteUsrRes, vdr *model.Usr) error {
	usrs := []model.Usr{}
	r := u.DB.Select("`id`, `name`, `apx_id`, `vdr_id`").Where("`vdr_id` = ?", vdr.ID).Find(&usrs)
	if r.Error != nil {
		return fmt.Errorf("Failed to search usrs: %s", r.Error.Error())
	}
	err := u.DB.Transaction(func(tx *gorm.DB) error {
		if len(usrs) > 0 {
			for _, usr := range usrs { // VDR 配下の USR とその関連データを全て削除
				errr := DeleteOneUsrInTransaction(u, tx, &usr)
				if errr != nil {
					return errr
				}
			}
		}
		// 以下、VDR 配下の USR とその関連データを全て削除
		deleteSql := "DELETE FROM `%s` WHERE `%s` = ?"
		tables := []string{
			model.Job{}.TableName(),
			model.Match{}.TableName(),
			model.MatchStatus{}.TableName(),
			model.Work{}.TableName(),
			model.Belong{}.TableName(),
			model.Badge{}.TableName(),
			model.UsrBadge{}.TableName(),
			model.Point{}.TableName(),
			model.Payment{}.TableName(),
			model.Pool{}.TableName(),
			model.Flush{}.TableName(),
			model.Payout{}.TableName(),
		}
		for _, table := range tables {
			errr := tx.Exec(fmt.Sprintf(deleteSql, table, "vdr_id"), vdr.ID).Error
			if errr != nil {
				return fmt.Errorf("Failed to delete %s: %s", table, errr.Error())
			}
		}
		errr := common.DeleteSingleTablePhysic(tx, &vdr)
		if errr != nil {
			return fmt.Errorf("Failed to delete vdr: %s", errr.Error())
		}
		return nil
	})
	if err != nil {
		return fmt.Errorf("Failed to delete vdr by apx: %s", err.Error())
	}
	return nil
}

func DeleteUsrByVdrOrUsr(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.DeleteUsrReq, res *rtres.DeleteUsrRes, usr *model.Usr) error {
	err := u.DB.Transaction(func(tx *gorm.DB) error { // USR とその関連データを全て削除
		return DeleteOneUsrInTransaction(u, tx, usr)
	})
	if err != nil {
		return err
	}
	return nil
}

func DeleteOneUsrInTransaction(u *rtutil.RtUtil, tx *gorm.DB, usr *model.Usr) error {
	// deleteSql := "DELETE FROM `%s` WHERE `%s` = ?"
	// err := tx.Exec(fmt.Sprintf(deleteSql, model.Act{}.TableName(), "owner_id"), usr.ID).Error
	// if err != nil {
	// 	return err
	// }
	// err = common.DeleteSingleTablePhysic(tx, &usr)
	// if err != nil {
	// 	return err
	// }
	err := common.DeleteSingleTablePhysic(tx, &usr)
	if err != nil {
		return err
	}
	return nil
}
```