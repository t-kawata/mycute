package utils

import (
	"fmt"
	"regexp"
	"strings"

	md "github.com/JohannesKaufmann/html-to-markdown"
	"github.com/PuerkitoBio/goquery"
	"github.com/go-shiori/go-readability"
	"golang.org/x/text/transform"
	"golang.org/x/text/unicode/norm"
	"golang.org/x/text/width"
)

var (
	// HTML 関連
	scriptStyleRe   = regexp.MustCompile(`(?is)<script[^>]*?>.*?</script>`)
	styleRe         = regexp.MustCompile(`(?is)<style[^>]*?>.*?</style>`)
	commentRe       = regexp.MustCompile(`(?s)<!--.*?-->`)
	tagRe           = regexp.MustCompile(`<[^>]+>`)
	numericEntityRe = regexp.MustCompile(`&#x?[0-9a-fA-F]+;`)

	// Markdown 関連
	// 注意: コードブロックは4+バッククォートのネスト構文にも対応
	codeBlockRe    = regexp.MustCompile("(?s)````+.*?````+|```.*?```")
	inlineCodeRe   = regexp.MustCompile("`([^`]*)`") // 空インラインコードも許容
	linkRe         = regexp.MustCompile(`\[([^\]]+)\]\([^\)]+\)`)
	imageRe        = regexp.MustCompile(`!\[([^\]]*)\]\([^\)]+\)`)
	headingRe      = regexp.MustCompile(`(?m)^#+\s+`)
	listRe         = regexp.MustCompile(`(?m)^[\*\-\+]\s+`)
	numberedListRe = regexp.MustCompile(`(?m)^\d+\.\s+`)
	quoteRe        = regexp.MustCompile(`(?m)^>\s*`)
	// 注意: 強調パターン (*bold*, _italic_) は削除
	// 理由: プログラミング (snake_case) や数式 (a*b*c) と衝突するため
	hrDashRe  = regexp.MustCompile(`(?m)^-{3,}$`)
	hrStarRe  = regexp.MustCompile(`(?m)^\*{3,}$`)
	hrUnderRe = regexp.MustCompile(`(?m)^_{3,}$`)

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
func CommonNormalize(text string) string {
	// 最初に空白を正規化（リテラル \n を実際の改行に変換）
	// これにより headingRe 等の行頭パターンが正しくマッチする
	text = normalizeWhitespace(text)

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
		return text
	}
	return extractedText
}

func detectHTML(text string) bool {
	header := text[:min(len(text), 1000)]
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
	if err == nil && article.Content != "" {
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
	text = styleRe.ReplaceAllString(text, "")
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
		if code > 0 && code < 0x10FFFF {
			return string(rune(code))
		}
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
	// 注意: 強調パターン (*bold*, _italic_) は適用しない
	// 理由: プログラミング (snake_case) や数式 (a*b*c) を破壊するため
	text = hrDashRe.ReplaceAllString(text, "")
	text = hrStarRe.ReplaceAllString(text, "")
	text = hrUnderRe.ReplaceAllString(text, "")

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
	// リテラルエスケープシーケンスを実際の制御文字に変換
	// （PDF抽出やJSON等で \n が文字として残るケース対策）
	text = strings.ReplaceAll(text, "\\n", "\n")
	text = strings.ReplaceAll(text, "\\r", "\r")
	text = strings.ReplaceAll(text, "\\t", "\t")

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
	if text == "" {
		return ""
	}
	text = norm.NFKC.String(text)
	text = reControl.ReplaceAllString(text, "")
	text = consecutiveSpacesRe.ReplaceAllString(text, " ")
	return strings.TrimSpace(text)
}

// NormalizeForGraph は決定論的なエンティティ解決を確保します。
func NormalizeForGraph(text string) string {
	if text == "" {
		return ""
	}
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
	if text == "" {
		return ""
	}
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
					buf.WriteString(addDakuten(replacement))
					i++
					continue
				} else if next == 'ﾟ' {
					buf.WriteString(addHandakuten(replacement))
					i++
					continue
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
	if res, ok := dakutenMap[s]; ok {
		return res
	}
	return s
}

func addHandakuten(s string) string {
	if res, ok := handakutenMap[s]; ok {
		return res
	}
	return s
}

// Ensure goquery is referenced (used implicitly by readability)
var _ = goquery.NewDocumentFromReader
