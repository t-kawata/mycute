package restsql

import (
	"github.com/t-kawata/mycute/lib/common"
	"github.com/t-kawata/mycute/mode/rt/rtreq"
	"github.com/t-kawata/mycute/model"
	"gorm.io/gorm"
)

func SearchCubesSQL(condsStr *string, isRead *bool) *string {
	s := "" +
		"SELECT " +
		"    c1.`id`, " +
		"    c1.`uuid`, " +
		"    c1.`usr_id`, " +
		"    c1.`name`, " +
		"    c1.`description`, " +
		"    c1.`expire_at`, " +
		"    c1.`permissions`, " +
		"    c1.`source_export_id`, " +
		"    c1.`apx_id`, " +
		"    c1.`vdr_id`, " +
		"    c1.`created_at`, " +
		"    c1.`updated_at`, " +
		"    c1.`embedding_provider`, " +
		"    c1.`embedding_base_url`, " +
		"    c1.`embedding_model`, " +
		"    c1.`embedding_dimension`, " +
		"    0 AS dummy " +
		"FROM " +
		"    `cubes` AS c1 " +
		"WHERE " +
		*condsStr +
		"LIMIT #limit OFFSET #offset "
	return &s
}

type SearchCubesVals struct {
	ApxID       *uint
	VdrID       *uint
	UsrID       *uint
	Name        *string
	Description *string
	Limit       *uint16
	Offset      *uint16
}

func SearchCubes(db *gorm.DB, dst *[]model.Cube, ids *common.IDs, tbl string, req *rtreq.SearchCubesReq, likeTargets *[]string, ftTargets *[]string, isRead bool) *gorm.DB {
	condsStr := common.GenSingleTableSearchCondsStr(ids, tbl, nil, req, likeTargets, ftTargets)
	sql := SearchCubesSQL(condsStr, &isRead)
	return common.SearchBySql(db, dst, sql, &SearchCubesVals{
		ApxID:       ids.ApxID,
		VdrID:       ids.VdrID,
		UsrID:       ids.UsrID,
		Name:        &req.Name,
		Description: &req.Description,
		Limit:       &req.Limit,
		Offset:      &req.Offset,
	})
}
