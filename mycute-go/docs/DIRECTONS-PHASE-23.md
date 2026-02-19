# 開発フェーズ23：LadybugDBにおけるACIDトランザクションの完全確保

## 目的
LadybugDB（旧 KuzuDB）において、`Absorb`（取り込み）、`Query`（質問・検索）、`Memify`（知識強化）といった主要な操作が、LLM呼び出しを含めた全フェーズにおいて完全にアトミックであることを保証します。

また、高並列なREST API環境下におけるパフォーマンスを維持しつつ、データの不整合や不要なファイルロックを回避する堅牢なアーキテクチャに刷新します。

---

## 技術背景と言及された考察
本設計は、LadybugDBの特性に基づいた以下の技術的根拠によって導き出されました。

### 1. Database と Connection の分離 (重量 vs. 軽量)
*   **lbug_database (重量)**: ファイル実体へのアクセス、カタログ情報の保持、数GB単位のバッファプール（キャッシュ）を管理します。これは非常に重いため、`CuberService` の `StorageMap` でキャッシュし続け、GCで適切に解放します。
*   **lbug_connection (軽量)**: `lbug_database` への単なる参照ハンドルです。プロセス間通信やTCP接続を伴わない「インメモ・セッション」として設計されているため、リクエスト単位で作って捨てるコストは極めて低いです。

### 2. 並行性と MVCC（マルチバージョン並行処理制御）
*   LadybugDBは「ひとつの書き込み中であっても、他の接続から読み取りが可能」というMVCC特性を持ちます。
*   コネクションを共有しすぎると、重い書き込み（Absorb）中に検索（Query）がブロックされますが、リクエストごとに独立したコネクションを提供することで、書き込みを待ち合わせつつ、読み取りは即座に並列実行できる高い応答性を実現します。

### 3. 書き込みのシリアライズ (1-Writer 原則)
*   LadybugDBは同一DBファイルに対して同時に1つの書き込みトランザクションのみを許容します。
*   複数の `Absorb` が同時に走ってエラーになることを防ぐため、`LadybugDBStorage` 単位で `sync.Mutex` を導入し、トランザクションの開始から終了までを順次実行化（シリアライズ）します。

---

## 重要な方針
1.  **接続の管理変更**: `LadybugDBStorage` が永続的に保持しているメンバ変数 `conn` を廃止します。
2.  **コンテキスト管理型接続 (Context-Aware Connections)**: `context.Context` を利用して、現在のリクエスト（トランザクション）に紐づく `Connection` を追跡します。
3.  **堅牢なトランザクションラッパー**: Mutex で競合を防ぎつつ、新規接続のオープン、BEGIN、実行、COMMIT/ROLLBACK、接続クローズまでをワンストップで行うラッパーを実装します。

---

## ステップ1：ストレージインターフェースの更新

### [MODIFY] [interfaces.go](file:///Users/kawata/shyme/mycute/src/pkg/cuber/storage/interfaces.go)

```go
type VectorStorage interface {
	// ... 既存のメソッド ...
	Transaction(ctx context.Context, fn func(txCtx context.Context) error) error
}
```

---

## ステップ2：LadybugDBStorage の構造とラッパーの実装

### [MODIFY] [ladybugdb_storage.go](file:///Users/kawata/shyme/mycute/src/pkg/cuber/db/ladybugdb/ladybugdb_storage.go)

```go
type contextKey string
const txConnKey contextKey = "txConn"

type LadybugDBStorage struct {
	db     *ladybug.Database
	// conn は廃止
	Logger *zap.Logger
	mu     sync.Mutex // トランザクションのシリアライズ用
}

func (s *LadybugDBStorage) getConn(ctx context.Context) *ladybug.Connection {
	if conn, ok := ctx.Value(txConnKey).(*ladybug.Connection); ok {
		return conn
	}
	// コネクションが渡されていない場合は、一時的な接続を作って返す（ReadOnlyクエリを想定）
	// Note: 長期的な解決案として、常に呼び出し元からTransaction経由でContextを渡す方針とする。
	return nil 
}

func (s *LadybugDBStorage) Transaction(ctx context.Context, fn func(txCtx context.Context) error) error {
	s.mu.Lock()
	defer s.mu.Unlock()

	// 1. 新しい接続のオープン（非常に軽量）
	conn, err := ladybug.OpenConnection(s.db)
	if err != nil {
		return fmt.Errorf("LadybugDB: Failed to open tx connection: %w", err)
	}
	defer conn.Close()

	// 2. BEGIN
	// ... 既存の処理と同様 ...

	// 3. 実行（txCtx に conn を埋め込む）
	txCtx := context.WithValue(ctx, txConnKey, conn)
	err = fn(txCtx)

	// ... ROLLBACK / COMMIT ...
}
```

---

## ステップ3：Cuber サービスへの統合

`Absorb`, `Query`, `Memify` を大胆にトランザクションでラップします。

### [MODIFY] [cuber.go](file:///Users/kawata/shyme/mycute/src/pkg/cuber/cuber.go)

```go
func (s *CuberService) Absorb(...) (types.TokenUsage, error) {
	st, _ := s.GetOrOpenStorage(...)
	
	var totalUsage types.TokenUsage
	err := st.Vector.Transaction(ctx, func(txCtx context.Context) error {
		// すべての内部コールに txCtx を渡し、同一トランザクションを維持
		u1, err := s.add(txCtx, ...)
		if err != nil { return err }
		totalUsage.Add(u1)

		u2, err := s.cognify(txCtx, ...)
		return err
	})
	
	return totalUsage, err
}
```

---

## 検証項目
- [ ] 並行して走る複数の `Absorb` が Mutex で安全に順次処理されること。
- [ ] `Absorb` による書き込み中であっても、`Query` による読み取りがブロックされず、かつコミット前のデータを見ないこと。
- [ ] エラーおよびパニック発生時に、DB接続が確実にクローズされ、リソースリークが発生しないこと。
