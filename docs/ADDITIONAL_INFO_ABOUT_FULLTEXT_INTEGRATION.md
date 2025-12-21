## 形態素解析インデックスの設計：品詞選択戦略

### IN:1a でのインデックス構築

kagomeで各チャンクを解析し、以下の**3層構造**でインデックスを作ることを推奨します：

```go
type ChunkIndex struct {
    ChunkID string
    // Layer 1: 名詞のみ（最も重要）
    Nouns []string  // 品詞=="名詞" AND 品詞細分類1 in ["一般", "固有名詞", "サ変接続"]
    
    // Layer 2: 名詞+動詞基本形
    NounsVerbs []string  // Nouns + (品詞=="動詞" の基本形)
    
    // Layer 3: 全内容語
    AllContentWords []string  // Nouns + Verbs + (品詞=="形容詞")
}
```

**エンティティ網羅性向上という目的**においては、以下の理由で**Layer 1（名詞のみ）+ Layer 2（名詞+動詞基本形）の併用**が最適です：

### Layer 1: 名詞のみ - Precision重視

- **利点**: エンティティは通常名詞句なので、False Positiveが少ない
- **欠点**: 動詞で表現される概念（「買収」「提携」など）を見逃す可能性

```go
tokens := tokenizer.Tokenize("テスラがトヨタと提携を発表した")
// 名詞のみ抽出: ["テスラ", "トヨタ", "提携", "発表"]
```

### Layer 2: 名詞+動詞基本形 - Recall向上

- **利点**: 「提携する」→「提携」のように、動詞の名詞化で概念カバレッジ拡大
- **欠点**: ノイズ増加（「ある」「する」など一般動詞も含む）

```go
// 名詞+動詞基本形: ["テスラ", "トヨタ", "提携", "発表", "発表する"]
// ※ただし一般動詞フィルタ必要
```

### Layer 3: 全内容語 - 過剰な可能性

形容詞を含めると**ノイズが大きく増加**します：

- 「大きい」「高い」などの一般形容詞はエンティティ拡張に寄与しない
- ただし専門用語的形容詞（「脱炭素的」「革新的」）は有用な場合もある

## 具体的な実装戦略

### IN:1a - チャンク解析時

```go
import "github.com/ikawaha/kagome/v2/tokenizer"

// 除外すべき一般動詞リスト
var stopVerbs = map[string]bool{
    "ある": true, "いる": true, "する": true, "なる": true,
    "できる": true, "思う": true, "考える": true,
}

func buildFullTextIndex(chunk string, chunkID string) ChunkIndex {
    t, _ := tokenizer.New(ipa.Dict(), tokenizer.OmitBosEos())
    tokens := t.Tokenize(chunk)
    
    var nouns, nounsVerbs []string
    for _, token := range tokens {
        pos := token.POS()
        surface := token.Surface
        baseForm := token.BaseForm()
        
        // Layer 1: 名詞
        if pos == "名詞" && (pos == "一般" || pos == "固有名詞" || pos == "サ変接続") {
            nouns = append(nouns, surface)
            nounsVerbs = append(nounsVerbs, surface)
        }
        
        // Layer 2: 動詞基本形（stopVerbs除外）
        if pos == "動詞" && !stopVerbs[baseForm] {
            nounsVerbs = append(nounsVerbs, baseForm)
        }
    }
    
    return ChunkIndex{ChunkID: chunkID, Nouns: nouns, NounsVerbs: nounsVerbs}
}
```

### OUT:1a-1c - 検索時のエンティティ拡張

```go
func expandEntities(initialEntities []string, fulltextDB FullTextDB) []string {
    expanded := make(map[string]bool)
    
    // 初期エンティティを追加
    for _, e := range initialEntities {
        expanded[e] = true
    }
    
    // Step 1: Layer 1（名詞のみ）で検索
    for _, entity := range initialEntities {
        chunks := fulltextDB.SearchNouns(entity, topK=5)
        for _, chunk := range chunks {
            // チャンクのノードプロパティから形態素リスト取得
            for _, noun := range chunk.NounList {
                if len(noun) >= 2 {  // 1文字名詞は除外
                    expanded[noun] = true
                }
            }
        }
    }
    
    // Step 2: Layer 2（名詞+動詞）で追加検索（必要に応じて）
    // より包括的な検索が必要な場合のみ
    
    return mapKeysToSlice(expanded)
}
```

## グラフDBへの形態素リスト保存（IN:3b-1）

Kuzu/MemgraphなどでノードプロパティとしてSTRING[]を保存：

```cypher
CREATE (c:Chunk {
  id: 'chunk_001',
  text: '...',
  nouns: ['テスラ', 'トヨタ', '提携'],
  nouns_verbs: ['テスラ', 'トヨタ', '提携', '発表する', '開発する']
})
```

## 推奨構成

エンティティ網羅性向上において最もバランスが良いのは：

1. **IN時**: Layer 1（名詞のみ）とLayer 2（名詞+動詞基本形）の両方をインデックス化
2. **OUT時**: まずLayer 1で検索、recall不足ならLayer 2へフォールバック
3. **動詞フィルタ**: 一般動詞（stopVerbs）は必ず除外
4. **最小文字数**: 1文字の名詞は除外（「社」「業」など）

この設計により、VectorRAGのみに比べてContext Recall 0.85→1.0への向上が期待できます。
