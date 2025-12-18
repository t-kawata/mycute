package rtbl

import (
	"bytes"
	"context"
	"errors"
	"fmt"
	"net/http"
	"net/url"
	"reflect"
	"regexp"

	"github.com/gin-gonic/gin"
	"github.com/t-kawata/mycute/enum/rterr"
	"github.com/t-kawata/mycute/enum/usrtype"
	"github.com/t-kawata/mycute/mode/rt/rtres"
	"github.com/t-kawata/mycute/mode/rt/rtutil"
	"github.com/t-kawata/mycute/model"
)

func OK[DATA any, RES any](c *gin.Context, data *DATA, res *RES) bool {
	v := reflect.ValueOf(res).Elem()
	field := v.FieldByName("Data")
	if field.IsValid() && field.CanSet() {
		field.Set(reflect.ValueOf(*data))
	}
	c.JSON(http.StatusOK, res)
	return true
}

func BadRequest[T any](c *gin.Context, res *T) bool {
	c.JSON(http.StatusBadRequest, res)
	return false
}

func BadRequestWithSpecErr[T any](c *gin.Context, res *T) bool {
	SetErrInRes(res, "system", rterr.BadRequest.Code(), rterr.BadRequest.Msg())
	c.JSON(http.StatusBadRequest, res)
	return false
}

func BadRequestCustomMsg[T any](c *gin.Context, res *T, msg string) bool {
	return errRes(c, res, http.StatusBadRequest, "system", rterr.BadRequest.Code(), msg)
}

func Unauthorized[T any](c *gin.Context, res *T) bool {
	return errRes(c, res, http.StatusUnauthorized, "auth", rterr.Unauthorized.Code(), rterr.Unauthorized.Msg())
}

func Forbidden[T any](c *gin.Context, res *T) bool {
	return errRes(c, res, http.StatusForbidden, "system", rterr.Forbidden.Code(), rterr.Forbidden.Msg())
}

func ForbiddenCustomMsg[T any](c *gin.Context, res *T, msg string) bool {
	return errRes(c, res, http.StatusForbidden, "system", rterr.Forbidden.Code(), msg)
}

func NotFound[T any](c *gin.Context, res *T) bool {
	return errRes(c, res, http.StatusNotFound, "system", rterr.NotFound.Code(), rterr.NotFound.Msg())
}

func NotFoundCustomMsg[T any](c *gin.Context, res *T, msg string) bool {
	return errRes(c, res, http.StatusNotFound, "system", rterr.NotFound.Code(), msg)
}

func InternalServerError[T any](c *gin.Context, res *T) bool {
	return errRes(c, res, http.StatusInternalServerError, "system", rterr.InternalServerError.Code(), rterr.InternalServerError.Msg())
}

func InternalServerErrorCustomMsg[T any](c *gin.Context, res *T, msg string) bool {
	return errRes(c, res, http.StatusInternalServerError, "system", rterr.InternalServerError.Code(), msg)
}

func SetErrInRes[T any](res *T, filed string, code uint16, msg string) {
	v := reflect.ValueOf(res).Elem()
	field := v.FieldByName("Errors")
	if field.IsValid() && field.CanSet() {
		field.Set(reflect.ValueOf([]rtres.Err{{Field: filed, Code: code, Message: msg}}))
	}
}

func errRes[T any](c *gin.Context, res *T, status int, filed string, code uint16, msg string) bool {
	v := reflect.ValueOf(res).Elem()
	field := v.FieldByName("Errors")
	if field.IsValid() && field.CanSet() {
		field.Set(reflect.ValueOf([]rtres.Err{{Field: filed, Code: code, Message: msg}}))
	}
	c.JSON(status, res)
	return false
}

func IsApx(aid *uint, vid *uint) bool {
	return *aid == 0 && *vid == 0
}

func IsVdr(aid *uint, vid *uint) bool {
	return *aid > 0 && *vid == 0
}

func IsUsr(aid *uint, vid *uint) bool {
	return *aid > 0 && *vid > 0
}

func RejectUsr(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, types []usrtype.UsrType) bool {
	j := ju
	for _, t := range types {
		if (t == usrtype.KEY && j.IsFromKey()) ||
			(t == usrtype.APX && j.IsApx()) ||
			(t == usrtype.VDR && j.IsVdr()) ||
			(t == usrtype.USR && j.IsUsr()) {
			Forbidden(c, &rtres.DummyRes{})
			return true
		}
	}
	return false
}

func SendSlack(u *rtutil.RtUtil, slackUrl string, slackIconUrl string, slackOrg string, lines *string, titleMessage string) (err error) {
	payloadBase := "{\"icon_url\": \"%s\",\"attachments\": [{\"author_name\": \"%s\",\"author_icon\": \"%s\",\"text\": \"```%s```\"}],\"text\": \"*%s Bot Notification*\n%s\"}"
	payload := fmt.Sprintf(
		payloadBase,
		slackIconUrl,
		slackOrg,
		slackIconUrl,
		*lines,
		slackOrg,
		titleMessage,
	)
	data := url.Values{}
	data.Set("payload", payload)
	resp, err := u.Client.Client.PostForm(slackUrl, data)
	if err != nil {
		return
	}
	defer resp.Body.Close()
	return
}

func SendChatwork(u *rtutil.RtUtil, chatworkRoomID string, chatworkToken string, lines *string) (err error) {
	endpoint := fmt.Sprintf("https://api.chatwork.com/v2/rooms/%s/messages?self_unread=1", chatworkRoomID)
	form := url.Values{}
	form.Set("body", *lines)
	req, err := http.NewRequestWithContext(context.Background(), http.MethodPost, endpoint, bytes.NewBufferString(form.Encode()))
	if err != nil {
		return fmt.Errorf("failed to create request: %w", err)
	}
	req.Header.Set("accept", "application/json")
	req.Header.Set("x-chatworktoken", chatworkToken)
	req.Header.Set("Content-Type", "application/x-www-form-urlencoded")
	resp, err := u.Client.Client.Do(req)
	if err != nil {
		return fmt.Errorf("failed to send request: %w", err)
	}
	defer resp.Body.Close()
	return nil
}

func HasPlaceholder(s string) bool {
	r, err := regexp.MatchString(`\{\{(\d+)\}\}`, s)
	if err != nil {
		return false
	}
	return r
}

func getJwtUsrName(u *rtutil.RtUtil, apxID *uint, vdrID *uint, usrID *uint) (string, error) {
	if apxID == nil || vdrID == nil || usrID == nil {
		return "", errors.New("Missing apxID or vdrID or usrID.")
	}
	var usr model.Usr
	if err := u.DB.Select("name").Where("apx_id = ? AND vdr_id = ? AND id = ?", *apxID, *vdrID, *usrID).First(&usr).Error; err != nil {
		return "", fmt.Errorf("Failed to get user: %s", err.Error())
	}
	return usr.Name, nil
}
