package rtparam

type CreateCubeParam struct {
	Name        string `json:"name" swaggertype:"string" format:"" example:"My Cube"`
	Description string `json:"description" swaggertype:"string" format:"" example:"Knowledge base for Go development"`
} // @name CreateCubeParam

type AbsorbCubeParam struct {
	CubeID      uint   `json:"cube_id" swaggertype:"integer" format:"" example:"1"`
	MemoryGroup string `json:"memory_group" swaggertype:"string" format:"" example:"legal_expert"`
	Content     string `json:"content" swaggertype:"string" format:"" example:"Knowledge base for Go development"`
} // @name AbsorbCubeParam

type ReKeyCubeParam struct {
	CubeID uint   `json:"cube_id" swaggertype:"integer" format:"" example:"1"`
	Key    string `json:"key" swaggertype:"string" format:"" example:"alknas38msd..."`
} // @name ReKeyCubeParam
