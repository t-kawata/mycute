# テキスト正規化実装ガイド
## Vector・Graph・FTS各用途別最適化実装（HTML→Markdown変換対応版・完全版）

***

## 0. 共通処理：入力テキストの前処理

### 0.1 共通処理の位置付け

すべての正規化パイプライン（Vector・Graph・Search）は、以下の共通処理を**最初に実施**します。この共通処理により、HTMLおよびMarkdownからのテキスト抽出、Boilerplate削除、空白正規化が行われ、以降の各用途別正規化への準備が整えられます。

```
入力テキスト (HTML / Markdown / Plain Text)
    ↓
【共通処理】
ステップ1: 入力形式の判定と統一化
    └─ HTML入力をMarkdownに変換（DOMプルーニング経由）
    └─ Markdown入力はそのまま保持
    └─ Plain Text入力はそのまま保持
    ↓
ステップ2: HTML・Markdownからテキストを抽出
    └─ HTMLタグ、Markdown記号を除去し、構造化テキストを取得
    └─ script、style タグは完全除外
    ↓
ステップ3: 不要ブロック・ノイズを削除
    └─ 広告、フッター、ナビゲーション等を除去
    ↓
ステップ4: 余計な空白・改行を正規化
    └─ 改行コード統一、連続空白縮約、連続改行縮約
    ↓
出力：共通処理済みテキスト（各用途別正規化へ）
```

### 0.2 ステップ1詳細：HTML → Markdown への変換（DOMプルーニング経由）

**重要なポイント**：VectorDB・GraphDB用途では、HTMLの構造情報（見出し、リスト、強調など）をMarkdown形式で保持することが必須です。ただし、同時に**ヘッダー・フッター・ナビゲーション・広告などのノイズ要素を確実に排除**することが不可欠です。

本設計では、**2段階のフィルタリング**により、高品質なMarkdown化を実現します：

#### 処理フロー（HTML DOMプルーニング）

```
入力（HTML）
    ↓
【ステップ1-A: HTMLレベルのDOMプルーニング】
    │
    ├─ go-readability / Trafilatura により、
    │  「本文コンテンツが含まれるDOMノード」を特定
    │
    ├─ 以下のタグ・属性を完全削除：
    │  ├─ <nav>, <header>, <footer> （ナビゲーション）
    │  ├─ <script>, <style> （実行コード）
    │  ├─ <aside> （サイドバー）
    │  ├─ class="ads", id="advertisement" 等 （広告タグ）
    │  ├─ class="breadcrumb", id="pagination" 等 （メタ情報）
    │  └─ <form> 等 （ユーザー入力フォーム）
    │
    └─ 出力：「本文のみ」を含むクリーンなHTML
    ↓
【ステップ1-B: HTMLからMarkdownへの変換】
    │
    ├─ HTML パーサー（golang.org/x/net/html等）で DOM 解析
    │
    ├─ DOM トラバーサル & Markdown 生成
    │  ├─ <h1> → # 
    │  ├─ <h2> → ## 
    │  ├─ <h3> → ### 
    │  ├─ <h4> → #### 
    │  ├─ <b>, <strong> → **テキスト**
    │  ├─ <i>, <em> → *テキスト*
    │  ├─ <ul>/<ol> → - リスト項目
    │  ├─ <a href="...">リンク</a> → [リンク](URL)
    │  ├─ <blockquote> → > 引用
    │  ├─ <code> → `コード`
    │  ├─ <pre> → ```コード```
    │  └─ <p> → 段落（改行で区切る）
    │
    └─ 出力：本文のみを含むMarkdown
    ↓
【ステップ1-C: テキストレベルのボイラープレート削除（後段処理）】
    │
    ├─ 正規表現やキーワードマッチにより、
    │  残存している「著作権表記」「SNS共有文言」などを除去
    │
    └─ 出力：最終的にノイズ除去されたMarkdown
    ↓
出力（クリーンなMarkdown テキスト）
```

#### 実装コード：DOMプルーニング関数

```go
// pruneHTMLBoilerplate は、HTML入力からノイズ要素（ヘッダー・フッター・広告など）を
// DOMレベルで除去し、「本文のみ」を含むクリーンなHTMLを返します。
//
// 【処理の流れ】
// 1. go-readability による記事抽出
//    → HTMLページから「本文領域」を自動認識
// 2. ノイズタグの明示的な削除
//    → <nav>, <footer>, <script> などを確実に削除
// 3. 広告・メタ要素の除去
//    → class="ads", id="advertisement" などのパターンマッチ削除
// 4. クリーンなHTML（本文のみ）を返す
//
// 【入力例】
// <!DOCTYPE html>
// <html>
// <head><title>Page Title</title></head>
// <body>
//   <header>
//     <nav>Home | Products | Contact</nav>
//   </header>
//   <main>
//     <article>
//       <h1>Article Title</h1>
//       <p>Main content here...</p>
//     </article>
//   </main>
//   <aside>
//     <div class="ads">Advertisement</div>
//   </aside>
//   <footer>Copyright 2024</footer>
// </body>
// </html>
//
// 【出力例（DOMプルーニング後）】
// <article>
//   <h1>Article Title</h1>
//   <p>Main content here...</p>
// </article>
func (n *LadybugTextNormalizer) pruneHTMLBoilerplate(
	htmlText string,
	sourceURL string,
) (string, error) {
	// 【ステップ1】go-readability による記事本文抽出
	//
	// go-readability は、一般的なWebサイトのDOM構造を学習しており、
	// <article>, <main>, <div class="content"> など、
	// 本文が含まれやすいセマンティック要素を認識します。
	//
	// 返された article オブジェクトは、
	// Content フィールドに本文HTMLを含みます。
	article, err := readability.FromReader(
		strings.NewReader(htmlText),
		sourceURL,
	)

	if err != nil || article == nil {
		// フォールバック：go-readability失敗時は、DOMパースして手動クリーニング
		return n.manualDOMPrune(htmlText)
	}

	// article.Content は既にクリーンなHTML
	cleanHTML := article.Content

	// 【ステップ2】さらに確実なノイズ除去（追加フィルタリング）
	//
	// go-readability が取りこぼす可能性のある要素を手動で削除
	cleanHTML = n.removeNoisyElements(cleanHTML)

	return cleanHTML, nil
}

// manualDOMPrune は、golang.org/x/net/html を用いて、
// HTMLから不要なタグを手動で削除します。
//
// これは go-readability のフォールバック処理です。
func (n *LadybugTextNormalizer) manualDOMPrune(htmlText string) (string, error) {
	doc, err := html.Parse(strings.NewReader(htmlText))
	if err != nil {
		return htmlText, err // パース失敗時は元のテキストを返す
	}

	// DOMツリーをトラバーサルして、不要なノードを削除
	n.walkAndRemoveNoiseNodes(doc)

	// 修正後のDOMをHTMLにシリアライズ
	var buf strings.Builder
	html.Render(&buf, doc)

	return buf.String(), nil
}

// walkAndRemoveNoiseNodes は、DOMノードを再帰的にトラバーサルし、
// ノイズ要素を削除します。
func (n *LadybugTextNormalizer) walkAndRemoveNoiseNodes(node *html.Node) {
	if node == nil {
		return
	}

	// 削除対象タグ
	noiseTagsToRemove := map[string]bool{
		"script":     true, // JavaScriptコード
		"style":      true, // CSSスタイル
		"nav":        true, // ナビゲーション
		"header":     true, // ヘッダー
		"footer":     true, // フッター
		"aside":      true, // サイドバー
		"form":       true, // ユーザー入力フォーム
		"noscript":   true, // NoScript代替コンテンツ
		"iframe":     true, // 外部フレーム
	}

	// 削除対象class/id パターン
	noisePatterns := []string{
		"ads", "advertisement", "advert",
		"sponsor", "sponsored",
		"breadcrumb", "pagination",
		"sidebar", "widget",
		"social-share", "share-buttons",
		"related-posts", "suggested",
		"comment", "comments",
	}

	for c := node.FirstChild; c != nil; {
		next := c.NextSibling // 次ノードを保持（削除時に参照が失われるため）

		// ノードの種類をチェック
		if c.Type == html.ElementNode {
			// タグ名チェック
			if noiseTagsToRemove[c.Data] {
				node.RemoveChild(c)
				c = next
				continue
			}

			// class/id 属性をチェック
			shouldRemove := false
			for _, attr := range c.Attr {
				if attr.Key == "class" || attr.Key == "id" {
					for _, pattern := range noisePatterns {
						if strings.Contains(
							strings.ToLower(attr.Val),
							pattern,
						) {
							shouldRemove = true
							break
						}
					}
				}
				if shouldRemove {
					break
				}
			}

			if shouldRemove {
				node.RemoveChild(c)
				c = next
				continue
			}
		}

		// 再帰的に子ノードを処理
		n.walkAndRemoveNoiseNodes(c)
		c = next
	}
}

// removeNoisyElements は、正規表現を用いた追加的なノイズ削除を行います。
// HTMLレベルで取りこぼされた要素をテキストレベルで清掃します。
func (n *LadybugTextNormalizer) removeNoisyElements(html string) string {
	// 典型的なボイラープレート要素を削除
	//
	// 例：
	// - <div class="ads">...</div>
	// - <div class="sidebar">...</div>
	// - <script>...</script> （再度の確保）

	// スタイル要素の完全削除
	html = regexp.MustCompile(`(?i)<style[^>]*>.*?</style>`).ReplaceAllString(html, "")

	// スクリプト要素の完全削除
	html = regexp.MustCompile(`(?i)<script[^>]*>.*?</script>`).ReplaceAllString(html, "")

	// よくある広告・メタ要素
	html = regexp.MustCompile(`(?i)<div\s+class="(ads|advertisement)"[^>]*>.*?</div>`).ReplaceAllString(html, "")
	html = regexp.MustCompile(`(?i)<aside[^>]*>.*?</aside>`).ReplaceAllString(html, "")
	html = regexp.MustCompile(`(?i)<!--.*?-->`).ReplaceAllString(html, "") // HTMLコメント

	return html
}

#### 実装コード：HTMLからMarkdownへの変換

```go
// convertHTMLToMarkdown は、（DOMプルーニング済みの）HTML文字列を
// Markdown文字列に変換します。
//
// 【重要】このメソッドは、pruneHTMLBoilerplate によって
// 既にノイズが除去されたHTMLを入力とします。
// したがって、変換結果のMarkdownは本文のみを含みます。
//
// 【処理の流れ】
// 1. DOMプルーニング済みHTML入力
// 2. github.com/JohannesKaufmann/html-to-markdown を使用して変換
// 3. Markdown形式でMarkdown出力（構造情報を保持）
//
// 【入力例（プルーニング済みHTML）】
// <h1>Article Title</h1>
// <p>This is the <b>main content</b> of the article.</p>
// <ul>
//   <li>Point 1</li>
//   <li>Point 2</li>
// </ul>
//
// 【出力例（Markdown）】
// # Article Title
//
// This is the **main content** of the article.
//
// - Point 1
// - Point 2
func convertHTMLToMarkdown(htmlText string) (string, error) {
	// github.com/JohannesKaufmann/html-to-markdown を使用
	// このライブラリは、HTML構造を忠実にMarkdownに変換し、
	// 見出しレベル、リスト、強調などを正確に処理します。

	converter := html.NewConverter("", true, nil)

	// HTMLからMarkdownへ変換
	markdown, err := converter.ConvertString(htmlText)
	if err != nil {
		return "", err
	}

	return markdown, nil
}
```

### 0.3 ステップ2-4：テキスト抽出・ノイズ除去・空白正規化

#### 実装コード：extractAndClean関数（HTML→Markdown変換完全版）

```go
// extractAndClean は、入力テキスト（HTML/Markdown/Plain Text）を、
// 共通処理により正規化します。
//
// 【処理フロー】
// 1. 入力形式の判定（HTML/Markdown/Plain Text）
// 2. HTML の場合：
//    a. DOMプルーニング（ヘッダー・フッター・広告を除去）
//    b. Markdown への変換
// 3. Markdown からプレーンテキストを抽出
// 4. Boilerplate削除（テキストレベルのノイズ除去）
// 5. 空白・改行の正規化
//
// 【入力】
//   - text: テキスト文字列
//   - contentType: "html", "markdown", "text" のいずれか
//   - sourceURL: 出典URL（HTMLプルーニング時に参照）
//
// 【出力】
//   - 共通処理済みテキスト
func (n *LadybugTextNormalizer) extractAndClean(
	text string,
	contentType string,
	sourceURL string,
) (string, error) {
	var extractedText string
	var err error

	// 【ステップ1-B：入力形式の統一化（HTML → Markdown 変換）】
	switch contentType {
	case "html":
		// 【ステップ1-A：HTMLレベルのDOMプルーニング】
		// ナビゲーション、ヘッダー、フッター、広告を DOMレベルで削除
		cleanHTML, pruneErr := n.pruneHTMLBoilerplate(text, sourceURL)
		if pruneErr != nil {
			// プルーニング失敗時も処理を続行
			cleanHTML = text
		}

		// 【ステップ1-B：プルーニング済みHTMLをMarkdownに変換】
		markdown, err := convertHTMLToMarkdown(cleanHTML)
		if err != nil {
			// 変換失敗時は、フォールバックとしてプレーンテキスト抽出
			extractedText = n.extractFromHTML(text, sourceURL)
		} else {
			// Markdown変換成功時は、このMarkdownをさらに処理
			extractedText, err = n.extractFromMarkdown(markdown)
		}

	case "markdown":
		// Markdownからテキストを抽出
		extractedText, err = n.extractFromMarkdown(text)

	default:
		// プレーンテキストはそのまま使用
		extractedText = text
	}

	if err != nil {
		// エラー時も処理を続行（部分的に抽出されたテキストを使用）
		// ただし、extractedText が空の場合は元のテキストを使用
		if extractedText == "" {
			extractedText = text
		}
	}

	// 【ステップ3：不要ブロック・ノイズを削除（テキストレベル）】
	extractedText = n.removeBoilerplate(extractedText)

	// 【ステップ4：余計な空白・改行を正規化】
	extractedText = n.normalizeWhitespace(extractedText)

	return extractedText, nil
}

// extractFromHTML は、HTML文字列からプレーンテキストを抽出します。
// このメソッドは、HTMLからMarkdownへの変換に失敗した場合のフォールバックです。
func (n *LadybugTextNormalizer) extractFromHTML(
	text string,
	sourceURL string,
) string {
	// go-readability を使用して、メイン記事コンテンツを抽出
	article, err := readability.FromReader(strings.NewReader(text), sourceURL)
	if err == nil && article != nil {
		return article.TextContent
	}

	// フォールバック：bluemonday でタグを除去してプレーンテキスト化
	return n.htmlPolicy.Sanitize(text)
}

// extractFromMarkdown は、Markdown文字列からプレーンテキストを抽出します。
// Markdownの記号（#, *, -, >, ` など）を除去します。
func (n *LadybugTextNormalizer) extractFromMarkdown(text string) (string, error) {
	// goldmark を使用して、MarkdownをHTMLに変換
	var buf bytes.Buffer
	if err := n.markdown.Convert([]byte(text), &buf); err != nil {
		return "", err
	}

	htmlOutput := buf.String()

	// 得られたHTMLをサニタイズして、プレーンテキストを抽出
	return n.htmlPolicy.Sanitize(htmlOutput), nil
}

// removeBoilerplate は、典型的なボイラープレート（広告、フッター等）を
// テキストレベルで除去します。
//
// 【除去対象の例】
// - 「Copyright 2024 All Rights Reserved」
// - 「Home About Contact Us」（ナビゲーション）
// - 「SNS: Share on Facebook, Follow us on Twitter」（SNS呼び出し）
// - 「Page 1 of 10」（ページング）
// - 「Advertisement」「Sponsored Content」（広告表記）
//
// 【注意】DOMレベルのプルーニングで大部分が削除されるため、
// このテキストレベルのフィルタリングは「最終的なノイズ除去」として機能します。
func (n *LadybugTextNormalizer) removeBoilerplate(text string) string {
	// 英語パターンの除去
	for _, re := range n.boilerplateEnPatterns {
		text = re.ReplaceAllString(text, "")
	}

	// 日本語パターンの除去
	for _, re := range n.boilerplateJaPatterns {
		text = re.ReplaceAllString(text, "")
	}

	// 短い行（1-2文字）や典型的なナビゲーション語を含む行を除去
	lines := strings.Split(text, "\n")
	var filteredLines []string

	for _, line := range lines {
		trimmed := strings.TrimSpace(line)
		runeCount := len([]rune(trimmed))

		// フィルタリング条件：
		// - 1-2文字の行は除去（見出しやナビゲーション可能性）
		// - 3-9文字で「Home」「Contact」「About Us」等を含む行は除去
		if runeCount >= 10 || (runeCount >= 3 && runeCount <= 9) {
			// 典型的なナビゲーション語を検出
			if !strings.Contains(trimmed, "Home") &&
				!strings.Contains(trimmed, "Contact") &&
				!strings.Contains(trimmed, "About") {
				filteredLines = append(filteredLines, line)
			}
		}
	}

	text = strings.Join(filteredLines, "\n")
	return text
}

// normalizeWhitespace は、以下の処理により空白・改行を正規化します：
//
// 1. 改行コード統一：CRLF → LF, CR → LF
// 2. 行末の空白・タブを除去
// 3. 行内の連続スペースを単一スペースに縮約
// 4. 連続改行（3行以上の空行）を2行の改行に縮約
func (n *LadybugTextNormalizer) normalizeWhitespace(text string) string {
	// 【ステップ1】改行コード統一：CRLF → LF, CR → LF
	text = strings.ReplaceAll(text, "\r\n", "\n")
	text = strings.ReplaceAll(text, "\r", "\n")

	// 【ステップ2】行ごとの処理
	lines := strings.Split(text, "\n")
	var processedLines []string

	for _, line := range lines {
		// 行末の空白・タブを除去
		line = strings.TrimRight(line, " \t")

		// 行内の連続スペースを単一スペースに縮約
		// Unicode空白（全角スペースなど）もカバー
		fields := strings.Fields(line)
		if len(fields) > 0 {
			line = strings.Join(fields, " ")
		} else {
			line = ""
		}

		processedLines = append(processedLines, line)
	}

	// 【ステップ3】連続改行を縮約：3行以上の改行 → 2行（段落間隔）
	text = strings.Join(processedLines, "\n")
	text = n.consecutiveNewlinesRe.ReplaceAllString(text, "\n\n")

	return strings.TrimSpace(text)
}
```

***

## 1. Vector正規化実装

### 1.1 Vector正規化の目的と設計原則

Vector正規化は、Embedding APIに入力するテキストの**意味的文脈**を最大限に保持することを最優先とします。

Embedding APIは、入力テキストの細微なニュアンスを捉えてベクトル空間上の位置を決定します。例えば、`"Go"` というプログラミング言語と `"go"` という動詞は、異なる文脈を持つ言葉であり、本来は異なるベクトル空間領域に配置されるべきです。しかし、テキストを過度に正規化してしまうと、こうした**意味的な差異が消失**し、検索精度やEmbedding品質が低下します。

同様に、句読点（。、！？など）も、文の区切りや感情的なニュアンスを表現しており、Embedding時には重要な情報です。例えば、`"すごい。"` と `"すごい!!!"` では、感情の強度が異なり、ベクトルも異なるべきです。

したがってVector正規化では、以下の原則に従います：

1. **意味を変えない最小限の正規化のみを実施**
2. **大文字小文字、句読点、感情表現は保持**
3. **除去するのはノイズのみ（制御文字、不可視文字、絵文字）**

なお、絵文字についてはやや議論の余地があります。最新のEmbeddingモデル（OpenAIの `text-embedding-3` など）は、絵文字の意味（感情、ニュアンス）を理解するため、削除すると情報が失われます。しかし、検索ノイズの観点からは削除する方が実用的な場合も多いため、ここでは**削除する実装**を提示しますが、プロダクション環境では要件に応じて選択してください。

### 1.2 Vector正規化の処理ステップ

```
入力テキスト
    ↓
[共通処理] (DOMプルーニング、抽出、Boilerplate削除、空白正規化済み)
    ↓
ステップ1: Unicode正規化 (NFKC)
    └─ 互換文字を統合（例：ﾊﾛｰ → ハロー）
    └─ 分離した合成文字を統合（例：e + ´ → é）
    ↓
ステップ2: 制御文字・不可視文字削除
    └─ NULL, BOM, ゼロ幅スペース, 制御文字を除去
    └─ 改行・タブは保持（文構造情報）
    ↓
ステップ3: 絵文字削除（オプション）
    └─ Unicode絵文字レンジを削除
    ↓
出力テキスト（意味的文脈を保持）
```

### 1.3 Vector正規化の完全実装コード

```go
package textprocessing

import (
	"strings"
	"unicode"

	"golang.org/x/text/unicode/norm"
)

// NormalizeForVector は、Embedding API用の正規化を実施します。
//
// この関数は以下の原則に基づいて設計されています：
//
// 【原則】
// Embedding APIは、入力テキストの細微なニュアンスを捉えてベクトル化する必要があります。
// したがって、意味を変えない範囲でのノイズ除去のみを行い、言語的特性は最大限保持します。
//
// 【保持される要素】
// - 大文字小文字（"Go"言語 vs "go"動詞の区別）
// - 句読点（。！？，など）
// - 感情表現記号（!!!，???など）
// - スペースと段落構造（\n による段落分割）
// - Markdown形式の構造情報（#見出し、*強調* など）
//
// 【除去される要素】
// - 制御文字（C0/C1制御文字）
// - 不可視文字（ゼロ幅スペース、BOMなど）
// - 絵文字（検索ノイズを避けるため）
//
// 【処理の流れ】
// 1. 共通処理済みテキスト（DOMプルーニング、HTML→Markdown変換、テキスト抽出、
//    Boilerplate削除、空白正規化済み）
// 2. NFKC正規化：互換文字を統合、分離した合成文字を統合
// 3. 制御文字・不可視文字削除
// 4. 絵文字削除
// 5. トリミング
//
// 【使用例】
// 入力（共通処理済みテキスト）：
//   "# LLM is powerful! すごい✨\u200b"
//   （ゼロ幅スペースとスマイル絵文字を含む）
//
// 出力：
//   "# LLM is powerful! すごい"
//   （絵文字とゼロ幅スペースが除去され、大文字小文字と感情表現は保持、Markdown記号も保持）
func (n *LadybugTextNormalizer) NormalizeForVector(text string) string {
	// ステップ1: NFKC正規化
	//
	// 説明：
	// NFKC (Compatibility Decomposition, followed by Canonical Composition) は、
	// Unicode正規化の一形式です。この処理により、以下が実現されます：
	//
	// (1) 互換文字を標準形に変換
	//     例：ﾊﾛｰ（半角カナ）→ ハロー（全角カナ）
	//     例：㈱（サラウンド合字）→ (株)（分解形）
	//
	// (2) 分離した合成文字を統合
	//     例：e（U+0065）+ ´（combining acute U+0301）→ é（U+00C9, プリコンポーズ形）
	//
	// なぜ NFKC か？
	// - Embedding APIは、見た目が同じだが異なるコードポイント列を異なるベクトルにマッピングする可能性がある
	// - 同じ見た目なら同じベクトルになるべき（正規化の役割）
	// - NFKC は最も互換性が高く、多くのテキスト処理で標準
	text = norm.NFKC.String(text)

	// ステップ2: 制御文字・不可視文字の削除
	//
	// 説明：
	// Embedding API にとって、不可視の制御文字は以下の問題を引き起こします：
	//
	// (1) 予期しない単語分割
	//     例：ゼロ幅スペース（U+200B）が含まれると、
	//     "Hello\u200bWorld" は "Hello" と "World" に分割される可能性がある
	//
	// (2) トークナイザの混乱
	//     例：BOM（U+FEFF）がテキスト中に含まれると、
	//     トークナイザが特殊トークンとして処理する可能性がある
	//
	// (3) Embeddingモデルの予期しない動作
	//     例：ソフトハイフン（U+00AD）が含まれると、
	//     通常のハイフンとは異なるトークンとして扱われる可能性がある
	//
	// したがって、これらの不可視文字を事前に削除することで、
	// Embedding品質を向上させます。
	//
	// 【注意】改行（U+000A）とタブ（U+0009）は保持します。
	// これらは段落や行構造を表現する重要な情報であり、
	// Embedding時にテキストの構造的意味を保持するために必要です。
	text = removeControlAndInvisibleChars(text)

	// ステップ3: 絵文字の削除
	//
	// 説明：
	// Embedding API は絵文字を理解できるモデルもありますが、
	// キーワード検索やグラフ構築など、LadybugDB での用途を考えると、
	// 絵文字は検索ノイズになる傾向があります。
	//
	// 例：
	//   "最高です！😂"
	//   → Embedding時には感情的ニュアンスとして機能するかもしれませんが、
	//   → キーワード検索では "😂" では誰も検索しないため、ノイズ
	//
	// ただし、プロダクション環境では要件に応じて削除有無を選択してください：
	// - 削除する場合：キーワード検索性能向上、ノイズ減少
	// - 削除しない場合：Embedding品質向上、感情的ニュアンス保持
	//
	// デフォルトではここで削除します。
	text = n.emojiRe.ReplaceAllString(text, "")

	return strings.TrimSpace(text)
}
```

***

## 2. Graph正規化実装

### 2.1 Graph正規化の目的と設計原則

Graph正規化は、グラフ構造（エンティティ、リレーション、属性）を正確に抽出し、**NERタスク（固有表現認識）**により正しくノード化できることを最優先とします。

グラフデータベースでは、テキストから「固有表現」（人名、組織名、製品名など）を抽出し、それらをノードとしてグラフに登録します。例えば：

> 「**Apple** announced new **iPhone 15 Pro Max** with **A18 chip**.」

から以下のノードとリレーションを抽出します：

```
ノード:
  - Apple (entity_type: ORGANIZATION)
  - iPhone 15 Pro Max (entity_type: PRODUCT)
  - A18 chip (entity_type: PRODUCT)

リレーション:
  - Apple → announced → iPhone 15 Pro Max
  - Apple → announced → A18 chip
  - iPhone 15 Pro Max ← equipped_with → A18 chip
```

ここで重要なのは、**NERモデルが「Apple」と「apple」を同一のエンティティとして認識すること**です。しかし、表記ゆらぎが多いと、NERの精度が低下し、不要なノード重複やリレーション漏れが生じます。

したがってGraph正規化では、以下の原則に従います：

1. **NER精度を最大化するために、表記ゆらぎを最小化**
2. **大文字小文字を統一（小文字に）して、エンティティ認識を確実に**
3. **見た目が同じ文字列は同じ正規形に（Unicode正規化）**
4. **制御文字・不可視文字を除去（NER混乱を防ぐ）**
5. **文字幅を統一（ASCII互換形に）し、異表記を吸収**

### 2.2 Graph正規化の処理ステップ

```
入力テキスト
    ↓
[共通処理] (DOMプルーニング、抽出、Boilerplate削除、空白正規化済み)
    ↓
ステップ1: Unicode正規化 (NFKC)
    └─ 互換文字を統合（ﾊﾛｰ → ハロー）
    └─ 分離した合成文字を統合（e + ´ → é）
    ↓
ステップ2: 文字幅統一 (Width.Fold)
    └─ 全角英数 → 半角、半角カナ → 全角
    ↓
ステップ3: 小文字化 (ToLower)
    └─ ASCII文字をすべて小文字に
    ↓
ステップ4: 制御文字・不可視文字削除
    └─ NULL, BOM, ゼロ幅スペース, 制御文字を除去
    ↓
ステップ5: 絵文字削除
    └─ Unicode絵文字レンジを削除
    ↓
出力テキスト（NER精度最大化）
```

### 2.3 Graph正規化の完全実装コード

```go
package textprocessing

import (
	"strings"
	"unicode"

	"golang.org/x/text/unicode/norm"
	"golang.org/x/text/width"
)

// NormalizeForGraph は、NER（固有表現認識）タスク用の正規化を実施します。
//
// この関数は以下の原則に基づいて設計されています：
//
// 【原則】
// NERモデルは、テキストの表記ゆらぎに弱いため、
// グラフ構築前に表記ゆらぎを最小化することで、
// NER精度の向上と重複ノード防止を実現します。
//
// 【処理の流れ】
// 1. 共通処理済みテキスト（DOMプルーニング、HTML→Markdown変換、テキスト抽出、
//    Boilerplate削除、空白正規化済み）
// 2. NFKC正規化：互換文字を統合、分離した合成文字を統合
// 3. 文字幅統一（Width.Fold）：全角英数を半角に、半角カナを全角に
// 4. 小文字化（ToLower）：ASCII文字をすべて小文字に
// 5. 制御文字・不可視文字削除
// 6. 絵文字削除
// 7. トリミング
//
// 【使用例】
// 入力（共通処理済みテキスト）：
//   "Apple announced new iPhone 15 Pro Max with A18 chip.ゼロ幅スペース"
//
// 処理中間値：
//   NFKC後：     "Apple announced new iPhone 15 Pro Max with A18 chip.ゼロ幅スペース"
//   Width.Fold後："Apple announced new iPhone 15 Pro Max with A18 chip.ゼロ幅スペース"
//   ToLower後：  "apple announced new iphone 15 pro max with a18 chip.ゼロ幅スペース"
//
// 出力：
//   "apple announced new iphone 15 pro max with a18 chip."
//   （小文字統一、ゼロ幅スペース・絵文字を除去）
//
// 【NER処理への接続】
// このテキストが NERモデルに入力されると、以下のように処理されます：
//
// 正規化前テキスト：
//   "Apple announced new iPhone 15 Pro Max with A18 chip."
//   → NERモデルが "Apple" と "APPLE" と "apple" を異なるトークンとして処理する可能性がある
//   → 重複エンティティが生じやすい
//
// 正規化後テキスト（Graph）：
//   "apple announced new iphone 15 pro max with a18 chip."
//   → NERモデルが一貫して小文字エンティティとして認識
//   → "apple" という単一ノードが確実に生成される
//   → グラフ構造が正確になる
func (n *LadybugTextNormalizer) NormalizeForGraph(text string) string {
	// ステップ1: NFKC正規化
	//
	// 説明：
	// グラフ構築では、「見た目が同じ文字列は同じエンティティ」として扱う必要があります。
	// NFKC正規化により、以下を実現します：
	//
	// (1) 互換文字の統一
	//     例1：Acme（U+0041 C M E）vs Ａｃｍｅ（全角）
	//     → NFKC後はすべて半角の"Acme"に統一
	//
	//     例2：café（e + combining accent）vs café（プリコンポーズ形）
	//     → NFKC後はすべてプリコンポーズ形に統一
	//
	// (2) 合成文字の統一
	text = norm.NFKC.String(text)

	// ステップ2: 文字幅統一 (Width.Fold)
	//
	// 説明：
	// NERが「SKU-12345」を正確に抽出するためには、
	// 「ＳＫＵ−１２３４５」（全角）と「SKU-12345」（半角）
	// が同じトークンとして認識される必要があります。
	//
	// Width.Fold は以下の統一を行います：
	// - 全角英数 → 半角英数
	// - 半角カナ → 全角カナ
	// - その他の互換文字も標準形に
	//
	// 例：
	// 入力：           "Apple released Ｉｐｈｏｎｅ １５ ｐｒｏ ｍａｘ with Ａ18 chip."
	// NFKC後（変化小）："Apple released Iphone 15 pro max with A18 chip."
	// Width.Fold後：  "Apple released Iphone 15 pro max with A18 chip."（統一形）
	text = width.Fold.String(text)

	// ステップ3: 小文字化 (ToLower)
	//
	// 説明：
	// NERは表記ゆらぎに敏感です。
	// "Apple" と "apple" を同じエンティティとして認識するため、
	// テキスト全体を小文字に統一します。
	//
	// 【重要な注意点】
	// Graph正規化では、大文字小文字情報が失われます。
	// これにより、「固有表現の識別」という観点では、精度が向上しますが、
	// 「元テキストの表現法の復元」という観点では、情報が失われます。
	//
	// そのため、LadybugDB では、以下の設計になっています：
	// - Graph: 小文字正規化版で NER とグラフ構築を実施
	// - Vector: 元テキスト寄り版で Embedding を実施
	// - Search: 極度に正規化版で検索を実施
	//
	// 必要に応じて、元テキストもメタデータとして保持することをお勧めします。
	text = strings.ToLower(text)

	// ステップ4: 制御文字・不可視文字の削除
	//
	// 説明：
	// NERモデルは、不可視の制御文字により混乱します。
	// 例えば、ゼロ幅スペース（U+200B）は NERトークナイザで予期しない分割を引き起こし、
	// 固有表現の境界を誤認識させます。
	//
	// 例：
	// 入力：     "New\u200bYork discovered rare mineral H\u200d2O"
	// NER解析：   ゼロ幅文字が境界と誤認識される可能性
	// 出力：     "newyork discovered rare mineral h2o"（不正な分割）
	//
	// 制御文字を削除することで、NERの正確性を向上させます。
	text = removeControlAndInvisibleChars(text)

	// ステップ5: 絵文字の削除
	//
	// 説明：
	// グラフ構築では、「リンゴ🍎」と「リンゴ」を同じエンティティとして扱うべきです。
	// 絵文字を削除することで、NERの重複認識を防止します。
	//
	// 例：
	// 入力：     "Product review 😂😂😂"
	// 削除後：   "product review"
	// NER出力：  []  （Product が抽出されない可能性も）
	text = n.emojiRe.ReplaceAllString(text, "")

	return strings.TrimSpace(text)
}
```

***

## 3. Search正規化実装（全文検索用）

### 3.1 Search正規化の目的と設計原則

Search正規化は、ユーザーの多様な検索表記に対して、「**どのような入力でもヒットする**」ことを最優先とします。つまり、**再現率（Recall）の最大化**が目標です。

例えば、インデックス内に「apple」というテキストがあるとき：

- ユーザー検索1: "APPLE"（大文字）→ ヒット
- ユーザー検索2: "ＡＰＰ"（全角英数）→ ヒット
- ユーザー検索3: "apple"（小文字）→ ヒット
- ユーザー検索4: "ａｐｐｌｅ"（全角小文字）→ ヒット

これらすべてが同じインデックス項目にマッチするよう、**入力とインデックスの両方を同じ形式に正規化**します。

そのため、Search正規化では、意味的な微妙さや表現のニュアンスは考慮せず、**純粋にテキスト一致性を最大化**することに注力します。

### 3.2 Search正規化の処理ステップ（7ステップ完全実装）

```
入力テキスト
    ↓
【共通処理】
ステップ1: 入力形式の統一化（HTML → Markdown 変換）
    └─ DOMプルーニング経由のMarkdown化
    ↓
ステップ2: HTML・Markdownからテキストを抽出
    └─ HTMLタグ、Markdown記号を除去し、テキストのみ抽出
    ↓
ステップ3: 不要ブロック・ノイズを削除
    └─ 広告、フッター、ナビゲーション等を除去
    ↓
ステップ4: 余計な空白・改行を正規化
    └─ 改行コード統一、連続空白縮約、連続改行縮約
    ↓
【Search専用処理】
ステップ5a: Unicode正規化 (NFKC)
    └─ 互換文字を統合、分離した合成文字を統合
    ↓
ステップ5b: 文字幅統一 (Width.Fold)
    └─ 全角英数 → 半角、半角カナ → 全角
    ↓
ステップ5c: 小文字化 (ToLower)
    └─ ASCII文字をすべて小文字に
    ↓
ステップ6: 絵文字を完全に除去
    └─ 全Unicode絵文字レンジを除去
    ↓
ステップ7: 制御文字・不可視文字を削除
    └─ NULL, BOM, ゼロ幅スペース等を除去
    ↓
出力テキスト（完全正規化・再現率最大化）
```

### 3.3 Search正規化の完全実装コード

```go
// NormalizeForSearch は、全文検索用の完全な正規化を実施します。
//
// この関数は以下の原則に基づいて設計されています：
//
// 【原則】
// 全文検索システムでは、「ユーザーがどう検索してもヒットする」ことが最優先です。
// そのため、テキストを積極的に正規化し、ユーザー入力の「揺らぎ」をすべて吸収します。
//
// 【例】
// ユーザー入力1: "APPLE"（大文字）
// ユーザー入力2: "ＡＰＰ"（全角英数）
// ユーザー入力3: "apple"（小文字）
// インデックス内: "apple"（小文字）
//
// 全て「apple」に正規化されるため、どの入力でもマッチします。
//
// 【7ステップ完全実装】
// このメソッドは、設計書で定義されたステップをすべて実装しています：
//
// 【共通処理】
// 1. HTML → Markdown への変換（DOMプルーニング経由、extractAndClean で実施済み）
// 2. HTML・Markdownからテキストを抽出（extractAndClean で実施済み）
// 3. 不要ブロック・ノイズを削除（extractAndClean で実施済み）
// 4. 余計な空白・改行を正規化（extractAndClean で実施済み）
//
// 【Search専用処理】
// 5a. Unicode正規化（NFKC）
// 5b. 文字幅統一（Width.Fold）
// 5c. 小文字化（ToLower）
// 6. 絵文字を完全に除去
// 7. 制御文字・不可視文字を削除
//
// 【注意点】
// この正規化は「再現率（Recall）の最大化」が目的です。
// 精度（Precision）は低下する可能性があります。
// そのため、LadybugDB では Vector と Graph の正規化形式を並存させており、
// ユーザーは Search で粗くマッチしたテキストを見つけた後、
// Vector や Graph で精密な分析を行うことができます。
func (n *LadybugTextNormalizer) NormalizeForSearch(text string) string {
	// ステップ5a: NFKC正規化
	//
	// 説明：
	// Unicode正規化は、見た目が同じだが異なるコードポイント列を、
	// 同一の正規形に変換します。
	//
	// NFKC（Compatibility Decomposition, followed by Canonical Composition）は、
	// 以下の変換を行います：
	//
	// (1) 互換分解
	//     ＡＢＣ（全角英数） → ABC（半角英数）
	//     ｱｲｳ（半角カナ） → アイウ（全角カナ）
	//     ㈱（サラウンド合字） → (株)（分解形）
	//
	// (2) 正準合成
	//     e（U+0065）+ ´（combining accent U+0301）→ é（U+00C9）
	//
	// なぜ NFKC か？
	// - 互換文字を統一することで、異なる表記の同一内容が統一される
	// - 合成文字を統一することで、見た目が同じなら同じコードポイント列になる
	// - 検索エンジンは NFKC を標準としている
	text = norm.NFKC.String(text)

	// ステップ5b: 文字幅統一（Width.Fold）
	//
	// 説明：
	// Width.Fold は、NFKC での互換分解をさらに進め、
	// 「標準的な幅」に統一します：
	//
	// - 英数字は半角に（これは NFKC で既に行われている）
	// - 記号も標準形に
	//
	// Width.Fold の特性：
	// - NFKC での互換分解をほぼカバーしている
	// - 追加で、一部の記号を標準化
	// - 文字幅を一意に決定
	//
	// 【処理例】
	// 入力：           "ＨｅｌｌｏＷｏｒｌｄ　１２３　カタカナ"
	// NFKC後：        "HelloWorld 123 カタカナ"（既に半角に）
	// Width.Fold後：  "HelloWorld 123 カタカナ"（後のToLowerで小文字化へ）
	//
	// 実際には、NFKC だけでもほぼ目的は達成されていますが、
	// Width.Fold を追加することで、エッジケースをカバーします。
	text = width.Fold.String(text)

	// ステップ5c: 小文字化（ToLower）
	//
	// 説明：
	// テキストをすべて小文字に統一します。
	// これにより、ユーザーが大文字で検索しても、小文字のインデックスとマッチします。
	//
	// 【処理例】
	// 入力：   "Go Programming Language"
	// 出力：   "go programming language"
	//
	// 【重要な注意点】
	// 小文字化により以下の情報が失われます：
	// - 固有名詞の識別（"Go"言語 vs "go"動詞の区別）
	// - 文の開始位置の特定
	// - ニュアンスの表現（"NO!" vs "no"）
	//
	// しかし、Search正規化の目的は「ヒット数を最大化する」ことなので、
	// この情報損失は許容されます。
	// 精密な分析が必要な場合は、Vector や Graph の正規化形式を参照すること。
	text = strings.ToLower(text)

	// ステップ6: 絵文字を完全に除去
	//
	// 説明：
	// 絵文字は、キーワード検索ではノイズになります。
	// ユーザーが「😂」で検索することは稀であるため、
	// インデックスから絵文字を削除することで、インデックスサイズを削減し、
	// 検索性能を向上させます。
	text = n.emojiRe.ReplaceAllString(text, "")

	// ステップ7: 制御文字・不可視文字を削除
	//
	// 説明：
	// 見えないが検索を壊す文字をここで掃除しておくと、
	// チャンク境界やトークン境界が安定します。
	//
	// 【除去対象】
	// - ゼロ幅スペース（U+200B）
	//   例：\"Hello\\u200bWorld\" → \"HelloWorld\"（予期しない分割を防止）
	// - NO-BREAK SPACE（U+00A0）
	// - ソフトハイフン（U+00AD）
	//   例：\"New-York\" → \"newyork\"（統一）
	// - BOM（U+FEFF）
	// - C0/C1 制御文字（改行やタブ以外）
	//
	// ただし、改行（U+000A）とタブ（U+0009）は除去していません。
	// これらは段落や行の区切りを表す重要な情報であり、
	// チャンク分割時に利用されるため、保持します。
	text = removeControlAndInvisibleChars(text)

	// トリミング：前後の空白を除去
	return strings.TrimSpace(text)
}
```

***

## 4. 共通ヘルパー関数

```go
import (
	"regexp"
	"strings"
	"unicode"

	"golang.org/x/text/unicode/norm"
)

// removeControlAndInvisibleChars は、制御文字と不可視文字を削除します。
//
// この関数は、Vector、Graph、Search すべての正規化で使用されます。
// 不可視文字の種類と処理方法は、以下の通りです：
func removeControlAndInvisibleChars(text string) string {
	// 不可視文字と制御文字のマップ
	invisibleChars := map[rune]bool{
		// === 不可視文字（個別処理） ===
		'\u0000': true,  // NULL
		'\u200B': true,  // Zero-width space
		'\u200C': true,  // Zero-width non-joiner
		'\u200D': true,  // Zero-width joiner
		'\u00A0': true,  // No-break space
		'\u00AD': true,  // Soft hyphen
		'\uFEFF': true,  // BOM / Zero-width no-break space

		// === 改行・タブ（保持） ===
		// '\u000A': false, // Line Feed (LF) - 保持
		// '\u0009': false, // Tab - 保持
	}

	return strings.Map(func(r rune) rune {
		// 既知の不可視文字なら除去
		if r == '\u000A' || r == '\u0009' {
			return r // 改行・タブは保持
		}

		if invisibleChars[r] {
			return -1 // 除去
		}

		// C0制御文字（U+0000-U+001F, 改行・タブ除外）
		// C1制御文字（U+0080-U+009F）
		if unicode.IsControl(r) {
			return -1 // 除去
		}

		// その他の文字は保持
		return r
	}, text)
}
```

***

## 5. 初期化と使用例

```go
import (
	"fmt"
	"regexp"

	"github.com/go-readability/readability"
	"github.com/microcosm-cc/bluemonday"
	"github.com/yuin/goldmark"
	"github.com/yuin/goldmark/extension"
)

// LadybugTextNormalizer はテキスト正規化を行うノーマライザーです。
type LadybugTextNormalizer struct {
	markdown              goldmark.Markdown
	htmlPolicy            *bluemonday.Policy
	boilerplateEnPatterns []*regexp.Regexp
	boilerplateJaPatterns []*regexp.Regexp
	emojiRe               *regexp.Regexp
	consecutiveNewlinesRe *regexp.Regexp
}

// NewLadybugTextNormalizer は、テキストノーマライザーを初期化します。
func NewLadybugTextNormalizer() *LadybugTextNormalizer {
	return &LadybugTextNormalizer{
		markdown: goldmark.New(
			goldmark.WithExtensions(extension.GFM),
		),
		htmlPolicy: bluemonday.StrictPolicy(),
		boilerplateEnPatterns: compileEnglishBoilerplatePatterns(),
		boilerplateJaPatterns: compileJapaneseBoilerplatePatterns(),
		emojiRe: regexp.MustCompile(
			`[\p{So}\p{Sk}\U0001F600-\U0001F64F\U0001F300-\U0001F5FF\U0001F680-\U0001F6FF\U0001F700-\U0001F77F\U0001F780-\U0001F7FF\U0001F800-\U0001F8FF\U0001F900-\U0001F9FF\U0001FA00-\U0001FA6F\U0001FA70-\U0001FAFF\u2600-\u26FF\u2700-\u27BF\UFE00-\UFE0F\U0001F1E0-\U0001F1FF]+`,
		),
		consecutiveNewlinesRe: regexp.MustCompile(`\n{3,}`),
	}
}

// 【使用例】
func main() {
	normalizer := NewLadybugTextNormalizer()

	// 入力HTML
	htmlText := `<!DOCTYPE html>
<html>
<head><title>Page Title</title></head>
<body>
  <nav>Home | Products | Contact</nav>
  <article>
    <h1>LladybugDB Guide</h1>
    <p>LadybugDB is a <b>powerful</b> Kuzu fork! 😊</p>
    <p>Supports <i>Vector</i>, <b>Graph</b>, and full-text search.</p>
  </article>
  <footer>Copyright 2024 TechCorp Inc.</footer>
</body>
</html>`

	// ステップ1：共通処理（DOMプルーニング、抽出、Boilerplate削除、空白正規化）
	cleanedText, _ := normalizer.extractAndClean(htmlText, "html", "https://example.com")

	// ステップ2：Vector用正規化
	vectorNorm := normalizer.NormalizeForVector(cleanedText)
	fmt.Println("【Vector】")
	fmt.Println(vectorNorm)
	// 出力例：
	// # LladybugDB Guide
	// LadybugDB is a **powerful** Kuzu fork!
	// Supports *Vector*, **Graph**, and full-text search.

	// ステップ3：Graph用正規化
	graphNorm := normalizer.NormalizeForGraph(cleanedText)
	fmt.Println("\n【Graph】")
	fmt.Println(graphNorm)
	// 出力例：
	// # lladybugdb guide
	// lladybugdb is a powerful kuzu fork!
	// supports vector, graph, and full-text search.

	// ステップ4：Search用正規化
	searchNorm := normalizer.NormalizeForSearch(cleanedText)
	fmt.Println("\n【Search】")
	fmt.Println(searchNorm)
	// 出力例：
	// lladybugdb guide
	// lladybugdb is a powerful kuzu fork!
	// supports vector, graph, and full-text search.
}
```

***

## 6. 設計のポイント

### 6.1 2段階のノイズ除去で高品質化

本設計では、HTMLからMarkdown化への過程で**2段階のフィルタリング**を実施します：

1. **DOMレベルのプルーニング（ステップ1-A）**
   - `go-readability` や手動DOM走査により、ナビゲーション・フッター・広告を**タグごと削除**
   - HTMLの構造的意味を活かした確実な除去

2. **テキストレベルのボイラープレート削除（ステップ3）**
   - 正規表現やキーワードマッチにより、取りこぼされたテキストを最終清掃
   - 「著作権表記」「SNS呼び出し」などを完全に除去

このアプローチにより、**Markdown化が非常に高い品質を保ちながら**、VectorDB・GraphDB・Search用途にそれぞれ最適な正規化が行われます。

### 6.2 HTML → Markdown 変換による構造保持

従来：HTML → Plain Text（構造情報の喪失）  
本設計：HTML → Markdown → Plain Text（構造情報を段階的に処理）

Markdown中間形式により、ベクトル化やグラフ化時に、見出しや強調などのセマンティック情報を活用でき、LLM処理の精度が向上します。

### 6.3 3つの正規化アプローチの並存

| 用途 | 目標 | 特徴 |
| :--- | :--- | :--- |
| **Vector** | 意味的ニュアンス保持 | 大文字小文字、句読点、段落構造を保持、Markdown形式も保持 |
| **Graph** | NER精度最大化 | 表記ゆらぎ最小化、エンティティ認識確実化、小文字統一 |
| **Search** | 再現率最大化 | 極度に正規化、あらゆる入力をカバー、プレーンテキスト化 |

LadybugDBはこの3つを並存させることで、各用途に最適なテキスト処理を実現します。

### 6.4 共通処理による効率化

すべてのパイプライン（Vector・Graph・Search）が共通処理層を共有することで：

- **DOMプルーニングが一度で済む**（HTML処理の最小化）
- **Boilerplate削除の論理が一元化**
- **空白正規化が統一**
- **メンテナンス・拡張が容易**
- **バグ修正の影響範囲を最小化**
