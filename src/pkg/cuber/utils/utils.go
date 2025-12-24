package utils

import (
	"strings"
	"unicode/utf8"

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

// EnsureFullGraphNodeID は、IDが既にメモリーグループのサフィックスを持っているかをチェックし、
// 持っていない場合のみサフィックスを追加したフルIDを返します。
// これにより、二重のサフィックス付加を防ぎ、安全にID復元を行うことができます。
func EnsureFullGraphNodeID(nodeID string, memoryGroup string) string {
	if strings.Contains(nodeID, consts.ID_MEMORY_GROUP_SEPARATOR) {
		// 既にセパレータを含んでいる場合はそのまま返す
		return nodeID
	}
	return MakeGraphNodeID(nodeID, memoryGroup)
}

// TruncateString は文字列を指定された文字数制限で切り詰めます。
// 制限を超える場合は切り詰めて「...」を付加します。
// 切り詰め後の最後の文字が「。」や「.」の場合は削除してから「...」を付加します。
func TruncateString(s string, limit int) string {
	// UTF-8での文字数（ルーン数）をカウント
	runeCount := utf8.RuneCountInString(s)
	// 制限以内ならそのまま返す
	if runeCount <= limit {
		return s
	}
	// []runeに変換して文字単位で切り詰め
	runes := []rune(s)
	truncated := runes[:limit]
	// 最後の文字が「。」または「.」の場合は削除
	if len(truncated) > 0 {
		lastChar := truncated[len(truncated)-1]
		if lastChar == '。' || lastChar == '.' {
			truncated = truncated[:len(truncated)-1]
		}
	}
	// 「...」を連結して返す
	return string(truncated) + "..."
}
