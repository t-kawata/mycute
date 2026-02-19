package restsql

import (
	"github.com/t-kawata/mycute/lib/common"
	"github.com/t-kawata/mycute/mode/rt/rtreq"
	"github.com/t-kawata/mycute/model"
	"gorm.io/gorm"
)

func SearchUsrsForKeyApxVdrSQL(condsStr *string, isRead *bool) *string {
	s := "" +
		"SELECT " +
		"    u1.`id`, " +
		"    u1.`apx_id`, " +
		"    u1.`vdr_id`, " +
		"    u1.`name`, " +
		"    u1.`email`, " +
		"    u1.`username`, " +
		"    u1.`bgn_at`, " +
		"    u1.`end_at`, " +
		"    u1.`type`, " +
		"    u1.`base_point`, " +
		"    u1.`belong_rate`, " +
		"    u1.`max_works`, " +
		"    u1.`flush_days`, " +
		"    u1.`rate`, " +
		"    u1.`flush_fee_rate`, " +
		"    0 AS dummy " +
		"FROM " +
		"    `usrs` AS u1 " +
		"WHERE " +
		*condsStr +
		common.TOpe(*isRead, "GROUP BY ", "") +
		common.TOpe(*isRead, "    u1.`id`, ", "") +
		common.TOpe(*isRead, "    u1.`apx_id`, ", "") +
		common.TOpe(*isRead, "    u1.`vdr_id`, ", "") +
		common.TOpe(*isRead, "    u1.`name`, ", "") +
		common.TOpe(*isRead, "    u1.`email`, ", "") +
		common.TOpe(*isRead, "    u1.`bgn_at`, ", "") +
		common.TOpe(*isRead, "    u1.`end_at` ", "") +
		"LIMIT #limit OFFSET #offset "
	return &s
}

func SearchUsrsForUsrSQL(condsStr *string, isRead *bool) *string {
	s := "" +
		"SELECT " +
		"    u1.`id`, " +
		"    u1.`apx_id`, " +
		"    u1.`vdr_id`, " +
		"    u1.`name`, " +
		"    u1.`email`, " +
		"    u1.`username`, " +
		"    u1.`bgn_at`, " +
		"    u1.`end_at`, " +
		"    u1.`type`, " +
		"    u1.`base_point`, " +
		"    u1.`belong_rate`, " +
		"    u1.`max_works`, " +
		"    u1.`flush_days`, " +
		"    u1.`rate`, " +
		"    u1.`flush_fee_rate`, " +
		"    0 AS dummy " +
		"FROM " +
		"    `usrs` AS u1 " +
		"WHERE " +
		*condsStr +
		"    AND u1.`id` = #usr_id " +
		common.TOpe(*isRead, "GROUP BY ", "") +
		common.TOpe(*isRead, "    u1.`id`, ", "") +
		common.TOpe(*isRead, "    u1.`apx_id`, ", "") +
		common.TOpe(*isRead, "    u1.`vdr_id`, ", "") +
		common.TOpe(*isRead, "    u1.`name`, ", "") +
		common.TOpe(*isRead, "    u1.`email`, ", "") +
		common.TOpe(*isRead, "    u1.`bgn_at`, ", "") +
		common.TOpe(*isRead, "    u1.`end_at` ", "") +
		"LIMIT #limit OFFSET #offset "
	return &s
}

type SearchUsrsVals struct {
	ApxID  *uint
	VdrID  *uint
	UsrID  *uint
	Name   *string
	Email  *string
	BgnAt  *string
	EndAt  *string
	Limit  *uint16
	Offset *uint16
}

func SearchUsrs(db *gorm.DB, dst *[]model.Usr, ids *common.IDs, tbl string, req *rtreq.SearchUsrsReq, likeTargets *[]string, ftTargets *[]string, isUsr bool, isRead bool) *gorm.DB {
	condsStr := common.GenSingleTableSearchCondsStr(ids, tbl, nil, req, likeTargets, ftTargets)
	sql := common.TOpe(isUsr, SearchUsrsForUsrSQL(condsStr, &isRead), SearchUsrsForKeyApxVdrSQL(condsStr, &isRead))
	return common.SearchBySql(db, dst, sql, &SearchUsrsVals{
		ApxID:  ids.ApxID,
		VdrID:  ids.VdrID,
		UsrID:  ids.UsrID,
		Name:   &req.Name,
		Email:  &req.Email,
		BgnAt:  &req.BgnAt,
		EndAt:  &req.EndAt,
		Limit:  &req.Limit,
		Offset: &req.Offset,
	})
}

type GetUsrVals struct {
	ID     *uint
	ApxID  *uint
	VdrID  *uint
	UsrID  *uint
	BgnAt  string
	EndAt  string
	Limit  uint16
	Offset uint16
}

func GetUsr(db *gorm.DB, dst *model.Usr, ids *common.IDs, tbl string, req *rtreq.GetUsrReq, likeTargets *[]string, ftTargets *[]string, isUsr bool, isRead bool) *gorm.DB {
	condsStr := common.GenSingleTableSearchCondsStr(ids, tbl, nil, req, likeTargets, ftTargets)
	now := common.GetNowStr()
	sql := common.TOpe(isUsr, SearchUsrsForUsrSQL(condsStr, &isRead), SearchUsrsForKeyApxVdrSQL(condsStr, &isRead))
	return common.SearchBySql(db, dst, sql, &GetUsrVals{
		ID:     &req.ID,
		ApxID:  ids.ApxID,
		VdrID:  ids.VdrID,
		UsrID:  ids.UsrID,
		BgnAt:  now,
		EndAt:  now,
		Limit:  1,
		Offset: 0,
	})
}

func SearchMyUsrForVdr(condsStr *string) *string {
	s := "" +
		"SELECT " +
		"    `usrs`.`id` " +
		"FROM " +
		"    `usrs` " +
		"WHERE " +
		*condsStr +
		"    AND `usrs`.`id` = #target_usr_id " +
		"    AND `usrs`.`vdr_id` = #usr_id " +
		"    AND `usrs`.`bgn_at` <= #bgn_at " +
		"    AND `usrs`.`end_at` >= #end_at " +
		"LIMIT #limit OFFSET #offset "
	return &s
}

func SearchMyUsrForUsr(condsStr *string, isIncludeWs bool, isIncludeRm bool) *string {
	s := "" +
		"SELECT " +
		"    `usrs`.`id` " +
		"FROM `usrs` " +
		"WHERE " +
		*condsStr +
		"    AND #target_usr_id = #usr_id " + // 自分
		"    AND `usrs`.`id` = #target_usr_id " +
		"    AND `usrs`.`bgn_at` <= #bgn_at " +
		"    AND `usrs`.`end_at` >= #end_at " +
		"LIMIT #limit OFFSET #offset "
	return &s
}

type GetMyUsrVals struct {
	ApxID       *uint
	VdrID       *uint
	UsrID       *uint
	TargetUsrID *uint
	BgnAt       string
	EndAt       string
	Limit       uint16
	Offset      uint16
}

func GetMyUsr(db *gorm.DB, dst *model.Usr, ids *common.IDs, usrID *uint, isUsr bool, isIncludeWs bool, isIncludeRm bool) *gorm.DB {
	condsStr := common.GenSingleTableSearchCondsStr(ids, "usrs", nil, &struct{}{}, nil, nil)
	now := common.GetNowStr()
	sql := common.TOpe(isUsr, SearchMyUsrForUsr(condsStr, isIncludeWs, isIncludeRm), SearchMyUsrForVdr(condsStr))
	return common.SearchBySql(db, dst, sql, &GetMyUsrVals{
		ApxID:       ids.ApxID,
		VdrID:       ids.VdrID,
		UsrID:       ids.UsrID,
		TargetUsrID: usrID,
		BgnAt:       now,
		EndAt:       now,
		Limit:       1,
		Offset:      0,
	})
}
