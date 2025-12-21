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
			if len(surface) > 1 && !seenNouns[surface] {
				nouns = append(nouns, surface)
				nounsVerbs = append(nounsVerbs, surface)
				allWords = append(allWords, surface)
				seenNouns[surface] = true
			}
		} else if pos[0] == "動詞" {
			// Layer 2: 動詞基本形（ストップワード除外）
			if !stopVerbsJA[base] && !seenVerbs[base] {
				nounsVerbs = append(nounsVerbs, base)
				allWords = append(allWords, base)
				seenVerbs[base] = true
			}
		} else if pos[0] == "形容詞" {
			// Layer 3: 形容詞
			if !seenAdj[base] {
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
