# Phase-16 Development Directives

本フェーズの目的は、埋め込みモデル（Embeddings）の設定と認証情報をグローバル設定（`CuberConfig`）から完全撤廃し、全ての制御を Cube 単位のデータベースレコード（`model.Cube`）に移行することです。
これにより、Cube ごとに異なるプロバイダーや API キーを使用可能にし、Import/Create 時の実在確認（テスト埋め込み）による厳格なバリデーションを実現します。

## コーディングルール (厳守)

1.  **エラーメッセージの大文字開始**: `fmt.Errorf("Invalid API key")` のように先頭を大文字にする。
2.  **無駄な改行の禁止**: 関数内のステートメント間に空行を入れない。コメントで論理ブロックを分ける。
3.  **バリデーションの責務**: `rtreq` で値の検証（テスト埋め込み含む）を完結させる。

---

## 1. データベースモデルの拡張 (MySQL)

`src/model/db_model.go` の `Cube` 構造体に `EmbeddingApiKey` を追加します。

```go
type Cube struct {
    // ... 既存フィールド ...
    // EmbeddingProvider, EmbeddingModel, EmbeddingDimension は Phase-15 で既に追加済み
    EmbeddingApiKey string `gorm:"size:1024;not null;default:''"` // [NEW] APIキー (暗号化保存)
}
```

**暗号化ルール (必須):**
- DBへの保存（Create/Import）の際は、`src/lib/mycrypto` パッケージの `Encrypt` 関数を使用して暗号化してください。
- DBからの読み出し（CuberServiceへのパス）の際は、`Decrypt` 関数を使用して復号化してください。
- 暗号化によるサイズ増加（Hexエンコード等）を考慮し、カラムサイズは `size:1024` とします。
- 暗号化キーは `os.Getenv("CUBER_CRYPTO_SECRET_KEY")` という環境変数キーを使用してください（既存の `mycrypto` 利用箇所を参考に）。

## 2. Configのクリーンアップ

`src/pkg/cuber/types/config_types.go` の `CuberConfig` から、以下の埋め込み関連フィールドを**完全削除**してください。

- `EmbeddingsAPIKey`
- `EmbeddingsBaseURL`
- `EmbeddingsModel`

これに伴い、`src/pkg/cuber/cuber.go` の `NewCuberService` にあるグローバルな Embedder 初期化ロジックも削除します。
`CuberService` 構造体から `Embedder` フィールドを削除し、各メソッド実行時に都度 Embedder を生成するか、引数として受け取る設計に変更します。

**環境変数の掃除:**
`src/mode/rt/main_of_rt.go` 等で、上記の削除されたフィールド及びEmbedding関連環境変数読み込みロジック（`os.Getenv` など）が残らないように、丁寧に除去してください。不要になったコード（ゴミ実装）を残さないことが絶対条件です。

埋め込みモデルの設定をグローバルから削除するための具体的な手順です。

### 2.1. 設定定義の削除 (`src/pkg/cuber/types/config_types.go`)

`CuberConfig` 構造体から以下のフィールドを削除します。

```go
type CuberConfig struct {
    // ...
    // Completion (テキスト生成) LLM の設定
    CompletionAPIKey    string
    CompletionBaseURL   string
    CompletionModel     string
    CompletionMaxTokens int

    // [削除ターゲット] 以下のブロックを完全に削除してください
    // Embeddings (ベクトル化) LLM の設定
    // EmbeddingsAPIKey  string
    // EmbeddingsBaseURL string
    // EmbeddingsModel   string

    // ...
}
```

### 2.2. 環境変数読み込み処理の削除 (`src/mode/rt/main_of_rt.go`)

`MainOfRT` 関数内の環境変数読み込みとバリデーション、設定への代入処理を削除します。

**変更対象:**

1.  変数定義の削除:
    ```go
    // [削除]
    // EMBEDDINGS_API_KEY := os.Getenv("EMBEDDINGS_API_KEY")
    // EMBEDDINGS_MODEL := os.Getenv("EMBEDDINGS_MODEL")
    ```
 ## 2. 環境変数と共通ユーティリティの整理

### 2.1. 環境変数の追加と削除

#### 2.1.1. 追加 (`src/.env`, `src/.env.sample`)

`src/.env` および `src/.env.sample` (または `.env.example`) に `CUBER_CRYPTO_SECRET_KEY` を追加します。

```env
# EmbeddingApiKey (DB保存時) 等の暗号化に使用する秘密鍵
CUBER_CRYPTO_SECRET_KEY=your_secret_key_at_least_32_bytes
```

#### 2.1.2. 削除 (`src/.env`, `src/.env.sample`)

以下の既存の環境変数を削除します。
- `EMBEDDINGS_BASE_URL`
- `EMBEDDINGS_API_KEY`
- `EMBEDDINGS_MODEL`

### 2.2. RtUtil へのフィールド追加 (`src/mode/rt/rtutil/rtutil.go`)

`rtbl` 層で安全に環境変数値（暗号化キー）を利用するため、`RtUtil` 構造体にフィールドを追加します。

```go
type RtUtil struct {
    // ... 既存フィールド ...
    CuberService *cuber.CuberService
    // [NEW]
    CryptoSecretKey string
}
```

### 2.3. Main 処理での環境変数読み込み (`src/mode/rt/main_of_rt.go`)

`src/mode/rt/main_of_rt.go` の `initRtUtil` (または `RtUtil` を初期化している箇所) で、環境変数 `CUBER_CRYPTO_SECRET_KEY` を読み込み、`RtUtil.CryptoSecretKey` に設定します。

```go
// main_of_rt.go
func main() {
    // ...
    // 環境変数読み込み
    cryptoKey := os.Getenv("CUBER_CRYPTO_SECRET_KEY")
    if cryptoKey == "" {
        // 必須なのでPanicまたはログ出力して終了
        panic("CUBER_CRYPTO_SECRET_KEY is required")
    }

    // ...
    // RtUtil 初期化時
    u := &rtutil.RtUtil{
        // ...
        CryptoSecretKey: cryptoKey,
    }
    // ...
}
```

### 2.4. `CuberConfig` のクリーンアップ (`src/pkg/cuber/types/config_types.go`)

`CuberConfig` 構造体から Embeddings 関連のフィールドを削除します（Phase-15以前のもの）。
- `EmbeddingsAPIKey`
- `EmbeddingsBaseURL`
- `EmbeddingsModel`

### 2.5. サービス初期化ロジックの変更 (`src/pkg/cuber/cuber.go`)

API情報を持たない状態となるため、起動時のEmbedder初期化を廃止します。

2.  バリデーションの削除:
    ```go
    // [削除]
    // if EMBEDDINGS_API_KEY == "" { ... }
    // if EMBEDDINGS_MODEL == "" { ... }
    ```

3.  設定オブジェクト作成時の削除:
    ```go
    flgs.CuberConfig = types.CuberConfig{
        // ...
        CompletionAPIKey: COMPLETION_API_KEY,
        CompletionModel:  COMPLETION_MODEL,
        // [削除] 以下のフィールド割り当てを削除
        // EmbeddingsAPIKey: EMBEDDINGS_API_KEY,
        // EmbeddingsModel:  EMBEDDINGS_MODEL,
        // ...
    }
    ```

### 2.6. 環境変数ファイルからの削除 (`src/.env`, `src/.env.sample`)

`src` ディレクトリ以下の `src/.env` および `src/.env.sample` (または `src/.env.example`) から、以下の行を削除します。

```env
# ベクトル化（Embeddings）に使用するLLMのベースURL
EMBEDDINGS_BASE_URL=...
# ベクトル化に使用するAPIキー
EMBEDDINGS_API_KEY=...
# ベクトル化に使用するモデル名
EMBEDDINGS_MODEL=...
```

### 2.7. サービス初期化ロジックの変更 (`src/pkg/cuber/cuber.go`)

API情報を持たない状態となるため、起動時のEmbedder初期化を廃止します。

**変更対象:**

1.  `CuberService` 構造体定義:
    ```go
    type CuberService struct {
        // ...
        // [削除] Embedder   storage.Embedder
        // ...
    }
    ```

2.  `NewCuberService` 関数:
    *   `3. Embeddings LLM の初期化` ブロック全体を削除。
    *   `embedder` 変数の生成処理を削除。
    *   `CuberService` 初期化時の `Embedder: embedder,` 行を削除。

**Embedder実装の代替策 (重要):**
削除された `Embedder` フィールドの代わりに、`CuberService` の各メソッド（`Absorb`, `Query`, `Memify`）内で、引数として渡される `types.EmbeddingModelConfig` (Phase-16で `ApiKey` を含むように拡張) を使用して、**都度 Embedder をインスタンス化**します。

例:
```go
func (s *CuberService) Absorb(ctx context.Context, ..., embeddingModelConfig types.EmbeddingModelConfig) ... {
    // 1. Storage Open (Decrypt済みApiKeyを含むConfigを使用)
    // 2. Embedder Init (オンデマンド生成)
    embConfig := providers.ProviderConfig{
        Type:      providers.ProviderOpenAI, // TODO: プロバイダー判定ロジック or Configから
        APIKey:    embeddingModelConfig.ApiKey, // 復号化済みのキー
        BaseURL:   "", // 必要な場合ModelConfigに追加
        ModelName: embeddingModelConfig.Model,
    }
    tempEmbedder, err := providers.NewEmbedder(ctx, embConfig) // EinoベースのEmbedder生成
    if err != nil { return ..., err }
    adapter := query.NewEinoEmbedderAdapter(tempEmbedder, embeddingModelConfig.Model)
    
    // 以降、s.Embedder の代わりに adapter を使用して処理
    // ...
}
```
これにより、Cubeごとに異なる埋め込みモデル/キーを使用するマルチテナント的な動作が可能になります。

## 3. REST API 実装の改修

`EmbeddingApiKey` の追加に伴い、パラメータ定義、バリデーション、ビジネスロジックを改修します。

### 3.1. パラメータ定義 (`src/mode/rt/rtparam/cubes_param.go`)

`CreateCubeParam` に `EmbeddingApiKey` フィールドを追加します。

```go
type CreateCubeParam struct {
    // ... 既存フィールド ...
    EmbeddingProvider  string `json:"embedding_provider" swaggertype:"string" example:"openai" binding:"required"`
    EmbeddingModel     string `json:"embedding_model" swaggertype:"string" example:"text-embedding-3-small" binding:"required"`
    EmbeddingDimension uint   `json:"embedding_dimension" swaggertype:"integer" example:"1536" binding:"required"`
    // [NEW] APIキーを追加
    EmbeddingApiKey string `json:"embedding_api_key" swaggertype:"string" binding:"required" example:"sk-proj-..."`
}
```

※ `ImportCubeParam` は `multipart/form-data` を使用しており、`rtreq` でフォームフィールドとして処理するため、ここでの構造体定義変更はありませんが、Swagger用定義があれば同様に追加してください。

### 3.2. バリデーションとテスト埋め込み (`src/mode/rt/rtreq/cubes_req.go`)

**重要:** ここが Phase-16 の要です。`CreateCubeReqBind` および `ImportCubeReqBind` にて、「入力されたAPIキーで実際に埋め込みができるか」を確認するロジック（Live Validation）を追加します。

#### 3.2.1. `CreateCubeReq` 修正

```go
type CreateCubeReq struct {
    // ...
    EmbeddingDimension uint   `json:"embedding_dimension" binding:"required,gte=1"`
    // [NEW]
    EmbeddingApiKey string `json:"embedding_api_key" binding:"required"`
}
```

#### 3.2.2. `CreateCubeReqBind` 実装例

```go
func CreateCubeReqBind(c *gin.Context, u *rtutil.RtUtil) (CreateCubeReq, rtres.CreateCubeRes, bool) {
    // ... BindJSON ...
    
    // 1. 基本的なバリデーション (validator.ValidateEmbeddingConfig) は既存のままでOK
    // ...

    // 2. [NEW] Live Test Embedding Validation
    // 入力された設定を使って一時的なEmbedderを作成し、"こんにちは" をベクトル化してみる
    ctx := context.Background()
    // プロバイダー設定の構築 (provider文字列からProviderTypeへの変換ロジックが必要)
    // providers.GetProviderType(req.EmbeddingProvider) のようなヘルパーが必要
    embConfig := providers.ProviderConfig{
        Type:      providers.ProviderOpenAI, // 仮: req.EmbeddingProvider に応じて設定
        APIKey:    req.EmbeddingApiKey,
        ModelName: req.EmbeddingModel,
    }
    
    // 一時Embedder生成
    tempEmbedder, err := providers.NewEmbedder(ctx, embConfig)
    if err != nil {
        res.Errors = append(res.Errors, rtres.Err{Field: "embedding_api_key", Message: fmt.Sprintf("Failed to initialize embedder with provided key: %s", err.Error())})
        return req, res, false
    }
    
    // テスト埋め込み実行
    vectors, err := tempEmbedder.Embed(ctx, []string{"こんにちは"})
    if err != nil {
        res.Errors = append(res.Errors, rtres.Err{Field: "embedding_api_key", Message: fmt.Sprintf("Test embedding generation failed: %s", err.Error())})
        return req, res, false
    }
    
    // 次元のチェック
    if len(vectors) == 0 || len(vectors[0]) != int(req.EmbeddingDimension) {
        res.Errors = append(res.Errors, rtres.Err{Field: "embedding_dimension", Message: fmt.Sprintf("Dimension mismatch: expected %d, got %d", req.EmbeddingDimension, len(vectors[0]))})
        return req, res, false
    }

    return req, res, ok
}
```

#### 3.2.3. `ImportCubeReq` 修正と `ImportCubeReqBind` 実装

`ImportCubeReq` に `EmbeddingApiKey` を追加し、`ImportCubeReqBind` でフォームフィールド `"embedding_api_key"` を取得・必須チェックしてください。
その後、`CreateCubeReqBind` と同様の `Live Test Embedding Validation` を行いたいところですが、**Import時は Zip 内の `embedding_config.json` にある Provider/Model/Dimension 情報が必要**です。
しかし `rtreq` 段階では Zip の解凍は行わない（BLで行う）のが通常のため、ここでは「APIキーが空でないこと」のチェックに留め、**厳密なLive Validationは `rtbl.ImportCube` の冒頭（Zip解凍後）で行う設計**とします。

**`rtreq` 修正:**
```go
type ImportCubeReq struct {
    // ...
    EmbeddingApiKey string `json:"embedding_api_key" binding:"required"` // BindQuery/PostForm で取得
}

func ImportCubeReqBind(...) {
    // ... c.PostForm("embedding_api_key") ...
}
```

### 3.3. Export の制御 (`src/mode/rt/rtbl/cubes_bl.go`)

`ExportCube` 処理において、zip ファイル内の `embedding_config.json` には **`EmbeddingApiKey` を絶対に含めないでください**。
API キーは各環境の所有者が管理すべき機密情報であり、Cube データ（Zip）と共に流出すべきではありません。

`ExportCube` 関数内で `embedding_config.json` を作成する際、APIキーが混入しないように注意します。

```go
// ExportCube 内
embConfig := types.EmbeddingModelConfig{
    Provider:  cube.EmbeddingProvider,
    Model:     cube.EmbeddingModel,
    Dimension: cube.EmbeddingDimension,
    // [重要] ApiKey フィールドには何もセットしない（または空文字を明示）
    ApiKey: "", 
}
```

### 3.4. Import の制御 (`src/mode/rt/rtbl/cubes_bl.go`)

`ImportCube` 関数を改修し、フォームから受け取った API キーと Zip 内の Config を組み合わせて保存、および Live Validation を実施します。

```go
// ImportCube 内
// 1. Zip 解凍 & embedding_config.json 読み込み (既存)
// ...

// 2. [NEW] Live Validation (ここで実施)
// Zipの Provider/Model と Formの ApiKey を組み合わせてテスト
zipEmbConfig := ... // Zipから読み取ったもの
inputApiKey := req.EmbeddingApiKey

ctx := context.Background()
testEmbConfig := providers.ProviderConfig{
    Type:      providers.GetProviderType(zipEmbConfig.Provider), // 文字列->Type変換必要
    APIKey:    inputApiKey,
    ModelName: zipEmbConfig.Model,
}
// ... NewEmbedder & Embed 実行 (CreateCubeReqBindと同様) ...
if err != nil {
    // Return BadRequest Error (API Keyが無効)
}

// 3. DB保存
// 暗号化して保存
encryptedKey, err := mycrypto.Encrypt(inputApiKey, os.Getenv("CUBER_CRYPTO_SECRET_KEY"))
// ...
newCube := model.Cube{
    // ...
    EmbeddingApiKey: encryptedKey, // 暗号化済みキー
    // ...
}
```

### 3.5. BL メソッドでの暗号化キー利用 (`Absorb`, `Query`, `Memify` etc.)

`src/mode/rt/rtbl/cubes_bl.go` の各メソッド（`CreateCube` 以外の `AbsorbCube`, `QueryCube` 等）で、DBから Cube を取得した後、`EmbeddingApiKey` を復号化して `CuberService` に渡します。

```go
// 例: AbsorbCube
cube, err := getCube(...)
// ...
// [NEW] 復号化
decryptedApiKey, err := mycrypto.Decrypt(cube.EmbeddingApiKey, os.Getenv("CUBER_CRYPTO_SECRET_KEY"))
if err != nil {
     return InternalServerErrorCustomMsg(c, res, "Failed to decrypt API key.")
}

// CuberService 呼び出し
usage, err := u.CuberService.Absorb(..., 
    types.EmbeddingModelConfig{
        Provider:  cube.EmbeddingProvider,
        Model:     cube.EmbeddingModel,
        Dimension: cube.EmbeddingDimension,
        ApiKey:    decryptedApiKey, // 復号化したキーをセット
    },
)
```

**CreateCube の場合:**
`CreateCube` では、リクエストから受け取った `req.EmbeddingApiKey`（平文）をそのまま `CuberService.CreateCubeDB` に渡して初期化を行い、DB保存時に `Encrypt` して `model.Cube` にセットします。

**ImportCube の場合:**
`ImportCube` では、フォームから受け取った平文キーで Live Validation をパスした後、DB保存時に `Encrypt` します。

## 4. サービス層の改修 (`CuberService`)

`src/pkg/cuber/cuber.go` の `CuberService` と各メソッドを、**完全にステートレスな Embedder 管理** に移行します。これに伴い、`Embedder` フィールドの削除と、メソッド呼び出しごとの一時的な Embedder インスタンス化を実装します。

### 4.1. 構造体定義と初期化の変更

`CuberService` 構造体から `Embedder` フィールドを削除し、`NewCuberService` での初期化処理も削除します（2.3節で指示済みですが再確認）。

```go
type CuberService struct {
    // ...
    // [削除] Embedder storage.Embedder
    // ...
}
```

### 4.2. メソッドシグネチャと実装の修正

全ての主要メソッド（`Absorb`, `Query`, `Memify`, および内部メソッド `add`, `cognify`, `memifyBatchProcess` など）で、引数として受け取る `types.EmbeddingModelConfig` を使用して Embedder を動的に生成します。

**共通ヘルパー関数の導入 (推奨):**
メソッド間でロジックが重複しないよう、Embedder 生成用のプライベートヘルパーメソッドを作成することを強く推奨します。

```go
// createTempEmbedder は指定された設定に基づいて一時的な Embedder と Adapter を生成します。
func (s *CuberService) createTempEmbedder(ctx context.Context, config types.EmbeddingModelConfig) (storage.Embedder, error) {
    // プロバイダータイプの解決 (文字列 -> providers.ProviderType)
    // 実装例: providers パッケージに IsValidProviderType などはあるが、文字列変換はないため、簡易的なキャストまたはスイッチが必要
    // 今回は単純にキャストして使用 (factory側でバリデーションされる)
    pType := providers.ProviderType(config.Provider)
    
    embConfig := providers.ProviderConfig{
        Type:      pType,
        APIKey:    config.ApiKey,
        ModelName: config.Model,
        // BaseURLをここに入れられる改修が必要
    }

    // Embedder (Eino interface) 生成
    einoEmb, err := providers.NewEmbedder(ctx, embConfig)
    if err != nil {
        return nil, fmt.Errorf("createTempEmbedder: failed to create raw embedder: %w", err)
    }

    // Adapter (storage.Embedder interface) 生成
    adapter := query.NewEinoEmbedderAdapter(einoEmb, config.Model)
    return adapter, nil
}
```

#### 4.2.1. `Absorb` / `add` / `cognify` の修正

`Embedder` を必要とする箇所全てで `createTempEmbedder` を呼び出し、メソッドスコープ内で使用します。

```go
// Absorb メソッド等
func (s *CuberService) Absorb(..., embeddingModelConfig types.EmbeddingModelConfig) ... {
    // 1. Storage Open
    st, err := s.GetOrOpenStorage(cubeDbFilePath, embeddingModelConfig)
    if err != nil { ... }

    // 2. Embedder 生成 (ここで生成!)
    embedder, err := s.createTempEmbedder(ctx, embeddingModelConfig)
    if err != nil {
        return totalUsage, fmt.Errorf("Absorb: Failed to create embedder: %w", err)
    }

    // 3. 内部メソッド呼び出し (Embedder を渡すようにシグネチャ変更が必要)
    // 旧: s.add(..., embeddingModelConfig)
    // 新: s.add(..., embedder, embeddingModelConfig)  <-- Embedder自体を渡すと効率的
    
    // ...
}

// 内部メソッド add, cognify も同様に、
// 「EmbeddingModelConfig から都度生成する」か「親から生成済みの Embedder を受け取る」ように変更してください。
// パフォーマンスの観点から、`Absorb` で一度生成したものを `add`, `cognify` に引数として渡す設計を推奨します。

func (s *CuberService) add(..., embedder storage.Embedder, ...) ... {
    // ... ingestion.NewIngestTask(..., embedder, ...) 
    // ※ IngestTask が Embedder を必要とする場合
}

func (s *CuberService) cognify(..., embedder storage.Embedder, ...) ... {
    // chunkingTask, summarizationTask など Embedder を使用するタスクの初期化に
    // s.Embedder ではなく、引数の embedder を使用する
    chunkingTask, err := chunking.NewChunkingTask(..., embedder, ...)
    // ...
    summarizationTask := summarization.NewSummarizationTask(..., embedder, ...)
    // ...
}
```

#### 4.2.2. `Query` の修正

```go
func (s *CuberService) Query(..., embeddingModelConfig types.EmbeddingModelConfig) ... {
    // 1. Storage Open
    st, err := s.GetOrOpenStorage(..., embeddingModelConfig)
    
    // 2. Embedder 生成
    embedder, err := s.createTempEmbedder(ctx, embeddingModelConfig) // Configから動的生成
    if err != nil { ... }

    // 3. SearchTool 初期化 (s.Embedder -> embedder に変更)
    searchTool := query.NewGraphCompletionTool(st.Vector, st.Graph, s.LLM, embedder, memoryGroup, s.Config.CompletionModel)
    
    // ...
}
```

#### 4.2.3. `Memify` の修正

`Memify` およびその下位メソッド (`executeMemifyCore`, `memifyBulkProcess` 等) も同様です。
最上位の `Memify` で Embedder を生成し、それを下位メソッドに伝播させてください。

```go
func (s *CuberService) Memify(..., embeddingModelConfig types.EmbeddingModelConfig) ... {
    // ...
    // Embedder 生成
    embedder, err := s.createTempEmbedder(ctx, embeddingModelConfig)
    if err != nil { ... }

    // Phase A: IgnoranceManager に embedder を渡す
    ignoranceManager := metacognition.NewIgnoranceManager(..., embedder, ...)

    // Phase B: executeMemifyCore に embedder を渡す
    // s.executeMemifyCore(..., embedder, ...)
}
```

**結論として、`CuberService` 内の `s.Embedder` への参照は一つ残らず排除し、全て引数から渡された `embedder` インスタンスまたは `createTempEmbedder` で生成したインスタンスに置き換えてください。**

## 5. ビジネスロジックの改修 (`rtbl`)

`src/mode/rt/rtbl/cubes_bl.go` における `EmbeddingApiKey` の取り扱い（暗号化/復号化）とバリデーションの実装詳細です。

### 5.1. 暗号化・復号化の共通処理

暗号化キーは `u.CryptoSecretKey` (RtUtilに追加したフィールド) から取得します。
`os.Getenv` をビジネスロジック内で直接呼ぶことは禁止とし、必ず `RtUtil` を経由してください。

### 5.2. `CreateCube` の改修

`CreateCube` では、リクエストの `EmbeddingApiKey` を暗号化して DB に保存します。

```go
func CreateCube(c *gin.Context, u *rtutil.RtUtil, ...) {
    // ...
    // 1. バリデーション (rtreq で完了済み)
    // 2. [NEW] 暗号化
    if u.CryptoSecretKey == "" {
        return InternalServerErrorCustomMsg(c, res, "Server crypto secret is not configured.")
    }
    encryptedApiKey, err := mycrypto.Encrypt(req.EmbeddingApiKey, u.CryptoSecretKey)
    if err != nil {
        return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to encrypt API key: %s", err.Error()))
    }

    // 3. Cube DB作成
    // Note: CreateCubeDB に渡す Config には、初期化時点では ApiKey は不要(スキーマ作成のみ)だが、
    // CuberService側の定義に合わせて渡す。
    err := cuber.CreateCubeDB(cubeDBFilePath, types.EmbeddingModelConfig{
        Provider:  req.EmbeddingProvider,
        Model:     req.EmbeddingModel,
        Dimension: req.EmbeddingDimension,
        ApiKey:    req.EmbeddingApiKey,
    })
    // ...

    // 4. Record Save
    newCube := model.Cube{
        // ...
        EmbeddingApiKey: encryptedApiKey, // [NEW] 保存
        // ...
    }
    // ...
}
```

### 5.3. `ImportCube` の改修

Zip解凍後、Live Validationを行い、暗号化して保存します。

```go
func ImportCube(c *gin.Context, u *rtutil.RtUtil, ...) {
    // ... Zip解凍 ...
    // embedding_config.json 読み込み -> zipEmbConfig

    // [NEW] Live Validation
    // CreateCubeReqBind 内のロジックと同様に、実際の Embedder を生成してテスト
    ctx := context.Background()
    // プロバイダータイプの解決 (文字列 -> providers.ProviderType)
    // 実装例: providers.GetProviderType(zipEmbConfig.Provider)
    pType := providers.GetProviderType(zipEmbConfig.Provider)

    // Temp Embedder Config
    tempConfig := providers.ProviderConfig{
        Type:      pType,
        APIKey:    req.EmbeddingApiKey, // フォームからの入力
        ModelName: zipEmbConfig.Model,
    }
    
    // Test Embedding (providers.NewEmbedder & Embed 実行)
    // 実装時は rtreq 等で使ったバリデーションロジックを再利用または同様に記述
    // 失敗したら BadRequest

    // [NEW] 暗号化
    if u.CryptoSecretKey == "" {
        return InternalServerErrorCustomMsg(c, res, "Server crypto secret is not configured.")
    }
    encryptedApiKey, err := mycrypto.Encrypt(req.EmbeddingApiKey, u.CryptoSecretKey)
    // ...

    // DB Save
    newCube := model.Cube{
        // ...
        EmbeddingProvider:  zipEmbConfig.Provider,
        EmbeddingModel:     zipEmbConfig.Model,
        EmbeddingDimension: zipEmbConfig.Dimension,
        EmbeddingApiKey:    encryptedApiKey, // [NEW]
        // ...
    }
    // ...
}
```

### 5.4. `AbsorbCube`, `QueryCube`, `MemifyCube` の改修

DBから取得した API Key を復号化し、`CuberService` に渡します。

```go
func AbsorbCube(c *gin.Context, u *rtutil.RtUtil, ...) {
    // ... Cube取得 ...

    // [NEW] 復号化
    if u.CryptoSecretKey == "" {
        return InternalServerErrorCustomMsg(c, res, "Server crypto secret is not configured.")
    }
    decryptedApiKey, err := mycrypto.Decrypt(cube.EmbeddingApiKey, u.CryptoSecretKey)
    if err != nil {
        return InternalServerErrorCustomMsg(c, res, "Failed to decrypt API key.")
    }

    // CuberService 呼び出し
    // Config に Decrypted Key をセット
    usage, err := u.CuberService.Absorb(c, cubeDbFilePath, req.MemoryGroup, files, 
        cognifyConfig,
        types.EmbeddingModelConfig{
            Provider:  cube.EmbeddingProvider,
            Model:     cube.EmbeddingModel,
            Dimension: cube.EmbeddingDimension,
            ApiKey:    decryptedApiKey, // [NEW] 復号化済みキー
        },
    )
    // ...
}
```
※ `QueryCube`, `MemifyCube` も同様に、`u.CuberService.Query(...)` / `u.CuberService.Memify(...)` 呼び出し前に復号化を行い、Config に詰めて渡してください。

---

**作業手順の注意**:
この指示書に基づき実装を行う際は、まず `types` の変更、次に `db_model` の変更、そして `CuberService` のリファクタリング、最後に `rtreq` / `rtbl` の実装という順序で進めるとスムーズです。
