# Go言語でKuzuを使用する完全ガイド

**作成日**: 2025年12月07日  
**対象**: go-kuzu + Kuzu データベース（グラフDB＋ベクトルDB）  
**言語**: Go 1.18以上  
**ディスク永続化**: はい（インメモリモード非対応）  

---

## 目次

1. [概要](#概要)
2. [環境構築](#環境構築)
3. [基本的な使用方法](#基本的な使用方法)
4. [接続とセッション管理](#接続とセッション管理)
5. [スキーマ定義](#スキーマ定義)
6. [データ操作（CRUD）](#データ操作crud)
7. [グラフクエリ（Cypher）](#グラフクエリcypher)
8. [ベクトルデータベース機能](#ベクトルデータベース機能)
9. [トランザクション管理](#トランザクション管理)
10. [インデックス管理](#インデックス管理)
11. [パフォーマンス最適化](#パフォーマンス最適化)
12. [エラーハンドリング](#エラーハンドリング)
13. [実装パターンと実例](#実装パターンと実例)

---

## 概要

### Kuzuとは

Kuzuは以下の特徴を持つ組み込み型グラフデータベースです：

- **Property Graph データモデル**: ノードとリレーションシップに属性を持つ
- **Cypher クエリ言語**: 標準的なグラフクエリ言語
- **ベクトルDB機能**: LLM時代のベクトル埋め込み検索に対応
- **ACID トランザクション**: 信頼性の高いトランザクション処理
- **ディスク永続化**: ファイルシステムにデータを永続化
- **マルチコア対応**: 並列クエリ処理でスケーラビリティを実現

### go-kuzuについて

go-kuzuはKuzuをGo言語に埋め込むためのバインディングライブラリです：

- C API の Go ラッパー
- Go標準的なインターフェース実装
- 自動メモリ管理（CGOを通じて）
- 型安全なAPI設計

---

## 環境構築

### 1. go-kuzuのインストール

あなたのフォークされたリポジトリから直接インストール：

```bash
go get github.com/t-kawata/go-kuzu
```

### 2. go.modの確認

```bash
go get -u github.com/t-kawata/go-kuzu@latest
```

`go.mod` に以下が追加されます：

```
require github.com/t-kawata/go-kuzu v0.x.x
```

### 3. 必要な前提条件

- **Go**: 1.18以上
- **C コンパイラ**: CGOが機能する環境（gcc/clang）
- **CMake**: Kuzu C APIをビルドするため（自動で行われる）
- **ディスク容量**: データベースファイル保存用

### 4. インポート

```go
import (
    "github.com/t-kawata/go-kuzu/pkg/kuzu"
)
```

---

## 基本的な使用方法

### 最小限の例

```go
package main

import (
    "fmt"
    "log"
    
    "github.com/t-kawata/go-kuzu/pkg/kuzu"
)

func main() {
    // 1. データベース初期化（ディスク上に保存）
    db, err := kuzu.NewDatabase("./my_kuzu_db")
    if err != nil {
        log.Fatalf("Failed to create database: %v", err)
    }
    defer db.Close()

    // 2. 接続作成
    conn, err := db.Connect()
    if err != nil {
        log.Fatalf("Failed to connect: %v", err)
    }
    defer conn.Close()

    // 3. シンプルなクエリ実行
    result, err := conn.Execute("RETURN 42 as answer")
    if err != nil {
        log.Fatalf("Query failed: %v", err)
    }
    defer result.Close()

    // 4. 結果取得
    for result.Next() {
        val, _ := result.GetValue(0)
        fmt.Printf("Result: %v\n", val)
    }
}
```

---

## 接続とセッション管理

### データベースの作成と初期化

```go
// ディスク上の新規データベース作成
db, err := kuzu.NewDatabase("./my_kuzu_db")
if err != nil {
    panic(err)
}
defer db.Close()

// データベース設定
type Config struct {
    BufferPoolSize   uint64  // バッファプール（バイト単位）
    MaxDBMemory      uint64  // 最大メモリ使用量
    MaxNumThreads    uint32  // スレッド数
    EncryptionKey    string  // 暗号化キー（オプション）
}
```

### 接続の管理

```go
// 単一接続
conn, err := db.Connect()
if err != nil {
    panic(err)
}
defer conn.Close()

// 複数接続の並行実行
// Kuzuは複数接続での並行クエリをサポート
for i := 0; i < 5; i++ {
    go func(connID int) {
        conn, _ := db.Connect()
        defer conn.Close()
        
        result, _ := conn.Execute("RETURN $1 as id", connID)
        defer result.Close()
        
        for result.Next() {
            val, _ := result.GetValue(0)
            fmt.Printf("Worker %d: %v\n", connID, val)
        }
    }(i)
}
```

### コネクションプールパターン

```go
type DBConnPool struct {
    db      *kuzu.Database
    connCh  chan *kuzu.Connection
    size    int
}

func NewDBConnPool(dbPath string, poolSize int) (*DBConnPool, error) {
    db, err := kuzu.NewDatabase(dbPath)
    if err != nil {
        return nil, err
    }

    pool := &DBConnPool{
        db:     db,
        connCh: make(chan *kuzu.Connection, poolSize),
        size:   poolSize,
    }

    // プール初期化
    for i := 0; i < poolSize; i++ {
        conn, err := db.Connect()
        if err != nil {
            return nil, err
        }
        pool.connCh <- conn
    }

    return pool, nil
}

func (p *DBConnPool) GetConnection() *kuzu.Connection {
    return <-p.connCh
}

func (p *DBConnPool) ReturnConnection(conn *kuzu.Connection) {
    p.connCh <- conn
}

func (p *DBConnPool) Close() {
    close(p.connCh)
    for conn := range p.connCh {
        conn.Close()
    }
    p.db.Close()
}
```

---

## スキーマ定義

### ノードテーブルの作成

```go
conn, _ := db.Connect()
defer conn.Close()

// シンプルなノードテーブル
_, err := conn.Execute(`
    CREATE NODE TABLE Person (
        id INT64 PRIMARY KEY,
        name STRING,
        age INT32,
        email STRING
    )
`)

// ベクトルを含むノードテーブル
_, err = conn.Execute(`
    CREATE NODE TABLE Document (
        id INT64 PRIMARY KEY,
        title STRING,
        content STRING,
        embedding FLOAT_LIST,
        created_at TIMESTAMP
    )
`)

// 複数の主キー
_, err = conn.Execute(`
    CREATE NODE TABLE CompoundKey (
        person_id INT64,
        org_id INT64,
        name STRING,
        PRIMARY KEY (person_id, org_id)
    )
`)
```

### リレーションシップテーブルの作成

```go
// 基本的なリレーションシップ
_, err = conn.Execute(`
    CREATE REL TABLE knows (
        FROM Person TO Person,
        since INT32,
        strength FLOAT
    )
`)

// より複雑なリレーションシップ
_, err = conn.Execute(`
    CREATE REL TABLE worksAt (
        FROM Person TO Organization,
        start_date DATE,
        position STRING,
        salary DECIMAL(10, 2)
    )
`)

// 複数リレーションシップ
_, err = conn.Execute(`
    CREATE REL TABLE manages (
        FROM Person TO Person,
        title STRING
    )
`)
```

### 制約とデフォルト値

```go
// NOT NULL制約
_, err = conn.Execute(`
    CREATE NODE TABLE User (
        id INT64 PRIMARY KEY,
        username STRING NOT NULL,
        email STRING NOT NULL UNIQUE,
        created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
    )
`)

// テーブル削除
_, err = conn.Execute("DROP TABLE IF EXISTS OldTable")

// テーブルの情報確認
result, _ := conn.Execute("SHOW TABLES")
defer result.Close()
for result.Next() {
    tableName, _ := result.GetValue(0)
    fmt.Println("Table:", tableName)
}
```

### スキーマの確認

```go
// ノード情報確認
result, _ := conn.Execute("SHOW NODES")
defer result.Close()

// リレーション情報確認
result, _ = conn.Execute("SHOW RELS")
defer result.Close()

// 特定テーブルの構造確認
result, _ = conn.Execute("SHOW NODES LIKE 'Person'")
defer result.Close()

// 詳細なスキーマ情報
result, _ = conn.Execute("DESCRIBE Person")
defer result.Close()
```

---

## データ操作（CRUD）

### CREATE（データ挿入）

```go
// 単一行挿入
_, err := conn.Execute(`
    CREATE (p:Person {id: 1, name: 'Alice', age: 30, email: 'alice@example.com'})
`)

// 複数行挿入（バッチ挿入）
_, err = conn.Execute(`
    CREATE 
        (p1:Person {id: 1, name: 'Alice', age: 30}),
        (p2:Person {id: 2, name: 'Bob', age: 28}),
        (p3:Person {id: 3, name: 'Charlie', age: 35})
`)

// INSERT文でのテーブル挿入
_, err = conn.Execute(`
    INSERT INTO Person (id, name, age, email) 
    VALUES (4, 'Diana', 32, 'diana@example.com')
`)

// 複数値を一度に挿入
_, err = conn.Execute(`
    INSERT INTO Person (id, name, age) VALUES 
        (5, 'Eve', 26),
        (6, 'Frank', 41),
        (7, 'Grace', 29)
`)

// リレーション作成
_, err = conn.Execute(`
    MATCH (p1:Person {id: 1}), (p2:Person {id: 2})
    CREATE (p1)-[r:knows {since: 2020, strength: 0.8}]->(p2)
`)
```

### 準備済みステートメント（パラメータクエリ）

```go
// パラメータ化クエリ（SQLインジェクション対策）
stmt, err := conn.Prepare(`
    INSERT INTO Person (id, name, age, email) 
    VALUES (?, ?, ?, ?)
`)
if err != nil {
    panic(err)
}
defer stmt.Close()

// 複数回実行
data := [][]interface{}{
    {10, "User1", 25, "user1@example.com"},
    {11, "User2", 30, "user2@example.com"},
    {12, "User3", 35, "user3@example.com"},
}

for _, row := range data {
    result, _ := stmt.Execute(row...)
    result.Close()
}
```

### READ（データ取得）

```go
// 全レコード取得
result, err := conn.Execute("MATCH (p:Person) RETURN p.id, p.name, p.age")
if err != nil {
    panic(err)
}
defer result.Close()

for result.Next() {
    id, _ := result.GetValue(0)
    name, _ := result.GetValue(1)
    age, _ := result.GetValue(2)
    fmt.Printf("ID: %v, Name: %v, Age: %v\n", id, name, age)
}

// 条件付き取得
result, _ = conn.Execute("MATCH (p:Person) WHERE p.age > 30 RETURN p")
defer result.Close()

// 列数確認
numColumns := result.GetNumColumns()
columnNames := make([]string, numColumns)
for i := 0; i < int(numColumns); i++ {
    columnNames[i] = result.GetColumnName(uint64(i))
}

// 型情報取得
for i := 0; i < int(numColumns); i++ {
    dataType := result.GetColumnDataType(uint64(i))
    fmt.Printf("Column %s: %v\n", columnNames[i], dataType)
}
```

### UPDATE（データ更新）

```go
// 単一ノード更新
_, err := conn.Execute(`
    MATCH (p:Person {id: 1})
    SET p.age = 31, p.email = 'newemail@example.com'
`)

// 条件付き更新
_, err = conn.Execute(`
    MATCH (p:Person)
    WHERE p.age < 30
    SET p.age = p.age + 1
`)

// リレーション属性更新
_, err = conn.Execute(`
    MATCH (p1:Person)-[r:knows]->(p2:Person)
    WHERE p1.id = 1 AND p2.id = 2
    SET r.strength = 0.9
`)

// プロパティ削除
_, err = conn.Execute(`
    MATCH (p:Person {id: 1})
    SET p.email = NULL
`)
```

### DELETE（データ削除）

```go
// 単一ノード削除
_, err := conn.Execute(`
    MATCH (p:Person {id: 1})
    DELETE p
`)

// リレーション削除（ノード保持）
_, err = conn.Execute(`
    MATCH (p1:Person)-[r:knows]->(p2:Person)
    WHERE p1.id = 1 AND p2.id = 2
    DELETE r
`)

// ノードとリレーションを一括削除
_, err = conn.Execute(`
    MATCH (p:Person {id: 1})-[r]-()
    DELETE r, p
`)

// 条件付き削除
_, err = conn.Execute(`
    MATCH (p:Person)
    WHERE p.age > 100
    DELETE p
`)
```

---

## グラフクエリ（Cypher）

### MATCH パターン

```go
// 単純なノードマッチ
result, _ := conn.Execute("MATCH (p:Person) RETURN p.name")

// リレーションマッチ
result, _ = conn.Execute(`
    MATCH (p1:Person)-[r:knows]->(p2:Person) 
    RETURN p1.name, r.strength, p2.name
`)

// 複数ノードマッチ
result, _ = conn.Execute(`
    MATCH (p:Person)-[r1:knows]->(friend:Person)-[r2:knows]->(foaf:Person)
    WHERE p.id = 1
    RETURN friend.name, foaf.name
`)

// パターン複数
result, _ = conn.Execute(`
    MATCH 
        (p:Person)-[r1:knows]->(f:Person),
        (p)-[r2:worksAt]->(o:Organization)
    WHERE p.age > 25
    RETURN p.name, f.name, o.name
`)

// 異なるリレーション
result, _ = conn.Execute(`
    MATCH (p:Person)-[r]->(target)
    WHERE p.id = 1
    RETURN type(r) as relationship_type, target
`)
```

### WHERE句（フィルタリング）

```go
// 単純比較
result, _ := conn.Execute(`
    MATCH (p:Person)
    WHERE p.age > 25 AND p.age < 40
    RETURN p.name, p.age
`)

// 文字列マッチ
result, _ = conn.Execute(`
    MATCH (p:Person)
    WHERE p.name STARTS WITH 'A' OR p.name ENDS WITH 'e'
    RETURN p.name
`)

// IN句
result, _ = conn.Execute(`
    MATCH (p:Person)
    WHERE p.id IN [1, 3, 5, 7]
    RETURN p.name
`)

// リスト操作
result, _ = conn.Execute(`
    MATCH (doc:Document)
    WHERE size(doc.tags) > 2
    RETURN doc.title, doc.tags
`)

// ノード存在判定
result, _ = conn.Execute(`
    MATCH (p:Person)
    WHERE EXISTS((p)-[:knows]->())
    RETURN p.name
`)

// NULL判定
result, _ = conn.Execute(`
    MATCH (p:Person)
    WHERE p.email IS NOT NULL
    RETURN p.name, p.email
`)
```

### RETURN と集計

```go
// 基本的なRETURN
result, _ := conn.Execute(`
    MATCH (p:Person)
    RETURN p.id, p.name, p.age
`)

// 式を含む
result, _ = conn.Execute(`
    MATCH (p:Person)
    RETURN p.name, p.age * 2 as double_age
`)

// DISTINCT（重複排除）
result, _ = conn.Execute(`
    MATCH (p1:Person)-[:knows]->(p2:Person)
    RETURN DISTINCT p2.name
`)

// 集計関数
result, _ = conn.Execute(`
    MATCH (p:Person)
    RETURN 
        count(p) as total_people,
        avg(p.age) as average_age,
        min(p.age) as youngest,
        max(p.age) as oldest,
        sum(p.age) as total_age
`)

// GROUP BY
result, _ = conn.Execute(`
    MATCH (p:Person)-[r:worksAt]->(o:Organization)
    RETURN o.name, count(p) as employee_count, avg(p.age) as avg_age
    ORDER BY employee_count DESC
`)

// HAVING（グループフィルタ）
result, _ = conn.Execute(`
    MATCH (p:Person)-[r:worksAt]->(o:Organization)
    WITH o, count(p) as emp_count
    WHERE emp_count > 5
    RETURN o.name, emp_count
`)
```

### ORDER BY と LIMIT

```go
// ソート
result, _ := conn.Execute(`
    MATCH (p:Person)
    RETURN p.name, p.age
    ORDER BY p.age DESC, p.name ASC
`)

// ページネーション
result, _ = conn.Execute(`
    MATCH (p:Person)
    RETURN p.name
    ORDER BY p.id
    LIMIT 10
    SKIP 20
`)

// TOP N
result, _ = conn.Execute(`
    MATCH (p:Person)-[r:knows]->(f:Person)
    RETURN p.name, count(f) as friend_count
    ORDER BY friend_count DESC
    LIMIT 5
`)
```

### パス関数と走査

```go
// パス長
result, _ := conn.Execute(`
    MATCH path = (p:Person)-[*]->(target:Person)
    WHERE p.id = 1
    RETURN target.name, length(path) as hops
    ORDER BY hops
`)

// パス情報
result, _ = conn.Execute(`
    MATCH path = (p1:Person)-[r:knows*1..3]-(p2:Person)
    WHERE p1.id = 1
    RETURN 
        nodes(path) as people,
        relationships(path) as connections,
        length(path) as distance
`)

// リレーションのタイプ
result, _ = conn.Execute(`
    MATCH (p:Person)-[r]->(target)
    RETURN p.name, type(r) as rel_type, target.name
`)
```

### オプショナルマッチ

```go
// OPTIONAL MATCH（左外結合相当）
result, _ := conn.Execute(`
    MATCH (p:Person)
    OPTIONAL MATCH (p)-[r:knows]->(friend:Person)
    RETURN p.name, friend.name
    ORDER BY p.id
`)

// NULLハンドリング
result, _ = conn.Execute(`
    MATCH (p:Person)
    OPTIONAL MATCH (p)-[r:worksAt]->(o:Organization)
    RETURN p.name, coalesce(o.name, 'Unemployed') as organization
`)
```

---

## ベクトルデータベース機能

### ベクトルの保存

```go
// ベクトルカラムを持つテーブル
_, err := conn.Execute(`
    CREATE NODE TABLE Embedding (
        id INT64 PRIMARY KEY,
        text STRING,
        vector FLOAT_LIST
    )
`)

// ベクトルデータの挿入
vectorData := []float32{0.1, 0.2, 0.3, 0.4, 0.5}

_, err = conn.Execute(`
    INSERT INTO Embedding (id, text, vector) 
    VALUES (1, 'Sample text', ?)
`, vectorData)

// 複数ベクトルの一括挿入
embeddingList := [][]interface{}{
    {1, "Text 1", []float32{0.1, 0.2, 0.3}},
    {2, "Text 2", []float32{0.2, 0.3, 0.4}},
    {3, "Text 3", []float32{0.3, 0.4, 0.5}},
}

for _, row := range embeddingList {
    conn.Execute("INSERT INTO Embedding (id, text, vector) VALUES (?, ?, ?)", row...)
}
```

### ベクトル検索（ANN）

```go
// 類似度検索（Cosine類似度）
queryVector := []float32{0.15, 0.25, 0.35}

result, _ := conn.Execute(`
    MATCH (e:Embedding)
    RETURN 
        e.id,
        e.text,
        cosine_similarity(e.vector, ?) as similarity
    ORDER BY similarity DESC
    LIMIT 10
`, queryVector)

defer result.Close()
for result.Next() {
    id, _ := result.GetValue(0)
    text, _ := result.GetValue(1)
    sim, _ := result.GetValue(2)
    fmt.Printf("ID: %v, Text: %v, Similarity: %v\n", id, text, sim)
}

// L2距離（ユークリッド距離）
result, _ = conn.Execute(`
    MATCH (e:Embedding)
    RETURN 
        e.id,
        e.text,
        l2_distance(e.vector, ?) as distance
    ORDER BY distance ASC
    LIMIT 10
`, queryVector)

// 内積距離
result, _ = conn.Execute(`
    MATCH (e:Embedding)
    RETURN 
        e.id,
        e.text,
        dot_product(e.vector, ?) as product
    ORDER BY product DESC
    LIMIT 10
`, queryVector)
```

### ベクトルインデックス（HNSW）

```go
// HNSWインデックス作成（高速ANN検索用）
_, err := conn.Execute(`
    CREATE INDEX ON Embedding (vector) USING HNSW
`)

if err != nil {
    // インデックス作成に失敗した場合のハンドリング
    log.Printf("HNSW index creation failed: %v", err)
}

// インデックスの確認
result, _ := conn.Execute("SHOW INDEXES")
defer result.Close()
for result.Next() {
    indexInfo, _ := result.GetValue(0)
    fmt.Println("Index:", indexInfo)
}

// インデックスの削除
_, err = conn.Execute("DROP INDEX ON Embedding (vector)")
```

### 複合検索（グラフ + ベクトル）

```go
// 類似ドキュメントの関連ノード検索
result, _ := conn.Execute(`
    MATCH (doc1:Document)
    WHERE doc1.id = ?
    MATCH (doc2:Document)
    WHERE cosine_similarity(doc1.embedding, doc2.embedding) > 0.8
    MATCH (doc2)-[r:mentions]->(entity:Entity)
    RETURN 
        doc2.title,
        cosine_similarity(doc1.embedding, doc2.embedding) as similarity,
        entity.name
    ORDER BY similarity DESC
`, docID)

// ベクトル検索 + グラフの深さ制限
result, _ = conn.Execute(`
    MATCH (q:Query)
    WHERE q.id = ?
    MATCH (doc:Document)
    WHERE cosine_similarity(q.query_vector, doc.embedding) > 0.7
    MATCH path = (doc)-[*..3]->(related)
    RETURN 
        doc.title,
        cosine_similarity(q.query_vector, doc.embedding) as score,
        length(path) as hops,
        related.name
    ORDER BY score DESC, hops ASC
    LIMIT 50
`, queryID)
```

### Langchain/LLMとの統合パターン

```go
// LLMで埋め込みを生成し、Kuzuに保存するパターン
func StoreEmbedding(conn *kuzu.Connection, text string, embeddingVec []float32, docID int64) error {
    _, err := conn.Execute(`
        INSERT INTO Document (id, title, content, embedding)
        VALUES (?, ?, ?, ?)
    `, docID, "title", text, embeddingVec)
    return err
}

// クエリテキストから関連ドキュメントを検索
func SearchSimilarDocuments(conn *kuzu.Connection, queryVec []float32, topK int) error {
    result, err := conn.Execute(`
        MATCH (d:Document)
        RETURN 
            d.id,
            d.content,
            cosine_similarity(d.embedding, ?) as relevance
        ORDER BY relevance DESC
        LIMIT ?
    `, queryVec, topK)
    
    if err != nil {
        return err
    }
    defer result.Close()

    for result.Next() {
        id, _ := result.GetValue(0)
        content, _ := result.GetValue(1)
        score, _ := result.GetValue(2)
        fmt.Printf("Doc %v: %v (score: %v)\n", id, content, score)
    }
    return nil
}
```

---

## トランザクション管理

### 自動コミット（デフォルト）

```go
// 各実行が自動的にコミットされる（autocommit mode）
_, err := conn.Execute("CREATE (p:Person {id: 1, name: 'Alice'})")
// 自動的にコミットされた

_, err = conn.Execute("MATCH (p:Person {id: 1}) SET p.age = 30")
// 自動的にコミットされた
```

### 明示的トランザクション

```go
// トランザクション開始
_, err := conn.Execute("BEGIN TRANSACTION")
if err != nil {
    panic(err)
}

// 複数の操作
_, err = conn.Execute("CREATE (p1:Person {id: 1, name: 'Alice'})")
if err != nil {
    conn.Execute("ROLLBACK")
    panic(err)
}

_, err = conn.Execute("CREATE (p2:Person {id: 2, name: 'Bob'})")
if err != nil {
    conn.Execute("ROLLBACK")
    panic(err)
}

// リレーション作成
_, err = conn.Execute(`
    MATCH (p1:Person {id: 1}), (p2:Person {id: 2})
    CREATE (p1)-[:knows {since: 2020}]->(p2)
`)
if err != nil {
    conn.Execute("ROLLBACK")
    panic(err)
}

// 全てが成功したらコミット
_, err = conn.Execute("COMMIT")
if err != nil {
    panic(err)
}
```

### トランザクション型ラッパー

```go
// トランザクション実行用のヘルパー関数
func WithTx(conn *kuzu.Connection, fn func() error) error {
    if _, err := conn.Execute("BEGIN TRANSACTION"); err != nil {
        return err
    }

    if err := fn(); err != nil {
        conn.Execute("ROLLBACK")
        return err
    }

    _, err := conn.Execute("COMMIT")
    return err
}

// 使用方法
err := WithTx(conn, func() error {
    if _, err := conn.Execute("CREATE (p:Person {id: 1, name: 'Alice'})"); err != nil {
        return err
    }
    
    if _, err := conn.Execute("CREATE (p:Person {id: 2, name: 'Bob'})"); err != nil {
        return err
    }
    
    return nil
})

if err != nil {
    log.Printf("Transaction failed: %v", err)
}
```

### 分離レベル

```go
// デフォルト: SERIALIZABLE（Kuzuの標準）
// トランザクション分離レベルはSERIALIZABLEのみサポート

// 読み取り専用トランザクション
_, err := conn.Execute("BEGIN TRANSACTION READ ONLY")
if err != nil {
    panic(err)
}

result, _ := conn.Execute("MATCH (p:Person) RETURN count(p)")
defer result.Close()

_, err = conn.Execute("COMMIT")
```

---

## インデックス管理

### インデックスの種類と作成

```go
// プライマリキーインデックス（自動作成）
_, err := conn.Execute(`
    CREATE NODE TABLE User (
        id INT64 PRIMARY KEY,
        name STRING
    )
`)
// id に対して自動的にインデックスが作成される

// セカンダリインデックス（単一列）
_, err = conn.Execute(`
    CREATE INDEX ON Person (name)
`)

// 複合インデックス
_, err = conn.Execute(`
    CREATE INDEX ON Person (age, name)
`)

// ユニークインデックス
_, err = conn.Execute(`
    CREATE UNIQUE INDEX ON User (email)
`)

// ベクトルインデックス（HNSW）
_, err = conn.Execute(`
    CREATE INDEX ON Document (embedding) USING HNSW
`)
```

### インデックスの管理

```go
// インデックス一覧確認
result, _ := conn.Execute("SHOW INDEXES")
defer result.Close()

for result.Next() {
    indexName, _ := result.GetValue(0)
    indexInfo, _ := result.GetValue(1)
    fmt.Printf("Index: %v, Info: %v\n", indexName, indexInfo)
}

// インデックス削除
_, err := conn.Execute("DROP INDEX ON Person (name)")

// インデックス統計確認
result, _ = conn.Execute("ANALYZE Person")
defer result.Close()

// インデックスのリビルド
_, err = conn.Execute("CREATE INDEX ON Person (age)")
// 既存インデックスがあればリビルド
```

### インデックスを活用したクエリ

```go
// インデックスが活用されるクエリ（WHERE句で選択性の高い条件）
result, _ := conn.Execute(`
    MATCH (p:Person)
    WHERE p.email = ?
    RETURN p.*
`, "user@example.com")

// 複合インデックスの活用
result, _ = conn.Execute(`
    MATCH (p:Person)
    WHERE p.age > 25 AND p.age < 65 AND p.name STARTS WITH 'A'
    RETURN p.id, p.name
`)

// ベクトルインデックスの活用
result, _ = conn.Execute(`
    MATCH (d:Document)
    WHERE cosine_similarity(d.embedding, ?) > 0.85
    RETURN d.title
    LIMIT 10
`, queryVector)
```

---

## パフォーマンス最適化

### バッファプールとメモリ設定

```go
// データベース作成時の最適化
db, err := kuzu.NewDatabase("./my_kuzu_db")
if err != nil {
    panic(err)
}

// 設定の詳細
// - BufferPoolSize: 大規模グラフの場合は512MB～2GB推奨
// - MaxDBMemory: トランザクション中のメモリ上限
// - MaxNumThreads: CPU物理コア数に合わせる
```

### バルク挿入の最適化

```go
// 方法1: 複数行を一度に挿入
_, err := conn.Execute(`
    INSERT INTO Person (id, name, age) VALUES
        (1, 'User1', 25),
        (2, 'User2', 30),
        (3, 'User3', 35),
        ... (最大10000行程度が効率的)
`)

// 方法2: トランザクション内での複数INSERT
err := WithTx(conn, func() error {
    for i := 0; i < 100000; i++ {
        _, err := conn.Execute(
            "INSERT INTO Person (id, name, age) VALUES (?, ?, ?)",
            i, fmt.Sprintf("User%d", i), 20+i%50)
        if err != nil {
            return err
        }
    }
    return nil
})

// 方法3: 準備済みステートメント（推奨）
stmt, _ := conn.Prepare("INSERT INTO Person (id, name, age) VALUES (?, ?, ?)")
defer stmt.Close()

for i := 0; i < 100000; i++ {
    stmt.Execute(i, fmt.Sprintf("User%d", i), 20+i%50)
}
```

### クエリ最適化

```go
// 非効率: 大量のデータをメモリに読む
result, _ := conn.Execute(`
    MATCH (p:Person)
    RETURN p.id, p.name, p.age
`)
// 全ノードをメモリに読み込む可能性

// 効率的: LIMIT + OFFSET でページング
result, _ = conn.Execute(`
    MATCH (p:Person)
    RETURN p.id, p.name, p.age
    ORDER BY p.id
    LIMIT 1000
    OFFSET 0
`)

// 効率的: 必要なカラムのみ選択
result, _ = conn.Execute(`
    MATCH (p:Person)
    RETURN p.name  // 必要なカラムのみ
`)

// 効率的: インデックスが効いた条件フィルタ
result, _ = conn.Execute(`
    MATCH (p:Person)
    WHERE p.age > 25  // インデックスがあれば活用される
    RETURN p.name
`)

// グラフの深さを制限
result, _ = conn.Execute(`
    MATCH (p:Person)-[r:knows*..3]-(friend:Person)
    WHERE p.id = 1
    RETURN friend.name
`)
```

### 統計情報の活用

```go
// テーブル統計の更新
_, err := conn.Execute("ANALYZE Person")

// 統計情報の確認
result, _ := conn.Execute("SHOW TABLE INFO Person")
defer result.Close()

for result.Next() {
    info, _ := result.GetValue(0)
    fmt.Println(info)
}

// グラフの接続性の確認
result, _ = conn.Execute(`
    MATCH (p:Person)
    RETURN p.id, size((p)-[]-()) as degree
`)
```

### 並行処理のベストプラクティス

```go
import "sync"

func InsertConcurrently(db *kuzu.Database, dataList []interface{}, numWorkers int) error {
    var wg sync.WaitGroup
    errCh := make(chan error, numWorkers)
    dataCh := make(chan interface{}, numWorkers)

    // ワーカー起動
    for i := 0; i < numWorkers; i++ {
        wg.Add(1)
        go func() {
            defer wg.Done()
            conn, _ := db.Connect()
            defer conn.Close()

            for data := range dataCh {
                _, err := conn.Execute(
                    "INSERT INTO Person (id, name, age) VALUES (?, ?, ?)",
                    data.([]interface{})...)
                if err != nil {
                    errCh <- err
                    return
                }
            }
        }()
    }

    // データ配布
    go func() {
        for _, data := range dataList {
            dataCh <- data
        }
        close(dataCh)
    }()

    wg.Wait()
    close(errCh)

    for err := range errCh {
        if err != nil {
            return err
        }
    }
    return nil
}
```

---

## エラーハンドリング

### エラーの型と判定

```go
// 基本的なエラーハンドリング
result, err := conn.Execute("MATCH (p:Person) RETURN p")
if err != nil {
    // エラーメッセージ確認
    fmt.Printf("Query failed: %v\n", err)
    
    // エラーの詳細判定
    if err.Error() == "Binder exception" {
        fmt.Println("Schema/binding error")
    } else if err.Error() == "Runtime exception" {
        fmt.Println("Runtime execution error")
    }
}
defer result.Close()
```

### 共通エラーと対策

```go
// 1. テーブルが存在しないエラー
_, err := conn.Execute("MATCH (p:NonExistentTable) RETURN p")
// 対策: CREATE IF NOT EXISTS や事前確認

// 2. スキーマミスマッチ
_, err = conn.Execute("INSERT INTO Person (id, name) VALUES (?, ?)", 1)
// error: 必須カラムが不足 → 全カラムを指定

// 3. 型の不一致
_, err = conn.Execute("INSERT INTO Person (id, name, age) VALUES (?, ?, ?)", 
    1, "Alice", "not_a_number")
// 対策: 正しい型を使用

// 4. プライマリキーの重複
_, err = conn.Execute("INSERT INTO Person (id, name) VALUES (1, 'Alice')")
_, err = conn.Execute("INSERT INTO Person (id, name) VALUES (1, 'Bob')")
// error: PRIMARY KEY constraint → UPSERT パターンを使用

// 5. リレーション先ノードが存在しない
_, err = conn.Execute(`
    MATCH (p1:Person {id: 999}), (p2:Person {id: 1})
    CREATE (p1)-[:knows]->(p2)
`)
// 対策: ノード存在確認
```

### トランザクション内のエラー処理

```go
func SafeTransaction(conn *kuzu.Connection, operations []string) error {
    _, err := conn.Execute("BEGIN TRANSACTION")
    if err != nil {
        return fmt.Errorf("transaction start failed: %w", err)
    }

    for i, op := range operations {
        _, err := conn.Execute(op)
        if err != nil {
            conn.Execute("ROLLBACK")
            return fmt.Errorf("operation %d failed: %w", i, err)
        }
    }

    _, err = conn.Execute("COMMIT")
    if err != nil {
        conn.Execute("ROLLBACK")
        return fmt.Errorf("commit failed: %w", err)
    }

    return nil
}

// 使用例
operations := []string{
    "CREATE (p:Person {id: 1, name: 'Alice'})",
    "CREATE (p:Person {id: 2, name: 'Bob'})",
    "MATCH (p1:Person {id: 1}), (p2:Person {id: 2}) CREATE (p1)-[:knows]->(p2)",
}

if err := SafeTransaction(conn, operations); err != nil {
    log.Printf("Transaction failed: %v", err)
}
```

### リトライロジック

```go
func ExecuteWithRetry(conn *kuzu.Connection, query string, maxRetries int, args ...interface{}) (*kuzu.QueryResult, error) {
    var result *kuzu.QueryResult
    var err error

    for attempt := 0; attempt < maxRetries; attempt++ {
        result, err = conn.Execute(query, args...)
        if err == nil {
            return result, nil
        }

        // リトライ可能なエラーかチェック
        if !isRetryable(err) {
            return nil, err
        }

        // 指数バックオフ
        backoff := time.Duration(math.Pow(2, float64(attempt))) * time.Second
        time.Sleep(backoff)
    }

    return nil, fmt.Errorf("query failed after %d retries: %w", maxRetries, err)
}

func isRetryable(err error) bool {
    // ロック競合、一時的なエラーなど
    errMsg := err.Error()
    return strings.Contains(errMsg, "timeout") ||
           strings.Contains(errMsg, "lock") ||
           strings.Contains(errMsg, "temporary")
}
```

---

## 実装パターンと実例

### パターン1: グラフベースの推奨エンジン

```go
package main

import (
    "fmt"
    "log"
    "github.com/t-kawata/go-kuzu/pkg/kuzu"
)

// ユーザーベースの協調フィルタリング
type RecommendationEngine struct {
    db   *kuzu.Database
    conn *kuzu.Connection
}

func NewRecommendationEngine(dbPath string) (*RecommendationEngine, error) {
    db, err := kuzu.NewDatabase(dbPath)
    if err != nil {
        return nil, err
    }

    conn, err := db.Connect()
    if err != nil {
        return nil, err
    }

    engine := &RecommendationEngine{db: db, conn: conn}
    engine.initSchema()
    return engine, nil
}

func (e *RecommendationEngine) initSchema() {
    e.conn.Execute(`
        CREATE NODE TABLE User (
            id INT64 PRIMARY KEY,
            name STRING,
            created_at TIMESTAMP
        )
    `)

    e.conn.Execute(`
        CREATE NODE TABLE Product (
            id INT64 PRIMARY KEY,
            name STRING,
            category STRING,
            embedding FLOAT_LIST
        )
    `)

    e.conn.Execute(`
        CREATE REL TABLE purchased (
            FROM User TO Product,
            rating FLOAT,
            timestamp TIMESTAMP
        )
    `)

    e.conn.Execute(`
        CREATE REL TABLE similar (
            FROM Product TO Product,
            similarity_score FLOAT
        )
    `)
}

// ユーザーが購入した商品に基づく推奨
func (e *RecommendationEngine) GetRecommendations(userID int64, limit int) error {
    result, err := e.conn.Execute(`
        MATCH (u:User {id: ?})-[r:purchased]->(p:Product)
        MATCH (p)-[s:similar]->(recommended:Product)
        WHERE NOT EXISTS((u)-[:purchased]->(recommended))
        RETURN 
            recommended.id,
            recommended.name,
            recommended.category,
            avg(s.similarity_score) as avg_similarity
        ORDER BY avg_similarity DESC
        LIMIT ?
    `, userID, limit)

    if err != nil {
        return err
    }
    defer result.Close()

    fmt.Printf("Recommendations for User %d:\n", userID)
    for result.Next() {
        id, _ := result.GetValue(0)
        name, _ := result.GetValue(1)
        category, _ := result.GetValue(2)
        similarity, _ := result.GetValue(3)
        fmt.Printf("  - %v: %v (%v) [similarity: %v]\n", id, name, category, similarity)
    }

    return nil
}

// ベクトル距離に基づく類似商品検索
func (e *RecommendationEngine) FindSimilarProducts(productID int64, topK int) error {
    result, err := e.conn.Execute(`
        MATCH (p:Product {id: ?})
        MATCH (other:Product)
        WHERE p.id <> other.id
        RETURN 
            other.id,
            other.name,
            cosine_similarity(p.embedding, other.embedding) as similarity
        ORDER BY similarity DESC
        LIMIT ?
    `, productID, topK)

    if err != nil {
        return err
    }
    defer result.Close()

    fmt.Printf("Similar products to %d:\n", productID)
    for result.Next() {
        id, _ := result.GetValue(0)
        name, _ := result.GetValue(1)
        sim, _ := result.GetValue(2)
        fmt.Printf("  - %v: %v [similarity: %v]\n", id, name, sim)
    }

    return nil
}

func (e *RecommendationEngine) Close() {
    e.conn.Close()
    e.db.Close()
}

func main() {
    engine, err := NewRecommendationEngine("./recommendation_db")
    if err != nil {
        log.Fatal(err)
    }
    defer engine.Close()

    // ユーザーとデータ追加...
    
    engine.GetRecommendations(1, 5)
}
```

### パターン2: ナレッジグラフ + RAG システム

```go
package main

import (
    "github.com/t-kawata/go-kuzu/pkg/kuzu"
)

// RAGシステム用ナレッジグラフ
type KnowledgeGraph struct {
    db   *kuzu.Database
    conn *kuzu.Connection
}

func (kg *KnowledgeGraph) initSchema() {
    kg.conn.Execute(`
        CREATE NODE TABLE Document (
            id INT64 PRIMARY KEY,
            title STRING,
            content STRING,
            embedding FLOAT_LIST,
            metadata STRING
        )
    `)

    kg.conn.Execute(`
        CREATE NODE TABLE Entity (
            id INT64 PRIMARY KEY,
            name STRING,
            type STRING,
            embedding FLOAT_LIST
        )
    `)

    kg.conn.Execute(`
        CREATE REL TABLE mentions (
            FROM Document TO Entity,
            frequency INT32,
            context STRING
        )
    `)

    kg.conn.Execute(`
        CREATE REL TABLE related_to (
            FROM Entity TO Entity,
            relationship_type STRING,
            strength FLOAT
        )
    `)
}

// クエリ埋め込みに基づいてドキュメントを検索
func (kg *KnowledgeGraph) SemanticSearch(queryEmbedding []float32, topK int) ([]map[string]interface{}, error) {
    result, err := kg.conn.Execute(`
        MATCH (d:Document)
        RETURN 
            d.id,
            d.title,
            d.content,
            cosine_similarity(d.embedding, ?) as relevance_score
        ORDER BY relevance_score DESC
        LIMIT ?
    `, queryEmbedding, topK)

    if err != nil {
        return nil, err
    }
    defer result.Close()

    var results []map[string]interface{}
    for result.Next() {
        id, _ := result.GetValue(0)
        title, _ := result.GetValue(1)
        content, _ := result.GetValue(2)
        score, _ := result.GetValue(3)

        results = append(results, map[string]interface{}{
            "id":    id,
            "title": title,
            "content": content,
            "score": score,
        })
    }

    return results, nil
}

// エンティティの関係を含めて取得
func (kg *KnowledgeGraph) GetEntityContext(entityID int64) error {
    result, err := kg.conn.Execute(`
        MATCH (e:Entity {id: ?})
        OPTIONAL MATCH (d:Document)-[m:mentions]->(e)
        OPTIONAL MATCH (e)-[r:related_to]->(related:Entity)
        RETURN 
            e.name,
            collect(DISTINCT d.title) as mentioned_in,
            collect({entity: related.name, type: r.relationship_type}) as related_entities
    `, entityID)

    if err != nil {
        return err
    }
    defer result.Close()

    for result.Next() {
        name, _ := result.GetValue(0)
        docs, _ := result.GetValue(1)
        related, _ := result.GetValue(2)
        fmt.Printf("Entity: %v\nMentioned in: %v\nRelated: %v\n", name, docs, related)
    }

    return nil
}
```

### パターン3: マルチモーダル検索システム

```go
// テキスト + イメージ埋め込みの管理
type MultimodalSearch struct {
    db   *kuzu.Database
    conn *kuzu.Connection
}

func (ms *MultimodalSearch) initSchema() {
    ms.conn.Execute(`
        CREATE NODE TABLE Content (
            id INT64 PRIMARY KEY,
            title STRING,
            text_embedding FLOAT_LIST,
            image_embedding FLOAT_LIST,
            modality STRING,
            source STRING
        )
    `)

    ms.conn.Execute(`
        CREATE REL TABLE cross_modal_match (
            FROM Content TO Content,
            alignment_score FLOAT
        )
    `)
}

// テキストベースの検索
func (ms *MultimodalSearch) SearchByText(textEmbedding []float32, topK int) error {
    result, err := ms.conn.Execute(`
        MATCH (c:Content)
        WHERE c.text_embedding IS NOT NULL
        RETURN 
            c.id,
            c.title,
            cosine_similarity(c.text_embedding, ?) as score
        ORDER BY score DESC
        LIMIT ?
    `, textEmbedding, topK)

    if err != nil {
        return err
    }
    defer result.Close()

    for result.Next() {
        id, _ := result.GetValue(0)
        title, _ := result.GetValue(1)
        score, _ := result.GetValue(2)
        fmt.Printf("Result: %v - %v (score: %v)\n", id, title, score)
    }

    return nil
}

// イメージベースの検索
func (ms *MultimodalSearch) SearchByImage(imageEmbedding []float32, topK int) error {
    result, err := ms.conn.Execute(`
        MATCH (c:Content)
        WHERE c.image_embedding IS NOT NULL
        RETURN 
            c.id,
            c.title,
            cosine_similarity(c.image_embedding, ?) as score
        ORDER BY score DESC
        LIMIT ?
    `, imageEmbedding, topK)

    if err != nil {
        return err
    }
    defer result.Close()

    for result.Next() {
        id, _ := result.GetValue(0)
        title, _ := result.GetValue(1)
        score, _ := result.GetValue(2)
        fmt.Printf("Image match: %v - %v (score: %v)\n", id, title, score)
    }

    return nil
}

// クロスモーダル検索（テキスト→イメージ関連）
func (ms *MultimodalSearch) SearchCrossModal(textEmbedding []float32, topK int) error {
    result, err := ms.conn.Execute(`
        MATCH (text:Content)-[match:cross_modal_match]->(image:Content)
        WHERE text.text_embedding IS NOT NULL
        AND cosine_similarity(text.text_embedding, ?) > 0.7
        RETURN 
            text.title,
            image.title,
            match.alignment_score,
            cosine_similarity(text.text_embedding, ?) as text_relevance
        ORDER BY text_relevance DESC
        LIMIT ?
    `, textEmbedding, textEmbedding, topK)

    if err != nil {
        return err
    }
    defer result.Close()

    for result.Next() {
        textTitle, _ := result.GetValue(0)
        imageTitle, _ := result.GetValue(1)
        alignment, _ := result.GetValue(2)
        relevance, _ := result.GetValue(3)
        fmt.Printf("Cross-modal: %v <-> %v (alignment: %v, relevance: %v)\n",
            textTitle, imageTitle, alignment, relevance)
    }

    return nil
}
```

### パターン4: グラフベースの権限管理

```go
// 複雑な権限系統を管理
type PermissionGraph struct {
    db   *kuzu.Database
    conn *kuzu.Connection
}

func (pg *PermissionGraph) initSchema() {
    pg.conn.Execute(`
        CREATE NODE TABLE User (
            id INT64 PRIMARY KEY,
            name STRING,
            department STRING
        )
    `)

    pg.conn.Execute(`
        CREATE NODE TABLE Role (
            id INT64 PRIMARY KEY,
            name STRING,
            description STRING
        )
    `)

    pg.conn.Execute(`
        CREATE NODE TABLE Permission (
            id INT64 PRIMARY KEY,
            name STRING,
            resource_type STRING
        )
    `)

    pg.conn.Execute(`
        CREATE REL TABLE has_role (
            FROM User TO Role,
            granted_at TIMESTAMP
        )
    `)

    pg.conn.Execute(`
        CREATE REL TABLE role_has_permission (
            FROM Role TO Permission
        )
    `)

    pg.conn.Execute(`
        CREATE REL TABLE can_delegate_to (
            FROM Role TO Role,
            max_level INT32
        )
    `)
}

// ユーザーが特定のリソースにアクセス可能かチェック
func (pg *PermissionGraph) CanAccess(userID int64, permissionName string) (bool, error) {
    result, err := pg.conn.Execute(`
        MATCH (u:User {id: ?})-[:has_role]->(r:Role)-[:role_has_permission]->(p:Permission {name: ?})
        RETURN COUNT(*) > 0 as has_access
    `, userID, permissionName)

    if err != nil {
        return false, err
    }
    defer result.Close()

    if result.Next() {
        hasAccess, _ := result.GetValue(0)
        return hasAccess.(bool), nil
    }

    return false, nil
}

// 委譲可能な権限を取得
func (pg *PermissionGraph) GetDelegatableRoles(userID int64) error {
    result, err := pg.conn.Execute(`
        MATCH (u:User {id: ?})-[:has_role]->(myRole:Role)
        MATCH (myRole)-[d:can_delegate_to]->(delegatableRole:Role)
        RETURN 
            delegatableRole.id,
            delegatableRole.name,
            d.max_level
        ORDER BY delegatableRole.name
    `, userID)

    if err != nil {
        return err
    }
    defer result.Close()

    for result.Next() {
        roleID, _ := result.GetValue(0)
        roleName, _ := result.GetValue(1)
        maxLevel, _ := result.GetValue(2)
        fmt.Printf("Can delegate: %v (%v) - max level: %v\n", roleID, roleName, maxLevel)
    }

    return nil
}
```

---

## 高度なトピック

### クエリの最適化と実行計画

```go
// クエリの実行計画を確認
result, _ := conn.Execute("EXPLAIN MATCH (p:Person) RETURN p")
defer result.Close()

for result.Next() {
    plan, _ := result.GetValue(0)
    fmt.Println("Execution Plan:", plan)
}

// PROFILE でも実行計画と統計を確認
result, _ = conn.Execute("PROFILE MATCH (p:Person) RETURN count(p)")
defer result.Close()
```

### カスタム関数の使用

```go
// Kuzuが提供するビルトイン関数の活用

// 文字列関数
result, _ := conn.Execute(`
    MATCH (p:Person)
    RETURN 
        p.name,
        upper(p.name) as uppercase,
        lower(p.name) as lowercase,
        length(p.name) as name_length,
        substring(p.name, 0, 3) as first_three
`)

// リスト関数
result, _ = conn.Execute(`
    MATCH (p:Person)-[r:knows]->(friends:Person)
    RETURN 
        p.name,
        collect(friends.name) as friend_names,
        size(collect(friends)) as friend_count,
        listElement(collect(friends.name), 0) as first_friend
`)

// 数学関数
result, _ = conn.Execute(`
    MATCH (p:Person)
    RETURN 
        p.name,
        p.age,
        ceil(p.age / 10.0) * 10 as age_decade,
        abs(p.age - 30) as distance_from_30
`)

// 日時関数
result, _ = conn.Execute(`
    MATCH (r:Resource)
    RETURN 
        r.name,
        r.created_at,
        currentDate() as today,
        dateDiff('day', r.created_at, currentDate()) as days_old
`)
```

---

## トラブルシューティング

### よくある問題と解決策

```go
// 問題1: "Table not found" エラー
// → CREATE IF NOT EXISTS を使用するか、先にテーブルを確認
result, _ := conn.Execute("SHOW TABLES")

// 問題2: メモリ不足
// → バッファプール設定を確認し、大規模クエリはLIMITを使用
// → トランザクション内で過度なデータ操作を避ける

// 問題3: クエリが遅い
// → EXPLAIN でクエリプランを確認
// → インデックスの追加を検討
// → WHERE句で選択性の高い条件を先に指定

// 問題4: デッドロック
// → トランザクションの順序を統一
// → ロック時間を短くする
// → READ ONLY トランザクションを活用
```

---

## まとめ

このドキュメントをまとめると、go-kuzuを使用してGo言語でKuzuデータベースを活用する際の全体像は以下の通りです：

| カテゴリ | 用途 | キーポイント |
|---------|------|-----------|
| **セットアップ** | DB初期化・接続 | コネクションプール、複数接続対応 |
| **スキーマ** | テーブル定義 | ノード・リレーション・ベクトル対応 |
| **CRUD操作** | データ管理 | パラメータクエリ、準備済みステートメント |
| **Cypherクエリ** | グラフ検索 | パターンマッチ、集計、パス走査 |
| **ベクトル検索** | LLM統合 | HNSW、類似度計算、ANN |
| **トランザクション** | 一貫性保証 | ACID、明示的コミット、ロールバック |
| **インデックス** | パフォーマンス | 複合インデックス、HNSW |
| **最適化** | スケーリング | バッチ処理、並行実行、統計活用 |

このガイドの全ての例を組み合わせることで、グラフデータベース + ベクトルデータベースの両方の機能をGo言語で完全に実装できます。
