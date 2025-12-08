package rtbl

import (
	"fmt"
	"net/http"

	"github.com/gin-gonic/gin"
	"github.com/t-kawata/mycute/enum/acctype"
	"github.com/t-kawata/mycute/lib/common"
	"github.com/t-kawata/mycute/mode/rt/rtreq"
	"github.com/t-kawata/mycute/mode/rt/rtres"
	"github.com/t-kawata/mycute/mode/rt/rtutil"
	"github.com/t-kawata/mycute/model"
	"github.com/t-kawata/mycute/sql/restsql"
	"gorm.io/gorm"
)

func AuthUsrByParam(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.AuthUsrReq, res *rtres.AuthUsrRes) {
	aid := common.StrToUint(req.ApxID)
	vid := common.StrToUint(req.VdrID)
	if IsApx(&aid, &vid) {
		AuthUsrAsApx(c, u, ju, req, res, &aid, &vid)
		return
	} else if IsVdr(&aid, &vid) {
		AuthUsrAsVdr(c, u, ju, req, res, &aid, &vid)
		return
	} else if IsUsr(&aid, &vid) {
		AuthUsrAsUsr(c, u, ju, req, res, &aid, &vid)
		return
	}
	Unauthorized(c, res)
}

func AuthUsrAsApx(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.AuthUsrReq, res *rtres.AuthUsrRes, aid *uint, vid *uint) {
	authUsrAs(c, u, ju, req, res, aid, vid, "`usrs`.`apx_id` IS NULL AND `usrs`.`vdr_id` IS NULL AND `usrs`.`email` = ?", req.Email)
}

func AuthUsrAsVdr(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.AuthUsrReq, res *rtres.AuthUsrRes, aid *uint, vid *uint) {
	authUsrAs(c, u, ju, req, res, aid, vid, "`usrs`.`apx_id` = ? AND `usrs`.`vdr_id` IS NULL AND `usrs`.`email` = ?", *aid, req.Email)
}

func AuthUsrAsUsr(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.AuthUsrReq, res *rtres.AuthUsrRes, aid *uint, vid *uint) {
	authUsrAs(c, u, ju, req, res, aid, vid, "`usrs`.`apx_id` = ? AND `usrs`.`vdr_id` = ? AND `usrs`.`email` = ?", *aid, *vid, req.Email)
}

func authUsrAs(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.AuthUsrReq, res *rtres.AuthUsrRes, aid *uint, vid *uint, conds string, vals ...any) {
	usr := model.Usr{}
	var condsSlice = append([]any{conds + " AND `usrs`.`bgn_at` <= ADDTIME(NOW(), '09:00:00') AND ADDTIME(NOW(), '09:00:00') <= `usrs`.`end_at`"}, vals...)
	u.DB.Select("id", "password", "is_staff", "type").First(&usr, condsSlice...)
	if len(usr.Password) == 0 || !u.IsEqualHashAndPassword(usr.Password, req.Password) {
		Unauthorized(c, res)
		return
	}
	token, err := generateToken(u, aid, vid, &usr.ID, &usr.IsStaff, &usr.Type, &req.Email, &req.Expire)
	if err != nil {
		Unauthorized(c, res)
		return
	}
	OK(c, &rtres.AuthUsrResData{Token: token}, res)
}

func AuthUsrByKey(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.AuthUsrReq, res *rtres.AuthUsrRes) {
	var (
		aid     uint  = 0
		vid     uint  = 0
		uid     uint  = 0
		email         = "key@key.com"
		isStaff       = false
		usrType uint8 = 0
	)
	token, err := generateToken(u, &aid, &vid, &uid, &isStaff, &usrType, &email, &req.Expire)
	if err != nil {
		Unauthorized(c, res)
		return
	}
	res.Data = rtres.AuthUsrResData{Token: token}
	c.JSON(http.StatusOK, res)
}

func generateToken(u *rtutil.RtUtil, aid *uint, vid *uint, uid *uint, isStaff *bool, usrType *uint8, email *string, expire *uint) (string, error) {
	staffID := uint(0)
	if *isStaff {
		staffID = *uid
	}
	jwtUsr := &rtutil.JwtUsr{
		ApxID:   aid,
		VdrID:   vid,
		UsrID:   uid,
		StaffID: &staffID,
		Email:   *email,
		Type:    *usrType,
	}
	return rtutil.GenerateToken(u.SKey, *expire, jwtUsr)
}

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
	if ju.IsApx() {
		// VDRを登録しているなら
	}
	if req.Type == acctype.CORP.Val() {
		// CORPを登録しているなら
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
		// deleteSql := "DELETE FROM `%s` WHERE `%s` = ?"
		// tables := []string{
		// 	model.Job{}.TableName(),
		// }
		// for _, table := range tables {
		// 	errr := tx.Exec(fmt.Sprintf(deleteSql, table, "vdr_id"), vdr.ID).Error
		// 	if errr != nil {
		// 		return fmt.Errorf("Failed to delete %s: %s", table, errr.Error())
		// 	}
		// }
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
