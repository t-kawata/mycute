package rtres

import (
	"github.com/t-kawata/mycute/lib/common"
	"github.com/t-kawata/mycute/model"
)

type AuthUsrResData struct {
	Token string `json:"token" swaggertype:"string" format:"" example:"???????????????"`
} // @name AuthUsrResData

type AuthUsrRes struct {
	Data   AuthUsrResData `json:"data"`
	Errors []Err          `json:"errors"`
} // @name AuthUsrRes

type SearchUsrsResData struct {
	ID    uint   `json:"id" swaggertype:"integer" format:"" example:"1"`
	ApxID uint   `json:"apx_id" swaggertype:"integer" format:"" example:"1"`
	VdrID uint   `json:"vdr_id" swaggertype:"integer" format:"" example:"1"`
	Name  string `json:"name" swaggertype:"string" format:"" example:"User01"`
	Email string `json:"email" swaggertype:"string" format:"email" example:"sample@example.com"`
	BgnAt string `json:"bgn_at" swaggertype:"string" format:"date-time" example:"2023-01-01T00:00:00"`
	EndAt string `json:"end_at" swaggertype:"string" format:"date-time" example:"2023-01-01T00:00:00"`
	Type  uint8  `json:"type" swaggertype:"integer" format:"" example:"1"`
} // @name SearchUsrsResData

func (d *SearchUsrsResData) Of(usrs *[]model.Usr) *[]SearchUsrsResData {
	data := []SearchUsrsResData{}
	for _, u := range *usrs {
		aid := uint(0)
		vid := uint(0)
		if u.ApxID != nil {
			aid = *u.ApxID
		}
		if u.VdrID != nil {
			vid = *u.VdrID
		}
		data = append(data, SearchUsrsResData{
			ID:    u.ID,
			ApxID: aid,
			VdrID: vid,
			Name:  u.Name,
			Email: u.Email,
			BgnAt: common.ParseDatetimeToStr(&u.BgnAt),
			EndAt: common.ParseDatetimeToStr(&u.EndAt),
			Type:  u.Type,
		})
	}
	return &data
}

type SearchUsrsRes struct {
	Data   []SearchUsrsResData `json:"data"`
	Errors []Err               `json:"errors"`
} // @name SearchUsrsRes

type GetUsrResData struct {
	ID    uint   `json:"id" swaggertype:"integer" format:"" example:"1"`
	ApxID uint   `json:"apx_id" swaggertype:"integer" format:"" example:"1"`
	VdrID uint   `json:"vdr_id" swaggertype:"integer" format:"" example:"1"`
	Name  string `json:"name" swaggertype:"string" format:"" example:"User01"`
	Email string `json:"email" swaggertype:"string" format:"email" example:"sample@example.com"`
	BgnAt string `json:"bgn_at" swaggertype:"string" format:"date-time" example:"2023-01-01T00:00:00"`
	EndAt string `json:"end_at" swaggertype:"string" format:"date-time" example:"2023-01-01T00:00:00"`
	Type  uint8  `json:"type" swaggertype:"integer" format:"" example:"1"`
} // @name GetUsrResData

func (d *GetUsrResData) Of(u *model.Usr) *GetUsrResData {
	aid := uint(0)
	vid := uint(0)
	if u.ApxID != nil {
		aid = *u.ApxID
	}
	if u.VdrID != nil {
		vid = *u.VdrID
	}
	data := GetUsrResData{
		ID:    u.ID,
		ApxID: aid,
		VdrID: vid,
		Name:  u.Name,
		Email: u.Email,
		BgnAt: common.ParseDatetimeToStr(&u.BgnAt),
		EndAt: common.ParseDatetimeToStr(&u.EndAt),
		Type:  u.Type,
	}
	return &data
}

type GetUsrRes struct {
	Data   GetUsrResData `json:"data"`
	Errors []Err         `json:"errors"`
} // @name GetUsrRes

type CreateUsrResData struct {
	ID uint `json:"id"`
} // @name CreateUsrResData

type CreateUsrRes struct {
	Data   CreateUsrResData `json:"data"`
	Errors []Err            `json:"errors"`
} // @name CreateUsrRes

type UpdateUsrResData struct {
} // @name UpdateUsrResData

type UpdateUsrRes struct {
	Data   UpdateUsrResData `json:"data"`
	Errors []Err            `json:"errors"`
} // @name UpdateUsrRes

type DeleteUsrResData struct {
} // @name DeleteUsrResData

type DeleteUsrRes struct {
	Data   DeleteUsrResData `json:"data"`
	Errors []Err            `json:"errors"`
} // @name DeleteUsrRes

type HireUsrResData struct {
} // @name HireUsrResData

type HireUsrRes struct {
	Data   HireUsrResData `json:"data"`
	Errors []Err          `json:"errors"`
} // @name HireUsrRes

type DehireUsrResData struct {
} // @name DehireUsrResData

type DehireUsrRes struct {
	Data   DehireUsrResData `json:"data"`
	Errors []Err            `json:"errors"`
} // @name DehireUsrRes

type ActivateLnByUsrResData struct {
} // @name ActivateLnByUsrResData

type ActivateLnByUsrRes struct {
	Data   ActivateLnByUsrResData `json:"data"`
	Errors []Err                  `json:"errors"`
} // @name ActivateLnByUsrRes

type DeactivateLnByUsrResData struct {
} // @name DeactivateLnByUsrResData

type DeactivateLnByUsrRes struct {
	Data   DeactivateLnByUsrResData `json:"data"`
	Errors []Err                    `json:"errors"`
} // @name DeactivateLnByUsrRes

type KickLnByUsrResData struct {
} // @name KickLnByUsrResData

type KickLnByUsrRes struct {
	Data   KickLnByUsrResData `json:"data"`
	Errors []Err              `json:"errors"`
} // @name KickLnByUsrRes
