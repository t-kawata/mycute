package rt

import (
	"net/http"

	"github.com/gin-gonic/gin"
	"github.com/t-kawata/mycute/config"
	"github.com/t-kawata/mycute/lib/httpclient"
	"github.com/t-kawata/mycute/lib/s3client"
	"github.com/t-kawata/mycute/mode/rt/rthandler/hv1"
	"github.com/t-kawata/mycute/mode/rt/rtmiddleware"
	"github.com/t-kawata/mycute/mode/rt/rtutil"
	"github.com/t-kawata/mycute/pkg/cuber"
	"go.uber.org/zap"
	"gorm.io/gorm"
)

func MapRequest(r *gin.Engine, l *zap.Logger, env *config.Env, hc *httpclient.HttpClient, hn *string, db *gorm.DB, sk *string, flgs *RTFlags, s3c *s3client.S3Client, cuberService *cuber.CuberService) {
	rtutil.RegisterValidations()

	/**********************
	 * v1 mapping
	 **********************/
	v1 := r.Group("/v1")
	v1.Use(rtmiddleware.AuthMiddleware(r, l, env, hc, hn, db, sk, s3c, &flgs.DBDirPath, cuberService))
	{

		// Key
		keys := v1.Group("/keys")
		keys.GET("/generate", func(c *gin.Context) {
			u, ju, ok := GetUtil(c)
			if !ok {
				c.JSON(http.StatusForbidden, nil)
				return
			}
			hv1.GenerateKeyHash(c, u, ju)
		})
		keys.POST("/check", func(c *gin.Context) {
			u, ju, ok := GetUtil(c)
			if !ok {
				c.JSON(http.StatusForbidden, nil)
				return
			}
			hv1.CheckKeyHash(c, u, ju)
		})

		// User
		usrs := v1.Group("/usrs")
		usrs.GET("/auth/:apx_id/:vdr_id", func(c *gin.Context) {
			u, ju, ok := GetUtil(c)
			if !ok {
				c.JSON(http.StatusForbidden, nil)
				return
			}
			hv1.AuthUsr(c, u, ju)
		})
		usrs.POST("/search", func(c *gin.Context) {
			u, ju, ok := GetUtil(c)
			if !ok {
				c.JSON(http.StatusForbidden, nil)
				return
			}
			hv1.SearchUsrs(c, u, ju)
		})
		usrs.GET("/:usr_id", func(c *gin.Context) {
			u, ju, ok := GetUtil(c)
			if !ok {
				c.JSON(http.StatusForbidden, nil)
				return
			}
			hv1.GetUsr(c, u, ju)
		})
		usrs.POST("/", func(c *gin.Context) {
			u, ju, ok := GetUtil(c)
			if !ok {
				c.JSON(http.StatusForbidden, nil)
				return
			}
			hv1.CreateUsr(c, u, ju)
		})
		usrs.PATCH("/:usr_id", func(c *gin.Context) {
			u, ju, ok := GetUtil(c)
			if !ok {
				c.JSON(http.StatusForbidden, nil)
				return
			}
			hv1.UpdateUsr(c, u, ju)
		})
		usrs.DELETE("/:usr_id", func(c *gin.Context) {
			u, ju, ok := GetUtil(c)
			if !ok {
				c.JSON(http.StatusForbidden, nil)
				return
			}
			hv1.DeleteUsr(c, u, ju)
		})
		usrs.PATCH("/:usr_id/hire", func(c *gin.Context) {
			u, ju, ok := GetUtil(c)
			if !ok {
				c.JSON(http.StatusForbidden, nil)
				return
			}
			hv1.HireUsr(c, u, ju)
		})
		usrs.DELETE("/:usr_id/hire", func(c *gin.Context) {
			u, ju, ok := GetUtil(c)
			if !ok {
				c.JSON(http.StatusForbidden, nil)
				return
			}
			hv1.DehireUsr(c, u, ju)
		})

		// Cubes
		cubes := v1.Group("/cubes")
		cubes.POST("/create", func(c *gin.Context) {
			u, ju, ok := GetUtil(c)
			if !ok {
				c.JSON(http.StatusForbidden, nil)
				return
			}
			hv1.CreateCube(c, u, ju)
		})
		cubes.PUT("/absorb", func(c *gin.Context) {
			u, ju, ok := GetUtil(c)
			if !ok {
				c.JSON(http.StatusForbidden, nil)
				return
			}
			hv1.AbsorbCube(c, u, ju)
		})
		cubes.GET("/stats", func(c *gin.Context) {
			u, ju, ok := GetUtil(c)
			if !ok {
				c.JSON(http.StatusForbidden, nil)
				return
			}
			hv1.StatsCube(c, u, ju)
		})
		cubes.GET("/export", func(c *gin.Context) {
			u, ju, ok := GetUtil(c)
			if !ok {
				c.JSON(http.StatusForbidden, nil)
				return
			}
			hv1.ExportCube(c, u, ju)
		})
		cubes.POST("/genkey", func(c *gin.Context) {
			u, ju, ok := GetUtil(c)
			if !ok {
				c.JSON(http.StatusForbidden, nil)
				return
			}
			hv1.GenKeyCube(c, u, ju)
		})
		cubes.POST("/import", func(c *gin.Context) {
			u, ju, ok := GetUtil(c)
			if !ok {
				c.JSON(http.StatusForbidden, nil)
				return
			}
			hv1.ImportCube(c, u, ju)
		})
		cubes.POST("/rekey", func(c *gin.Context) {
			u, ju, ok := GetUtil(c)
			if !ok {
				c.JSON(http.StatusForbidden, nil)
				return
			}
			hv1.ReKeyCube(c, u, ju)
		})
		cubes.GET("/query", func(c *gin.Context) {
			u, ju, ok := GetUtil(c)
			if !ok {
				c.JSON(http.StatusForbidden, nil)
				return
			}
			hv1.QueryCube(c, u, ju)
		})
		cubes.PUT("/memify", func(c *gin.Context) {
			u, ju, ok := GetUtil(c)
			if !ok {
				c.JSON(http.StatusForbidden, nil)
				return
			}
			hv1.MemifyCube(c, u, ju)
		})
		cubes.DELETE("/delete", func(c *gin.Context) {
			u, ju, ok := GetUtil(c)
			if !ok {
				c.JSON(http.StatusForbidden, nil)
				return
			}
			hv1.DeleteCube(c, u, ju)
		})

	}

}

func GetUtil(c *gin.Context) (*rtutil.RtUtil, *rtutil.JwtUsr, bool) {
	v, ok := c.Get(rtmiddleware.UTIL_KEY)
	if !ok {
		return nil, nil, false
	}
	u, ok := v.(*rtutil.RtUtil)
	if !ok {
		return nil, nil, false
	}
	v2, ok := c.Get(rtmiddleware.JWT_U_KEY)
	if !ok {
		return nil, nil, false
	}
	ju, ok := v2.(*rtutil.JwtUsr)
	if !ok {
		return nil, nil, false
	}
	return u, ju, true
}
