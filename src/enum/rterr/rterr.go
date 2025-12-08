package rterr

import (
	"fmt"

	"github.com/t-kawata/mycute/config"
)

type err struct {
	code uint16
	msg  string
}

var (
	ValidKey              = err{code: 0, msg: "Key must be valid one."}
	BadRequest            = err{code: 1, msg: "Bad Request."}
	Unauthorized          = err{code: 2, msg: "Failed to authenticate."}
	Forbidden             = err{code: 3, msg: "Forbidden."}
	NotFound              = err{code: 4, msg: "Not Found."}
	SystemError           = err{code: 5, msg: "System error."}
	InternalServerError   = err{code: 6, msg: "Internal Server Error."}
	Required              = err{code: 7, msg: "This field is required."}
	Number                = err{code: 8, msg: "This field must be a number."}
	Regexp                = err{code: 9, msg: "This field must be match the correct format: %s"}
	Email                 = err{code: 10, msg: "This field must be email format."}
	Min                   = err{code: 11, msg: "This field must be at least %s characters."}
	Max                   = err{code: 12, msg: "This field must be %s characters or less."}
	Password              = err{code: 13, msg: fmt.Sprintf("This field must be match the correct password format: %s", config.PW_REGEXP)}
	Half                  = err{code: 14, msg: "This field must be all single-byte characters."}
	Halfs                 = err{code: 15, msg: "This field must be an array consist of only items which are all single-byte characters."}
	Time                  = err{code: 16, msg: "This field must be time format like '23:59:59'."}
	Datetime              = err{code: 17, msg: "This field must be datetime format like '2023-01-01T23:59:59'."}
	UniqueByUsr           = err{code: 18, msg: "This field must be unique."}
	ValidUsr              = err{code: 19, msg: "This id is not matched with any valid user for you."}
	ValidLn               = err{code: 20, msg: "This id is not matched with any valid line for you."}
	ValidNotify           = err{code: 21, msg: "This id is not matched with any valid notify for you."}
	ValidRule             = err{code: 22, msg: "This id is not matched with any valid rule for you. The rule specified by the id may be not ready or currently being updated."}
	ValidRuleJson         = err{code: 23, msg: "This field must be correct rule json format. Any of the actions used in the rule json may be not ready or currently being updated."}
	ExistUsr              = err{code: 24, msg: "This id is not matched with any valid user."}
	Ipv4                  = err{code: 25, msg: "This field must be ipv4 format."}
	Ipv4AndPort           = err{code: 26, msg: "This field must be ipv4 and port format."}
	Ipv6                  = err{code: 27, msg: "This field must be ipv6 format."}
	Hostname              = err{code: 28, msg: "This field must be hostname format."}
	HostOrIpv4            = err{code: 29, msg: "This field must be in hostname or ipv4 format and the ':{port}' is optional."}
	MaskedIpv4            = err{code: 30, msg: "This field must be in masked ipv4 format like '10.1.0.0/16'."}
	HttpUrl               = err{code: 31, msg: "This field must be http url format."}
	HttpUrls              = err{code: 32, msg: "This field must be an array consist of only items which are http url format."}
	Oneof                 = err{code: 33, msg: "This field must match one of (%s)."}
	Gte                   = err{code: 34, msg: "This field must be greater than or equal to %s."}
	Lte                   = err{code: 35, msg: "This field must be less than or equal to %s."}
	Boolean               = err{code: 36, msg: "This field must be boolean format."}
	Uint8sGte             = err{code: 37, msg: "This field must be an array consist of only items which are greater than or equal to %s."}
	Uint16sGte            = err{code: 38, msg: "This field must be an array consist of only items which are greater than or equal to %s."}
	UintsGte              = err{code: 39, msg: "This field must be an array consist of only items which are greater than or equal to %s."}
	Uint8sLte             = err{code: 40, msg: "This field must be an array consist of only items which are less than or equal to %s."}
	Uint16sLte            = err{code: 41, msg: "This field must be an array consist of only items which are less than or equal to %s."}
	UintsLte              = err{code: 42, msg: "This field must be an array consist of only items which are less than or equal to %s."}
	NotEmptyStrArr        = err{code: 43, msg: "This field must not be an empty array."}
	UniqueExtenNumber     = err{code: 44, msg: "This number is not unique for this user."}
	ValidRegex            = err{code: 45, msg: "This field must be a valid regex."}
	ValidDetailedRuleJson = err{code: 46, msg: "This field must be correct detailed-rule-json format."}
	ValidCttsHostIdent    = err{code: 47, msg: "This field must be a valid pair of ctts_host and ctts_ident."}
)

func (e *err) Code() uint16 {
	return e.code
}

func (e *err) Msg() string {
	return e.msg
}
