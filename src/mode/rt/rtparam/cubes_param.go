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

type QueryCubeParam struct {
	CubeID      uint   `form:"cube_id" swaggertype:"integer" example:"1"`
	MemoryGroup string `form:"memory_group" swaggertype:"string" example:"legal_expert"`
	Text        string `form:"text" swaggertype:"string" example:"契約違反の場合の対処法は？"`
	QueryType   string `form:"query_type" swaggertype:"string" example:"GRAPH_COMPLETION"`
} // @name QueryCubeParam

type MemifyCubeParam struct {
	CubeID             uint   `json:"cube_id" swaggertype:"integer" example:"1"`
	MemoryGroup        string `json:"memory_group" swaggertype:"string" example:"legal_expert"`
	Epochs             int    `json:"epochs" swaggertype:"integer" example:"1"`
	PrioritizeUnknowns bool   `json:"prioritize_unknowns" swaggertype:"boolean" example:"true"`
} // @name MemifyCubeParam
