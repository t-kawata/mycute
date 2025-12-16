package rtutil

import (
	"errors"
	"fmt"
	"path/filepath"
	"reflect"
	"regexp"
	"strings"
	"time"

	"github.com/gin-gonic/gin"
	"github.com/gin-gonic/gin/binding"
	"github.com/go-playground/validator/v10"
	"github.com/golang-jwt/jwt/v5"
	"github.com/iancoleman/strcase"
	"github.com/t-kawata/mycute/config"
	"github.com/t-kawata/mycute/enum/rterr"
	"github.com/t-kawata/mycute/lib/common"
	"github.com/t-kawata/mycute/lib/httpclient"
	"github.com/t-kawata/mycute/lib/s3client"
	"github.com/t-kawata/mycute/mode/rt/rtres"
	"github.com/t-kawata/mycute/model"
	"github.com/t-kawata/mycute/pkg/cuber"
	"go.uber.org/zap"
	"golang.org/x/crypto/bcrypt"
	"gorm.io/gorm"
)

type RtUtil struct {
	Logger          *zap.Logger
	Env             *config.Env
	Client          *httpclient.HttpClient
	DB              *gorm.DB
	SKey            string
	S3c             *s3client.S3Client
	Hostname        *string
	DBDirPath       *string
	CuberService    *cuber.CuberService
	CuberCryptoSkey string
}

type JwtUsr struct {
	ApxID   *uint
	VdrID   *uint
	UsrID   *uint
	StaffID *uint
	Email   string
	Type    uint8
	Exp     time.Time
}

var (
	v             = validator.New()
	RegexpChecker = func(str string, exp string) bool {
		re := regexp.MustCompile(exp)
		return re.MatchString(str)
	}
	IsJwtFormat = func(str string) bool {
		return RegexpChecker(str, "^[A-Za-z0-9-_]+\\.[A-Za-z0-9-_]+\\.[A-Za-z0-9-_]*$")
	}
	RegexpValidator = func() validator.Func {
		return func(fl validator.FieldLevel) bool {
			if f, ok := fl.Field().Interface().(string); ok {
				if len(f) == 0 {
					return true
				}
				p := fl.Param()
				return RegexpChecker(f, p)
			}
			return false
		}
	}
	PasswordValidator = func() validator.Func {
		return func(fl validator.FieldLevel) bool {
			if f, ok := fl.Field().Interface().(string); ok {
				if len(f) == 0 {
					return true
				}
				return RegexpChecker(f, config.PW_REGEXP)
			}
			return false
		}
	}
	HalfValidator = func() validator.Func {
		return func(fl validator.FieldLevel) bool {
			if f, ok := fl.Field().Interface().(string); ok {
				if len(f) == 0 {
					return true
				}
				return RegexpChecker(f, "^[!-~]+$")
			}
			return false
		}
	}
	HalfsValidator = func() validator.Func {
		return func(fl validator.FieldLevel) bool {
			i := fl.Field().Interface()
			if i == nil {
				return true
			}
			if f, ok := i.([]string); ok {
				if len(f) == 0 {
					return true
				}
				for _, item := range f {
					if !RegexpChecker(item, "^[!-~]+$") {
						return false
					}
				}
				return true
			}
			return false
		}
	}
	TimeValidator = func() validator.Func {
		return func(fl validator.FieldLevel) bool {
			if f, ok := fl.Field().Interface().(string); ok {
				if len(f) == 0 {
					return true
				}
				return RegexpChecker(f, "^([01][0-9]|2[0-3]):[0-5][0-9]:[0-5][0-9]$")
			}
			return false
		}
	}
	DatetimeValidator = func() validator.Func {
		return func(fl validator.FieldLevel) bool {
			if f, ok := fl.Field().Interface().(string); ok {
				if len(f) == 0 {
					return true
				}
				return RegexpChecker(f, "^[12][0-9]{3}-(0[1-9]|1[0-2])-([0-2][0-9]|3[01])T([01][0-9]|2[0-3]):[0-5][0-9]:[0-5][0-9]$")
			}
			return false
		}
	}
	Ipv4Validator = func() validator.Func {
		return func(fl validator.FieldLevel) bool {
			if f, ok := fl.Field().Interface().(string); ok {
				if len(f) == 0 {
					return true
				}
				return RegexpChecker(f, `^(?:[0-9]{1,3}\.){3}[0-9]{1,3}$`)
			}
			return false
		}
	}
	Ipv4AndPortValidator = func() validator.Func {
		return func(fl validator.FieldLevel) bool {
			if f, ok := fl.Field().Interface().(string); ok {
				if len(f) == 0 {
					return true
				}
				return RegexpChecker(f, `^(?:[0-9]{1,3}\.){3}[0-9]{1,3}:(80|[1-9][0-9]{2,3}|[1-5][0-9]{4}|6[0-4][0-9]{3}|65[0-4][0-9]{2}|655[0-2][0-9]|6553[0-5])$`)
			}
			return false
		}
	}
	Ipv6Validator = func() validator.Func {
		return func(fl validator.FieldLevel) bool {
			if f, ok := fl.Field().Interface().(string); ok {
				if len(f) == 0 {
					return true
				}
				return RegexpChecker(f, `^(?:[0-9a-fA-F]{1,4}:){7}[0-9a-fA-F]{1,4}$`)
			}
			return false
		}
	}
	HostnameValidator = func() validator.Func {
		return func(fl validator.FieldLevel) bool {
			if f, ok := fl.Field().Interface().(string); ok {
				if len(f) == 0 {
					return true
				}
				return RegexpChecker(f, `^([a-zA-Z0-9]{1}[a-zA-Z0-9-]{0,62})(\.[a-zA-Z0-9]{1}[a-zA-Z0-9-]{0,62})*?(\.[a-zA-Z]{1}[a-zA-Z0-9]{0,62})\.?$`)
			}
			return false
		}
	}
	HostOrIpv4Validator = func() validator.Func {
		return func(fl validator.FieldLevel) bool {
			if f, ok := fl.Field().Interface().(string); ok {
				if len(f) == 0 {
					return true
				}
				return RegexpChecker(f, `^((?:[0-9]{1,3}\.){3}[0-9]{1,3}(:([1-9][0-9]{0,3}|[1-5][0-9]{4}|6[0-4][0-9]{3}|65[0-4][0-9]{2}|655[0-2][0-9]|6553[0-5]))?|([a-zA-Z0-9]{1}[a-zA-Z0-9-]{0,62})(\.[a-zA-Z0-9]{1}[a-zA-Z0-9-]{0,62})*?(\.[a-zA-Z]{1}[a-zA-Z0-9]{0,62})\.?(:([1-9][0-9]{0,3}|[1-5][0-9]{4}|6[0-4][0-9]{3}|65[0-4][0-9]{2}|655[0-2][0-9]|6553[0-5]))?)$`)
			}
			return false
		}
	}
	MaskedIpv4Validator = func() validator.Func {
		return func(fl validator.FieldLevel) bool {
			if f, ok := fl.Field().Interface().(string); ok {
				if len(f) == 0 {
					return true
				}
				return RegexpChecker(f, `^((?:[0-9]{1,3}\.){3}[0-9]{1,3}(:([1-9][0-9]{0,3}|[1-5][0-9]{4}|6[0-4][0-9]{3}|65[0-4][0-9]{2}|655[0-2][0-9]|6553[0-5]))?|([a-zA-Z0-9]{1}[a-zA-Z0-9-]{0,62})(\.[a-zA-Z0-9]{1}[a-zA-Z0-9-]{0,62})*?(\.[a-zA-Z]{1}[a-zA-Z0-9]{0,62})\.?(:([1-9][0-9]{0,3}|[1-5][0-9]{4}|6[0-4][0-9]{3}|65[0-4][0-9]{2}|655[0-2][0-9]|6553[0-5]))?)/([1-9]|[12][0-9]|3[0-2])$`)
			}
			return false
		}
	}
	EmailValidator = func() validator.Func {
		return func(fl validator.FieldLevel) bool {
			if f, ok := fl.Field().Interface().(string); ok {
				if len(f) == 0 {
					return true
				}
				return RegexpChecker(f, "^(?:(?:(?:(?:[a-zA-Z]|\\d|[!#\\$%&'\\*\\+\\-\\/=\\?\\^_`{\\|}~]|[\\x{00A0}-\\x{D7FF}\\x{F900}-\\x{FDCF}\\x{FDF0}-\\x{FFEF}])+(?:\\.([a-zA-Z]|\\d|[!#\\$%&'\\*\\+\\-\\/=\\?\\^_`{\\|}~]|[\\x{00A0}-\\x{D7FF}\\x{F900}-\\x{FDCF}\\x{FDF0}-\\x{FFEF}])+)*)|(?:(?:\\x22)(?:(?:(?:(?:\\x20|\\x09)*(?:\\x0d\\x0a))?(?:\\x20|\\x09)+)?(?:(?:[\\x01-\\x08\\x0b\\x0c\\x0e-\\x1f\\x7f]|\\x21|[\\x23-\\x5b]|[\\x5d-\\x7e]|[\\x{00A0}-\\x{D7FF}\\x{F900}-\\x{FDCF}\\x{FDF0}-\\x{FFEF}])|(?:(?:[\\x01-\\x09\\x0b\\x0c\\x0d-\\x7f]|[\\x{00A0}-\\x{D7FF}\\x{F900}-\\x{FDCF}\\x{FDF0}-\\x{FFEF}]))))*(?:(?:(?:\\x20|\\x09)*(?:\\x0d\\x0a))?(\\x20|\\x09)+)?(?:\\x22))))@(?:(?:(?:[a-zA-Z]|\\d|[\\x{00A0}-\\x{D7FF}\\x{F900}-\\x{FDCF}\\x{FDF0}-\\x{FFEF}])|(?:(?:[a-zA-Z]|\\d|[\\x{00A0}-\\x{D7FF}\\x{F900}-\\x{FDCF}\\x{FDF0}-\\x{FFEF}])(?:[a-zA-Z]|\\d|-|\\.|~|[\\x{00A0}-\\x{D7FF}\\x{F900}-\\x{FDCF}\\x{FDF0}-\\x{FFEF}])*(?:[a-zA-Z]|\\d|[\\x{00A0}-\\x{D7FF}\\x{F900}-\\x{FDCF}\\x{FDF0}-\\x{FFEF}])))\\.)+(?:(?:[a-zA-Z]|[\\x{00A0}-\\x{D7FF}\\x{F900}-\\x{FDCF}\\x{FDF0}-\\x{FFEF}])|(?:(?:[a-zA-Z]|[\\x{00A0}-\\x{D7FF}\\x{F900}-\\x{FDCF}\\x{FDF0}-\\x{FFEF}])(?:[a-zA-Z]|\\d|-|\\.|~|[\\x{00A0}-\\x{D7FF}\\x{F900}-\\x{FDCF}\\x{FDF0}-\\x{FFEF}])*(?:[a-zA-Z]|[\\x{00A0}-\\x{D7FF}\\x{F900}-\\x{FDCF}\\x{FDF0}-\\x{FFEF}])))\\.?$")
			}
			return false
		}
	}
	HttpUrlValidator = func() validator.Func {
		return func(fl validator.FieldLevel) bool {
			if f, ok := fl.Field().Interface().(string); ok {
				if len(f) == 0 {
					return true
				}
				return RegexpChecker(f, `^https?://[\w/:%#\$&\?\(\)~\.=\+\-]+$`)
			}
			return false
		}
	}
	HttpUrlsValidator = func() validator.Func {
		return func(fl validator.FieldLevel) bool {
			i := fl.Field().Interface()
			if i == nil {
				return true
			}
			if f, ok := i.([]string); ok {
				if len(f) == 0 {
					return true
				}
				for _, item := range f {
					if !RegexpChecker(item, `^https?://[\w/:%#\$&\?\(\)~\.=\+\-]+$`) {
						return false
					}
				}
				return true
			}
			return false
		}
	}
	UintsGteValidator = func() validator.Func {
		return func(fl validator.FieldLevel) bool {
			i := fl.Field().Interface()
			if i == nil {
				return true
			}
			if f, ok := i.([]uint); ok {
				if len(f) == 0 {
					return true
				}
				p := common.StrToUint(fl.Param())
				for _, item := range f {
					if item < p {
						return false
					}
				}
				return true
			}
			return false
		}
	}
	UintsLteValidator = func() validator.Func {
		return func(fl validator.FieldLevel) bool {
			i := fl.Field().Interface()
			if i == nil {
				return true
			}
			if f, ok := i.([]uint); ok {
				if len(f) == 0 {
					return true
				}
				p := common.StrToUint(fl.Param())
				for _, item := range f {
					if item > p {
						return false
					}
				}
				return true
			}
			return false
		}
	}
	Uint16sGteValidator = func() validator.Func {
		return func(fl validator.FieldLevel) bool {
			i := fl.Field().Interface()
			if i == nil {
				return true
			}
			if f, ok := i.([]uint16); ok {
				if len(f) == 0 {
					return true
				}
				p := common.StrToUint16(fl.Param())
				for _, item := range f {
					if item < p {
						return false
					}
				}
				return true
			}
			return false
		}
	}
	Uint16sLteValidator = func() validator.Func {
		return func(fl validator.FieldLevel) bool {
			i := fl.Field().Interface()
			if i == nil {
				return true
			}
			if f, ok := i.([]uint16); ok {
				if len(f) == 0 {
					return true
				}
				p := common.StrToUint16(fl.Param())
				for _, item := range f {
					if item > p {
						return false
					}
				}
				return true
			}
			return false
		}
	}
	Uint8sGteValidator = func() validator.Func {
		return func(fl validator.FieldLevel) bool {
			i := fl.Field().Interface()
			if i == nil {
				return true
			}
			if f, ok := i.([]uint8); ok {
				if len(f) == 0 {
					return true
				}
				p := common.StrToUint8(fl.Param())
				for _, item := range f {
					if item < p {
						return false
					}
				}
				return true
			}
			return false
		}
	}
	Uint8sLteValidator = func() validator.Func {
		return func(fl validator.FieldLevel) bool {
			i := fl.Field().Interface()
			if i == nil {
				return true
			}
			if f, ok := i.([]uint8); ok {
				if len(f) == 0 {
					return true
				}
				p := common.StrToUint8(fl.Param())
				for _, item := range f {
					if item > p {
						return false
					}
				}
				return true
			}
			return false
		}
	}
	NotEmptyStrArrValidator = func() validator.Func {
		return func(fl validator.FieldLevel) bool {
			i := fl.Field().Interface()
			if i == nil {
				return true
			}
			if f, ok := i.([]string); ok {
				return len(f) != 0
			}
			return false
		}
	}
)

func (u *RtUtil) HashPassword(password string) string {
	hash, err := bcrypt.GenerateFromPassword([]byte(password), bcrypt.DefaultCost)
	if err != nil {
		return ""
	}
	return string(hash)
}

func (u *RtUtil) IsEqualHashAndPassword(hash string, password string) bool {
	err := bcrypt.CompareHashAndPassword([]byte(hash), []byte(password))
	return err == nil
}

func (u *RtUtil) IsValidKey(key string) bool {
	var (
		keys []model.Key
		r    = false
		err  error
	)
	u.DB.Select("hash").Where("`keys`.`bgn_at` <= NOW() + INTERVAL 9 HOUR AND NOW() + INTERVAL 9 HOUR <= `keys`.`end_at`").Find(&keys)
	for _, k := range keys {
		err = bcrypt.CompareHashAndPassword([]byte(k.Hash), []byte(key))
		if err == nil {
			r = true
			break
		}
	}
	return r
}

func (u *RtUtil) GetRequestID(c *gin.Context) (requestID *string) {
	rID := ""
	requestID = &rID
	v, ok := c.Get("RequestID")
	if !ok || v == nil {
		*requestID = ""
		return
	}
	rID, ok = v.(string)
	if !ok {
		*requestID = ""
		return
	}
	requestID = &rID
	return
}

func (u *RtUtil) GetToken(c *gin.Context) string {
	a := c.Request.Header.Get("Authorization")
	if !u.RegexpChecker(a, "^Bearer +.+$") || len(a) <= 7 {
		return ""
	}
	return a[7:]
}

func (u *RtUtil) GetValidationErrs(err error) []rtres.Err {
	rtn := []rtres.Err{}
	if err != nil {
		if ve, ok := err.(validator.ValidationErrors); ok {
			for _, fe := range ve {
				code, msg := CreateCodeMsg(fe.Tag(), fe.Param())
				rtn = append(rtn, rtres.Err{Field: strcase.ToSnake(fe.Field()), Code: code, Message: msg})
			}
		} else {
			rtn = append(rtn, rtres.Err{Field: "system", Code: 9999, Message: "Any of the parameters sent may have fatal formatting errors."})
		}
	}
	return rtn
}

func (u *RtUtil) RegexpChecker(str string, exp string) bool {
	re := regexp.MustCompile(exp)
	return re.MatchString(str)
}

func (u *RtUtil) GetQueMohName(actId *uint) string {
	return fmt.Sprintf("q-%d", *actId)
}

func (u *RtUtil) GetStructName(structPointer any) (name *string) {
	var nameStr string
	ptrValue := reflect.ValueOf(structPointer)
	if ptrValue.Kind() == reflect.Ptr {
		structType := ptrValue.Elem().Type()
		nameStr = structType.Name()
	} else {
		nameStr = reflect.TypeOf(structPointer).Name()
	}
	name = &nameStr
	return
}

// GetCubeDBFilePath は Cube の DB ファイルのパスを返します。
func (u *RtUtil) GetCubeDBFilePath(uuid *string, apxID *uint, vdrID *uint, usrID *uint) (string, error) {
	if uuid == nil || apxID == nil || vdrID == nil || usrID == nil {
		return "", errors.New("Empty uuid or apxID or vdrID or usrID.")
	}
	return filepath.Join(*u.DBDirPath, fmt.Sprintf("%d-%d-%d", *apxID, *vdrID, *usrID), *uuid+".db"), nil
}

func GetStructName(structPointer any) (name *string) {
	var nameStr string
	ptrValue := reflect.ValueOf(structPointer)
	if ptrValue.Kind() == reflect.Ptr {
		structType := ptrValue.Elem().Type()
		nameStr = structType.Name()
	} else {
		nameStr = reflect.TypeOf(structPointer).Name()
	}
	name = &nameStr
	return
}

type AmiOriginateExtenParams struct {
	Channel      string
	Context      string
	Exten        string
	Priority     uint8
	Timeout      uint16 // in seconds
	CallerIDName string
	CallerIDNum  string
	Variable     string
}

func (j *JwtUsr) IsFromKey() bool {
	return j.ApxID == nil && j.VdrID == nil && j.UsrID == nil
}

func (j *JwtUsr) IsApx() bool {
	if j.UsrID == nil {
		return false
	}
	return j.ApxID == nil && j.VdrID == nil && *j.UsrID > 0
}

func (j *JwtUsr) IsVdr() bool {
	if j.ApxID == nil || j.UsrID == nil {
		return false
	}
	return *j.ApxID > 0 && j.VdrID == nil && *j.UsrID > 0
}

func (j *JwtUsr) IsUsr() bool {
	if j.ApxID == nil || j.VdrID == nil || j.UsrID == nil {
		return false
	}
	return *j.ApxID > 0 && *j.VdrID > 0 && *j.UsrID > 0
}

func (j *JwtUsr) IDs(isOnlyForVdrAndUsr bool) *common.IDs {
	var (
		aid *uint = nil
		vid *uint = nil
		uid *uint = nil
	)
	if isOnlyForVdrAndUsr {
		if j.IsVdr() {
			aid = j.ApxID
			vid = j.UsrID
			uid = j.UsrID
		} else if j.IsUsr() {
			aid = j.ApxID
			vid = j.VdrID
			uid = j.UsrID
		}
	} else {
		if j.IsApx() {
			aid = j.UsrID
		} else if j.IsVdr() {
			aid = j.ApxID
			vid = j.UsrID
		} else if j.IsUsr() {
			aid = j.ApxID
			vid = j.VdrID
			uid = j.UsrID
		}
	}
	return &common.IDs{ApxID: aid, VdrID: vid, UsrID: uid}
}

func GenerateToken(skey string, lifeTime uint, u *JwtUsr) (string, error) {
	claims := jwt.MapClaims{
		"apx_id":   u.ApxID,
		"vdr_id":   u.VdrID,
		"usr_id":   u.UsrID,
		"email":    u.Email,
		"type":     u.Type,
		"is_staff": *u.StaffID > 0,
		"exp":      time.Now().Add(time.Hour * time.Duration(lifeTime)).Unix(),
	}
	token := jwt.NewWithClaims(jwt.SigningMethodHS256, claims)
	tokenString, err := token.SignedString([]byte(skey))
	if err != nil {
		return "", err
	}
	return tokenString, nil
}

func GetToken(c *gin.Context) string {
	a := c.Request.Header.Get("Authorization")
	if !RegexpChecker(a, "^Bearer +.+$") || len(a) <= 7 {
		return ""
	}
	return a[7:]
}

func ParseToken(skey string, tokenString string) (*jwt.Token, error) {
	token, err := jwt.Parse(tokenString, func(token *jwt.Token) (interface{}, error) {
		if _, ok := token.Method.(*jwt.SigningMethodHMAC); !ok {
			return nil, fmt.Errorf("Unexpected signing method: %v", token.Header["alg"])
		}
		return []byte(skey), nil
	})
	if err != nil {
		return nil, err
	}
	return token, nil
}

func CreateCodeMsg(tag string, param string) (uint16, string) {
	switch tag {
	case "required":
		return rterr.Required.Code(), rterr.Required.Msg()
	case "number":
		return rterr.Number.Code(), rterr.Number.Msg()
	case "regexp":
		return rterr.Regexp.Code(), fmt.Sprintf(rterr.Regexp.Msg(), param)
	case "email":
		return rterr.Email.Code(), rterr.Email.Msg()
	case "min":
		return rterr.Min.Code(), fmt.Sprintf(rterr.Min.Msg(), param)
	case "max":
		return rterr.Max.Code(), fmt.Sprintf(rterr.Max.Msg(), param)
	case "password":
		return rterr.Password.Code(), rterr.Password.Msg()
	case "half":
		return rterr.Half.Code(), rterr.Half.Msg()
	case "halfs":
		return rterr.Halfs.Code(), rterr.Halfs.Msg()
	case "time":
		return rterr.Time.Code(), rterr.Time.Msg()
	case "datetime":
		return rterr.Datetime.Code(), rterr.Datetime.Msg()
	case "ipv4":
		return rterr.Ipv4.Code(), rterr.Ipv4.Msg()
	case "ipv4_and_port":
		return rterr.Ipv4AndPort.Code(), rterr.Ipv4AndPort.Msg()
	case "ipv6":
		return rterr.Ipv6.Code(), rterr.Ipv6.Msg()
	case "hostname":
		return rterr.Hostname.Code(), rterr.Hostname.Msg()
	case "host_or_ipv4":
		return rterr.HostOrIpv4.Code(), rterr.HostOrIpv4.Msg()
	case "masked_ipv4":
		return rterr.MaskedIpv4.Code(), rterr.MaskedIpv4.Msg()
	case "http_url":
		return rterr.HttpUrl.Code(), rterr.HttpUrl.Msg()
	case "http_urls":
		return rterr.HttpUrls.Code(), rterr.HttpUrls.Msg()
	case "oneof":
		return rterr.Oneof.Code(), fmt.Sprintf(rterr.Oneof.Msg(), strings.ReplaceAll(param, " ", ", "))
	case "gte":
		return rterr.Gte.Code(), fmt.Sprintf(rterr.Gte.Msg(), param)
	case "lte":
		return rterr.Lte.Code(), fmt.Sprintf(rterr.Lte.Msg(), param)
	case "boolean":
		return rterr.Boolean.Code(), rterr.Boolean.Msg()
	case "uint8s_gte":
		return rterr.Uint8sGte.Code(), fmt.Sprintf(rterr.Uint8sGte.Msg(), param)
	case "uint16s_gte":
		return rterr.Uint16sGte.Code(), fmt.Sprintf(rterr.Uint16sGte.Msg(), param)
	case "uints_gte":
		return rterr.UintsGte.Code(), fmt.Sprintf(rterr.UintsGte.Msg(), param)
	case "uint8s_lte":
		return rterr.Uint8sLte.Code(), fmt.Sprintf(rterr.Uint8sLte.Msg(), param)
	case "uint16s_lte":
		return rterr.Uint16sLte.Code(), fmt.Sprintf(rterr.Uint16sLte.Msg(), param)
	case "uints_lte":
		return rterr.UintsLte.Code(), fmt.Sprintf(rterr.UintsLte.Msg(), param)
	case "not_empty_str_arr":
		return rterr.NotEmptyStrArr.Code(), rterr.NotEmptyStrArr.Msg()
	}
	return 0, ""
}

func RegisterValidations() {
	if v, ok := binding.Validator.Engine().(*validator.Validate); ok {
		v.RegisterValidation("regexp", RegexpValidator())
		v.RegisterValidation("password", PasswordValidator())
		v.RegisterValidation("half", HalfValidator())
		v.RegisterValidation("halfs", HalfsValidator())
		v.RegisterValidation("time", TimeValidator())
		v.RegisterValidation("datetime", DatetimeValidator())
		v.RegisterValidation("ipv4", Ipv4Validator())
		v.RegisterValidation("ipv4_and_port", Ipv4AndPortValidator())
		v.RegisterValidation("ipv6", Ipv6Validator())
		v.RegisterValidation("hostname", HostnameValidator())
		v.RegisterValidation("host_or_ipv4", HostOrIpv4Validator())
		v.RegisterValidation("masked_ipv4", MaskedIpv4Validator())
		v.RegisterValidation("http_url", HttpUrlValidator())
		v.RegisterValidation("http_urls", HttpUrlsValidator())
		v.RegisterValidation("email", EmailValidator())
		v.RegisterValidation("uint8s_gte", Uint8sGteValidator())
		v.RegisterValidation("uint16s_gte", Uint16sGteValidator())
		v.RegisterValidation("uints_gte", UintsGteValidator())
		v.RegisterValidation("uint8s_lte", Uint8sLteValidator())
		v.RegisterValidation("uint16s_lte", Uint16sLteValidator())
		v.RegisterValidation("uints_lte", UintsLteValidator())
		v.RegisterValidation("not_empty_str_arr", NotEmptyStrArrValidator())
	}
}

func DirectValidate(value any, binding string) error {
	if err := v.Var(v, binding); err != nil {
		return err
	}
	return nil
}

func DirectValidateReqField[T any](req *T, fieldName string) error {
	if v, b := GetValueAndBindingByField(req, fieldName); v != nil && len(b) > 0 {
		return DirectValidate(v, b)
	}
	return nil
}

func DirectValidateReqFields[T any](req *T, fieldNames []string) error {
	for _, fn := range fieldNames {
		if v, b := GetValueAndBindingByField(req, fn); v != nil && len(b) > 0 {
			return DirectValidate(v, b)
		}
	}
	return nil
}

func GetBindingByField[T any](req *T, fieldName string) string {
	t := reflect.TypeOf(req)
	if t.Kind() == reflect.Ptr {
		t = t.Elem()
	}
	field, ok := t.FieldByName(fieldName)
	if !ok {
		return ""
	}
	return field.Tag.Get("binding")
}

func GetValueAndBindingByField[T any](req *T, fieldName string) (any, string) {
	v := reflect.ValueOf(req)
	t := v.Type()
	if t.Kind() == reflect.Ptr {
		t = t.Elem()
		v = v.Elem()
	}
	field, ok := t.FieldByName(fieldName)
	if !ok {
		return nil, ""
	}
	return v.FieldByIndex(field.Index).Interface(), field.Tag.Get("binding")
}
