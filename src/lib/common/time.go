package common

import (
	"fmt"
	"time"
)

const (
	DATETIME_LAYOUT = "2006-01-02T15:04:05"
	TIME_LAYOUT     = "15:04:05"
)

func ParseStrToTime(timeStr *string) (time.Time, error) {
	str := fmt.Sprintf("9999-01-01T%s", *timeStr)
	t, err := time.ParseInLocation(DATETIME_LAYOUT, str, time.Local)
	if err != nil {
		return time.Time{}, err
	}
	return t, nil
}

func ParseStrToDatetime(timeStr *string) (time.Time, error) {
	t, err := time.ParseInLocation(DATETIME_LAYOUT, *timeStr, time.Local)
	if err != nil {
		return time.Time{}, err
	}
	return t, nil
}

func ParseTimeToStr(t *time.Time) string {
	if t == nil {
		return ""
	}
	return t.In(time.Local).Format(TIME_LAYOUT)
}

func ParseDatetimeToStr(t *time.Time) string {
	if t == nil {
		return ""
	}
	return t.In(time.Local).Format(DATETIME_LAYOUT)
}

func GetNow() time.Time {
	return time.Now().In(time.Local)
}

func GetNowStr() string {
	t := time.Now().In(time.Local)
	return ParseDatetimeToStr(&t)
}

func GetNowUnix() *int64 {
	unix := time.Now().In(time.Local).Unix()
	return &unix
}

func GetNowUnixMilli() *int64 {
	unix := time.Now().In(time.Local).UnixMilli()
	return &unix
}
