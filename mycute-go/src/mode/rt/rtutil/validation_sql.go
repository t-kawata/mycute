package rtutil

import (
	"github.com/t-kawata/mycute/lib/common"
	"gorm.io/gorm"
)

type ValidType uint8

const (
	FOR_USE ValidType = iota + 1
	FOR_SHARE
	FOR_OUT
)

func (t *ValidType) IsForUse() bool {
	return *t == FOR_USE
}

func (t *ValidType) IsForShare() bool {
	return *t == FOR_SHARE
}

func (t *ValidType) IsForOut() bool {
	return *t == FOR_OUT
}

func CountValidUsrsForVdr() *string {
	s := "" +
		"SELECT " +
		"    COUNT(`usrs`.`id`) AS cnt " +
		"FROM `usrs` " +
		"WHERE " +
		"        `usrs`.`id` = #target_usr_id " +
		"    AND `usrs`.`bgn_at` <= NOW() + INTERVAL 9 HOUR " +
		"    AND `usrs`.`end_at` >= NOW() + INTERVAL 9 HOUR " +
		"    AND `usrs`.`apx_id` = #apx_id " +
		"    AND `usrs`.`vdr_id` = #vdr_id " +
		"    AND `usrs`.`deleted_at` IS NULL "
	return &s
}

func CountValidUsrsForUsr() *string {
	s := "" +
		"SELECT " +
		"    COUNT(`usrs`.`id`) AS cnt " +
		"FROM `usrs` " +
		"WHERE " +
		"        #target_usr_id = #usr_id " + // 自分
		"    AND `usrs`.`id` = #target_usr_id " +
		"    AND `usrs`.`bgn_at` <= NOW() + INTERVAL 9 HOUR " +
		"    AND `usrs`.`end_at` >= NOW() + INTERVAL 9 HOUR " +
		"    AND `usrs`.`apx_id` = #apx_id " +
		"    AND `usrs`.`vdr_id` = #vdr_id " +
		"    AND `usrs`.`deleted_at` IS NULL "
	return &s
}

type CountValidUsrsVals struct {
	ApxID       *uint
	VdrID       *uint
	UsrID       *uint
	TargetUsrID *uint
}

func CountValidUsrs(db *gorm.DB, dst *uint, ids *common.IDs, targetUsrID *uint, isUsr bool) *gorm.DB {
	sql := common.TOpe(isUsr, CountValidUsrsForUsr(), CountValidUsrsForVdr())
	return common.SearchBySql(db, dst, sql, &CountValidUsrsVals{
		ApxID:       ids.ApxID,
		VdrID:       ids.VdrID,
		UsrID:       ids.UsrID,
		TargetUsrID: targetUsrID,
	})
}

func CountValidLnsForVdr(t *ValidType) *string {
	s := "" +
		"SELECT " +
		"    COUNT(`lns`.`id`) AS cnt " +
		"FROM `lns` " +
		"WHERE " +
		"        `lns`.`id` = #ln_id " +
		"    AND `lns`.`bgn_at` <= NOW() + INTERVAL 9 HOUR " +
		"    AND `lns`.`end_at` >= NOW() + INTERVAL 9 HOUR " +
		"    AND `lns`.`apx_id` = #apx_id " +
		"    AND `lns`.`vdr_id` = #vdr_id " +
		"    AND `lns`.`deleted_at` IS NULL "
	return &s
}

func CountValidLnsForUsr(t *ValidType) *string {
	s := "" +
		"SELECT " +
		"    COUNT(`lns`.`id`) AS cnt " +
		"FROM `lns` " +
		"WHERE " +
		"        `lns`.`id` = #ln_id " +
		"    AND `lns`.`bgn_at` <= NOW() + INTERVAL 9 HOUR " +
		"    AND `lns`.`end_at` >= NOW() + INTERVAL 9 HOUR " +
		"    AND `lns`.`apx_id` = #apx_id " +
		"    AND `lns`.`vdr_id` = #vdr_id " +
		"    AND `lns`.`deleted_at` IS NULL " +
		"    AND `lns`.`owner_id` = #usr_id " // 自分が所有者
	return &s
}

type CountValidLnsVals struct {
	ApxID *uint
	VdrID *uint
	UsrID *uint
	LnID  *uint
}

func CountValidLns(db *gorm.DB, dst *uint, ids *common.IDs, lnID *uint, isUsr bool, t *ValidType) *gorm.DB {
	sql := common.TOpe(isUsr, CountValidLnsForUsr(t), CountValidLnsForVdr(t))
	return common.SearchBySql(db, dst, sql, &CountValidLnsVals{
		ApxID: ids.ApxID,
		VdrID: ids.VdrID,
		UsrID: ids.UsrID,
		LnID:  lnID,
	})
}

func CountValidRulesForVdr(t *ValidType) *string {
	s := "" +
		"SELECT " +
		"    COUNT(`rules`.`id`) AS cnt " +
		"FROM `rules` " +
		"WHERE " +
		"        `rules`.`id` = #rule_id " +
		"    AND `rules`.`bgn_at` <= NOW() + INTERVAL 9 HOUR " +
		"    AND `rules`.`end_at` >= NOW() + INTERVAL 9 HOUR " +
		"    AND `rules`.`apx_id` = #apx_id " +
		"    AND `rules`.`vdr_id` = #vdr_id " +
		"    AND `rules`.`deleted_at` IS NULL "
	return &s
}

func CountValidRulesForUsr(t *ValidType) *string {
	s := "" +
		"SELECT " +
		"    COUNT(`rules`.`id`) AS cnt " +
		"FROM `rules` " +
		"WHERE " +
		"        `rules`.`id` = #rule_id " +
		"    AND `rules`.`bgn_at` <= NOW() + INTERVAL 9 HOUR " +
		"    AND `rules`.`end_at` >= NOW() + INTERVAL 9 HOUR " +
		"    AND `rules`.`apx_id` = #apx_id " +
		"    AND `rules`.`vdr_id` = #vdr_id " +
		"    AND `rules`.`deleted_at` IS NULL " +
		"    AND `rules`.`owner_id` = #usr_id " // 自分が所有者
	return &s
}

type CountValidRulesVals struct {
	ApxID  *uint
	VdrID  *uint
	UsrID  *uint
	RuleID *uint
}

func CountValidRules(db *gorm.DB, dst *uint, ids *common.IDs, ruleID *uint, isUsr bool, t *ValidType) *gorm.DB {
	sql := common.TOpe(isUsr, CountValidRulesForUsr(t), CountValidRulesForVdr(t))
	return common.SearchBySql(db, dst, sql, &CountValidRulesVals{
		ApxID:  ids.ApxID,
		VdrID:  ids.VdrID,
		UsrID:  ids.UsrID,
		RuleID: ruleID,
	})
}

func CountValidNotifiesForVdr(t *ValidType) *string {
	s := "" +
		"SELECT " +
		"    COUNT(`notifies`.`id`) AS cnt " +
		"FROM `notifies` " +
		"WHERE " +
		"        `notifies`.`id` = #notify_id " +
		"    AND `notifies`.`bgn_at` <= NOW() + INTERVAL 9 HOUR " +
		"    AND `notifies`.`end_at` >= NOW() + INTERVAL 9 HOUR " +
		"    AND `notifies`.`apx_id` = #apx_id " +
		"    AND `notifies`.`vdr_id` = #vdr_id " +
		"    AND `notifies`.`deleted_at` IS NULL "
	return &s
}

func CountValidNotifiesForUsr(t *ValidType) *string {
	s := "" +
		"SELECT " +
		"    COUNT(`notifies`.`id`) AS cnt " +
		"FROM `notifies` " +
		"WHERE " +
		"        `notifies`.`id` = #notify_id " +
		"    AND `notifies`.`bgn_at` <= NOW() + INTERVAL 9 HOUR " +
		"    AND `notifies`.`end_at` >= NOW() + INTERVAL 9 HOUR " +
		"    AND `notifies`.`apx_id` = #apx_id " +
		"    AND `notifies`.`vdr_id` = #vdr_id " +
		"    AND `notifies`.`deleted_at` IS NULL " +
		"    AND `notifies`.`owner_id` = #usr_id " // 自分が所有者
	return &s
}

type CountValidNotifiesVals struct {
	ApxID    *uint
	VdrID    *uint
	UsrID    *uint
	NotifyID *uint
}

func CountValidNotifies(db *gorm.DB, dst *uint, ids *common.IDs, notifyID *uint, isUsr bool, t *ValidType) *gorm.DB {
	sql := common.TOpe(isUsr, CountValidNotifiesForUsr(t), CountValidNotifiesForVdr(t))
	return common.SearchBySql(db, dst, sql, &CountValidNotifiesVals{
		ApxID:    ids.ApxID,
		VdrID:    ids.VdrID,
		UsrID:    ids.UsrID,
		NotifyID: notifyID,
	})
}

func CountValidActsForVdr() *string {
	s := "" +
		"SELECT " +
		"    COUNT(`acts`.`id`) AS cnt " +
		"FROM `acts` " +
		"WHERE " +
		"        `acts`.`id` IN (#act_ids) " +
		"    AND `acts`.`bgn_at` <= NOW() + INTERVAL 9 HOUR " +
		"    AND `acts`.`end_at` >= NOW() + INTERVAL 9 HOUR " +
		"    AND `acts`.`apx_id` = #apx_id " +
		"    AND `acts`.`vdr_id` = #vdr_id " +
		"    AND `acts`.`deleted_at` IS NULL "
	return &s
}

func CountValidActsForUsr() *string {
	s := "" +
		"SELECT " +
		"    COUNT(`acts`.`id`) AS cnt " +
		"FROM `acts` " +
		"WHERE " +
		"        `acts`.`id` IN (#act_ids) " +
		"    AND `acts`.`bgn_at` <= NOW() + INTERVAL 9 HOUR " +
		"    AND `acts`.`end_at` >= NOW() + INTERVAL 9 HOUR " +
		"    AND `acts`.`apx_id` = #apx_id " +
		"    AND `acts`.`vdr_id` = #vdr_id " +
		"    AND `acts`.`deleted_at` IS NULL " +
		"    AND `acts`.`owner_id` = #usr_id " // 自分が所有者
	return &s
}

type CountValidActsVals struct {
	ApxID  *uint
	VdrID  *uint
	UsrID  *uint
	ActIDs *[]uint `sql:"act_ids"`
}

func CountValidActs(db *gorm.DB, dst *uint, ids *common.IDs, actIDs *[]*uint, isUsr bool) *gorm.DB {
	sql := common.TOpe(isUsr, CountValidActsForUsr(), CountValidActsForVdr())
	lastActIDs := []uint{}
	for _, id := range *actIDs {
		lastActIDs = append(lastActIDs, *id)
	}
	return common.SearchBySql(db, dst, sql, &CountValidActsVals{
		ActIDs: &lastActIDs,
		ApxID:  ids.ApxID,
		VdrID:  ids.VdrID,
		UsrID:  ids.UsrID,
	})
}

func CountValidEvcRulesForVdr(t *ValidType) *string {
	s := "" +
		"SELECT " +
		"    COUNT(`evc_rules`.`id`) AS cnt " +
		"FROM `evc_rules` " +
		"WHERE " +
		"        `evc_rules`.`id` = #evc_rule_id " +
		"    AND `evc_rules`.`bgn_at` <= NOW() + INTERVAL 9 HOUR " +
		"    AND `evc_rules`.`end_at` >= NOW() + INTERVAL 9 HOUR " +
		"    AND `evc_rules`.`apx_id` = #apx_id " +
		"    AND `evc_rules`.`vdr_id` = #vdr_id " +
		"    AND `evc_rules`.`deleted_at` IS NULL "
	return &s
}

func CountValidEvcRulesForUsr(t *ValidType) *string {
	s := "" +
		"SELECT " +
		"    COUNT(`evc_rules`.`id`) AS cnt " +
		"FROM `evc_rules` " +
		"WHERE " +
		"        `evc_rules`.`id` = #evc_rule_id " +
		"    AND `evc_rules`.`bgn_at` <= NOW() + INTERVAL 9 HOUR " +
		"    AND `evc_rules`.`end_at` >= NOW() + INTERVAL 9 HOUR " +
		"    AND `evc_rules`.`apx_id` = #apx_id " +
		"    AND `evc_rules`.`vdr_id` = #vdr_id " +
		"    AND `evc_rules`.`deleted_at` IS NULL "
	return &s
}

type CountValidEvcRulesVals struct {
	ApxID *uint
	VdrID *uint
	// UsrID  *uint
	EvcRuleID *uint
}

func CountValidEvcRules(db *gorm.DB, dst *uint, ids *common.IDs, evRuleID *uint, isUsr bool, t *ValidType) *gorm.DB {
	sql := common.TOpe(isUsr, CountValidEvcRulesForUsr(t), CountValidEvcRulesForVdr(t))
	return common.SearchBySql(db, dst, sql, &CountValidEvcRulesVals{
		ApxID: ids.ApxID,
		VdrID: ids.VdrID,
		// UsrID:  ids.UsrID,
		EvcRuleID: evRuleID,
	})
}
