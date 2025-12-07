# Cognee Go Implementation: Phase-10F Detailed Development Directives
# CogneeService Integration (統合・最終テスト)

## 0. はじめに (Introduction)

本ドキュメントは、**Phase-10F: CogneeService Integration** の実装を「迷いなく」「確実に」遂行するための詳細な開発指示書です。

> [!IMPORTANT]
> **Phase-10Fのゴール**
> CogneeServiceで `DatabaseMode` によるDB切替を実装し、全既存テストをKuzuDBモードでも実行できるようにする。
> Phase-10の最終検証として、全機能がDuckDB+CozoDBモードとKuzuDBモードの両方で動作することを確認する。

> [!CAUTION]
> **前提条件**
> Phase-10A〜Phase-10Eが完了していること（VectorStorageとGraphStorageがすべて実装されていること）

---

## 1. 実装ステップ一覧 (Implementation Steps)

| Step | 内容 | 対象ファイル | 行数目安 |
|------|------|-------------|---------|
| 1 | CogneeConfigにデフォルト値設定 | `cognee.go` | +10行 |
| 2 | NewCogneeServiceのモード分岐実装 | `cognee.go` | +50行 |
| 3 | KuzuDBStorageインポート追加 | `cognee.go` | +1行 |
| 4 | 環境変数サポート追加 | `settings.go` | +10行 |
| 5 | test-kuzudb-integrationコマンド追加 | `main.go` | +200行 |
| 6 | 既存テストのKuzuDBモード対応 | `main.go` | 調整 |
| 7 | ビルドと全テスト実行 | - | - |
| 8 | パフォーマンス比較ベンチマーク | `main.go` | +100行 |

---

## Step 1: CogneeConfigにデフォルト値設定

### 1.1 DuckDB + CozoDB での参照実装

現在の cognee.go で CogneeConfig は以下のように定義されている：

```go
// cognee.go より
type CogneeConfig struct {
    COGNEE_DB_DIR              string  // データベース格納ディレクトリ
    DATAPIPE_URL               string  // DatapipeサービスURL
    OPENAI_API_KEY             string  // OpenAI APIキー
    S3UseLocal                 bool    // S3の代わりにローカルストレージを使用
    S3LocalPath                string  // ローカルストレージのパス
    // ... 他のフィールド
}
```

**根拠説明**: 現在のCogneeConfigはDuckDB+CozoDBモード固定になっている。Phase-10FではDatabaseModeフィールドを追加し、KuzuDBとDuckDB+CozoDBのどちらを使用するかを選択可能にする必要がある。

### 1.2 KuzuDB拡張実装

```go
type CogneeConfig struct {
    // 既存フィールド...
    COGNEE_DB_DIR              string
    DATAPIPE_URL               string
    OPENAI_API_KEY             string
    S3UseLocal                 bool
    S3LocalPath                string
    S3Endpoint                 string
    S3AccessKey                string
    S3SecretKey                string
    S3Region                   string
    S3Bucket                   string
    GraphMetabolismAlpha           float64
    GraphMetabolismDelta           float64
    GraphMetabolismPruneThreshold  float64
    GraphPruningGracePeriodMinutes int

    // ========================================
    // Phase-10追加: データベースモード設定
    // ========================================

    // DatabaseMode はデータベースの動作モードを指定します。
    //
    // 使用可能な値:
    //   - "duckdb+cozodb" (デフォルト): DuckDBとCozoDBを併用
    //     VectorStorage: DuckDBStorage
    //     GraphStorage:  CozoStorage
    //
    //   - "kuzudb": KuzuDBのみを使用（VectorStorage + GraphStorage統合）
    //     VectorStorage: KuzuDBStorage
    //     GraphStorage:  KuzuDBStorage
    //
    // 参照:
    //   DuckDBStorage: src/pkg/cognee/db/duckdb/duckdb_storage.go
    //   CozoStorage:   src/pkg/cognee/db/cozodb/cozo_storage.go
    //   KuzuDBStorage: src/pkg/cognee/db/kuzudb/kuzudb_storage.go
    DatabaseMode string

    // KuzuDBDatabasePath はKuzuDBデータベースのパスを指定します。
    // DatabaseMode が "kuzudb" の場合に使用されます。
    // 空の場合は COGNEE_DB_DIR + "/kuzudb" がデフォルト値として使用されます。
    KuzuDBDatabasePath string
}
```

---

## Step 2: NewCogneeServiceのモード分岐実装

### 2.1 DuckDB + CozoDB での参照実装

現在の NewCogneeService は以下のように実装されている：

```go
// cognee.go より（現在の実装）
func NewCogneeService(config CogneeConfig) (*CogneeService, error) {
    // DuckDB初期化
    duckDBFilePath := filepath.Join(config.COGNEE_DB_DIR, "duckdb", "cognee.duckdb")
    duckDB, err := sql.Open("duckdb", duckDBFilePath)
    if err != nil {
        return nil, fmt.Errorf("failed to open DuckDB: %w", err)
    }
    
    // VSS拡張ロード
    if err := loadDuckDBExtension(duckDB); err != nil {
        return nil, err
    }
    
    // スキーマ作成
    if _, err := duckDB.ExecContext(ctx, duckDBSchema); err != nil {
        return nil, err
    }
    
    duckDBStorage := duckdb.NewDuckDBStorage(duckDB)
    
    // CozoDB初期化
    cozoDBFilePath := filepath.Join(config.COGNEE_DB_DIR, "cozodb", "cognee.cozo")
    cozoDB, err := cozo.NewCozoDB("rocksdb", cozoDBFilePath)
    if err != nil {
        return nil, err
    }
    
    cozoStorage := cozodb.NewCozoStorage(cozoDB)
    if err := cozoStorage.EnsureSchema(ctx); err != nil {
        return nil, err
    }

    return &CogneeService{
        VectorStorage: duckDBStorage,
        GraphStorage:  cozoStorage,
        // ...
    }, nil
}
```

**根拠説明**: 現在のコードはDuckDBとCozoDBを直接初期化している。Phase-10Fでは DatabaseMode に応じて、KuzuDBStorage または DuckDBStorage+CozoStorage を選択的に初期化する分岐を追加する。

### 2.2 KuzuDB対応実装

```go
func NewCogneeService(config CogneeConfig) (*CogneeService, error) {
    ctx := context.Background()

    // ========================================
    // デフォルト値設定
    // ========================================
    if config.DatabaseMode == "" {
        config.DatabaseMode = "duckdb+cozodb"
    }
    if config.KuzuDBDatabasePath == "" && config.DatabaseMode == "kuzudb" {
        config.KuzuDBDatabasePath = filepath.Join(config.COGNEE_DB_DIR, "kuzudb")
    }

    var vectorStorage storage.VectorStorage
    var graphStorage storage.GraphStorage

    // ========================================
    // データベースモードによる分岐
    // ========================================
    switch config.DatabaseMode {
    case "kuzudb":
        // ========================================
        // KuzuDBモード: 単一のKuzuDBStorageを両方に使用
        // ========================================
        // 参照: Phase-10A〜10Eで実装したKuzuDBStorage
        // 
        // CogneeServiceのインターフェース:
        //   VectorStorage: storage.VectorStorage (DuckDBStorageが実装)
        //   GraphStorage:  storage.GraphStorage  (CozoStorageが実装)
        //
        // KuzuDBモードでは:
        //   VectorStorage: KuzuDBStorage (VectorStorageインターフェースを実装)
        //   GraphStorage:  KuzuDBStorage (GraphStorageインターフェースも実装)
        //   → 同一インスタンスが両方のインターフェースを満たす
        fmt.Printf("CogneeService: Initializing in KUZUDB mode (path: %s)\n", config.KuzuDBDatabasePath)

        kuzuDBStorage, err := kuzudb.NewKuzuDBStorage(config.KuzuDBDatabasePath)
        if err != nil {
            return nil, fmt.Errorf("failed to create KuzuDBStorage: %w", err)
        }

        // スキーマを作成
        if err := kuzuDBStorage.EnsureSchema(ctx); err != nil {
            kuzuDBStorage.Close()
            return nil, fmt.Errorf("failed to ensure KuzuDB schema: %w", err)
        }

        // 両インターフェースに同じインスタンスを設定
        // これはKuzuDBStorageが両方のインターフェースを実装しているため可能
        vectorStorage = kuzuDBStorage
        graphStorage = kuzuDBStorage

        fmt.Println("CogneeService: KuzuDB mode initialized successfully")

    default: // "duckdb+cozodb" または空
        // ========================================
        // DuckDB + CozoDB モード（既存実装）
        // ========================================
        fmt.Printf("CogneeService: Initializing in DUCKDB+COZODB mode\n")

        // DuckDB 初期化
        duckDBFilePath := filepath.Join(config.COGNEE_DB_DIR, "duckdb", "cognee.duckdb")
        if err := os.MkdirAll(filepath.Dir(duckDBFilePath), 0755); err != nil {
            return nil, fmt.Errorf("failed to create DuckDB directory: %w", err)
        }

        duckDB, err := sql.Open("duckdb", duckDBFilePath)
        if err != nil {
            return nil, fmt.Errorf("failed to open DuckDB: %w", err)
        }

        // VSS拡張ロード
        if err := loadDuckDBExtension(duckDB); err != nil {
            duckDB.Close()
            return nil, fmt.Errorf("failed to load DuckDB VSS extension: %w", err)
        }

        // スキーマ作成
        if _, err := duckDB.ExecContext(ctx, duckDBSchema); err != nil {
            duckDB.Close()
            return nil, fmt.Errorf("failed to create DuckDB schema: %w", err)
        }

        duckDBStorage := duckdb.NewDuckDBStorage(duckDB)

        // CozoDB 初期化
        cozoDBFilePath := filepath.Join(config.COGNEE_DB_DIR, "cozodb", "cognee.cozo")
        if err := os.MkdirAll(filepath.Dir(cozoDBFilePath), 0755); err != nil {
            duckDBStorage.Close()
            return nil, fmt.Errorf("failed to create CozoDB directory: %w", err)
        }

        cozoDB, err := cozo.NewCozoDB("rocksdb", cozoDBFilePath)
        if err != nil {
            duckDBStorage.Close()
            return nil, fmt.Errorf("failed to open CozoDB: %w", err)
        }

        cozoStorage := cozodb.NewCozoStorage(cozoDB)
        if err := cozoStorage.EnsureSchema(ctx); err != nil {
            cozoStorage.Close()
            duckDBStorage.Close()
            return nil, fmt.Errorf("failed to create CozoDB schema: %w", err)
        }

        vectorStorage = duckDBStorage
        graphStorage = cozoStorage

        fmt.Println("CogneeService: DuckDB+CozoDB mode initialized successfully")
    }

    // ========================================
    // 共通の初期化処理
    // ========================================
    // Embedder, S3Client などは既存のまま
    // ...

    return &CogneeService{
        VectorStorage: vectorStorage,
        GraphStorage:  graphStorage,
        // Embedder, LLMClient, S3Client は既存のまま
    }, nil
}
```

---

## Step 3: KuzuDBStorageインポート追加

### 3.1 DuckDB + CozoDB での参照実装

```go
// cognee.go より
import (
    "database/sql"
    
    cozo "github.com/cozodb/cozo-lib-go"
    _ "github.com/marcboeker/go-duckdb"
    
    "mycute/pkg/cognee/db/cozodb"
    "mycute/pkg/cognee/db/duckdb"
    "mycute/pkg/cognee/storage"
)
```

### 3.2 KuzuDB追加

```go
import (
    "database/sql"
    
    cozo "github.com/cozodb/cozo-lib-go"
    _ "github.com/marcboeker/go-duckdb"
    
    "mycute/pkg/cognee/db/cozodb"
    "mycute/pkg/cognee/db/duckdb"
    "mycute/pkg/cognee/db/kuzudb"  // Phase-10追加
    "mycute/pkg/cognee/storage"
)
```

---

## Step 4: 環境変数サポート追加

### 4.1 DuckDB + CozoDB での参照実装

```go
// settings.go より
var (
    COGNEE_DB_DIR   = os.Getenv("COGNEE_DB_DIR")
    DATAPIPE_URL    = os.Getenv("DATAPIPE_URL")
    OPENAI_API_KEY  = os.Getenv("OPENAI_API_KEY")
    // ...
)
```

### 4.2 KuzuDB追加

```go
// settings.go に追加
var (
    // ... 既存の環境変数 ...
    
    // Phase-10追加: データベースモード設定
    
    // COGNEE_DATABASE_MODE: データベースの動作モードを指定
    //   - "duckdb+cozodb" (デフォルト): DuckDB + CozoDB
    //   - "kuzudb": KuzuDBのみ
    COGNEE_DATABASE_MODE = os.Getenv("COGNEE_DATABASE_MODE")

    // COGNEE_KUZUDB_PATH: KuzuDBデータベースのパス（オプション）
    // 空の場合は COGNEE_DB_DIR + "/kuzudb" を使用
    COGNEE_KUZUDB_PATH = os.Getenv("COGNEE_KUZUDB_PATH")
)
```

### 4.3 main.goでの設定読み取り

```go
// main.go で CogneeConfig 作成時
cogneeConfig := cognee.CogneeConfig{
    // 既存フィールド...
    COGNEE_DB_DIR:  cfg.COGNEE_DB_DIR,
    DATAPIPE_URL:   cfg.DATAPIPE_URL,
    OPENAI_API_KEY: cfg.OPENAI_API_KEY,
    // ...
    
    // Phase-10追加
    DatabaseMode:       cfg.COGNEE_DATABASE_MODE,
    KuzuDBDatabasePath: cfg.COGNEE_KUZUDB_PATH,
}
```

---

## Step 5: test-kuzudb-integrationコマンド追加

### 5.1 目的

CogneeServiceがKuzuDBモードで正常に動作することを確認する統合テスト。
DuckDB+CozoDBモードと同等の操作がKuzuDBモードでも動作することを検証する。

### 5.2 DuckDB + CozoDB での参照実装

既存の test-metabolism, test-pruning などのテストでは、以下のようにCogneeServiceを初期化している：

```go
// 既存のテストコマンドより
cogneeConfig := cognee.CogneeConfig{
    COGNEE_DB_DIR:  cfg.COGNEE_DB_DIR,
    // ... 他の設定 ...
}
cogneeService, err := cognee.NewCogneeService(cogneeConfig)
if err != nil {
    log.Fatalf("Failed to create CogneeService: %v", err)
}
defer cogneeService.Close()

// VectorStorage を使用
cogneeService.VectorStorage.SaveData(ctx, data)

// GraphStorage を使用
cogneeService.GraphStorage.AddNodes(ctx, nodes)
```

**根拠説明**: 既存テストは CogneeService を通じて VectorStorage と GraphStorage にアクセスしている。KuzuDBモードでも同じインターフェースを使用するため、テストコードを大きく変更する必要はない。

### 5.3 KuzuDB統合テスト実装

```go
    case "test-kuzudb-integration":
        // ============================================================
        // Phase-10F: KuzuDB統合テスト
        // ============================================================
        // このテストは、CogneeServiceがKuzuDBモードで正常に動作することを確認します。
        //
        // DuckDB + CozoDB 参照:
        //   cogneeConfig := cognee.CogneeConfig{...}
        //   cogneeService, _ := cognee.NewCogneeService(cogneeConfig)
        //   cogneeService.VectorStorage.SaveData(ctx, data)
        //   cogneeService.GraphStorage.AddNodes(ctx, nodes)
        //
        // KuzuDBモードでも同じインターフェースで操作可能なことを確認
        // ============================================================
        
        log.Println("--- Phase 10F Test: KuzuDB Integration ---")
        
        // KuzuDBモードで CogneeService を作成
        kuzuDBConfig := cognee.CogneeConfig{
            COGNEE_DB_DIR:              cfg.COGNEE_DB_DIR + "/kuzudb_integration_test",
            DATAPIPE_URL:               cfg.DATAPIPE_URL,
            OPENAI_API_KEY:             cfg.OPENAI_API_KEY,
            DatabaseMode:               "kuzudb",  // ← KuzuDBモードを指定
            // その他の設定...
        }
        
        kuzuDBService, err := cognee.NewCogneeService(kuzuDBConfig)
        if err != nil {
            log.Fatalf("❌ Failed to create CogneeService in KuzuDB mode: %v", err)
        }
        defer func() {
            kuzuDBService.Close()
            os.RemoveAll(kuzuDBConfig.COGNEE_DB_DIR)
            log.Println("✅ Test database cleaned up")
        }()
        log.Println("✅ CogneeService created in KuzuDB mode")

        // ========================================
        // 1. VectorStorage基本テスト
        // ========================================
        // DuckDB参照:
        //   duckDBStorage.SaveData(ctx, data)
        //   duckDBStorage.Exists(ctx, contentHash, groupID)
        //
        // KuzuDBでも同じインターフェースで操作
        log.Println("Testing VectorStorage via CogneeService...")
        
        testData := &storage.Data{
            ID:                   "integration_data_1",
            GroupID:              "test_user-test_dataset",
            Name:                 "integration_test.txt",
            RawDataLocation:      "/tmp/test.txt",
            OriginalDataLocation: "/original/test.txt",
            Extension:            ".txt",
            MimeType:             "text/plain",
            ContentHash:          "integration_hash_123",
            OwnerID:              "test_user",
            CreatedAt:            time.Now(),
        }
        
        // DuckDB: duckDBStorage.SaveData(ctx, data)
        // KuzuDB: kuzuDBService.VectorStorage.SaveData(ctx, data) - 同じインターフェース
        if err := kuzuDBService.VectorStorage.SaveData(ctx, testData); err != nil {
            log.Fatalf("❌ VectorStorage.SaveData failed: %v", err)
        }
        log.Println("  ✅ VectorStorage.SaveData: OK")
        
        // DuckDB: duckDBStorage.Exists(ctx, contentHash, groupID)
        // KuzuDB: kuzuDBService.VectorStorage.Exists(ctx, contentHash, groupID) - 同じインターフェース
        if !kuzuDBService.VectorStorage.Exists(ctx, "integration_hash_123", "test_user-test_dataset") {
            log.Fatalf("❌ VectorStorage.Exists returned false")
        }
        log.Println("  ✅ VectorStorage.Exists: OK")

        // ========================================
        // 2. GraphStorage基本テスト
        // ========================================
        // CozoDB参照:
        //   cozoStorage.AddNodes(ctx, nodes)
        //   cozoStorage.AddEdges(ctx, edges)
        //
        // KuzuDBでも同じインターフェースで操作
        log.Println("Testing GraphStorage via CogneeService...")
        
        testNodes := []*storage.Node{
            {
                ID:      "integration_node_1",
                GroupID: "test_user-test_dataset",
                Type:    "TestEntity",
                Properties: map[string]any{
                    "name":       "Integration Test Entity",
                    "created_at": time.Now().Add(-2 * time.Hour).Format(time.RFC3339),
                },
            },
            {
                ID:      "integration_node_2",
                GroupID: "test_user-test_dataset",
                Type:    "TestEntity",
                Properties: map[string]any{
                    "name":       "Another Test Entity",
                    "created_at": time.Now().Add(-2 * time.Hour).Format(time.RFC3339),
                },
            },
        }
        
        // CozoDB: cozoStorage.AddNodes(ctx, nodes)
        // KuzuDB: kuzuDBService.GraphStorage.AddNodes(ctx, nodes) - 同じインターフェース
        if err := kuzuDBService.GraphStorage.AddNodes(ctx, testNodes); err != nil {
            log.Fatalf("❌ GraphStorage.AddNodes failed: %v", err)
        }
        log.Println("  ✅ GraphStorage.AddNodes: OK")
        
        testEdges := []*storage.Edge{
            {
                SourceID:   "integration_node_1",
                TargetID:   "integration_node_2",
                GroupID:    "test_user-test_dataset",
                Type:       "RELATED_TO",
                Weight:     0.9,
                Confidence: 0.95,
            },
        }
        
        // CozoDB: cozoStorage.AddEdges(ctx, edges)
        // KuzuDB: kuzuDBService.GraphStorage.AddEdges(ctx, edges) - 同じインターフェース
        if err := kuzuDBService.GraphStorage.AddEdges(ctx, testEdges); err != nil {
            log.Fatalf("❌ GraphStorage.AddEdges failed: %v", err)
        }
        log.Println("  ✅ GraphStorage.AddEdges: OK")

        // ========================================
        // 3. 検索テスト（SaveEmbedding + Search）
        // ========================================
        // DuckDB参照:
        //   duckDBStorage.SaveEmbedding(ctx, collectionName, id, text, embedding, groupID)
        //   duckDBStorage.Search(ctx, collectionName, queryVector, k, groupID)
        log.Println("Testing Search functionality...")
        
        testEmbedding := []float32{0.1, 0.2, 0.3, 0.4, 0.5}
        if err := kuzuDBService.VectorStorage.SaveEmbedding(
            ctx, "TestCollection", "emb_integration", "Integration test text", testEmbedding, "test_user-test_dataset",
        ); err != nil {
            log.Fatalf("❌ VectorStorage.SaveEmbedding failed: %v", err)
        }
        log.Println("  ✅ VectorStorage.SaveEmbedding: OK")
        
        results, err := kuzuDBService.VectorStorage.Search(ctx, "TestCollection", testEmbedding, 5, "test_user-test_dataset")
        if err != nil {
            log.Fatalf("❌ VectorStorage.Search failed: %v", err)
        }
        if len(results) < 1 {
            log.Fatalf("❌ VectorStorage.Search returned no results")
        }
        log.Printf("  ✅ VectorStorage.Search: Found %d results", len(results))

        // ========================================
        // 4. GetTriplets テスト
        // ========================================
        // CozoDB参照:
        //   cozoStorage.GetTriplets(ctx, nodeIDs, groupID)
        log.Println("Testing GetTriplets...")
        
        triplets, err := kuzuDBService.GraphStorage.GetTriplets(ctx, []string{"integration_node_1"}, "test_user-test_dataset")
        if err != nil {
            log.Fatalf("❌ GraphStorage.GetTriplets failed: %v", err)
        }
        if len(triplets) < 1 {
            log.Fatalf("❌ GraphStorage.GetTriplets returned no triplets")
        }
        log.Printf("  ✅ GraphStorage.GetTriplets: Found %d triplets", len(triplets))

        // ========================================
        // 5. UpdateEdgeMetrics テスト
        // ========================================
        // CozoDB参照:
        //   cozoStorage.UpdateEdgeMetrics(ctx, sourceID, targetID, groupID, weight, confidence)
        log.Println("Testing UpdateEdgeMetrics...")
        
        if err := kuzuDBService.GraphStorage.UpdateEdgeMetrics(
            ctx, "integration_node_1", "integration_node_2", "test_user-test_dataset", 0.99, 0.999,
        ); err != nil {
            log.Fatalf("❌ GraphStorage.UpdateEdgeMetrics failed: %v", err)
        }
        log.Println("  ✅ GraphStorage.UpdateEdgeMetrics: OK")

        // ========================================
        // 6. GetOrphanNodes テスト
        // ========================================
        // CozoDB参照:
        //   cozoStorage.GetOrphanNodes(ctx, groupID, gracePeriod)
        log.Println("Testing GetOrphanNodes...")
        
        orphans, err := kuzuDBService.GraphStorage.GetOrphanNodes(ctx, "test_user-test_dataset", 1*time.Hour)
        if err != nil {
            log.Fatalf("❌ GraphStorage.GetOrphanNodes failed: %v", err)
        }
        log.Printf("  ✅ GraphStorage.GetOrphanNodes: Found %d orphans", len(orphans))

        // ========================================
        // 結果サマリー
        // ========================================
        log.Println("========================================")
        log.Println("Phase-10F Integration Test Summary:")
        log.Println("  ✅ CogneeService (KuzuDB mode): PASSED")
        log.Println("  ✅ VectorStorage operations: PASSED")
        log.Println("  ✅ GraphStorage operations: PASSED")
        log.Println("  ✅ Search functionality: PASSED")
        log.Println("  ✅ Graph queries (Triplets): PASSED")
        log.Println("  ✅ Edge/Node management: PASSED")
        log.Println("========================================")
        log.Println("✅ test-kuzudb-integration PASSED")
```

---

## Step 6: 既存テストのKuzuDBモード対応

### 6.1 目的

環境変数 `COGNEE_DATABASE_MODE` が設定されている場合、そのモードでテストを実行する。

### 6.2 DuckDB + CozoDB での参照実装

既存テストでは CogneeConfig に特定のデータベースモードを指定していない：

```go
// 既存テストでの初期化
cogneeConfig := cognee.CogneeConfig{
    COGNEE_DB_DIR: cfg.COGNEE_DB_DIR,
    // DatabaseMode は指定なし → デフォルトで "duckdb+cozodb"
}
```

### 6.3 KuzuDB対応実装

```go
// main.go の初期化部分
// 環境変数からデータベースモードを取得
databaseMode := cfg.COGNEE_DATABASE_MODE
if databaseMode == "" {
    databaseMode = "duckdb+cozodb" // デフォルト
}

// 既存テスト (test-metabolism, test-pruning, ...) での CogneeConfig
cogneeConfig := cognee.CogneeConfig{
    // 既存フィールド...
    COGNEE_DB_DIR: cfg.COGNEE_DB_DIR,
    // ...
    
    // Phase-10追加: 環境変数から読み取ったモードを設定
    DatabaseMode:       databaseMode,
    KuzuDBDatabasePath: cfg.COGNEE_KUZUDB_PATH,
}
```

### 6.4 テスト実行例

```bash
# DuckDB+CozoDB モード（デフォルト）
./dist/mycute-darwin-arm64 test-metabolism

# KuzuDB モード
COGNEE_DATABASE_MODE=kuzudb ./dist/mycute-darwin-arm64 test-metabolism
```

---

## Step 7: ビルドと全テスト実行

### 7.1 ビルド確認

```bash
make build
make build-linux-amd64
```

### 7.2 全テスト実行（DuckDB+CozoDBモード）

```bash
# Phase-10で追加したテスト
./dist/mycute-darwin-arm64 test-kuzudb-build
./dist/mycute-darwin-arm64 test-kuzudb-schema
./dist/mycute-darwin-arm64 test-kuzudb-vector
./dist/mycute-darwin-arm64 test-kuzudb-graph
./dist/mycute-darwin-arm64 test-kuzudb-integration

# 既存テスト（DuckDB+CozoDBモードで続行可能であることを確認）
./dist/mycute-darwin-arm64 test-metabolism
./dist/mycute-darwin-arm64 test-pruning
./dist/mycute-darwin-arm64 test-crystallization
./dist/mycute-darwin-arm64 benchmark-optimization -n 50
```

### 7.3 全テスト実行（KuzuDBモード）

```bash
export COGNEE_DATABASE_MODE=kuzudb

# KuzuDBモードで既存テストを実行
./dist/mycute-darwin-arm64 test-metabolism
./dist/mycute-darwin-arm64 test-pruning
./dist/mycute-darwin-arm64 test-crystallization
./dist/mycute-darwin-arm64 benchmark-optimization -n 50
```

---

## Step 8: パフォーマンス比較ベンチマーク

### 8.1 目的

DuckDB+CozoDBモードとKuzuDBモードのパフォーマンスを比較する。

### 8.2 追加するコマンド

```go
    case "benchmark-db-comparison":
        // Phase-10F: DuckDB+CozoDB vs KuzuDB パフォーマンス比較
        // 
        // 比較対象:
        //   - ノード追加速度
        //   - エッジ追加速度
        //   - 検索速度
        //   - GetOrphanNodes速度
        log.Println("--- Phase 10F: Database Performance Comparison ---")
        
        benchCmd := flag.NewFlagSet("benchmark-db-comparison", flag.ExitOnError)
        nodeCountPtr := benchCmd.Int("n", 100, "Number of test nodes")
        benchCmd.Parse(os.Args[2:])
        
        nodeCount := *nodeCountPtr
        groupID := "benchmark_user-benchmark_dataset"
        
        // ========================================
        // DuckDB + CozoDB ベンチマーク
        // ========================================
        log.Println("Benchmarking DuckDB + CozoDB mode...")
        
        duckCozoConfig := cognee.CogneeConfig{
            COGNEE_DB_DIR:  cfg.COGNEE_DB_DIR + "/bench_duckcozodb",
            DatabaseMode:   "duckdb+cozodb",
            // その他設定...
        }
        
        dcService, _ := cognee.NewCogneeService(duckCozoConfig)
        
        // ノード追加ベンチマーク
        testNodes := make([]*storage.Node, nodeCount)
        for i := 0; i < nodeCount; i++ {
            testNodes[i] = &storage.Node{
                ID:      fmt.Sprintf("dc_node_%d", i),
                GroupID: groupID,
                Type:    "BenchNode",
                Properties: map[string]any{
                    "value":      i,
                    "created_at": time.Now().Add(-2 * time.Hour).Format(time.RFC3339),
                },
            }
        }
        
        startDC := time.Now()
        dcService.GraphStorage.AddNodes(ctx, testNodes)
        dcAddDuration := time.Since(startDC)
        
        // オーファン検出ベンチマーク
        startDCOrphan := time.Now()
        dcService.GraphStorage.GetOrphanNodes(ctx, groupID, 1*time.Hour)
        dcOrphanDuration := time.Since(startDCOrphan)
        
        dcService.Close()
        os.RemoveAll(duckCozoConfig.COGNEE_DB_DIR)
        
        // ========================================
        // KuzuDB ベンチマーク
        // ========================================
        log.Println("Benchmarking KuzuDB mode...")
        
        kuzuDBConfig := cognee.CogneeConfig{
            COGNEE_DB_DIR:  cfg.COGNEE_DB_DIR + "/bench_kuzudb",
            DatabaseMode:   "kuzudb",
            // その他設定...
        }
        
        kuzuDBService, _ := cognee.NewCogneeService(kuzuDBConfig)
        
        // ノード追加ベンチマーク
        for i := 0; i < nodeCount; i++ {
            testNodes[i].ID = fmt.Sprintf("kuzudb_node_%d", i)
        }
        
        startKuzuDB := time.Now()
        kuzuDBService.GraphStorage.AddNodes(ctx, testNodes)
        kuzuDBAddDuration := time.Since(startKuzuDB)
        
        // オーファン検出ベンチマーク
        startKuzuDBOrphan := time.Now()
        kuzuDBService.GraphStorage.GetOrphanNodes(ctx, groupID, 1*time.Hour)
        kuzuDBOrphanDuration := time.Since(startKuzuDBOrphan)
        
        kuzuDBService.Close()
        os.RemoveAll(kuzuDBConfig.COGNEE_DB_DIR)
        
        // ========================================
        // 結果サマリー
        // ========================================
        log.Println("========================================")
        log.Println("Database Performance Comparison:")
        log.Printf("  Nodes: %d", nodeCount)
        log.Println("----------------------------------------")
        log.Println("  AddNodes:")
        log.Printf("    DuckDB+CozoDB: %v", dcAddDuration)
        log.Printf("    KuzuDB:        %v", kuzuDBAddDuration)
        log.Println("  GetOrphanNodes:")
        log.Printf("    DuckDB+CozoDB: %v", dcOrphanDuration)
        log.Printf("    KuzuDB:        %v", kuzuDBOrphanDuration)
        log.Println("========================================")
```

---

## 9. 環境変数まとめ

| 環境変数 | 説明 | デフォルト値 |
|---------|------|-------------|
| `COGNEE_DATABASE_MODE` | データベースモード | `duckdb+cozodb` |
| `COGNEE_KUZUDB_PATH` | KuzuDBデータベースパス | `COGNEE_DB_DIR/kuzudb` |

---

## 10. 成功条件チェックリスト

### Phase-10F 完了条件

- [ ] CogneeConfigに`DatabaseMode`と`KuzuDBDatabasePath`が追加されている
- [ ] NewCogneeServiceでモード分岐が実装されている
- [ ] settings.goに環境変数が追加されている
- [ ] `test-kuzudb-integration`コマンドがPASSED
- [ ] 既存テストがDuckDB+CozoDBモードで引き続き動作
- [ ] 既存テストがKuzuDBモードでも動作
- [ ] `benchmark-db-comparison`で両モードの比較が可能
- [ ] `make build`がエラーなしで成功
- [ ] `make build-linux-amd64`がエラーなしで成功

---

## 11. Phase-10 完了サマリー

Phase-10の全サブフェーズが完了した時点で、以下が達成される：

### 11.1 機能面

| 機能 | DuckDB+CozoDB | KuzuDB |
|------|---------------|--------|
| VectorStorage | ✅ DuckDBStorage | ✅ KuzuDBStorage |
| GraphStorage | ✅ CozoStorage | ✅ KuzuDBStorage |
| 既存テスト | ✅ PASSED | ✅ PASSED |
| ベンチマーク | ✅ | ✅ |

### 11.2 アーキテクチャ

- **モード切替**: `COGNEE_DATABASE_MODE` 環境変数で選択可能
- **コード共有**: storage.Interfaceを通じて同じ上位レイヤーコードを使用
- **データ分離**: モードごとに異なるデータベースファイルを使用

### 11.3 今後の拡張

- KuzuDBの高度なグラフ機能（パス検索、サブグラフなど）の活用
- ベクトルインデックス（HNSW）の本格活用
- パフォーマンスチューニング

---

## 12. ドキュメント更新

Phase-10完了後、以下のドキュメントを更新：

1. **README.md**: データベースモード設定の説明を追加
2. **docs/AFTER-PHASE-10.md**: Phase-10の成果と学びを記録（新規作成）
3. **.env.example**: 新しい環境変数を追加

```env
# Database Mode Configuration
# COGNEE_DATABASE_MODE=duckdb+cozodb  # Default: DuckDB + CozoDB
# COGNEE_DATABASE_MODE=kuzudb         # Alternative: KuzuDB only

# KuzuDB Configuration (only used when COGNEE_DATABASE_MODE=kuzudb)
# COGNEE_KUZUDB_PATH=./db/kuzudb
```
