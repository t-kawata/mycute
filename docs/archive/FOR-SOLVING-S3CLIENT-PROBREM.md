# S3Client 統合計画 (S3Client Integration Plan)

本ドキュメントは、`src/pkg/s3client` を Cognee に統合し、ローカルとS3の両方で統一的なファイル操作を実現するための詳細な実装計画です。

## 1. 問題の所在 (Problem)

現在、`IngestTask` や `ChunkingTask` は `os.Open` や `os.ReadFile` を使用して直接ファイルシステムにアクセスしています。
これにより以下の問題が発生しています：
1.  **S3非対応**: S3バケットへの保存やS3からの読み込みに対応していない。
2.  **抽象化の無視**: 既に実装されている `S3Client` の抽象化レイヤーが活用されていない。
3.  **ポータビリティの欠如**: 環境（ローカル/クラウド）に応じた柔軟なファイル管理ができない。

## 2. 解決策 (Solution)

`CogneeService` に `S3Client` を組み込み、各タスク（Ingest, Chunking）に注入します。
これにより、タスク側は「ファイルの保存場所（ローカルパスかS3キーか）」を意識せず、`S3Client` のメソッド（`Up`, `Down`）を通じて統一的にファイルを扱えるようになります。

---

## 3. 実装詳細 (Implementation Details)

### 3.1 設定の更新 (`src/pkg/cognee/cognee.go`)

**変更点**: `CogneeConfig` にS3関連の設定を追加します。

```go
type CogneeConfig struct {
    // ... 既存の設定 ...

    // Storage Configuration
    S3UseLocal bool   // trueならローカルストレージを使用
    S3LocalDir string // ローカル保存先ディレクトリ (例: "data/files")
    
    // AWS S3 Configuration (S3UseLocal=falseの場合に使用)
    S3AccessKey string
    S3SecretKey string
    S3Region    string
    S3Bucket    string
}
```

**理由**: 
設定を一元管理することで、`NewCogneeService` 内で `S3Client` を初期化しやすくなります。環境変数からこれらの値を読み込むように `main.go` も修正します。

### 3.2 サービスの初期化 (`src/pkg/cognee/cognee.go`)

**変更点**: `CogneeService` に `S3Client` を持たせ、初期化時にインスタンスを作成します。

```go
import "mycute/pkg/s3client"

type CogneeService struct {
    // ... 既存フィールド ...
    S3Client *s3client.S3Client // 追加
}

func NewCogneeService(config CogneeConfig) (*CogneeService, error) {
    // ... DB初期化 ...

    // S3Clientの初期化
    // ダウンロード用ディレクトリは一時ディレクトリまたはキャッシュディレクトリを指定
    downDir := filepath.Join(config.DBDirPath, "downloads")
    
    s3Client, err := s3client.NewS3Client(
        config.S3AccessKey,
        config.S3SecretKey,
        config.S3Region,
        config.S3Bucket,
        config.S3LocalDir, // アップロード先（ローカルモード時）
        downDir,           // ダウンロード先（キャッシュ）
        config.S3UseLocal,
    )
    if err != nil {
        return nil, fmt.Errorf("failed to initialize S3Client: %w", err)
    }

    return &CogneeService{
        // ...
        S3Client: s3Client,
    }, nil
}
```

**理由**: 
サービス層で `S3Client` を保持することで、各メソッド（`Add`, `Cognify` 等）からタスクへ容易にクライアントを渡すことができます。DI（依存性の注入）のパターンに従います。

### 3.3 IngestTask の修正 (`src/pkg/cognee/tasks/ingestion/ingest_task.go`)

**変更点**: ファイルを直接読み込むのではなく、`S3Client.Up` を使用してストレージに保存し、そのパス（キー）を記録します。

```go
type IngestTask struct {
    vectorStorage storage.VectorStorage
    groupID       string
    s3Client      *s3client.S3Client // 追加
}

func NewIngestTask(vectorStorage storage.VectorStorage, groupID string, s3Client *s3client.S3Client) *IngestTask {
    return &IngestTask{
        vectorStorage: vectorStorage,
        groupID:       groupID,
        s3Client:      s3Client,
    }
}

func (t *IngestTask) Run(ctx context.Context, input any) (any, error) {
    // ...
    for _, path := range filePaths {
        // 1. ハッシュ計算 (ローカルの元ファイルを使用)
        hash, err := calculateFileHash(path)
        // ...

        // 2. ファイルのアップロード/保存
        // S3Client.Up は、ローカルモードなら指定ディレクトリにコピー、S3モードならアップロードを行い、
        // 保存先のキー（相対パス）を返します。
        storageKey, err := t.s3Client.Up(path)
        if err != nil {
            return nil, fmt.Errorf("failed to upload file %s: %w", path, err)
        }

        // 3. データオブジェクト作成
        data := &storage.Data{
            // ...
            RawDataLocation: *storageKey, // 保存された場所のキーを記録
            // ...
        }
        // ...
    }
    // ...
}
```

**理由**: 
`RawDataLocation` にローカルの絶対パスではなく、ストレージ上のキー（S3キーまたは管理されたローカルパス）を保存することで、データの場所を抽象化できます。これにより、どのマシンからでも（S3経由で）データにアクセス可能になります。

### 3.4 ChunkingTask の修正 (`src/pkg/cognee/tasks/chunking/chunking_task.go`)

**変更点**: `RawDataLocation` からファイルを読み込む際、`S3Client.Down` を使用してローカルに取得（キャッシュ）してから読み込みます。

```go
type ChunkingTask struct {
    // ...
    s3Client *s3client.S3Client // 追加
}

func NewChunkingTask(..., s3Client *s3client.S3Client) (*ChunkingTask, error) {
    // ...
}

func (t *ChunkingTask) Run(ctx context.Context, input any) (any, error) {
    // ...
    for _, data := range dataList {
        // ファイルを取得（S3ならダウンロード、ローカルならパス解決）
        localPath, err := t.s3Client.Down(data.RawDataLocation)
        if err != nil {
            return nil, fmt.Errorf("failed to download file %s: %w", data.RawDataLocation, err)
        }

        // 取得したローカルパスから読み込み
        content, err := os.ReadFile(*localPath)
        // ...
    }
    // ...
}
```

**理由**: 
`S3Client.Down` は、S3にあるファイルをローカルのキャッシュディレクトリにダウンロードし、そのパスを返します。ローカルモードの場合は単にファイルのパスを返します。
これにより、`ChunkingTask` はファイルが物理的にどこにあるかを気にせず、常にローカルファイルとして処理できるようになります。

### 3.5 メイン関数の修正 (`src/main.go`)

**変更点**: 環境変数からS3設定を読み込みます。

```go
    config := cognee.CogneeConfig{
        // ...
        
        // Storage Config
        S3UseLocal: func() bool {
            if v := os.Getenv("COGNEE_S3_USE_LOCAL"); v == "false" {
                return false
            }
            return true // デフォルトはローカル
        }(),
        S3LocalDir:  "data/files", // デフォルトの保存先
        S3AccessKey: os.Getenv("AWS_ACCESS_KEY_ID"),
        S3SecretKey: os.Getenv("AWS_SECRET_ACCESS_KEY"),
        S3Region:    os.Getenv("AWS_REGION"),
        S3Bucket:    os.Getenv("S3_BUCKET"),
    }
```

### 3.6 環境変数の更新 (`src/.env.example`, `src/.env`)

**変更点**: 新しい環境変数を定義ファイルに追加します。

```bash
# S3 Configuration
# COGNEE_S3_USE_LOCAL=true # true: Local storage, false: S3 storage
# AWS_ACCESS_KEY_ID=your_access_key
# AWS_SECRET_ACCESS_KEY=your_secret_key
# AWS_REGION=us-east-1
# S3_BUCKET=your_bucket_name
```

**理由**: 
開発者が新しい設定項目を認識し、適切に設定できるようにするためです。


## 4. テスト計画 (Test Plan)

改修後、以下の手順でクリーンインストール状態からの完全な動作確認を行います。

### 4.1 準備 (Cleanup)

データベースとデータファイルを完全に削除し、初期状態に戻します。

```bash
# データのクリーンアップ
rm -rf data/cognee.duckdb
rm -rf data/cognee.cozodb
rm -rf data/files/*
rm -rf data/downloads/*

# テストデータの準備
mkdir -p test_data
echo "Cognee S3Client Integration Test Data" > test_data/s3_test.txt
```

### 4.2 テストケース 1: ローカルモード (Default)

環境変数 `COGNEE_S3_USE_LOCAL=true` (または未設定) で実行。

1.  **Add**: `make run ARGS="add -f test_data/s3_test.txt"`
    *   **確認**: `data/files/YYYY/MM/.../s3_test.txt` が作成されていること。
    *   **確認**: ログに `Ingested file: s3_test.txt` が表示されること。
2.  **Cognify**: `make run ARGS="cognify"`
    *   **確認**: エラーなく完了すること。
    *   **確認**: 内部で `S3Client.Down` が呼ばれ、ファイルが読み込まれていること。
3.  **Search**: `make run ARGS="search -q 'Test Data'"`
    *   **確認**: 検索結果が返ってくること。

### 4.3 テストケース 2: S3モード (Optional / If Credentials Available)

AWS認証情報がある場合のみ実施。
環境変数: `COGNEE_S3_USE_LOCAL=false`, `AWS_...`, `S3_BUCKET` を設定。

1.  **Cleanup**: (4.1と同様)
2.  **Add**: `make run ARGS="add -f test_data/s3_test.txt"`
    *   **確認**: 指定したS3バケットにファイルがアップロードされていること。
3.  **Cognify**: `make run ARGS="cognify"`
    *   **確認**: S3から `data/downloads/...` にファイルがダウンロードされ、処理されること。

### 4.4 テストケース 3: Memify (Integration Check)

`Memify` は `Cognify` で作成されたグラフデータを使用するため、ファイルアクセスは直接行いませんが、システム全体の整合性を確認します。

1.  **Memify**: `make run ARGS="memify"`
    *   **確認**: エラーなく完了すること。

---

## 5. 結論 (Conclusion)

この計画により、`S3Client` が適切に統合され、Cogneeはローカル環境とクラウド環境の両方でシームレスに動作するようになります。
特に `IngestTask` と `ChunkingTask` の修正により、ファイルアクセスの抽象化が完了し、将来的なストレージ変更にも強い設計となります。
