package rtres

type GenerateKeyHashResData struct {
	Hash string `json:"hash" swaggertype:"string" format:"" example:"???????????"`
} // @name GenerateKeyHashResData

type GenerateKeyHashRes struct {
	Data   GenerateKeyHashResData `json:"data"`
	Errors []Err                  `json:"errors"`
} // @name GenerateKeyHashRes

type CheckKeyHashResData struct {
	Result bool `json:"result" swaggertype:"boolean" format:"" example:"true"`
} // @name CheckKeyHashResData

type CheckKeyHashRes struct {
	Data   CheckKeyHashResData `json:"data"`
	Errors []Err               `json:"errors"`
} // @name CheckKeyHashRes
