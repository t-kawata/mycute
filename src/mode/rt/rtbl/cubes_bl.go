package rtbl

import (
	"fmt"
	"os"
	"path/filepath"

	"github.com/gin-gonic/gin"
	"github.com/t-kawata/mycute/lib/common"
	"github.com/t-kawata/mycute/mode/rt/rtreq"
	"github.com/t-kawata/mycute/mode/rt/rtres"
	"github.com/t-kawata/mycute/mode/rt/rtutil"
	"github.com/t-kawata/mycute/model"
	"github.com/t-kawata/mycute/pkg/cuber"
	"gorm.io/datatypes"
	"gorm.io/gorm"
)

func getCube(u *rtutil.RtUtil, id uint, apxID uint, vdrID uint) (*model.Cube, error) {
	var cube model.Cube
	if err := u.DB.Where("id = ? AND apx_id = ? AND vdr_id = ?", id, apxID, vdrID).First(&cube).Error; err != nil {
		return nil, err
	}
	return &cube, nil
}

// CreateCube は新しい Cube を作成します。
func CreateCube(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.CreateCubeReq, res *rtres.CreateCubeRes) bool {
	// 1. UUID 生成
	newUUID := *common.GenUUID()
	// 2. パス決定
	ids := ju.IDs(!(ju.IsApx() || ju.IsFromKey()))
	cubeDBFilePath, err := u.GetCubeDBFilePath(&newUUID, ids.ApxID, ids.VdrID, ids.UsrID)
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to get cube path: %s", err.Error()))
	}
	// 3. 親ディレクトリ作成
	if err := os.MkdirAll(filepath.Dir(cubeDBFilePath), 0755); err != nil {
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to create cube directory: %s", err.Error()))
	}
	// 4. KuzuDB 初期化
	if err := cuber.CreateCubeDB(cubeDBFilePath); err != nil {
		// Cleanup on failure
		os.RemoveAll(filepath.Dir(cubeDBFilePath))
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to initialize cube: %s", err.Error()))
	}
	// 5. 初期権限設定 (All Unlimited: 0 means no limit)
	initialPerm := model.CubePermission{
		ExportLimit: 0, RekeyLimit: 0, GenKeyLimit: 0,
		AbsorbLimit: 0, MemifyLimit: 0, SearchLimit: 0,
		AllowStats: true,
	}
	permJSON, err := common.ToJson(initialPerm)
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to convert initial permission to JSON: %s", err.Error()))
	}
	// 6. DBレコード作成
	newCube := model.Cube{
		UUID:        newUUID,
		UsrID:       *ids.UsrID,
		Name:        req.Name,
		Description: req.Description,
		ExpireAt:    nil, // 自分がゼロから作成するCubeは無期限
		Permissions: datatypes.JSON(permJSON),
		ApxID:       *ids.ApxID,
		VdrID:       *ids.VdrID,
	}
	if err := u.DB.Create(&newCube).Error; err != nil {
		// DB保存失敗時は作成した物理ファイルを削除してゴミを残さない
		os.RemoveAll(cubeDBFilePath)
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to save cube: %s", err.Error()))
	}
	data := rtres.CreateCubeResData{UUID: newUUID}
	return OK(c, &data, res)
}

// AbsorbCube はコンテンツをCubeに取り込みます。
func AbsorbCube(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.AbsorbCubeReq, res *rtres.AbsorbCubeRes) bool {
	// 1. Cubeの取得と権限チェック
	cube, err := getCube(u, req.CubeID, *ju.ApxID, *ju.VdrID)
	if err != nil {
		return NotFoundCustomMsg(c, res, "Cube not found")
	}
	// 権限JSONパース
	perm, err := common.ParseDatatypesJson[model.CubePermission](&cube.Permissions)
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, "Failed to parse permissions")
	}
	// 2. Limit チェック
	if perm.AbsorbLimit < 0 {
		return BadRequestCustomMsg(c, res, "Absorb limit exceeded")
	}
	nextLimit := perm.AbsorbLimit
	shouldUpdateLimit := false
	if perm.AbsorbLimit > 0 {
		nextLimit = perm.AbsorbLimit - 1
		if nextLimit == 0 {
			nextLimit = -1 // 0は無制限なので、使い切ったら-1(禁止)にする
		}
		shouldUpdateLimit = true
	}
	// 3. 一時ファイル作成
	tempDir := filepath.Join(*u.DBDirPath, "temp_absorb")
	if err := os.MkdirAll(tempDir, 0755); err != nil {
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to create temp dir: %s", err.Error()))
	}
	tempFile := filepath.Join(tempDir, fmt.Sprintf("%s.txt", *common.GenUUID()))
	if err := os.WriteFile(tempFile, []byte(req.Content), 0644); err != nil {
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to write temp file: %s", err.Error()))
	}
	defer os.Remove(tempFile) // 関数終了時に削除
	// 4. Cuber 呼び出し
	// CuberServiceの初期化は不要 (Singleton in RtUtil)
	if u.CuberService == nil {
		return InternalServerErrorCustomMsg(c, res, "CuberService is not available")
	}
	// Cubeパスの取得
	ids := ju.IDs(!(ju.IsApx() || ju.IsFromKey()))
	cubeDbFilePath, err := u.GetCubeDBFilePath(&cube.UUID, ids.ApxID, ids.VdrID, ids.UsrID)
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to get cube path: %s", err.Error()))
	}
	// Absorb実行
	usage, err := u.CuberService.Absorb(c, cubeDbFilePath, req.MemoryGroup, []string{tempFile})
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Absorb failed: %s", err.Error()))
	}
	// usageチェック
	if usage.InputTokens < 0 || usage.OutputTokens < 0 {
		return InternalServerErrorCustomMsg(c, res, "Invalid token usage reported")
	}
	// 5. DBトランザクション (Limit更新 & Stats更新)
	err = u.DB.Transaction(func(tx *gorm.DB) error {
		// Limit 更新
		if shouldUpdateLimit {
			perm.AbsorbLimit = nextLimit
			newJSONStr, err := common.ToJson(perm)
			if err != nil {
				return err
			}
			cube.Permissions = datatypes.JSON(newJSONStr)
			if err := tx.Save(cube).Error; err != nil {
				return err
			}
		}
		// Stats & Contributor 更新 (MemoryGroup を含む階層構造)
		// usage.Details は map[string]TokenUsage
		for modelName, detail := range usage.Details {
			// CubeModelStat (Training) - MemoryGroup を必ず含める
			var ms model.CubeModelStat
			if err := tx.Where("cube_id = ? AND memory_group = ? AND model_name = ? AND action_type = ? AND apx_id = ? AND vdr_id = ?",
				cube.ID, req.MemoryGroup, modelName, "training", *ids.ApxID, *ids.VdrID).
				FirstOrCreate(&ms, model.CubeModelStat{
					CubeID: cube.ID, MemoryGroup: req.MemoryGroup, ModelName: modelName, ActionType: "training",
					ApxID: *ids.ApxID, VdrID: *ids.VdrID,
				}).Error; err != nil {
				return err
			}
			ms.InputTokens += detail.InputTokens
			ms.OutputTokens += detail.OutputTokens
			if err := tx.Save(&ms).Error; err != nil {
				return err
			}
			contributorName, err := getJwtUsrName(u, ids.ApxID, ids.VdrID, ids.UsrID)
			if err != nil {
				return fmt.Errorf("Failed to get contributor name: %s", err.Error())
			}
			// CubeContributor (Training) - MemoryGroup を必ず含める
			var cc model.CubeContributor
			if err := tx.Where("cube_id = ? AND memory_group = ? AND contributor_name = ? AND model_name = ? AND apx_id = ? AND vdr_id = ?",
				cube.ID, req.MemoryGroup, contributorName, modelName, *ids.ApxID, *ids.VdrID).
				FirstOrCreate(&cc, model.CubeContributor{
					CubeID: cube.ID, MemoryGroup: req.MemoryGroup, ContributorName: contributorName, ModelName: modelName,
					ApxID: *ids.ApxID, VdrID: *ids.VdrID,
				}).Error; err != nil {
				return err
			}
			cc.InputTokens += detail.InputTokens
			cc.OutputTokens += detail.OutputTokens
			if err := tx.Save(&cc).Error; err != nil {
				return err
			}
		}
		return nil
	})
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("DB update failed: %s", err.Error()))
	}
	// 6. レスポンス作成
	data := rtres.AbsorbCubeResData{
		InputTokens:  usage.InputTokens,
		OutputTokens: usage.OutputTokens,
		AbsorbLimit:  perm.AbsorbLimit,
	}
	// To return new limit, need to update local perm or use nextLimit
	if shouldUpdateLimit {
		data.AbsorbLimit = nextLimit
	}
	return OK(c, &data, res)
}

// StatsCube はCubeの統計情報を取得します。
func StatsCube(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.StatsCubeReq, res *rtres.StatsCubeRes) bool {
	// 1. Cubeの取得と権限チェック
	cube, err := getCube(u, req.CubeID, *ju.ApxID, *ju.VdrID)
	if err != nil {
		return NotFoundCustomMsg(c, res, "Cube not found")
	}
	// 権限JSONパース
	perm, err := common.ParseDatatypesJson[model.CubePermission](&cube.Permissions)
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, "Failed to parse permissions")
	}
	// 2. AllowStats チェック
	if !perm.AllowStats {
		return ForbiddenCustomMsg(c, res, "Stats access is not allowed")
	}
	// 3. データ取得
	var modelStats []model.CubeModelStat
	var contribs []model.CubeContributor
	var lineage []model.CubeLineage

	// MemoryGroup フィルタ (オプション)
	statQuery := u.DB.Where("cube_id = ? AND apx_id = ? AND vdr_id = ?", cube.ID, *ju.ApxID, *ju.VdrID)
	contribQuery := u.DB.Where("cube_id = ? AND apx_id = ? AND vdr_id = ?", cube.ID, *ju.ApxID, *ju.VdrID)
	if req.MemoryGroup != nil && *req.MemoryGroup != "" {
		statQuery = statQuery.Where("memory_group = ?", *req.MemoryGroup)
		contribQuery = contribQuery.Where("memory_group = ?", *req.MemoryGroup)
	}
	statQuery.Find(&modelStats)
	contribQuery.Find(&contribs)
	u.DB.Where("cube_id = ? AND apx_id = ? AND vdr_id = ?", cube.ID, *ju.ApxID, *ju.VdrID).Order("generation asc").Find(&lineage)

	// 4. MemoryGroup でグループ化
	mgMap := make(map[string]*rtres.MemoryGroupStatsRes)

	for _, s := range modelStats {
		if _, ok := mgMap[s.MemoryGroup]; !ok {
			mgMap[s.MemoryGroup] = &rtres.MemoryGroupStatsRes{
				MemoryGroup:  s.MemoryGroup,
				Stats:        []rtres.ModelStatRes{},
				Contributors: []rtres.ContributorRes{},
			}
		}
		mgMap[s.MemoryGroup].Stats = append(mgMap[s.MemoryGroup].Stats, rtres.ModelStatRes{
			ModelName:    s.ModelName,
			ActionType:   s.ActionType,
			InputTokens:  s.InputTokens,
			OutputTokens: s.OutputTokens,
		})
	}

	for _, c := range contribs {
		if _, ok := mgMap[c.MemoryGroup]; !ok {
			mgMap[c.MemoryGroup] = &rtres.MemoryGroupStatsRes{
				MemoryGroup:  c.MemoryGroup,
				Stats:        []rtres.ModelStatRes{},
				Contributors: []rtres.ContributorRes{},
			}
		}
		mgMap[c.MemoryGroup].Contributors = append(mgMap[c.MemoryGroup].Contributors, rtres.ContributorRes{
			ContributorName: c.ContributorName,
			ModelName:       c.ModelName,
			InputTokens:     c.InputTokens,
			OutputTokens:    c.OutputTokens,
		})
	}

	// 5. Map to slice
	var mgList []rtres.MemoryGroupStatsRes
	for _, mg := range mgMap {
		mgList = append(mgList, *mg)
	}

	// 6. Lineage mapping
	var lineageRes []rtres.LineageRes
	for _, l := range lineage {
		lineageRes = append(lineageRes, rtres.LineageRes{
			UUID:          l.AncestorUUID,
			Owner:         l.AncestorOwner,
			ExportedAt:    l.ExportedAt,
			ExportedAtJST: common.UnixMilliToJSTStr(l.ExportedAt),
			Generation:    l.Generation,
		})
	}

	// 7. レスポンス作成
	data := rtres.StatsCubeResData{
		MemoryGroups: mgList,
		Lineage:      lineageRes,
	}
	return OK(c, &data, res)
}
