package logger

import (
	"go.uber.org/zap"
	"go.uber.org/zap/zapcore"
)

func BuildLogger(ll *string, o *string) *zap.Logger {
	var LOG_LEVELS = map[string]zapcore.Level{
		"debug":  zapcore.DebugLevel,
		"info":   zapcore.InfoLevel,
		"warn":   zapcore.WarnLevel,
		"error":  zapcore.ErrorLevel,
		"dpanic": zapcore.DPanicLevel,
		"panic":  zapcore.PanicLevel,
		"fatal":  zapcore.FatalLevel,
	}
	logLevel := zap.NewAtomicLevel()
	level, ok := LOG_LEVELS[*ll]
	if !ok {
		return nil
	}
	logLevel.SetLevel(level)
	zapConfig := zap.Config{
		Level:    logLevel,
		Encoding: "console",
		EncoderConfig: zapcore.EncoderConfig{
			TimeKey:        "Time",
			LevelKey:       "Level",
			NameKey:        "Name",
			CallerKey:      "Caller",
			MessageKey:     "Msg",
			StacktraceKey:  "St",
			EncodeLevel:    zapcore.CapitalLevelEncoder,
			EncodeTime:     zapcore.ISO8601TimeEncoder,
			EncodeDuration: zapcore.StringDurationEncoder,
			EncodeCaller:   zapcore.ShortCallerEncoder,
		},
		OutputPaths:      []string{*o},
		ErrorOutputPaths: []string{"stderr"},
	}
	l, _ := zapConfig.Build()
	return l
}

type Logger struct {
	Level  int8
	Levels map[string]int8
	Zap    *zap.Logger
}

func (l *Logger) Debug(msg string, fields ...zapcore.Field) {
	if l.Level > l.Levels["DEBUG"] {
		return
	}
	l.Zap.Debug(msg, fields...)
}

func (l *Logger) Info(msg string, fields ...zapcore.Field) {
	if l.Level > l.Levels["INFO"] {
		return
	}
	l.Zap.Info(msg, fields...)
}

func (l *Logger) Warn(msg string, fields ...zapcore.Field) {
	if l.Level > l.Levels["WARN"] {
		return
	}
	l.Zap.Warn(msg, fields...)
}

func (l *Logger) Error(msg string, fields ...zapcore.Field) {
	if l.Level > l.Levels["ERROR"] {
		return
	}
	l.Zap.Error(msg, fields...)
}

func (l *Logger) DPanic(msg string, fields ...zapcore.Field) {
	if l.Level > l.Levels["DPANIC"] {
		return
	}
	l.Zap.DPanic(msg, fields...)
}

func (l *Logger) Panic(msg string, fields ...zapcore.Field) {
	if l.Level > l.Levels["PANIC"] {
		return
	}
	l.Zap.Panic(msg, fields...)
}

func (l *Logger) Fatal(msg string, fields ...zapcore.Field) {
	if l.Level > l.Levels["FATAL"] {
		return
	}
	l.Zap.Fatal(msg, fields...)
}
