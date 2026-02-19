package cutil

import (
	"fmt"

	"github.com/t-kawata/mycute/mode/rt/rtutil"
	"github.com/t-kawata/mycute/model"
	"go.uber.org/zap"
	"gorm.io/gorm"
)

func IsUniqueByUsrForCreateUsr(l *zap.Logger, db *gorm.DB, ju *rtutil.JwtUsr, field string, value *string) bool {
	var (
		cnt int64
		r   *gorm.DB
	)
	if ju.IsFromKey() {
		r = db.Model(&model.Usr{}).Where(fmt.Sprintf("`usrs`.`apx_id` IS NULL AND `usrs`.`vdr_id` IS NULL AND `usrs`.`%s` = ?", field), value).Count(&cnt)
	} else if ju.IsApx() {
		r = db.Model(&model.Usr{}).Where(fmt.Sprintf("`usrs`.`apx_id` = ? AND `usrs`.`vdr_id` IS NULL AND `usrs`.`%s` = ?", field), ju.UsrID, value).Count(&cnt)
	} else if ju.IsVdr() {
		r = db.Model(&model.Usr{}).Where(fmt.Sprintf("`usrs`.`apx_id` = ? AND `usrs`.`vdr_id` = ? AND `usrs`.`%s` = ?", field), ju.ApxID, ju.UsrID, value).Count(&cnt)
	} else if ju.IsUsr() {
		r = db.Model(&model.Usr{}).Where(fmt.Sprintf("`usrs`.`apx_id` = ? AND `usrs`.`vdr_id` = ? AND `usrs`.`%s` = ?", field), ju.ApxID, ju.VdrID, value).Count(&cnt)
	}
	if r.Error != nil {
		return false
	}
	return cnt == 0
}

func IsUniqueByUsrForUpdateUsr(l *zap.Logger, db *gorm.DB, ju *rtutil.JwtUsr, field string, value *string, id *uint) bool {
	var (
		cnt int64
		r   *gorm.DB
	)
	if ju.IsFromKey() {
		r = db.Model(&model.Usr{}).Where(fmt.Sprintf("`usrs`.`apx_id` IS NULL AND `usrs`.`vdr_id` IS NULL AND `usrs`.`%s` = ? AND `usrs`.`id` <> ?", field), value, id).Count(&cnt)
	} else if ju.IsApx() {
		r = db.Model(&model.Usr{}).Where(fmt.Sprintf("`usrs`.`apx_id` = ? AND `usrs`.`vdr_id` IS NULL AND `usrs`.`%s` = ? AND `usrs`.`id` <> ?", field), ju.UsrID, value, id).Count(&cnt)
	} else if ju.IsVdr() {
		r = db.Model(&model.Usr{}).Where(fmt.Sprintf("`usrs`.`apx_id` = ? AND `usrs`.`vdr_id` = ? AND `usrs`.`%s` = ? AND `usrs`.`id` <> ?", field), ju.ApxID, ju.UsrID, value, id).Count(&cnt)
	} else if ju.IsUsr() {
		r = db.Model(&model.Usr{}).Where(fmt.Sprintf("`usrs`.`apx_id` = ? AND `usrs`.`vdr_id` = ? AND `usrs`.`%s` = ? AND `usrs`.`id` <> ?", field), ju.ApxID, ju.VdrID, value, id).Count(&cnt)
	}
	if r.Error != nil {
		return false
	}
	return cnt == 0
}

func IsExistUsr(l *zap.Logger, db *gorm.DB, ju *rtutil.JwtUsr, usrID *uint) bool {
	if *usrID == 0 {
		return true
	}
	var (
		cnt int64
		r   *gorm.DB
	)
	if ju.IsFromKey() {
		r = db.Model(&model.Usr{}).Where("`usrs`.`apx_id` IS NULL AND `usrs`.`vdr_id` IS NULL AND `usrs`.`id` = ?", usrID).Count(&cnt)
	} else if ju.IsApx() {
		r = db.Model(&model.Usr{}).Where("`usrs`.`apx_id` = ? AND `usrs`.`vdr_id` IS NULL AND `usrs`.`id` = ?", ju.UsrID, usrID).Count(&cnt)
	} else if ju.IsVdr() {
		r = db.Model(&model.Usr{}).Where("`usrs`.`apx_id` = ? AND `usrs`.`vdr_id` = ? AND `usrs`.`id` = ?", ju.ApxID, ju.UsrID, usrID).Count(&cnt)
	} else if ju.IsUsr() {
		r = db.Model(&model.Usr{}).Where("`usrs`.`apx_id` = ? AND `usrs`.`vdr_id` = ? AND `usrs`.`id` = ?", ju.ApxID, ju.VdrID, usrID).Count(&cnt)
	}
	if r.Error != nil {
		return false
	}
	return cnt > 0
}
