package utils

import (
	"strings"

	"github.com/ikawaha/kagome/v2/tokenizer"
	"github.com/jdkato/prose/v2"
)

// KeywordsResult は3層構造のキーワード抽出結果を保持します。
type KeywordsResult struct {
	Nouns           string // Layer 0: 名詞のみ
	NounsVerbs      string // Layer 1: 名詞 + 動詞基本形
	AllContentWords string // Layer 2: 全内容語（形容詞等含む）
}

// ExtractKeywords はテキストから3層構造のキーワードを抽出します。
// isEn=true の場合は英語用のPOSタガー、false の場合は日本語用のkagomeを使用します。
func ExtractKeywords(tok *tokenizer.Tokenizer, text string, isEn bool) KeywordsResult {
	if isEn {
		return extractKeywordsEN(text)
	}
	return extractKeywordsJA(tok, text)
}

// 日本語用ストップ動詞（ノイズ除去）
// これらの一般的な動詞はエンティティ拡張に寄与しないため除外します。
var stopVerbsJA = map[string]bool{
	"ある": true, "いる": true, "する": true, "なる": true,
	"できる": true, "思う": true, "考える": true,
	// 受身・使役の助動詞
	"れる": true, "られる": true, "せる": true, "させる": true,
}

// 英語ストップワード（日本語テキスト内の英語ノイズ除去用）
var stopWordsEN = map[string]bool{
	// 冠詞
	"a": true, "an": true, "the": true,
	// 前置詞
	"of": true, "in": true, "to": true, "for": true, "on": true, "at": true,
	"by": true, "with": true, "from": true, "as": true, "into": true,
	// 接続詞
	"and": true, "or": true, "but": true, "if": true, "so": true,
	// 代名詞
	"it": true, "its": true, "this": true, "that": true, "these": true, "those": true,
	// be動詞
	"is": true, "are": true, "was": true, "were": true, "be": true, "been": true,
	// その他
	"has": true, "have": true, "had": true, "do": true, "does": true, "did": true,
	"will": true, "would": true, "can": true, "could": true, "may": true, "might": true,
	"not": true, "no": true, "yes": true,
}

// 短いアルファベット単語の例外（保持すべき重要な単語）
var shortAlphabetExceptions = map[string]bool{
	// プログラミング言語
	"c": true, "go": true, "r": true, "d": true, "f#": true,
	// 技術用語
	"ai": true, "ml": true, "ui": true, "ux": true, "os": true, "db": true,
	"ip": true, "id": true, "io": true, "vm": true, "ci": true, "cd": true,
	"qa": true, "it": false, // "it" は代名詞としてストップワードに含めた
	// その他重要な略語
	"ok": true, "vs": true,
}

// isAlphabetOnly は文字列がASCIIアルファベットのみで構成されているかをチェックします。
func isAlphabetOnly(s string) bool {
	for _, r := range s {
		if !((r >= 'a' && r <= 'z') || (r >= 'A' && r <= 'Z')) {
			return false
		}
	}
	return len(s) > 0
}

// isSymbolOnly は文字列が記号のみで構成されているかをチェックします。
func isSymbolOnly(s string) bool {
	for _, r := range s {
		// ひらがな、カタカナ、漢字、アルファベット、数字以外なら記号とみなす
		if (r >= 'ぁ' && r <= 'ゖ') || // ひらがな
			(r >= 'ァ' && r <= 'ヺ') || // カタカナ
			(r >= '一' && r <= '龯') || // CJK統合漢字
			(r >= 'a' && r <= 'z') ||
			(r >= 'A' && r <= 'Z') ||
			(r >= '0' && r <= '9') {
			return false
		}
	}
	return len(s) > 0
}

// shouldIncludeToken はトークンをキーワードとして含めるべきかを判定します。
func shouldIncludeToken(surface string) bool {
	lower := strings.ToLower(surface)

	// 1. 記号のみのトークンは除外
	if isSymbolOnly(surface) {
		return false
	}

	// 2. 英語ストップワードは除外
	if stopWordsEN[lower] {
		return false
	}

	// 3. アルファベットのみの単語は3文字以上、ただし例外は許可
	if isAlphabetOnly(surface) {
		if len(surface) < 3 && !shortAlphabetExceptions[lower] {
			return false
		}
	} else {
		// 日本語を含む場合は2文字以上
		if len([]rune(surface)) <= 1 {
			return false
		}
	}

	return true
}

// extractKeywordsJA は日本語テキストからkagomeを使用してキーワードを抽出します。
func extractKeywordsJA(tok *tokenizer.Tokenizer, text string) KeywordsResult {
	tokens := tok.Tokenize(text)
	var nouns, nounsVerbs, allWords []string
	seenNouns := make(map[string]bool)
	seenVerbs := make(map[string]bool)
	seenAdj := make(map[string]bool)

	for _, t := range tokens {
		pos := t.POS()
		if len(pos) < 1 {
			continue
		}

		surface := t.Surface
		base, _ := t.BaseForm()

		// Layer 1: 名詞（一般、固有名詞、サ変接続）
		if pos[0] == "名詞" && len(pos) > 1 && (pos[1] == "固有名詞" || pos[1] == "一般" || pos[1] == "サ変接続") {
			if shouldIncludeToken(surface) && !seenNouns[surface] {
				nouns = append(nouns, surface)
				nounsVerbs = append(nounsVerbs, surface)
				allWords = append(allWords, surface)
				seenNouns[surface] = true
			}
		} else if pos[0] == "動詞" {
			// Layer 2: 動詞基本形（ストップワード除外）
			if !stopVerbsJA[base] && !seenVerbs[base] && shouldIncludeToken(base) {
				nounsVerbs = append(nounsVerbs, base)
				allWords = append(allWords, base)
				seenVerbs[base] = true
			}
		} else if pos[0] == "形容詞" {
			// Layer 3: 形容詞
			if !seenAdj[base] && shouldIncludeToken(base) {
				allWords = append(allWords, base)
				seenAdj[base] = true
			}
		}
	}

	return KeywordsResult{
		Nouns:           strings.Join(nouns, " "),
		NounsVerbs:      strings.Join(nounsVerbs, " "),
		AllContentWords: strings.Join(allWords, " "),
	}
}

// extractKeywordsEN は英語テキストからproseを使用してキーワードを抽出します。
// Penn Treebank POS タグに基づいて内容語を分類します。
func extractKeywordsEN(text string) KeywordsResult {
	doc, err := prose.NewDocument(text)
	if err != nil {
		return KeywordsResult{}
	}

	var nouns, nounsVerbs, allWords []string
	seenNouns := make(map[string]bool)
	seenVerbs := make(map[string]bool)
	seenContent := make(map[string]bool)

	// 英語用ストップワード
	stopWords := map[string]bool{"the": true, "this": true, "that": true, "is": true, "are": true}

	for _, tok := range doc.Tokens() {
		word := strings.ToLower(tok.Text)
		tag := tok.Tag
		if len(word) <= 2 || stopWords[word] {
			continue
		}

		// Layer 1: Nouns (NN*)
		if strings.HasPrefix(tag, "NN") {
			if !seenNouns[word] {
				nouns = append(nouns, word)
				nounsVerbs = append(nounsVerbs, word)
				allWords = append(allWords, word)
				seenNouns[word] = true
			}
		} else if strings.HasPrefix(tag, "VB") {
			// Layer 2: Verbs (VB*)
			if !seenVerbs[word] {
				nounsVerbs = append(nounsVerbs, word)
				allWords = append(allWords, word)
				seenVerbs[word] = true
			}
		} else if strings.HasPrefix(tag, "JJ") || strings.HasPrefix(tag, "RB") {
			// Layer 3: Adjectives/Adverbs
			if !seenContent[word] {
				allWords = append(allWords, word)
				seenContent[word] = true
			}
		}
	}

	return KeywordsResult{
		Nouns:           strings.Join(nouns, " "),
		NounsVerbs:      strings.Join(nounsVerbs, " "),
		AllContentWords: strings.Join(allWords, " "),
	}
}
