# Cognee Go Implementation: Phase-09 Detailed Development Directives

## 0. はじめに (Introduction)

本ドキュメントは、**Phase-09: Optimization & Performance (効率化と最適化)** の実装を「迷いなく」「確実に」遂行するための詳細な開発指示書です。
`docs/AFTER-PHASE-08.md` で策定された設計に基づき、具体的なコード、ファイルパス、手順を網羅しています。

> [!IMPORTANT]
> **実装のゴール**
> Phase-08で構築した「生きた知識グラフ」の処理効率を大幅に向上させること。
> 具体的には、Embedding API呼び出しの削減（キャッシュ活用）、N+1問題の解消（効率的なクエリ）、大規模グラフでのスケーラビリティの確保を実現します。

> [!CAUTION]
> **オリジナルCogneeとの差異**
> Phase-09以降は、オリジナルのPython版Cogneeには存在しない独自の実装領域です。
> したがって、参照実装がなく、全ての設計判断はこのドキュメントと関連する設計書に基づいて行います。
> 不明点があれば、必ず確認してから実装を進めてください。

---

## 1. 実装ステップ一覧 (Implementation Steps)

以下の順序で実装を進めます。各ステップは依存関係に基づいています。

| Step | 内容 | 対象ファイル | 行数目安 |
|------|------|-------------|---------|
| 1 | VectorStorage Interface Extension | `interfaces.go` | +20行 |
| 2 | DuckDBStorage Embedding Cache | `duckdb_storage.go` | +80行 |
| 3 | GraphStorage Interface Extension | `interfaces.go` | +15行 |
| 4 | CozoStorage Orphan Detection | `cozo_storage.go` | +50行 |
| 5 | CrystallizationTask Optimization | `crystallization_task.go` | 置換 |
| 6 | PruningTask Optimization | `pruning_task.go` | 置換 |
| 7 | Performance Verification | `main.go` | +60行 |

---

## Step 1: VectorStorage Interface Extension

**目的**: 保存済みのEmbeddingを取得するAPIをインターフェースに追加し、不要なAPI呼び出しを削減する基盤を整備します。

### 1.1 設計の根拠

**なぜEmbedding取得APIが必要か:**

1. **現状の問題**: `CrystallizationTask.clusterBySimilarity()` (行223) では、各ノードのテキストを毎回 `t.Embedder.EmbedQuery(ctx, text)` で再ベクトル化している。
2. **非効率性**: ノードがVectorStorageに保存された時点で、そのEmbeddingは既にDBに存在している。
3. **コスト**: 外部API（OpenAI等）呼び出しはコストと遅延の両面で高価。
4. **解決策**: DBからEmbeddingを直接取得することで、API呼び出しを回避する。

**DuckDBのベクトルテーブル設計:**

現在の `vectors` テーブルは以下の構造を持っています（`duckdb_storage.go` の SaveEmbedding で確認）：

```sql
CREATE TABLE vectors (
    id VARCHAR,
    group_id VARCHAR,
    collection_name VARCHAR,  -- "Chunk", "Entity_name", "Rule_text" など
    text VARCHAR,
    embedding FLOAT[1536],    -- Embedding配列（1536次元）
    PRIMARY KEY (id, collection_name, group_id)
);
```

このテーブルから `id` と `group_id` を指定してEmbeddingを取得するAPIを提供します。

### 1.2 変更対象ファイル

**ファイル**: `src/pkg/cognee/storage/interfaces.go`

**変更位置**: 95行目と96行目の間（`Search` メソッドの直後、`Close` の直前）

### 1.3 追加するコード（完全版）

```go
	// ========================================
	// Embedding取得操作 (Phase-09追加)
	// ========================================

	// GetEmbeddingByID は、指定されたIDのEmbeddingをvectorsテーブルから取得します。
	// この関数は、既にDBに保存されているEmbeddingを再利用する際に使用します。
	// API呼び出しを削減し、処理効率を向上させます。
	//
	// 引数:
	//   - ctx: コンテキスト
	//   - collectionName: コレクション名（例: "Rule_text", "Entity_name", "Chunk"）
	//   - id: ノードID
	//   - groupID: グループID（パーティション分離用）
	//
	// 返り値:
	//   - []float32: Embedding配列（見つからない場合はnil）
	//   - error: エラーが発生した場合
	//
	// 使用例:
	//   vec, err := vectorStorage.GetEmbeddingByID(ctx, "Rule_text", "rule_123", "user1-dataset1")
	//   if err != nil { return err }
	//   if vec == nil { /* キャッシュミス: Embedderを使用 */ }
	GetEmbeddingByID(ctx context.Context, collectionName, id, groupID string) ([]float32, error)

	// GetEmbeddingsByIDs は、複数IDのEmbeddingを一括取得します。
	// バッチ処理で効率的にEmbeddingを取得する際に使用します。
	// 個別のGetEmbeddingByIDを繰り返すよりも、1回のクエリで取得する方が効率的です。
	//
	// 引数:
	//   - ctx: コンテキスト
	//   - collectionName: コレクション名
	//   - ids: ノードIDのスライス
	//   - groupID: グループID
	//
	// 返り値:
	//   - map[string][]float32: IDをキーとしたEmbeddingのマップ（見つからないIDは含まれない）
	//   - error: エラーが発生した場合
	//
	// 注意:
	//   - 返り値のマップには、DBに存在するIDのみが含まれます
	//   - 存在しないIDはマップに含まれないため、呼び出し側でキャッシュミスを検出できます
	//
	// 使用例:
	//   embeddings, err := vectorStorage.GetEmbeddingsByIDs(ctx, "Rule_text", nodeIDs, groupID)
	//   for _, id := range nodeIDs {
	//       if vec, exists := embeddings[id]; exists {
	//           // キャッシュヒット
	//       } else {
	//           // キャッシュミス: Embedderを使用
	//       }
	//   }
	GetEmbeddingsByIDs(ctx context.Context, collectionName string, ids []string, groupID string) (map[string][]float32, error)
```

### 1.4 変更後のインターフェース構造（確認用）

変更後、`VectorStorage` インターフェースは以下の構造になります：

```go
type VectorStorage interface {
    // メタデータ操作
    SaveData(...)
    Exists(...)
    GetDataByID(...)
    GetDataList(...)
    
    // ベクトル操作
    SaveDocument(...)
    SaveChunk(...)
    SaveEmbedding(...)
    Search(...)
    
    // Embedding取得操作 (Phase-09追加)  <== ここに追加
    GetEmbeddingByID(...)
    GetEmbeddingsByIDs(...)
    
    // クローズ
    Close() error
}
```

### 1.5 ビルド確認

このステップ完了後、ビルドは **失敗します**（実装がないため）。
これは正常な動作です。Step 2で実装を追加するまでエラーが出ます。

---

## Step 2: DuckDBStorage Embedding Cache Implementation

**目的**: Step 1で追加したインターフェースを `DuckDBStorage` に実装します。

### 2.1 変更対象ファイル

**ファイル**: `src/pkg/cognee/db/duckdb/duckdb_storage.go`

**変更位置**: 317行目の `Close` 関数の直前

### 2.2 必要なインポートの追加

**変更位置**: 6-13行目の `import` ブロック

**変更前:**
```go
import (
	"context"
	"database/sql"
	"encoding/json"
	"fmt"

	"mycute/pkg/cognee/storage"
)
```

**変更後:**
```go
import (
	"context"
	"database/sql"
	"encoding/json"
	"fmt"
	"strconv"
	"strings"

	"mycute/pkg/cognee/storage"
)
```

### 2.3 追加するコード（完全版）

`Close()` 関数の直前（317行目付近）に以下を追加します：

```go
// ========================================
// Embedding取得操作 (Phase-09追加)
// ========================================

// GetEmbeddingByID は、指定されたIDのEmbeddingをvectorsテーブルから取得します。
//
// 実装詳細:
//   - vectorsテーブルから id, collection_name, group_id に一致するレコードを検索
//   - embedding カラムの値を []float32 に変換して返す
//   - レコードが見つからない場合は nil を返す（エラーではない）
//
// DuckDBのFLOAT[]の取り扱い:
//   - DuckDBのGo driverでは、FLOAT[]型は文字列として返される場合がある
//   - "[0.1, 0.2, 0.3, ...]" 形式の文字列をパースする必要がある
//
// 引数:
//   - ctx: コンテキスト
//   - collectionName: コレクション名
//   - id: ノードID
//   - groupID: グループID
//
// 返り値:
//   - []float32: Embedding配列（見つからない場合はnil、エラーではない）
//   - error: クエリ実行エラーやパースエラーの場合
func (s *DuckDBStorage) GetEmbeddingByID(ctx context.Context, collectionName, id, groupID string) ([]float32, error) {
	// SQLクエリ: id, collection_name, group_id の3つの条件で検索
	// vectorsテーブルの構造: (id, group_id, collection_name, text, embedding)
	query := `
		SELECT embedding 
		FROM vectors 
		WHERE id = ? AND collection_name = ? AND group_id = ?
	`

	row := s.db.QueryRowContext(ctx, query, id, collectionName, groupID)

	// DuckDBのFLOAT[1536]をGoの[]float32に変換
	// DuckDBのGo driverでは配列型の取り扱いに注意が必要
	var vectorData any
	if err := row.Scan(&vectorData); err != nil {
		if err == sql.ErrNoRows {
			// 見つからない場合はエラーではなくnilを返す
			// これにより、呼び出し側で「キャッシュミス」を検出できる
			return nil, nil
		}
		return nil, fmt.Errorf("failed to scan embedding: %w", err)
	}

	// vectorDataの型に応じて処理
	// DuckDBのドライバーによって返り値の型が異なる場合がある
	switch v := vectorData.(type) {
	case []float32:
		// 直接float32スライスの場合（理想的なケース）
		return v, nil
	case []float64:
		// float64スライスの場合（変換が必要）
		result := make([]float32, len(v))
		for i, f := range v {
			result[i] = float32(f)
		}
		return result, nil
	case []any:
		// anyスライスの場合（要素ごとに変換）
		result := make([]float32, len(v))
		for i, elem := range v {
			switch e := elem.(type) {
			case float32:
				result[i] = e
			case float64:
				result[i] = float32(e)
			default:
				return nil, fmt.Errorf("unexpected element type at index %d: %T", i, elem)
			}
		}
		return result, nil
	case string:
		// 文字列の場合（"[0.1, 0.2, ...]" 形式）
		return parseVectorString(v)
	default:
		return nil, fmt.Errorf("unexpected vector data type: %T", vectorData)
	}
}

// GetEmbeddingsByIDs は、複数IDのEmbeddingを一括取得します。
//
// 実装詳細:
//   - IN句を使用して複数IDを1回のクエリで取得
//   - 結果をマップに格納して返す
//   - 見つからないIDはマップに含まれない
//
// パフォーマンス:
//   - 個別にGetEmbeddingByIDを呼ぶよりも効率的
//   - 1回のクエリで全てのEmbeddingを取得
//   - ネットワークラウンドトリップを削減
//
// 引数:
//   - ctx: コンテキスト
//   - collectionName: コレクション名
//   - ids: ノードIDのスライス
//   - groupID: グループID
//
// 返り値:
//   - map[string][]float32: IDをキーとしたEmbeddingのマップ
//   - error: クエリ実行エラーの場合
func (s *DuckDBStorage) GetEmbeddingsByIDs(ctx context.Context, collectionName string, ids []string, groupID string) (map[string][]float32, error) {
	// 空のスライスの場合は空のマップを返す
	if len(ids) == 0 {
		return make(map[string][]float32), nil
	}

	// プレースホルダーを動的に生成
	// 例: len(ids) = 3 の場合、"?, ?, ?"
	placeholders := make([]string, len(ids))
	args := make([]any, 0, len(ids)+2)

	for i, id := range ids {
		placeholders[i] = "?"
		args = append(args, id)
	}
	// collection_name と group_id を追加
	args = append(args, collectionName, groupID)

	// IN句を使用した効率的なクエリ
	query := fmt.Sprintf(`
		SELECT id, embedding 
		FROM vectors 
		WHERE id IN (%s) AND collection_name = ? AND group_id = ?
	`, strings.Join(placeholders, ", "))

	rows, err := s.db.QueryContext(ctx, query, args...)
	if err != nil {
		return nil, fmt.Errorf("failed to query embeddings: %w", err)
	}
	defer rows.Close()

	result := make(map[string][]float32)

	for rows.Next() {
		var id string
		var vectorData any

		if err := rows.Scan(&id, &vectorData); err != nil {
			return nil, fmt.Errorf("failed to scan row: %w", err)
		}

		// vectorDataをfloat32スライスに変換
		var vector []float32
		switch v := vectorData.(type) {
		case []float32:
			vector = v
		case []float64:
			vector = make([]float32, len(v))
			for i, f := range v {
				vector[i] = float32(f)
			}
		case []any:
			vector = make([]float32, len(v))
			for i, elem := range v {
				switch e := elem.(type) {
				case float32:
					vector[i] = e
				case float64:
					vector[i] = float32(e)
				}
			}
		case string:
			vec, err := parseVectorString(v)
			if err != nil {
				return nil, fmt.Errorf("failed to parse vector for id %s: %w", id, err)
			}
			vector = vec
		default:
			return nil, fmt.Errorf("unexpected vector data type for id %s: %T", id, vectorData)
		}

		result[id] = vector
	}

	if err := rows.Err(); err != nil {
		return nil, fmt.Errorf("error iterating rows: %w", err)
	}

	return result, nil
}

// parseVectorString は、DuckDBから返されるベクトル文字列をパースします。
//
// 入力形式: "[0.1, 0.2, 0.3, ...]"
// 出力: []float32{0.1, 0.2, 0.3, ...}
//
// この関数は、DuckDBのGo driverがFLOAT[]を文字列として返す場合に使用します。
// ドライバーの実装によっては、直接配列として返される場合もあります。
func parseVectorString(s string) ([]float32, error) {
	// 先頭と末尾の角括弧を除去
	s = strings.TrimPrefix(s, "[")
	s = strings.TrimSuffix(s, "]")

	// 空文字列の場合は空のスライスを返す
	if s == "" {
		return nil, nil
	}

	// カンマで分割
	parts := strings.Split(s, ",")
	result := make([]float32, len(parts))

	// 各要素をfloat32に変換
	for i, p := range parts {
		p = strings.TrimSpace(p)
		f, err := strconv.ParseFloat(p, 32)
		if err != nil {
			return nil, fmt.Errorf("invalid float at index %d ('%s'): %w", i, p, err)
		}
		result[i] = float32(f)
	}

	return result, nil
}
```

### 2.4 ビルド確認

このステップ完了後、以下を実行してビルドが成功することを確認します：

```bash
make build
```

**期待される結果**: ビルド成功

---

## Step 3: GraphStorage Interface Extension

**目的**: 孤立ノードを効率的に検出するAPIをインターフェースに追加し、N+1問題を解消する基盤を整備します。

### 3.1 設計の根拠

**なぜ専用APIが必要か:**

1. **現状の問題**: `PruningTask` (行60-107) では、全ノードを取得した後、各ノードについてエッジの有無を個別クエリで確認している（N+1問題）。
   ```go
   // 現在の実装（行87）
   edges, err := t.GraphStorage.GetEdgesByNode(ctx, node.ID, t.GroupID) // N回のクエリ
   ```
2. **計算量**: ノード数Nに対し、N+1回のクエリが必要。
3. **解決策**: CozoDBのDatalogで「エッジを持たないノード」を1クエリで取得する。

**CozoDBの否定クエリ:**

Datalogでは `not` キーワードを使用して否定条件を表現できます。

```datalog
?[id, type, props] := 
    *nodes[id, group_id, type, props],
    group_id = $group_id,
    not *edges[id, _, group_id, _, _],  # このノードがsourceのエッジがない
    not *edges[_, id, group_id, _, _]   # このノードがtargetのエッジがない
```

### 3.2 変更対象ファイル

**ファイル**: `src/pkg/cognee/storage/interfaces.go`

**変更位置**: 206行目（`GetEdgesByNode` の直後）

### 3.3 インポートの確認

`interfaces.go` で `time.Duration` を使用するため、`time` パッケージがインポートされていることを確認します。

**現在のインポート (行5-8):**
```go
import (
	"context"
	"time"
)
```

`time` は既にインポート済みのため、変更不要です。

### 3.4 追加するコード（完全版）

206行目の `GetEdgesByNode` の直後に以下を追加します：

```go
	// ========================================
	// 効率化API (Phase-09追加)
	// ========================================

	// GetOrphanNodes は、エッジを持たない孤立ノードを取得します。
	// この関数は、グラフのガベージコレクション（Pruning）で不要ノードを特定する際に使用します。
	// 1回のクエリで全孤立ノードを取得することで、N+1問題を回避します。
	//
	// 引数:
	//   - ctx: コンテキスト
	//   - groupID: グループID
	//   - gracePeriod: 作成からこの期間内のノードは除外（誤削除防止）
	//
	// 返り値:
	//   - []*Node: 孤立ノードのスライス
	//   - error: エラーが発生した場合
	//
	// 孤立ノードの定義:
	//   - InDegree = 0（このノードをターゲットとするエッジがない）
	//   - OutDegree = 0（このノードをソースとするエッジがない）
	//
	// gracePeriodの役割:
	//   - 作成直後のノードはまだエッジが張られていない可能性がある
	//   - 誤削除を防ぐため、作成から一定時間経過したノードのみを対象とする
	//
	// 前提条件:
	//   - ノードの作成日時は properties["created_at"] にRFC3339形式で格納されていること
	//
	// 使用例:
	//   orphans, err := graphStorage.GetOrphanNodes(ctx, groupID, 1*time.Hour)
	//   if err != nil { return err }
	//   for _, node := range orphans {
	//       graphStorage.DeleteNode(ctx, node.ID, groupID)
	//   }
	GetOrphanNodes(ctx context.Context, groupID string, gracePeriod time.Duration) ([]*Node, error)
```

### 3.5 変更後のインターフェース構造（確認用）

変更後、`GraphStorage` インターフェースは以下の構造になります：

```go
type GraphStorage interface {
    AddNodes(...)
    AddEdges(...)
    GetTriplets(...)
    StreamDocumentChunks(...)
    GetDocumentChunkCount(...)
    GetNodesByType(...)
    GetNodesByEdge(...)
    UpdateEdgeWeight(...)
    UpdateEdgeMetrics(...)
    DeleteEdge(...)
    DeleteNode(...)
    GetEdgesByNode(...)
    
    GetOrphanNodes(...)  // <== ここに追加
    
    EnsureSchema(...)
    Close() error
}
```

### 3.6 ビルド確認

このステップ完了後、ビルドは **失敗します**（実装がないため）。
これは正常な動作です。Step 4で実装を追加するまでエラーが出ます。

---

## Step 4: CozoStorage Orphan Detection Implementation

**目的**: Step 3で追加したインターフェースを `CozoStorage` に実装します。

### 4.1 変更対象ファイル

**ファイル**: `src/pkg/cognee/db/cozodb/cozo_storage.go`

**変更位置**: ファイル末尾（`Close()` 関数の直前、または `DeleteNode` の後）

### 4.2 必要なインポートの確認

`cozo_storage.go` で `time` パッケージがインポートされていることを確認します。
既にインポートされていない場合は追加してください。

### 4.3 追加するコード（完全版）

```go
// GetOrphanNodes は、エッジを持たない孤立ノードを取得します。
//
// 実装詳細:
//   - Datalogの否定(not)を使用して、edgesリレーションに存在しないノードを検出
//   - gracePeriodを考慮し、作成直後のノードは除外
//   - 1回のクエリで全ての孤立ノードを取得（N+1問題の解消）
//
// Datalogクエリの解説:
//   ?[id, type, props] :=
//       *nodes[id, group_id, type, props],      <- nodesリレーションからすべてのノードを取得
//       group_id = %s,                          <- 指定されたgroup_idでフィルタ
//       created_at = get(props, "created_at", ""), <- propsからcreated_atを抽出
//       created_at != "",                       <- created_atが存在するものだけ
//       created_at < %s,                        <- 猶予期間外のもののみ対象
//       not *edges[id, _, %s, _, _],            <- このIDがsource_idとして存在しない
//       not *edges[_, id, %s, _, _]             <- このIDがtarget_idとして存在しない
//
// CozoDBの否定(not)演算子について:
//   - Datalogの否定は「閉世界仮説」に基づく
//   - 「DBに存在しない = False」と解釈される
//   - not内で使用する変数は、その前に束縛されている必要がある
//     (ここでは id と group_id が事前に束縛されている)
//
// 引数:
//   - ctx: コンテキスト
//   - groupID: グループID
//   - gracePeriod: 猶予期間（この時間より前に作成されたノードのみ対象）
//
// 返り値:
//   - []*storage.Node: 孤立ノードのスライス
//   - error: クエリ実行エラーの場合
func (s *CozoStorage) GetOrphanNodes(ctx context.Context, groupID string, gracePeriod time.Duration) ([]*storage.Node, error) {
	// 猶予期間のカットオフ時刻を計算
	// これより前に作成されたノードのみが削除対象
	cutoffTime := time.Now().Add(-gracePeriod).Format(time.RFC3339)

	// グループIDとカットオフ時刻をクォート
	// SQLインジェクション対策としてシングルクォートをエスケープ
	quotedGroupID := fmt.Sprintf("'%s'", strings.ReplaceAll(groupID, "'", "\\'"))
	quotedCutoffTime := fmt.Sprintf("'%s'", cutoffTime)

	// 孤立ノード検出クエリ
	//
	// クエリの各行の解説:
	// 1. ?[id, type, props] := 結果として返すカラム
	// 2. *nodes[id, group_id, type, props] ノードテーブルから読み取り
	// 3. group_id = %s 指定されたグループIDでフィルタ
	// 4. created_at = get(props, "created_at", "") propsからcreated_atを抽出（なければ空文字）
	// 5. created_at != "" created_atが存在するノードのみ
	// 6. created_at < %s 猶予期間外のノードのみ
	// 7. not *edges[id, _, %s, _, _] このノードがソースとなるエッジがない
	// 8. not *edges[_, id, %s, _, _] このノードがターゲットとなるエッジがない
	query := fmt.Sprintf(`
		?[id, type, props] := 
			*nodes[id, group_id, type, props],
			group_id = %s,
			created_at = get(props, "created_at", ""),
			created_at != "",
			created_at < %s,
			not *edges[id, _, %s, _, _],
			not *edges[_, id, %s, _, _]
	`, quotedGroupID, quotedCutoffTime, quotedGroupID, quotedGroupID)

	res, err := s.db.Run(query, nil)
	if err != nil {
		return nil, fmt.Errorf("failed to query orphan nodes: %w", err)
	}

	var orphans []*storage.Node

	for _, row := range res.Rows {
		// row: [id, type, props]
		id := row[0].(string)
		typ := row[1].(string)

		// propsをmap[string]anyに変換
		var props map[string]any
		if p, ok := row[2].(map[string]any); ok {
			props = p
		} else if pStr, ok := row[2].(string); ok {
			json.Unmarshal([]byte(pStr), &props)
		}

		orphans = append(orphans, &storage.Node{
			ID:         id,
			GroupID:    groupID,
			Type:       typ,
			Properties: props,
		})
	}

	return orphans, nil
}
```

### 4.4 CozoDBの否定クエリに関する注意事項

> [!WARNING]
> **CozoDBの `not` 演算子の制限**
> - `not` は「閉世界仮説」に基づく否定です。つまり、「DBに存在しない = False」と解釈されます。
> - 変数のスコープに注意が必要です。`not` 内で使用する変数は、その前に束縛（バインド）されている必要があります。
> - パフォーマンスへの影響: 大規模データの場合、`not` クエリは重くなる可能性があります。

> [!IMPORTANT]
> **トラブルシューティング**
> 
> クエリエラーが発生した場合、以下を確認してください：
> 1. `edges` リレーションのカラム順序: `[source_id, target_id, group_id, type, properties]`
> 2. `nodes` リレーションのカラム順序: `[id, group_id, type, properties]`
> 3. `created_at` プロパティが RFC3339 形式で保存されていること

### 4.5 ビルド確認

このステップ完了後、以下を実行してビルドが成功することを確認します：

```bash
make build
```

**期待される結果**: ビルド成功

---

## Step 5: CrystallizationTask Optimization

**目的**: `CrystallizationTask` を改修し、Embedderへの不要なAPI呼び出しを削減します。

### 5.1 変更対象ファイル

**ファイル**: `src/pkg/cognee/tasks/metacognition/crystallization_task.go`

**変更対象メソッド**: `clusterBySimilarity` (行200-291)

### 5.2 変更の概要

**変更前の問題箇所 (行219-226):**
```go
// Embeddingを取得（VectorStorageに保存されているはずだが、ここではEmbedderを使用）
// Note: 本番ではVectorStorage.Searchを使用する方が効率的だが、
// ここではメモリ上のノードリストに対して処理を行うため、Embedderを使用する。
// 既にEmbeddingが保存されている場合はそれを使うべき。
vector, err := t.Embedder.EmbedQuery(ctx, text)  // <== これが問題
```

**解決策:**
1. まずVectorStorageからバッチでEmbeddingを取得
2. キャッシュにないものだけEmbedderで計算

### 5.3 置換するコード（完全版）

行200-291の `clusterBySimilarity` メソッド全体を以下で置換します：

```go
// clusterBySimilarity は、ノードを類似度でクラスタリングします。
// ベクトル検索を使用して近傍グラフを構築し、連結成分分解を行います。
//
// Phase-09最適化:
//   - VectorStorageからEmbeddingをバッチ取得（キャッシュ活用）
//   - キャッシュミスの場合のみEmbedderを使用
//   - API呼び出し回数を大幅に削減
func (t *CrystallizationTask) clusterBySimilarity(ctx context.Context, nodes []*storage.Node, threshold float64) [][]*storage.Node {
	if len(nodes) == 0 {
		return nil
	}

	// ========================================
	// Step 1: ノードIDからインデックスへのマップ作成
	// ========================================
	nodeIndex := make(map[string]int)
	nodeIDs := make([]string, len(nodes))
	for i, n := range nodes {
		nodeIndex[n.ID] = i
		nodeIDs[i] = n.ID
	}

	// ========================================
	// Step 2: Embeddingをバッチ取得（キャッシュ活用）
	// ========================================
	// VectorStorageから既存のEmbeddingを一括取得
	// コレクション名は "Rule_text" を使用（Rule ノードの text フィールドに対応）
	cachedEmbeddings, err := t.VectorStorage.GetEmbeddingsByIDs(ctx, "Rule_text", nodeIDs, t.GroupID)
	if err != nil {
		// エラーの場合は空のマップで続行（フォールバックでEmbedderを使用）
		fmt.Printf("CrystallizationTask: Warning - failed to fetch cached embeddings: %v\n", err)
		cachedEmbeddings = make(map[string][]float32)
	}

	// キャッシュヒット率をログ出力
	cacheHitCount := len(cachedEmbeddings)
	fmt.Printf("CrystallizationTask: Embedding cache hit: %d/%d (%.1f%%)\n",
		cacheHitCount, len(nodes), float64(cacheHitCount)/float64(len(nodes))*100)

	// ========================================
	// Step 3: キャッシュミスのノードのみEmbedderで計算
	// ========================================
	embeddings := make(map[string][]float32)
	cacheMissCount := 0

	for _, node := range nodes {
		// キャッシュにあればそれを使用
		if vec, exists := cachedEmbeddings[node.ID]; exists {
			embeddings[node.ID] = vec
			continue
		}

		// キャッシュにない場合はEmbedderで計算
		text, ok := node.Properties["text"].(string)
		if !ok {
			continue
		}

		vec, err := t.Embedder.EmbedQuery(ctx, text)
		if err != nil {
			fmt.Printf("CrystallizationTask: Warning - failed to embed text for node %s: %v\n", node.ID, err)
			continue
		}
		embeddings[node.ID] = vec
		cacheMissCount++
	}

	if cacheMissCount > 0 {
		fmt.Printf("CrystallizationTask: Computed %d embeddings via API (cache miss)\n", cacheMissCount)
	}

	// ========================================
	// Step 4: 隣接リストの構築
	// ========================================
	adj := make([][]int, len(nodes))

	for i, node := range nodes {
		vec, exists := embeddings[node.ID]
		if !exists {
			continue
		}

		// VectorStorageで類似検索
		results, err := t.VectorStorage.Search(ctx, "Rule_text", vec, 10, t.GroupID)
		if err != nil {
			continue
		}

		for _, res := range results {
			// 類似度が閾値以上かチェック
			// DuckDBのarray_cosine_similarityは類似度を返す（大きいほど類似）
			// res.Distance >= threshold なら類似
			if res.Distance < threshold {
				continue
			}

			// 検索結果のIDが現在の処理対象ノードリストに含まれているか確認
			if idx, exists := nodeIndex[res.ID]; exists {
				if idx != i { // 自分自身は除外
					adj[i] = append(adj[i], idx)
					adj[idx] = append(adj[idx], i) // 無向グラフとして扱う
				}
			}
		}
	}

	// ========================================
	// Step 5: 連結成分分解（BFS）
	// ========================================
	visited := make([]bool, len(nodes))
	var clusters [][]*storage.Node

	for i := 0; i < len(nodes); i++ {
		if visited[i] {
			continue
		}

		var cluster []*storage.Node
		queue := []int{i}
		visited[i] = true

		for len(queue) > 0 {
			curr := queue[0]
			queue = queue[1:]
			cluster = append(cluster, nodes[curr])

			for _, neighbor := range adj[curr] {
				if !visited[neighbor] {
					visited[neighbor] = true
					queue = append(queue, neighbor)
				}
			}
		}

		if len(cluster) > 0 {
			clusters = append(clusters, cluster)
		}
	}

	return clusters
}
```

### 5.4 変更のポイント

| 項目 | 変更前 | 変更後 |
|------|--------|--------|
| Embedding取得 | 毎回 `Embedder.EmbedQuery()` | まず `VectorStorage.GetEmbeddingsByIDs()` |
| API呼び出し | N回（ノード数） | 0〜数回（キャッシュミスのみ） |
| ログ出力 | なし | キャッシュヒット率を出力 |

### 5.5 ビルド確認

このステップ完了後、以下を実行してビルドが成功することを確認します：

```bash
make build
```

**期待される結果**: ビルド成功

---

## Step 6: PruningTask Optimization

**目的**: `PruningTask` を改修し、N+1問題を解消します。

### 6.1 変更対象ファイル

**ファイル**: `src/pkg/cognee/tasks/metacognition/pruning_task.go`

**変更対象メソッド**: `PruneOrphans` (行33-57) と `pruneNodesByType` (行60-108)

### 6.2 変更の概要

**変更前の問題箇所 (行44-100):**
```go
// 各タイプごとに全ノードを取得し、個別にエッジを確認
for _, nodeType := range targetTypes {
    nodes, _ := t.GraphStorage.GetNodesByType(ctx, nodeType, t.GroupID)
    for _, node := range nodes {
        edges, _ := t.GraphStorage.GetEdgesByNode(ctx, node.ID, t.GroupID) // N回のクエリ
        // ...
    }
}
```

**解決策:**
1. `GetOrphanNodes` を使用して1クエリで孤立ノードを取得
2. ループ内でのエッジ確認を不要にする

### 6.3 置換するコード（完全版）

`PruneOrphans` メソッドと `pruneNodesByType` メソッド全体（行33-108）を以下で置換します：

```go
// PruneOrphans は、孤立ノードを特定し、削除します。
//
// Phase-09最適化:
//   - GetOrphanNodesを使用して1クエリで孤立ノードを取得
//   - N+1問題を解消
//   - 処理時間を大幅に短縮
//
// 処理フロー:
//   1. GetOrphanNodesで孤立ノードを取得（GracePeriod考慮済み）
//   2. 取得したノードを順次削除
func (t *PruningTask) PruneOrphans(ctx context.Context) error {
	fmt.Printf("PruningTask: Starting pruning for group %s (GracePeriod: %v)\n", t.GroupID, t.GracePeriod)

	// ========================================
	// 1クエリで全孤立ノードを取得
	// ========================================
	// GetOrphanNodesは、CozoDB側で以下を実行:
	// - エッジを持たないノードを検出
	// - GracePeriod内のノードを除外
	orphans, err := t.GraphStorage.GetOrphanNodes(ctx, t.GroupID, t.GracePeriod)
	if err != nil {
		return fmt.Errorf("PruningTask: failed to get orphan nodes: %w", err)
	}

	if len(orphans) == 0 {
		fmt.Println("PruningTask: No orphan nodes found")
		return nil
	}

	fmt.Printf("PruningTask: Found %d orphan nodes to delete\n", len(orphans))

	// ========================================
	// 孤立ノードを削除
	// ========================================
	deletedCount := 0
	failedCount := 0

	for _, node := range orphans {
		if err := t.GraphStorage.DeleteNode(ctx, node.ID, t.GroupID); err != nil {
			fmt.Printf("PruningTask: Warning - failed to delete node %s (type: %s): %v\n", node.ID, node.Type, err)
			failedCount++
			continue
		}
		deletedCount++
	}

	fmt.Printf("PruningTask: Completed. Deleted %d nodes, failed %d nodes\n", deletedCount, failedCount)
	return nil
}

// pruneNodesByType は後方互換性のために残しますが、Phase-09以降は使用しません。
// 代わりに GetOrphanNodes を使用します。
//
// Deprecated: Use PruneOrphans with GetOrphanNodes instead.
func (t *PruningTask) pruneNodesByType(ctx context.Context, nodeType string) (int, error) {
	// この関数はPhase-09では使用しませんが、
	// 既存のテストやデバッグ目的で残しておきます。
	
	// 指定されたタイプのノードを取得
	nodes, err := t.GraphStorage.GetNodesByType(ctx, nodeType, t.GroupID)
	if err != nil {
		return 0, err
	}

	deletedCount := 0
	now := time.Now()

	for _, node := range nodes {
		// 1. GracePeriodのチェック
		var createdAt time.Time
		if createdStr, ok := node.Properties["created_at"].(string); ok {
			if parsed, err := time.Parse(time.RFC3339, createdStr); err == nil {
				createdAt = parsed
			}
		}

		if createdAt.IsZero() || now.Sub(createdAt) < t.GracePeriod {
			continue
		}

		// 2. エッジの有無をチェック
		edges, err := t.GraphStorage.GetEdgesByNode(ctx, node.ID, t.GroupID)
		if err != nil {
			continue
		}

		if len(edges) == 0 {
			if err := t.GraphStorage.DeleteNode(ctx, node.ID, t.GroupID); err != nil {
				fmt.Printf("PruningTask: Failed to delete node %s: %v\n", node.ID, err)
			} else {
				deletedCount++
			}
		}
	}

	if deletedCount > 0 {
		fmt.Printf("PruningTask: Deleted %d orphan nodes of type %s\n", deletedCount, nodeType)
	}

	return deletedCount, nil
}
```

### 6.4 変更のポイント

| 項目 | 変更前 | 変更後 |
|------|--------|--------|
| クエリ回数 | N+1回 | 1回 |
| ノードタイプ | 個別にループ | 全タイプを一括取得 |
| GracePeriod | アプリ側で判定 | CozoDB側で判定 |
| 処理時間 | O(N × M) | O(1) + O(K) |

### 6.5 ビルド確認

このステップ完了後、以下を実行してビルドが成功することを確認します：

```bash
make build
```

**期待される結果**: ビルド成功

---

## Step 7: Performance Verification

**目的**: 実装した最適化の効果を定量的に測定し、期待通りの改善が達成されていることを確認します。

### 7.1 変更対象ファイル

**ファイル**: `src/main.go`

**変更位置**: `default:` ケースの直前（既存のテストコマンドの後）

### 7.2 追加するコード（完全版）

```go
	case "benchmark-optimization":
		// Phase-09最適化のベンチマークテスト
		log.Println("--- Phase 9 Benchmark: Optimization Performance ---")
		benchOptCmd := flag.NewFlagSet("benchmark-optimization", flag.ExitOnError)
		nodeCountPtr := benchOptCmd.Int("n", 100, "Number of test nodes")
		datasetPtr := benchOptCmd.String("d", "test_dataset", "Dataset name")
		userPtr := benchOptCmd.String("u", "test_user", "User ID")
		benchOptCmd.Parse(os.Args[2:])

		groupID := *userPtr + "-" + *datasetPtr
		nodeCount := *nodeCountPtr

		log.Printf("Benchmark settings: %d nodes, groupID: %s", nodeCount, groupID)

		// ========================================
		// 1. テストデータ作成
		// ========================================
		log.Printf("Creating %d test nodes...", nodeCount)
		testNodes := make([]*storage.Node, nodeCount)
		for i := 0; i < nodeCount; i++ {
			testNodes[i] = &storage.Node{
				ID:      fmt.Sprintf("bench_node_%d", i),
				GroupID: groupID,
				Type:    "Rule",
				Properties: map[string]any{
					"text":       fmt.Sprintf("Test rule %d for benchmarking optimization performance", i),
					"created_at": time.Now().Add(-2 * time.Hour).Format(time.RFC3339),
				},
			}
		}
		if err := cogneeService.GraphStorage.AddNodes(ctx, testNodes); err != nil {
			log.Fatalf("Failed to create test nodes: %v", err)
		}
		log.Printf("✅ Created %d test nodes", nodeCount)

		// ========================================
		// 2. GetEmbeddingsByIDs ベンチマーク
		// ========================================
		nodeIDs := make([]string, nodeCount)
		for i := 0; i < nodeCount; i++ {
			nodeIDs[i] = testNodes[i].ID
		}

		log.Println("Benchmarking GetEmbeddingsByIDs...")
		startEmbedding := time.Now()
		embeddings, err := cogneeService.VectorStorage.GetEmbeddingsByIDs(ctx, "Rule_text", nodeIDs, groupID)
		embeddingDuration := time.Since(startEmbedding)
		if err != nil {
			log.Printf("⚠️ GetEmbeddingsByIDs returned error (expected if vectors not stored): %v", err)
		} else {
			log.Printf("✅ GetEmbeddingsByIDs (%d IDs): %v, found %d embeddings", nodeCount, embeddingDuration, len(embeddings))
		}

		// ========================================
		// 3. GetOrphanNodes ベンチマーク
		// ========================================
		log.Println("Benchmarking GetOrphanNodes...")
		startOrphan := time.Now()
		orphans, err := cogneeService.GraphStorage.GetOrphanNodes(ctx, groupID, 1*time.Hour)
		orphanDuration := time.Since(startOrphan)
		if err != nil {
			log.Fatalf("❌ GetOrphanNodes failed: %v", err)
		}
		log.Printf("✅ GetOrphanNodes: %v (found %d orphans)", orphanDuration, len(orphans))

		// ========================================
		// 4. 比較: 旧実装（N+1問題）
		// ========================================
		log.Println("Benchmarking old implementation (N+1 problem)...")
		startOld := time.Now()
		oldOrphanCount := 0
		for _, node := range testNodes {
			edges, _ := cogneeService.GraphStorage.GetEdgesByNode(ctx, node.ID, groupID)
			if len(edges) == 0 {
				oldOrphanCount++
			}
		}
		oldDuration := time.Since(startOld)
		log.Printf("⏱️ Old implementation (N+1): %v (found %d orphans)", oldDuration, oldOrphanCount)

		// ========================================
		// 5. 結果サマリー
		// ========================================
		log.Println("========================================")
		log.Println("Benchmark Results:")
		log.Printf("  GetEmbeddingsByIDs: %v", embeddingDuration)
		log.Printf("  GetOrphanNodes (new): %v", orphanDuration)
		log.Printf("  N+1 pattern (old): %v", oldDuration)
		if orphanDuration < oldDuration {
			speedup := float64(oldDuration) / float64(orphanDuration)
			log.Printf("  Speedup: %.1fx faster", speedup)
		}
		log.Println("========================================")

		// ========================================
		// 6. クリーンアップ
		// ========================================
		log.Println("Cleaning up test data...")
		for _, node := range testNodes {
			cogneeService.GraphStorage.DeleteNode(ctx, node.ID, groupID)
		}
		log.Println("✅ Benchmark completed and test data cleaned up")
```

### 7.3 ビルドと実行確認

```bash
# ビルド
make build

# ベンチマーク実行
./dist/mycute-darwin-arm64 benchmark-optimization -n 100
```

**期待される結果例:**
```
--- Phase 9 Benchmark: Optimization Performance ---
Benchmark settings: 100 nodes, groupID: test_user-test_dataset
Creating 100 test nodes...
✅ Created 100 test nodes
Benchmarking GetEmbeddingsByIDs...
✅ GetEmbeddingsByIDs (100 IDs): 5ms, found 0 embeddings
Benchmarking GetOrphanNodes...
✅ GetOrphanNodes: 10ms (found 100 orphans)
Benchmarking old implementation (N+1 problem)...
⏱️ Old implementation (N+1): 500ms (found 100 orphans)
========================================
Benchmark Results:
  GetEmbeddingsByIDs: 5ms
  GetOrphanNodes (new): 10ms
  N+1 pattern (old): 500ms
  Speedup: 50.0x faster
========================================
Cleaning up test data...
✅ Benchmark completed and test data cleaned up
```

---

## 8. 既存テストの動作確認

Phase-08で追加したテストコマンドが引き続き動作することを確認します。

```bash
# 代謝サイクルのテスト
./dist/mycute-darwin-arm64 test-metabolism

# 孤立ノード削除のテスト
./dist/mycute-darwin-arm64 test-pruning

# 知識結晶化のテスト
./dist/mycute-darwin-arm64 test-crystallization
```

---

## 9. トラブルシューティング

### 9.1 コンパイルエラー

**エラー**: `undefined: storage.VectorStorage.GetEmbeddingByID`

**原因**: Step 1のインターフェース追加が漏れている

**解決策**: `interfaces.go` に `GetEmbeddingByID` と `GetEmbeddingsByIDs` が追加されていることを確認

---

**エラー**: `DuckDBStorage does not implement storage.VectorStorage`

**原因**: Step 2の実装が漏れている

**解決策**: `duckdb_storage.go` に `GetEmbeddingByID` と `GetEmbeddingsByIDs` の実装を追加

---

**エラー**: `CozoStorage does not implement storage.GraphStorage`

**原因**: Step 4の実装が漏れている

**解決策**: `cozo_storage.go` に `GetOrphanNodes` の実装を追加

---

### 9.2 ランタイムエラー

**エラー**: `failed to query orphan nodes: ...`

**原因**: CozoDBのDatalogクエリ構文エラー

**解決策**:
1. `edges` と `nodes` のカラム順序を確認
2. `not` 演算子内の変数が事前に束縛されていることを確認

---

**エラー**: `unexpected vector data type: ...`

**原因**: DuckDBからの返り値の型が想定と異なる

**解決策**:
1. `parseVectorString` 関数が正しく実装されていることを確認
2. DuckDBのバージョンとドライバーの互換性を確認

---

### 9.3 パフォーマンス問題

**症状**: `GetOrphanNodes` が期待より遅い

**解決策**:
1. `nodes` テーブルに `group_id` のインデックスがあることを確認
2. `edges` テーブルに適切なインデックスがあることを確認

---

## Phase-09 実装チェックリスト

### Step 1: VectorStorage Interface Extension
- [ ] `interfaces.go` の行95と96の間に `GetEmbeddingByID` を追加
- [ ] `interfaces.go` の行95と96の間に `GetEmbeddingsByIDs` を追加
- [ ] コメントが正しく記述されていることを確認

### Step 2: DuckDBStorage Implementation
- [ ] `duckdb_storage.go` のインポートに `strconv` と `strings` を追加
- [ ] `duckdb_storage.go` の `Close()` 直前に `GetEmbeddingByID` を実装
- [ ] `duckdb_storage.go` の `Close()` 直前に `GetEmbeddingsByIDs` を実装
- [ ] `parseVectorString` ヘルパー関数を実装
- [ ] `make build` が成功することを確認

### Step 3: GraphStorage Interface Extension
- [ ] `interfaces.go` の行206直後に `GetOrphanNodes` を追加
- [ ] `time` パッケージがインポートされていることを確認

### Step 4: CozoStorage Implementation
- [ ] `cozo_storage.go` の `time` パッケージインポートを確認
- [ ] `cozo_storage.go` に `GetOrphanNodes` を実装
- [ ] Datalogクエリの構文が正しいことを確認
- [ ] `make build` が成功することを確認

### Step 5: CrystallizationTask Optimization
- [ ] `crystallization_task.go` の `clusterBySimilarity` を置換
- [ ] キャッシュヒット率のログ出力を確認
- [ ] `make build` が成功することを確認

### Step 6: PruningTask Optimization
- [ ] `pruning_task.go` の `PruneOrphans` を置換
- [ ] `pruneNodesByType` をDeprecatedとしてマーク
- [ ] `make build` が成功することを確認

### Step 7: Performance Verification
- [ ] `main.go` に `benchmark-optimization` コマンドを追加
- [ ] `make build` が成功することを確認
- [ ] `./dist/mycute-darwin-arm64 benchmark-optimization` が正常に実行されることを確認

### 最終確認
- [ ] `test-metabolism` が正常に動作すること
- [ ] `test-pruning` が正常に動作すること
- [ ] `test-crystallization` が正常に動作すること
- [ ] `benchmark-optimization` でスピードアップが確認できること
