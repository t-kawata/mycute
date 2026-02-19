# DuckDB と CozoDB の削除計画

## エグゼクティブサマリー

KuzuDB実装の包括的なテストにより、すべてのデータベース操作がKuzuDBのみで正しく動作することが検証されました。本計画では、DuckDBとCozoDBの依存関係を安全かつ完全に削除し、コードベースとビルドプロセスを簡素化しながら、KuzuDBを唯一のデータベースバックエンドとして完全な機能を維持する方法を詳述します。

## 背景

- **テスト結果**: JSONログの手動検証により、KuzuDBで100%の精度を確認
- **決定**: DuckDBとCozoDBはもはや不要
- **目標**: KuzuDBのみを使用するようアーキテクチャを簡素化

## 影響分析

### 完全削除対象ファイル
```
src/pkg/cognee/db/duckdb/
src/pkg/cognee/db/duckdb/duckdb_storage.go
src/pkg/cognee/db/duckdb/schema.sql
src/pkg/cognee/db/duckdb/extensions/
src/pkg/cognee/db/cozodb/
src/pkg/cognee/db/cozodb/cozo_storage.go
src/pkg/cognee/db/cozodb/lib/
```

### 修正対象ファイル

#### 1. `src/pkg/cognee/cognee.go`
**削除する行:**
- インポート文:
  - 18行目: `"mycute/pkg/cognee/db/cozodb"`
  - 19行目: `duckdbRepo "mycute/pkg/cognee/db/duckdb"`
  - 33行目: `cozo "github.com/cozodb/cozo-lib-go"`
  - 34行目: `_ "github.com/duckdb/duckdb-go/v2"`
- 埋め込みファイル:
  - 42-43行目: `//go:embed db/duckdb/extensions/v1.4.2/darwin_arm64/vss.duckdb_extension`
  - 48-49行目: Linux拡張の埋め込み
  - 54-55行目: `//go:embed db/duckdb/schema.sql`
  - これらの埋め込み用変数宣言
- 設定:
  - 63行目: `.duckdb` と `.cozodb` ファイルへのコメント参照
  - 67行目: `DatabaseMode` フィールド → **完全削除または "kuzudb" 固定に変更**
- NewCogneeService のロジック:
  - 198-200行目: DatabaseMode のデフォルトロジック → **削除**
  - 209-295行目: Switch文 → **KuzuDBのみのロジックに置き換え**
  - 245-295行目: DuckDB+CozoDB の初期化 → **ブロック全体を削除**

**変更内容:**
```go
// CogneeConfig から DatabaseMode フィールドを削除
type CogneeConfig struct {
    DBDirPath string
    DBName    string
    // DatabaseMode を削除 - KuzuDB が唯一のオプションに
    KuzuDBDatabasePath string
    // ... その他のフィールド
}

// NewCogneeService を簡素化
func NewCogneeService(config *CogneeConfig) (*CogneeService, error) {
    // DatabaseMode デフォルトロジックを削除
    // Switch文を削除
    // KuzuDB 初期化のみを保持 (210-242行目)
}
```

#### 2. `src/main.go`
**削除する行:**
- 24行目: `_ "github.com/duckdb/duckdb-go/v2"`
- 206行目: `DatabaseMode: os.Getenv("COGNEE_DATABASE_MODE")`
- 358, 428, 544行目: テスト設定の `DatabaseMode = "kuzudb"` 設定（不要になる）

**変更内容:**
```go
// DuckDB インポートを削除
// 設定初期化から DatabaseMode を削除（デフォルトで KuzuDB を使用）
// テスト設定での明示的な DatabaseMode 割り当てを削除
```

#### 3. `sh/build`
**削除する行:**
- 37-39行目: CozoDB ライブラリパスの構築
  ```bash
  PLATFORM_DIR="${OS}_${ARCH}"
  COZO_LIB_PATH="${PWD}/pkg/cognee/db/cozodb/lib/${PLATFORM_DIR}"
  ```
- 66行目: `CGO_LDFLAGS` での CozoDB ライブラリリンク
  ```bash
  # 削除: -L${COZO_LIB_PATH} -lcozo_c
  ```
- 78行目: macOS ビルドでの CozoDB ライブラリパス
  ```bash
  # 削除: -L${COZO_LIB_PATH}
  ```

**新しい CGO_LDFLAGS (Linux):**
```bash
export CGO_LDFLAGS="/tmp/lgamma_compat.o /tmp/explicit_bzero.o -lstdc++ -Wl,--no-as-needed -lm -lpthread -ldl -lc"
```

**新しい CGO_LDFLAGS (macOS):**
```bash
export CGO_LDFLAGS="-framework Security"
```

#### 4. `Makefile`
**2行目を修正:**
```makefile
# 修正前:
run:
	cd ./src && CGO_ENABLED=1 CGO_LDFLAGS="-L$$(pwd)/pkg/cognee/db/cozodb/lib/darwin_arm64 -framework Security" go run -ldflags='-extldflags "-Wl,-w"' main.go ${ARGS}

# 修正後:
run:
	cd ./src && CGO_ENABLED=1 CGO_LDFLAGS="-framework Security" go run -ldflags='-extldflags "-Wl,-w"' main.go ${ARGS}
```

#### 5. `go.mod` (存在する場合)
**削除する依存関係:**
```
github.com/duckdb/duckdb-go/v2
github.com/cozodb/cozo-lib-go
```

## 実装手順

### フェーズ 1: 準備
1. ✅ バックアップブランチを作成
2. ✅ 現在のビルドプロセスを文書化
3. ✅ ベースラインテストを実行して現在の状態を確認

### フェーズ 2: コード修正
1. **`cognee.go` を更新**:
   - インポート削除（duckdb, cozodb パッケージと C ライブラリ）
   - DuckDB 拡張用の `//go:embed` ディレクティブを削除
   - `CogneeConfig` から `DatabaseMode` フィールドを削除
   - `NewCogneeService` を KuzuDB パスのみを使用するよう簡素化
   - DuckDB/CozoDB 初期化ロジックを削除

2. **`main.go` を更新**:
   - DuckDB インポートを削除
   - `DatabaseMode` 設定への参照を削除
   - テスト設定をクリーンアップ（明示的なモード設定を削除）

3. **ビルドスクリプトを更新**:
   - `sh/build` での CozoDB lib パス構築を削除
   - CGO_LDFLAGS を簡素化（CozoDB リンクを削除）
   - `Makefile` の run ターゲットを更新

### フェーズ 3: ファイルクリーンアップ
1. `src/pkg/cognee/db/duckdb/` ディレクトリを削除
2. `src/pkg/cognee/db/cozodb/` ディレクトリを削除

### フェーズ 4: 依存関係のクリーンアップ
1. `go mod tidy` を実行して未使用の依存関係を削除
2. `go.mod` と `go.sum` がクリーンであることを確認

### フェーズ 5: ビルド検証
1. **macOS ビルドをテスト (Apple Silicon)**:
   ```bash
   make build
   ./dist/mycute-darwin-arm64 --version
   ```

2. **Linux クロスコンパイルをテスト**:
   ```bash
   make build-linux-amd64
   # バイナリサイズが妥当であることを確認
   ```

3. **包括的テストを実行**:
   ```bash
   ./dist/mycute-darwin-arm64 test-kuzudb-comprehensive-setup
   ./dist/mycute-darwin-arm64 test-kuzudb-comprehensive-run --phase baseline --runs 1 --n 5
   ```

### フェーズ 6: ドキュメント更新
1. データベースモードに言及している場合は README を更新
2. アーキテクチャドキュメントを更新
3. 「デュアルデータベース」アーキテクチャに言及しているコードコメントを更新

## 検証チェックリスト

- [ ] `make build` がエラーなく成功
- [ ] `make build-linux-amd64` がエラーなく成功
- [ ] `grep -r "duckdb\|cozodb" src/` で参照がない
- [ ] バイナリサイズが妥当（削除された依存関係による肥大化なし）
- [ ] `go mod tidy` がクリーンに実行される
- [ ] KuzuDB でテストスイートが通る
- [ ] `lib/` ディレクトリに残存ライブラリファイルがない

## ロールバック計画

問題が発生した場合:
1. バックアップブランチから復元
2. ビルドログで特定のエラーを確認
3. `rg "duckdb|cozodb" -i` で見逃した参照をチェック

## 期待される利点

1. **ビルドの簡素化**: CozoDB ネイティブライブラリの管理が不要
2. **バイナリサイズの削減**: 未使用の依存関係を削除
3. **ビルド時間の短縮**: CGO コンパイルの削減
4. **クリーンなコードベース**: 単一のデータベースバックエンド
5. **保守の容易化**: 複雑なデータベースモード切り替えがない

## リスクと緩和策

| リスク | 緩和策 |
|------|------------|
| Linux でのビルド失敗 | KuzuDB CGO 用の explicit_bzero と lgamma スタブを保持 |
| KuzuDB 機能の不足 | 包括的テストで既に検証済み |
| 設定の破損 | 環境変数 `COGNEE_DATABASE_MODE` は無効化されるが無害 |

---

**推定作業時間**: 2-3時間
**リスクレベル**: 低（包括的テストで KuzuDB の動作を検証済み）
**ロールバック所要時間**: < 15分（git restore）
