package rtparam

// CreateCubeParam はCube作成リクエストのパラメータです。
type CreateCubeParam struct {
	Name        string `json:"name" binding:"required" swaggertype:"string" format:"" example:"My Cube"`
	Description string `json:"description" swaggertype:"string" format:"" example:"Knowledge base for Go development"`
} // @name CreateCubeParam
