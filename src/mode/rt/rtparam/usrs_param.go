package rtparam

import "time"

type SearchUsrParam struct {
	Name   string    `json:"name" swaggertype:"string" format:"" example:"User01"`
	Email  string    `json:"email" swaggertype:"string" format:"email" example:"sample@example.com"`
	BgnAt  time.Time `json:"bgn_at" swaggertype:"string" format:"date-time" example:"2023-01-01T00:00:00"`
	EndAt  time.Time `json:"end_at" swaggertype:"string" format:"date-time" example:"2100-12-31T23:59:59"`
	Limit  uint16    `json:"limit" swaggertype:"integer" format:"" example:"10"`
	Offset uint16    `json:"offset" swaggertype:"integer" format:"" example:"0"`
} // @name SearchUsrParam

type CreateUsrParam struct {
	Name     string    `json:"name" swaggertype:"string" format:"" example:"User01"`
	Email    string    `json:"email" swaggertype:"string" format:"email" example:"sample@example.com"`
	Password string    `json:"password" swaggertype:"string" format:"password" example:"ta5!CAzQz8DjMydju?"`
	BgnAt    time.Time `json:"bgn_at" swaggertype:"string" format:"date-time" example:"2023-01-01T00:00:00"`
	EndAt    time.Time `json:"end_at" swaggertype:"string" format:"date-time" example:"2100-12-31T23:59:59"`
	Type     uint8     `json:"type" swaggertype:"integer" format:"" example:"1"`
} // @name CreateUsrParam

type UpdateUsrParam struct {
	Name     string    `json:"name" swaggertype:"string" format:"" example:"User01"`
	Email    string    `json:"email" swaggertype:"string" format:"email" example:"sample@example.com"`
	Password string    `json:"password" swaggertype:"string" format:"password" example:"ta5!CAzQz8DjMydju?"`
	BgnAt    time.Time `json:"bgn_at" swaggertype:"string" format:"date-time" example:"2023-01-01T00:00:00"`
	EndAt    time.Time `json:"end_at" swaggertype:"string" format:"date-time" example:"2100-12-31T23:59:59"`
	Type     uint8     `json:"type" swaggertype:"integer" format:"" example:"1"`
} // @name UpdateUsrParam
