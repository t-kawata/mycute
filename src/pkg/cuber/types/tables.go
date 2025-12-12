package types

type TableName string

const (
	TABLE_NAME_DATA       TableName = "Data"
	TABLE_NAME_DOCUMENT   TableName = "Document"
	TABLE_NAME_CHUNK      TableName = "Chunk"
	TABLE_NAME_GRAPH_NODE TableName = "GraphNode"
	TABLE_NAME_ENTITY     TableName = "Entity"
	TABLE_NAME_SUMMARY    TableName = "Summary"
	TABLE_NAME_RULE       TableName = "Rule"
	TABLE_NAME_UNKNOWN    TableName = "Unknown"
	TABLE_NAME_CAPABILITY TableName = "Capability"
	// REL
	TABLE_NAME_GRAPH_EDGE TableName = "GraphEdge"
)
