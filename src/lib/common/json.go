package common

import (
	"encoding/json"
	"reflect"

	"gorm.io/datatypes"
)

func ParseJson[T any](jsonStr *string) (T, error) {
	var rtn T
	err := json.Unmarshal([]byte(*jsonStr), &rtn)
	return rtn, err
}

func ParseJsonDirect[T any](jsonStr *string) *T {
	if jsonStr == nil {
		return nil
	}
	var rtn T
	err := json.Unmarshal([]byte(*jsonStr), &rtn)
	if err != nil {
		return nil
	}
	return &rtn
}

func ToJson[T any](obj T) (string, error) {
	// Tが[]uint8型の場合は[]intに変換
	v := reflect.ValueOf(obj)
	if v.Kind() == reflect.Slice && v.Type().Elem().Kind() == reflect.Uint8 {
		length := v.Len()
		intSlice := make([]int, length)
		for i := range length {
			intSlice[i] = int(v.Index(i).Uint())
		}
		jsonStr, err := json.Marshal(intSlice)
		return string(jsonStr), err
	}
	jsonStr, err := json.Marshal(obj)
	return string(jsonStr), err
}

func ToJsonDirect[T any](obj T) string {
	// Tが[]uint8型の場合は[]intに変換
	v := reflect.ValueOf(obj)
	if v.Kind() == reflect.Slice && v.Type().Elem().Kind() == reflect.Uint8 {
		length := v.Len()
		intSlice := make([]int, length)
		for i := range length {
			intSlice[i] = int(v.Index(i).Uint())
		}
		jsonStr, err := json.Marshal(intSlice)
		if err != nil {
			return ""
		}
		return string(jsonStr)
	}
	jsonStr, err := json.Marshal(obj)
	if err != nil {
		return ""
	}
	return string(jsonStr)
}

func ParseDatatypesJson[T any](jsonData *datatypes.JSON) (T, error) {
	var rtn T
	bt, err := (*jsonData).MarshalJSON()
	if err != nil {
		return rtn, err
	}
	err = json.Unmarshal(bt, &rtn)
	if err != nil {
		return rtn, err
	}
	return rtn, nil
}

func ToJsonForDatatypesJson(jsonData *datatypes.JSON) (string, error) {
	jsonStr, err := json.Marshal(*jsonData)
	if err != nil {
		return "", err
	}
	return string(jsonStr), nil
}
