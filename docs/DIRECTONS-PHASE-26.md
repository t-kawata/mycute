# フェーズ 26: 統合テキスト正規化戦略

このディレクティブは、Cuberパイプライン（Absorb, Query, Memify, Metacognition）全体における統合テキスト正規化戦略の実施について概説します。適切な正規化により、取り込まれたデータとユーザーのクエリの一貫性が確保され、検索精度と知識グラフの整合性が大幅に向上します。

## 正規化戦略の概要

異なる検索メカニズムに合わせて、3つの異なる正規化レベルを実装します。すべての処理において、最初に `CommonNormalize` によるクリーンアップ（HTML除去、Boilerplate削除）が適用されます。

| 正規化タイプ | 目的 | 主な原則 | 対象コンポーネント |
| :--- | :--- | :--- | :--- |
| **Vector** | セマンティック検索 | 意味の重みを保持。NFKCで正規化し、制御文字を除去。 | `ChunkingTask`, `MemifyTask`, `Metacognition` (登録時) |
| **Graph** | エンティティ解決 | 決定論的マッピング。幅統一(Fold)、小文字化、記号・絵文字除去。 | `GraphExtractionTask` (Node ID/Label) |
| **Search** | 再現率の最大化 | キーワードヒット最大化。半角カナ全角化、小文字化、記号・絵文字・Markdown除去。 | `GraphCompletionTool` (FTS), `Metacognition` (検索時) |

---

## 実装詳細: `src/pkg/cuber/utils/normalize.go`

正規化ロジックの「信頼できる唯一の情報源（Single Source of Truth）」として、以下の完全な実装コードを `normalize.go` に記述します。簡略化や省略は一切行いません。

```go
package utils

import (
	"bytes"
	"fmt"
	"regexp"
	"strings"
	"unicode"

	"github.com/PuerkitoBio/goquery"
	"github.com/go-shiori/go-readability"
	"github.com/microcosm-cc/bluemonday"
	"golang.org/x/net/html"
	"golang.org/x/text/unicode/norm"
	"golang.org/x/text/width"
    "golang.org/x/text/transform"
    
    // Markdown conversion
	md "github.com/JohannesKaufmann/html-to-markdown"
    // Markdown parsing for text extraction
    "github.com/yuin/goldmark"
)

var (
	// HTML 関連
	scriptStyleRe   = regexp.MustCompile(`(?is)<(script|style)[^>]*?>.*?</\1>`)
	commentRe       = regexp.MustCompile(`(?s)<!--.*?-->`)
	tagRe           = regexp.MustCompile(`<[^>]+>`)
	numericEntityRe = regexp.MustCompile(`&#x?[0-9a-fA-F]+;`)

	// Markdown 関連
	codeBlockRe      = regexp.MustCompile("(?s)``````")
	inlineCodeRe     = regexp.MustCompile("`([^`]+)`")
	linkRe           = regexp.MustCompile(`\[([^\]]+)\]\([^\)]+\)`)
	imageRe          = regexp.MustCompile(`!\[([^\]]*)\]\([^\)]+\)`)
	headingRe        = regexp.MustCompile(`(?m)^#+\s+`)
	listRe           = regexp.MustCompile(`(?m)^[\*\-\+]\s+`)
	numberedListRe   = regexp.MustCompile(`(?m)^\d+\.\s+`)
	quoteRe          = regexp.MustCompile(`(?m)^>\s*`)
	emphasisBoldRe   = regexp.MustCompile(`(\*\*|__)(.*?)\1`)
	emphasisItalicRe = regexp.MustCompile(`(\*|_)(.*?)\1`)
	hrRe             = regexp.MustCompile(`(?m)^([-*_])\1{2,}$`)

	// Boilerplate 関連（英語）
	boilerplateEnPatterns = []*regexp.Regexp{
		regexp.MustCompile(`(?i)(copyright|©)\s*(\d{4}(-\d{4})?|present).*?(all\s+rights\s+reserved|inc\.|ltd\.|llc)?`),
		regexp.MustCompile(`(?i)(privacy\s+(policy|statement|notice)|terms\s+(of\s+(service|use)|and\s+conditions)|cookie\s+policy|legal\s+notice).*`),
		regexp.MustCompile(`(?i)all\s+rights\s+reserved\.?`),
		regexp.MustCompile(`(?i)powered\s+by\s+\w+`),
		regexp.MustCompile(`(?i)(designed|developed)\s+by\s+[\w\s]+`),
		regexp.MustCompile(`(?i)(home|about(\s+us)?|contact(\s+us)?|services|products|blog|news|faq|sitemap|careers|support)\s*(\||\/|•|·|\n)`),
		regexp.MustCompile(`(?i)(share\s+(on|this|via)?|follow\s+us(\s+on)?|connect\s+with\s+us).*?(facebook|twitter|instagram|linkedin|youtube|pinterest|tiktok)`),
		regexp.MustCompile(`(?i)(like|tweet|pin|share)(\s+this)?(\s+post)?`),
		regexp.MustCompile(`(?i)(leave\s+a\s+)?(comment|reply|subscribe|sign\s+up|join|newsletter)`),
		regexp.MustCompile(`(?i)enter\s+your\s+email(\s+address)?`),
		regexp.MustCompile(`(?i)(related\s+(posts?|articles?|content)|you\s+may\s+also\s+like|recommended\s+for\s+you|more\s+from)`),
		regexp.MustCompile(`(?i)(previous|next|older|newer)\s+(post|article|page)`),
		regexp.MustCompile(`(?i)page\s+\d+\s+of\s+\d+`),
		regexp.MustCompile(`(?i)(first|last|prev|next)\s*(\||»|«|›|‹)`),
		regexp.MustCompile(`(?i)home\s*(»|›|>|\/)\s*[\w\s]+(»|›|>|\/)`),
		regexp.MustCompile(`(?i)(advertisement|sponsored(\s+content)?|ad|promoted|affiliate\s+links?)`),
		regexp.MustCompile(`(?i)(read\s+more|continue\s+reading|full\s+article|view\s+all)`),
		regexp.MustCompile(`(?i)(posted|published|updated|last\s+modified)\s+(on|at|in|:)`),
		regexp.MustCompile(`(?i)(by|author|written\s+by):\s*[\w\s]+`),
		regexp.MustCompile(`(?i)(\d+\s+)?(min|minute|hour|day|week|month|year)s?\s+(ago|read)`),
		regexp.MustCompile(`(?i)(tags?|categories|filed\s+under):\s*[\w\s,]+`),
	}

	// Boilerplate 関連（日本語）
	boilerplateJaPatterns = []*regexp.Regexp{
		regexp.MustCompile(`(?i)copyright\s*©?\s*\d{4}(-\d{4})?.*?(株式会社|有限会社|合同会社)?`),
		regexp.MustCompile(`©?\s*\d{4}(-\d{4})?\s*(株式会社|有限会社|合同会社|Inc\.|Ltd\.|LLC)[\w\s]*`),
		regexp.MustCompile(`(著作権|版権).*?(株式会社|有限会社|合同会社)?[\w\s]*`),
		regexp.MustCompile(`無断(転載|複製|使用|引用)(禁止|厳禁|を禁じます|はご遠慮ください)`),
		regexp.MustCompile(`(すべて|全て)の(権利|著作権)を?(保有|所有)(します|しています|しております)`),
		regexp.MustCompile(`(プライバシー|個人情報保護)(ポリシー|方針|規約)`),
		regexp.MustCompile(`(利用|ご利用|サービス利用)(規約|約款|条件)`),
		regexp.MustCompile(`(特定商取引法|特商法)(に基づく表記|表記)`),
		regexp.MustCompile(`(免責事項|お問い合わせ|会社概要|サイトマップ)`),
		regexp.MustCompile(`(ホーム|トップ|会社概要|事業内容|サービス|製品|お問い?合わせ|採用情報|ニュース|ブログ|よくある質問|FAQ)\s*(\||／|・|｜)`),
		regexp.MustCompile(`(シェア|共有|フォロー|いいね|ツイート)(する|して|ください|してください|ボタン)?.*?(Facebook|Twitter|Instagram|LINE|YouTube)`),
		regexp.MustCompile(`(SNS|ソーシャルメディア)で(シェア|共有|フォロー)`),
		regexp.MustCompile(`(コメント|返信)(を|する|して|ください|を残す)`),
		regexp.MustCompile(`(メルマガ|メールマガジン|ニュースレター)(登録|購読|に登録)`),
		regexp.MustCompile(`(会員|アカウント)(登録|作成|お申し込み)`),
		regexp.MustCompile(`(関連|おすすめ|人気|最新|こちらもどうぞ)(記事|投稿|コンテンツ|ページ)`),
		regexp.MustCompile(`(前|次|前の|次の)(記事|投稿|ページ)`),
		regexp.MustCompile(`(もっと|さらに|続きを|全文を)(見る|読む|表示)`),
		regexp.MustCompile(`(ページ|Page)\s*\d+\s*(\/|of)\s*\d+`),
		regexp.MustCompile(`(最初|最後|前|次|前へ|次へ)\s*(\||»|«|›|‹)`),
		regexp.MustCompile(`(ホーム|トップ)\s*(»|›|>|＞)\s*[\w\s]+(»|›|>|＞)`),
		regexp.MustCompile(`(広告|PR|スポンサー|提供|協賛|アフィリエイト)`),
		regexp.MustCompile(`(投稿|公開|更新|最終更新|掲載)(日|日時|時刻)?\s*[:：]\s*\d{4}`),
		regexp.MustCompile(`(著者|執筆者|ライター|投稿者)\s*[:：]\s*[\w\s]+`),
		regexp.MustCompile(`\d+\s*(分|時間|日|週間|ヶ月|年)(前|で読めます)`),
		regexp.MustCompile(`(タグ|カテゴリー?|分類)\s*[:：]\s*[\w\s、,]+`),
		regexp.MustCompile(`(この記事|本記事)を?(シェア|共有)`),
	}

    // HTML エンティティマッピング
    htmlEntities = map[string]string{
        "&lt;":     "<",
        "&gt;":     ">",
        "&amp;":    "&",
        "&quot;":   "\"",
        "&apos;":   "'",
        "&nbsp;":   " ",
        "&ndash;":  "–",
        "&mdash;":  "—",
        "&hellip;": "…",
        "&copy;":   "©",
        "&reg;":    "®",
        "&trade;":  "™",
    }
    
    // 半角カナマッピング
    halfWidthKanaMap = map[rune]string{
        'ｱ': "ア", 'ｲ': "イ", 'ｳ': "ウ", 'ｴ': "エ", 'ｵ': "オ",
        'ｶ': "カ", 'ｷ': "キ", 'ｸ': "ク", 'ｹ': "ケ", 'ｺ': "コ",
        'ｻ': "サ", 'ｼ': "シ", 'ｽ': "ス", 'ｾ': "セ", 'ｿ': "ソ",
        'ﾀ': "タ", 'ﾁ': "チ", 'ﾂ': "ツ", 'ﾃ': "テ", 'ﾄ': "ト",
        'ﾅ': "ナ", 'ﾆ': "ニ", 'ﾇ': "ヌ", 'ﾈ': "ネ", 'ﾉ': "ノ",
        'ﾊ': "ハ", 'ﾋ': "ヒ", 'ﾌ': "フ", 'ﾍ': "ヘ", 'ﾎ': "ホ",
        'ﾏ': "マ", 'ﾐ': "ミ", 'ﾑ': "ム", 'ﾒ': "メ", 'ﾓ': "モ",
        'ﾔ': "ヤ", 'ﾕ': "ユ", 'ﾖ': "ヨ",
        'ﾗ': "ラ", 'ﾘ': "リ", 'ﾙ': "ル", 'ﾚ': "レ", 'ﾛ': "ロ",
        'ﾜ': "ワ", 'ｦ': "ヲ", 'ﾝ': "ン",
        'ｧ': "ァ", 'ｨ': "ィ", 'ｩ': "ゥ", 'ｪ': "ェ", 'ｫ': "ォ",
        'ｬ': "ャ", 'ｭ': "ュ", 'ｮ': "ョ", 'ｯ': "ッ",
        'ｰ': "ー", '｡': "。", '｢': "「", '｣': "」", '､': "、", '･': "・",
        'ﾞ': "゛", 'ﾟ': "゜",
    }
    
    // 濁点マッピング
    dakutenMap = map[string]string{
        "カ": "ガ", "キ": "ギ", "ク": "グ", "ケ": "ゲ", "コ": "ゴ",
        "サ": "ザ", "シ": "ジ", "ス": "ズ", "セ": "ゼ", "ソ": "ゾ",
        "タ": "ダ", "チ": "ヂ", "ツ": "ヅ", "テ": "デ", "ト": "ド",
        "ハ": "バ", "ヒ": "ビ", "フ": "ブ", "ヘ": "ベ", "ホ": "ボ",
        "ウ": "ヴ",
    }
    
    // 半濁点マッピング
    handakutenMap = map[string]string{
        "ハ": "パ", "ヒ": "ピ", "フ": "プ", "ヘ": "ペ", "ホ": "ポ",
    }

    // 空白・改行関連
	consecutiveSpacesRe   = regexp.MustCompile(`[ \t]+`)
	consecutiveNewlinesRe = regexp.MustCompile(`\n{3,}`)
	trailingSpacesRe      = regexp.MustCompile(`[ \t]+\n`)

	// 絵文字関連
	emojiRe = regexp.MustCompile(`[\x{1F600}-\x{1F64F}]|[\x{1F300}-\x{1F5FF}]|[\x{1F680}-\x{1F6FF}]|[\x{1F700}-\x{1F77F}]|[\x{1F780}-\x{1F7FF}]|[\x{1F800}-\x{1F8FF}]|[\x{1F900}-\x{1F9FF}]|[\x{1FA00}-\x{1FA6F}]|[\x{1FA70}-\x{1FAFF}]|[\x{2600}-\x{26FF}]|[\x{2700}-\x{27BF}]|[\x{FE00}-\x{FE0F}]|[\x{1F1E0}-\x{1F1FF}]`)
    
    // 制御文字
    reControl = regexp.MustCompile(`[\x00-\x1F\x7F-\x9F\xAD]`)
    // 特殊記号（Graph用）
    reSymbols = regexp.MustCompile(`[\x{1F300}-\x{1F9FF}\x{2600}-\x{26FF}\x{2700}-\x{27BF}]`)
)

// =================================================================================
// 0. 共通処理 (Common Processing)
// =================================================================================

// CommonNormalize は、入力テキストから HTML/Markdown 記法を除去しノイズをクリーニングします。
func CommonNormalize(text string) (string, error) {
    isHTML := detectHTML(text)
    var extractedText string
    var err error

    if isHTML {
        extractedText, err = extractAndCleanHTML(text)
    } else {
        // Markdownとして扱う（プレーンテキストも安全に処理可能）
        extractedText, err = extractFromMarkdown(text)
    }

	if err != nil {
		if extractedText == "" {
            extractedText = text
        }
	}
	return normalizeWhitespace(extractedText), nil
}

func detectHTML(text string) bool {
    checkLen := len(text)
    if checkLen > 1000 { checkLen = 1000 }
    header := text[:checkLen]
    hLower := strings.ToLower(header)
    if strings.Contains(hLower, "<!doctype html") || 
       strings.Contains(hLower, "<html") ||
       strings.Contains(hLower, "<head") ||
       strings.Contains(hLower, "<body") {
        return true
    }
    return tagRe.MatchString(header)
}

func extractAndCleanHTML(text string) (string, error) {
    cleanHTML, _ := pruneHTMLBoilerplate(text) 
    markdown, err := convertHTMLToMarkdown(cleanHTML)
    if err != nil {
        return extractFromHTML(text), nil
    }
    return extractFromMarkdown(markdown)
}

func pruneHTMLBoilerplate(htmlText string) (string, error) {
	article, err := readability.FromReader(strings.NewReader(htmlText), nil)
	if err == nil && article != nil {
		return article.Content, nil
	}
	return htmlText, nil 
}

func convertHTMLToMarkdown(htmlText string) (string, error) {
	converter := md.NewConverter("", true, nil)
	return converter.ConvertString(htmlText)
}

func extractFromHTML(text string) string {
    text = scriptStyleRe.ReplaceAllString(text, "")
    text = commentRe.ReplaceAllString(text, "")
    text = tagRe.ReplaceAllString(text, " ")
    return decodeHTMLEntities(text)
}

func decodeHTMLEntities(text string) string {
    for entity, char := range htmlEntities {
        text = strings.ReplaceAll(text, entity, char)
    }
    return numericEntityRe.ReplaceAllStringFunc(text, func(match string) string {
        var code int
        if strings.HasPrefix(match, "&#x") || strings.HasPrefix(match, "&#X") {
            fmt.Sscanf(match, "&#x%x;", &code)
        } else {
            fmt.Sscanf(match, "&#%d;", &code)
        }
        if code > 0 && code < 0x10FFFF { return string(rune(code)) }
        return ""
    })
}

func extractFromMarkdown(text string) (string, error) {
    // 構造を維持しつつ記号を除去
    text = codeBlockRe.ReplaceAllString(text, "")
    text = inlineCodeRe.ReplaceAllString(text, "$1")
    text = linkRe.ReplaceAllString(text, "$1")
    text = imageRe.ReplaceAllString(text, "$1")
    text = headingRe.ReplaceAllString(text, "")
    text = listRe.ReplaceAllString(text, "")
    text = numberedListRe.ReplaceAllString(text, "")
    text = quoteRe.ReplaceAllString(text, "")
    text = emphasisBoldRe.ReplaceAllString(text, "$2")
    text = emphasisItalicRe.ReplaceAllString(text, "$2")
    text = hrRe.ReplaceAllString(text, "")
    
    // Boilerplate 除去
    text = removeBoilerplate(text)
	return text, nil
}

func removeBoilerplate(text string) string {
    for _, re := range boilerplateEnPatterns {
        text = re.ReplaceAllString(text, "")
    }
    for _, re := range boilerplateJaPatterns {
        text = re.ReplaceAllString(text, "")
    }

	lines := strings.Split(text, "\n")
	var cleanLines []string
	for _, line := range lines {
		trim := strings.TrimSpace(line)
        // 10文字を超えるか、空白を含む3文字以上の行を意味のある行とみなす
		if len([]rune(trim)) > 10 || (len([]rune(trim)) > 3 && strings.Contains(trim, " ")) {
            cleanLines = append(cleanLines, line)
		}
	}
	return strings.Join(cleanLines, "\n")
}

func normalizeWhitespace(text string) string {
	text = strings.ReplaceAll(text, "\r\n", "\n")
	text = strings.ReplaceAll(text, "\r", "\n")
	text = consecutiveSpacesRe.ReplaceAllString(text, " ")
	text = consecutiveNewlinesRe.ReplaceAllString(text, "\n\n")
	text = trailingSpacesRe.ReplaceAllString(text, "\n")
	return strings.TrimSpace(text)
}

// =================================================================================
// 1. 各用途向けの正規化 (Vector / Graph / Search)
// =================================================================================

// NormalizeForVector は意味を保持しつつノイズを除去します。
func NormalizeForVector(text string) string {
	if text == "" { return "" }
	text = norm.NFKC.String(text)
	text = reControl.ReplaceAllString(text, "")
	text = consecutiveSpacesRe.ReplaceAllString(text, " ")
	return strings.TrimSpace(text)
}

// NormalizeForGraph は決定論的なエンティティ解決を確保します。
func NormalizeForGraph(text string) string {
	if text == "" { return "" }
	text = norm.NFKC.String(text)
	text = transformWidth(text, width.Fold)
	text = strings.ToLower(text)
	text = reSymbols.ReplaceAllString(text, "")
    text = emojiRe.ReplaceAllString(text, "")
	text = consecutiveSpacesRe.ReplaceAllString(text, " ")
	return strings.TrimSpace(text)
}

// NormalizeForSearch は全文検索（FTS）の精度を最大化します。
func NormalizeForSearch(text string) string {
	if text == "" { return "" }
	text = norm.NFKC.String(text)
    text = transformWidth(text, width.Fold)
    text = convertHalfWidthKanaToFullWidth(text)
	text = strings.ToLower(text)
	text = reControl.ReplaceAllString(text, "")
	text = reSymbols.ReplaceAllString(text, "")
    text = emojiRe.ReplaceAllString(text, "")
	text = consecutiveSpacesRe.ReplaceAllString(text, " ")
	return strings.TrimSpace(text)
}

func transformWidth(text string, t transform.Transformer) string {
    res, _, _ := transform.String(t, text)
    return res
}

func convertHalfWidthKanaToFullWidth(text string) string {
    var buf strings.Builder
    runes := []rune(text)
    for i := 0; i < len(runes); i++ {
        r := runes[i]
        if replacement, ok := halfWidthKanaMap[r]; ok {
            if i+1 < len(runes) {
                next := runes[i+1]
                if next == 'ﾞ' { 
                    buf.WriteString(addDakuten(replacement)); i++; continue
                } else if next == 'ﾟ' {
                    buf.WriteString(addHandakuten(replacement)); i++; continue
                }
            }
            buf.WriteString(replacement)
        } else {
            buf.WriteRune(r)
        }
    }
    return buf.String()
}

func addDakuten(s string) string {
    if res, ok := dakutenMap[s]; ok { return res }
    return s
}

func addHandakuten(s string) string {
    if res, ok := handakutenMap[s]; ok { return res }
    return s
}
```

---

## 統合プラン

### 1. Absorb / Ingest パイプライン

#### [MODIFY] `src/pkg/cuber/tasks/chunking/chunking_task.go`
チャンク分割前の**生テキスト**に対して、共通正規化とVector用正規化を順に適用します。
**重要:** 正規化は `SaveDocument` を呼び出す**前**に行い、Documentテーブルにも正規化されたテキストが保存されるようにします（これまではRawテキストが保存されていました）。

```go
func (t *ChunkingTask) Run(ctx context.Context, input any) (any, types.TokenUsage, error) {
    // ... dataList ループ内
    text := string(content)
    
    // 1. 共通正規化 (HTML除去、Boilerplate削除)
    cleaned, err := utils.CommonNormalize(text)
    if err != nil { 
        utils.LogWarn(t.Logger, "CommonNormalize failed", zap.Error(err))
        cleaned = text 
    }
    
    // 2. Vector用正規化
    text = utils.NormalizeForVector(cleaned)

    // 3. Document保存 (正規化済みテキストを使用)
    doc := &storage.Document{
        // ...
        Text: text, 
    }
    if err := t.VectorStorage.SaveDocument(ctx, doc); err != nil {
        // ...
    }

    // 4. 以降、正規化済み text を用いてチャンク分割と埋め込みを行う
}
```

#### [MODIFY] `src/pkg/cuber/tasks/summarization/summarization_task.go`
生成された**要約テキスト**に対して、埋め込みと保存の前にVector用正規化を適用します。

```go
func (t *SummarizationTask) Run(ctx context.Context, input any) (...) {
    // ... summaryText 生成後
    // 3. Vector用正規化
    summaryText = utils.NormalizeForVector(summaryText)
    
    // ... 以降、embedding生成と保存に使用
}
```

#### [MODIFY] `src/pkg/cuber/tasks/graph/graph_extraction_task.go`
抽出された**ノードIDとラベル**、および**プロパティ内の文字列値**に対して正規化を適用します。これにより、同一エンティティの統合に加え、検索時のプロパティマッチング精度も向上させます。

```go
// Run メソッド内、LLMパース後
for i := range graphData.Nodes {
    node := &graphData.Nodes[i]
    // IDを正規化 (小文字化、幅統一、記号除去)
    node.ID = utils.NormalizeForGraph(node.ID)
    // ラベル/タイプも同様にクリーンアップ
    node.Type = utils.NormalizeForGraph(node.Type)

    // [NEW] プロパティ内の文字列も正規化 (CommonNormalize推奨)
    for k, v := range node.Properties {
        if strVal, ok := v.(string); ok {
            normalizedVal, _ := utils.CommonNormalize(strVal)
            node.Properties[k] = normalizedVal
        }
    }
    
    // IDの一意性確保ロジック
    node.ID = fmt.Sprintf("%s%s%s", node.ID, consts.ID_MEMORY_GROUP_SEPARATOR, t.MemoryGroup)
}
// エッジについても同様に Type と Properties を正規化
```

#### [MODIFY] `src/pkg/cuber/tasks/storage/storage_task.go`
Entityテーブルへの保存時、**エンティティ名（`node.Properties["name"]`）**に対して、埋め込み生成前にVector用正規化を適用します。これにより、検索時のクエリ正規化と整合性を取ります。

```go
func (t *StorageTask) Run(ctx context.Context, input any) (...) {
    // ...
    // エンティティ名のembeddingを生成部分
    name := node.Properties["name"]
    // 正規化
    normalizedName := utils.NormalizeForVector(name)
    
    // 正規化された名前でEmbedding生成
    embedding, u, err := t.Embedder.EmbedQuery(ctx, normalizedName)
    // ...
    // 保存時は元のnameを使うか正規化後を使うか要検討だが、
    // Entityテーブルのtextカラムは検索対象なので正規化後が望ましい
    t.VectorStorage.SaveEmbedding(..., normalizedName, embedding, ...)
}
```

### 2. Query パイプライン

#### [MODIFY] `src/pkg/cuber/tools/query/graph_completion.go`
ユーザーから入力された**クエリ文字列**に対して、検索用正規化を適用します。FTSと埋め込みベクトルの両方で同じ正規化クエリを使用することが重要です。

```go
func (t *GraphCompletionTool) Query(ctx context.Context, query string, config types.QueryConfig) (...) {
    // 検索精度のための正規化 (半角カナ変換等を含む)
    normalizedQuery := utils.NormalizeForSearch(query)
    // 以降、FTS検索および Vector検索用の埋め込み生成に normalizedQuery を使用
}

func (t *GraphCompletionTool) getGraph(ctx context.Context, entityTopk int, query string, ...) {
    normQuery := utils.NormalizeForSearch(query)
    // ...
}
```

### 3. Memify パイプライン

#### [MODIFY] `src/pkg/cuber/tasks/memify/rule_extraction_task.go`
LLMによって抽出された**ルールテキスト**に対して、Vector用正規化を適用します。ルールの重複を防ぎ、検索性を高めます。

```go
func (t *RuleExtractionTask) ProcessBatch(ctx context.Context, texts []string) (...) {
    // ... LLMレスポンスパース後
    for _, rule := range ruleSet.Rules {
        rule.Text = utils.NormalizeForVector(rule.Text)
    }
}
```

### 4. Metacognition (自己反省・Unknown管理) パイプライン

#### [MODIFY] `src/pkg/cuber/tasks/metacognition/self_reflection_task.go`
LLMが生成した「問い」を用いて情報を検索する際、クエリを正規化します。

```go
func (t *SelfReflectionTask) TryAnswer(ctx context.Context, question string, unknownID string) (...) {
    // 検索に使用する問いを検索用正規化
    normQuestion := utils.NormalizeForSearch(question)
    embedding, u, err := t.Embedder.EmbedQuery(ctx, normQuestion)
    // ...
}
```

#### [MODIFY] `src/pkg/cuber/tasks/metacognition/ignorance_manager.go`
Unknown や Capability を登録する際、テキストを正規化して保存します。また、`CheckAndResolveUnknowns` での照合時も正規化を適用します。

```go
func (m *IgnoranceManager) RegisterUnknown(ctx context.Context, text string, ...) (...) {
    // 登録テキストをVector正規化
    normText := utils.NormalizeForVector(text)
    // normText を ID生成、ノードプロパティ、埋め込み、FTSインデックスに使用
}

func (m *IgnoranceManager) RegisterCapability(ctx context.Context, text string, ...) (...) {
    // 登録テキストをVector正規化
    normText := utils.NormalizeForVector(text)
    // normText を ID生成、ノードプロパティ、埋め込み、FTSインデックスに使用
}

func (m *IgnoranceManager) CheckAndResolveUnknowns(ctx context.Context, newKnowledgeTexts []string, ...) (...) {
    for _, knowledgeText := range newKnowledgeTexts {
        // 照合用テキストをVector正規化
        normText := utils.NormalizeForVector(knowledgeText)
        embedding, usage, err := m.Embedder.EmbedQuery(ctx, normText)
        // ...
    }
}
```

#### [MODIFY] `src/pkg/cuber/tasks/metacognition/crystallization_task.go`
結晶化（統合）された新しいルールテキストに対して、Vector用正規化を適用します。さらに、**統合ルールの埋め込みベクトルを生成し、VectorStorageに保存します**（既存実装では欠落していました）。

```go
func (t *CrystallizationTask) CrystallizeRules(ctx context.Context) (...) {
    // ... mergTexts 呼び出し後
    crystallized, usage2, err := t.mergTexts(ctx, texts)
    
    // 1. 生成された統合テキストを正規化
    crystallized = utils.NormalizeForVector(crystallized)
    
    // ... Node作成 (crystallizedNode) ...
    // ... AddNodes ...

    // 2. 埋め込みベクトルの生成と保存 (NEW)
    vec, u, err := t.Embedder.EmbedQuery(ctx, crystallized)
    totalUsage.Add(u)
    if err == nil {
        t.VectorStorage.SaveEmbedding(ctx, types.TABLE_NAME_RULE, crystallizedID, crystallized, vec, t.MemoryGroup)
    }
    
    // ... 以降、エッジ付け替え処理
}
```

---

## 期待される成果
1. **表記ゆらぎの完全吸収**: 「ﾊﾟｿｺﾝ」と「パソコン」が全文検索でもベクトル検索でも同一視される。
2. **ノイズのない知識グラフ**: グラフ抽出後のエンティティ名から余計な記号や絵文字が消え、エンティティ解決の精度が向上する。
3. **再現率の向上**: HTMLタグやBoilerplateが消え、純粋な意味内容のみが検索対象となる。
4. **一貫した成長ループ**: 自己反省プロセスで生成される問いも正規化されるため、過去の知識と正しくマッチングされる。

---

## 検証プラン

### 1. 自動テスト
- `go build ./...`
- `make build-linux-amd64`
- `utils/normalize.go` のユニットテストを作成し、以下のケースを検証する:
    - **HTML除去**: `<div>content</div>` -> `content`
    - **Boilerplate除去**: Copyright, Navigation, Social Links が消えること
    - **全角・半角統一**: `ＡＢＣ` -> `ABC`, `ﾊﾝｶｸ` -> `ハンカク`
    - **決定論的グラフID**: `Tesla Inc.` と `tesla inc.` が同一IDになること

### 2. 手動検証
- **Absorb**: HTMLを含むドキュメントを取り込み、LadybugDBの `chunk` テーブル内のテキストがクリーンであることを確認
- **Query**: 検索クエリに絵文字や記号を含めて実行し、正しくヒットすることを確認
- **Memify**: ルール抽出を実行し、`rule` テーブルのテキストが正規化されていることを確認
- **Metacognition**: ログを確認し、`SelfReflectionTask` が生成した「問い」が正規化されて検索に利用されていることを確認

