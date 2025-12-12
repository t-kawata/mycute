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
