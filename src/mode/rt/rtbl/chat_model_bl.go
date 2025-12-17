package rtbl

import (
	"fmt"

	"github.com/gin-gonic/gin"
	"github.com/t-kawata/mycute/lib/mycrypto"
	"github.com/t-kawata/mycute/mode/rt/rtreq"
	"github.com/t-kawata/mycute/mode/rt/rtres"
	"github.com/t-kawata/mycute/mode/rt/rtutil"
	"github.com/t-kawata/mycute/model"
)

func SearchChatModels(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.SearchChatModelsReq, res *rtres.SearchChatModelsRes) bool {
	ids := ju.IDs(!(ju.IsApx() || ju.IsFromKey()))
	var chatModels []model.ChatModel
	query := u.DB.Where("apx_id = ? AND vdr_id = ?", ids.ApxID, ids.VdrID)
	if req.Name != "" {
		query = query.Where("name LIKE ?", "%"+req.Name+"%")
	}
	if req.Provider != "" {
		query = query.Where("provider LIKE ?", "%"+req.Provider+"%")
	}
	if req.Model != "" {
		query = query.Where("model LIKE ?", "%"+req.Model+"%")
	}
	if req.BaseURL != "" {
		query = query.Where("base_url LIKE ?", "%"+req.BaseURL+"%")
	}
	if err := query.Find(&chatModels).Error; err != nil {
		return InternalServerErrorCustomMsg(c, res, err.Error())
	}
	return OK(c, new(rtres.SearchChatModelsResData).Of(&chatModels), res)
}

func GetChatModel(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.GetChatModelReq, res *rtres.GetChatModelRes) bool {
	ids := ju.IDs(!(ju.IsApx() || ju.IsFromKey())) // USRだけが使用可能なので
	var m model.ChatModel
	if err := u.DB.Where("id = ? AND apx_id = ? AND vdr_id = ?", req.ID, ids.ApxID, ids.VdrID).First(&m).Error; err != nil {
		return NotFoundCustomMsg(c, res, "Chat model not found")
	}
	return OK(c, new(rtres.GetChatModelResData).Of(&m), res)
}

func CreateChatModel(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.CreateChatModelReq, res *rtres.CreateChatModelRes) bool {
	ids := ju.IDs(!(ju.IsApx() || ju.IsFromKey())) // USRだけが使用可能なので
	// 暗号化
	encKey, err := mycrypto.Encrypt(req.ApiKey, u.CuberCryptoSkey)
	if err != nil {
		return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to encrypt API key: %s", err.Error()))
	}
	m := model.ChatModel{
		Name:        req.Name,
		Provider:    req.Provider,
		Model:       req.Model,
		BaseURL:     req.BaseURL,
		ApiKey:      encKey,
		MaxTokens:   req.MaxTokens,
		Temperature: req.Temperature,
		ApxID:       *ids.ApxID,
		VdrID:       *ids.VdrID,
	}
	if err := u.DB.Create(&m).Error; err != nil {
		return InternalServerErrorCustomMsg(c, res, err.Error())
	}
	data := rtres.CreateChatModelResData{ID: m.ID}
	return OK(c, &data, res)
}

func UpdateChatModel(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.UpdateChatModelReq, res *rtres.UpdateChatModelRes) bool {
	ids := ju.IDs(!(ju.IsApx() || ju.IsFromKey())) // USRだけが使用可能なので
	var m model.ChatModel
	if err := u.DB.Where("id = ? AND apx_id = ? AND vdr_id = ?", req.ID, ids.ApxID, ids.VdrID).First(&m).Error; err != nil {
		return NotFoundCustomMsg(c, res, "Chat model not found")
	}
	// Update fields if present
	if req.Name != "" {
		m.Name = req.Name
	}
	if req.Provider != "" {
		m.Provider = req.Provider
	}
	if req.Model != "" {
		m.Model = req.Model
	}
	if req.BaseURL != "" {
		m.BaseURL = req.BaseURL
	}
	if req.MaxTokens > 0 {
		m.MaxTokens = req.MaxTokens
	}
	// Temperature (Pointer check)
	if req.Temperature != nil {
		m.Temperature = *req.Temperature
	}
	if req.ApiKey != "" {
		encKey, err := mycrypto.Encrypt(req.ApiKey, u.CuberCryptoSkey)
		if err != nil {
			return InternalServerErrorCustomMsg(c, res, "Failed to encrypt API key")
		}
		m.ApiKey = encKey
	}
	if err := u.DB.Save(&m).Error; err != nil {
		return InternalServerErrorCustomMsg(c, res, err.Error())
	}
	return OK(c, &rtres.UpdateChatModelResData{}, res)
}

func DeleteChatModel(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.DeleteChatModelReq, res *rtres.DeleteChatModelRes) bool {
	ids := ju.IDs(!(ju.IsApx() || ju.IsFromKey())) // USRだけが使用可能なので
	// .Unscoped() で論理削除を解除して物理削除する
	result := u.DB.Where("id = ? AND apx_id = ? AND vdr_id = ?", req.ID, ids.ApxID, ids.VdrID).Unscoped().Delete(&model.ChatModel{})
	if result.Error != nil {
		return InternalServerErrorCustomMsg(c, res, result.Error.Error())
	}
	if result.RowsAffected == 0 {
		return NotFoundCustomMsg(c, res, "Chat model not found")
	}
	return OK(c, &rtres.DeleteChatModelResData{}, res)
}
