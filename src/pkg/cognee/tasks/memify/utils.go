package memify

import "unicode/utf8"

// CountUTF8Chars は、文字列のUTF-8文字数（rune数）を返します。
// len(string) はバイト数を返すため、この関数を使用してください。
//
// 例:
//   - "Hello" → 5 (バイト数も5)
//   - "こんにちは" → 5 (バイト数は15)
//   - "日本語テキスト" → 7 (バイト数は21)
//
// この関数は Memify のハイブリッド処理判定に使用されます。
// MemifyMaxCharsForBulkProcess との比較には必ずこの関数を使用すること。
func CountUTF8Chars(s string) int {
	return utf8.RuneCountInString(s)
}

// CountTotalUTF8Chars は、文字列スライスの合計UTF-8文字数を返します。
func CountTotalUTF8Chars(texts []string) int {
	total := 0
	for _, text := range texts {
		total += utf8.RuneCountInString(text)
	}
	return total
}
