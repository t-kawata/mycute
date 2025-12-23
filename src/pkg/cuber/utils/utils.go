package utils

import (
	"strings"

	"github.com/t-kawata/mycute/pkg/cuber/consts"
)

func GetNameStrByGraphNodeID(graphNodeID string) string {
	ex := strings.Split(graphNodeID, consts.ID_MEMORY_GROUP_SEPARATOR)
	if len(ex) > 1 {
		return ex[0]
	}
	return graphNodeID
}

// MakeGraphNodeID は、ノードIDとメモリーグループを連結して、GraphNodeテーブルのid形式を生成します。
// これは GetNameStrByGraphNodeID の逆操作です。
func MakeGraphNodeID(nodeID string, memoryGroup string) string {
	return strings.TrimSpace(nodeID) + consts.ID_MEMORY_GROUP_SEPARATOR + memoryGroup
}
