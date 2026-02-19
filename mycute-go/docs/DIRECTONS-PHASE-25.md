# 開発フェーズ25：全文検索（FTS）統合による検索精度の向上

## 目的
`Absorb` (IN) および `Query` (OUT) フローに全文検索エンジン（LadybugDB 内蔵）を統合し、ベクトル検索の「意味的類似性」だけでなく「キーワード一致」によるエンティティ網羅性を向上させます。これにより、知識グラフのトラバーサル（探索）の起点を大幅に強化し、回答の欠落（False Negative）を最小限に抑えます。

## 概要
1.  **形態素解析の導入**: `ChunkingTask` 内で `kagome` を使用してチャンクから名詞・動詞などのキーワードを抽出。
2.  **DBスキーマ拡張**: `Chunk` テーブルに `keywords` カラムを追加し、FTS（Full-Text Search）インデックスを構築。
3.  **エンティティ拡張 (Entity Expansion)**: `Query` 時に初期エンティティをキーとしてチャンクを全文検索し、ヒットしたチャンクのキーワード群で探索対象エンティティを拡張。
4.  **英語対応**: `IsEn=true` 時に英語に最適化されたキーワード抽出ロジックへの分岐。
5.  **明示的な制御**: 検索時にどのレイヤー（名詞のみ等）を使用するかを `QueryConfig` で指定可能にし、決定論的なフォールバックを排除。

---

## 重要な実装指針

### 1. 形態素解析とキーワード抽出（3層構造）
- **Layer 1: 名詞のみ (Precision重視)**: 日本語は「一般・固有名詞・サ変」、英語は「NN*」を抽出。
- **Layer 2: 名詞+動詞 (Recall向上)**: Layer 1に「動詞（基本形/VB*）」を追加。日本語は `stopVerbs` 除外。
- **Layer 3: 全内容語 (統合検索)**: Layer 2に「形容詞・副詞（JJ*/RB*）」を追加。
- **正規化**: 各レイヤーの結果はスペース区切りの文字列として `Chunk` の各カラムに保存。

### 2. 全文検索の実行タイミングとレイヤー活用
- **柔軟な戦略選択**: `Query` 時にどのレイヤーでエンティティ拡張を行うかを **`FtsLayer`** パラメータで明示的に指定します。
- **クエリの形態素解析**: FTS検索を実行する直前にも、検索クエリ自体を `IsEn` に基づいて形態素解析し、ターゲットのカラム（名詞のみ等）に合致するキーワード群を生成してから検索を実行します。

---

## 実装詳細

### 1. データ構造とインターフェースの更新

#### [MODIFY] [interfaces.go](file:///Users/kawata/shyme/mycute/src/pkg/cuber/storage/interfaces.go)

`Chunk` 構造体に `Keywords` フィールドを追加し、`VectorStorage` インターフェースに `FullTextSearch` を追加します。

```diff
 type Chunk struct {
     ID          string    `json:"id"`
     MemoryGroup string    `json:"memory_group"`
     DocumentID  string    `json:"document_id"`
     Text        string    `json:"text"`
+    Keywords    string    `json:"keywords"`    // 統合キーワード（全内容語）
+    Nouns       string    `json:"nouns"`       // Layer 1: 名詞のみ（Precision重視）
+    NounsVerbs  string    `json:"nouns_verbs"` // Layer 2: 名詞+動詞基本形（Recall向上）
     Embedding   []float32 `json:"embedding"`
     TokenCount  int       `json:"token_count"`
     ChunkIndex  int       `json:"chunk_index"`
 }

  // storage/interfaces.go 等
  type QueryResult struct {
      ID         string
      Text       string
      Distance   float64
      Nouns      string // NEW: FTS拡張用にチャンクから取り出すキーワード
      NounsVerbs string // NEW: FTS拡張用にチャンクから取り出すキーワード
  }

 type VectorStorage interface {
     // ...
     Query(ctx context.Context, tableName types.TableName, vector []float32, topk int, memoryGroup string) ([]*QueryResult, error)
+    // NEW: 全文検索 (言語と対象レイヤーを明示)
+    FullTextSearch(ctx context.Context, tableName types.TableName, query string, topk int, memoryGroup string, isEn bool, layer types.FtsLayer) ([]*QueryResult, error)
     // ...
 }
```

#### [MODIFY] [config_types.go](file:///Users/kawata/shyme/mycute/src/pkg/cuber/types/config_types.go)

`QueryConfig` に FTS の制御パラメータを追加します。

> [!NOTE]
> **2種類の型が存在する理由**:
> - `FtsLayerType` (`uint8`): REST API のリクエスト/レスポンス、および Swagger ドキュメント用。JSON シリアライズで整数として送受信。
> - `FtsLayer` (`string`): Cuber 内部ロジック（`QueryConfig`）で使用。可読性が高く、switch文での比較が明確。
> BL 層 (`cubes_bl.go`) でリクエストの `uint8` から内部の `string` へマッピングします。

```go
// REST API用の型 (uint8)
type FtsLayerType uint8

const (
	FTS_LAYER_TYPE_NOUNS       FtsLayerType = 1 // Layer 1: 名詞のみ
	FTS_LAYER_TYPE_NOUNS_VERBS FtsLayerType = 2 // Layer 2: 名詞 + 動詞 (デフォルト)
	FTS_LAYER_TYPE_ALL         FtsLayerType = 3 // Layer 3: 全内容語
)

// Cuber内部ロジック用の型 (string)
type FtsLayer string

const (
	FTS_LAYER_NOUNS       FtsLayer = "nouns"       // Layer 1: 名詞のみ
	FTS_LAYER_NOUNS_VERBS FtsLayer = "nouns_verbs" // Layer 2: 名詞 + 動詞
	FTS_LAYER_ALL         FtsLayer = "all"         // Layer 3: 全内容語
)

type QueryConfig struct {
	// ... (既存フィールド)
	IsEn       bool
	FtsLayer   FtsLayer // NEW: 検索に使用するFTSレイヤーの指定 (内部用 string)
	FtsTopk    int      // NEW: FTSによる拡張数 (デフォルト: 3)
}
```

---

### 2. 形態素解析ユーティリティの作成

#### [NEW] [morphological.go](file:///Users/kawata/shyme/mycute/src/pkg/cuber/utils/morphological.go)

```go
package utils

import (
	"strings"

	"github.com/ikawaha/kagome/v2/tokenizer"
	"github.com/jdkato/prose/v2"
)

// KeywordsResult は3層構造のキーワード抽出結果を保持します。
type KeywordsResult struct {
	Nouns           string // Layer 1: 名詞のみ
	NounsVerbs      string // Layer 2: 名詞 + 動詞基本形
	AllContentWords string // Layer 3: 全内容語（形容詞等含む）
}

// ExtractKeywords はテキストから3層構造のキーワードを抽出します。
func ExtractKeywords(tok *tokenizer.Tokenizer, text string, isEn bool) KeywordsResult {
	if isEn {
		return extractKeywordsEN(text)
	}
	return extractKeywordsJA(tok, text)
}

// 日本語用ストップ動詞（ノイズ除去）
var stopVerbsJA = map[string]bool{
	"ある": true, "いる": true, "する": true, "なる": true,
	"できる": true, "思う": true, "考える": true,
}

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
		base := t.BaseForm()

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

func extractKeywordsEN(text string) KeywordsResult {
	doc, _ := prose.NewDocument(text)
	var nouns, nounsVerbs, allWords []string
	seenNouns := make(map[string]bool)
	seenVerbs := make(map[string]bool)
	seenContent := make(map[string]bool)

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
```

---

### 3. INフロー (Absorb) の改修

#### [MODIFY] [chunking_task.go](file:///Users/kawata/shyme/mycute/src/pkg/cuber/tasks/chunking/chunking_task.go)

`ChunkingTask` に `IsEn` を追加し、チャンク確定時にキーワード抽出を実行します。

```go
type ChunkingTask struct {
    // ...
    IsEn bool // 追加
}

func NewChunkingTask(..., isEn bool) (*ChunkingTask, error) { // 引数追加
    // ...
    return &ChunkingTask{
        // ...
        IsEn: isEn,
    }, nil
}

func (t *ChunkingTask) finalizeChunk(...) error {
    chunkText := strings.Join(*currentChunk, "")
    // ...
    kwRes := utils.ExtractKeywords(t.Tokenizer, chunkText, t.IsEn)
    *chunks = append(*chunks, &storage.Chunk{
        // ...
        Text:       chunkText,
        Keywords:   kwRes.AllContentWords, // 全内容語を統合検索用に
        Nouns:      kwRes.Nouns,           // Layer 1
        NounsVerbs: kwRes.NounsVerbs,      // Layer 2
        // ...
    })
    // ...
}
```

#### [MODIFY] [cuber.go](file:///Users/kawata/shyme/mycute/src/pkg/cuber/cuber.go)

`cognify` メソッド内で `NewChunkingTask` を呼び出す際、`isEn` を渡すように修正します。

---

### 4. 保存層 (LadybugDB) の改修

#### [MODIFY] [ladybugdb_storage.go](file:///Users/kawata/shyme/mycute/src/pkg/cuber/db/ladybugdb/ladybugdb_storage.go)

スキーマ、保存、全文検索メソッドを追加します。また、`LadybugDBStorage` 構造体に `tokenizer` フィールドを追加し、初期化時に日本語トークナイザを設定する必要があります。

```go
// LadybugDBStorage 構造体に追加
type LadybugDBStorage struct {
    // ...
    tokenizer *tokenizer.Tokenizer // NEW: 形態素解析器 (kagome)
}

// NewLadybugDBStorage 内で初期化
func NewLadybugDBStorage(...) (*LadybugDBStorage, error) {
    // ...
    tok, err := tokenizer.New(ipa.Dict(), tokenizer.OmitBosEos())
    if err != nil {
        return nil, fmt.Errorf("Failed to initialize tokenizer: %w", err)
    }
    return &LadybugDBStorage{
        // ...
        tokenizer: tok,
    }, nil
}
```

1.  **EnsureSchema**: `keywords` カラム追加とインデックス作成。
    ```go
    // Node Tables 修正
    fmt.Sprintf(`CREATE NODE TABLE Chunk (
        id STRING,
        memory_group STRING,
        document_id STRING,
        text STRING,
        keywords STRING,    // Layer 3 (全内容語)
        nouns STRING,       // Layer 1 (名詞のみ)
        nouns_verbs STRING, // Layer 2 (名詞+動詞)
        token_count INT64,
        chunk_index INT64,
        embedding %s,
        PRIMARY KEY (id)
    )`, vectorType),

    // インデックス作成（EnsureSchemaの最後に追加）
    `CALL CREATE_FTS_INDEX('Chunk', 'nouns_fts_index', ['nouns'])`,
    `CALL CREATE_FTS_INDEX('Chunk', 'nouns_verbs_fts_index', ['nouns_verbs'])`,
    `CALL CREATE_FTS_INDEX('Chunk', 'keywords_fts_index', ['keywords'])`
    ```

2.  **SaveChunk**: `keywords` の保存ロジックを追加（MERGE句を更新）。

3.  **FullTextSearch**: 実装追加。
    ```go
    func (s *LadybugDBStorage) FullTextSearch(ctx context.Context, tableName types.TableName, query string, topk int, memoryGroup string, isEn bool, layer types.FtsLayer) ([]*QueryResult, error) {
        // 1. 検索クエリ自体を形態素解析して、検索語を正規化・抽出
        kwRes := utils.ExtractKeywords(s.tokenizer, query, isEn)
        
        var searchQuery string
        var indexName string
        switch layer {
        case types.FTS_LAYER_NOUNS:
            searchQuery = kwRes.Nouns
            indexName = "nouns_fts_index"
        case types.FTS_LAYER_NOUNS_VERBS:
            searchQuery = kwRes.NounsVerbs
            indexName = "nouns_verbs_fts_index"
        default:
            searchQuery = kwRes.AllContentWords
            indexName = "keywords_fts_index"
        }
        
        if searchQuery == "" {
            return nil, nil // 検索語がない場合は空
        }

        // 2. memory_group で厳格にフィルタリングしつつ、指定されたレイヤーのインデックスに対してクエリ実行
        // LadybugDBのFTSでは、MATCH句とFTS関数を組み合わせることでパーティション分離を維持します。
        ftsQuery := fmt.Sprintf(`
            MATCH (node:%s)
            WHERE node.memory_group = '%s'
            CALL QUERY_FTS_INDEX('%s', '%s', '%s')
            WHERE node.id = id
            RETURN node.id, node.text, node.nouns, node.nouns_verbs, score
            ORDER BY score DESC
            LIMIT %d
        `, tableName, escapeString(memoryGroup), tableName, indexName, escapeString(searchQuery), topk)

        rows, err := s.getConn(ctx).Query(ftsQuery)
        if err != nil {
            return nil, err
        }
        defer rows.Close()

        var results []*QueryResult
        for rows.Next() {
            var res QueryResult
            if err := rows.Scan(&res.ID, &res.Text, &res.Nouns, &res.NounsVerbs, &res.Distance); err != nil {
                return nil, err
            }
            results = append(results, &res)
        }
        return results, nil
    }
    ```

---

### 5. OUTフロー (Query) の改修

#### [MODIFY] [graph_completion.go](file:///Users/kawata/shyme/mycute/src/pkg/cuber/tools/query/graph_completion.go)

`getGraph` メソッド内を修正し、FTS によるエンティティ拡張ロジックを追加します。以下のコードスニペットは、ベクトル検索の結果をループし、各エンティティ名で FTS を実行し、ヒットしたチャンクの `Nouns` フィールドから新たな候補エンティティを抽出してリストに追加しています。

```go
func (t *GraphCompletionTool) getGraph(ctx context.Context, entityTopk int, query string, embeddingVecs *[]float32, config types.QueryConfig) (embedding *[]float32, graph *[]*storage.Triple, usage types.TokenUsage, err error) {
    // 1. ベクトル検索 (既存) - Entity テーブルからクエリに意味的に近いエンティティを取得
    entityResults, err := t.VectorStorage.Query(ctx, types.TABLE_NAME_ENTITY, *embeddingVecs, entityTopk, t.memoryGroup)
    if err != nil {
        return nil, nil, usage, err
    }

    // 2. FTSによるエンティティ拡張 (NEW)
    // 最終的な探索対象エンティティを管理（IDの重複をマップで完全に排除）
    finalEntityIDsMap := make(map[string]bool)
    var finalEntityIDs []string

    for _, res := range entityResults {
        // ベクトル検索結果のエンティティIDを追加
        if !finalEntityIDsMap[res.ID] {
            finalEntityIDsMap[res.ID] = true
            finalEntityIDs = append(finalEntityIDs, res.ID)
        }

        // res.Text (エンティティ名、例: "テスラ") を検索クエリとして Chunk テーブルを FTS 検索
        // config.FtsLayer で指定されたレイヤー（例: nouns）のインデックスを使用
        ftsResults, ftsErr := t.VectorStorage.FullTextSearch(
            ctx,
            types.TABLE_NAME_CHUNK,
            res.Text,           // エンティティ名で検索
            config.FtsTopk,     // FTS の Top-K (例: 3)
            t.memoryGroup,
            config.IsEn,
            config.FtsLayer,    // 例: types.FTS_LAYER_NOUNS_VERBS
        )
        if ftsErr != nil {
            // FTS エラーは致命的ではないためログのみ
            utils.LogWarn(t.Logger, fmt.Sprintf("FTS error for entity '%s': %v", res.Text, ftsErr))
            continue
        }

        // ヒットしたチャンクからキーワードを取り出し、エンティティ候補として追加
        for _, ftsRes := range ftsResults {
            // QueryResult.Nouns には、チャンクから抽出された名詞がスペース区切りで格納されている
            candidateTerms := strings.Split(ftsRes.Nouns, " ")
            for _, term := range candidateTerms {
                // 1文字のノイズを除外し、既に追加済みでなければリストに追加
                if term != "" && len(term) > 1 && !finalEntityIDsMap[term] {
                    finalEntityIDsMap[term] = true
                    finalEntityIDs = append(finalEntityIDs, term)
                }
            }
        }
    }

    // 3. グラフトラバーサル (重複が排除された finalEntityIDs を使用)
    // ベクトル検索のみよりも拡張された起点から、より豊かな部分グラフを取得
    triples, err := t.GraphStorage.GetTriples(ctx, finalEntityIDs, t.memoryGroup)
    if err != nil {
        return nil, nil, usage, err
    }

    return embeddingVecs, &triples, usage, nil
}
```

---

### 6. REST API 層の改修

FTS の挙動を外部から制御できるように、API パラメータと Swagger を更新します。

#### [MODIFY] [cubes_param.go](file:///Users/kawata/shyme/mycute/src/mode/rt/rtparam/cubes_param.go)

`QueryCubeParam` にフィールドを追加します。

```go
type QueryCubeParam struct {
    // ...
    FtsType  uint8 `form:"fts_type" swaggertype:"integer" example:"2"` // 1: 名詞, 2: 名詞+動詞, 3: 全部
    FtsTopk  int   `form:"fts_topk" swaggertype:"integer" example:"3"` // FTSでの拡張数
}
```

#### [MODIFY] [cubes_req.go](file:///Users/kawata/shyme/mycute/src/mode/rt/rtreq/cubes_req.go)

`QueryCubeReq` にフィールドを追加します。

```go
type QueryCubeReq struct {
    // ...
    FtsType uint8 `json:"fts_type" binding:"omitempty,gte=1,lte=3"`
    FtsTopk int   `json:"fts_topk" binding:"omitempty,gte=0"`
}
```

#### [MODIFY] [cubes_handler.go](file:///Users/kawata/shyme/mycute/src/mode/rt/rthandler/hv1/cubes_handler.go)

`QueryCube` の Swagger アノテーションを更新します。

```go
// @Description | ... | ... | ... |
// @Description | fts_type | uint8 | 1: 名詞のみ, 2: 名詞+動詞 (Default), 3: 全内容語 |
// @Description | fts_topk | int | FTSで検索・拡張するチャンクの数 (Default: 3) |
```

#### [MODIFY] [cubes_bl.go](file:///Users/kawata/shyme/mycute/src/mode/rt/rtbl/cubes_bl.go)

`QueryCube` 関数内で `QueryConfig` へのマッピングを実装します。**デフォルト値（2）の適用**を確実に行います。

```go
// QueryCube 内でのマッピング例
ftsLayer := types.FTS_LAYER_NOUNS_VERBS // Default: 2
switch req.FtsType {
case 1:
    ftsLayer = types.FTS_LAYER_NOUNS
case 3:
    ftsLayer = types.FTS_LAYER_ALL
}

ftsTopk := req.FtsTopk
if ftsTopk == 0 {
    ftsTopk = 3 // Default
}

config := types.QueryConfig{
    // ...
    FtsLayer: ftsLayer,
    FtsTopk:  ftsTopk,
}
```


---

## 期待される効果
- **固有名詞の強一致**: 「テスラ」という単語が含まれるチャンクがベクトル検索で漏れても、FTSで見つけ出し、そこから「イーロン・マスク」などの関連ノードへのパスを復元できます。
- **表記揺れ対策**: 英語モードでの小文字化・ステミングにより、`Tesla` と `tesla` を同じキーワードとして扱えるようになります。
- **リコールの向上**: 文脈抽出フェーズでの情報不足が原因の回答ミスが激減します。

---

## 今後の展望：Memify への影響
現時点では `Absorb` (IN) と `Query` (OUT) を優先しますが、`Memify`（知識の結晶化・整理）においても FTS を活用することで以下の向上が見込めます：
1.  **Unknown エンティティの解消**: 意味的に近いだけでなく、名前がキーワードとして部分一致する既存エンティティを FTS で効率的に探し出すことができます。
2.  **グループ化の精度向上**: 長いドキュメント群から共通のキーワードを持つチャンクを FTS で集約し、より精度の高い要約（Summary）を生成できます。

`Memify` への統合は、IN/OUT での品質向上が確認された後の次フェーズとして計画します。
