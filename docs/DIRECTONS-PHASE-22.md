# 開発フェーズ22：KuzuDBからLadybugDBへの完全移行

## 目的
リポジトリがアーカイブされた KuzuDB（go-kuzu）を、その後継プロジェクトである LadybugDB（go-ladybug）に安全かつ完全に差し替えます。これに伴い、コードベース内の「Kuzu/kuzu」という呼称をすべて「Ladybug/ladybug」に統一し、今後のメンテナンス性を確保します。

## 重要な変更点
- **依存関係の変更**: `go.mod` において `github.com/kuzudb/go-kuzu` への依存を排除し、`github.com/LadybugDB/go-ladybug` を導入します。
- **リソース管理の厳格化**: LadybugDB の Go クライアントでは、`result.Next()` が返す `FlatTuple` オブジェクトを明示的にクローズする必要があります。これを怠るとメモリリークやセグメンテーションフォールトの原因となるため、必ず `defer tuple.Close()` を記述します。
- **呼称の統一**: パッケージ名、構造体名、変数名、ログメッセージ、コメント内のすべての `kuzu` を `ladybug` に置換します。

---

## ステップ1：依存関係の更新

`src/go.mod` を修正し、新しいライブラリへの replace を設定します。

### [MODIFY] [go.mod](file:///Users/kawata/shyme/mycute/src/go.mod)

```diff
-	github.com/kuzudb/go-kuzu v0.11.3
+	github.com/LadybugDB/go-ladybug v0.0.0-latest // バージョンは `go mod tidy` で解決

-replace github.com/kuzudb/go-kuzu => github.com/t-kawata/go-kuzu v0.0.0-20251010145220-3950bb8051f9
+replace github.com/LadybugDB/go-ladybug => github.com/t-kawata/go-ladybug v0.0.0-latest
```

> [!NOTE]
> `v0.0.0-latest` は仮の表記です。実際には `go get` や `go mod tidy` を実行して、フォークリポジトリの最新コミットを指すように調整してください。

---

## ステップ2：ディレクトリおよびファイル名の変更

混乱を避けるため、ディレクトリ名およびファイル名から `kuzu` を排除します。

1. `src/pkg/cuber/db/kuzudb/` を `src/pkg/cuber/db/ladybugdb/` にリネームします。
2. その中の `kuzudb_storage.go` を `ladybugdb_storage.go` にリネームします。

---

## ステップ3：ストレージ実装の修正

`ladybugdb_storage.go`（旧 `kuzudb_storage.go`）を完全に書き換えます。主な変更点はインポートパス、パッケージ名、型名、および `tuple.Close()` の追加です。

### [MODIFY] [ladybugdb_storage.go](file:///Users/kawata/shyme/mycute/src/pkg/cuber/db/ladybugdb/ladybugdb_storage.go)

```go
package ladybugdb

import (
	"context"
	"encoding/json"
	"fmt"
	"log"
	"strings"
	"time"

	"github.com/t-kawata/mycute/pkg/cuber/storage"
	"github.com/t-kawata/mycute/pkg/cuber/types"
	"github.com/t-kawata/mycute/pkg/cuber/utils"

	ladybug "github.com/LadybugDB/go-ladybug"
	"go.uber.org/zap"
)

// LadybugDBStorage は、LadybugDBを使用した統合ストレージ実装です。
// VectorStorage と GraphStorage の両インターフェースを実装します。
type LadybugDBStorage struct {
	db     *ladybug.Database
	conn   *ladybug.Connection
	Logger *zap.Logger
}

// コンパイル時チェック: インターフェースを満たしているか確認
var _ storage.VectorStorage = (*LadybugDBStorage)(nil)
var _ storage.GraphStorage = (*LadybugDBStorage)(nil)

// NewLadybugDBStorage は新しい LadybugDBStorage インスタンスを作成します。
func NewLadybugDBStorage(dbPath string, l *zap.Logger) (*LadybugDBStorage, error) {
	var db *ladybug.Database
	var err error
	// データベースを開く
	if dbPath == ":memory:" {
		utils.LogInfo(l, "LadybugDB: Opening in-memory database")
		db, err = ladybug.OpenInMemoryDatabase(ladybug.DefaultSystemConfig())
	} else {
		db, err = ladybug.OpenDatabase(dbPath, ladybug.DefaultSystemConfig())
	}
	if err != nil {
		return nil, fmt.Errorf("Failed to open LadybugDB database: %w", err)
	}
	// 接続を開く
	conn, err := ladybug.OpenConnection(db)
	if err != nil {
		db.Close()
		return nil, fmt.Errorf("Failed to open LadybugDB connection: %w", err)
	}

	return &LadybugDBStorage{
		db:     db,
		conn:   conn,
		Logger: l,
	}, nil
}

// ... Close(), IsOpen(), EnsureSchema() なども同様に Kuzu -> Ladybug に置換 ...

// [重要] クエリ結果の取得処理（例：Exists）
func (s *LadybugDBStorage) Exists(ctx context.Context, contentHash string, memoryGroup string) bool {
	query := fmt.Sprintf(`
		MATCH (d:Data)
		WHERE d.content_hash = '%s' AND d.memory_group = '%s'
		RETURN count(d)
	`, escapeString(contentHash), escapeString(memoryGroup))
	result, err := s.conn.Query(query)
	if err != nil {
		utils.LogWarn(s.Logger, "LadybugDB: Exists query failed", zap.Error(err))
		return false
	}
	defer result.Close()
	if result.HasNext() {
		row, err := result.Next() // Ladybugでは第2戻り値にerrが返る場合がある
		if err != nil {
			return false
		}
		defer row.Close() // [重要] Tupleは必ずクローズする
		
		cntV, _ := row.GetValue(0)
		cnt := getInt64(cntV)
		return cnt > 0
	}
	return false
}

// 以下、GetDataByID, GetDataList, SaveDocument などの全てのメソッドにおいて
// result.Next() から得られる row (tuple) に対して row.Close() を追加してください。
```

---

## ステップ4：呼び出し側の修正

`src/pkg/cuber/cuber.go` におけるインポートおよび型名の参照を更新します。

### [MODIFY] [cuber.go](file:///Users/kawata/shyme/mycute/src/pkg/cuber/cuber.go)

```diff
 import (
 	...
 	"github.com/t-kawata/mycute/lib/eventbus"
-	"github.com/t-kawata/mycute/pkg/cuber/db/kuzudb"
+	"github.com/t-kawata/mycute/pkg/cuber/db/ladybugdb"
 	...
 )

 type StorageSet struct {
-	Vector     storage.VectorStorage // ベクトルストレージ（KuzuDB）
-	Graph      storage.GraphStorage  // グラフストレージ（KuzuDB）
+	Vector     storage.VectorStorage // ベクトルストレージ（LadybugDB）
+	Graph      storage.GraphStorage  // グラフストレージ（LadybugDB）
 	...
 }

 // NewCuberService は、CuberServiceの新しいインスタンスを作成します。
-//  1. KuzuDBのファイルパスを構築
-//  2. KuzuDBを初期化し、スキーマを適用
+//  1. LadybugDBのファイルパスを構築
+//  2. LadybugDBを初期化し、スキーマを適用
 ...

-	// Initialize KuzuDB
-	kuzuSt, err := kuzudb.NewKuzuDBStorage(cubeDbFilePath, s.Logger)
+	// Initialize LadybugDB
+	ladybugSt, err := ladybugdb.NewLadybugDBStorage(cubeDbFilePath, s.Logger)
-	if err != nil {
-		return nil, fmt.Errorf("Failed to open KuzuDB at %s: %w", cubeDbFilePath, err)
-	}
+	if err != nil {
+		return nil, fmt.Errorf("Failed to open LadybugDB at %s: %w", cubeDbFilePath, err)
+	}
-	// Ensure schema (lazy init)
-	if err := kuzuSt.EnsureSchema(context.Background(), embeddingModelConfig); err != nil {
-		kuzuSt.Close()
+	if err := ladybugSt.EnsureSchema(context.Background(), embeddingModelConfig); err != nil {
+		ladybugSt.Close()
 		return nil, fmt.Errorf("Failed to ensure schema: %w", err)
 	}
 	newSet := &StorageSet{
-		Vector:     kuzuSt,
-		Graph:      kuzuSt,
+		Vector:     ladybugSt,
+		Graph:      ladybugSt,
 		LastUsedAt: time.Now(),
 	}
```

---

## ステップ5：全体的な文字列置換と最終調整

コードベースのコメント、ログ、ドキュメントに残っている `kuzu` 文字列をすべて `ladybug` に置換します。

### 検索・置換対象
- `KuzuDB` -> `LadybugDB`
- `kuzu` -> `ladybug`
- `Kuzu` -> `Ladybug`

> [!TIP]
> IDEの置換機能（Case Sensitive）を使用して、`src/pkg/cuber` ディレクトリ全体に適用することをお勧めします。
> 特に以下のファイルにはコメント内に KuzuDB の記載があることが確認されています：
> - `src/pkg/cuber/storage/interfaces.go`
> - `src/pkg/cuber/storage/storage_utils.go`

---

## 期待される結果

1. `make build` および `make build-linux-amd64` が正常に終了すること。
2. 既存の KuzuDB データベースファイル（.db）が LadybugDB と互換性を持ち、正常に読み書きできること（LadybugDB は Kuzu のフォークであり、ファイルフォーマットに互換がある想定ですが、不一致がある場合は新規作成が必要です）。
3. 全てのログが `LadybugDB: ...` と出力されるようになること。

---

## 完了条件
- [ ] `go.mod` で `github.com/LadybugDB/go-ladybug` への replace が完了している
- [ ] パッケージ名およびファイル名が `ladybugdb` に変更されている
- [ ] 全ての `result.Next()` 戻り値に対して `Close()` が呼ばれている
- [ ] コード内、コメント内、ログメッセージ内の `kuzu` がすべて排除されている
- [ ] MacOS および Linux でのビルドが成功する
