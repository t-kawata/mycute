package rtres

type EmptyObj struct{} // @name EmptyObj

type Err struct {
	Field   string `json:"field" example:"field_name"`
	Code    uint16 `json:"code" example:"100000"`
	Message string `json:"message" example:"Some Error Message"`
} // @name Err

type ErrRes struct {
	Data   EmptyObj `json:"data"`
	Errors []Err    `json:"errors"`
} // @name ErrRes

type DummyRes struct {
	Data   EmptyObj `json:"data"`
	Errors []Err    `json:"errors"`
} // @name DummyRes

type Res struct {
	Errors []Err
}
