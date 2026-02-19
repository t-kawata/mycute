Go言語で実装されているユーザーの作成に関連するコードを以下にスニペットとして貼り付けます。

src/mode/rt/rthandler/hv1/usrs_handler.go
```go
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
```

src/mode/rt/rtparam/usrs_param.go
```go
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
```

src/mode/rt/rtreq/usrs_req.go
```go
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
```

src/mode/rt/rtres/usrs_res.go
```go
type CreateUsrResData struct {
	ID uint `json:"id"`
} // @name CreateUsrResData

type CreateUsrRes struct {
	Data   CreateUsrResData `json:"data"`
	Errors []Err            `json:"errors"`
} // @name CreateUsrRes
```

src/mode/rt/rtbl/usrs_bl.go
```go
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
```