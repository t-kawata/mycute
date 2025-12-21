package rtparam

type SearchCubesParam struct {
	Name        string `json:"name" swaggertype:"string" example:"Legal Bot"`
	Description string `json:"description" swaggertype:"string" example:"Legal advisor"`
	Limit       uint16 `json:"limit" swaggertype:"integer" example:"20"`
	Offset      uint16 `json:"offset" swaggertype:"integer" example:"0"`
} // @name SearchCubesParam

type CreateCubeParam struct {
	Name               string `json:"name" swaggertype:"string" format:"" example:"My Cube"`
	Description        string `json:"description" swaggertype:"string" format:"" example:"Knowledge base for Go development"`
	EmbeddingProvider  string `json:"embedding_provider" swaggertype:"string" example:"openai"`
	EmbeddingModel     string `json:"embedding_model" swaggertype:"string" example:"text-embedding-3-small"`
	EmbeddingDimension uint   `json:"embedding_dimension" swaggertype:"integer" example:"1536"`
	EmbeddingBaseURL   string `json:"embedding_base_url" swaggertype:"string" example:"https://api.openai.com/v1"`
	EmbeddingApiKey    string `json:"embedding_api_key" swaggertype:"string" example:"sk-proj-..."`
} // @name CreateCubeParam

type ImportCubeParam struct {
	Name            string `form:"name" example:"my-restored-cube"`
	Description     string `form:"description" example:"Restored from backup"`
	Key             string `form:"key" example:"(Base64EncodedKeyString)"`
	EmbeddingApiKey string `form:"embedding_api_key" example:"sk-..."`
} // @name ImportCubeParam

type AbsorbCubeParam struct {
	CubeID       uint   `json:"cube_id" swaggertype:"integer" format:"" example:"1"`
	MemoryGroup  string `json:"memory_group" swaggertype:"string" format:"" example:"legal_expert"`
	Content      string `json:"content" swaggertype:"string" format:"" example:"Knowledge base for Go development"`
	ChunkSize    int    `json:"chunk_size" swaggertype:"integer" format:"" example:"512"`
	ChunkOverlap int    `json:"chunk_overlap" swaggertype:"integer" format:"" example:"16"`
	ChatModelID  uint   `json:"chat_model_id" swaggertype:"integer" format:"" example:"1"`
	Stream       bool   `json:"stream" swaggertype:"boolean" format:"" example:"false"`
	IsEn         bool   `json:"is_en" swaggertype:"boolean" format:"" example:"false"`
} // @name AbsorbCubeParam

type ReKeyCubeParam struct {
	CubeID uint   `json:"cube_id" swaggertype:"integer" format:"" example:"1"`
	Key    string `json:"key" swaggertype:"string" format:"" example:"alknas38msd..."`
} // @name ReKeyCubeParam

type QueryCubeParam struct {
	CubeID      uint   `form:"cube_id" swaggertype:"integer" example:"1"`
	MemoryGroup string `form:"memory_group" swaggertype:"string" example:"legal_expert"`
	Text        string `form:"text" swaggertype:"string" example:"契約違反の場合の対処法は？"`
	Type        uint8  `form:"type" swaggertype:"integer" example:"1"`
	SummaryTopk int    `form:"summary_topk" swaggertype:"integer" example:"3"`
	ChunkTopk   int    `form:"chunk_topk" swaggertype:"integer" example:"3"`
	EntityTopk  int    `form:"entity_topk" swaggertype:"integer" example:"3"`
	FtsType     uint8  `form:"fts_type" swaggertype:"integer" example:"0"` // 0=nouns, 1=nouns_verbs, 2=all
	FtsTopk     int    `form:"fts_topk" swaggertype:"integer" example:"0"` // 0=disabled
	ChatModelID uint   `form:"chat_model_id" swaggertype:"integer" example:"1"`
	Stream      bool   `form:"stream" swaggertype:"boolean" example:"false"`
	IsEn        bool   `form:"is_en" swaggertype:"boolean" example:"false"`
} // @name QueryCubeParam

type MemifyCubeParam struct {
	CubeID             uint   `json:"cube_id" swaggertype:"integer" example:"1"`
	MemoryGroup        string `json:"memory_group" swaggertype:"string" example:"legal_expert"`
	Epochs             int    `json:"epochs" swaggertype:"integer" example:"1"`
	PrioritizeUnknowns bool   `json:"prioritize_unknowns" swaggertype:"boolean" example:"true"`
	ChatModelID        uint   `json:"chat_model_id" swaggertype:"integer" example:"1"`
	Stream             bool   `json:"stream" swaggertype:"boolean" example:"false"`
	IsEn               bool   `json:"is_en" swaggertype:"boolean" example:"false"`
} // @name MemifyCubeParam
