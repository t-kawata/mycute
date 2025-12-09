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
	data := rtres.CreateCubeResData{ID: newCube.ID, UUID: newUUID}
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
	tempFile := filepath.Join(os.TempDir(), fmt.Sprintf("%s.txt", *common.GenUUID()))
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

// ExportCube はCubeをエクスポートします。
func ExportCube(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.ExportCubeReq, res *rtres.ExportCubeRes) (*bytes.Buffer, string, bool) {
	// 1. Cubeの取得と権限チェック
	cube, err := getCube(u, req.CubeID, *ju.ApxID, *ju.VdrID)
	if err != nil {
		NotFoundCustomMsg(c, res, "Cube not found")
		return nil, "", false
	}
	// 権限JSONパース
	perm, err := common.ParseDatatypesJson[model.CubePermission](&cube.Permissions)
	if err != nil {
		InternalServerErrorCustomMsg(c, res, "Failed to parse permissions")
		return nil, "", false
	}
	// 2. Limit チェック
	if perm.ExportLimit < 0 {
		ForbiddenCustomMsg(c, res, "Export limit exceeded")
		return nil, "", false
	}
	nextLimit := perm.ExportLimit
	shouldUpdateLimit := false
	if perm.ExportLimit > 0 {
		nextLimit = perm.ExportLimit - 1
		if nextLimit == 0 {
			nextLimit = -1 // 0は無制限なので、使い切ったら-1(禁止)にする
		}
		shouldUpdateLimit = true
	}
	// Limit消費 (DB更新)
	if shouldUpdateLimit {
		perm.ExportLimit = nextLimit
		newJSONStr, err := common.ToJson(perm)
		if err != nil {
			InternalServerErrorCustomMsg(c, res, "Failed to serialize permissions")
			return nil, "", false
		}
		cube.Permissions = datatypes.JSON(newJSONStr)
		if err := u.DB.Save(cube).Error; err != nil {
			InternalServerErrorCustomMsg(c, res, "Failed to update export limit")
			return nil, "", false
		}
	}
	// 3. データ準備 (New UUID & Lineage)
	newUUID := *common.GenUUID()
	// Lineage取得
	var ancestors []model.CubeLineage
	if err := u.DB.Where("cube_id = ? AND apx_id = ? AND vdr_id = ?", cube.ID, *ju.ApxID, *ju.VdrID).Order("generation asc").Find(&ancestors).Error; err != nil {
		InternalServerErrorCustomMsg(c, res, "Failed to fetch lineage")
		return nil, "", false
	}
	// 自身のLineage追加
	ownerName, err := getJwtUsrName(u, ju.ApxID, ju.VdrID, ju.UsrID)
	if err != nil {
		InternalServerErrorCustomMsg(c, res, "Failed to get owner name")
		return nil, "", false
	}
	myLineage := model.CubeLineage{
		AncestorUUID:  cube.UUID,
		AncestorOwner: ownerName,
		ExportedAt:    time.Now().UnixMilli(),
		Generation:    len(ancestors) + 1,
	}
	exportLineageList := append(ancestors, myLineage)
	lineageJSON, err := common.ToJson(exportLineageList)
	if err != nil {
		InternalServerErrorCustomMsg(c, res, "Failed to serialize lineage")
		return nil, "", false
	}
	// 4. Statsデータ取得
	var modelStats []model.CubeModelStat
	u.DB.Where("cube_id = ? AND apx_id = ? AND vdr_id = ?", cube.ID, *ju.ApxID, *ju.VdrID).Find(&modelStats)
	statsUsageJSON, err := common.ToJson(modelStats)
	if err != nil {
		InternalServerErrorCustomMsg(c, res, "Failed to serialize stats usage")
		return nil, "", false
	}
	var contributors []model.CubeContributor
	u.DB.Where("cube_id = ? AND apx_id = ? AND vdr_id = ?", cube.ID, *ju.ApxID, *ju.VdrID).Find(&contributors)
	statsContribJSON, err := common.ToJson(contributors)
	if err != nil {
		InternalServerErrorCustomMsg(c, res, "Failed to serialize stats contributors")
		return nil, "", false
	}
	// 5. Zip作成
	// Cubeパス取得
	ids := ju.IDs(!(ju.IsApx() || ju.IsFromKey()))
	cubeDbFilePath, err := u.GetCubeDBFilePath(&cube.UUID, ids.ApxID, ids.VdrID, ids.UsrID)
	if err != nil {
		InternalServerErrorCustomMsg(c, res, "Failed to get cube path")
		return nil, "", false
	}
	extraFiles := map[string][]byte{
		"metadata.json":           []byte(lineageJSON),
		"stats_usage.json":        []byte(statsUsageJSON),
		"stats_contributors.json": []byte(statsContribJSON),
	}
	zipBuffer, err := cuber.ExportCubeToZip(cubeDbFilePath, extraFiles)
	if err != nil {
		InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Export failed: %s", err.Error()))
		return nil, "", false
	}
	// 6. Security & Packaging Sequence
	// --------------------------------------------------------------------------
	// 1. zipBuffer is already created

	// 2. Generate RSA Key Pair
	privateKey, err := rsa.GenerateKey(rand.Reader, 2048)
	if err != nil {
		InternalServerErrorCustomMsg(c, res, "Failed to generate RSA key")
		return nil, "", false
	}
	publicKey := &privateKey.PublicKey

	// Encode keys for storage/distribution
	privBytes := x509.MarshalPKCS1PrivateKey(privateKey)
	privPEM := pem.EncodeToMemory(&pem.Block{Type: "RSA PRIVATE KEY", Bytes: privBytes})

	pubBytes := x509.MarshalPKCS1PublicKey(publicKey)
	pubPEM := pem.EncodeToMemory(&pem.Block{Type: "RSA PUBLIC KEY", Bytes: pubBytes})

	// 3. Generate AES Key (32 bytes)
	aesKey := make([]byte, 32)
	if _, err := rand.Read(aesKey); err != nil {
		InternalServerErrorCustomMsg(c, res, "Failed to generate AES key")
		return nil, "", false
	}

	// 4. Encrypt zipBuffer with AES-GCM
	block, err := aes.NewCipher(aesKey)
	if err != nil {
		InternalServerErrorCustomMsg(c, res, "Failed to create AES cipher")
		return nil, "", false
	}
	gcm, err := cipher.NewGCM(block)
	if err != nil {
		InternalServerErrorCustomMsg(c, res, "Failed to create GCM")
		return nil, "", false
	}
	nonce := make([]byte, gcm.NonceSize())
	if _, err := rand.Read(nonce); err != nil {
		InternalServerErrorCustomMsg(c, res, "Failed to generate nonce")
		return nil, "", false
	}
	// Seal(dst, nonce, plaintext, additionalData)
	// We prepend the nonce to the ciphertext for decryption later.
	encryptedData := gcm.Seal(nonce, nonce, zipBuffer.Bytes(), nil)

	// 5. Encrypt AES Key with RSA Public Key (OAEP)
	encryptedAESKey, err := rsa.EncryptOAEP(sha256.New(), rand.Reader, publicKey, aesKey, nil)
	if err != nil {
		InternalServerErrorCustomMsg(c, res, "Failed to encrypt AES key")
		return nil, "", false
	}

	// 6. Create Signature for Encrypted Data
	hash := sha256.Sum256(encryptedData)
	signature, err := rsa.SignPSS(rand.Reader, privateKey, crypto.SHA256, hash[:], nil)
	if err != nil {
		InternalServerErrorCustomMsg(c, res, "Failed to sign data")
		return nil, "", false
	}

	// 7. Create Export Record
	dataHash := common.CalculateSHA256(encryptedData)

	record := model.Export{
		CubeID:     cube.ID,
		NewUUID:    newUUID,
		Hash:       dataHash,
		PrivateKey: string(privPEM),
		ApxID:      *ids.ApxID,
		VdrID:      *ids.VdrID,
	}
	// Note: 'newUUID' defined at line 336 is technically accessible.
	// However, I need to make sure I am using it correctly.
	// Let's assume 'newUUID' is available.
	// Wait, I am removing line 336? No, I am replacing from line 390 (step 6).
	// Let's look at the TargetContent carefully.
	// The target content is:
	// 	// 6. Export Record 作成
	// 	hash := common.CalculateSHA256(zipBuffer.Bytes())
	// 	record := model.Export{
	// 		CubeID:  cube.ID,
	// 		NewUUID: newUUID,
	// 		Hash:    hash,
	// 		ApxID:   *ids.ApxID,
	// 		VdrID:   *ids.VdrID,
	// 	}
	// 	if err := u.DB.Create(&record).Error; err != nil {
	// 		InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to create export record: %s", err.Error()))
	// 		return nil, "", false
	// 	}
	// 	fileName := fmt.Sprintf("cube_%d_%s.cube", cube.ID, newUUID)
	// 	return zipBuffer, fileName, true
	// }

	// So I am replacing the whole block.
	// I need to use 'newUUID' which is defined earlier.

	record.NewUUID = newUUID // newUUID is available in the scope.

	if err := u.DB.Create(&record).Error; err != nil {
		InternalServerErrorCustomMsg(c, res, "Failed to update export record")
		return nil, "", false
	}

	// 8. Create Final Zip
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

	if err := addToZip(zwFinal, "encrypted_data.bin", encryptedData); err != nil {
		InternalServerErrorCustomMsg(c, res, "Zip error")
		return nil, "", false
	}
	if err := addToZip(zwFinal, "signature.bin", signature); err != nil {
		InternalServerErrorCustomMsg(c, res, "Zip error")
		return nil, "", false
	}
	if err := addToZip(zwFinal, "public_key.pem", pubPEM); err != nil {
		InternalServerErrorCustomMsg(c, res, "Zip error")
		return nil, "", false
	}
	if err := addToZip(zwFinal, "encrypted_aes_key.bin", encryptedAESKey); err != nil {
		InternalServerErrorCustomMsg(c, res, "Zip error")
		return nil, "", false
	}
	if err := addToZip(zwFinal, "export_id.txt", []byte(fmt.Sprintf("%d", record.ID))); err != nil {
		InternalServerErrorCustomMsg(c, res, "Zip error")
		return nil, "", false
	}

	if err := zwFinal.Close(); err != nil {
		InternalServerErrorCustomMsg(c, res, "Zip close error")
		return nil, "", false
	}

	fileName := fmt.Sprintf("cube_%d_%s.cube", cube.ID, newUUID)
	return finalZip, fileName, true
}

// CheckInheritance は親子間の権限継承ルールを検証します。
func CheckInheritance(parent, child model.CubePermission, pExpire, cExpire *time.Time) error {
	// 1. 禁止機能の復活チェック (親が禁止(-1)なら、子も禁止(-1)でなければならない)
	// 親が0(無制限)なら何でもOK。親が正数(回数制限)の場合、子は制限以下...というチェックは複雑なため「機能の有無」に留める実装が一般的だが、
	// ここでは厳密に「親が禁止なら子も禁止」のみをチェックする。
	if parent.ExportLimit < 0 && child.ExportLimit >= 0 {
		return fmt.Errorf("Cannot enable export (parent forbidden)")
	}
	if parent.RekeyLimit < 0 && child.RekeyLimit >= 0 {
		return fmt.Errorf("Cannot enable rekey (parent forbidden)")
	}
	if parent.GenKeyLimit < 0 && child.GenKeyLimit >= 0 {
		return fmt.Errorf("Cannot enable genkey (parent forbidden)")
	}
	if parent.AbsorbLimit < 0 && child.AbsorbLimit >= 0 {
		return fmt.Errorf("Cannot enable absorb (parent forbidden)")
	}
	if parent.MemifyLimit < 0 && child.MemifyLimit >= 0 {
		return fmt.Errorf("Cannot enable memify (parent forbidden)")
	}
	if parent.SearchLimit < 0 && child.SearchLimit >= 0 {
		return fmt.Errorf("Cannot enable search (parent forbidden)")
	}
	// Stats
	if !parent.AllowStats && child.AllowStats {
		return fmt.Errorf("Cannot enable stats (parent forbidden)")
	}

	// 2. Expire チェック
	// 親に期限がある場合、子はそれより前でなければならない
	if pExpire != nil {
		if cExpire == nil {
			return fmt.Errorf("Cannot remove expiration (parent has expire)")
		}
		if cExpire.After(*pExpire) {
			return fmt.Errorf("Cannot extend expiration beyond parent")
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
		return BadRequestCustomMsg(c, res, "File 'file' is required")
	}
	f, err := file.Open()
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, "Failed to open uploaded file")
	}
	defer f.Close()

	// Read Zip
	buf := new(bytes.Buffer)
	if _, err := buf.ReadFrom(f); err != nil {
		return InternalServerErrorCustomMsg(c, res, "Failed to read file")
	}
	zipReader, err := zip.NewReader(bytes.NewReader(buf.Bytes()), int64(buf.Len()))
	if err != nil {
		return BadRequestCustomMsg(c, res, "Invalid zip file")
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

	encAESKey, err := readZipFile("encrypted_aes_key.bin")
	if err != nil {
		return BadRequestCustomMsg(c, res, "Missing encrypted_aes_key.bin")
	}
	signature, err := readZipFile("signature.bin")
	if err != nil {
		return BadRequestCustomMsg(c, res, "Missing signature.bin")
	}
	pubKeyBytes, err := readZipFile("public_key.pem")
	if err != nil {
		return BadRequestCustomMsg(c, res, "Missing public_key.pem")
	}
	exportIDBytes, err := readZipFile("export_id.txt")
	if err != nil {
		return BadRequestCustomMsg(c, res, "Missing export_id.txt")
	}
	exportIDStr := string(exportIDBytes)
	encData, err := readZipFile("encrypted_data.bin")
	if err != nil {
		return BadRequestCustomMsg(c, res, "Missing encrypted_data.bin")
	}

	// 3. DB Lookup (Export Record)
	ids := ju.IDs(!(ju.IsApx() || ju.IsFromKey()))
	var exp model.Export
	if err := u.DB.Where("id = ? AND apx_id = ? AND vdr_id = ?", exportIDStr, ids.ApxID, ids.VdrID).First(&exp).Error; err != nil {
		return NotFoundCustomMsg(c, res, "Export record not found or access denied")
	}
	// Check Owner via Source Cube
	var sourceCube model.Cube
	if err := u.DB.First(&sourceCube, exp.CubeID).Error; err != nil {
		return InternalServerErrorCustomMsg(c, res, "Source cube not found")
	}
	if sourceCube.UsrID != *ju.UsrID {
		return ForbiddenCustomMsg(c, res, "Not the owner of the source cube")
	}

	// Parse Private Key from DB
	blockPriv, _ := pem.Decode([]byte(exp.PrivateKey))
	if blockPriv == nil {
		return InternalServerErrorCustomMsg(c, res, "Failed to parse private key from DB")
	}
	privateKey, err := x509.ParsePKCS1PrivateKey(blockPriv.Bytes)
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, "Failed to parse private key")
	}

	// 4. Verify Signature (Integrity Check)
	blockPub, _ := pem.Decode(pubKeyBytes)
	if blockPub == nil {
		return BadRequestCustomMsg(c, res, "Failed to parse public key from zip")
	}
	publicKey, err := x509.ParsePKCS1PublicKey(blockPub.Bytes)
	if err != nil {
		return BadRequestCustomMsg(c, res, "Failed to parse public key")
	}

	hash := sha256.Sum256(encData)
	if err := rsa.VerifyPSS(publicKey, crypto.SHA256, hash[:], signature, nil); err != nil {
		return BadRequestCustomMsg(c, res, "File signature verification failed (tampered?)")
	}

	// 5. Decrypt AES Key
	aesKey, err := rsa.DecryptOAEP(sha256.New(), rand.Reader, privateKey, encAESKey, nil)
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, "Failed to decrypt AES key")
	}

	// 6. Inheritance Check
	parentPerm, err := common.ParseDatatypesJson[model.CubePermission](&sourceCube.Permissions)
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, "Failed to parse source permissions")
	}

	var reqExpire *time.Time
	if req.ExpireAt != nil {
		t, err := common.ParseStrToDatetime(req.ExpireAt)
		if err != nil {
			return BadRequestCustomMsg(c, res, "Invalid expire_at format")
		}
		reqExpire = &t
	}

	if err := CheckInheritance(parentPerm, req.Permissions, sourceCube.ExpireAt, reqExpire); err != nil {
		return BadRequestCustomMsg(c, res, fmt.Sprintf("Inheritance error: %s", err.Error()))
	}

	// 7. Limit Consumption (GenKeyLimit)
	if parentPerm.GenKeyLimit < 0 {
		return ForbiddenCustomMsg(c, res, "GenKey limit exceeded")
	}
	shouldConsume := parentPerm.GenKeyLimit > 0
	if shouldConsume {
		next := parentPerm.GenKeyLimit - 1
		if next == 0 {
			next = -1
		}
		parentPerm.GenKeyLimit = next
		newJSON, _ := common.ToJson(parentPerm)
		sourceCube.Permissions = datatypes.JSON(newJSON)
		u.DB.Save(&sourceCube)
	}

	// 8. Construct Key Payload & Sign
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
		return InternalServerErrorCustomMsg(c, res, "Failed to marshal key payload")
	}

	// Sign Payload
	hashPayload := sha256.Sum256(payloadBytes)
	sigPayload, err := rsa.SignPSS(rand.Reader, privateKey, crypto.SHA256, hashPayload[:], nil)
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, "Failed to sign key payload")
	}

	// 9. Format Final Key
	keyStr := base64.StdEncoding.EncodeToString(payloadBytes) + "." + base64.StdEncoding.EncodeToString(sigPayload)

	// Result
	res.Data.Key = keyStr
	return OK(c, &res.Data, res)
}

// ImportCube は.cubeファイルと鍵を受け取りCubeを復元（インポート）します。
func ImportCube(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.ImportCubeReq, res *rtres.ImportCubeRes) bool {
	// 1. Multipart Form Parsing (File)
	file, err := c.FormFile("file")
	if err != nil {
		return BadRequestCustomMsg(c, res, "File 'file' is required")
	}
	f, err := file.Open()
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, "Failed to open uploaded file")
	}
	defer f.Close()

	// Read Zip
	buf := new(bytes.Buffer)
	if _, err := buf.ReadFrom(f); err != nil {
		return InternalServerErrorCustomMsg(c, res, "Failed to read file")
	}
	zipReader, err := zip.NewReader(bytes.NewReader(buf.Bytes()), int64(buf.Len()))
	if err != nil {
		return BadRequestCustomMsg(c, res, "Invalid zip file")
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

	pubKeyBytes, err := readZipFile("public_key.pem")
	if err != nil {
		return BadRequestCustomMsg(c, res, "Missing public_key.pem")
	}
	exportIDBytes, err := readZipFile("export_id.txt")
	if err != nil {
		return BadRequestCustomMsg(c, res, "Missing export_id.txt")
	}
	exportIDFromZip := string(exportIDBytes)
	encData, err := readZipFile("encrypted_data.bin")
	if err != nil {
		return BadRequestCustomMsg(c, res, "Missing encrypted_data.bin")
	}

	// 3. Parse Key String
	// Format: Base64(Payload) + "." + Base64(Signature)
	keyParts := strings.Split(req.Key, ".")
	if len(keyParts) != 2 {
		return BadRequestCustomMsg(c, res, "Invalid key format")
	}
	payloadBytes, err := base64.StdEncoding.DecodeString(keyParts[0])
	if err != nil {
		return BadRequestCustomMsg(c, res, "Failed to decode key payload")
	}
	sigBytes, err := base64.StdEncoding.DecodeString(keyParts[1])
	if err != nil {
		return BadRequestCustomMsg(c, res, "Failed to decode key signature")
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
		return BadRequestCustomMsg(c, res, "Invalid key payload JSON")
	}

	// 4. Verify Key Signature with Public Key from .cube
	blockPub, _ := pem.Decode(pubKeyBytes)
	if blockPub == nil {
		return BadRequestCustomMsg(c, res, "Failed to parse public key from zip")
	}
	publicKey, err := x509.ParsePKCS1PublicKey(blockPub.Bytes)
	if err != nil {
		return BadRequestCustomMsg(c, res, "Failed to parse public key")
	}

	hash := sha256.Sum256(payloadBytes)
	if err := rsa.VerifyPSS(publicKey, crypto.SHA256, hash[:], sigBytes, nil); err != nil {
		return BadRequestCustomMsg(c, res, "Key signature verification failed")
	}

	// 5. Integrity Checks
	// Export ID match
	if fmt.Sprintf("%d", payload.ExportID) != exportIDFromZip {
		return BadRequestCustomMsg(c, res, "Key does not match this cube file (Export ID mismatch)")
	}
	// Expiration check
	if payload.ExpireAt != nil && payload.ExpireAt.Before(time.Now()) {
		return ForbiddenCustomMsg(c, res, "Key has expired")
	}

	// 6. Decrypt Data with AES Key
	aesKey, err := base64.StdEncoding.DecodeString(payload.AESKey)
	if err != nil {
		return BadRequestCustomMsg(c, res, "Invalid AES key in payload")
	}

	block, err := aes.NewCipher(aesKey)
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, "Failed to create AES cipher")
	}
	gcm, err := cipher.NewGCM(block)
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, "Failed to create GCM")
	}
	if len(encData) < gcm.NonceSize() {
		return BadRequestCustomMsg(c, res, "Encrypted data too short")
	}
	nonce := encData[:gcm.NonceSize()]
	ciphertext := encData[gcm.NonceSize():]
	plainData, err := gcm.Open(nil, nonce, ciphertext, nil)
	if err != nil {
		return BadRequestCustomMsg(c, res, "Failed to decrypt data (AES key mismatch or corrupted)")
	}

	// 7. Extract plainData (inner zip) and restore files
	innerZipReader, err := zip.NewReader(bytes.NewReader(plainData), int64(len(plainData)))
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, "Failed to read inner zip")
	}

	// Generate new UUID for imported Cube
	newUUID := *common.GenUUID()
	ids := ju.IDs(!(ju.IsApx() || ju.IsFromKey()))
	cubeDbFilePath, err := u.GetCubeDBFilePath(&newUUID, ids.ApxID, ids.VdrID, ids.UsrID)
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, "Failed to get cube path")
	}

	// Create directory
	cubeDir := filepath.Dir(cubeDbFilePath)
	if err := os.MkdirAll(cubeDir, 0755); err != nil {
		return InternalServerErrorCustomMsg(c, res, "Failed to create cube directory")
	}

	// Extract files from inner zip
	for _, zf := range innerZipReader.File {
		rc, err := zf.Open()
		if err != nil {
			os.RemoveAll(cubeDir)
			return InternalServerErrorCustomMsg(c, res, "Failed to extract file")
		}
		content, err := io.ReadAll(rc)
		rc.Close()
		if err != nil {
			os.RemoveAll(cubeDir)
			return InternalServerErrorCustomMsg(c, res, "Failed to read extracted file")
		}

		// Determine target path based on zf.Name
		// db/ prefix -> cubeDbFilePath
		// metadata.json, stats_usage.json, stats_contributors.json -> handle specially
		if after, ok := strings.CutPrefix(zf.Name, "db/"); ok {
			// KuzuDB files
			relativePath := after
			targetPath := filepath.Join(cubeDir, relativePath)
			targetDir := filepath.Dir(targetPath)
			if err := os.MkdirAll(targetDir, 0755); err != nil {
				os.RemoveAll(cubeDir)
				return InternalServerErrorCustomMsg(c, res, "Failed to create db directory")
			}
			if err := os.WriteFile(targetPath, content, 0644); err != nil {
				os.RemoveAll(cubeDir)
				return InternalServerErrorCustomMsg(c, res, "Failed to write db file")
			}
		}
		// metadata.json contains lineage info - we'll use this later
		// For now, we just ensure db files are extracted
	}

	// 8. Extract Lineage from inner zip (metadata.json)
	metadataBytes, err := func() ([]byte, error) {
		for _, zf := range innerZipReader.File {
			if zf.Name == "metadata.json" {
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

	// 9. Register new Cube in DB
	descVal := ""
	if req.Description != nil {
		descVal = *req.Description
	}
	permJSON, _ := common.ToJson(payload.Permissions)
	newCube := model.Cube{
		UUID:           newUUID,
		UsrID:          *ids.UsrID,
		Name:           req.Name,
		Description:    descVal,
		ExpireAt:       payload.ExpireAt,
		Permissions:    datatypes.JSON(permJSON),
		SourceExportID: &payload.ExportID, // Link to source export for ReKey
		ApxID:          *ids.ApxID,
		VdrID:          *ids.VdrID,
	}
	if err := u.DB.Create(&newCube).Error; err != nil {
		os.RemoveAll(cubeDir)
		return InternalServerErrorCustomMsg(c, res, "Failed to save cube record")
	}

	// 10. Register Lineage
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
		u.DB.Create(&linRecord)
	}

	// Result
	res.Data.ID = newCube.ID
	res.Data.UUID = newUUID
	return OK(c, &res.Data, res)
}

// ReKeyCube は既存のCubeに新しい鍵を適用し、権限と有効期限を更新します。
func ReKeyCube(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.ReKeyCubeReq, res *rtres.ReKeyCubeRes) bool {
	// 1. Get Target Cube
	cube, err := getCube(u, req.CubeID, *ju.ApxID, *ju.VdrID)
	if err != nil {
		return NotFoundCustomMsg(c, res, "Cube not found")
	}
	if cube.UsrID != *ju.UsrID {
		return ForbiddenCustomMsg(c, res, "Not the owner of the cube")
	}

	// 2. Check SourceExportID
	if cube.SourceExportID == nil {
		return BadRequestCustomMsg(c, res, "This cube was not imported (no source export)")
	}

	// 3. Parse Key String
	keyParts := strings.Split(req.Key, ".")
	if len(keyParts) != 2 {
		return BadRequestCustomMsg(c, res, "Invalid key format")
	}
	payloadBytes, err := base64.StdEncoding.DecodeString(keyParts[0])
	if err != nil {
		return BadRequestCustomMsg(c, res, "Failed to decode key payload")
	}
	sigBytes, err := base64.StdEncoding.DecodeString(keyParts[1])
	if err != nil {
		return BadRequestCustomMsg(c, res, "Failed to decode key signature")
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
		return BadRequestCustomMsg(c, res, "Invalid key payload JSON")
	}

	// 4. Verify Key matches this Cube's source
	if payload.ExportID != *cube.SourceExportID {
		return BadRequestCustomMsg(c, res, "Key does not match this cube's source")
	}

	// 5. Fetch Export Record and derive Public Key
	var exportRecord model.Export
	if err := u.DB.First(&exportRecord, payload.ExportID).Error; err != nil {
		return InternalServerErrorCustomMsg(c, res, "Export record not found")
	}

	blockPriv, _ := pem.Decode([]byte(exportRecord.PrivateKey))
	if blockPriv == nil {
		return InternalServerErrorCustomMsg(c, res, "Failed to parse private key")
	}
	privateKey, err := x509.ParsePKCS1PrivateKey(blockPriv.Bytes)
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, "Failed to parse private key")
	}
	publicKey := &privateKey.PublicKey

	// 6. Verify Signature
	hash := sha256.Sum256(payloadBytes)
	if err := rsa.VerifyPSS(publicKey, crypto.SHA256, hash[:], sigBytes, nil); err != nil {
		return BadRequestCustomMsg(c, res, "Key signature verification failed")
	}

	// 7. Check Expiration
	if payload.ExpireAt != nil && payload.ExpireAt.Before(time.Now()) {
		return ForbiddenCustomMsg(c, res, "Key has expired")
	}

	// 8. Check RekeyLimit
	currentPerm, err := common.ParseDatatypesJson[model.CubePermission](&cube.Permissions)
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, "Failed to parse permissions")
	}
	if currentPerm.RekeyLimit < 0 {
		return ForbiddenCustomMsg(c, res, "ReKey limit exceeded")
	}

	// 9. Inheritance Check
	if err := CheckInheritance(currentPerm, payload.Permissions, cube.ExpireAt, payload.ExpireAt); err != nil {
		return BadRequestCustomMsg(c, res, fmt.Sprintf("Inheritance error: %s", err.Error()))
	}

	// 10. Consume RekeyLimit
	if currentPerm.RekeyLimit > 0 {
		next := currentPerm.RekeyLimit - 1
		if next == 0 {
			next = -1
		}
		currentPerm.RekeyLimit = next
	}

	// 11. Update Cube
	newPermJSON, _ := common.ToJson(payload.Permissions)
	cube.Permissions = datatypes.JSON(newPermJSON)
	cube.ExpireAt = payload.ExpireAt
	if err := u.DB.Save(cube).Error; err != nil {
		return InternalServerErrorCustomMsg(c, res, "Failed to update cube")
	}

	return OK[rtres.ReKeyCubeRes](c, nil, res)
}
