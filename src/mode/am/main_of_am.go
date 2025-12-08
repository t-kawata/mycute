package am

import (
	"fmt"
	"log"

	"github.com/t-kawata/mycute/lib/common"
	"go.uber.org/zap"
	"go.uber.org/zap/zapcore"
	"gorm.io/gorm/logger"
)

func MainOfAM() {
	_, cflgs, l, env, _, err := common.Init("mycute am mode", &[]common.Flag{})
	if err != nil {
		log.Fatalf("Error: %s", err.Error())
		return
	}
	db, err := common.GetDb(env)
	if err != nil {
		l.Fatal(fmt.Sprintf("Failed to get db: %s", err.Error()))
		return
	}
	if l.Level() == zapcore.DebugLevel {
		db.Logger = db.Logger.LogMode(logger.Info)
	}
	l.Info("Set AM flags: ", zap.String("e", cflgs.Env), zap.String("l", cflgs.LogLevel), zap.String("o", cflgs.Output))
	err = common.AutoMigrateNDb(db)
	if err != nil {
		log.Fatalf("Error: %s", err.Error())
		return
	}
	l.Info("Succeeded to auto migrate.")
}
