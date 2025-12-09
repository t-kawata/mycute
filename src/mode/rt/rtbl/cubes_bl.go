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
		// Stats & Contributor 更新
		// usage.Details は map[string]TokenUsage
		for modelName, detail := range usage.Details {
			// CubeModelStat (Training)
			var ms model.CubeModelStat
			if err := tx.Where("cube_id = ? AND model_name = ? AND action_type = ? AND apx_id = ? AND vdr_id = ?", cube.ID, modelName, "training", *ids.ApxID, *ids.VdrID).
				FirstOrCreate(&ms, model.CubeModelStat{
					CubeID: cube.ID, ModelName: modelName, ActionType: "training",
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
			// CubeContributor (Training)
			var cc model.CubeContributor
			if err := tx.Where("cube_id = ? AND contributor_name = ? AND model_name = ? AND apx_id = ? AND vdr_id = ?", cube.ID, contributorName, modelName, *ids.ApxID, *ids.VdrID).
				FirstOrCreate(&cc, model.CubeContributor{
					CubeID: cube.ID, ContributorName: contributorName, ModelName: modelName,
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

func getCube(u *rtutil.RtUtil, id uint, apxID uint, vdrID uint) (*model.Cube, error) {
	var cube model.Cube
	if err := u.DB.Where("id = ? AND apx_id = ? AND vdr_id = ?", id, apxID, vdrID).First(&cube).Error; err != nil {
		return nil, err
	}
	return &cube, nil
}
