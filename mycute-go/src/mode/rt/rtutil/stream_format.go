package rtutil

import (
	"encoding/json"
	"fmt"
	"strings"
	"unicode"

	"github.com/t-kawata/mycute/mode/rt/rtres"
	"github.com/t-kawata/mycute/pkg/cuber/tools/query"
)

// FormatAbsorbResDataAsText は AbsorbCubeResData を読みやすいテキストに変換します。
func FormatAbsorbResDataAsText(data *rtres.AbsorbCubeResData, isEn bool) string {
	if isEn {
		return fmt.Sprintf(
			"Absorption completed.\n"+
				"- Input tokens used: %d\n"+
				"- Output tokens used: %d\n"+
				"- Remaining absorb limit: %s",
			data.InputTokens,
			data.OutputTokens,
			formatLimit(data.AbsorbLimit, isEn),
		)
	}
	return fmt.Sprintf(
		"吸収処理が完了しました。\n"+
			"- 使用した入力トークン数: %d\n"+
			"- 使用した出力トークン数: %d\n"+
			"- 残りの吸収可能回数: %s",
		data.InputTokens,
		data.OutputTokens,
		formatLimit(data.AbsorbLimit, isEn),
	)
}

// FormatQueryResDataAsText は QueryCubeResData を読みやすいテキストに変換します。
func FormatQueryResDataAsText(data *rtres.QueryCubeResData, isEn bool) string {
	var sb strings.Builder

	if isEn {
		sb.WriteString("Query completed.\n\n")

		if data.Answer != nil && *data.Answer != "" {
			sb.WriteString("**Answer:**\n")
			sb.WriteString(*data.Answer)
			sb.WriteString("\n\n")
		}

		if data.Chunks != nil && *data.Chunks != "" {
			sb.WriteString("**Retrieved Chunks:**\n")
			sb.WriteString(*data.Chunks)
			sb.WriteString("\n\n")
		}

		if data.Summaries != nil && *data.Summaries != "" {
			sb.WriteString("**Summaries:**\n")
			sb.WriteString(*data.Summaries)
			sb.WriteString("\n\n")
		}

		if data.Graph != nil && len(*data.Graph) > 0 {
			sb.WriteString(fmt.Sprintf("**Related Graph:** %d triples found:\n\n", len(*data.Graph)))
			graphText := &strings.Builder{}
			query.GenerateNaturalEnglishGraphExplanationByTriples(data.Graph, graphText)
			sb.WriteString(graphText.String())
			sb.WriteString("\n\n")
		}

		sb.WriteString(fmt.Sprintf("- Input tokens used: %d\n", data.InputTokens))
		sb.WriteString(fmt.Sprintf("- Output tokens used: %d\n", data.OutputTokens))
		sb.WriteString(fmt.Sprintf("- Remaining query limit: %s", formatLimit(data.QueryLimit, isEn)))
	} else {
		sb.WriteString("問い合わせが完了しました。\n\n")

		if data.Answer != nil && *data.Answer != "" {
			sb.WriteString("**回答:**\n")
			sb.WriteString(*data.Answer)
			sb.WriteString("\n\n")
		}

		if data.Chunks != nil && *data.Chunks != "" {
			sb.WriteString("**取得したチャンク:**\n")
			sb.WriteString(*data.Chunks)
			sb.WriteString("\n\n")
		}

		if data.Summaries != nil && *data.Summaries != "" {
			sb.WriteString("**要約:**\n")
			sb.WriteString(*data.Summaries)
			sb.WriteString("\n\n")
		}

		if data.Graph != nil && len(*data.Graph) > 0 {
			sb.WriteString(fmt.Sprintf("**関連するグラフ:** %d個のトリプルが見つかりました:\n\n", len(*data.Graph)))
			graphText := &strings.Builder{}
			query.GenerateNaturalJapaneseGraphExplanationByTriples(data.Graph, graphText)
			sb.WriteString(graphText.String())
			sb.WriteString("\n\n")
		}

		sb.WriteString(fmt.Sprintf("- 使用した入力トークン数: %d\n", data.InputTokens))
		sb.WriteString(fmt.Sprintf("- 使用した出力トークン数: %d\n", data.OutputTokens))
		sb.WriteString(fmt.Sprintf("- 残りのクエリ可能回数: %s", formatLimit(data.QueryLimit, isEn)))
	}

	return sb.String()
}

// FormatMemifyResDataAsText は MemifyCubeResData を読みやすいテキストに変換します。
func FormatMemifyResDataAsText(data *rtres.MemifyCubeResData, isEn bool) string {
	if isEn {
		return fmt.Sprintf(
			"Memify completed.\n"+
				"- Input tokens used: %d\n"+
				"- Output tokens used: %d\n"+
				"- Remaining memify limit: %s",
			data.InputTokens,
			data.OutputTokens,
			formatLimit(data.MemifyLimit, isEn),
		)
	}
	return fmt.Sprintf(
		"Memify処理が完了しました。\n"+
			"- 使用した入力トークン数: %d\n"+
			"- 使用した出力トークン数: %d\n"+
			"- 残りのMemify可能回数: %s",
		data.InputTokens,
		data.OutputTokens,
		formatLimit(data.MemifyLimit, isEn),
	)
}

// formatLimit は limit値を読みやすく変換します。
// 0=unlimited, -1=disabled, >0=remaining count
func formatLimit(limit int, isEn bool) string {
	if limit == 0 {
		if isEn {
			return "unlimited"
		}
		return "無制限"
	}
	if limit < 0 {
		if isEn {
			return "disabled"
		}
		return "利用不可"
	}
	if isEn {
		return fmt.Sprintf("%d remaining", limit)
	}
	return fmt.Sprintf("あと%d回", limit)
}

// FormatResDataAsJSON は任意のResDataをインデント付きJSONに変換します。
func FormatResDataAsJSON(data interface{}) (string, error) {
	b, err := json.MarshalIndent(data, "", "  ")
	if err != nil {
		return "", err
	}
	return string(b), nil
}

// TokenizeWithSpaceCollapse は文字列をトークン化しますが、連続するスペースは1つのトークンとして扱います。
// これにより、インデントされたJSONを送信する際に、スペースによる待ち時間を削減できます。
func TokenizeWithSpaceCollapse(s string, tokenSize int) []string {
	if len(s) == 0 {
		return nil
	}

	var tokens []string
	runes := []rune(s)
	i := 0

	for i < len(runes) {
		// 連続するスペース（ホワイトスペース）を検出
		if unicode.IsSpace(runes[i]) {
			start := i
			for i < len(runes) && unicode.IsSpace(runes[i]) {
				i++
			}
			// 連続するスペースを1トークンとして追加
			tokens = append(tokens, string(runes[start:i]))
		} else {
			// 非スペース文字をtokenSizeごとに分割
			start := i
			count := 0
			for i < len(runes) && !unicode.IsSpace(runes[i]) && count < tokenSize {
				i++
				count++
			}
			tokens = append(tokens, string(runes[start:i]))
		}
	}

	return tokens
}
