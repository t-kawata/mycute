package rtbl

import (
	"bytes"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"strings"
	"time"

	"github.com/gin-gonic/gin"
	"github.com/t-kawata/mycute/lib/common"
	"github.com/t-kawata/mycute/mode/rt/rtreq"
	"github.com/t-kawata/mycute/mode/rt/rtres"
	"github.com/t-kawata/mycute/mode/rt/rtutil"
	"github.com/t-kawata/mycute/model"
	"github.com/t-kawata/mycute/pkg/cuber"
	"gorm.io/datatypes"
	"gorm.io/gorm"

	"archive/zip"
	"crypto"
	"crypto/aes"
	"crypto/cipher"
	"crypto/rand"
	"crypto/rsa"
	"crypto/sha256"
	"crypto/x509"
	"encoding/base64"
	"encoding/json"
	"encoding/pem"
)

const (
	METADATA_JSON           = "metadata.json"
	STATS_USAGE_JSON        = "stats_usage.json"
	STATS_CONTRIBUTORS_JSON = "stats_contributors.json"
	ENCRYPTED_DATA_BIN      = "encrypted_data.bin"
	SIGNATURE_BIN           = "signature.bin"
	PUBLIC_KEY_PEM          = "public_key.pem"
	ENCRYPTED_AES_KEY_BIN   = "encrypted_aes_key.bin"
	EXPORT_ID_TXT           = "export_id.txt"
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
		AllowStats:        true,
		MemifyConfigLimit: map[string]any{},
		SearchTypeLimit:   []string{},
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
	data := rtres.CreateCubeResData{ID: newCube.ID, UUID: newUUID}
	return OK(c, &data, res)
}

// AbsorbCube はコンテンツをCubeに取り込みます。
func AbsorbCube(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.AbsorbCubeReq, res *rtres.AbsorbCubeRes) bool {
	ids := ju.IDs(!(ju.IsApx() || ju.IsFromKey()))
	// 1. Cubeの取得と権限チェック
	cube, err := getCube(u, req.CubeID, *ids.ApxID, *ids.VdrID)
	if err != nil {
		return NotFoundCustomMsg(c, res, "Cube not found.")
	}
	// 権限JSONパース
	perm, err := common.ParseDatatypesJson[model.CubePermission](&cube.Permissions)
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, "Failed to parse permissions.")
	}
	// 2. Limit チェック
	if perm.AbsorbLimit < 0 {
		return BadRequestCustomMsg(c, res, "Absorb limit exceeded.")
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
	tempFile := filepath.Join(os.TempDir(), fmt.Sprintf("%s.txt", *common.GenUUID()))
	if err := os.WriteFile(tempFile, []byte(req.Content), 0644); err != nil {
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to write temp file: %s", err.Error()))
	}
	defer os.Remove(tempFile) // 関数終了時に削除
	// 4. Cuber 呼び出し
	// CuberServiceの初期化は不要 (Singleton in RtUtil)
	if u.CuberService == nil {
		return InternalServerErrorCustomMsg(c, res, "CuberService is not available.")
	}
	// Cubeパスの取得
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
		return InternalServerErrorCustomMsg(c, res, "Invalid token usage reported.")
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
	ids := ju.IDs(!(ju.IsApx() || ju.IsFromKey()))
	// 1. Cubeの取得と権限チェック
	cube, err := getCube(u, req.CubeID, *ids.ApxID, *ids.VdrID)
	if err != nil {
		return NotFoundCustomMsg(c, res, "Cube not found.")
	}
	// 権限JSONパース
	perm, err := common.ParseDatatypesJson[model.CubePermission](&cube.Permissions)
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, "Failed to parse permissions.")
	}
	// 2. AllowStats チェック
	if !perm.AllowStats {
		return ForbiddenCustomMsg(c, res, "Stats access is not allowed.")
	}
	// 3. データ取得
	var modelStats []model.CubeModelStat
	var contribs []model.CubeContributor
	var lineage []model.CubeLineage
	// MemoryGroup フィルタ (オプション)
	statQuery := u.DB.Where("cube_id = ? AND apx_id = ? AND vdr_id = ?", cube.ID, *ids.ApxID, *ids.VdrID)
	contribQuery := u.DB.Where("cube_id = ? AND apx_id = ? AND vdr_id = ?", cube.ID, *ids.ApxID, *ids.VdrID)
	if req.MemoryGroup != nil && *req.MemoryGroup != "" {
		statQuery = statQuery.Where("memory_group = ?", *req.MemoryGroup)
		contribQuery = contribQuery.Where("memory_group = ?", *req.MemoryGroup)
	}
	err = statQuery.Find(&modelStats).Error
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, "Failed to fetch stats usage.")
	}
	err = contribQuery.Find(&contribs).Error
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, "Failed to fetch stats contributors.")
	}
	err = u.DB.Where("cube_id = ? AND apx_id = ? AND vdr_id = ?", cube.ID, *ids.ApxID, *ids.VdrID).Order("generation asc").Find(&lineage).Error
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, "Failed to fetch lineage.")
	}
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

// ExportCube はCubeをエクスポートします。
func ExportCube(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.ExportCubeReq, res *rtres.ExportCubeRes) (*bytes.Buffer, string, bool) {
	ids := ju.IDs(!(ju.IsApx() || ju.IsFromKey()))
	// 1. Cubeの取得と権限チェック
	cube, err := getCube(u, req.CubeID, *ids.ApxID, *ids.VdrID)
	if err != nil {
		NotFoundCustomMsg(c, res, "Cube not found.")
		return nil, "", false
	}
	// 権限JSONパース
	perm, err := common.ParseDatatypesJson[model.CubePermission](&cube.Permissions)
	if err != nil {
		InternalServerErrorCustomMsg(c, res, "Failed to parse permissions.")
		return nil, "", false
	}
	// 2. Limit チェック（事前チェック、Tx内で再確認）
	if perm.ExportLimit < 0 {
		ForbiddenCustomMsg(c, res, "Export limit exceeded.")
		return nil, "", false
	}
	// 3. データ準備 (Lineage)
	var ancestors []model.CubeLineage
	if err := u.DB.Where("cube_id = ? AND apx_id = ? AND vdr_id = ?", cube.ID, *ids.ApxID, *ids.VdrID).Order("generation asc").Find(&ancestors).Error; err != nil {
		InternalServerErrorCustomMsg(c, res, "Failed to fetch lineage.")
		return nil, "", false
	}
	ownerName, err := getJwtUsrName(u, ids.ApxID, ids.VdrID, ids.UsrID)
	if err != nil {
		InternalServerErrorCustomMsg(c, res, "Failed to get owner name.")
		return nil, "", false
	}
	myLineage := model.CubeLineage{
		AncestorUUID:  cube.UUID, // ここは Create / Import の時に作ったUUIDで良い
		AncestorOwner: ownerName,
		ExportedAt:    *common.GetNowUnixMilli(),
		Generation:    len(ancestors) + 1,
	}
	exportLineageList := append(ancestors, myLineage)
	lineageJSON, err := common.ToJson(exportLineageList)
	if err != nil {
		InternalServerErrorCustomMsg(c, res, "Failed to serialize lineage.")
		return nil, "", false
	}
	// 4. Statsデータ取得
	var modelStats []model.CubeModelStat
	if err := u.DB.Where("cube_id = ? AND apx_id = ? AND vdr_id = ?", cube.ID, *ids.ApxID, *ids.VdrID).Find(&modelStats).Error; err != nil {
		InternalServerErrorCustomMsg(c, res, "Failed to fetch stats usage.")
		return nil, "", false
	}
	statsUsageJSON, err := common.ToJson(modelStats)
	if err != nil {
		InternalServerErrorCustomMsg(c, res, "Failed to serialize stats usage.")
		return nil, "", false
	}
	var contributors []model.CubeContributor
	if err := u.DB.Where("cube_id = ? AND apx_id = ? AND vdr_id = ?", cube.ID, *ids.ApxID, *ids.VdrID).Find(&contributors).Error; err != nil {
		InternalServerErrorCustomMsg(c, res, "Failed to fetch stats contributors.")
		return nil, "", false
	}
	statsContribJSON, err := common.ToJson(contributors)
	if err != nil {
		InternalServerErrorCustomMsg(c, res, "Failed to serialize stats contributors.")
		return nil, "", false
	}
	// 5. Zip作成
	cubeDbFilePath, err := u.GetCubeDBFilePath(&cube.UUID, ids.ApxID, ids.VdrID, ids.UsrID)
	if err != nil {
		InternalServerErrorCustomMsg(c, res, "Failed to get cube path")
		return nil, "", false
	}
	extraFiles := map[string][]byte{
		METADATA_JSON:           []byte(lineageJSON),
		STATS_USAGE_JSON:        []byte(statsUsageJSON),
		STATS_CONTRIBUTORS_JSON: []byte(statsContribJSON),
	}
	zipBuffer, err := cuber.ExportCubeToZip(cubeDbFilePath, extraFiles)
	if err != nil {
		InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Export failed: %s", err.Error()))
		return nil, "", false
	}
	// 6. セキュリティとパッケージング処理（Tx前に完了）
	privateKey, err := rsa.GenerateKey(rand.Reader, 2048)
	if err != nil {
		InternalServerErrorCustomMsg(c, res, "Failed to generate RSA key.")
		return nil, "", false
	}
	publicKey := &privateKey.PublicKey
	// 保存/配布のためにキーをエンコード
	privBytes := x509.MarshalPKCS1PrivateKey(privateKey)
	privPEM := pem.EncodeToMemory(&pem.Block{Type: "RSA PRIVATE KEY", Bytes: privBytes})
	pubBytes := x509.MarshalPKCS1PublicKey(publicKey)
	pubPEM := pem.EncodeToMemory(&pem.Block{Type: "RSA PUBLIC KEY", Bytes: pubBytes})
	// 3. AES Key 生成
	aesKey := make([]byte, 32)
	if _, err := rand.Read(aesKey); err != nil {
		InternalServerErrorCustomMsg(c, res, "Failed to generate AES key.")
		return nil, "", false
	}
	// 4. zipBuffer を AES-GCM で暗号化
	block, err := aes.NewCipher(aesKey)
	if err != nil {
		InternalServerErrorCustomMsg(c, res, "Failed to create AES cipher.")
		return nil, "", false
	}
	gcm, err := cipher.NewGCM(block)
	if err != nil {
		InternalServerErrorCustomMsg(c, res, "Failed to create GCM.")
		return nil, "", false
	}
	nonce := make([]byte, gcm.NonceSize())
	if _, err := rand.Read(nonce); err != nil {
		InternalServerErrorCustomMsg(c, res, "Failed to generate nonce.")
		return nil, "", false
	}
	// Seal(dst, nonce, plaintext, additionalData)
	// 復号化のために、ノンスを暗号文の先頭に追加
	encryptedData := gcm.Seal(nonce, nonce, zipBuffer.Bytes(), nil)
	// 5. AES Key を RSA Public Key で暗号化
	encryptedAESKey, err := rsa.EncryptOAEP(sha256.New(), rand.Reader, publicKey, aesKey, nil)
	if err != nil {
		InternalServerErrorCustomMsg(c, res, "Failed to encrypt AES key.")
		return nil, "", false
	}
	// 6. 暗号化されたデータの署名を作成
	hash := sha256.Sum256(encryptedData)
	signature, err := rsa.SignPSS(rand.Reader, privateKey, crypto.SHA256, hash[:], nil)
	if err != nil {
		InternalServerErrorCustomMsg(c, res, "Failed to sign data.")
		return nil, "", false
	}
	// 7. Export Record 作成
	dataHash := common.CalculateSHA256(encryptedData)
	newUUID := *common.GenUUID()
	// 8. Transaction: Limit更新 + Export作成
	var record model.Export
	txErr := u.DB.Transaction(func(tx *gorm.DB) error {
		// Cubeを再取得して最新のLimit確認
		var txCube model.Cube
		if err := tx.Where("id = ?", cube.ID).First(&txCube).Error; err != nil {
			return err
		}
		txPerm, err := common.ParseDatatypesJson[model.CubePermission](&txCube.Permissions)
		if err != nil {
			return err
		}
		// Limit再確認
		if txPerm.ExportLimit < 0 {
			return fmt.Errorf("export limit exceeded")
		}
		// Limit消費
		if txPerm.ExportLimit > 0 {
			nextLimit := txPerm.ExportLimit - 1
			if nextLimit == 0 {
				nextLimit = -1
			}
			txPerm.ExportLimit = nextLimit
			newJSONStr, err := common.ToJson(txPerm)
			if err != nil {
				return err
			}
			txCube.Permissions = datatypes.JSON(newJSONStr)
			if err := tx.Save(&txCube).Error; err != nil {
				return err
			}
		}
		// Export作成
		record = model.Export{
			CubeID:     txCube.ID,
			NewUUID:    newUUID,
			Hash:       dataHash,
			PrivateKey: string(privPEM),
			ApxID:      *ids.ApxID,
			VdrID:      *ids.VdrID,
		}
		if err := tx.Create(&record).Error; err != nil {
			return err
		}
		return nil
	})
	if txErr != nil {
		if txErr.Error() == "export limit exceeded" {
			ForbiddenCustomMsg(c, res, "Export limit exceeded.")
		} else {
			InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Transaction failed: %s", txErr.Error()))
		}
		return nil, "", false
	}
	// 9. Create Final Zip
	finalZip := new(bytes.Buffer)
	zwFinal := zip.NewWriter(finalZip)
	addToZip := func(w *zip.Writer, name string, content []byte) error {
		f, err := w.Create(name)
		if err != nil {
			return err
		}
		_, err = f.Write(content)
		return err
	}
	if err := addToZip(zwFinal, ENCRYPTED_DATA_BIN, encryptedData); err != nil {
		InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Zip error (%s): %s", ENCRYPTED_DATA_BIN, err.Error()))
		return nil, "", false
	}
	if err := addToZip(zwFinal, SIGNATURE_BIN, signature); err != nil {
		InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Zip error (%s): %s", SIGNATURE_BIN, err.Error()))
		return nil, "", false
	}
	if err := addToZip(zwFinal, PUBLIC_KEY_PEM, pubPEM); err != nil {
		InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Zip error (%s): %s", PUBLIC_KEY_PEM, err.Error()))
		return nil, "", false
	}
	if err := addToZip(zwFinal, ENCRYPTED_AES_KEY_BIN, encryptedAESKey); err != nil {
		InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Zip error (%s): %s", ENCRYPTED_AES_KEY_BIN, err.Error()))
		return nil, "", false
	}
	if err := addToZip(zwFinal, EXPORT_ID_TXT, []byte(fmt.Sprintf("%d", record.ID))); err != nil {
		InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Zip error (%s): %s", EXPORT_ID_TXT, err.Error()))
		return nil, "", false
	}
	if err := zwFinal.Close(); err != nil {
		InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Zip close error: %s", err.Error()))
		return nil, "", false
	}
	fileName := fmt.Sprintf("cube_%d_%s.cube", cube.ID, newUUID)
	return finalZip, fileName, true
}

// CheckInheritance は親子間の権限継承ルールを検証します。
func CheckInheritance(parent model.CubePermission, child model.CubePermission, pExpire, cExpire *time.Time) error {
	// 1. 禁止であるはずの機能や制限が子の時点で復活してしまっていないかチェック
	//     - 親が禁止(-1)なら、子も禁止(-1)でなければならない
	//     - 親が false なら、子も false でなければならない
	if parent.ExportLimit < 0 && child.ExportLimit >= 0 { // 親が禁止(-1)なら、子も禁止(-1)でなければならない
		return fmt.Errorf("Cannot enable export (parent forbidden).")
	} else if parent.ExportLimit > 0 && child.ExportLimit > parent.ExportLimit { // 親が正数(回数制限)なら、子はその制限以下でなければならない
		return fmt.Errorf("Cannot enable export (parent limit exceeded).")
	}
	if parent.RekeyLimit < 0 && child.RekeyLimit >= 0 { // 親が禁止(-1)なら、子も禁止(-1)でなければならない
		return fmt.Errorf("Cannot enable rekey (parent forbidden).")
	} else if parent.RekeyLimit > 0 && child.RekeyLimit > parent.RekeyLimit { // 親が正数(回数制限)なら、子はその制限以下でなければならない
		return fmt.Errorf("Cannot enable rekey (parent limit exceeded).")
	}
	if parent.GenKeyLimit < 0 && child.GenKeyLimit >= 0 { // 親が禁止(-1)なら、子も禁止(-1)でなければならない
		return fmt.Errorf("Cannot enable genkey (parent forbidden).")
	} else if parent.GenKeyLimit > 0 && child.GenKeyLimit > parent.GenKeyLimit { // 親が正数(回数制限)なら、子はその制限以下でなければならない
		return fmt.Errorf("Cannot enable genkey (parent limit exceeded).")
	}
	if parent.AbsorbLimit < 0 && child.AbsorbLimit >= 0 { // 親が禁止(-1)なら、子も禁止(-1)でなければならない
		return fmt.Errorf("Cannot enable absorb (parent forbidden).")
	} else if parent.AbsorbLimit > 0 && child.AbsorbLimit > parent.AbsorbLimit { // 親が正数(回数制限)なら、子はその制限以下でなければならない
		return fmt.Errorf("Cannot enable absorb (parent limit exceeded).")
	}
	if parent.MemifyLimit < 0 && child.MemifyLimit >= 0 { // 親が禁止(-1)なら、子も禁止(-1)でなければならない
		return fmt.Errorf("Cannot enable memify (parent forbidden).")
	} else if parent.MemifyLimit > 0 && child.MemifyLimit > parent.MemifyLimit { // 親が正数(回数制限)なら、子はその制限以下でなければならない
		return fmt.Errorf("Cannot enable memify (parent limit exceeded).")
	}
	if parent.SearchLimit < 0 && child.SearchLimit >= 0 { // 親が禁止(-1)なら、子も禁止(-1)でなければならない
		return fmt.Errorf("Cannot enable search (parent forbidden).")
	} else if parent.SearchLimit > 0 && child.SearchLimit > parent.SearchLimit { // 親が正数(回数制限)なら、子はその制限以下でなければならない
		return fmt.Errorf("Cannot enable search (parent limit exceeded).")
	}
	if !parent.AllowStats && child.AllowStats { // 親が禁止なら、子も禁止でなければならない
		return fmt.Errorf("Cannot enable stats (parent forbidden).")
	}
	// 2. Expire チェック
	// 親に期限がある場合、子はそれより前でなければならない
	if pExpire != nil {
		if cExpire == nil {
			return fmt.Errorf("Cannot remove expiration (parent has expire).")
		}
		if cExpire.After(*pExpire) {
			return fmt.Errorf("Cannot extend expiration beyond parent.")
		}
	}
	return nil
}

// GenKeyCube は新しい鍵を発行します。
// GenKeyCubeシーケンス: ファイルアップロードを受け取り、署名検証後に新しい鍵を発行
func GenKeyCube(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.GenKeyCubeReq, res *rtres.GenKeyCubeRes) bool {
	// 1. Multipart Form Parsing (File)
	file, err := c.FormFile("file")
	if err != nil {
		return BadRequestCustomMsg(c, res, fmt.Sprintf("File 'file' is required: %s", err.Error()))
	}
	f, err := file.Open()
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to open uploaded file: %s", err.Error()))
	}
	defer f.Close()
	// Read Zip
	buf := new(bytes.Buffer)
	if _, err := buf.ReadFrom(f); err != nil {
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to read file: %s", err.Error()))
	}
	zipReader, err := zip.NewReader(bytes.NewReader(buf.Bytes()), int64(buf.Len()))
	if err != nil {
		return BadRequestCustomMsg(c, res, fmt.Sprintf("Invalid zip file: %s", err.Error()))
	}
	// 2. Extract Components from Zip
	readZipFile := func(name string) ([]byte, error) {
		for _, zf := range zipReader.File {
			if zf.Name == name {
				rc, err := zf.Open()
				if err != nil {
					return nil, err
				}
				defer rc.Close()
				return io.ReadAll(rc)
			}
		}
		return nil, fmt.Errorf("File not found: %s", name)
	}
	encAESKey, err := readZipFile(ENCRYPTED_AES_KEY_BIN)
	if err != nil {
		return BadRequestCustomMsg(c, res, fmt.Sprintf("Missing '%s': %s", ENCRYPTED_AES_KEY_BIN, err.Error()))
	}
	signature, err := readZipFile(SIGNATURE_BIN)
	if err != nil {
		return BadRequestCustomMsg(c, res, fmt.Sprintf("Missing '%s': %s", SIGNATURE_BIN, err.Error()))
	}
	pubKeyBytes, err := readZipFile(PUBLIC_KEY_PEM)
	if err != nil {
		return BadRequestCustomMsg(c, res, fmt.Sprintf("Missing '%s': %s", PUBLIC_KEY_PEM, err.Error()))
	}
	exportIDBytes, err := readZipFile(EXPORT_ID_TXT)
	if err != nil {
		return BadRequestCustomMsg(c, res, fmt.Sprintf("Missing '%s': %s", EXPORT_ID_TXT, err.Error()))
	}
	exportIDStr := string(exportIDBytes)
	encData, err := readZipFile(ENCRYPTED_DATA_BIN)
	if err != nil {
		return BadRequestCustomMsg(c, res, fmt.Sprintf("Missing '%s': %s", ENCRYPTED_DATA_BIN, err.Error()))
	}
	// 3. DB Lookup (Export Record)
	ids := ju.IDs(!(ju.IsApx() || ju.IsFromKey()))
	var exp model.Export
	if err := u.DB.Where("id = ? AND apx_id = ? AND vdr_id = ?", exportIDStr, ids.ApxID, ids.VdrID).First(&exp).Error; err != nil {
		return NotFoundCustomMsg(c, res, fmt.Sprintf("Export record not found or access denied: %s", err.Error()))
	}
	// Check Owner via Source Cube
	var sourceCube model.Cube
	if err := u.DB.First(&sourceCube, exp.CubeID).Error; err != nil {
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Source cube not found: %s", err.Error()))
	}
	if sourceCube.UsrID != *ids.UsrID { // 認証JWTユーザー自身が所有者である Cube にしか GenKey することはできない
		return ForbiddenCustomMsg(c, res, "Not the owner of the source cube.")
	}
	// Parse Private Key from DB
	blockPriv, _ := pem.Decode([]byte(exp.PrivateKey))
	if blockPriv == nil {
		return InternalServerErrorCustomMsg(c, res, "Failed to parse private key from DB.")
	}
	privateKey, err := x509.ParsePKCS1PrivateKey(blockPriv.Bytes)
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to parse private key: %s", err.Error()))
	}
	// 4. Verify Signature (Integrity Check)
	blockPub, _ := pem.Decode(pubKeyBytes)
	if blockPub == nil {
		return BadRequestCustomMsg(c, res, "Failed to parse public key from zip.")
	}
	publicKey, err := x509.ParsePKCS1PublicKey(blockPub.Bytes)
	if err != nil {
		return BadRequestCustomMsg(c, res, fmt.Sprintf("Failed to parse public key: %s", err.Error()))
	}

	hash := sha256.Sum256(encData)
	if err := rsa.VerifyPSS(publicKey, crypto.SHA256, hash[:], signature, nil); err != nil {
		return BadRequestCustomMsg(c, res, fmt.Sprintf("File signature verification failed (tampered?): %s", err.Error()))
	}
	// 5. Decrypt AES Key
	aesKey, err := rsa.DecryptOAEP(sha256.New(), rand.Reader, privateKey, encAESKey, nil)
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to decrypt AES key: %s", err.Error()))
	}
	// 6. Inheritance Check
	parentPerm, err := common.ParseDatatypesJson[model.CubePermission](&sourceCube.Permissions)
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to parse source permissions: %s", err.Error()))
	}
	var reqExpire *time.Time
	if req.ExpireAt != nil {
		t, err := common.ParseStrToDatetime(req.ExpireAt)
		if err != nil {
			return BadRequestCustomMsg(c, res, fmt.Sprintf("Invalid expire_at format: %s", err.Error()))
		}
		reqExpire = &t
	}
	if err := CheckInheritance(parentPerm, req.Permissions, sourceCube.ExpireAt, reqExpire); err != nil {
		return BadRequestCustomMsg(c, res, fmt.Sprintf("Cube permissions inheritance error: %s", err.Error()))
	}
	// 7. Limit Check（事前チェック）
	if parentPerm.GenKeyLimit < 0 {
		return ForbiddenCustomMsg(c, res, "GenKey limit exceeded.")
	}
	// 8. Construct Key Payload（Tx前に準備）
	type KeyPayload struct {
		AESKey      string               `json:"aes_key"`
		Permissions model.CubePermission `json:"permissions"`
		ExpireAt    *time.Time           `json:"expire_at"`
		ExportID    uint                 `json:"export_id"`
	}
	payload := KeyPayload{
		AESKey:      base64.StdEncoding.EncodeToString(aesKey),
		Permissions: req.Permissions,
		ExpireAt:    reqExpire,
		ExportID:    exp.ID,
	}
	payloadBytes, err := json.Marshal(payload)
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to marshal key payload: %s", err.Error()))
	}
	// Sign Payload
	hashPayload := sha256.Sum256(payloadBytes)
	sigPayload, err := rsa.SignPSS(rand.Reader, privateKey, crypto.SHA256, hashPayload[:], nil)
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to sign key payload: %s", err.Error()))
	}
	// Format Final Key
	keyStr := base64.StdEncoding.EncodeToString(payloadBytes) + "." + base64.StdEncoding.EncodeToString(sigPayload)
	// 9. Transaction: Limit消費
	txErr := u.DB.Transaction(func(tx *gorm.DB) error {
		var txCube model.Cube
		if err := tx.Where("id = ?", sourceCube.ID).First(&txCube).Error; err != nil {
			return err
		}
		txPerm, err := common.ParseDatatypesJson[model.CubePermission](&txCube.Permissions)
		if err != nil {
			return err
		}
		if txPerm.GenKeyLimit < 0 {
			return fmt.Errorf("genkey limit exceeded")
		}
		if txPerm.GenKeyLimit > 0 {
			next := txPerm.GenKeyLimit - 1
			if next == 0 {
				next = -1
			}
			txPerm.GenKeyLimit = next
			newJSON, err := common.ToJson(txPerm)
			if err != nil {
				return err
			}
			txCube.Permissions = datatypes.JSON(newJSON)
			if err := tx.Save(&txCube).Error; err != nil {
				return err
			}
		}
		return nil
	})
	if txErr != nil {
		if txErr.Error() == "genkey limit exceeded" {
			return ForbiddenCustomMsg(c, res, "GenKey limit exceeded.")
		}
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Transaction failed: %s", txErr.Error()))
	}
	// Result
	res.Data.Key = keyStr
	return OK(c, &res.Data, res)
}

// ImportCube は.cubeファイルと鍵を受け取りCubeを復元（インポート）します。
func ImportCube(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.ImportCubeReq, res *rtres.ImportCubeRes) bool {
	// 1. Multipart Form Parsing (File)
	file, err := c.FormFile("file")
	if err != nil {
		return BadRequestCustomMsg(c, res, fmt.Sprintf("Failed to parse file: %s", err.Error()))
	}
	f, err := file.Open()
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to open uploaded file: %s", err.Error()))
	}
	defer f.Close()
	// Read Zip
	buf := new(bytes.Buffer)
	if _, err := buf.ReadFrom(f); err != nil {
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to read file: %s", err.Error()))
	}
	zipReader, err := zip.NewReader(bytes.NewReader(buf.Bytes()), int64(buf.Len()))
	if err != nil {
		return BadRequestCustomMsg(c, res, fmt.Sprintf("Invalid zip file: %s", err.Error()))
	}
	// 2. Extract Components from Zip
	readZipFile := func(name string) ([]byte, error) {
		for _, zf := range zipReader.File {
			if zf.Name == name {
				rc, err := zf.Open()
				if err != nil {
					return nil, err
				}
				defer rc.Close()
				return io.ReadAll(rc)
			}
		}
		return nil, fmt.Errorf("file not found: %s", name)
	}
	pubKeyBytes, err := readZipFile(PUBLIC_KEY_PEM)
	if err != nil {
		return BadRequestCustomMsg(c, res, fmt.Sprintf("Missing public_key.pem: %s", err.Error()))
	}
	exportIDBytes, err := readZipFile(EXPORT_ID_TXT)
	if err != nil {
		return BadRequestCustomMsg(c, res, fmt.Sprintf("Missing export_id.txt: %s", err.Error()))
	}
	exportIDFromZip := string(exportIDBytes)
	encData, err := readZipFile(ENCRYPTED_DATA_BIN)
	if err != nil {
		return BadRequestCustomMsg(c, res, fmt.Sprintf("Missing encrypted_data.bin: %s", err.Error()))
	}
	// 3. Parse Key String
	// Format: Base64(Payload) + "." + Base64(Signature)
	keyParts := strings.Split(req.Key, ".")
	if len(keyParts) != 2 {
		return BadRequestCustomMsg(c, res, fmt.Sprintf("Invalid key format: %s", req.Key))
	}
	payloadBytes, err := base64.StdEncoding.DecodeString(keyParts[0])
	if err != nil {
		return BadRequestCustomMsg(c, res, fmt.Sprintf("Failed to decode key payload: %s", err.Error()))
	}
	sigBytes, err := base64.StdEncoding.DecodeString(keyParts[1])
	if err != nil {
		return BadRequestCustomMsg(c, res, fmt.Sprintf("Failed to decode key signature: %s", err.Error()))
	}
	// Parse JSON Payload
	type KeyPayload struct {
		AESKey      string               `json:"aes_key"`
		Permissions model.CubePermission `json:"permissions"`
		ExpireAt    *time.Time           `json:"expire_at"`
		ExportID    uint                 `json:"export_id"`
	}
	var payload KeyPayload
	if err := json.Unmarshal(payloadBytes, &payload); err != nil {
		return BadRequestCustomMsg(c, res, fmt.Sprintf("Invalid key payload JSON: %s", err.Error()))
	}
	// 4. Verify Key Signature with Public Key from .cube
	blockPub, _ := pem.Decode(pubKeyBytes)
	if blockPub == nil {
		return BadRequestCustomMsg(c, res, "Failed to parse public key from zip")
	}
	publicKey, err := x509.ParsePKCS1PublicKey(blockPub.Bytes)
	if err != nil {
		return BadRequestCustomMsg(c, res, fmt.Sprintf("Failed to parse public key: %s", err.Error()))
	}
	hash := sha256.Sum256(payloadBytes)
	if err := rsa.VerifyPSS(publicKey, crypto.SHA256, hash[:], sigBytes, nil); err != nil {
		return BadRequestCustomMsg(c, res, fmt.Sprintf("Key signature verification failed: %s", err.Error()))
	}
	// 5. Integrity Checks
	// Export ID match
	if fmt.Sprintf("%d", payload.ExportID) != exportIDFromZip {
		return BadRequestCustomMsg(c, res, fmt.Sprintf("Key does not match this cube file (Export ID mismatch): %s", exportIDFromZip))
	}
	// Expiration check
	if payload.ExpireAt != nil && payload.ExpireAt.Before(time.Now()) {
		return ForbiddenCustomMsg(c, res, fmt.Sprintf("Key has expired: %s", common.ParseDatetimeToStr(payload.ExpireAt)))
	}
	// 6. Decrypt Data with AES Key
	aesKey, err := base64.StdEncoding.DecodeString(payload.AESKey)
	if err != nil {
		return BadRequestCustomMsg(c, res, fmt.Sprintf("Invalid AES key in payload: %s", err.Error()))
	}
	block, err := aes.NewCipher(aesKey)
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to create AES cipher: %s", err.Error()))
	}
	gcm, err := cipher.NewGCM(block)
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to create GCM: %s", err.Error()))
	}
	if len(encData) < gcm.NonceSize() {
		return BadRequestCustomMsg(c, res, fmt.Sprintf("Encrypted data too short: %d", len(encData)))
	}
	nonce := encData[:gcm.NonceSize()]
	ciphertext := encData[gcm.NonceSize():]
	plainData, err := gcm.Open(nil, nonce, ciphertext, nil)
	if err != nil {
		return BadRequestCustomMsg(c, res, fmt.Sprintf("Failed to decrypt data (AES key mismatch or corrupted): %s", err.Error()))
	}
	// 7. Extract plainData (inner zip) and restore files
	innerZipReader, err := zip.NewReader(bytes.NewReader(plainData), int64(len(plainData)))
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to read inner zip: %s", err.Error()))
	}
	// Generate new UUID for imported Cube
	newUUID := *common.GenUUID()
	ids := ju.IDs(!(ju.IsApx() || ju.IsFromKey()))
	cubeDbFilePath, err := u.GetCubeDBFilePath(&newUUID, ids.ApxID, ids.VdrID, ids.UsrID)
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to get cube path: %s", err.Error()))
	}
	// Create directory
	cubeDir := filepath.Dir(cubeDbFilePath)
	if err := os.MkdirAll(cubeDir, 0755); err != nil {
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to create cube directory: %s", err.Error()))
	}
	// Extract files from inner zip
	for _, zf := range innerZipReader.File {
		rc, err := zf.Open()
		if err != nil {
			os.RemoveAll(cubeDir)
			return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to extract file: %s", err.Error()))
		}
		content, err := io.ReadAll(rc)
		rc.Close()
		if err != nil {
			os.RemoveAll(cubeDir)
			return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to read extracted file: %s", err.Error()))
		}
		// Determine target path based on zf.Name
		// db/ prefix -> cubeDbFilePath
		// metadata.json, stats_usage.json, stats_contributors.json -> 特別な処理
		if after, ok := strings.CutPrefix(zf.Name, "db/"); ok {
			// KuzuDB files
			relativePath := after
			targetPath := filepath.Join(cubeDir, relativePath)
			targetDir := filepath.Dir(targetPath)
			if err := os.MkdirAll(targetDir, 0755); err != nil {
				os.RemoveAll(cubeDir)
				return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to create db directory: %s", err.Error()))
			}
			if err := os.WriteFile(targetPath, content, 0644); err != nil {
				os.RemoveAll(cubeDir)
				return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to write db file: %s", err.Error()))
			}
		}
		// metadata.json contains lineage info - we'll use this later
		// For now, we just ensure db files are extracted
	}
	// 8. Extract Lineage from inner zip (metadata.json)
	metadataBytes, err := func() ([]byte, error) {
		for _, zf := range innerZipReader.File {
			if zf.Name == METADATA_JSON {
				rc, err := zf.Open()
				if err != nil {
					return nil, err
				}
				defer rc.Close()
				return io.ReadAll(rc)
			}
		}
		return nil, nil // metadata may not exist
	}()
	var importedLineage []model.CubeLineage
	if len(metadataBytes) > 0 {
		_ = json.Unmarshal(metadataBytes, &importedLineage)
	}
	// 9. Transaction: Cube作成 + Lineage作成
	permJSON, _ := common.ToJson(payload.Permissions)
	var newCube model.Cube
	txErr := u.DB.Transaction(func(tx *gorm.DB) error {
		newCube = model.Cube{
			UUID:           newUUID,
			UsrID:          *ids.UsrID,
			Name:           req.Name,
			Description:    req.Description,
			ExpireAt:       payload.ExpireAt,
			Permissions:    datatypes.JSON(permJSON),
			SourceExportID: &payload.ExportID, // Link to source export for ReKey
			ApxID:          *ids.ApxID,
			VdrID:          *ids.VdrID,
		}
		if err := tx.Create(&newCube).Error; err != nil {
			return err
		}
		// Lineage作成
		for _, lin := range importedLineage {
			linRecord := model.CubeLineage{
				CubeID:        newCube.ID,
				AncestorUUID:  lin.AncestorUUID,
				AncestorOwner: lin.AncestorOwner,
				ExportedAt:    lin.ExportedAt,
				Generation:    lin.Generation,
				ApxID:         *ids.ApxID,
				VdrID:         *ids.VdrID,
			}
			if err := tx.Create(&linRecord).Error; err != nil {
				return err
			}
		}
		return nil
	})
	if txErr != nil {
		os.RemoveAll(cubeDir)
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Transaction failed: %s", txErr.Error()))
	}
	// Result
	res.Data.ID = newCube.ID
	res.Data.UUID = newUUID
	return OK(c, &res.Data, res)
}

// ReKeyCube は既存のCubeに新しい鍵を適用し、権限と有効期限を更新します。
func ReKeyCube(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.ReKeyCubeReq, res *rtres.ReKeyCubeRes) bool {
	ids := ju.IDs(!(ju.IsApx() || ju.IsFromKey()))
	// 1. Get Target Cube
	cube, err := getCube(u, req.CubeID, *ids.ApxID, *ids.VdrID)
	if err != nil {
		return NotFoundCustomMsg(c, res, fmt.Sprintf("Cube not found: %s", err.Error()))
	}
	if cube.UsrID != *ids.UsrID { // 自分が所有者になっている Cube に対してしか ReKey できない
		return ForbiddenCustomMsg(c, res, fmt.Sprintf("Not the owner of the cube: %d", cube.UsrID))
	}
	// 2. Check SourceExportID
	if cube.SourceExportID == nil {
		return BadRequestCustomMsg(c, res, fmt.Sprintf("This cube was not imported (no source export): %d", cube.ID))
	}
	// 3. Parse Key String
	keyParts := strings.Split(req.Key, ".")
	if len(keyParts) != 2 {
		return BadRequestCustomMsg(c, res, "Invalid key format.")
	}
	payloadBytes, err := base64.StdEncoding.DecodeString(keyParts[0])
	if err != nil {
		return BadRequestCustomMsg(c, res, fmt.Sprintf("Failed to decode key payload: %s", err.Error()))
	}
	sigBytes, err := base64.StdEncoding.DecodeString(keyParts[1])
	if err != nil {
		return BadRequestCustomMsg(c, res, fmt.Sprintf("Failed to decode key signature: %s", err.Error()))
	}
	// Parse JSON Payload
	type KeyPayload struct {
		AESKey      string               `json:"aes_key"`
		Permissions model.CubePermission `json:"permissions"`
		ExpireAt    *time.Time           `json:"expire_at"`
		ExportID    uint                 `json:"export_id"`
	}
	var payload KeyPayload
	if err := json.Unmarshal(payloadBytes, &payload); err != nil {
		return BadRequestCustomMsg(c, res, fmt.Sprintf("Invalid key payload JSON: %s", err.Error()))
	}
	// 4. Verify Key matches this Cube's source
	if payload.ExportID != *cube.SourceExportID {
		return BadRequestCustomMsg(c, res, fmt.Sprintf("Key does not match this cube's source: %d", payload.ExportID))
	}
	// 5. Fetch Export Record and derive Public Key
	var exportRecord model.Export
	if err := u.DB.First(&exportRecord, payload.ExportID).Error; err != nil {
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Export record not found: %s", err.Error()))
	}
	blockPriv, _ := pem.Decode([]byte(exportRecord.PrivateKey))
	if blockPriv == nil {
		return InternalServerErrorCustomMsg(c, res, "Failed to parse private key")
	}
	privateKey, err := x509.ParsePKCS1PrivateKey(blockPriv.Bytes)
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to parse private key: %s", err.Error()))
	}
	publicKey := &privateKey.PublicKey
	// 6. Verify Signature
	hash := sha256.Sum256(payloadBytes)
	if err := rsa.VerifyPSS(publicKey, crypto.SHA256, hash[:], sigBytes, nil); err != nil {
		return BadRequestCustomMsg(c, res, fmt.Sprintf("Key signature verification failed: %s", err.Error()))
	}
	// 7. Check Expiration
	if payload.ExpireAt != nil && payload.ExpireAt.Before(time.Now()) {
		return ForbiddenCustomMsg(c, res, fmt.Sprintf("Key has expired: %s", common.ParseDatetimeToStr(payload.ExpireAt)))
	}
	// 8. Limit Check（事前チェック）
	currentPerm, err := common.ParseDatatypesJson[model.CubePermission](&cube.Permissions)
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, "Failed to parse permissions")
	}
	if currentPerm.RekeyLimit < 0 {
		return ForbiddenCustomMsg(c, res, fmt.Sprintf("ReKey limit exceeded: %d", currentPerm.RekeyLimit))
	}
	// 9. Inheritance Check
	if err := CheckInheritance(currentPerm, payload.Permissions, cube.ExpireAt, payload.ExpireAt); err != nil {
		return BadRequestCustomMsg(c, res, fmt.Sprintf("Cube permissions inheritance error: %s", err.Error()))
	}
	// 10. Transaction: Limit消費 + Cube更新
	newPermJSON, _ := common.ToJson(payload.Permissions)
	txErr := u.DB.Transaction(func(tx *gorm.DB) error {
		var txCube model.Cube
		if err := tx.Where("id = ?", cube.ID).First(&txCube).Error; err != nil {
			return err
		}
		txPerm, err := common.ParseDatatypesJson[model.CubePermission](&txCube.Permissions)
		if err != nil {
			return err
		}
		// Limit再確認
		if txPerm.RekeyLimit < 0 {
			return fmt.Errorf("rekey limit exceeded")
		}
		// Limit消費（新しい権限に置き換えるのでtxPermの更新は不要、ただしチェックのみ）
		// 注意: payload.PermissionsにはRekeyLimitが含まれており、それが新しい値になる
		// Update Cube
		txCube.Permissions = datatypes.JSON(newPermJSON)
		txCube.ExpireAt = payload.ExpireAt
		if err := tx.Save(&txCube).Error; err != nil {
			return err
		}
		return nil
	})
	if txErr != nil {
		if txErr.Error() == "rekey limit exceeded" {
			return ForbiddenCustomMsg(c, res, "ReKey limit exceeded.")
		}
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Transaction failed: %s", txErr.Error()))
	}
	return OK[rtres.ReKeyCubeRes](c, nil, res)
}
