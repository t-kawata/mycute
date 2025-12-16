# Phase-15 Development Directives

本フェーズの目的は、Cube 作成時における埋め込みモデル（Embeddings）設定の厳格化と、データ整合性の保証です。
ユーザーは Cube 作成時に「プロバイダー」「モデル名」「次元数」を明示的に指定し、システムはその組み合わせが妥当であるかを検証した上で、
MySQL および KuzuDB にその設定を固定（イミュータブル）なものとして登録します。

## コーディングルール (厳守)

開発にあたっては以下のルールを徹底的に遵守してください。

1.  **エラーメッセージの大文字開始**: `fmt.Errorf` や `errors.New` 等で生成するエラーメッセージの先頭は必ず**大文字**にしてください。
    *   NG: `fmt.Errorf("invalid dimension")`
    *   OK: `fmt.Errorf("Invalid dimension")`
2.  **無駄な改行の禁止**: 関数内の各ステートメント間に無駄な改行を入れないでください。可読性向上のための論理的なブロック分けには、空行ではなく「コメント (`//`)」を使用してください。空行は徹底的に排除してください。
3.  **バリデーションの責務**: バリデーションに関連する処理は `rtreq` に記述し、`rtbl` には検証済みのデータのみが渡るようにしてください。

---

## 1. データベースモデルの改修 (MySQL)

`src/model/db_model.go` の `Cube` 構造体に、以下のフィールドを追加してください。
これにより、各 Cube がどの埋め込みモデルを使用しているかを永続的に保持します。

```go
type Cube struct {
    // ... 既存フィールド ...
    EmbeddingProvider  string `gorm:"size:50;not null;default:''"`  // 例: "openai"
    EmbeddingModel     string `gorm:"size:100;not null;default:''"` // 例: "text-embedding-3-small"
    EmbeddingDimension uint    `gorm:"not null;default:0"`           // 例: 1536
}
```

## 2. REST API 実装の改修

REST API 層の各ファイルについて、以下の通り改修を行い、リクエストパラメータの受け渡しと処理を実装してください。

### 2.1. `src/mode/rt/rtparam/cubes_param.go` (Swagger定義)
Swagger ドキュメント生成用の構造体 `CreateCubeParam` に、新しい必須フィールドを追加します。

```go
type CreateCubeParam struct {
    Name               string `json:"name" swaggertype:"string" format:"" example:"My Cube"`
    Description        string `json:"description" swaggertype:"string" format:"" example:"Knowledge base for Go development"`
    // [NEW] 追加フィールド
    EmbeddingProvider  string `json:"embedding_provider" swaggertype:"string" example:"openai" binding:"required"`
    EmbeddingModel     string `json:"embedding_model" swaggertype:"string" example:"text-embedding-3-small" binding:"required"`
    EmbeddingDimension uint   `json:"embedding_dimension" swaggertype:"integer" example:"1536" binding:"required"`
} // @name CreateCubeParam
```

### 2.2. `src/mode/rt/rtreq/cubes_req.go` (リクエストバインディング & バリデーション)
実際に API リクエストを受け取る構造体 `CreateCubeReq` にも同様のフィールドを追加し、`ReqBind` 関数内でバリデーションを実行します。

```go
type CreateCubeReq struct {
    Name               string `json:"name" binding:"required,max=50"`
    Description        string `json:"description" binding:"max=255"`
    // [NEW] 追加フィールド
    EmbeddingProvider  string `json:"embedding_provider" binding:"required,max=50"`
    EmbeddingModel     string `json:"embedding_model" binding:"required,max=100"`
    EmbeddingDimension uint   `json:"embedding_dimension" binding:"required,gte=1"` // 0は不可
}

// CreateCubeReqBind 内で Validator を呼び出します
func CreateCubeReqBind(c *gin.Context, u *rtutil.RtUtil) (CreateCubeReq, rtres.CreateCubeRes, bool) {
	ok := true
	req := CreateCubeReq{}
	res := rtres.CreateCubeRes{Errors: []rtres.Err{}}
	if err := c.ShouldBindJSON(&req); err != nil {
		res.Errors = u.GetValidationErrs(err)
		ok = false
        return req, res, ok
	}
    // バリデーション実行 (Validatorパッケージ)
    // 検証エラーがある場合はレスポンスのエラーリストに追加して false を返す
    if err := validator.ValidateEmbeddingConfig(req.EmbeddingProvider, req.EmbeddingModel, req.EmbeddingDimension); err != nil {
        res.Errors = append(res.Errors, rtres.Err{Field: "embedding_config", Message: fmt.Sprintf("Invalid embedding configuration: %s", err.Error())})
        ok = false
    }
	return req, res, ok
}
```

### 2.3. `src/mode/rt/rthandler/hv1/cubes_handler.go` (ハンドラ)
`CreateCube` 関数の Swagger アノテーションを更新し、新しいパラメータの説明がドキュメントに反映されるようにします。

```go
// @Param json body CreateCubeParam true "json"
```

### 2.4. `src/mode/rt/request_mapper.go` (ルーティング)
変更不要ですが、リクエストフローの理解のために記載します。

### 2.5. `src/mode/rt/rtres/cubes_res.go` (レスポンス)
変更不要です。(Search/Get の変更は後述の第5章で扱います)

### 2.6. `src/mode/rt/rtbl/cubes_bl.go` (ビジネスロジック - CreateCube)
`rtreq` ですでにバリデーション済みのデータを受け取るため、ここではバリデーションを行わず、DB保存とKuzuDB作成に集中します。

**変更内容:**
1.  **KuzuDB作成**: `cuber.CreateCubeDB` を呼び出す際、`EmbeddingDimension` を引数として渡せるように呼び出し元を変更します。
2.  **MySQL保存**: `model.Cube` を作成する際、検証済みの `EmbeddingProvider`, `EmbeddingModel`, `EmbeddingDimension` をセットして保存します。

```go
func CreateCube(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.CreateCubeReq, res *rtres.CreateCubeRes) bool {
    // ... (UUID生成など既存ロジック) ...
    // KuzuDB 初期化 (Dimension を渡す)
    // エラーメッセージは大文字開始
    if err := cuber.CreateCubeDB(cubeDBFilePath, req.EmbeddingDimension); err != nil {
         return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to initialize cube: %s", err.Error()))
    }
    // ... (初期権限設定など) ...
    // DBレコード作成
    newCube := model.Cube{
        // ... (既存フィールド) ...
        EmbeddingProvider:  req.EmbeddingProvider,
        EmbeddingModel:     req.EmbeddingModel,
        EmbeddingDimension: req.EmbeddingDimension,
    }
    // ... (DB保存) ...
}
```

---

## 3. モデル設定検証ロジックの実装 (バリデータの作成)

`docs/embeddings_models_complete_accurate.csv` の内容に基づき、`src/pkg/cuber/validator/embedding_validator.go` を新規作成し、以下のコードを実装してください。

### 実装コード (src/pkg/cuber/validator/embedding_validator.go)

```go
package validator

import (
	"fmt"
	"strings"
)

// KnownModel は既知の埋め込みモデルの定義です。
type KnownModel struct {
	ProviderKeyword string // プロバイダー判定用キーワード (lowercase)
	ModelKeyword    string // モデル判定用キーワード (lowercase)
	Dimension       uint   // 固定次元数
}

// knownModels は docs/embeddings_models_complete_accurate.csv に基づく定義リストです。
var knownModels = []KnownModel{
	// OpenAI
	{ProviderKeyword: "openai", ModelKeyword: "text-embedding-3-small", Dimension: 1536},
	{ProviderKeyword: "openai", ModelKeyword: "text-embedding-3-large", Dimension: 3072},
	{ProviderKeyword: "openai", ModelKeyword: "text-embedding-ada-002", Dimension: 1536},
	// Mistral
	{ProviderKeyword: "mistral", ModelKeyword: "mistral-embed", Dimension: 1024},
	// Gemini (AI Studio / Vertex AI)
	{ProviderKeyword: "gemini", ModelKeyword: "gemini-embedding-001", Dimension: 3072}, // "AI Studio" or "Vertex AI" keys handled by "gemini"
	{ProviderKeyword: "gemini", ModelKeyword: "text-embedding-004", Dimension: 768},
	// DeepSeek
	{ProviderKeyword: "deepseek", ModelKeyword: "deepseek-embedding-v2", Dimension: 768},
	// Qwen
	{ProviderKeyword: "qwen", ModelKeyword: "qwen3-embedding-0.6b", Dimension: 1024},
	{ProviderKeyword: "qwen", ModelKeyword: "qwen3-embedding-4b", Dimension: 2560},
	{ProviderKeyword: "qwen", ModelKeyword: "qwen3-embedding-8b", Dimension: 4096},
}

// ValidateEmbeddingConfig は入力されたプロバイダー、モデル、次元数の妥当性を検証します。
func ValidateEmbeddingConfig(provider, model string, dimension uint) error {
	providerLower := strings.ToLower(provider)
	modelLower := strings.ToLower(model)
	// 明示的にサポートされていないプロバイダー/モデルのチェック
	if (strings.Contains(providerLower, "anthropic") && strings.Contains(modelLower, "claude")) ||
		(strings.Contains(providerLower, "xai") && strings.Contains(modelLower, "grok")) {
		return fmt.Errorf("Provider '%s' with model '%s' does not support embeddings (Chat only)", provider, model)
	}
	for _, km := range knownModels {
		// プロバイダーとモデル名の両方が含まれているか ("推定")
		if strings.Contains(providerLower, km.ProviderKeyword) && strings.Contains(modelLower, km.ModelKeyword) {
			// 固定次元モデルの場合 check exact match
			if dimension != km.Dimension {
				return fmt.Errorf("Invalid dimension %d for fixed model '%s' (Provider: %s). Expected: %d",
					dimension, model, provider, km.Dimension)
			}
			return nil // Valid (Matched and dimension correct)
		}
	}
	// リストにマッチしない場合は、未知のモデルとして許容する (将来のモデルやマイナーなモデルのため)
	return nil
}
```

## 4. KuzuDB スキーマの動的化

埋め込みモデルの次元数（Duration）を動的に設定できるように、KuzuDBの初期化フローを改修します。

### 4.1. 設定構造体の定義 (`src/pkg/cuber/types/types.go`)
`cuber` パッケージ全体で利用可能な型定義として、以下を追加してください。
既存のファイルがない場合は作成してください。

```go
package types

// EmbeddingModelConfig は埋め込みモデルの設定を保持します。
type EmbeddingModelConfig struct {
	Provider  string `json:"provider"`
	Model     string `json:"model"`
	Dimension uint   `json:"dimension"`
}
```

### 4.2. `EnsureSchema` の改修 (`src/pkg/cuber/db/kuzudb/kuzudb_storage.go`)
`EnsureSchema` メソッドのシグネチャを変更し、`EmbeddingModelConfig` を受け取るようにします。
そして、Cypherクエリ（CREATE TABLE文）内の `FLOAT[1536]` を動的に生成します。

```go
// EnsureSchema は必要なテーブルスキーマを作成します。
// config.Dimension を使用して、ベクトルカラムの次元数を動的に設定します。
func (s *KuzuDBStorage) EnsureSchema(ctx context.Context, config types.EmbeddingModelConfig) error {
	log.Println("[KuzuDB] EnsureSchema: Starting schema creation...")

	// ベクトル型の定義文字列を生成 (例: "FLOAT[1536]" or "FLOAT[768]")
	vectorType := fmt.Sprintf("FLOAT[%d]", config.Dimension)

	// 1. Node Tables
	nodeTables := []string{
		// Data: ファイルメタデータ (ベクトルなし)
		`CREATE NODE TABLE Data (
			id STRING,
			memory_group STRING,
			name STRING,
			raw_data_location STRING,
			original_data_location STRING,
			extension STRING,
			mime_type STRING,
			content_hash STRING,
			owner_id STRING,
			created_at TIMESTAMP,
			PRIMARY KEY (id)
		)`,
		// Document: ドキュメント (ベクトルなし)
		`CREATE NODE TABLE Document (
			id STRING,
			memory_group STRING,
			data_id STRING,
			text STRING,
			metadata STRING,
			PRIMARY KEY (id)
		)`,
		// Chunk: チャンクとEmbedding (動的次元)
		fmt.Sprintf(`CREATE NODE TABLE Chunk (
			id STRING,
			memory_group STRING,
			document_id STRING,
			text STRING,
			token_count INT64,
			chunk_index INT64,
			embedding %s,
			PRIMARY KEY (id)
		)`, vectorType),
		// GraphNode: 知識グラフのノード (ベクトルなし)
		`CREATE NODE TABLE GraphNode (
			id STRING,
			memory_group STRING,
			type STRING,
			properties STRING,
			PRIMARY KEY (id)
		)`,
		// Entity: エンティティ (動的次元)
		fmt.Sprintf(`CREATE NODE TABLE Entity (
			id STRING,
			memory_group STRING,
			text STRING,
			embedding %s,
			PRIMARY KEY (id)
		)`, vectorType),
		// Summary: 要約 (動的次元)
		fmt.Sprintf(`CREATE NODE TABLE Summary (
			id STRING,
			memory_group STRING,
			text STRING,
			embedding %s,
			PRIMARY KEY (id)
		)`, vectorType),
		// Rule: ルール (動的次元)
		fmt.Sprintf(`CREATE NODE TABLE Rule (
			id STRING,
			memory_group STRING,
			text STRING,
			embedding %s,
			PRIMARY KEY (id)
		)`, vectorType),
		// Unknown: 知らないこと (動的次元)
		fmt.Sprintf(`CREATE NODE TABLE Unknown (
			id STRING,
			memory_group STRING,
			text STRING,
			embedding %s,
			PRIMARY KEY (id)
		)`, vectorType),
		// Capability: できること (動的次元)
		fmt.Sprintf(`CREATE NODE TABLE Capability (
			id STRING,
			memory_group STRING,
			text STRING,
			embedding %s,
			PRIMARY KEY (id)
		)`, vectorType),
	}

	for _, query := range nodeTables {
		if err := s.createTable(query); err != nil {
			return err
		}
	}

	// 2. Rel Tables (Relationships)
	// (リレーション定義に変更はないため、既存コードを維持)
	// ... (省略せずに記述する実装時は既存コードをそのまま利用) ...
    relTables := []string{
		// Data -> Document
		`CREATE REL TABLE HAS_DOCUMENT (
			FROM Data TO Document,
			memory_group STRING
		)`,
		// Document -> Chunk
		`CREATE REL TABLE HAS_CHUNK (
			FROM Document TO Chunk,
			memory_group STRING
		)`,
		// Chunk -> Chunk (Sequence)
		`CREATE REL TABLE NEXT_CHUNK (
			FROM Chunk TO Chunk,
			memory_group STRING
		)`,
		// GraphNode -> GraphNode (Knowledge Graph Edges)
		`CREATE REL TABLE GraphEdge (
			FROM GraphNode TO GraphNode,
			memory_group STRING,
			type STRING,
			properties STRING,
			weight DOUBLE,
			confidence DOUBLE
		)`,
	}

	for _, query := range relTables {
		if err := s.createTable(query); err != nil {
			return err
		}
	}

	log.Println("[KuzuDB] EnsureSchema: Schema creation completed.")
	return nil
}
```

### 4.3. `CreateCubeDB` の改修 (`src/pkg/cuber/cuber.go`)
`CreateCubeDB` 関数が `EmbeddingDimension` を受け取り、`EnsureSchema` に渡すように変更します。
第2章の `cubes_bl.go` からの呼び出し変更と対になります。

```go
// CreateCubeDB は、新しい空の Cube データベースを初期化します。
// 指定された次元数でスキーマを作成します。
func CreateCubeDB(dbFilePath string, dimension uint) error {
	// 親ディレクトリの作成
	parentDir := filepath.Dir(dbFilePath)
	if err := os.MkdirAll(parentDir, 0755); err != nil {
		return fmt.Errorf("CreateCubeDB: failed to create parent directory: %w", err)
	}
	// KuzuDB を初期化
	kuzuSt, err := kuzudb.NewKuzuDBStorage(dbFilePath)
	if err != nil {
		return fmt.Errorf("CreateCubeDB: failed to create KuzuDBStorage: %w", err)
	}
	defer kuzuSt.Close()
    
    // Config作成 (初期化時は Dimension だけが必要)
    config := types.EmbeddingModelConfig{
        Dimension: dimension,
    }

	// スキーマ適用 (Dimensionを渡す)
	if err := kuzuSt.EnsureSchema(context.Background(), config); err != nil {
		return fmt.Errorf("CreateCubeDB: failed to apply schema: %w", err)
	}
	log.Printf("[Cuber] Created new Cube at %s with dimension %d", dbFilePath, dimension)
	return nil
}
```

### 4.4. 既存コードへの影響修正 (`src/pkg/cuber/cuber.go` - `GetOrOpenStorage`)
`GetOrOpenStorage` 内での `EnsureSchema` 呼び出しも修正が必要です。既存の DB を開く場合、スキーマは既に存在するため通常 `EnsureSchema` はスキップされますが（createTable内のexistsチェックによる）、シグネチャ変更に伴い引数を渡す必要があります。
既存のDBを開く場合、次元数はDBから読み取るのが理想ですが、ここでは簡易的に `0` またはデフォルト値を渡しておき、`EnsureSchema` 内の `createTable` が "exists" エラーでスルーすることで整合性を保ちます。

```go
// GetOrOpenStorage 内の該当箇所
	// Ensure schema (lazy init)
    // 既存DBの場合は次元数は無視されるため、ダミーのConfigを渡す
    // (createTableが既に存在する場合は何もしないため)
	if err := kuzuSt.EnsureSchema(context.Background(), types.EmbeddingModelConfig{Dimension: 0}); err != nil {
		kuzuSt.Close()
		return nil, fmt.Errorf("Failed to ensure schema: %w", err)
	}
```
※ 注意: 本来であれば既存のテーブル定義を確認すべきですが、Phase-15では「新規作成時の固定化」に焦点を当てるため、既存DBオープン時のEnsureSchemaは「何もしない（既存テーブル維持）」動作となることを前提とします。

## 5. Cubeのエクスポート/インポート/検索における厳格な制御

モデル設定の一貫性を保つため、`rtbl` を中心に厳密な実装を行います。

### 5.1. 定数定義の追加 (`src/mode/rt/rtbl/cubes_bl.go`)
定数ブロックに以下を追加してください。
```go
const (
    // ... 既存の定数 ...
    EMBEDDING_CONFIG_JSON = "embedding_config.json"
)
```

### 5.2. Export処理の改修 (`src/mode/rt/rtbl/cubes_bl.go` - `ExportCube`)
Cube をエクスポートする際、現在の埋め込みモデル設定を `embedding_config.json` として zip 内に含めます。

**実装指示:**
`ExportCube` 関数内で、`extraFiles` マップを作成している箇所を修正します。

```go
// ... (前略) ...

    // Embedding Config の作成
    // 保存されている設定をそのままJSONにする
    embConfig := types.EmbeddingModelConfig{
        Provider:  cube.EmbeddingProvider,
        Model:     cube.EmbeddingModel,
        Dimension: cube.EmbeddingDimension,
    }
    embConfigJSON, err := common.ToJson(embConfig)
    if err != nil {
        InternalServerErrorCustomMsg(c, res, "Failed to serialize embedding config.")
        return nil, "", false
    }

    // Zip作成
    cubeDbFilePath, err := u.GetCubeDBFilePath(&cube.UUID, ids.ApxID, ids.VdrID, ids.UsrID)
    if err != nil {
        InternalServerErrorCustomMsg(c, res, "Failed to get cube path")
        return nil, "", false
    }
    extraFiles := map[string][]byte{
        METADATA_JSON:           []byte(lineageJSON),
        STATS_USAGE_JSON:        []byte(statsUsageJSON),
        STATS_CONTRIBUTORS_JSON: []byte(statsContribJSON),
        EMBEDDING_CONFIG_JSON:   []byte(embConfigJSON), // [NEW] 追加
    }
// ... (後略) ...
```
※ `types.EmbeddingModelConfig` は `src/pkg/cuber/types` パッケージに定義するか、既存の類似構造体を利用してください（`kuzudb`パッケージ内の定義と重複しないよう注意。`types`への定義推奨）。

### 5.3. Import処理の改修 (`src/mode/rt/rtbl/cubes_bl.go` - `ImportCube`)
Cube をインポートする際、暗号化Zip内にある `embedding_config.json` を読み取り、**必ずその設定値を新しいCubeレコードに適用**します。ユーザー入力などで上書きすることはできません。

**実装指示:**
1. `innerZipReader` からファイルを抽出するループで、`EMBEDDING_CONFIG_JSON` を読み取ります。
2. 読み取った設定がない場合はエラーとします。
3. `model.Cube` 作成時に、読み取った設定をセットします。

```go
// ... (前略: zip解凍ループ) ...
    // Extract Embedding Config
    embConfigBytes, _ := func() ([]byte, error) {
        for _, zf := range innerZipReader.File {
            if zf.Name == EMBEDDING_CONFIG_JSON {
                rc, err := zf.Open()
                if err != nil { return nil, err }
                defer rc.Close()
                return io.ReadAll(rc)
            }
        }
        return nil, nil
    }()
    if embConfigBytes == nil {
        // 必須ファイル欠落
        return InternalServerErrorCustomMsg(c, res, "Missing embedding_config.json in export file.")
    }
    var importedEmbConfig types.EmbeddingModelConfig
    if err := json.Unmarshal(embConfigBytes, &importedEmbConfig); err != nil {
        return InternalServerErrorCustomMsg(c, res, fmt.Sprintf("Failed to parse embedding config: %s", err.Error()))
    }
    
    // ... (Lineage, Stats 読み込み) ...

    // 9. Transaction: Cube作成
    // ...
    txErr := u.DB.Transaction(func(tx *gorm.DB) error {
        newCube = model.Cube{
            UUID:           newUUID,
            UsrID:          *ids.UsrID,
            Name:           req.Name,
            Description:    req.Description,
            // [NEW] インポートされた埋め込み設定を強制適用
            EmbeddingProvider:  importedEmbConfig.Provider,
            EmbeddingModel:     importedEmbConfig.Model,
            EmbeddingDimension: importedEmbConfig.Dimension,
            
            ExpireAt:       payload.ExpireAt,
            // ... (以下既存) ...
        }
        // ...
// ... (後略) ...
```

### 5.4. Search/Get処理の改修 (`SearchCubes`, `GetCube`)
クライアントが埋め込みモデルを確認できるよう、レスポンスにフィールドを追加し、値をセットします。

**1. レスポンス構造体の変更 (`src/mode/rt/rtres/cubes_res.go`)**
`SearchCubesResCube` および `GetCubeResCube` にフィールドを追加します。

```go
type SearchCubesResCube struct {
    // ... 既存フィールド ...
    EmbeddingProvider  string `json:"embedding_provider"`
    EmbeddingModel     string `json:"embedding_model"`
    EmbeddingDimension uint   `json:"embedding_dimension"`
}
// GetCubeResCube も同様に追加
```

**2. 値のマッピング (`src/mode/rt/rtbl/cubes_bl.go`)**
`SearchCubes` のループ内および `GetCube` 内で、`model.Cube` から値をコピーします。

```go
// SearchCubes 内
results = append(results, rtres.SearchCubesResData{
    Cube: rtres.SearchCubesResCube{
        // ... 既存 ...
        EmbeddingProvider:  cube.EmbeddingProvider,
        EmbeddingModel:     cube.EmbeddingModel,
        EmbeddingDimension: cube.EmbeddingDimension,
    },
    // ...
})

// GetCube 内
data := rtres.GetCubeResData{
    Cube: rtres.GetCubeResCube{
        // ... 既存 ...
        EmbeddingProvider:  cube.EmbeddingProvider,
        EmbeddingModel:     cube.EmbeddingModel,
        EmbeddingDimension: cube.EmbeddingDimension,
    },
    // ...
}
```

---
**注意**: 実装作業には入らず、この指示書の作成完了をもって報告してください。
