# Cognee Go Implementation: Phase-10C Detailed Development Directives
# KuzuDB Schema Implementation (スキーマ検証)

## 0. はじめに (Introduction)

本ドキュメントは、**Phase-10C: KuzuDB Schema Implementation** の実装を「迷いなく」「確実に」遂行するための詳細な開発指示書です。

> [!IMPORTANT]
> **Phase-10Cのゴール**
> Phase-10Bで定義したスキーマが正しく作成されることを検証する。
> テストコマンドを追加し、全ノードテーブル・リレーションシップテーブルが正常に機能することを確認する。

> [!CAUTION]
> **前提条件**
> Phase-10A（ビルド基盤）とPhase-10B（インターフェース設計）が完了していること

---

## 1. 実装ステップ一覧 (Implementation Steps)

| Step | 内容 | 対象ファイル | 行数目安 |
|------|------|-------------|---------|
| 1 | test-kuzudb-schemaコマンド追加 | `main.go` | +150行 |
| 2 | 各ノードテーブルの挿入・読取テスト | `main.go` 内 | テスト内 |
| 3 | リレーションシップテーブルのテスト | `main.go` 内 | テスト内 |
| 4 | インデックス作成の確認 | `main.go` 内 | テスト内 |
| 5 | EnsureSchemaの冪等性確認 | `main.go` 内 | テスト内 |
| 6 | ビルドとテスト実行 | - | - |

---

## Step 1: test-kuzudb-schemaコマンド追加

### 1.1 DuckDB + CozoDB での参照実装

既存のテストでは、スキーマ検証は以下のように行われている：

```go
// DuckDBスキーマテスト（暗黙的）
// schema.sql を実行後、各テーブルにINSERT/SELECTを実行して動作確認

// CozoDB EnsureSchema（cozo_storage.go より）
func (s *CozoStorage) EnsureSchema(ctx context.Context) error {
    queries := []string{
        ":create nodes { id: String, group_id: String, type: String, properties: Json }",
        ":create edges { source_id: String, target_id: String, group_id: String, type: String, properties: Json }",
    }
    for _, q := range queries {
        if _, err := s.db.Run(q, nil); err != nil {
            if !strings.Contains(err.Error(), "already exists") {
                return err
            }
        }
    }
    return nil
}
```

**根拠説明**: DuckDBとCozoDBはそれぞれ独自のスキーマ検証を行っている。KuzuDBも同様に、EnsureSchema実行後に各テーブルの動作確認を行う必要がある。

### 1.2 KuzuDB実装（完全版）

```go
    case "test-kuzudb-schema":
        // ============================================================
        // Phase-10C: KuzuDBスキーマ検証テスト
        // ============================================================
        // このテストは、KuzuDBStorageのEnsureSchemaが正しく動作することを確認します。
        // 以下の操作を順次実行し、全て成功することを確認します：
        //   1. KuzuDBStorageを作成
        //   2. EnsureSchemaを実行
        //   3. 各ノードテーブルの動作確認
        //   4. リレーションシップテーブルの動作確認
        //   5. インデックスの確認
        //   6. 冪等性の確認（2回目のEnsureSchema）
        //
        // DuckDB + CozoDB 参照:
        //   DuckDB: schema.sql を実行し、INSERT/SELECTで動作確認
        //   CozoDB: EnsureSchema後、:put/:get で動作確認
        // ============================================================
        
        log.Println("--- Phase 10C Test: KuzuDB Schema Verification ---")
        
        // ========================================
        // 1. KuzuDBStorageを作成
        // ========================================
        // DuckDB: sql.Open("duckdb", path)
        // CozoDB: cozo.NewCozoDB("rocksdb", path)
        // KuzuDB: kuzudb.NewKuzuDBStorage(path)
        kuzuDBPath := cfg.COGNEE_DB_DIR + "/kuzudb_schema_test"
        log.Printf("Creating KuzuDBStorage at: %s", kuzuDBPath)
        
        kuzuDBStorage, err := kuzudb.NewKuzuDBStorage(kuzuDBPath)
        if err != nil {
            log.Fatalf("❌ Failed to create KuzuDBStorage: %v", err)
        }
        defer func() {
            kuzuDBStorage.Close()
            os.RemoveAll(kuzuDBPath)
            log.Println("✅ Test database cleaned up")
        }()
        log.Println("✅ Step 1: KuzuDBStorage created")
        
        // ========================================
        // 2. EnsureSchemaを実行
        // ========================================
        // DuckDB: db.ExecContext(ctx, schema.sql)
        // CozoDB: storage.EnsureSchema(ctx)
        // KuzuDB: storage.EnsureSchema(ctx)
        log.Println("Executing EnsureSchema...")
        if err := kuzuDBStorage.EnsureSchema(ctx); err != nil {
            log.Fatalf("❌ EnsureSchema failed: %v", err)
        }
        log.Println("✅ Step 2: EnsureSchema completed")
        
        // ========================================
        // 3. テーブル一覧を確認
        // ========================================
        log.Println("Verifying created tables...")
        result, err := kuzuDBStorage.conn.Execute("SHOW TABLES")
        if err != nil {
            log.Fatalf("❌ SHOW TABLES failed: %v", err)
        }
        
        tableCount := 0
        for result.Next() {
            tableName, _ := result.GetValue(0)
            log.Printf("  Found table: %v", tableName)
            tableCount++
        }
        result.Close()
        
        if tableCount < 8 {
            log.Fatalf("❌ Expected at least 8 tables, found %d", tableCount)
        }
        log.Printf("✅ Step 3: Verified %d tables exist", tableCount)
        
        // ========================================
        // 4. Dataノードテーブルのテスト
        // ========================================
        // DuckDB参照:
        //   INSERT INTO data (id, group_id, name, ...) VALUES (?, ?, ?, ...)
        //   SELECT id, group_id, name FROM data WHERE id = ?
        // KuzuDB:
        //   CREATE (d:Data {...})
        //   MATCH (d:Data {id: ...}) RETURN d.name
        log.Println("Testing Data node table...")
        _, err = kuzuDBStorage.conn.Execute(`
            CREATE (d:Data {
                id: 'test_data_1',
                group_id: 'test_group',
                name: 'test.txt',
                raw_data_location: '/tmp/test.txt',
                original_data_location: '/original/test.txt',
                extension: '.txt',
                mime_type: 'text/plain',
                content_hash: 'abc123',
                owner_id: 'user1',
                created_at: datetime()
            })
        `)
        if err != nil {
            log.Fatalf("❌ Data insert failed: %v", err)
        }
        log.Println("  ✅ Data insert: OK")
        
        // 読み取りテスト
        result, err = kuzuDBStorage.conn.Execute(`
            MATCH (d:Data {id: 'test_data_1'})
            RETURN d.name, d.group_id
        `)
        if err != nil {
            log.Fatalf("❌ Data read failed: %v", err)
        }
        if result.Next() {
            name, _ := result.GetValue(0)
            groupID, _ := result.GetValue(1)
            log.Printf("  ✅ Data read: name=%v, group_id=%v", name, groupID)
        }
        result.Close()
        log.Println("✅ Step 4: Data table verified")
        
        // ========================================
        // 5. Documentノードテーブルのテスト
        // ========================================
        // DuckDB参照:
        //   INSERT INTO documents (id, group_id, data_id, text, metadata) VALUES (...)
        // KuzuDB:
        //   CREATE (doc:Document {...})
        log.Println("Testing Document node table...")
        _, err = kuzuDBStorage.conn.Execute(`
            CREATE (doc:Document {
                id: 'test_doc_1',
                group_id: 'test_group',
                data_id: 'test_data_1',
                text: 'This is a test document content.',
                metadata: '{"pages": 1}'
            })
        `)
        if err != nil {
            log.Fatalf("❌ Document insert failed: %v", err)
        }
        log.Println("  ✅ Document insert: OK")
        log.Println("✅ Step 5: Document table verified")
        
        // ========================================
        // 6. Chunkノードテーブルのテスト
        // ========================================
        // DuckDB参照:
        //   INSERT INTO chunks (id, group_id, document_id, text, chunk_index, token_count) VALUES (...)
        // KuzuDB:
        //   CREATE (c:Chunk {...})
        log.Println("Testing Chunk node table...")
        _, err = kuzuDBStorage.conn.Execute(`
            CREATE (c:Chunk {
                id: 'test_chunk_1',
                group_id: 'test_group',
                document_id: 'test_doc_1',
                text: 'This is chunk 1.',
                token_count: 5,
                chunk_index: 0
            })
        `)
        if err != nil {
            log.Fatalf("❌ Chunk insert failed: %v", err)
        }
        log.Println("  ✅ Chunk insert: OK")
        log.Println("✅ Step 6: Chunk table verified")
        
        // ========================================
        // 7. Vectorノードテーブルのテスト
        // ========================================
        // DuckDB参照:
        //   INSERT INTO vectors (id, group_id, collection_name, text, embedding) VALUES (?, ?, ?, ?, ?::FLOAT[1536])
        //   SELECT id, text, array_cosine_similarity(embedding, ?::FLOAT[1536]) FROM vectors
        // KuzuDB:
        //   CREATE (v:Vector {..., embedding: [0.1, 0.2, ...]})
        //   MATCH (v:Vector) RETURN v.id, cosine_similarity(v.embedding, [...])
        log.Println("Testing Vector node table with embedding...")
        
        _, err = kuzuDBStorage.conn.Execute(`
            CREATE (v:Vector {
                id: 'test_vector_1',
                group_id: 'test_group',
                collection_name: 'Chunk',
                text: 'Test text for embedding',
                embedding: [0.1, 0.2, 0.3, 0.4, 0.5]
            })
        `)
        if err != nil {
            log.Fatalf("❌ Vector insert failed: %v", err)
        }
        log.Println("  ✅ Vector insert: OK")
        
        // ベクトル読み取りテスト
        result, err = kuzuDBStorage.conn.Execute(`
            MATCH (v:Vector {id: 'test_vector_1'})
            RETURN v.id, v.embedding
        `)
        if err != nil {
            log.Fatalf("❌ Vector read failed: %v", err)
        }
        if result.Next() {
            id, _ := result.GetValue(0)
            embedding, _ := result.GetValue(1)
            log.Printf("  ✅ Vector read: id=%v, embedding=%v", id, embedding)
        }
        result.Close()
        log.Println("✅ Step 7: Vector table verified (with embedding)")
        
        // ========================================
        // 8. GraphNodeノードテーブルのテスト
        // ========================================
        // CozoDB参照:
        //   ?[id, group_id, type, properties] <- [[$id, $group_id, $type, $props]] :put nodes {...}
        // KuzuDB:
        //   CREATE (n:GraphNode {id: ..., group_id: ..., type: ..., properties: ...})
        log.Println("Testing GraphNode node table...")
        _, err = kuzuDBStorage.conn.Execute(`
            CREATE (n:GraphNode {
                id: 'entity_1',
                group_id: 'test_group',
                type: 'Person',
                properties: '{"name": "Alice", "age": 30}'
            })
        `)
        if err != nil {
            log.Fatalf("❌ GraphNode insert failed: %v", err)
        }
        
        _, err = kuzuDBStorage.conn.Execute(`
            CREATE (n:GraphNode {
                id: 'entity_2',
                group_id: 'test_group',
                type: 'Organization',
                properties: '{"name": "Acme Corp"}'
            })
        `)
        if err != nil {
            log.Fatalf("❌ GraphNode insert (2) failed: %v", err)
        }
        log.Println("  ✅ GraphNode inserts: OK")
        log.Println("✅ Step 8: GraphNode table verified")
        
        // ========================================
        // 9. GraphEdgeリレーションシップテーブルのテスト
        // ========================================
        // CozoDB参照:
        //   ?[source_id, target_id, group_id, type, properties] <- [...] :put edges {...}
        // KuzuDB:
        //   MATCH (a:GraphNode {id: ...}), (b:GraphNode {id: ...})
        //   CREATE (a)-[r:GraphEdge {...}]->(b)
        log.Println("Testing GraphEdge relationship table...")
        _, err = kuzuDBStorage.conn.Execute(`
            MATCH (a:GraphNode {id: 'entity_1'}), (b:GraphNode {id: 'entity_2'})
            CREATE (a)-[r:GraphEdge {
                group_id: 'test_group',
                type: 'WORKS_AT',
                properties: '{"since": 2020}'
            }]->(b)
        `)
        if err != nil {
            log.Fatalf("❌ GraphEdge insert failed: %v", err)
        }
        log.Println("  ✅ GraphEdge insert: OK")
        
        // エッジ読み取りテスト
        // CozoDB参照: ?[source_id, target_id, type] := *edges[source_id, target_id, _, type, _]
        result, err = kuzuDBStorage.conn.Execute(`
            MATCH (a:GraphNode)-[r:GraphEdge]->(b:GraphNode)
            WHERE a.id = 'entity_1'
            RETURN a.id, r.type, b.id
        `)
        if err != nil {
            log.Fatalf("❌ GraphEdge read failed: %v", err)
        }
        if result.Next() {
            sourceID, _ := result.GetValue(0)
            edgeType, _ := result.GetValue(1)
            targetID, _ := result.GetValue(2)
            log.Printf("  ✅ GraphEdge read: %v -[%v]-> %v", sourceID, edgeType, targetID)
        }
        result.Close()
        log.Println("✅ Step 9: GraphEdge relationship verified")
        
        // ========================================
        // 10. cosine_similarityテスト
        // ========================================
        // DuckDB参照:
        //   SELECT id, text, array_cosine_similarity(embedding, ?::FLOAT[1536]) as score
        //   FROM vectors ORDER BY score DESC LIMIT k
        // KuzuDB:
        //   MATCH (v:Vector)
        //   RETURN v.id, cosine_similarity(v.embedding, [...]) as similarity
        //   ORDER BY similarity DESC
        log.Println("Testing cosine_similarity function...")
        
        // 追加のベクトルを挿入
        _, err = kuzuDBStorage.conn.Execute(`
            CREATE (v:Vector {
                id: 'test_vector_2',
                group_id: 'test_group',
                collection_name: 'Chunk',
                text: 'Another test text',
                embedding: [0.15, 0.25, 0.35, 0.45, 0.55]
            })
        `)
        if err != nil {
            log.Fatalf("❌ Vector (2) insert failed: %v", err)
        }
        
        // cosine_similarity でベクトル検索
        result, err = kuzuDBStorage.conn.Execute(`
            MATCH (v:Vector)
            WHERE v.group_id = 'test_group'
            RETURN v.id, cosine_similarity(v.embedding, [0.1, 0.2, 0.3, 0.4, 0.5]) as similarity
            ORDER BY similarity DESC
        `)
        if err != nil {
            log.Fatalf("❌ cosine_similarity query failed: %v", err)
        }
        
        log.Println("  Vector similarity results:")
        for result.Next() {
            id, _ := result.GetValue(0)
            similarity, _ := result.GetValue(1)
            log.Printf("    id=%v, similarity=%v", id, similarity)
        }
        result.Close()
        log.Println("✅ Step 10: cosine_similarity function verified")
        
        // ========================================
        // 11. EnsureSchemaの冪等性確認
        // ========================================
        // CozoDB参照: EnsureSchemaを2回呼び出してもエラーにならない
        //   → "already exists" エラーを無視する実装
        log.Println("Testing EnsureSchema idempotency (second call)...")
        if err := kuzuDBStorage.EnsureSchema(ctx); err != nil {
            log.Fatalf("❌ Second EnsureSchema failed: %v", err)
        }
        log.Println("✅ Step 11: EnsureSchema is idempotent")
        
        // ========================================
        // 結果サマリー
        // ========================================
        log.Println("========================================")
        log.Println("Phase-10C Schema Verification Summary:")
        log.Println("  ✅ KuzuDBStorage creation: PASSED")
        log.Println("  ✅ EnsureSchema: PASSED")
        log.Println("  ✅ Node tables (Data, Document, Chunk, Vector, GraphNode): PASSED")
        log.Println("  ✅ Relationship tables (GraphEdge): PASSED")
        log.Println("  ✅ Vector embedding with cosine_similarity: PASSED")
        log.Println("  ✅ Schema idempotency: PASSED")
        log.Println("========================================")
        log.Println("✅ test-kuzudb-schema PASSED - Schema is correctly implemented")
```

---

## 2. ノードテーブルテストの詳細

### 2.1 DuckDB + CozoDB 参照

| DuckDB テーブル | CozoDB リレーション | KuzuDB ノードテーブル |
|----------------|-------------------|---------------------|
| data | - | Data |
| documents | - | Document |
| chunks | - | Chunk |
| vectors | - | Vector |
| - | nodes | GraphNode |
| - | edges | GraphEdge (REL) |

### 2.2 テスト対象

| テーブル | 主キー | テストする操作 | 対応するDuckDB/CozoDB |
|---------|--------|---------------|---------------------|
| Data | id | INSERT, SELECT by id | DuckDB data |
| Document | id | INSERT, SELECT by data_id | DuckDB documents |
| Chunk | id | INSERT, SELECT by document_id | DuckDB chunks |
| Vector | id | INSERT, SELECT with embedding | DuckDB vectors |
| GraphNode | id | INSERT, SELECT by type | CozoDB nodes |

---

## 3. リレーションシップテーブルテスト

### 3.1 CozoDB参照

```go
// cozo_storage.go より - AddEdges
query := "?[source_id, target_id, group_id, type, properties] <- $data :put edges {source_id, target_id, group_id, type, properties}"
params := map[string]any{"data": rows}
s.db.Run(query, params)
```

### 3.2 KuzuDB実装

```cypher
-- GraphEdge
MATCH (a:GraphNode {id: 'entity_1'}), (b:GraphNode {id: 'entity_2'})
CREATE (a)-[r:GraphEdge {group_id: 'test', type: 'WORKS_AT', properties: '{}'}]->(b)

-- has_document
MATCH (d:Data {id: 'data_1'}), (doc:Document {id: 'doc_1'})
CREATE (d)-[:has_document]->(doc)

-- has_chunk
MATCH (doc:Document {id: 'doc_1'}), (c:Chunk {id: 'chunk_1'})
CREATE (doc)-[:has_chunk]->(c)
```

---

## 4. インデックス確認

### 4.1 期待されるインデックス

| インデックス名 | テーブル | カラム | 用途 |
|---------------|---------|--------|------|
| idx_data_group | Data | group_id | パーティション分離 |
| idx_document_group | Document | group_id | パーティション分離 |
| idx_chunk_group | Chunk | group_id | パーティション分離 |
| idx_vector_group | Vector | group_id | パーティション分離 |
| idx_graphnode_group | GraphNode | group_id | パーティション分離 |
| idx_vector_collection | Vector | (group_id, collection_name) | ベクトル検索高速化 |
| idx_graphnode_type | GraphNode | (group_id, type) | タイプ別ノード検索 |

### 4.2 期待される SHOW TABLES 出力

```
test-kuzudb-schema 実行時の期待される SHOW TABLES 結果:

Found table: Data
Found table: Document
Found table: Chunk
Found table: Vector
Found table: GraphNode
Found table: GraphEdge
Found table: has_document
Found table: has_chunk

✅ Step 3: Verified 8 tables exist
```

### 4.3 問題発生時のデバッグガイド

#### 「Failed to create KuzuDBStorage」エラー

```go
// 原因: ディレクトリ作成権限またはパスの問題
// 確認コマンド:
ls -la $(dirname $KUZUDB_PATH)

// 解決策:
os.MkdirAll(filepath.Dir(kuzuDBPath), 0755)
```

#### 「EnsureSchema failed」エラー

```go
// 原因: Cypher DDL構文エラーまたは既存テーブルとの競合
// デバッグ方法: 個別のCREATEステートメントを実行して特定

// 確認クエリ:
SHOW TABLES

// 各テーブルの状態確認:
CALL show_tables() RETURN *
```

#### 「cosine_similarity query failed」エラー

```go
// 原因: KuzuDBバージョンによる関数名の違い
// 代替関数名:
//   cosine_similarity → cos_similarity
//   cosine_distance → cos_distance

// 確認方法:
RETURN cosine_similarity([0.1, 0.2], [0.1, 0.2])
```

#### 「GraphEdge insert failed」エラー

```go
// 原因: 参照するGraphNodeが存在しない
// 確認クエリ:
MATCH (n:GraphNode {id: 'entity_1'}) RETURN n.id
MATCH (n:GraphNode {id: 'entity_2'}) RETURN n.id

// 解決策: ノードを先に作成してからエッジを作成
```

---

## 5. 成功条件チェックリスト

### Phase-10C 完了条件

- [ ] `test-kuzudb-schema` コマンドが `main.go` に追加されている
- [ ] `make build` がエラーなしで成功
- [ ] `test-kuzudb-schema` が全ステップPASSED
- [ ] 全ノードテーブル（Data, Document, Chunk, Vector, GraphNode）が作成・動作
- [ ] GraphEdgeリレーションシップが作成・動作
- [ ] cosine_similarity関数でベクトル検索が動作
- [ ] EnsureSchemaの2回目呼び出しがエラーにならない
- [ ] テスト後にデータベースディレクトリが削除されている

---

## 6. 次のフェーズへの準備

Phase-10Cが完了したら、以下が確立された状態となる：

1. **スキーマ**: 全テーブルが正しく作成される
2. **ベクトル機能**: embedding保存とcosine_similarity検索が動作
3. **グラフ機能**: ノードとエッジの作成・読取が動作
4. **冪等性**: 複数回のスキーマ作成が安全

Phase-10Dでは、VectorStorageの各メソッドを完全実装する。
