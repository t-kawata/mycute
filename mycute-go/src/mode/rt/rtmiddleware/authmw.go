package rtmiddleware

import (
	"net/http"
	"slices"
	"time"

	"github.com/gin-gonic/gin"
	"github.com/golang-jwt/jwt/v5"
	"github.com/t-kawata/mycute/config"
	"github.com/t-kawata/mycute/enum/rterr"
	"github.com/t-kawata/mycute/lib/eventbus"
	"github.com/t-kawata/mycute/lib/httpclient"
	"github.com/t-kawata/mycute/lib/s3client"
	"github.com/t-kawata/mycute/mode/rt/rtres"
	"github.com/t-kawata/mycute/mode/rt/rtutil"
	"github.com/t-kawata/mycute/pkg/cuber"
	"go.uber.org/zap"
	"gorm.io/gorm"
)

const UTIL_KEY = "RUTIL"

const JWT_U_KEY = "JWT_U"

func AuthMiddleware(r *gin.Engine, l *zap.Logger, env *config.Env, hc *httpclient.HttpClient, hn *string, db *gorm.DB, sk *string, cuberCryptoSkey *string, s3c *s3client.S3Client, dbDirPath *string, cuberService *cuber.CuberService) gin.HandlerFunc {
	authSkipTargets := []string{
		"/v1/keys/check",
		"/v1/keys/generate",
		"/v1/crypto/enc",
		"/v1/crypto/dec",
		"/v1/crypto/vdr/:key",
		"/v1/usrs/auth/:apx_id/:vdr_id",
	}
	return func(c *gin.Context) {
		u := initRequest(l, env, hc, hn, db, sk, cuberCryptoSkey, s3c, dbDirPath, cuberService)
		ju := &rtutil.JwtUsr{}
		fp := c.FullPath()
		if slices.Contains(authSkipTargets, fp) {
			c.Set(UTIL_KEY, u)
			c.Set(JWT_U_KEY, ju)
			c.Next()
			return
		}
		token := rtutil.GetToken(c)
		res := rtres.DummyRes{}
		if len(token) <= 100 || !rtutil.IsJwtFormat(token) {
			c.Set(UTIL_KEY, u)
			c.Set(JWT_U_KEY, ju)
			authFailed(c, &res)
			return
		}
		if t, err := rtutil.ParseToken(u.SKey, token); err == nil && t.Valid {
			if clames, ok := t.Claims.(jwt.MapClaims); ok {
				exp := clames["exp"].(float64)
				expt := time.Unix(int64(exp), 0)
				now := time.Now()
				if now.After(expt) {
					c.Set(UTIL_KEY, u)
					c.Set(JWT_U_KEY, ju)
					authFailed(c, &res)
					return
				}
				aid := clames["apx_id"].(float64)
				vid := clames["vdr_id"].(float64)
				uid := clames["usr_id"].(float64)
				email := clames["email"].(string)
				isStaff := clames["is_staff"].(bool)
				var (
					aID *uint = nil
					vID *uint = nil
					uID *uint = nil
				)
				if aid > 0 {
					ai := uint(aid)
					aID = &ai
				}
				if vid > 0 {
					vi := uint(vid)
					vID = &vi
				}
				if uid > 0 {
					ui := uint(uid)
					uID = &ui
				}
				if isStaff {
					ju = &rtutil.JwtUsr{ApxID: aID, VdrID: nil, UsrID: vID, Email: email, StaffID: uID, Exp: expt}
				} else {
					ju = &rtutil.JwtUsr{ApxID: aID, VdrID: vID, UsrID: uID, Email: email, StaffID: nil, Exp: expt}
				}
				c.Set(JWT_U_KEY, ju)
			}
		} else {
			c.Set(UTIL_KEY, u)
			c.Set(JWT_U_KEY, ju)
			authFailed(c, &res)
			return
		}
		c.Set(UTIL_KEY, u)
		c.Next()
	}
}

func authFailed(c *gin.Context, res *rtres.DummyRes) {
	res.Errors = []rtres.Err{{Field: "auth", Code: rterr.Unauthorized.Code(), Message: rterr.Unauthorized.Msg()}}
	c.JSON(http.StatusUnauthorized, res)
	c.Abort()
}

func initRequest(l *zap.Logger, env *config.Env, hc *httpclient.HttpClient, hn *string, db *gorm.DB, sk *string, cuberCryptoSkey *string, s3c *s3client.S3Client, dbDirPath *string, cuberService *cuber.CuberService) (u *rtutil.RtUtil) {
	u = &rtutil.RtUtil{
		Logger:          l,
		Env:             env,
		Client:          hc,
		Hostname:        hn,
		DB:              db,
		SKey:            *sk,
		CuberCryptoSkey: *cuberCryptoSkey,
		S3c:             s3c,
		DBDirPath:       dbDirPath,
		CuberService:    cuberService,
		EventBus:        eventbus.New(), // リクエスト単位でイベントを発行できるようにする
	}
	return
}
