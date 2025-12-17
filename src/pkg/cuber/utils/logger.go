package utils

import (
	"go.uber.org/zap"
)

const LOG_PREFIX = "[CUBER]: "

// LogDebug outputs a debug log with [CUBER] prefix.
func LogDebug(l *zap.Logger, msg string, fields ...zap.Field) {
	if l != nil {
		l.Debug(LOG_PREFIX+msg, fields...)
	}
}

// LogInfo outputs an info log with [CUBER] prefix.
// Use this only for significant milestones.
func LogInfo(l *zap.Logger, msg string, fields ...zap.Field) {
	if l != nil {
		l.Info(LOG_PREFIX+msg, fields...)
	}
}

// LogWarn outputs a warning log with [CUBER] prefix.
// Use this for recoverable errors or issues.
func LogWarn(l *zap.Logger, msg string, fields ...zap.Field) {
	if l != nil {
		l.Warn(LOG_PREFIX+msg, fields...)
	}
}
