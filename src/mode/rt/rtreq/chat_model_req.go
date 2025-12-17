package rtreq

import (
	"fmt"

	"github.com/gin-gonic/gin"
	"github.com/t-kawata/mycute/lib/common"
	"github.com/t-kawata/mycute/lib/mycrypto"
	"github.com/t-kawata/mycute/mode/rt/rtres"
	"github.com/t-kawata/mycute/mode/rt/rtutil"
	"github.com/t-kawata/mycute/model"
	"github.com/t-kawata/mycute/pkg/cuber/types"
)

type SearchChatModelsReq struct {
	Name     string `json:"name" binding:"max=50"`
	Provider string `json:"provider" binding:""`
	Model    string `json:"model" binding:""`
	BaseURL  string `json:"base_url" binding:""`
}

func SearchChatModelsReqBind(c *gin.Context, u *rtutil.RtUtil) (SearchChatModelsReq, rtres.SearchChatModelsRes, bool) {
	ok := true
	req := SearchChatModelsReq{}
	res := rtres.SearchChatModelsRes{Errors: []rtres.Err{}}
	if err := c.ShouldBindJSON(&req); err != nil {
		res.Errors = u.GetValidationErrs(err)
		ok = false
	}
	return req, res, ok
}

type GetChatModelReq struct {
	ID uint `binding:"gte=1"` // Path Paramなのでjsonタグ不要
}

func GetChatModelReqBind(c *gin.Context, u *rtutil.RtUtil) (GetChatModelReq, rtres.GetChatModelRes, bool) {
	ok := true
	req := GetChatModelReq{ID: common.StrToUint(c.Param("chat_model_id"))}
	res := rtres.GetChatModelRes{Errors: []rtres.Err{}}
	if err := c.ShouldBind(&req); err != nil {
		res.Errors = u.GetValidationErrs(err)
		ok = false
	}
	return req, res, ok
}

type CreateChatModelReq struct {
	Name        string  `json:"name" binding:"required,max=50"`
	Provider    string  `json:"provider" binding:"required,max=50"`
	Model       string  `json:"model" binding:"required,max=100"`
	BaseURL     string  `json:"base_url" binding:"max=255"`
	ApiKey      string  `json:"api_key" binding:"required,max=1024"`
	MaxTokens   int     `json:"max_tokens" binding:"required,gte=1"`
	Temperature float64 `json:"temperature" binding:"required,gte=0,lte=1"`
}

func CreateChatModelReqBind(c *gin.Context, u *rtutil.RtUtil) (CreateChatModelReq, rtres.CreateChatModelRes, bool) {
	ok := true
	req := CreateChatModelReq{}
	res := rtres.CreateChatModelRes{Errors: []rtres.Err{}}
	if err := c.ShouldBindJSON(&req); err != nil {
		res.Errors = u.GetValidationErrs(err)
		ok = false
	}
	if ok {
		// Live Test
		chatConfig := types.ChatModelConfig{
			Provider:    req.Provider,
			Model:       req.Model,
			BaseURL:     req.BaseURL,
			ApiKey:      req.ApiKey,
			MaxTokens:   req.MaxTokens,
			Temperature: &req.Temperature,
		}
		if err := u.CuberService.VerifyChatModelConfiguration(c.Request.Context(), chatConfig); err != nil {
			res.Errors = append(res.Errors, rtres.Err{Field: "api_key", Message: fmt.Sprintf("Live verification failed: %s", err.Error())})
			ok = false
		}
	}
	return req, res, ok
}

type UpdateChatModelReq struct {
	ID          uint     `json:"-" binding:"gte=1"` // Path Param -> Internal
	Name        string   `json:"name" binding:"max=50"`
	Provider    string   `json:"provider" binding:"max=50"`
	Model       string   `json:"model" binding:"max=100"`
	BaseURL     string   `json:"base_url" binding:"max=255"`
	ApiKey      string   `json:"api_key" binding:"max=1024"` // Optional update
	MaxTokens   int      `json:"max_tokens" binding:"min=0"`
	Temperature *float64 `json:"temperature" binding:"omitempty,min=0,max=2"`
}

func UpdateChatModelReqBind(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr) (UpdateChatModelReq, rtres.UpdateChatModelRes, bool) {
	ok := true
	req := UpdateChatModelReq{ID: common.StrToUint(c.Param("chat_model_id"))}
	res := rtres.UpdateChatModelRes{Errors: []rtres.Err{}}
	if err := c.ShouldBindJSON(&req); err != nil {
		res.Errors = u.GetValidationErrs(err)
		ok = false
	}
	if ok {
		// Live Test for Update
		needsLiveTest := req.Provider != "" || req.Model != "" || req.BaseURL != "" || req.ApiKey != ""
		if needsLiveTest {
			ids := ju.IDs(!(ju.IsApx() || ju.IsFromKey()))
			var currentModel model.ChatModel
			// IDはPath Parameterから取得済み (req.ID)
			if err := u.DB.Where("id = ? AND apx_id = ? AND vdr_id = ?", req.ID, ids.ApxID, ids.VdrID).First(&currentModel).Error; err != nil {
				res.Errors = append(res.Errors, rtres.Err{Field: "id", Message: "Chat model not found for verification."})
				return req, res, false
			}
			// Config構築 (Request優先, 空ならExisting)
			targetProvider := currentModel.Provider
			if req.Provider != "" {
				targetProvider = req.Provider
			}
			targetModelName := currentModel.Model
			if req.Model != "" {
				targetModelName = req.Model
			}
			targetBaseURL := currentModel.BaseURL
			if req.BaseURL != "" {
				targetBaseURL = req.BaseURL
			}
			targetApiKey := ""
			if req.ApiKey != "" {
				targetApiKey = req.ApiKey
			} else {
				// 既存のKeyを復号して使う
				decrypted, err := mycrypto.Decrypt(currentModel.ApiKey, u.CuberCryptoSkey)
				if err != nil {
					res.Errors = append(res.Errors, rtres.Err{Field: "api_key", Message: "Failed to decrypt existing API key for verification"})
					return req, res, false
				}
				targetApiKey = decrypted
			}
			// 検証用Config
			chatConfig := types.ChatModelConfig{
				Provider: targetProvider,
				Model:    targetModelName,
				BaseURL:  targetBaseURL,
				ApiKey:   targetApiKey,
				// MaxTokens/Temperatureは疎通確認には本質的に不要だが渡しておく
				MaxTokens:   currentModel.MaxTokens,
				Temperature: &currentModel.Temperature,
			}
			if err := u.CuberService.VerifyChatModelConfiguration(c.Request.Context(), chatConfig); err != nil {
				res.Errors = append(res.Errors, rtres.Err{Field: "configuration", Message: fmt.Sprintf("Live verification failed: %s", err.Error())})
				return req, res, false
			}
		}
	}
	return req, res, ok
}

type DeleteChatModelReq struct {
	ID uint `binding:"gte=1"`
}

func DeleteChatModelReqBind(c *gin.Context, u *rtutil.RtUtil) (DeleteChatModelReq, rtres.DeleteChatModelRes, bool) {
	ok := true
	req := DeleteChatModelReq{ID: common.StrToUint(c.Param("chat_model_id"))}
	res := rtres.DeleteChatModelRes{Errors: []rtres.Err{}}
	if err := c.ShouldBind(&req); err != nil {
		res.Errors = u.GetValidationErrs(err)
		ok = false
	}
	return req, res, ok
}
