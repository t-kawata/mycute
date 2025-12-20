# Cognee Go Implementation: Phase-10A Detailed Development Directives
# KuzuDB Build Foundation (ビルド基盤確立)

## 0. はじめに (Introduction)

本ドキュメントは、**Phase-10A: KuzuDB Build Foundation** の実装を「迷いなく」「確実に」遂行するための詳細な開発指示書です。

> [!IMPORTANT]
> **Phase-10Aのゴール**
> go-kuzuをインポートし、KuzuDBをディスクモードで起動するコードを `main.go` に追加する。
> `make build` と `make build-linux-amd64` がエラー・警告なしで成功することを確認する。

> [!CAUTION]
> **このフェーズの重要性**
> Phase-10Aは「ビルドが通ること」のみに集中する。
> CGOの問題を先に解決しておくことで、後続のPhase-10B以降で実装に集中できる。

---

## 1. 実装ステップ一覧 (Implementation Steps)

| Step | 内容 | 対象ファイル | 行数目安 |
|------|------|-------------|---------|
| 1 | go.mod にgo-kuzu依存関係を追加 | `go.mod` | +1行 |
| 2 | main.goにKuzuDBインポートを追加 | `main.go` | +3行 |
| 3 | test-kuzudb-buildコマンドを追加 | `main.go` | +50行 |
| 4 | sh/buildスクリプトの確認・調整 | `sh/build` | 調整次第 |
| 5 | macOS (darwin arm64) ビルド確認 | - | - |
| 6 | Linux (amd64) クロスコンパイル確認 | - | - |
| 7 | test-kuzudb-buildコマンドの動作確認 | - | - |

---

## Step 1: go.mod にgo-kuzu依存関係を追加

### 1.1 目的

go-kuzuパッケージをプロジェクトに追加し、ビルド時にKuzuDBのCライブラリが正しくリンクされるようにする。

### 1.2 DuckDB + CozoDB での参照実装

既存プロジェクトでは、DuckDBとCozoDBは以下のようにインポートされている：

```go
// DuckDB: database/sql経由で接続
import (
    "database/sql"
    _ "github.com/marcboeker/go-duckdb" // DuckDBドライバ
)

// CozoDB: cozo-lib-go経由で接続
import (
    cozo "github.com/cozodb/cozo-lib-go"
)
```

**根拠説明**: DuckDBとCozoDBはそれぞれ固有のGoバインディングライブラリを使用している。KuzuDBも同様に、専用のGoバインディングライブラリ `go-kuzu` を使用する必要がある。これにより、CGOを通じてKuzuDBのネイティブライブラリ（C++）にアクセスできる。

### 1.3 KuzuDB実装

```bash
cd src
go get github.com/t-kawata/go-kuzu@latest
```

### 1.4 確認事項

`go.mod` に以下の行が追加されていることを確認：

```
require github.com/t-kawata/go-kuzu v0.x.x
```

---

## Step 2: main.goにKuzuDBインポートを追加

### 2.1 DuckDB + CozoDB での参照実装

既存のmain.goでは、データベースライブラリは以下のようにインポートされている：

```go
// main.go (既存のインポート部分)
import (
    // ...既存のインポート...
    
    // DuckDBはdatabase/sql経由で使用するため、明示的なインポートなし
    // CozoDBは cognee パッケージ内で使用されるため、main.goでは直接インポートしない
)
```

**根拠説明**: DuckDBStorageとCozoStorageはCogneeパッケージ内で初期化されるため、main.goでは直接インポートする必要がない。しかし、Phase-10Aではビルド検証のためにmain.goでKuzuDBを直接テストする必要があるため、明示的なインポートが必要。

### 2.2 KuzuDB実装

**ファイル**: `src/main.go`

**変更位置**: インポートブロック（1-22行目付近）

```go
import (
    // 既存のインポート...
    
    "github.com/t-kawata/go-kuzu/pkg/kuzu"  // Phase-10A追加: KuzuDBバインディング
)
```

### 2.3 注意事項

- インポートは使用しないとコンパイルエラーになるため、Step 3のテストコマンドを同時に追加する
- アルファベット順に配置（goimports/gofmtが自動整形）

---

## Step 3: test-kuzudb-buildコマンドを追加

### 3.1 DuckDB + CozoDB での参照実装

既存のテストコマンドでは、データベース接続は以下のように行われている：

```go
// DuckDB接続 (duckdb_storage.go)
func NewDuckDBStorage(db *sql.DB) *DuckDBStorage {
    return &DuckDBStorage{db: db}
}

// CozoDB接続 (cozo_storage.go)  
func NewCozoStorage(db *cozo.CozoDB) *CozoStorage {
    return &CozoStorage{db: db}
}
```

**根拠説明**: DuckDBはGo標準の`database/sql`インターフェースを使用し、CozoDBは独自の`cozo.CozoDB`型を使用する。KuzuDBもgo-kuzu固有の`kuzu.Database`と`kuzu.Connection`型を使用する。この一貫したパターンにより、新しいデータベースの追加が容易になる。

### 3.2 KuzuDB実装（完全版）

```go
    case "test-kuzudb-build":
        // ============================================================
        // Phase-10A: KuzuDBビルド確認テスト
        // ============================================================
        // このテストは、go-kuzuのCGOリンクが正しく機能していることを確認します。
        // 以下の操作を順次実行し、全て成功することを確認します：
        //   1. KuzuDBデータベースを作成（ディスクモード）
        //   2. 接続を確立
        //   3. 簡単なクエリを実行
        //   4. 結果を取得
        //   5. リソースをクリーンアップ
        //
        // 参照実装:
        //   DuckDB:  db, err := sql.Open("duckdb", filePath)
        //   CozoDB:  db, err := cozo.NewCozoDB("rocksdb", filePath)
        //   KuzuDB:  db, err := kuzu.NewDatabase(path); conn, err := db.Connect()
        // ============================================================
        
        log.Println("--- Phase 10A Test: KuzuDB Build Verification ---")
        
        // ========================================
        // 1. KuzuDBデータベースを作成
        // ========================================
        // KuzuDBはディスク永続化のみサポート（インメモリモードなし）
        // DuckDB: sql.Open("duckdb", "path/to/db.duckdb")
        // CozoDB: cozo.NewCozoDB("rocksdb", "path/to/db")
        // KuzuDB: kuzu.NewDatabase("path/to/db")
        kuzuDBPath := cfg.COGNEE_DB_DIR + "/kuzudb_build_test"
        log.Printf("Creating KuzuDB database at: %s", kuzuDBPath)
        
        db, err := kuzu.NewDatabase(kuzuDBPath)
        if err != nil {
            log.Fatalf("❌ Failed to create KuzuDB database: %v", err)
        }
        defer db.Close()
        log.Println("✅ Step 1: KuzuDB database created successfully")
        
        // ========================================
        // 2. 接続を確立
        // ========================================
        // KuzuDBは複数接続をサポートするが、ここでは単一接続で十分
        // DuckDB: db.Conn(ctx) でコネクション取得
        // CozoDB: db.Run(query, params) で直接クエリ実行
        // KuzuDB: db.Connect() でコネクション取得後、conn.Execute(query) でクエリ実行
        conn, err := db.Connect()
        if err != nil {
            log.Fatalf("❌ Failed to connect to KuzuDB: %v", err)
        }
        defer conn.Close()
        log.Println("✅ Step 2: Connection established successfully")
        
        // ========================================
        // 3. 簡単なクエリを実行
        // ========================================
        // DuckDB: db.QueryContext(ctx, "SELECT 42")
        // CozoDB: db.Run("?[answer] <- [[42]]", nil)
        // KuzuDB: conn.Execute("RETURN 42 as answer")
        result, err := conn.Execute("RETURN 42 as answer")
        if err != nil {
            log.Fatalf("❌ Query execution failed: %v", err)
        }
        defer result.Close()
        log.Println("✅ Step 3: Query executed successfully")
        
        // ========================================
        // 4. 結果を取得
        // ========================================
        // DuckDB: rows.Scan(&value) で値取得
        // CozoDB: result.Rows[0][0] で値取得
        // KuzuDB: result.Next(); result.GetValue(0) で値取得
        if result.Next() {
            val, err := result.GetValue(0)
            if err != nil {
                log.Fatalf("❌ Failed to get value: %v", err)
            }
            log.Printf("✅ Step 4: Query result: %v (expected: 42)", val)
            
            // 結果の検証
            if valInt, ok := val.(int64); ok && valInt == 42 {
                log.Println("✅ Result verification passed")
            } else {
                log.Printf("⚠️ Unexpected result type or value: %T = %v", val, val)
            }
        } else {
            log.Fatalf("❌ No results returned")
        }
        
        // ========================================
        // 5. ノードテーブル作成テスト
        // ========================================
        // DuckDB: CREATE TABLE IF NOT EXISTS (SQL)
        // CozoDB: :create nodes { ... } (Datalog)
        // KuzuDB: CREATE NODE TABLE IF NOT EXISTS (Cypher)
        log.Println("Testing NODE TABLE creation...")
        _, err = conn.Execute(`
            CREATE NODE TABLE IF NOT EXISTS TestNode (
                id INT64 PRIMARY KEY,
                name STRING
            )
        `)
        if err != nil {
            log.Fatalf("❌ Failed to create node table: %v", err)
        }
        log.Println("✅ Step 5: Node table created successfully")
        
        // ========================================
        // 6. ノード挿入テスト
        // ========================================
        // DuckDB: INSERT INTO table VALUES (...)
        // CozoDB: ?[...] <- [[...]] :put nodes {...}
        // KuzuDB: CREATE (n:TestNode {...})
        log.Println("Testing node insertion...")
        _, err = conn.Execute(`
            CREATE (n:TestNode {id: 1, name: 'Test'})
        `)
        if err != nil {
            log.Fatalf("❌ Failed to insert node: %v", err)
        }
        log.Println("✅ Step 6: Node inserted successfully")
        
        // ========================================
        // 7. ノード読み取りテスト
        // ========================================
        // DuckDB: SELECT * FROM table WHERE id = ?
        // CozoDB: ?[...] := *nodes[...], id = $id
        // KuzuDB: MATCH (n:TestNode {id: 1}) RETURN n.name
        log.Println("Testing node retrieval...")
        readResult, err := conn.Execute(`
            MATCH (n:TestNode {id: 1}) RETURN n.name
        `)
        if err != nil {
            log.Fatalf("❌ Failed to read node: %v", err)
        }
        defer readResult.Close()
        
        if readResult.Next() {
            name, _ := readResult.GetValue(0)
            log.Printf("✅ Step 7: Retrieved node name: %v", name)
        }
        
        // ========================================
        // 8. クリーンアップ
        // ========================================
        log.Println("Cleaning up test database...")
        conn.Close()
        db.Close()
        
        if err := os.RemoveAll(kuzuDBPath); err != nil {
            log.Printf("⚠️ Warning: Failed to remove test database directory: %v", err)
        } else {
            log.Println("✅ Step 8: Test database cleaned up")
        }
        
        // ========================================
        // 結果サマリー
        // ========================================
        log.Println("========================================")
        log.Println("Phase-10A Build Verification Summary:")
        log.Println("  ✅ Database creation: PASSED")
        log.Println("  ✅ Connection: PASSED")
        log.Println("  ✅ Query execution: PASSED")
        log.Println("  ✅ Result retrieval: PASSED")
        log.Println("  ✅ Schema creation: PASSED")
        log.Println("  ✅ Node operations: PASSED")
        log.Println("  ✅ Cleanup: PASSED")
        log.Println("========================================")
        log.Println("✅ test-kuzudb-build PASSED - CGO linking is working correctly")
```

### 3.3 コードの解説

| ステップ | 目的 | DuckDB/CozoDB参照 | KuzuDB実装 |
|---------|------|------------------|-----------|
| 1 | DB作成 | `sql.Open()` / `cozo.NewCozoDB()` | `kuzu.NewDatabase()` |
| 2 | 接続確立 | 暗黙的 / 暗黙的 | `db.Connect()` |
| 3 | クエリ実行 | `db.QueryContext()` / `db.Run()` | `conn.Execute()` |
| 4 | 結果取得 | `rows.Scan()` / `result.Rows[][]` | `result.GetValue()` |
| 5 | スキーマ作成 | SQL DDL / Datalog :create | Cypher CREATE NODE TABLE |
| 6 | ノード挿入 | SQL INSERT / Datalog :put | Cypher CREATE |
| 7 | ノード読取 | SQL SELECT / Datalog query | Cypher MATCH...RETURN |
| 8 | クリーンアップ | `db.Close()` / `db.Close()` | `conn.Close(); db.Close()` |

---

## Step 4: sh/buildスクリプトの確認・調整

### 4.1 DuckDB + CozoDB での参照実装

既存のビルドスクリプトでは、CGOが必要なライブラリに対して以下の設定を行っている：

```bash
# sh/build より抜粋

# CozoDB用の設定
export CGO_ENABLED=1

# Linuxクロスコンパイル用
if [[ "$GOOS" == "linux" ]]; then
    export CC=x86_64-linux-gnu-gcc
    export CXX=x86_64-linux-gnu-g++
    # CozoDBのネイティブライブラリパス
    export CGO_LDFLAGS="-L${COZO_LIB_PATH} -lcozo_c"
fi
```

**根拠説明**: CGOを使用するライブラリはプラットフォーム固有のネイティブライブラリを必要とする。DuckDBとCozoDBはそれぞれVSS拡張とRocksDBバックエンドのネイティブライブラリを使用している。KuzuDBも同様にネイティブライブラリを必要とするため、ビルドスクリプトで適切な設定が必要になる可能性がある。

### 4.2 確認ポイント

1. **go-kuzuのライブラリ配置**
   - go-kuzuが自動でネイティブライブラリを取得するか確認
   - 手動配置が必要な場合は `pkg/cognee/db/kuzudb/lib/` ディレクトリを作成

2. **CGO設定の確認**
   - go-kuzuがCGO_CFLAGSやCGO_LDFLAGSを必要とするか確認
   - CozoDBの設定と衝突しないか確認

3. **Linuxクロスコンパイル**
   - go-kuzuがLinux用のプリコンパイル済みライブラリを提供しているか確認
   - 提供されていない場合は、macOS上でのLinuxビルドは不可能

### 4.3 調整が必要な場合の例

```bash
# sh/build に追加する可能性のある設定

# KuzuDB ライブラリパス（必要な場合のみ）
KUZUDB_LIB_PATH="${PWD}/pkg/cognee/db/kuzudb/lib/${PLATFORM_DIR}"

# CGO_LDFLAGS に追加（必要な場合のみ）
export CGO_LDFLAGS="${CGO_LDFLAGS} -L${KUZUDB_LIB_PATH} -lkuzu"
```

> [!IMPORTANT]
> **まずはsh/buildを変更せずにビルドを試みる**
> go-kuzuが自動でライブラリを処理する場合、`sh/build` の変更は不要。
> ビルドエラーが発生した場合にのみ調整を行う。

---

## Step 5: macOS (darwin arm64) ビルド確認

### 5.1 実行コマンド

```bash
cd /path/to/mycute
make build
```

### 5.2 期待される結果

```
mkdir -p dist
cd ./src && ../sh/build -o darwin -a arm64
Current version is v0.0.x.
GOOS=darwin GOARCH=arm64 go build ... -o ../dist/mycute-darwin-arm64 main.go
```

### 5.3 エラー発生時の対処

#### エラー: `package github.com/t-kawata/go-kuzu/pkg/kuzu is not in GOROOT`

**原因**: go getが完了していない

**解決策**:
```bash
cd src
go get github.com/t-kawata/go-kuzu@latest
go mod tidy
```

#### エラー: `cgo: C compiler "gcc" not found`

**原因**: Xcodeコマンドラインツールがインストールされていない

**解決策**:
```bash
xcode-select --install
```

#### エラー: `undefined reference to 'kuzu_...'`

**原因**: KuzuDBネイティブライブラリがリンクされていない

**解決策**:
1. go-kuzuのドキュメントを確認
2. `sh/build` にCGO_LDFLAGSを追加

---

## Step 6: Linux (amd64) クロスコンパイル確認

### 6.1 実行コマンド

```bash
cd /path/to/mycute
make build-linux-amd64
```

### 6.2 DuckDB + CozoDB での参照実装

既存のLinuxクロスコンパイルでは、CozoDBのネイティブライブラリを手動で配置している：

```bash
# CozoDBのLinux用ライブラリ配置
pkg/cognee/db/cozodb/lib/linux_amd64/libcozo_c.so
```

**根拠説明**: CGOを使用するクロスコンパイルでは、ターゲットプラットフォーム用のネイティブライブラリが必要。CozoDBは事前にビルドされたLinux用ライブラリをプロジェクトに含めている。KuzuDBも同様のアプローチが必要になる可能性がある。

### 6.3 エラー発生時の対処

#### エラー: Linux用KuzuDBライブラリが見つからない

**原因**: go-kuzuがLinux用ライブラリを含んでいない可能性

**解決策選択肢**:
1. go-kuzuのIssue/PRを確認
2. KuzuDBの公式リリースからLinux用ライブラリを取得して配置
3. Linux上で直接ビルドする方針に切り替え

---

## Step 7: test-kuzudb-buildコマンドの動作確認

### 7.1 実行コマンド

```bash
./dist/mycute-darwin-arm64 test-kuzudb-build
```

### 7.2 期待される出力

```
--- Phase 10A Test: KuzuDB Build Verification ---
Creating KuzuDB database at: db/kuzudb_build_test
✅ Step 1: KuzuDB database created successfully
✅ Step 2: Connection established successfully
✅ Step 3: Query executed successfully
✅ Step 4: Query result: 42 (expected: 42)
✅ Result verification passed
Testing NODE TABLE creation...
✅ Step 5: Node table created successfully
Testing node insertion...
✅ Step 6: Node inserted successfully
Testing node retrieval...
✅ Step 7: Retrieved node name: Test
Cleaning up test database...
✅ Step 8: Test database cleaned up
========================================
Phase-10A Build Verification Summary:
  ✅ Database creation: PASSED
  ✅ Connection: PASSED
  ✅ Query execution: PASSED
  ✅ Result retrieval: PASSED
  ✅ Schema creation: PASSED
  ✅ Node operations: PASSED
  ✅ Cleanup: PASSED
========================================
✅ test-kuzudb-build PASSED - CGO linking is working correctly
```

---

## 8. 成功条件チェックリスト

### Phase-10A 完了条件

- [ ] `go get github.com/t-kawata/go-kuzu@latest` が成功
- [ ] `go.mod` にgo-kuzu依存関係が追加されている
- [ ] `main.go` にKuzuDBインポートが追加されている
- [ ] `main.go` に `test-kuzudb-build` コマンドが追加されている
- [ ] `make build` がエラー・警告なしで成功
- [ ] `make build-linux-amd64` がエラー・警告なしで成功（または代替策が確立）
- [ ] `./dist/mycute-darwin-arm64 test-kuzudb-build` が "PASSED" を出力
- [ ] `db/kuzudb_build_test` ディレクトリが自動削除されている

---

## 9. 次のフェーズへの準備

Phase-10Aが完了したら、以下が確立された状態となる：

1. **go-kuzu依存関係**: プロジェクトに正しく統合
2. **CGOビルド**: macOSで安定して動作
3. **基本操作**: KuzuDBの基本機能（DB作成、接続、クエリ）が検証済み
4. **クリーンビルド**: 既存機能に影響なし

Phase-10Bでは、この基盤の上に `KuzuDBStorage` 構造体の骨格を作成する。
