package event

import (
	"unicode/utf8"
)

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
