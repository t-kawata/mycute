package hv1

import (
	"net/http"

	"github.com/gin-gonic/gin"
	"github.com/t-kawata/mycute/mode/rt/rtparam"
	"github.com/t-kawata/mycute/mode/rt/rtres"
	"github.com/t-kawata/mycute/mode/rt/rtutil"
	"golang.org/x/crypto/bcrypt"
)

// @Tags v1 Key
// @Router /v1/keys/generate [get]
// @Summary 鍵を生成する。
// @Accept application/json
// @Success 200 {object} GenerateKeyHashRes{errors=[]int}
// @Param key query string true "key"
// @Failure 400 {object} ErrRes
// @Failure 401 {object} ErrRes
// @Failure 403 {object} ErrRes
// @Failure 404 {object} ErrRes
func GenerateKeyHash(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr) {
	hash := GetHash(c)
	if len(hash) == 0 {
		c.JSON(http.StatusBadRequest, rtres.EmptyObj{})
		return
	}
	res := rtres.GenerateKeyHashRes{Data: rtres.GenerateKeyHashResData{Hash: string(hash)}}
	c.JSON(http.StatusOK, res)
}

// @Tags v1 Key
// @Router /v1/keys/check [post]
// @Summary 鍵の一致を確認する。
// @Accept application/json
// @Param json body CheckKeyHashParam true "json"
// @Success 200 {object} CheckKeyHashRes{errors=[]int}
// @Failure 400 {object} ErrRes
// @Failure 401 {object} ErrRes
// @Failure 403 {object} ErrRes
// @Failure 404 {object} ErrRes
func CheckKeyHash(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr) {
	key := GetKey(c)
	res := rtres.CheckKeyHashRes{Data: rtres.CheckKeyHashResData{Result: u.IsValidKey(key)}}
	c.JSON(http.StatusOK, res)
}

func GetHash(c *gin.Context) string {
	key := c.Query("key")
	if len(key) == 0 {
		return ""
	}
	hash, err := bcrypt.GenerateFromPassword([]byte(key), bcrypt.DefaultCost)
	if err != nil {
		return ""
	}
	return string(hash)
}

func GetKey(c *gin.Context) string {
	var param rtparam.CheckKeyHashParam
	if err := c.ShouldBindJSON(&param); err != nil {
		return ""
	}
	return param.Key
}
