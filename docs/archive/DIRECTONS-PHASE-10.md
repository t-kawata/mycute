# Cognee Go Implementation: Phase-10 Development Directives
# Kuzu Database Integration (グラフ+ベクトル統合)

## 0. はじめに (Introduction)

本ドキュメントは、**Phase-10: Kuzu Database Integration** の実装を「迷いなく」「確実に」遂行するための詳細な開発指示書です。

> [!IMPORTANT]
> **Phase-10のゴール**
> DuckDB + CozoDB で実現されている全データベース操作を、単一のKuzuデータベースで完全再現すること。
> KuzuはグラフDB + ベクトルDBの両機能を持つため、2つのDBを1つに統合できる。

> [!CAUTION]
> **危険度の高いフェーズ**
> - CGOビルドの複雑さ（go-kuzu）
> - 既存のDuckDB+CozoDB実装との共存
> - 大規模なリファクタリング
> 
> **Phase-10は以下のサブフェーズに分割して慎重に進める**

---

## 1. サブフェーズ構成

| サブフェーズ | 内容 | 成功条件 |
|-------------|------|---------|
| **Phase-10A** | Kuzuビルド基盤確立 | `make build` と `make build-linux-amd64` がエラーなしで成功 |
| **Phase-10B** | KuzuStorage統合インターフェース設計 | 新インターフェース定義、CogneeConfig拡張 |
| **Phase-10C** | KuzuStorageスキーマ実装 | nodes/edges/vectors対応スキーマ |
| **Phase-10D** | VectorStorage機能実装 | DuckDBの全機能をKuzuで再現 |
| **Phase-10E** | GraphStorage機能実装 | CozoDBの全機能をKuzuで再現 |
| **Phase-10F** | モード切替統合・テスト | CogneeServiceのDB切替、全テスト合格 |

---

## 2. アーキテクチャ設計

### 2.1 現行構成 (DuckDB + CozoDB)

```
CogneeService
├── VectorStorage (interface)
│   └── DuckDBStorage (implementation)
│       ├── data テーブル
│       ├── documents テーブル
│       ├── chunks テーブル
│       └── vectors テーブル
└── GraphStorage (interface)
    └── CozoStorage (implementation)
        ├── nodes リレーション
        └── edges リレーション
```

### 2.2 新構成 (Kuzu統合モード)

```
CogneeService
├── VectorStorage (interface)
│   ├── DuckDBStorage (既存実装)
│   └── KuzuStorage   (新規実装) *same instance*
└── GraphStorage (interface)
    ├── CozoStorage   (既存実装)
    └── KuzuStorage   (新規実装) *same instance*
```

**重要**: Kuzuモードでは、`KuzuStorage` が `VectorStorage` と `GraphStorage` の両方のインターフェースを実装する。
単一のKuzuインスタンスが両機能を提供。

### 2.3 モード切替設計

```go
// CogneeConfig (config/settings.go または cognee.go)
type CogneeConfig struct {
    // 既存フィールド...
    
    // Phase-10追加: データベースモード
    // "duckdb+cozodb" (デフォルト、既存動作)
    // "kuzu" (新規)
    DatabaseMode string
    
    // Kuzu専用設定
    KuzuDatabasePath string // 例: "db/kuzu"
}
```

---

## 3. go-kuzu 依存関係

### 3.1 依存ライブラリ

```go
import (
    "github.com/t-kawata/go-kuzu/pkg/kuzu"
)
```

### 3.2 go.mod追加

```bash
go get github.com/t-kawata/go-kuzu@latest
```

### 3.3 CGOビルド要件

go-kuzuはCGOを使用するため、ビルド時に以下が必要：

- **macOS (darwin)**: Xcodeコマンドラインツール
- **Linux (amd64)**: クロスコンパイルツールチェーン

現在の `sh/build` スクリプトを参照し、Kuzu用の設定を追加する必要がある。

---

## 4. Kuzu スキーマ設計

### 4.1 ノードテーブル

```cypher
// Data (DuckDB dataテーブル相当)
CREATE NODE TABLE Data (
    id STRING PRIMARY KEY,
    group_id STRING,
    name STRING,
    raw_data_location STRING,
    original_data_location STRING,
    extension STRING,
    mime_type STRING,
    content_hash STRING,
    owner_id STRING,
    created_at TIMESTAMP
)

// Document (DuckDB documentsテーブル相当)
CREATE NODE TABLE Document (
    id STRING PRIMARY KEY,
    group_id STRING,
    data_id STRING,
    text STRING,
    metadata STRING  // JSON as STRING
)

// Chunk (DuckDB chunksテーブル相当)
CREATE NODE TABLE Chunk (
    id STRING PRIMARY KEY,
    group_id STRING,
    document_id STRING,
    text STRING,
    token_count INT32,
    chunk_index INT32
)

// Vector (DuckDB vectorsテーブル相当)
CREATE NODE TABLE Vector (
    id STRING PRIMARY KEY,
    group_id STRING,
    collection_name STRING,
    text STRING,
    embedding FLOAT_LIST
)

// Node (CozoDB nodesリレーション相当)
CREATE NODE TABLE GraphNode (
    id STRING PRIMARY KEY,
    group_id STRING,
    type STRING,
    properties STRING  // JSON as STRING
)
```

### 4.2 リレーションシップテーブル

```cypher
// Edge (CozoDB edgesリレーション相当)
CREATE REL TABLE Edge (
    FROM GraphNode TO GraphNode,
    group_id STRING,
    type STRING,
    properties STRING  // JSON as STRING
)

// DataToDocument関連
CREATE REL TABLE has_document (
    FROM Data TO Document
)

// DocumentToChunk関連
CREATE REL TABLE has_chunk (
    FROM Document TO Chunk
)
```

### 4.3 インデックス

```cypher
// group_id による高速フィルタリング
CREATE INDEX ON Data (group_id)
CREATE INDEX ON Document (group_id)
CREATE INDEX ON Chunk (group_id)
CREATE INDEX ON Vector (group_id, collection_name)
CREATE INDEX ON GraphNode (group_id, type)

// ベクトル検索用 (HNSW)
CREATE INDEX ON Vector (embedding) USING HNSW
```

---

## 5. KuzuStorage 実装設計

### 5.1 構造体定義

```go
// src/pkg/cognee/db/kuzu/kuzu_storage.go

package kuzu

import (
    "context"
    "github.com/t-kawata/go-kuzu/pkg/kuzu"
    "mycute/pkg/cognee/storage"
)

// KuzuStorage は、Kuzuを使用した統合ストレージ実装です。
// VectorStorage と GraphStorage の両インターフェースを実装します。
type KuzuStorage struct {
    db   *kuzu.Database
    conn *kuzu.Connection
}

// インターフェース実装の確認
var _ storage.VectorStorage = (*KuzuStorage)(nil)
var _ storage.GraphStorage = (*KuzuStorage)(nil)

func NewKuzuStorage(dbPath string) (*KuzuStorage, error) {
    db, err := kuzu.NewDatabase(dbPath)
    if err != nil {
        return nil, err
    }
    
    conn, err := db.Connect()
    if err != nil {
        db.Close()
        return nil, err
    }
    
    return &KuzuStorage{
        db:   db,
        conn: conn,
    }, nil
}
```

### 5.2 VectorStorage メソッド

以下のメソッドを実装する必要がある：

| メソッド | 機能 | Kuzuでの実装 |
|---------|------|-------------|
| `SaveData` | メタデータ保存 | `CREATE (d:Data {...})` |
| `Exists` | データ存在確認 | `MATCH (d:Data) WHERE ...` |
| `GetDataByID` | データ取得 | `MATCH (d:Data {id: $id})` |
| `GetDataList` | データ一覧 | `MATCH (d:Data) WHERE group_id = $gid` |
| `SaveDocument` | ドキュメント保存 | `CREATE (d:Document {...})` |
| `SaveChunk` | チャンク保存 | `CREATE (c:Chunk {...})` |
| `SaveEmbedding` | ベクトル保存 | `CREATE (v:Vector {...})` |
| `Search` | ベクトル検索 | `cosine_similarity()` |
| `GetEmbeddingByID` | Embedding取得 | `MATCH (v:Vector {id: $id})` |
| `GetEmbeddingsByIDs` | Embedding一括取得 | `WHERE id IN [...]` |
| `Close` | 接続クローズ | `conn.Close(); db.Close()` |

### 5.3 GraphStorage メソッド

以下のメソッドを実装する必要がある：

| メソッド | 機能 | Kuzuでの実装 |
|---------|------|-------------|
| `AddNodes` | ノード追加 | `CREATE (n:GraphNode {...})` |
| `AddEdges` | エッジ追加 | `MATCH ... CREATE ...` |
| `GetTriplets` | トリプレット取得 | グラフパターンマッチ |
| `StreamDocumentChunks` | チャンクストリーム | LIMIT/OFFSET付きクエリ |
| `GetDocumentChunkCount` | チャンク数 | `count()` |
| `GetNodesByType` | タイプ別ノード | `WHERE type = $type` |
| `GetNodesByEdge` | エッジ経由ノード | パターンマッチ |
| `UpdateEdgeWeight` | エッジ重み更新 | `SET r.properties = ...` |
| `UpdateEdgeMetrics` | エッジメトリクス更新 | `SET r.properties = ...` |
| `DeleteEdge` | エッジ削除 | `DELETE r` |
| `DeleteNode` | ノード削除 | `DELETE n` |
| `GetEdgesByNode` | ノードのエッジ | パターンマッチ |
| `GetOrphanNodes` | 孤立ノード検出 | `NOT EXISTS (...)` |
| `EnsureSchema` | スキーマ作成 | CREATE NODE/REL TABLE |
| `Close` | 接続クローズ | `conn.Close(); db.Close()` |

---

## 6. CogneeService 統合設計

### 6.1 NewCogneeService の変更

```go
func NewCogneeService(config CogneeConfig) (*CogneeService, error) {
    var vectorStorage storage.VectorStorage
    var graphStorage storage.GraphStorage
    
    switch config.DatabaseMode {
    case "kuzu":
        // Kuzuモード: 単一のKuzuStorageを両方に使用
        kuzuStorage, err := kuzu.NewKuzuStorage(config.KuzuDatabasePath)
        if err != nil {
            return nil, err
        }
        if err := kuzuStorage.EnsureSchema(ctx); err != nil {
            return nil, err
        }
        vectorStorage = kuzuStorage
        graphStorage = kuzuStorage
        
    default: // "duckdb+cozodb" or empty
        // 既存実装（変更なし）
        duckStorage, err := duckdb.NewDuckDBStorage(...)
        if err != nil {
            return nil, err
        }
        cozoStorage, err := cozodb.NewCozoStorage(...)
        if err != nil {
            return nil, err
        }
        vectorStorage = duckStorage
        graphStorage = cozoStorage
    }
    
    return &CogneeService{
        VectorStorage: vectorStorage,
        GraphStorage:  graphStorage,
        // ...
    }, nil
}
```

---

## 7. Phase-10A: Kuzuビルド基盤確立

### 7.1 目的

**go-kuzuをインポートし、ディスクモードでKuzuを起動するコードのみを `main.go` に追加する。**
`make build` と `make build-linux-amd64` がエラー・警告なしで成功することをゴールとする。

### 7.2 実装スコープ

#### 7.2.1 go.mod 更新

```bash
cd src
go get github.com/t-kawata/go-kuzu@latest
```

#### 7.2.2 main.go に追加するテストコマンド

```go
case "test-kuzu-build":
    // Phase-10A: Kuzuビルド確認テスト
    log.Println("--- Phase 10A Test: Kuzu Build Verification ---")
    
    // 1. Kuzuデータベースを開く（ディスクモード）
    kuzuDBPath := cfg.COGNEE_DB_DIR + "/kuzu_test"
    db, err := kuzu.NewDatabase(kuzuDBPath)
    if err != nil {
        log.Fatalf("❌ Failed to create Kuzu database: %v", err)
    }
    defer db.Close()
    log.Println("✅ Kuzu database created")
    
    // 2. 接続を作成
    conn, err := db.Connect()
    if err != nil {
        log.Fatalf("❌ Failed to connect to Kuzu: %v", err)
    }
    defer conn.Close()
    log.Println("✅ Kuzu connection established")
    
    // 3. 基本的なクエリ実行
    result, err := conn.Execute("RETURN 42 as answer")
    if err != nil {
        log.Fatalf("❌ Query failed: %v", err)
    }
    defer result.Close()
    
    if result.Next() {
        val, _ := result.GetValue(0)
        log.Printf("✅ Kuzu query result: %v", val)
    }
    
    // 4. クリーンアップ
    os.RemoveAll(kuzuDBPath)
    log.Println("✅ test-kuzu-build PASSED")
```

#### 7.2.3 sh/build スクリプト調整

go-kuzuのCGO要件に応じて、`sh/build` スクリプトに以下の調整が必要になる可能性がある：

1. **Kuzu C ライブラリパス**: go-kuzuがライブラリを自動ダウンロードする場合は不要
2. **Linuxクロスコンパイル**: CozoDBと同様の調整が必要になる可能性

> [!WARNING]
> go-kuzuが提供するビルド方法を確認し、CozoDBとDuckDBの既存ビルド設定との干渉がないことを確認すること。

### 7.3 成功条件

```bash
# macOS (darwin arm64)
make build
# → エラーなし、警告なしでバイナリ生成

# Linux (amd64) クロスコンパイル
make build-linux-amd64
# → エラーなし、警告なしでバイナリ生成

# テスト実行 (macOSのみ)
./dist/mycute-darwin-arm64 test-kuzu-build
# → "test-kuzu-build PASSED" が出力される
```

---

## 8. Phase-10B〜F: 後続サブフェーズ概要

### Phase-10B: KuzuStorage インターフェース設計

- `KuzuStorage` 構造体の骨格を作成
- `EnsureSchema` のみ実装
- CogneeConfig に `DatabaseMode` と `KuzuDatabasePath` を追加
- ビルド確認

### Phase-10C: スキーマ実装

- 全ノードテーブルの CREATE
- 全リレーションシップテーブルの CREATE
- インデックス作成
- `SHOW TABLES` でスキーマ確認

### Phase-10D: VectorStorage実装

- メタデータ操作 (SaveData, Exists, GetDataByID, GetDataList)
- ドキュメント/チャンク操作 (SaveDocument, SaveChunk)
- ベクトル操作 (SaveEmbedding, Search, GetEmbeddingByID, GetEmbeddingsByIDs)
- 既存テストコマンドで動作確認

### Phase-10E: GraphStorage実装

- ノード操作 (AddNodes, GetNodesByType, DeleteNode)
- エッジ操作 (AddEdges, UpdateEdgeMetrics, DeleteEdge, GetEdgesByNode)
- 高度なクエリ (GetTriplets, StreamDocumentChunks, GetOrphanNodes)
- 既存テストコマンドで動作確認

### Phase-10F: 統合・最終テスト

- NewCogneeService のモード切替統合
- 全既存テストをKuzuモードで実行
- ベンチマーク比較 (DuckDB+CozoDB vs Kuzu)
- ドキュメント更新

---

## 9. 実装成功のための重要事項 (Critical Success Factors)

> [!IMPORTANT]
> **このセクションを熟読してから実装を開始すること**
> 以下の内容を理解していないと、実装中に予期せぬ問題に遭遇する可能性が高い。

### 9.1 KuzuDB特有の制約

| 項目 | 制約内容 | 対処法 |
|------|---------|--------|
| **ディスク専用** | KuzuDBはインメモリモードをサポートしない | 必ずディスクパスを指定 |
| **PRIMARY KEY必須** | ノードテーブルには必ずPRIMARY KEYが必要 | 全テーブルで `id STRING PRIMARY KEY` を定義 |
| **接続管理** | `Connection`は`Database`から作成 | `db.Connect()`で接続取得 |
| **結果クローズ必須** | `result.Close()`を必ず呼ぶ | `defer result.Close()` パターン使用 |
| **MERGE構文** | ノードのUPSERTには`MERGE`句を使用 | `CREATE`ではなく`MERGE`を使用 |

### 9.2 go-kuzu API注意点

```go
// ❌ 間違い: パラメータ化クエリは現在サポートされていない
result, err := conn.Execute("RETURN $param", map[string]any{"param": 42})

// ✅ 正解: 文字列フォーマットでクエリを構築
result, err := conn.Execute(fmt.Sprintf("RETURN %d", 42))
```

```go
// ❌ 間違い: GetValue()の結果を直接キャスト
name := result.GetValue(0).(string)  // パニックの可能性

// ✅ 正解: エラーチェック + 型アサーション
val, err := result.GetValue(0)
if err != nil {
    return err
}
name, ok := val.(string)
if !ok {
    // デフォルト値またはエラー処理
}
```

### 9.3 エスケープ処理詳細

```go
// 完全版 escapeString 関数
// Cypher文字列リテラル内で問題を起こす全文字をエスケープ
func escapeString(s string) string {
    s = strings.ReplaceAll(s, "\\", "\\\\")  // バックスラッシュ (最初に処理)
    s = strings.ReplaceAll(s, "'", "\\'")    // シングルクォート
    s = strings.ReplaceAll(s, "\n", "\\n")   // 改行
    s = strings.ReplaceAll(s, "\r", "\\r")   // キャリッジリターン
    s = strings.ReplaceAll(s, "\t", "\\t")   // タブ
    return s
}
```

> [!CAUTION]
> **エスケープ忘れはSQLインジェクション的な脆弱性を生む**
> ユーザー入力を含む全ての文字列フィールドで `escapeString()` を使用すること。

---

## 10. 既知の制限事項 (Known Limitations)

### 10.1 KuzuDB自体の制限

| 制限 | 影響 | 回避策 |
|------|------|--------|
| パラメータ化クエリ未サポート | SQLインジェクション対策が必要 | `escapeString()`で手動エスケープ |
| バッチインサート性能 | 大量データでは個別INSERT | トランザクション使用を検討 |
| FLOAT[]の次元数上限 | 非常に高次元ベクトルで問題の可能性 | 1536次元(OpenAI)は問題なし |

### 10.2 go-kuzuバインディングの制限

| 制限 | 現状 | 対処 |
|------|------|------|
| エラー型 | 文字列エラーのみ | エラーメッセージ文字列でパターンマッチ |
| 結果型推論 | 自動型変換なし | 手動で型アサーション |
| コンテキストキャンセル | 未サポート | 長時間クエリに注意 |

---

## 11. よくある落とし穴 (Common Gotchas)

### 11.1 型変換の落とし穴

```go
// 落とし穴1: int64が期待されるがfloat64が返る場合がある
val, _ := result.GetValue(0)
count := val.(int64)  // ❌ パニックの可能性

// 安全な方法:
func getInt64(v any) int64 {
    switch val := v.(type) {
    case int64:
        return val
    case float64:  // KuzuDBはcount()でfloat64を返す場合がある
        return int64(val)
    case int32:
        return int64(val)
    default:
        return 0
    }
}
```

### 11.2 ベクトル変換の落とし穴

```go
// 落とし穴2: ベクトルのフォーマット
vec := []float32{0.1, 0.2, 0.3}

// ❌ 間違い: そのまま使用
query := fmt.Sprintf("CREATE (v:Vector {embedding: %v})", vec)
// 結果: embedding: [0.1 0.2 0.3]  ← スペース区切り、エラーになる

// ✅ 正解: カンマ区切りに変換
func formatVectorForKuzuDB(vec []float32) string {
    parts := make([]string, len(vec))
    for i, v := range vec {
        parts[i] = fmt.Sprintf("%f", v)
    }
    return "[" + strings.Join(parts, ", ") + "]"
}
// 結果: [0.100000, 0.200000, 0.300000]
```

### 11.3 MERGE句の落とし穴

```cypher
-- 落とし穴3: MERGEのキー指定
-- ❌ 間違い: 全フィールドをMERGEのパターンに含める
MERGE (d:Data {id: 'x', group_id: 'g', name: 'N', ...})

-- ✅ 正解: キーフィールドのみをパターンに、残りはON CREATE/ON MATCH SET
MERGE (d:Data {id: 'x', group_id: 'g'})
ON CREATE SET d.name = 'N', ...
ON MATCH SET d.name = 'N', ...
```

### 11.4 日時変換の落とし穴

```go
// 落とし穴4: time.Timeのフォーマット
t := time.Now()

// ❌ 間違い: Goのデフォルトフォーマット
query := fmt.Sprintf("... created_at: '%s'", t)
// 結果: 2024-01-01 12:00:00.123456 +0900 JST ← KuzuDBで解析失敗

// ✅ 正解: RFC3339形式 + datetime()関数
createdAt := t.Format(time.RFC3339)
query := fmt.Sprintf("... created_at: datetime('%s')", createdAt)
// 結果: datetime('2024-01-01T12:00:00+09:00')
```

---

## 12. 型変換リファレンス (Type Conversion Reference)

### 12.1 Go → Cypher 変換表

| Go Type | Cypher Type | 変換コード | 注意点 |
|---------|-------------|-----------|--------|
| `string` | `STRING` | `'%s'` + escapeString | シングルクォート必須 |
| `int`, `int64` | `INT64` | `%d` | そのまま |
| `int32` | `INT32` | `%d` | そのまま |
| `float32` | `FLOAT` | `%f` | 精度注意 |
| `float64` | `DOUBLE` | `%f` | 精度注意 |
| `bool` | `BOOL` | `true`/`false` | 小文字 |
| `time.Time` | `TIMESTAMP` | `datetime('%s')` | RFC3339形式 |
| `[]float32` | `FLOAT[]` | `[0.1, 0.2, ...]` | formatVector使用 |
| `map[string]any` | `STRING` | JSON文字列化 | json.Marshal使用 |
| `nil` | 該当なし | フィールド省略 | NULLを直接使わない |

### 12.2 Cypher → Go 変換表 (GetValue戻り値)

| Cypher Type | Go戻り値型 | 取得コード |
|-------------|-----------|-----------|
| `STRING` | `string` | `getString(val)` |
| `INT64` | `int64` | `getInt64(val)` |
| `INT32` | `int64` | `getInt64(val)` |
| `FLOAT` | `float64` | `getFloat64(val)` |
| `DOUBLE` | `float64` | `getFloat64(val)` |
| `BOOL` | `bool` | `val.(bool)` |
| `TIMESTAMP` | `string` | `time.Parse` で変換 |
| `FLOAT[]` | `[]any` or `string` | `parseEmbedding(val)` |

### 12.3 ヘルパー関数一覧

```go
// getString - nil安全文字列変換
func getString(v any) string {
    if v == nil {
        return ""
    }
    switch val := v.(type) {
    case string:
        return val
    case []byte:
        return string(val)
    default:
        return fmt.Sprintf("%v", v)
    }
}

// getInt64 - nil安全整数変換
func getInt64(v any) int64 {
    if v == nil {
        return 0
    }
    switch val := v.(type) {
    case int64:
        return val
    case float64:
        return int64(val)
    case int32:
        return int64(val)
    case int:
        return int64(val)
    default:
        return 0
    }
}

// getFloat64 - nil安全浮動小数点変換
func getFloat64(v any) float64 {
    if v == nil {
        return 0
    }
    switch val := v.(type) {
    case float64:
        return val
    case float32:
        return float64(val)
    case int64:
        return float64(val)
    default:
        return 0
    }
}

// parseJSONProperties - JSON文字列をmapに変換
func parseJSONProperties(s string) map[string]any {
    if s == "" {
        return make(map[string]any)
    }
    var props map[string]any
    if err := json.Unmarshal([]byte(s), &props); err != nil {
        return make(map[string]any)
    }
    return props
}

// formatVectorForKuzuDB - []float32をFLOAT[]リテラルに変換
func formatVectorForKuzuDB(vec []float32) string {
    parts := make([]string, len(vec))
    for i, v := range vec {
        parts[i] = fmt.Sprintf("%f", v)
    }
    return "[" + strings.Join(parts, ", ") + "]"
}

// parseEmbedding - KuzuDB結果からfloat32スライスに変換
func parseEmbedding(v any) []float32 {
    if v == nil {
        return nil
    }
    switch val := v.(type) {
    case []any:
        result := make([]float32, len(val))
        for i, elem := range val {
            result[i] = float32(getFloat64(elem))
        }
        return result
    case string:
        // 文字列形式の場合: "[0.1, 0.2, ...]"
        s := strings.Trim(val, "[]")
        parts := strings.Split(s, ",")
        result := make([]float32, len(parts))
        for i, p := range parts {
            f, _ := strconv.ParseFloat(strings.TrimSpace(p), 32)
            result[i] = float32(f)
        }
        return result
    default:
        return nil
    }
}
```

---

## 13. エラー処理パターン (Error Handling Patterns)

### 13.1 想定されるエラーと対処

| エラーメッセージパターン | 原因 | 対処法 |
|------------------------|------|--------|
| `table already exists` | CREATE TABLE 重複 | `IF NOT EXISTS` を使用 |
| `no table named` | テーブル未作成 | EnsureSchema() を先に実行 |
| `node not found` | MATCH結果が0件 | 存在確認後に操作 |
| `syntax error` | Cypher構文エラー | クォート/エスケープ確認 |
| `type mismatch` | 型の不一致 | 型変換コード確認 |
| `constraint violation` | PRIMARY KEY重複 | MERGE使用 or 事前チェック |

### 13.2 標準エラー処理テンプレート

```go
result, err := s.conn.Execute(query)
if err != nil {
    // 1. 詳細ログ出力
    log.Printf("[KuzuDB] Query failed: %v\nQuery: %s", err, query)
    
    // 2. エラーラップして返す
    return fmt.Errorf("failed to [操作名]: %w", err)
}
defer result.Close()  // 3. 結果のクローズを忘れない

// 4. 結果処理
if !result.Next() {
    return nil, nil  // 結果なしは nil を返す（エラーではない）
}
```

---

## 14. トラブルシューティング早見表

### 14.1 ビルドエラー

```
問題: package github.com/t-kawata/go-kuzu/pkg/kuzu is not in GOROOT
↓
解決: cd src && go get github.com/t-kawata/go-kuzu@latest && go mod tidy
```

```
問題: cgo: C compiler "gcc" not found
↓
解決: xcode-select --install (macOS) or apt install build-essential (Linux)
```

```
問題: undefined reference to 'kuzu_...'
↓
解決: go-kuzuのバージョン確認、CGO_LDFLAGS設定確認
```

### 14.2 実行時エラー

```
問題: Failed to create KuzuDB database
↓
確認: ディレクトリの書き込み権限、パスの存在確認
解決: os.MkdirAll(path, 0755) でディレクトリを事前作成
```

```
問題: Query execution failed: syntax error
↓
確認: クエリ文字列をログ出力してCypher構文を確認
解決: シングルクォートのエスケープ、キーワードの大文字確認
```

```
問題: type assertion panic
↓
確認: GetValue()の戻り値型をログ出力: fmt.Printf("%T", val)
解決: switch型アサーションパターンを使用
```

### 14.3 デバッグ用コード

```go
// クエリデバッグ: 実行前にクエリを出力
func (s *KuzuDBStorage) executeWithLog(query string) (*kuzu.QueryResult, error) {
    log.Printf("[KuzuDB DEBUG] Executing query:\n%s", query)
    result, err := s.conn.Execute(query)
    if err != nil {
        log.Printf("[KuzuDB DEBUG] Query failed: %v", err)
    }
    return result, err
}

// 型デバッグ: GetValueの戻り値型を確認
func debugValue(result *kuzu.QueryResult, col int) {
    val, err := result.GetValue(col)
    if err != nil {
        log.Printf("[DEBUG] Column %d: error=%v", col, err)
        return
    }
    log.Printf("[DEBUG] Column %d: type=%T, value=%v", col, val, val)
}
```

---

## 15. 参考資料

- **Kuzu公式ドキュメント**: https://kuzudb.com/docs/
- **go-kuzu**: https://github.com/t-kawata/go-kuzu
- **Kuzu Cypher リファレンス**: https://kuzudb.com/docs/cypher/
- **Kuzu ベクトル検索**: `docs/KUZU-IN-GO-COMPREHENSIVE-DOCUMENT.md`

---

## 16. 実装チェックリスト

### Phase-10A
- [ ] `go get github.com/t-kawata/go-kuzu@latest`
- [ ] `main.go` に `test-kuzudb-build` コマンド追加
- [ ] `import "github.com/t-kawata/go-kuzu/pkg/kuzu"` の追加
- [ ] `make build` 成功
- [ ] `make build-linux-amd64` 成功
- [ ] `./dist/mycute-darwin-arm64 test-kuzudb-build` 成功

### Phase-10B
- [ ] `src/pkg/cognee/db/kuzudb/kuzudb_storage.go` 作成
- [ ] `KuzuDBStorage` 構造体定義
- [ ] `EnsureSchema` 実装
- [ ] `CogneeConfig` に `DatabaseMode` 追加
- [ ] ビルド成功

### Phase-10C
- [ ] 全ノードテーブル作成テスト
- [ ] 全リレーションシップテーブル作成テスト
- [ ] データ挿入・読み取りテスト
- [ ] `test-kuzudb-schema` コマンド成功

### Phase-10D
- [ ] `SaveData` 実装 + テスト
- [ ] `Exists` 実装 + テスト
- [ ] `GetDataByID` / `GetDataList` 実装 + テスト
- [ ] `SaveDocument` / `SaveChunk` 実装 + テスト
- [ ] `SaveEmbedding` / `Search` 実装 + テスト
- [ ] `GetEmbeddingByID` / `GetEmbeddingsByIDs` 実装 + テスト
- [ ] `test-kuzudb-vector` コマンド成功

### Phase-10E
- [ ] `AddNodes` / `AddEdges` 実装 + テスト
- [ ] `GetTriplets` 実装 + テスト
- [ ] `StreamDocumentChunks` / `GetDocumentChunkCount` 実装 + テスト
- [ ] `GetNodesByType` / `GetNodesByEdge` 実装 + テスト
- [ ] `UpdateEdgeMetrics` / `DeleteEdge` / `DeleteNode` 実装 + テスト
- [ ] `GetEdgesByNode` / `GetOrphanNodes` 実装 + テスト
- [ ] `test-kuzudb-graph` コマンド成功

### Phase-10F
- [ ] `CogneeService` でのモード切替実装
- [ ] 環境変数設定 (`COGNEE_DATABASE_MODE`, `COGNEE_KUZUDB_PATH`)
- [ ] `test-kuzudb-integration` コマンド成功
- [ ] DuckDB+CozoDB モードでの既存テスト成功確認
- [ ] ベンチマーク比較実行

---

## 付録A: DuckDB → Kuzu マッピング

| DuckDB (SQL) | Kuzu (Cypher) |
|--------------|---------------|
| `INSERT INTO tbl VALUES (...)` | `CREATE (n:Tbl {...})` |
| `SELECT * FROM tbl WHERE id = ?` | `MATCH (n:Tbl {id: $id}) RETURN n` |
| `UPDATE tbl SET col = ? WHERE id = ?` | `MATCH (n:Tbl {id: $id}) SET n.col = $val` |
| `DELETE FROM tbl WHERE id = ?` | `MATCH (n:Tbl {id: $id}) DELETE n` |
| `array_cosine_similarity(a, b)` | `cosine_similarity(a, b)` |

## 付録B: CozoDB → Kuzu マッピング

| CozoDB (Datalog) | Kuzu (Cypher) |
|------------------|---------------|
| `:create tbl { ... }` | `CREATE NODE TABLE Tbl (...)` |
| `?[id] <- [[$id]] :put tbl {...}` | `CREATE (n:Tbl {...})` |
| `?[id] := *tbl[id, ...]` | `MATCH (n:Tbl) RETURN n.id` |
| `?[id] <- [[$id]] :rm tbl {...}` | `MATCH (n:Tbl {id: $id}) DELETE n` |
| `not *edges[id, ...]` | `NOT EXISTS ((n)-[]->())` |
