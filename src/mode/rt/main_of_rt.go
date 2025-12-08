package rt

import (
	"fmt"
	"log"
	"net/http"
	"os"
	"time"

	"github.com/gin-contrib/cors"
	"github.com/gin-gonic/gin"
	"github.com/google/uuid"
	"github.com/joho/godotenv"
	swaggerfiles "github.com/swaggo/files"
	ginSwagger "github.com/swaggo/gin-swagger"
	"github.com/t-kawata/mycute/config"
	"github.com/t-kawata/mycute/lib/common"
	"github.com/t-kawata/mycute/lib/s3client"

	_ "github.com/t-kawata/mycute/docs"
	"go.uber.org/zap"
	"go.uber.org/zap/zapcore"
	"gorm.io/gorm/logger"
)

type RTFlags struct {
	SKey                     string
	Dotenv                   string
	StorageUseLocal          bool
	StorageS3AccessKey       string
	StorageS3SecretAccessKey string
	StorageS3Region          string
	StorageS3Bucket          string
	MinFreeDisk              int32
	CorsOnAtRT               bool
}

func MainOfRT() {
	flgs := RTFlags{}
	_, cflgs, l, env, hc, err := common.Init("mycute rt mode", &[]common.Flag{
		{Dst: &flgs.SKey, Name: "s", Default: "", Doc: "Secret Key to generate and check jwt."},
		{Dst: &flgs.Dotenv, Name: "d", Default: ".env", Doc: "Settings dotenv file path."},
	})
	if err != nil {
		log.Fatalf("Error: %s", err.Error())
		return
	}
	err = godotenv.Load(flgs.Dotenv)
	if err != nil {
		log.Fatalf("Error loading env file: %s", err.Error())
		return
	}
	hn, err := os.Hostname()
	if err != nil {
		log.Fatalf("Failed to get hostname: %s", err.Error())
		return
	}
	l.Info(
		"Set RT flags: ",
		zap.String("e", cflgs.Env),
		zap.String("l", cflgs.LogLevel),
		zap.String("o", cflgs.Output),
		zap.String("s", flgs.SKey),
		zap.String("d", flgs.Dotenv),
	)
	defer l.Info("REST API server was closed.")
	CUBER_S3_USE_LOCAL := os.Getenv("CUBER_S3_USE_LOCAL")
	AWS_ACCESS_KEY_ID := os.Getenv("AWS_ACCESS_KEY_ID")
	AWS_SECRET_ACCESS_KEY := os.Getenv("AWS_SECRET_ACCESS_KEY")
	AWS_REGION := os.Getenv("AWS_REGION")
	S3_BUCKET := os.Getenv("S3_BUCKET")
	MIN_FREE_DISK := os.Getenv("MIN_FREE_DISK")
	CORS_ON_AT_RT := os.Getenv("CORS_ON_AT_RT")
	if len(CUBER_S3_USE_LOCAL) == 0 {
		l.Warn(fmt.Sprintf("Failed to read CUBER_S3_USE_LOCAL from env file (%s).", flgs.Dotenv))
		return
	}
	if len(AWS_ACCESS_KEY_ID) == 0 {
		l.Warn(fmt.Sprintf("Failed to read AWS_ACCESS_KEY_ID from env file (%s).", flgs.Dotenv))
		return
	}
	if len(AWS_SECRET_ACCESS_KEY) == 0 {
		l.Warn(fmt.Sprintf("Failed to read AWS_SECRET_ACCESS_KEY from env file (%s).", flgs.Dotenv))
		return
	}
	if len(AWS_REGION) == 0 {
		l.Warn(fmt.Sprintf("Failed to read AWS_REGION from env file (%s).", flgs.Dotenv))
		return
	}
	if len(S3_BUCKET) == 0 {
		l.Warn(fmt.Sprintf("Failed to read S3_BUCKET from env file (%s).", flgs.Dotenv))
		return
	}
	if len(MIN_FREE_DISK) == 0 {
		l.Warn(fmt.Sprintf("Failed to read MIN_FREE_DISK from env file (%s).", flgs.Dotenv))
		return
	}
	if len(CORS_ON_AT_RT) == 0 {
		l.Warn(fmt.Sprintf("Failed to read CORS_ON_AT_RT from env file (%s).", flgs.Dotenv))
		return
	}
	flgs.StorageUseLocal = CUBER_S3_USE_LOCAL == "1"
	flgs.StorageS3AccessKey = AWS_ACCESS_KEY_ID
	flgs.StorageS3SecretAccessKey = AWS_SECRET_ACCESS_KEY
	flgs.StorageS3Region = AWS_REGION
	flgs.StorageS3Bucket = S3_BUCKET
	flgs.MinFreeDisk = common.StrToInt32(MIN_FREE_DISK) * 1024 // MB
	flgs.CorsOnAtRT = CORS_ON_AT_RT == "1"
	// ファイル保管用
	s3c, err := s3client.NewS3Client(flgs.StorageS3AccessKey, flgs.StorageS3SecretAccessKey, flgs.StorageS3Region, flgs.StorageS3Bucket, config.S3C_LOCAL_ROOT, config.DL_LOCAL_ROOT, flgs.StorageUseLocal)
	if err != nil {
		l.Warn(fmt.Sprintf("Failed to build new s3 client for general: %s", err.Error()))
		return
	}

	if env.Name == config.ProdEnv.Name {
		gin.SetMode(gin.ReleaseMode)
	}

	r := gin.Default()
	r.Use(func(c *gin.Context) {
		c.Set("RequestID", uuid.New().String())
		c.Next()
	})

	if flgs.CorsOnAtRT {
		r.Use(corsFunc())
	}

	r.NoRoute(func(c *gin.Context) { c.JSON(http.StatusNotFound, gin.H{}) })
	r.GET("/swagger/*any", ginSwagger.WrapHandler(swaggerfiles.Handler))

	db, err := common.GetDb(env)
	if err != nil {
		l.Fatal(fmt.Sprintf("Failed to connect to a DB. Error: %s", err.Error()))
		return
	}

	if l.Level() == zapcore.DebugLevel {
		db.Logger = db.Logger.LogMode(logger.Info)
	}

	sk := flgs.SKey
	if len(sk) == 0 {
		sk = config.DEFAULT_SKEY
	}
	MapRequest(r, l, env, hc, &hn, db, &sk, &flgs, s3c)

	err = r.Run(fmt.Sprintf(":%d", config.REST_PORT))
	if err != nil {
		log.Fatalf("Failed to create REST API on port %d.", config.REST_PORT)
		return
	}
}

func corsFunc() gin.HandlerFunc {
	return cors.New(cors.Config{
		AllowOrigins:     []string{"*"},
		AllowMethods:     []string{"GET", "POST", "PUT", "PATCH", "DELETE"},
		AllowHeaders:     []string{"Origin", "Content-Type", "Authorization", "X-Key"},
		ExposeHeaders:    []string{"Content-Length"},
		AllowCredentials: true,
		MaxAge:           12 * time.Hour,
	})
}
