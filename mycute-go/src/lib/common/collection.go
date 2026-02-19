package common

import (
	"reflect"
)

func StructForEach[T any](obj *T, callback func(k string, v reflect.Value) bool) {
	values := reflect.ValueOf(*obj)
	types := values.Type()
	for i := 0; i < values.NumField(); i++ {
		k := types.Field(i).Name
		v := values.Field(i)
		if !callback(k, v) {
			break
		}
	}
}

func InArray[T comparable](val *T, arr *[]T) bool {
	if arr == nil {
		return false
	}
	if len(*arr) == 0 {
		return false
	}
	for _, item := range *arr {
		if item == *val {
			return true
		}
	}
	return false
}

func RmElmFromStrArr(arr *[]string, elm *string) *[]string {
	var r []string
	for _, i := range *arr {
		if i != *elm {
			r = append(r, i)
		}
	}
	return &r
}
