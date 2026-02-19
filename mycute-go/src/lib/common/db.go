package common

import (
	"errors"
	"fmt"
	"reflect"
	"strings"
	"time"

	"github.com/iancoleman/strcase"
	"github.com/t-kawata/mycute/config"
	"github.com/t-kawata/mycute/model"
	"gorm.io/datatypes"
	"gorm.io/driver/mysql"
	"gorm.io/gorm"
	"gorm.io/plugin/dbresolver"
)

type IDs struct {
	ApxID *uint
	VdrID *uint
	UsrID *uint
}

type Cond struct {
	Table           string
	Request         any
	LikeTargets     *[]string
	FullTextTargets *[]string
}

func GetDb(env *config.Env) (*gorm.DB, error) {
	db, err := gorm.Open(mysql.New(mysql.Config{
		DSN: getDbDns(env.NodesMDb.Username, env.NodesMDb.Password, env.NodesMDb.Host, env.NodesMDb.Port),
	}), &gorm.Config{})

	if err != nil {
		return db, err
	}

	var ds []gorm.Dialector
	for _, rdb := range env.NodesRDbs {
		ds = append(ds, mysql.Open(getDbDns(rdb.Username, rdb.Password, rdb.Host, rdb.Port)))
	}
	err = db.Use(dbresolver.Register(dbresolver.Config{Replicas: ds, TraceResolverMode: true}))
	return db, err
}

func GetDbWithConnInfo(dbname string, host string, port string, un string, pw string) (db *gorm.DB, err error) {
	dns := fmt.Sprintf("%s:%s@tcp(%s:%s)/%s?charset=utf8&parseTime=True&loc=Asia%%2FTokyo", un, pw, host, port, dbname)
	db, err = gorm.Open(mysql.New(mysql.Config{DSN: dns}), &gorm.Config{})
	if err != nil {
		err = fmt.Errorf("Failed to open a mysql database: %s", err.Error())
		return
	}
	return
}

func AutoMigrateNDb(db *gorm.DB) error {
	err := db.Transaction(func(tx *gorm.DB) error {
		tx.Exec("CREATE TABLE IF NOT EXISTS `usrs` (`id` bigint unsigned AUTO_INCREMENT,PRIMARY KEY (`id`))")
		return tx.AutoMigrate(
			&model.Key{},
			&model.Usr{},
			&model.ChatModel{},
			// Cube 関連モデル (Phase-11A)
			&model.Cube{},
			&model.CubeModelStat{},
			&model.CubeContributor{},
			&model.CubeLineage{},
			&model.Export{},
			&model.BurnedKey{},
		)
	})
	return err
}

func getDbDns(un string, pw string, h string, pt string) string {
	return fmt.Sprintf("%s:%s@tcp(%s:%s)/%s?charset=utf8&parseTime=True&loc=Asia%%2FTokyo", un, pw, h, pt, config.NODES_DB_NAME)
}

func SearchBySql[D any, V any](db *gorm.DB, dst *D, sql *string, vals *V) *gorm.DB {
	s, c := BuildSqlByStruct(sql, vals)
	ConvertAllWhiteToSingleSpace(s)
	return db.Raw(*s, *c...).Scan(dst)
}

func BuildSqlByStruct[T any](sql *string, vals *T) (*string, *[]any) {
	var conds []any
	paramValue := reflect.ValueOf(*vals)
	paramType := reflect.TypeOf(*vals)
	for i := 0; i < paramType.NumField(); i++ {
		field := paramType.Field(i)
		value := paramValue.Field(i).Interface()
		fn := field.Name
		fieldTag := field.Tag.Get("sql")
		if fieldTag == "" {
			fieldTag = TOpe(fn != "Ipv4" && fn != "Ipv6", strcase.ToSnake(fn), strings.ToLower(fn))
		} else if fieldTag == "-" {
			continue
		}
		rv := reflect.ValueOf(value)
		if rv.Kind() == reflect.Ptr {
			if !rv.IsNil() {
				rv = rv.Elem()
			}
		}
		if rv.Kind() == reflect.Array || rv.Kind() == reflect.Slice {
			rl := rv.Len()
			if rl > 0 {
				tmp := struct {
					F []string
					V map[string]any
				}{
					F: []string{},
					V: map[string]any{},
				}
				for j := range rl {
					ft := fmt.Sprintf("%s_[%d]", fieldTag, j)
					tmp.F = append(tmp.F, ft)
					tmp.V[ft] = rv.Index(j).Interface()
				}
				*sql = strings.Replace(*sql, "#"+fieldTag, "#"+strings.Join(tmp.F, ",#"), -1)
				for k, v := range tmp.V {
					replace(sql, &k, &conds, &v)
				}
			} else {
				var dummy any
				if isStrSliceReflectValue(&rv) {
					dummy = "bbf68dfe-c139-4e59-9688-ffa5a2ae4acc"
				} else {
					dummy = -10000
				}
				replace(sql, &fieldTag, &conds, &dummy)
			}
		} else {
			replace(sql, &fieldTag, &conds, &value)
		}
	}
	return sql, &conds
}

func isStrSliceReflectValue(v *reflect.Value) bool {
	if v.Kind() != reflect.Ptr {
		return false
	}
	elemType := v.Elem().Type()
	switch elemType.Kind() {
	case reflect.Slice:
		if elemType.Elem().Kind() == reflect.String {
			return true
		}
		if elemType.Elem().Kind() == reflect.Pointer && elemType.Elem().Elem().Kind() == reflect.String {
			return true
		}
		if elemType.Elem().Kind() == reflect.Pointer && elemType.Elem().Elem().Elem().Kind() == reflect.String {
			return true
		}
		if elemType.Elem().Kind() == reflect.Pointer && elemType.Elem().Elem().Elem().Elem().Kind() == reflect.String {
			return true
		}
	}
	return false
}

func replace(sql *string, fieldTag *string, conds *[]any, value *any) {
	placeholder := fmt.Sprintf("#%s", *fieldTag)
	for {
		placeholderIndex := strings.Index(*sql, placeholder)
		if placeholderIndex == -1 {
			break
		}
		*sql = strings.Replace(*sql, placeholder, "?", 1)
		valueIndex := strings.Count((*sql)[:placeholderIndex], "?")
		insertAt(conds, value, &valueIndex)
	}
}

func insertAt(slice *[]any, value *any, index *int) {
	*slice = append(*slice, nil)
	copy((*slice)[*index+1:], (*slice)[*index:])
	(*slice)[*index] = *value
}

func SearchSingleTable[D any](db *gorm.DB, ids *IDs, dst *D, base string, conds *[]Cond, limit *uint16, offset *uint16) *gorm.DB {
	var (
		sql         string
		condsValues []any
	)
	sql = base
	cnt := 0
	for _, c := range *conds {
		cs, cv, _, _ := GenSingleTableSearchCondsStrAndVals(ids, &c.Table, nil, c.Request, c.LikeTargets, c.FullTextTargets, false)
		if cnt == 0 {
			sql += " WHERE "
		} else {
			sql += " AND"
		}
		sql += fmt.Sprintf(" ((%s) AND `%s`.`deleted_at` IS NULL)", *cs, c.Table)
		condsValues = append(condsValues, *cv...)
		cnt++
	}
	if limit == nil {
		sql += " LIMIT 1"
	} else {
		sql += " LIMIT ?"
		condsValues = append(condsValues, *limit)
	}
	if offset != nil {
		sql += " OFFSET ?"
		condsValues = append(condsValues, *offset)
	}
	ConvertAllWhiteToSingleSpace(&sql)
	return db.Raw(sql, condsValues...).Scan(dst)
}

func GenSingleTableSearchConds[R any](db *gorm.DB, ids *IDs, tbl string, req *R, likeTargets *[]string, ftTargets *[]string) *gorm.DB {
	condsStr, condsValues, limit, offset := GenSingleTableSearchCondsStrAndVals(ids, &tbl, nil, req, likeTargets, ftTargets, false)
	return db.Where(*condsStr, *condsValues...).Limit(*limit).Offset(*offset)
}

func GenSingleTableSearchCondsStr[R any](ids *IDs, tbl string, subTbls map[string][]string, req *R, likeTargets *[]string, ftTargets *[]string) *string {
	cs, _, _, _ := GenSingleTableSearchCondsStrAndVals(ids, &tbl, &subTbls, req, likeTargets, ftTargets, true)
	if len(*cs) > 0 {
		*cs += " AND"
	}
	*cs += fmt.Sprintf(" `%s`.`deleted_at` IS NULL ", tbl)
	return cs
}

func GenSingleTableSearchCondsStrAndVals(ids *IDs, tbl *string, subTbls *map[string][]string, req any, likeTargets *[]string, ftTargets *[]string, isStrMode bool) (*string, *[]any, *int, *int) {
	var (
		condsStr    = ""
		condsValues = []any{}
		limit       int
		offset      int
	)
	v := reflect.ValueOf(req).Elem()
	for i := range v.NumField() {
		fld := v.Type().Field(i)
		fn := fld.Name
		f := TOpe(fn != "Ipv4" && fn != "Ipv6", strcase.ToSnake(fn), strings.ToLower(fn))
		val := v.Field(i).Interface()
		if (f == "usr_id" && val.(uint) == 0) ||
			IsEmpty(val) {
			continue
		}
		table := *tbl
		for k, slce := range *subTbls {
			if InArray(&f, &slce) {
				table = k
				break
			}
		}
		if len(condsStr) > 0 && f != "limit" && f != "offset" {
			condsStr += " AND "
		}
		if InArray(&f, ftTargets) {
			if !isStrMode {
				condsStr += fmt.Sprintf("MATCH(`%s`.`%s`) AGAINST(?)", table, f)
				condsValues = append(condsValues, val)
			} else {
				condsStr += fmt.Sprintf("MATCH(`%s`.`%s`) AGAINST(#%s)", table, f, f)
			}
		} else if InArray(&f, likeTargets) {
			if !isStrMode {
				condsStr += fmt.Sprintf("`%s`.`%s` LIKE ?", table, f)
				condsValues = append(condsValues, fmt.Sprintf("%%%s%%", val))
			} else {
				condsStr += fmt.Sprintf("`%s`.`%s` LIKE #%s", table, f, f)
				fieldValue := v.Field(i)
				fieldType := fieldValue.Type()
				newValue := reflect.ValueOf(fmt.Sprintf("%%%s%%", val))
				if newValue.Type().ConvertibleTo(fieldType) {
					fieldValue.Set(newValue.Convert(fieldType))
				}
			}
		} else {
			switch f {
			case "bgn_at", "open_at", "work_bgn_at":
				if !isStrMode {
					condsStr += fmt.Sprintf("`%s`.`%s` <= ?", table, f)
					condsValues = append(condsValues, val)
				} else {
					condsStr += fmt.Sprintf("`%s`.`%s` <= #%s", table, f, f)
				}
			case "end_at", "close_at", "work_end_at":
				if !isStrMode {
					condsStr += fmt.Sprintf("`%s`.`%s` >= ?", table, f)
					condsValues = append(condsValues, val)
				} else {
					condsStr += fmt.Sprintf("`%s`.`%s` >= #%s", table, f, f)
				}
			case "limit":
				if !isStrMode {
					limit = int(val.(uint16))
				}
			case "offset":
				if !isStrMode {
					offset = int(val.(uint16))
				}
			default:
				if !isStrMode {
					condsStr += fmt.Sprintf("`%s`.`%s` = ?", table, f)
					condsValues = append(condsValues, val)
				} else {
					if f == "master_id" && val.(int) == -1 {
						condsStr += fmt.Sprintf("`%s`.`%s` IS #%s", table, f, f)
					} else {
						condsStr += fmt.Sprintf("`%s`.`%s` = #%s", table, f, f)
					}
				}
			}
		}
	}
	cs, cv := SetIDs(ids, tbl, &condsStr, &condsValues, isStrMode)
	return cs, cv, &limit, &offset
}

func SetIDs(ids *IDs, tbl *string, condsStr *string, condsValues *[]any, isStrMode bool) (*string, *[]any) {
	if ids != nil {
		if ids.ApxID != nil {
			if len(*condsStr) > 0 {
				*condsStr += " AND "
			}
			if !isStrMode {
				*condsStr += fmt.Sprintf("`%s`.`%s` = ?", *tbl, "apx_id")
				*condsValues = append(*condsValues, *ids.ApxID)
			} else {
				*condsStr += fmt.Sprintf("`%s`.`%s` = #apx_id", *tbl, "apx_id")
			}
		}
		if ids.VdrID != nil {
			if len(*condsStr) > 0 {
				*condsStr += " AND "
			}
			if !isStrMode {
				*condsStr += fmt.Sprintf("`%s`.`%s` = ?", *tbl, "vdr_id")
				*condsValues = append(*condsValues, *ids.VdrID)
			} else {
				*condsStr += fmt.Sprintf("`%s`.`%s` = #vdr_id", *tbl, "vdr_id")
			}
		}
		// if ids.UsrID != nil {
		// 	if len(*condsStr) > 0 {
		// 		*condsStr += " AND "
		// 	}
		// 	clmName := TOpe(*tbl == "usrs", "master_id", "owner_id")
		// 	if !isStrMode {
		// 		*condsStr += fmt.Sprintf("`%s`.`%s` = ?", *tbl, clmName)
		// 		*condsValues = append(*condsValues, *ids.UsrID)
		// 	} else {
		// 		*condsStr += fmt.Sprintf("`%s`.`%s` = #%s", *tbl, clmName, clmName)
		// 	}
		// }
	}
	return condsStr, condsValues
}

func UpdateSingleTable[M any, R any](db *gorm.DB, tbl string, mdl *M, req *R) error {
	reqValues := reflect.ValueOf(req).Elem()
	reqMap := make(map[string]any)
	for i := range reqValues.NumField() {
		reqMap[reqValues.Type().Field(i).Name] = reqValues.Field(i).Interface()
	}
	mdlValues := reflect.ValueOf(mdl).Elem()
	mdlMap := make(map[string]any)
	for i := range mdlValues.NumField() {
		if mdlValues.Field(i).CanInterface() {
			mdlMap[mdlValues.Type().Field(i).Name] = mdlValues.Field(i).Interface()
		}
	}
	changedFields := make(map[string]any)
	for k, v := range reqMap {
		if k == "ID" ||
			(k == "Type" && v.(uint8) == 0) ||
			(k == "UsrID" && v.(uint) == 0) ||
			(k == "TargetApxID" && v.(uint) == 0) ||
			(k == "TargetVdrID" && v.(uint) == 0) ||
			(!InArray(&k, &[]string{ // 空でも空で更新するもの
				"Years",
				"Months",
				"Days",
				"DayOfWeeks",
			}) && IsEmpty(v)) {
			continue
		}
		if mdlMap[k] != v {
			f := TOpe(k != "Ipv4" && k != "Ipv6", strcase.ToSnake(k), strings.ToLower(k))
			if k == "MasterID" && v.(int) == 0 {
				continue
			} else if k == "MasterID" && v.(int) == -1 {
				changedFields[f] = nil
			} else {
				if InArray(&k, &[]string{"Urls", "Numbers", "Years", "Months", "Days", "DayOfWeeks"}) {
					if json, err := ToJson(v); err == nil {
						changedFields[f] = datatypes.JSON([]byte(json))
					} else {
						changedFields[f] = datatypes.JSON([]byte("[]"))
					}
				} else if k == "Json" {
					if json, ok := v.(string); ok {
						changedFields[f] = datatypes.JSON([]byte(json))
					} else {
						changedFields[f] = datatypes.JSON([]byte("{}"))
					}
				} else {
					changedFields[f] = v
				}
			}
		}
	}
	if len(changedFields) == 0 {
		return nil
	}
	changedFields["updated_at"] = GetNowStr()
	return db.Table(tbl).Where("id = ?", mdlValues.FieldByName("ID").Interface()).Updates(changedFields).Error
}

func DeleteSingleTable[M any](db *gorm.DB, mdl *M) error {
	_, ok := reflect.TypeOf(mdl).Elem().FieldByName("DeletedAt")
	if !ok {
		return errors.New("This model does not have a DeletedAt field.")
	}
	now := time.Now().In(time.Local)
	reflect.ValueOf(mdl).Elem().FieldByName("DeletedAt").Set(reflect.ValueOf(gorm.DeletedAt{Time: now}))
	err := db.Delete(mdl).Error
	if err != nil {
		return err
	}
	return nil
}

func DeleteSingleTablePhysic[M any](db *gorm.DB, mdl *M) error {
	err := db.Unscoped().Delete(mdl).Error
	if err != nil {
		return err
	}
	return nil
}
