package rtbl

import (
	"bytes"
	"context"
	"fmt"
	"io"
	"os"
	"path/filepath"
	"slices"
	"strings"
	"time"

	"github.com/gin-gonic/gin"
	"github.com/t-kawata/mycute/lib/common"
	"github.com/t-kawata/mycute/lib/mycrypto"
	"github.com/t-kawata/mycute/mode/rt/rtreq"
	"github.com/t-kawata/mycute/mode/rt/rtres"
	"github.com/t-kawata/mycute/mode/rt/rtstream"
	"github.com/t-kawata/mycute/mode/rt/rtutil"
	"github.com/t-kawata/mycute/model"
	"github.com/t-kawata/mycute/pkg/cuber"
	"github.com/t-kawata/mycute/pkg/cuber/event"
	"github.com/t-kawata/mycute/pkg/cuber/storage"
	"github.com/t-kawata/mycute/pkg/cuber/types"
	"github.com/t-kawata/mycute/pkg/cuber/utils"
	"github.com/t-kawata/mycute/sql/restsql"
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
	EMBEDDING_CONFIG_JSON   = "embedding_config.json"
	ENCRYPTED_DATA_BIN      = "encrypted_data.bin"
	SIGNATURE_BIN           = "signature.bin"
	PUBLIC_KEY_PEM          = "public_key.pem"
	ENCRYPTED_AES_KEY_BIN   = "encrypted_aes_key.bin"
	EXPORT_ID_TXT           = "export_id.txt"
)

var (
	MIN_STREAM_DELAY = 25 * time.Millisecond // 最低25ms間隔（40 letters/s）
	TOKEN_SIZE       = 3                     // 演出としてのトークン区切りを何文字単位にするか
)

func getCube(u *rtutil.RtUtil, id uint, apxID uint, vdrID uint) (*model.Cube, error) {
	var cube model.Cube
	if err := u.DB.Where("id = ? AND apx_id = ? AND vdr_id = ?", id, apxID, vdrID).First(&cube).Error; err != nil {
		return nil, err
	}
	return &cube, nil
}

func fetchChatModelConfig(u *rtutil.RtUtil, chatModelID uint, apxID uint, vdrID uint) (types.ChatModelConfig, error) {
	var chatModel model.ChatModel
	if err := u.DB.Where("id = ? AND apx_id = ? AND vdr_id = ?", chatModelID, apxID, vdrID).First(&chatModel).Error; err != nil {
		return types.ChatModelConfig{}, err
	}
	decryptedKey, err := mycrypto.Decrypt(chatModel.ApiKey, u.CuberCryptoSkey)
	if err != nil {
		return types.ChatModelConfig{}, fmt.Errorf("failed to decrypt api key: %w", err)
	}
	return types.ChatModelConfig{
		Provider:    chatModel.Provider,
		Model:       chatModel.Model,
		BaseURL:     chatModel.BaseURL,
		ApiKey:      decryptedKey,
		MaxTokens:   chatModel.MaxTokens,
		Temperature: &chatModel.Temperature,
	}, nil
}

// SearchCubes は条件に一致するCubeを検索し、詳細情報を返します。
func SearchCubes(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.SearchCubesReq, res *rtres.SearchCubesRes) bool {
	cubes := []model.Cube{}
	ids := ju.IDs(!(ju.IsApx() || ju.IsFromKey()))
	// 1. 検索 (restsql)
	r := restsql.SearchCubes(u.DB, &cubes, ids, "c1", req, &[]string{"name", "description"}, nil, true)
	if r.Error != nil {
		return InternalServerError(c, res)
	}
	results := []rtres.SearchCubesResData{}
	// 2. 詳細情報の付加
	for _, cube := range cubes {
		permissions, err := common.ParseDatatypesJson[model.CubePermissions](&cube.Permissions)
		if err != nil {
			return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to parse permissions: %s", err.Error()))
		}
		var lineageRes []rtres.LineageRes
		var memoryGroupsRes []rtres.MemoryGroupStatsRes
		if permissions.AllowStats {
			l, err := fetchLineage(u.DB, cube.ID, *ids.ApxID, *ids.VdrID)
			if err == nil {
				lineageRes = l
			}
			m, err := fetchMemoryGroupStats(u.DB, cube.ID, *ids.ApxID, *ids.VdrID)
			if err == nil {
				memoryGroupsRes = m
			}
		} else {
			lineageRes = []rtres.LineageRes{}
			memoryGroupsRes = []rtres.MemoryGroupStatsRes{}
		}
		results = append(results, rtres.SearchCubesResData{
			Cube: rtres.SearchCubesResCube{
				ID:                 cube.ID,
				UUID:               cube.UUID,
				Name:               cube.Name,
				Description:        cube.Description,
				ExpireAt:           common.ParseDatetimeToStr(cube.ExpireAt),
				Permissions:        permissions,
				SourceExportID:     cube.SourceExportID,
				ApxID:              cube.ApxID,
				VdrID:              cube.VdrID,
				CreatedAt:          common.ParseDatetimeToStr(&cube.CreatedAt),
				UpdatedAt:          common.ParseDatetimeToStr(&cube.UpdatedAt),
				EmbeddingProvider:  cube.EmbeddingProvider,
				EmbeddingBaseURL:   cube.EmbeddingBaseURL,
				EmbeddingModel:     cube.EmbeddingModel,
				EmbeddingDimension: cube.EmbeddingDimension,
			},
			Lineage:      lineageRes,
			MemoryGroups: memoryGroupsRes,
		})
	}
	return OK(c, &results, res)
}

// GetCube はCubeの詳細情報を取得します。
func GetCube(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.GetCubeReq, res *rtres.GetCubeRes) bool {
	ids := ju.IDs(!(ju.IsApx() || ju.IsFromKey()))
	// 1. Cubeの取得と権限チェック
	cube, err := getCube(u, req.ID, *ids.ApxID, *ids.VdrID)
	if err != nil {
		return NotFoundCustomMsg(c, res, "Cube not found.")
	}
	// 権限JSONパース
	permissions, err := common.ParseDatatypesJson[model.CubePermissions](&cube.Permissions)
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, "Failed to parse permissions.")
	}
	var lineageRes []rtres.LineageRes
	var memoryGroupsRes []rtres.MemoryGroupStatsRes
	// 2. AllowStats チェック
	if permissions.AllowStats {
		lineageRes, err = fetchLineage(u.DB, cube.ID, *ids.ApxID, *ids.VdrID)
		if err != nil {
			return InternalServerErrorCustomMsg(c, res, "Failed to fetch lineage.")
		}
		memoryGroupsRes, err = fetchMemoryGroupStats(u.DB, cube.ID, *ids.ApxID, *ids.VdrID)
		if err != nil {
			return InternalServerErrorCustomMsg(c, res, "Failed to fetch stats.")
		}
	} else {
		lineageRes = []rtres.LineageRes{}
		memoryGroupsRes = []rtres.MemoryGroupStatsRes{}
	}
	data := rtres.GetCubeResData{
		Cube: rtres.GetCubeResCube{
			ID:             cube.ID,
			UUID:           cube.UUID,
			Name:           cube.Name,
			Description:    cube.Description,
			ExpireAt:       common.ParseDatetimeToStr(cube.ExpireAt),
			Permissions:    permissions,
			SourceExportID: cube.SourceExportID,
			ApxID:          cube.ApxID,
			VdrID:          cube.VdrID,
			CreatedAt:      common.ParseDatetimeToStr(&cube.CreatedAt),
			UpdatedAt:      common.ParseDatetimeToStr(&cube.UpdatedAt),
		},
		Lineage:      lineageRes,
		MemoryGroups: memoryGroupsRes,
	}
	return OK(c, &data, res)
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
	cubeDir := filepath.Dir(cubeDBFilePath)
	if err := os.MkdirAll(cubeDir, 0755); err != nil {
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to create cube directory: %s", err.Error()))
	}
	// 4. KuzuDB 初期化 (Dimension を渡す)
	if err := cuber.CreateCubeDB(cubeDBFilePath, types.EmbeddingModelConfig{
		Provider:  req.EmbeddingProvider,
		Model:     req.EmbeddingModel,
		Dimension: req.EmbeddingDimension,
	}, u.Logger); err != nil {
		os.RemoveAll(cubeDir) // Clean up
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to initialize cube: %s", err.Error()))
	}
	// 5. 初期権限設定
	initialPerms := model.CubePermissions{
		ExportLimit: 0, RekeyLimit: 0, GenKeyLimit: 0,
		AbsorbLimit: 0, MemifyLimit: 0, QueryLimit: 0,
		AllowStats:        true,
		MemifyConfigLimit: map[string]any{},
		QueryTypeLimit:    []uint8{},
	}
	permJSON, err := common.ToJson(initialPerms)
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to convert initial permission to JSON: %s", err.Error()))
	}
	// 6. DBレコード作成
	// Encyrpt API Key
	encryptedEmbeddingApiKey, err := mycrypto.Encrypt(req.EmbeddingApiKey, u.CuberCryptoSkey)
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to encrypt embedding API key: %s", err.Error()))
	}
	newCube := model.Cube{
		UUID:               newUUID,
		UsrID:              *ids.UsrID,
		Name:               req.Name,
		Description:        req.Description,
		EmbeddingProvider:  req.EmbeddingProvider,
		EmbeddingModel:     req.EmbeddingModel,
		EmbeddingDimension: req.EmbeddingDimension,
		EmbeddingBaseURL:   req.EmbeddingBaseURL,
		EmbeddingApiKey:    encryptedEmbeddingApiKey,
		ExpireAt:           nil, // 自分がゼロから作成するCubeは無期限
		Permissions:        datatypes.JSON(permJSON),
		ApxID:              *ids.ApxID,
		VdrID:              *ids.VdrID,
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
// SSEストリーミングモードに対応。
func AbsorbCube(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.AbsorbCubeReq, res *rtres.AbsorbCubeRes) bool {
	ids := ju.IDs(!(ju.IsApx() || ju.IsFromKey()))
	// 1. Cubeの取得と権限チェック
	cube, err := getCube(u, req.CubeID, *ids.ApxID, *ids.VdrID)
	if err != nil {
		return NotFoundCustomMsg(c, res, "Cube not found.")
	}
	// 権限JSONパース
	perm, err := common.ParseDatatypesJson[model.CubePermissions](&cube.Permissions)
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
	// Decrypt API Key
	decryptedEmbeddingApiKey, err := mycrypto.Decrypt(cube.EmbeddingApiKey, u.CuberCryptoSkey)
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to decrypt embedding API key: %s", err.Error()))
	}
	// Fetch Chat Model Config
	chatConf, err := fetchChatModelConfig(u, req.ChatModelID, *ids.ApxID, *ids.VdrID)
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to fetch chat model: %s", err.Error()))
	}

	// 5. ストリーミング設定
	var streamWriter *rtstream.StreamWriter
	if req.Stream {
		c.Header("Content-Type", "text/event-stream")
		c.Header("Cache-Control", "no-cache")
		c.Header("Connection", "keep-alive")
		c.Header("X-Accel-Buffering", "no") // Nginx対策
		streamWriter = rtstream.NewStreamWriter(c.Request.Context(), MIN_STREAM_DELAY)
		requestUUID := common.GenUUID() // リクエスト単位で共通のID
		// ストリーム送信ゴルーチン
		go func() {
			defer streamWriter.Done() // ゴルーチン終了時にDoneを呼び出す
			ticker := time.NewTicker(streamWriter.MinDelay())
			defer ticker.Stop()
			for {
				select {
				case token, ok := <-streamWriter.Ch():
					if !ok {
						// チャンネルクローズ = 終了
						fmt.Fprint(c.Writer, rtstream.CreateSSEChunk(*requestUUID, "cuber-absorb", "", true))
						c.Writer.Flush()
						return
					}
					// OpenAI形式のチャンク送信
					chunk := rtstream.CreateSSEChunk(*requestUUID, "cuber-absorb", token, false)
					fmt.Fprint(c.Writer, chunk)
					c.Writer.Flush()
					// 最低遅延を保証
					<-ticker.C
				case <-c.Request.Context().Done():
					return
				}
			}
		}()
	}
	// 6. Absorb実行
	type AbsorbResult struct {
		Usage types.TokenUsage
		Err   error
	}
	dataCh := make(chan event.StreamEvent)
	resultCh := make(chan AbsorbResult, 1)
	isEn := req.IsEn
	ctx, cancel := context.WithCancel(c.Request.Context())
	defer cancel()
	go func() {
		u, e := u.CuberService.Absorb(ctx, u.EventBus, cubeDbFilePath, req.MemoryGroup, []string{tempFile},
			types.CognifyConfig{
				ChunkSize:    req.ChunkSize,
				ChunkOverlap: req.ChunkOverlap,
			},
			types.EmbeddingModelConfig{
				Provider:  cube.EmbeddingProvider,
				Model:     cube.EmbeddingModel,
				Dimension: cube.EmbeddingDimension,
				BaseURL:   cube.EmbeddingBaseURL,
				ApiKey:    decryptedEmbeddingApiKey,
			},
			chatConf,
			dataCh,
			isEn,
		)
		resultCh <- AbsorbResult{Usage: u, Err: e}
	}()
	var usage types.TokenUsage
AbsorbLoop:
	for {
		select {
		case evt := <-dataCh:
			msg, fmtErr := event.FormatEvent(evt, isEn)
			if fmtErr == nil {
				if req.Stream {
					utils.LogInfo(u.Logger, "=================================")
					utils.LogInfo(u.Logger, fmt.Sprintf("%s: %s", evt.EventName, msg))
					utils.LogInfo(u.Logger, "=================================")
					streamWriter.Write("- [x]: ")
					time.Sleep(MIN_STREAM_DELAY)
					// トークン化してストリームに流す
					tokens := rtstream.Tokenize(msg, TOKEN_SIZE)
					for _, token := range tokens {
						streamWriter.Write(token)
					}
					streamWriter.Write(fmt.Sprintf(" (%s)", evt.EventName))
					time.Sleep(MIN_STREAM_DELAY)
					streamWriter.Write("\n\n")
					time.Sleep(MIN_STREAM_DELAY)
				} else {
					// 既存のログ出力
					utils.LogInfo(u.Logger, "=================================")
					utils.LogInfo(u.Logger, fmt.Sprintf("%s: %s", evt.EventName, msg))
					utils.LogInfo(u.Logger, "=================================")
				}
			} else {
				err = fmt.Errorf("Failed to format event '%s': %s", evt.EventName, fmtErr.Error())
				break AbsorbLoop
			}
		case result := <-resultCh:
			usage = result.Usage
			err = result.Err
			break AbsorbLoop
		case <-ctx.Done():
			if req.Stream && streamWriter != nil {
				streamWriter.Close()
				streamWriter.Wait()
			}
			return InternalServerErrorCustomMsg(c, res, "Request cancelled")
		}
	}
	// 7. エラーチェック（ストリーミング含む）
	if err != nil {
		if req.Stream && streamWriter != nil {
			// エラーメッセージをストリームで送信
			errorMsg := fmt.Sprintf("\nError: Absorb failed - %s", err.Error())
			tokens := rtstream.Tokenize(errorMsg, TOKEN_SIZE)
			for _, token := range tokens {
				streamWriter.Write(token)
			}
			streamWriter.Close()
			streamWriter.Wait()
		}
		// ストリームモードでもエラーはロールバック（DB更新しない）
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Absorb failed: %s", err.Error()))
	}
	// usageチェック
	if usage.InputTokens < 0 || usage.OutputTokens < 0 {
		if req.Stream && streamWriter != nil {
			streamWriter.Close()
			streamWriter.Wait()
		}
		return InternalServerErrorCustomMsg(c, res, "Invalid token usage reported.")
	}
	// 8. DBトランザクション (Limit更新 & Stats更新)
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
			// CubeModelStat (Train) - MemoryGroup を必ず含める
			var ms model.CubeModelStat
			if err := tx.Where("cube_id = ? AND memory_group = ? AND model_name = ? AND action_type = ? AND apx_id = ? AND vdr_id = ?",
				cube.ID, req.MemoryGroup, modelName, types.ACTION_TYPE_ABSORB, *ids.ApxID, *ids.VdrID).
				FirstOrCreate(&ms, model.CubeModelStat{
					CubeID: cube.ID, MemoryGroup: req.MemoryGroup, ModelName: modelName, ActionType: string(types.ACTION_TYPE_ABSORB),
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
			// CubeContributor (Train) - MemoryGroup を必ず含める
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
		if req.Stream && streamWriter != nil {
			streamWriter.Close()
			streamWriter.Wait()
		}
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("DB update failed: %s", err.Error()))
	}
	// 9. レスポンス作成
	data := rtres.AbsorbCubeResData{
		InputTokens:  usage.InputTokens,
		OutputTokens: usage.OutputTokens,
		AbsorbLimit:  perm.AbsorbLimit,
	}
	// To return new limit, need to update local perm or use nextLimit
	if shouldUpdateLimit {
		data.AbsorbLimit = nextLimit
	}
	if req.Stream {
		// 最終結果を自然言語で送信
		summary := ""
		if isEn {
			summary = fmt.Sprintf(
				"\n\nKnowledge absorption and memorization completed successfully. Input tokens used: `%d`, Output tokens used: `%d`. %s",
				data.InputTokens,
				data.OutputTokens,
				common.TOpe(data.AbsorbLimit == 0, "", fmt.Sprintf("You can absorb knowledge into this Cube %d more time(s).", data.AbsorbLimit)),
			)
		} else {
			summary = fmt.Sprintf(
				"\n\n知識の吸収と記憶が完了しました。使用した入力トークンは `%d` 、出力トークンは `%d` です。%s",
				data.InputTokens,
				data.OutputTokens,
				common.TOpe(data.AbsorbLimit == 0, "", fmt.Sprintf("このCubeに対しては、あと%d回の知識吸収が可能です。", data.AbsorbLimit)),
			)
		}
		tokens := rtstream.Tokenize(summary, TOKEN_SIZE)
		for _, token := range tokens {
			streamWriter.Write(token)
		}
		streamWriter.Close()
		streamWriter.Wait() // 全てのトークンが送信されるまで待機
		return true         // ストリームは既に送信完了
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
	perm, err := common.ParseDatatypesJson[model.CubePermissions](&cube.Permissions)
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
	// 4. Create embedding config JSON
	embConfig := types.EmbeddingModelConfig{
		Provider:  cube.EmbeddingProvider,
		Model:     cube.EmbeddingModel,
		Dimension: cube.EmbeddingDimension,
		BaseURL:   cube.EmbeddingBaseURL,
	}
	embeddingConfigJSON, err := common.ToJson(embConfig)
	if err != nil {
		InternalServerErrorCustomMsg(c, res, "Failed to serialize embedding config.")
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
		EMBEDDING_CONFIG_JSON:   []byte(embeddingConfigJSON),
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
		txPerm, err := common.ParseDatatypesJson[model.CubePermissions](&txCube.Permissions)
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
	parentPermissions, err := common.ParseDatatypesJson[model.CubePermissions](&sourceCube.Permissions)
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
	fmt.Println(common.ToJsonDirect(parentPermissions))
	if err := CheckInheritance(parentPermissions, req.Permissions, sourceCube.ExpireAt, reqExpire); err != nil {
		return BadRequestCustomMsg(c, res, fmt.Sprintf("Cube permissions inheritance error: %s", err.Error()))
	}
	// 7. Limit Check（事前チェック）
	if parentPermissions.GenKeyLimit < 0 {
		return ForbiddenCustomMsg(c, res, "GenKey limit exceeded.")
	}
	// 8. Construct Key Payload（Tx前に準備）
	type KeyPayload struct {
		AESKey      string                `json:"aes_key"`
		Permissions model.CubePermissions `json:"permissions"`
		ExpireAt    *time.Time            `json:"expire_at"`
		ExportID    uint                  `json:"export_id"`
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
		txPerm, err := common.ParseDatatypesJson[model.CubePermissions](&txCube.Permissions)
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
		AESKey      string                `json:"aes_key"`
		Permissions model.CubePermissions `json:"permissions"`
		ExpireAt    *time.Time            `json:"expire_at"`
		ExportID    uint                  `json:"export_id"`
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
		if after, ok := strings.CutPrefix(zf.Name, "db/"); ok {
			// KuzuDB files handling (Single File Mode)
			// db/OLD_UUID.db -> Write to cubeDbFilePath (.../NEW_UUID.db)
			// KuzuDBは単一ファイル構成であるため、サブディレクトリを含むエントリは除外
			if strings.Contains(after, "/") {
				continue
			}
			targetPath := cubeDbFilePath
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
	// Extract Stats Usage
	statsUsageBytes, _ := func() ([]byte, error) {
		for _, zf := range innerZipReader.File {
			if zf.Name == STATS_USAGE_JSON {
				rc, err := zf.Open()
				if err != nil {
					return nil, err
				}
				defer rc.Close()
				return io.ReadAll(rc)
			}
		}
		return nil, nil // stats may not exist
	}()
	var importedStats []model.CubeModelStat
	if len(statsUsageBytes) > 0 {
		err = json.Unmarshal(statsUsageBytes, &importedStats)
		if err != nil {
			return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to unmarshal stats usage: %s", err.Error()))
		}
	}
	// Extract Stats Contributors
	statsContribBytes, _ := func() ([]byte, error) {
		for _, zf := range innerZipReader.File {
			if zf.Name == STATS_CONTRIBUTORS_JSON {
				rc, err := zf.Open()
				if err != nil {
					return nil, err
				}
				defer rc.Close()
				return io.ReadAll(rc)
			}
		}
		return nil, nil // stats may not exist
	}()
	var importedContributors []model.CubeContributor
	if len(statsContribBytes) > 0 {
		err = json.Unmarshal(statsContribBytes, &importedContributors)
		if err != nil {
			return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to unmarshal stats contributors: %s", err.Error()))
		}
	}
	// Extract Embedding Config
	embeddingConfigBytes, _ := func() ([]byte, error) {
		for _, zf := range innerZipReader.File {
			if zf.Name == EMBEDDING_CONFIG_JSON {
				rc, err := zf.Open()
				if err != nil {
					return nil, err
				}
				defer rc.Close()
				return io.ReadAll(rc)
			}
		}
		return nil, nil
	}()
	if len(embeddingConfigBytes) == 0 {
		return BadRequestCustomMsg(c, res, fmt.Sprintf("Missing %s: strictly required for import.", EMBEDDING_CONFIG_JSON))
	}
	var importedEmbConfig types.EmbeddingModelConfig
	if err := json.Unmarshal(embeddingConfigBytes, &importedEmbConfig); err != nil {
		return BadRequestCustomMsg(c, res, fmt.Sprintf("Invalid embedding config JSON: %s", err.Error()))
	}
	// 9. Live Test Embedding (Strict Verification for Import)
	// Uses the API Key from request and BaseURL from imported config (Strict)
	verConfig := types.EmbeddingModelConfig{
		Provider:  importedEmbConfig.Provider,
		Model:     importedEmbConfig.Model,
		Dimension: importedEmbConfig.Dimension,
		BaseURL:   importedEmbConfig.BaseURL,
		ApiKey:    req.EmbeddingApiKey,
	}
	if err := u.CuberService.VerifyEmbeddingConfiguration(c.Request.Context(), verConfig); err != nil {
		return BadRequestCustomMsg(c, res, fmt.Sprintf("Live embedding verification failed: %s", err.Error()))
	}
	// 10. Transaction: Cube作成 + Lineage作成
	// Encrypt API Key
	encryptedEmbeddingApiKey, err := mycrypto.Encrypt(req.EmbeddingApiKey, u.CuberCryptoSkey)
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to encrypt embedding API key: %s", err.Error()))
	}
	permJSON, _ := common.ToJson(payload.Permissions)
	var newCube model.Cube
	txErr := u.DB.Transaction(func(tx *gorm.DB) error {
		newCube = model.Cube{
			UUID:               newUUID,
			UsrID:              *ids.UsrID,
			Name:               req.Name,
			Description:        req.Description,
			EmbeddingProvider:  importedEmbConfig.Provider,
			EmbeddingModel:     importedEmbConfig.Model,
			EmbeddingDimension: importedEmbConfig.Dimension,
			EmbeddingBaseURL:   importedEmbConfig.BaseURL, // Strictly use imported BaseURL
			EmbeddingApiKey:    encryptedEmbeddingApiKey,
			ExpireAt:           payload.ExpireAt,
			Permissions:        datatypes.JSON(permJSON),
			SourceExportID:     &payload.ExportID, // Link to source export for ReKey
			ApxID:              *ids.ApxID,
			VdrID:              *ids.VdrID,
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
		// Stats Usage作成
		for _, stat := range importedStats {
			statRecord := model.CubeModelStat{
				CubeID:       newCube.ID,
				MemoryGroup:  stat.MemoryGroup,
				ModelName:    stat.ModelName,
				ActionType:   stat.ActionType,
				InputTokens:  stat.InputTokens,
				OutputTokens: stat.OutputTokens,
				ApxID:        *ids.ApxID,
				VdrID:        *ids.VdrID,
			}
			if err := tx.Create(&statRecord).Error; err != nil {
				return err
			}
		}
		// Contributors作成
		for _, contrib := range importedContributors {
			contribRecord := model.CubeContributor{
				CubeID:          newCube.ID,
				MemoryGroup:     contrib.MemoryGroup,
				ContributorName: contrib.ContributorName,
				ModelName:       contrib.ModelName,
				InputTokens:     contrib.InputTokens,
				OutputTokens:    contrib.OutputTokens,
				ApxID:           *ids.ApxID,
				VdrID:           *ids.VdrID,
			}
			if err := tx.Create(&contribRecord).Error; err != nil {
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
		AESKey      string                `json:"aes_key"`
		Permissions model.CubePermissions `json:"permissions"`
		ExpireAt    *time.Time            `json:"expire_at"`
		ExportID    uint                  `json:"export_id"`
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
	currentPerm, err := common.ParseDatatypesJson[model.CubePermissions](&cube.Permissions)
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, "Failed to parse permissions")
	}
	if currentPerm.RekeyLimit < 0 {
		return ForbiddenCustomMsg(c, res, fmt.Sprintf("ReKey limit exceeded: %d", currentPerm.RekeyLimit))
	}

	// # MEMO
	//     - GenKey のタイミングで CheckInheritance() による「親を子は超えられない」という制限チェックを行なっているため、
	//     - ReKey のタイミングで再び行う必要はない。

	// 9. Transaction: Limit消費 + Cube更新
	newPermJSON, _ := common.ToJson(payload.Permissions)
	txErr := u.DB.Transaction(func(tx *gorm.DB) error {
		var txCube model.Cube
		if err := tx.Where("id = ?", cube.ID).First(&txCube).Error; err != nil {
			return err
		}
		txPerm, err := common.ParseDatatypesJson[model.CubePermissions](&txCube.Permissions)
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

// QueryCube はCubeにクエリを実行します。
func QueryCube(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.QueryCubeReq, res *rtres.QueryCubeRes) bool {
	ids := ju.IDs(!(ju.IsApx() || ju.IsFromKey()))
	// 1. Cubeの取得と権限チェック
	cube, err := getCube(u, req.CubeID, *ids.ApxID, *ids.VdrID)
	if err != nil {
		return NotFoundCustomMsg(c, res, "Cube not found.")
	}
	perm, err := common.ParseDatatypesJson[model.CubePermissions](&cube.Permissions)
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, "Failed to parse permissions.")
	}
	// 2. QueryLimit チェック
	if perm.QueryLimit < 0 {
		return ForbiddenCustomMsg(c, res, "Query limit exceeded.")
	}
	// 3. QueryTypeLimit ホワイトリストチェック
	queryType := req.Type
	if !types.IsValidQueryType(queryType) {
		return ForbiddenCustomMsg(c, res, fmt.Sprintf("Invalid query type: %d", queryType))
	}
	if len(perm.QueryTypeLimit) > 0 {
		allowed := slices.Contains(perm.QueryTypeLimit, queryType)
		if !allowed {
			return ForbiddenCustomMsg(c, res, fmt.Sprintf("Query type not allowed: %d", queryType))
		}
	}
	// 4. CuberService.Query() 呼び出し準備
	cubeDBFilePath, err := u.GetCubeDBFilePath(&cube.UUID, ids.ApxID, ids.VdrID, ids.UsrID)
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, "Failed to get cube path.")
	}
	// Decrypt API Key
	decryptedEmbeddingApiKey, err := mycrypto.Decrypt(cube.EmbeddingApiKey, u.CuberCryptoSkey)
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to decrypt embedding API key: %s", err.Error()))
	}
	// Fetch Chat Model Config
	chatConf, err := fetchChatModelConfig(u, req.ChatModelID, *ids.ApxID, *ids.VdrID)
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to fetch chat model: %s", err.Error()))
	}
	// 5. ストリーミング設定
	var streamWriter *rtstream.StreamWriter
	if req.Stream {
		c.Header("Content-Type", "text/event-stream")
		c.Header("Cache-Control", "no-cache")
		c.Header("Connection", "keep-alive")
		c.Header("X-Accel-Buffering", "no") // Nginx対策
		streamWriter = rtstream.NewStreamWriter(c.Request.Context(), MIN_STREAM_DELAY)
		requestUUID := common.GenUUID() // リクエスト単位で共通のID
		go func() {
			defer streamWriter.Done() // ゴルーチン終了時にDoneを呼び出す
			ticker := time.NewTicker(streamWriter.MinDelay())
			defer ticker.Stop()
			for {
				select {
				case token, ok := <-streamWriter.Ch():
					if !ok {
						fmt.Fprint(c.Writer, rtstream.CreateSSEChunk(*requestUUID, "cuber-query", "", true))
						c.Writer.Flush()
						return
					}
					chunk := rtstream.CreateSSEChunk(*requestUUID, "cuber-query", token, false)
					fmt.Fprint(c.Writer, chunk)
					c.Writer.Flush()
					<-ticker.C
				case <-c.Request.Context().Done():
					return
				}
			}
		}()
	}
	// 6. Query実行
	type QueryResult struct {
		Answer    *string
		Chunks    *string
		Summaries *string
		Graph     *[]*storage.Triple
		Usage     types.TokenUsage
		Err       error
	}
	dataCh := make(chan event.StreamEvent)
	resultCh := make(chan QueryResult, 1)
	isEn := req.IsEn
	ctx, cancel := context.WithCancel(c.Request.Context())
	defer cancel()
	go func() {
		ans, chk, sum, grp, _, usg, e := u.CuberService.Query(ctx, u.EventBus, cubeDBFilePath, req.MemoryGroup, req.Text,
			types.QueryConfig{
				QueryType:   types.QueryType(queryType),
				SummaryTopk: req.SummaryTopk,
				ChunkTopk:   req.ChunkTopk,
				EntityTopk:  req.EntityTopk,
				FtsLayer:    types.FtsLayerType(req.FtsType).ToFtsLayer(),
				FtsTopk:     req.FtsTopk,
			},
			types.EmbeddingModelConfig{
				Provider:  cube.EmbeddingProvider,
				Model:     cube.EmbeddingModel,
				Dimension: cube.EmbeddingDimension,
				BaseURL:   cube.EmbeddingBaseURL,
				ApiKey:    decryptedEmbeddingApiKey,
			},
			chatConf,
			dataCh,
			isEn,
		)
		resultCh <- QueryResult{
			Answer:    ans,
			Chunks:    chk,
			Summaries: sum,
			Graph:     grp,
			Usage:     usg,
			Err:       e,
		}
	}()
	var (
		answer    *string
		chunks    *string
		summaries *string
		graph     *[]*storage.Triple
		usage     types.TokenUsage
	)
QueryLoop:
	for {
		select {
		case evt := <-dataCh:
			msg, fmtErr := event.FormatEvent(evt, isEn)
			if fmtErr == nil {
				if req.Stream {
					// トークン化してストリームに流す
					tokens := rtstream.Tokenize(msg, TOKEN_SIZE)
					for _, token := range tokens {
						streamWriter.Write(token)
					}
				} else {
					utils.LogInfo(u.Logger, "=================================")
					utils.LogInfo(u.Logger, fmt.Sprintf("%s: %s", evt.EventName, msg))
					utils.LogInfo(u.Logger, "=================================")
				}
			}
		case result := <-resultCh:
			answer = result.Answer
			chunks = result.Chunks
			summaries = result.Summaries
			graph = result.Graph
			usage = result.Usage
			err = result.Err
			break QueryLoop
		case <-ctx.Done():
			if req.Stream && streamWriter != nil {
				streamWriter.Close()
				streamWriter.Wait()
			}
			return InternalServerErrorCustomMsg(c, res, "Request cancelled")
		}
	}
	// 7. エラーチェック
	if err != nil {
		if req.Stream && streamWriter != nil {
			errorMsg := fmt.Sprintf("\nError: Query failed - %s", err.Error())
			tokens := rtstream.Tokenize(errorMsg, TOKEN_SIZE)
			for _, token := range tokens {
				streamWriter.Write(token)
			}
			streamWriter.Close()
			streamWriter.Wait()
		}
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Query failed: %s", err.Error()))
	}
	// 8. トークン使用量の厳格チェック
	if usage.InputTokens == 0 && usage.OutputTokens == 0 {
		if req.Stream && streamWriter != nil {
			streamWriter.Close()
			streamWriter.Wait()
		}
		return InternalServerErrorCustomMsg(c, res, "Token accounting failed: no tokens recorded.")
	}
	// 9. DBトランザクションで Limit更新 + CubeModelStat 更新
	var newQueryLimit int
	txErr := u.DB.Transaction(func(tx *gorm.DB) error {
		var txCube model.Cube
		if err := tx.Where("id = ? AND apx_id = ? AND vdr_id = ?", cube.ID, cube.ApxID, cube.VdrID).First(&txCube).Error; err != nil {
			return err
		}
		txPerm, err := common.ParseDatatypesJson[model.CubePermissions](&txCube.Permissions)
		if err != nil {
			return err
		}
		// Limit更新
		if txPerm.QueryLimit > 0 {
			txPerm.QueryLimit--
			if txPerm.QueryLimit == 0 {
				txPerm.QueryLimit = -1 // 0は無制限を意味するので、-1に変更して禁止にする
			}
		}
		newQueryLimit = txPerm.QueryLimit
		newPermJSON, err := common.ToJson(txPerm)
		if err != nil {
			return fmt.Errorf("Failed to convert permissions to JSON: %s", err.Error())
		}
		txCube.Permissions = datatypes.JSON(newPermJSON)
		if err := tx.Save(&txCube).Error; err != nil {
			return fmt.Errorf("Failed to update cube: %s", err.Error())
		}
		// Stats Update (ActionType="query")
		for modelName, detail := range usage.Details {
			var ms model.CubeModelStat
			tx.Where("cube_id = ? AND memory_group = ? AND model_name = ? AND action_type = ? AND apx_id = ? AND vdr_id = ?",
				cube.ID, req.MemoryGroup, modelName, types.ACTION_TYPE_QUERY, cube.ApxID, cube.VdrID).
				FirstOrCreate(&ms, model.CubeModelStat{
					CubeID: cube.ID, MemoryGroup: req.MemoryGroup, ModelName: modelName, ActionType: string(types.ACTION_TYPE_QUERY),
					ApxID: cube.ApxID, VdrID: cube.VdrID,
				})
			ms.InputTokens += detail.InputTokens
			ms.OutputTokens += detail.OutputTokens
			if err := tx.Save(&ms).Error; err != nil {
				return err
			}
		}
		// CubeContributor は更新しない（Queryは利用であり貢献ではない）
		return nil
	})
	if txErr != nil {
		if req.Stream && streamWriter != nil {
			streamWriter.Close()
			streamWriter.Wait()
		}
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Transaction failed: %s", txErr.Error()))
	}
	// 10. レスポンス
	data := rtres.QueryCubeResData{
		Answer:       answer,
		Chunks:       chunks,
		Summaries:    summaries,
		Graph:        graph,
		InputTokens:  usage.InputTokens,
		OutputTokens: usage.OutputTokens,
		QueryLimit:   newQueryLimit,
	}
	if req.Stream {
		// 最終結果を自然言語で送信
		summary := ""
		if isEn {
			summary = fmt.Sprintf(
				"\n\nQuery completed successfully. Input tokens used: `%d`, Output tokens used: `%d`. %s",
				data.InputTokens,
				data.OutputTokens,
				common.TOpe(data.QueryLimit == 0, "", fmt.Sprintf("You can query this Cube %d more time(s).", data.QueryLimit)),
			)
		} else {
			summary = fmt.Sprintf(
				"\n\n問い合わせが完了しました。使用した入力トークンは `%d` 、出力トークンは `%d` です。%s",
				data.InputTokens,
				data.OutputTokens,
				common.TOpe(data.QueryLimit == 0, "", fmt.Sprintf("このCubeに対しては、あと%d回の問い合わせが可能です。", data.QueryLimit)),
			)
		}
		tokens := rtstream.Tokenize(summary, TOKEN_SIZE)
		for _, token := range tokens {
			streamWriter.Write(token)
		}
		streamWriter.Close()
		streamWriter.Wait() // 全てのトークンが送信されるまで待機
		return true         // ストリームは既に送信完了
	}
	return OK(c, &data, res)
}

// MemifyCube はCubeを自己強化します。
func MemifyCube(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.MemifyCubeReq, res *rtres.MemifyCubeRes) bool {
	ids := ju.IDs(!(ju.IsApx() || ju.IsFromKey()))
	// 1. Cubeの取得と権限チェック
	cube, err := getCube(u, req.CubeID, *ids.ApxID, *ids.VdrID)
	if err != nil {
		return NotFoundCustomMsg(c, res, "Cube not found.")
	}
	perm, err := common.ParseDatatypesJson[model.CubePermissions](&cube.Permissions)
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to parse permissions: %s", err.Error()))
	}
	// 2. MemifyLimit チェック
	if perm.MemifyLimit < 0 {
		return ForbiddenCustomMsg(c, res, "Memify limit exceeded.")
	}
	// 3. MemifyConfigLimit チェック（epochs等）
	epochs := req.Epochs
	if epochs == 0 {
		epochs = 1
	}
	if maxEpochs, ok := perm.MemifyConfigLimit["max_epochs"]; ok {
		if maxE, ok := maxEpochs.(float64); ok && epochs > int(maxE) {
			return ForbiddenCustomMsg(c, res, fmt.Sprintf("Epochs exceeds limit (%d).", int(maxE)))
		}
	}
	// 4. CuberService.Memify() 呼び出し
	cubeDBFilePath, err := u.GetCubeDBFilePath(&cube.UUID, ids.ApxID, ids.VdrID, ids.UsrID)
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to get cube path: %s", err.Error()))
	}
	contributorName, err := getJwtUsrName(u, ids.ApxID, ids.VdrID, ids.UsrID)
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to get contributor name: %s", err.Error()))
	}
	// Decrypt API Key
	decryptedEmbeddingApiKey, err := mycrypto.Decrypt(cube.EmbeddingApiKey, u.CuberCryptoSkey)
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to decrypt embedding API key: %s", err.Error()))
	}
	// Fetch Chat Model Config
	chatConf, err := fetchChatModelConfig(u, req.ChatModelID, *ids.ApxID, *ids.VdrID)
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to fetch chat model: %s", err.Error()))
	}
	// 5. ストリーミング設定
	var streamWriter *rtstream.StreamWriter
	if req.Stream {
		c.Header("Content-Type", "text/event-stream")
		c.Header("Cache-Control", "no-cache")
		c.Header("Connection", "keep-alive")
		c.Header("X-Accel-Buffering", "no") // Nginx対策
		streamWriter = rtstream.NewStreamWriter(c.Request.Context(), MIN_STREAM_DELAY)
		requestUUID := common.GenUUID() // リクエスト単位で共通のID
		go func() {
			defer streamWriter.Done() // ゴルーチン終了時にDoneを呼び出す
			ticker := time.NewTicker(streamWriter.MinDelay())
			defer ticker.Stop()
			for {
				select {
				case token, ok := <-streamWriter.Ch():
					if !ok {
						fmt.Fprint(c.Writer, rtstream.CreateSSEChunk(*requestUUID, "cuber-memify", "", true))
						c.Writer.Flush()
						return
					}
					chunk := rtstream.CreateSSEChunk(*requestUUID, "cuber-memify", token, false)
					fmt.Fprint(c.Writer, chunk)
					c.Writer.Flush()
					<-ticker.C
				case <-c.Request.Context().Done():
					return
				}
			}
		}()
	}
	// 6. Memify実行
	type MemifyResult struct {
		Usage types.TokenUsage
		Err   error
	}
	dataCh := make(chan event.StreamEvent)
	resultCh := make(chan MemifyResult, 1)
	isEn := req.IsEn
	ctx, cancel := context.WithCancel(c.Request.Context())
	defer cancel()
	go func() {
		u, e := u.CuberService.Memify(ctx, u.EventBus, cubeDBFilePath, req.MemoryGroup,
			&types.MemifyConfig{
				RecursiveDepth:     epochs - 1, // epochs=1 means depth=0
				PrioritizeUnknowns: req.PrioritizeUnknowns,
			},
			types.EmbeddingModelConfig{
				Provider:  cube.EmbeddingProvider,
				Model:     cube.EmbeddingModel,
				Dimension: cube.EmbeddingDimension,
				BaseURL:   cube.EmbeddingBaseURL,
				ApiKey:    decryptedEmbeddingApiKey,
			},
			chatConf,
			dataCh,
			isEn,
		)
		resultCh <- MemifyResult{Usage: u, Err: e}
	}()
	var usage types.TokenUsage
MemifyLoop:
	for {
		select {
		case evt := <-dataCh:
			msg, fmtErr := event.FormatEvent(evt, isEn)
			if fmtErr == nil {
				if req.Stream {
					// トークン化して進捗をストリームに流す
					tokens := rtstream.Tokenize(msg, TOKEN_SIZE)
					for _, token := range tokens {
						streamWriter.Write(token)
					}
				} else {
					utils.LogInfo(u.Logger, "=================================")
					utils.LogInfo(u.Logger, fmt.Sprintf("%s: %s", evt.EventName, msg))
					utils.LogInfo(u.Logger, "=================================")
				}
			}
		case result := <-resultCh:
			usage = result.Usage
			err = result.Err
			break MemifyLoop
		case <-ctx.Done():
			if req.Stream && streamWriter != nil {
				streamWriter.Close()
				streamWriter.Wait()
			}
			return InternalServerErrorCustomMsg(c, res, "Request cancelled")
		}
	}
	// 7. エラーチェック
	if err != nil {
		if req.Stream && streamWriter != nil {
			errorMsg := fmt.Sprintf("\nError: Memify failed - %s", err.Error())
			tokens := rtstream.Tokenize(errorMsg, TOKEN_SIZE)
			for _, token := range tokens {
				streamWriter.Write(token)
			}
			streamWriter.Close()
			streamWriter.Wait()
		}
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Memify failed: %s", err.Error()))
	}
	// 8. トークン使用量の厳格チェック
	if usage.InputTokens == 0 && usage.OutputTokens == 0 {
		if req.Stream && streamWriter != nil {
			streamWriter.Close()
			streamWriter.Wait()
		}
		return InternalServerErrorCustomMsg(c, res, "Token accounting failed: no tokens recorded.")
	}
	// 9. DBトランザクションで Limit更新 + CubeModelStat + CubeContributor 更新
	var newMemifyLimit int
	txErr := u.DB.Transaction(func(tx *gorm.DB) error {
		var txCube model.Cube
		if err := tx.Where("id = ? AND apx_id = ? AND vdr_id = ?", cube.ID, cube.ApxID, cube.VdrID).First(&txCube).Error; err != nil {
			return err
		}
		txPerm, err := common.ParseDatatypesJson[model.CubePermissions](&txCube.Permissions)
		if err != nil {
			return err
		}
		// Limit更新
		if txPerm.MemifyLimit > 0 {
			txPerm.MemifyLimit--
			if txPerm.MemifyLimit == 0 {
				txPerm.MemifyLimit = -1 // 0は無制限を意味するので、-1に変更して禁止にする
			}
		}
		newMemifyLimit = txPerm.MemifyLimit
		newPermJSON, err := common.ToJson(txPerm)
		if err != nil {
			return err
		}
		txCube.Permissions = datatypes.JSON(newPermJSON)
		if err := tx.Save(&txCube).Error; err != nil {
			return err
		}
		// Stats Update (ActionType="memify")
		for modelName, detail := range usage.Details {
			var ms model.CubeModelStat
			tx.Where("cube_id = ? AND memory_group = ? AND model_name = ? AND action_type = ? AND apx_id = ? AND vdr_id = ?",
				cube.ID, req.MemoryGroup, modelName, types.ACTION_TYPE_MEMIFY, cube.ApxID, cube.VdrID).
				FirstOrCreate(&ms, model.CubeModelStat{
					CubeID: cube.ID, MemoryGroup: req.MemoryGroup, ModelName: modelName, ActionType: string(types.ACTION_TYPE_MEMIFY),
					ApxID: cube.ApxID, VdrID: cube.VdrID,
				})
			ms.InputTokens += detail.InputTokens
			ms.OutputTokens += detail.OutputTokens
			if err := tx.Save(&ms).Error; err != nil {
				return err
			}
			// CubeContributor 更新
			var cc model.CubeContributor
			tx.Where("cube_id = ? AND memory_group = ? AND contributor_name = ? AND model_name = ? AND apx_id = ? AND vdr_id = ?",
				cube.ID, req.MemoryGroup, contributorName, modelName, cube.ApxID, cube.VdrID).
				FirstOrCreate(&cc, model.CubeContributor{
					CubeID: cube.ID, MemoryGroup: req.MemoryGroup, ContributorName: contributorName, ModelName: modelName,
					ApxID: cube.ApxID, VdrID: cube.VdrID,
				})
			cc.InputTokens += detail.InputTokens
			cc.OutputTokens += detail.OutputTokens
			if err := tx.Save(&cc).Error; err != nil {
				return err
			}
		}
		return nil
	})
	if txErr != nil {
		if req.Stream && streamWriter != nil {
			streamWriter.Close()
			streamWriter.Wait()
		}
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Transaction failed: %s", txErr.Error()))
	}
	// 10. レスポンス
	data := rtres.MemifyCubeResData{
		InputTokens:  usage.InputTokens,
		OutputTokens: usage.OutputTokens,
		MemifyLimit:  newMemifyLimit,
	}
	if req.Stream {
		// 最終結果を自然言語で送信
		summary := ""
		if isEn {
			summary = fmt.Sprintf(
				"\n\nSelf-reinforcement of knowledge and memory through traversal of the knowledge network space and self-questioning completed successfully. Input tokens used: `%d`, Output tokens used: `%d`. %s",
				data.InputTokens,
				data.OutputTokens,
				common.TOpe(data.MemifyLimit == 0, "", fmt.Sprintf("You can perform self-reinforcement on this Cube %d more time(s).", data.MemifyLimit)),
			)
		} else {
			summary = fmt.Sprintf(
				"\n\n知識ネットワーク空間の回遊及び自問自答による知識及び記憶の自己強化が完了しました。使用した入力トークンは `%d` 、出力トークンは `%d` です。%s",
				data.InputTokens,
				data.OutputTokens,
				common.TOpe(data.MemifyLimit == 0, "", fmt.Sprintf("このCubeに対しては、あと%d回の自己強化が可能です。", data.MemifyLimit)),
			)
		}
		tokens := rtstream.Tokenize(summary, TOKEN_SIZE)
		for _, token := range tokens {
			streamWriter.Write(token)
		}
		streamWriter.Close()
		streamWriter.Wait() // 全てのトークンが送信されるまで待機
		return true         // ストリームは既に送信完了
	}
	return OK(c, &data, res)
}

// DeleteCube はCubeを削除します。
func DeleteCube(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.DeleteCubeReq, res *rtres.DeleteCubeRes) bool {
	ids := ju.IDs(!(ju.IsApx() || ju.IsFromKey()))
	// 1. Cubeの取得と所有者チェック
	cube, err := getCube(u, req.CubeID, *ids.ApxID, *ids.VdrID)
	if err != nil {
		return NotFoundCustomMsg(c, res, "Cube not found.")
	}
	// 所有者チェック
	if cube.UsrID != *ids.UsrID {
		return ForbiddenCustomMsg(c, res, "Only the owner can delete the cube.")
	}
	cubeDBFilePath, err := u.GetCubeDBFilePath(&cube.UUID, ids.ApxID, ids.VdrID, ids.UsrID)
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to get cube path: %s", err.Error()))
	}
	// 2. DBトランザクションで関連データ削除
	txErr := u.DB.Transaction(func(tx *gorm.DB) error {
		// CubeModelStat 削除
		if err := tx.Where("cube_id = ? AND apx_id = ? AND vdr_id = ?", cube.ID, cube.ApxID, cube.VdrID).Delete(&model.CubeModelStat{}).Error; err != nil {
			return err
		}
		// CubeContributor 削除
		if err := tx.Where("cube_id = ? AND apx_id = ? AND vdr_id = ?", cube.ID, cube.ApxID, cube.VdrID).Delete(&model.CubeContributor{}).Error; err != nil {
			return err
		}
		// CubeLineage 削除
		if err := tx.Where("cube_id = ? AND apx_id = ? AND vdr_id = ?", cube.ID, cube.ApxID, cube.VdrID).Delete(&model.CubeLineage{}).Error; err != nil {
			return err
		}
		// Export 削除
		if err := tx.Where("cube_id = ? AND apx_id = ? AND vdr_id = ?", cube.ID, cube.ApxID, cube.VdrID).Delete(&model.Export{}).Error; err != nil {
			return err
		}
		// Cube 削除
		if err := tx.Delete(&cube).Error; err != nil {
			return err
		}
		return nil
	})
	if txErr != nil {
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Transaction failed: %s", txErr.Error()))
	}
	// 3. 物理ファイル（KuzuDBファイル）削除（トランザクション成功後に削除）
	// cubeDBFilePath のデータベースファイルの存在を確認して、存在していたら削除
	if _, err := os.Stat(cubeDBFilePath); err == nil {
		if err := os.Remove(cubeDBFilePath); err != nil {
			return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to delete cube db file: %s", err.Error()))
		}
	} else {
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("cubeDBFilePath not found: %s", err.Error()))
	}
	return OK[rtres.DeleteCubeRes](c, nil, res)
}

// CheckInheritance は親子間の権限継承ルールを検証します。
func CheckInheritance(parent model.CubePermissions, child model.CubePermissions, pExpire, cExpire *time.Time) error {
	// 1. 禁止であるはずの機能や制限が子の時点で復活してしまっていないかチェック
	//     - 親が禁止(-1)なら、子も禁止(-1)でなければならない
	//     - 親が false なら、子も false でなければならない
	if parent.ExportLimit < 0 && child.ExportLimit >= 0 { // 親が禁止(-1)なら、子も禁止(-1)でなければならない
		return fmt.Errorf("ExportLimit: Cannot enable export (parent forbidden).")
	} else if parent.ExportLimit > 0 && child.ExportLimit > parent.ExportLimit { // 親が正数(回数制限)なら、子はその制限以下でなければならない
		return fmt.Errorf("ExportLimit: Cannot enable export (parent limit exceeded, value = %d).", parent.ExportLimit)
	}
	if parent.RekeyLimit < 0 && child.RekeyLimit >= 0 { // 親が禁止(-1)なら、子も禁止(-1)でなければならない
		return fmt.Errorf("RekeyLimit: Cannot enable rekey (parent forbidden).")
	} else if parent.RekeyLimit > 0 && child.RekeyLimit > parent.RekeyLimit { // 親が正数(回数制限)なら、子はその制限以下でなければならない
		return fmt.Errorf("RekeyLimit: Cannot enable rekey (parent limit exceeded, value = %d).", parent.RekeyLimit)
	}
	if parent.GenKeyLimit < 0 && child.GenKeyLimit >= 0 { // 親が禁止(-1)なら、子も禁止(-1)でなければならない
		return fmt.Errorf("GenKeyLimit: Cannot enable genkey (parent forbidden).")
	} else if parent.GenKeyLimit > 0 && child.GenKeyLimit > parent.GenKeyLimit { // 親が正数(回数制限)なら、子はその制限以下でなければならない
		return fmt.Errorf("GenKeyLimit: Cannot enable genkey (parent limit exceeded, value = %d).", parent.GenKeyLimit)
	}
	if parent.AbsorbLimit < 0 && child.AbsorbLimit >= 0 { // 親が禁止(-1)なら、子も禁止(-1)でなければならない
		return fmt.Errorf("AbsorbLimit: Cannot enable absorb (parent forbidden).")
	} else if parent.AbsorbLimit > 0 && child.AbsorbLimit > parent.AbsorbLimit { // 親が正数(回数制限)なら、子はその制限以下でなければならない
		return fmt.Errorf("AbsorbLimit: Cannot enable absorb (parent limit exceeded, value = %d).", parent.AbsorbLimit)
	}
	if parent.MemifyLimit < 0 && child.MemifyLimit >= 0 { // 親が禁止(-1)なら、子も禁止(-1)でなければならない
		return fmt.Errorf("MemifyLimit: Cannot enable memify (parent forbidden).")
	} else if parent.MemifyLimit > 0 && child.MemifyLimit > parent.MemifyLimit { // 親が正数(回数制限)なら、子はその制限以下でなければならない
		return fmt.Errorf("MemifyLimit: Cannot enable memify (parent limit exceeded, value = %d).", parent.MemifyLimit)
	}
	if parent.QueryLimit < 0 && child.QueryLimit >= 0 { // 親が禁止(-1)なら、子も禁止(-1)でなければならない
		return fmt.Errorf("QueryLimit: Cannot enable query (parent forbidden).")
	} else if parent.QueryLimit > 0 && child.QueryLimit > parent.QueryLimit { // 親が正数(回数制限)なら、子はその制限以下でなければならない
		return fmt.Errorf("QueryLimit: Cannot enable query (parent limit exceeded, value = %d).", parent.QueryLimit)
	}
	if !parent.AllowStats && child.AllowStats { // 親が禁止なら、子も禁止でなければならない
		return fmt.Errorf("AllowStats: Cannot enable stats (parent forbidden, value = %t).", parent.AllowStats)
	}
	// 2. Expire チェック
	// 親に期限がある場合、子はそれより前でなければならない
	if pExpire != nil {
		if cExpire == nil {
			return fmt.Errorf("Expire: Cannot remove expiration (parent has expire, value = %s).", common.ParseDatetimeToStr(pExpire))
		}
		if cExpire.After(*pExpire) {
			return fmt.Errorf("Expire: Cannot extend expiration beyond parent (value = %s).", common.ParseDatetimeToStr(pExpire))
		}
	}
	return nil
}

func fetchLineage(db *gorm.DB, cubeID, apxID, vdrID uint) ([]rtres.LineageRes, error) {
	var lineage []model.CubeLineage
	if err := db.Where("cube_id = ? AND apx_id = ? AND vdr_id = ?", cubeID, apxID, vdrID).Order("generation asc").Find(&lineage).Error; err != nil {
		return nil, err
	}
	res := make([]rtres.LineageRes, len(lineage))
	for i, l := range lineage {
		res[i] = rtres.LineageRes{
			UUID:          l.AncestorUUID,
			Owner:         l.AncestorOwner,
			ExportedAt:    l.ExportedAt,
			ExportedAtJST: common.UnixMilliToJSTStr(l.ExportedAt), // ms -> sec conversion handled inside if needed, assuming ms input
			Generation:    l.Generation,
		}
	}
	return res, nil
}

func fetchMemoryGroupStats(db *gorm.DB, cubeID, apxID, vdrID uint) ([]rtres.MemoryGroupStatsRes, error) {
	var modelStats []model.CubeModelStat
	var contribs []model.CubeContributor
	if err := db.Where("cube_id = ? AND apx_id = ? AND vdr_id = ?", cubeID, apxID, vdrID).Find(&modelStats).Error; err != nil {
		return nil, err
	}
	if err := db.Where("cube_id = ? AND apx_id = ? AND vdr_id = ?", cubeID, apxID, vdrID).Find(&contribs).Error; err != nil {
		return nil, err
	}
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
	var result []rtres.MemoryGroupStatsRes
	for _, v := range mgMap {
		result = append(result, *v)
	}
	return result, nil
}
