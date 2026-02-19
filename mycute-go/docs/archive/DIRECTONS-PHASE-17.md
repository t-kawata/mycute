# Phase-17 Development Directives

本フェーズの目的は、Chat Completion (テキスト生成) モデルの設定と認証情報をグローバル設定 (`CuberConfig` / 環境変数) から完全撤廃し、動的に管理可能な `ChatModel` リソースへと移行することです。
Embeddingモデルとは異なり、ChatModelはCubeに1対1で紐づくものではなく、独立したリソースとして管理され、Cubeの操作時 (`Absorb`, `Query`, `Memify`) に都度選択して使用します。

## コーディングルール (厳守)

1.  **エラーメッセージの大文字開始**: `fmt.Errorf("Invalid API key")` のように先頭を大文字にする。
2.  **無駄な改行の禁止**: 関数内のステートメント間に空行を入れない。コメントで論理ブロックを分ける。
3.  **バリデーションの責務**: `rtreq` で値の検証（**Live Test** 含む）を完結させる。
4.  **厳格なパーティショニング**: 全てのDB操作において `ApxID` と `VdrID` を条件に含める。
5.  **REST API 構造の遵守**: `docs/REST_API_STRUCTURE_AND_RULES.md` に従い、`rthandler` -> `rtreq` -> `rtbl` の責務分担を守る。

---

## 1. データベースモデルの拡張 (MySQL)

`src/model/db_model.go` に新しいモデル `ChatModel` を追加します。

```go
// ChatModel は、テキスト生成に使用するLLMの設定を保持します。
// Cubeとは独立して管理され、実行時にID指定で利用されます。
type ChatModel struct {
	ID          uint `gorm:"primarykey;index:chat_model_apxid_vdrid_id_idx"`
	Name        string `gorm:"size:50;not null;default:''"`  // 表示名 (例: "My GPT-4")
	Provider    string `gorm:"size:50;not null;default:''"`  // 例: "openai", "anthropic"
	Model       string `gorm:"size:100;not null;default:''"` // 例: "gpt-4o", "claude-3-opus-20240229"
	BaseURL     string `gorm:"size:255;not null;default:''"` // オプション (例: Ollama用URL)
	ApiKey      string `gorm:"size:1024;not null;default:''" json:"-"` // 暗号化して保存
	MaxTokens   int    `gorm:"not null;default:0"`           // 最大生成トークン数 (0=デフォルト)
    Temperature float64 `gorm:"not null;default:0.2"`        // 生成温度
    ApxID       uint `gorm:"index:chat_model_apxid_vdrid_id_idx"`
	VdrID       uint `gorm:"index:chat_model_apxid_vdrid_id_idx"`
    CreatedAt          time.Time
	UpdatedAt          time.Time
	DeletedAt          gorm.DeletedAt `gorm:"index"`
}

func (ChatModel) TableName() string {
	return "chat_models"
}
```

**暗号化ルール:**
EmbeddingApiKey と同様に、`ApiKey` フィールドは `mycrypto.Encrypt` で暗号化して保存し、使用直前に `Decrypt` してください。カラムサイズは `1024` を確保します。

## 2. Configのクリーンアップ

`src/pkg/cuber/types/config_types.go` の `CuberConfig` から、以下の Chat Completion 関連フィールドを**完全削除**してください。

- `CompletionAPIKey`
- `CompletionBaseURL`
- `CompletionModel`
- `CompletionMaxTokens`

これに伴い、`src/pkg/cuber/cuber.go` の `NewCuberService` にあるグローバルな LLM (`s.LLM`) 初期化ロジックも削除します。
`CuberService` 構造体から `LLM` (`langchain.ChatModel` interface 等) フィールドを削除し、各メソッド実行時に都度 LLM を生成する設計に変更します。

**環境変数の掃除:**
`src/mode/rt/main_of_rt.go`, `src/.env`, `src/.env.sample` から、以下の変数を削除してください。
- `COMPLETION_API_KEY`
- `COMPLETION_BASE_URL`
- `COMPLETION_MODEL

埋め込みモデルの設定をグローバルから削除するための具体的な手順です。

### 2.1. 設定定義の削除 (`src/pkg/cuber/types/config_types.go`)

`CuberConfig` 構造体から以下のフィールドを削除します。

```go
type CuberConfig struct {
    // ...
    // [削除ターゲット] 以下のブロックを完全に削除してください
    // Completion (テキスト生成) LLM の設定
    // CompletionAPIKey    string
    // CompletionBaseURL   string
    // CompletionModel     string
    // CompletionMaxTokens int

    // ...
}
```

### 2.2. 環境変数読み込み処理の削除 (`src/mode/rt/main_of_rt.go`)

`MainOfRT` 関数内の環境変数読み込みとバリデーション、設定への代入処理を削除します。

**変更対象:**

1.  変数定義の削除:
    ```go
    // [削除]
    // COMPLETION_API_KEY := os.Getenv("COMPLETION_API_KEY")
    // COMPLETION_BASE_URL := os.Getenv("COMPLETION_BASE_URL")
    // COMPLETION_MODEL := os.Getenv("COMPLETION_MODEL")
    ```

2.  バリデーションの削除:
    ```go
    // [削除]
    // if COMPLETION_API_KEY == "" { ... }
    // if COMPLETION_MODEL == "" { ... }
    ```

3.  設定オブジェクト作成時の削除:
    ```go
    flgs.CuberConfig = types.CuberConfig{
        // ...
        // [削除] 以下のフィールド割り当てを削除
        // CompletionAPIKey: COMPLETION_API_KEY,
        // CompletionBaseURL: COMPLETION_BASE_URL,
        // CompletionModel:  COMPLETION_MODEL,
        // ...
    }
    ```

### 2.3. 環境変数ファイルからの削除 (`src/.env`, `src/.env.sample`)

`src` ディレクトリ以下の `src/.env` および `src/.env.sample` (または `src/.env.example`) から、以下の行を削除します。

```env
# テキスト生成（Completion）に使用するLLMのベースURL
COMPLETION_BASE_URL=...
# テキスト生成に使用するAPIキー
COMPLETION_API_KEY=...
# テキスト生成に使用するモデル名
COMPLETION_MODEL=...
```

### 2.4. サービス初期化ロジックの変更 (`src/pkg/cuber/cuber.go`)

API情報を持たない状態となるため、起動時のLLM初期化を廃止します。

**変更対象:**

1.  `CuberService` 構造体定義:
    ```go
    type CuberService struct {
        // ...
        // [削除] LLM        llm.ChatModel
        // ...
    }
    ```

2.  `NewCuberService` 関数:
    *   `2. Completion (Text Generation) LLM の初期化` ブロック全体を削除。
    *   `llm` 変数の生成処理を削除。
    *   `CuberService` 初期化時の `LLM:        llm,` 行を削除。

**LLM実装の代替策 (重要):**
削除された `LLM` フィールドの代わりに、`CuberService` の各メソッド（`Absorb`, `Query`, `Memify`）内で、引数として渡される `types.ChatModelConfig` (Phase-17で動的生成) を使用して、**都度 LLM をインスタンス化**します。

コーディングの一貫性を保つため、Embeddingにおける `createTempEmbedder` と同様に、プライベートヘルパーメソッド `createTempChatModel` を必ず介してインスタンス化を行ってください。メソッド内で直接ファクトリを呼び出すことは禁止します。

例:
```go
func (s *CuberService) Absorb(ctx context.Context, ..., chatModelConfig types.ChatModelConfig) ... {
    // 1. Storage Open
    // 2. ChatModel Init (オンデマンド生成 & 抽象化)
    chatModel, err := s.createTempChatModel(ctx, chatModelConfig)
    if err != nil {
         return ..., fmt.Errorf("Absorb: Failed to create chat model: %w", err)
    }
    
    // 以降、s.LLM の代わりに chatModel を使用してタスク生成
    // ...
}
```
これにより、Cubeやリクエストごとに異なるモデル/キーを使用するマルチテナント的な動作が可能になります。

## 3. REST API 実装 (ChatModel CRUD)

`ChatModel` に対する CRUD API を実装します。
`docs/REST_API_STRUCTURE_AND_RULES.md` の命名規則と実装パターンに完全に従ってください。

**エンドポイント一覧:**
- `POST /v1/chat_models/search`: 一覧検索
- `POST /v1/chat_models/get`: ID指定取得
- `POST /v1/chat_models/create`: 新規作成 (**Live Test** 必須)
- `POST /v1/chat_models/update`: 更新 (**Live Test** 必須)
- `POST /v1/chat_models/delete`: 削除

### 3.1. Live Test (Create/Update)

ユーザーが誤った設定（無効なAPIキーやモデル名）を登録しないよう、作成・更新リクエストのバリデーション時に**実際にLLMを呼び出して疎通確認（Live Test）**を行ってください。

**実装場所:**
`rtreq.CreateChatModelReqBind` および `UpdateChatModelReqBind` 内。
または、Phase-16 で行ったように `CuberService` に `VerifyChatModelConfiguration` メソッドを追加し、それを呼び出す形が望ましいです。

## 3. REST API 実装 (ChatModel CRUD)

`ChatModel` リソースに対するCRUD操作を実装します。
`docs/REST_API_STRUCTURE_AND_RULES.md` の「8. 厳格なコーディング規則」に基づき、以下の順序・ルールで実装してください。

**実装順序:**
1. Search
2. Get
3. Create
4. Update
5. Delete

### 3.2. パラメータ定義 (`src/mode/rt/rtparam/chat_models_param.go`)

JSONボディ用の構造体定義です。Swagger用のアノテーションも含みます。

```go
package rtparam

// 1. Search Param
type SearchChatModelsParam struct {
    Name        string  `json:"name" swaggertype:"string" example:"My GPT-4" binding:"required"`
	Provider    string  `json:"provider" swaggertype:"string" example:"openai" binding:"required"`
	Model       string  `json:"model" swaggertype:"string" example:"gpt-4o" binding:"required"`
	BaseURL     string  `json:"base_url" swaggertype:"string" example:"https://api.openai.com/v1"`
} // @name SearchChatModelsParam

// 2. Get Param
// IDはPath Parameterで取得するため、ここには定義しません (GETリクエストはBodyを持たない)。

// 3. Create Param
type CreateChatModelParam struct {
	Name        string  `json:"name" swaggertype:"string" example:"My GPT-4" binding:"required"`
	Provider    string  `json:"provider" swaggertype:"string" example:"openai" binding:"required"`
	Model       string  `json:"model" swaggertype:"string" example:"gpt-4o" binding:"required"`
	BaseURL     string  `json:"base_url" swaggertype:"string" example:"https://api.openai.com/v1"`
	ApiKey      string  `json:"api_key" swaggertype:"string" example:"sk-proj-..." binding:"required"`
	MaxTokens   int     `json:"max_tokens" swaggertype:"integer" example:"4096"`
	Temperature float64 `json:"temperature" swaggertype:"number" example:"0.2"`
} // @name CreateChatModelParam

// 4. Update Param
// IDはPath Parameterで取得するため含めません。
type UpdateChatModelParam struct {
	Name        string  `json:"name" swaggertype:"string" example:"My GPT-4"`
	Provider    string  `json:"provider" swaggertype:"string" example:"openai"`
	Model       string  `json:"model" swaggertype:"string" example:"gpt-4o"`
	BaseURL     string  `json:"base_url" swaggertype:"string" example:"https://api.openai.com/v1"`
	ApiKey      string  `json:"api_key" swaggertype:"string" example:"sk-proj-..."`
	MaxTokens   int     `json:"max_tokens" swaggertype:"integer" example:"4096"`
	Temperature *float64 `json:"temperature" swaggertype:"number" example:"0.2"`
} // @name UpdateChatModelParam

// 5. Delete Param
// IDはPath Parameterで取得するため、ここには定義しません (DELETEリクエストはBodyを持たない)。
```

### 3.3. レスポンス定義 (`src/mode/rt/rtres/chat_models_res.go`)

レスポンス構造体とDataへの変換ロジック (`Of` メソッド) です。
Search/Get/Create/Update/Delete それぞれに対し、専用の型定義を行います。共通化は禁止です。

```go
package rtres

import (
	"mycute/src/lib/common"
	"mycute/src/model"
)

// --- 1. Search ---

type SearchChatModelsResData struct {
	ID          uint    `json:"id"`
	Name        string  `json:"name"`
	Provider    string  `json:"provider"`
	Model       string  `json:"model"`
	BaseURL     string  `json:"base_url"`
	MaxTokens   int     `json:"max_tokens"`
	Temperature float64 `json:"temperature"`
	CreatedAt   string  `json:"created_at"`
	UpdatedAt   string  `json:"updated_at"`
} // @name SearchChatModelsResData

// Of method for Search (Slice -> Slice Pointer)
func (d *SearchChatModelsResData) Of(ms *[]model.ChatModel) *[]SearchChatModelsResData {
	data := []SearchChatModelsResData{}
	for _, m := range *ms {
		data = append(data, SearchChatModelsResData{
			ID:          m.ID,
			Name:        m.Name,
			Provider:    m.Provider,
			Model:       m.Model,
			BaseURL:     m.BaseURL,
			MaxTokens:   m.MaxTokens,
			Temperature: m.Temperature,
			CreatedAt:   common.ParseDatetimeToStr(&m.CreatedAt),
			UpdatedAt:   common.ParseDatetimeToStr(&m.UpdatedAt),
		})
	}
	return &data
}

type SearchChatModelsRes struct {
	Data   []SearchChatModelsResData `json:"data"`
	Errors []Err                     `json:"errors"`
} // @name SearchChatModelsRes


// --- 2. Get ---

type GetChatModelResData struct {
	ID          uint    `json:"id"`
	Name        string  `json:"name"`
	Provider    string  `json:"provider"`
	Model       string  `json:"model"`
	BaseURL     string  `json:"base_url"`
	MaxTokens   int     `json:"max_tokens"`
	Temperature float64 `json:"temperature"`
	CreatedAt   string  `json:"created_at"`
	UpdatedAt   string  `json:"updated_at"`
} // @name GetChatModelResData

// Of method for Get (Model -> Data Pointer)
func (d *GetChatModelResData) Of(m *model.ChatModel) *GetChatModelResData {
	data := GetChatModelResData{
		ID:          m.ID,
		Name:        m.Name,
		Provider:    m.Provider,
		Model:       m.Model,
		BaseURL:     m.BaseURL,
		MaxTokens:   m.MaxTokens,
		Temperature: m.Temperature,
		CreatedAt:   common.ParseDatetimeToStr(&m.CreatedAt),
		UpdatedAt:   common.ParseDatetimeToStr(&m.UpdatedAt),
	}
	return &data
}

type GetChatModelRes struct {
	Data   GetChatModelResData `json:"data"`
	Errors []Err               `json:"errors"`
} // @name GetChatModelRes


// --- 3. Create ---

type CreateChatModelResData struct {
	ID          uint    `json:"id"`
} // @name CreateChatModelResData

type CreateChatModelRes struct {
	Data   CreateChatModelResData `json:"data"`
	Errors []Err                  `json:"errors"`
} // @name CreateChatModelRes

// --- 4. Update ---

type UpdateChatModelResData struct {
} // @name UpdateChatModelResData

type UpdateChatModelRes struct {
	Data   UpdateChatModelResData `json:"data"`
	Errors []Err                  `json:"errors"`
} // @name UpdateChatModelRes

// --- 5. Delete ---

type DeleteChatModelResData struct {
} // @name DeleteChatModelResData

type DeleteChatModelRes struct {
	Data   DeleteChatModelResData `json:"data"`
	Errors []Err                  `json:"errors"`
} // @name DeleteChatModelRes
```

### 3.4. リクエスト処理 (`src/mode/rt/rtreq/chat_models_req.go`)

Request構造体の定義と、Bind処理の実装です。
CreateとUpdateでは Live Test (`VerifyChatModelConfiguration`) を実行します。
UpdateとDeleteではPathパラメータからIDを取得します。

```go
package rtreq

import (
    "fmt"
    "github.com/gin-gonic/gin"
    "mycute/src/lib/common"
    "mycute/src/mode/rt/rtres"
    "mycute/src/mode/rt/rtutil"
    "mycute/src/pkg/cuber/types"
)

// --- 1. Search ---

type SearchChatModelsReq struct {
    Name        string `json:"name" binding:"max=50"`
    Provider    string `json:"provider" binding:""`
	Model       string `json:"model" binding:""`
	BaseURL     string `json:"base_url" binding:""`
}

func SearchChatModelsReqBind(c *gin.Context, u *rtutil.RtUtil) (SearchChatModelsReq, rtres.SearchChatModelsRes, bool) {
    ok := true
    req := SearchChatModelsReq{}
    res := rtres.SearchChatModelsRes{Errors: []rtres.Err{}}
    if err := c.ShouldBindJSON(&req); err != nil {
        res.Errors = u.GetValidationErrs(err)
        ok = false
    }
    return req, res, ok
}


// --- 2. Get ---

type GetChatModelReq struct {
    ID uint `binding:"gte=1"` // Path Paramなのでjsonタグ不要
}

func GetChatModelReqBind(c *gin.Context, u *rtutil.RtUtil) (GetChatModelReq, rtres.GetChatModelRes, bool) {
    ok := true
    req := GetChatModelReq{ID: common.StrToUint(c.Param("chat_model_id"))}
    res := rtres.GetChatModelRes{Errors: []rtres.Err{}}
    if err := c.ShouldBind(&req); err != nil {
		res.Errors = u.GetValidationErrs(err)
		ok = false
	}
	return req, res, ok
}


// --- 3. Create ---

type CreateChatModelReq struct {
    Name        string  `json:"name" binding:"required,max=50"`
    Provider    string  `json:"provider" binding:"required,max=50"`
    Model       string  `json:"model" binding:"required,max=100"`
    BaseURL     string  `json:"base_url" binding:"max=255"`
    ApiKey      string  `json:"api_key" binding:"required,max=1024"`
    MaxTokens   int     `json:"max_tokens" binding:"min=0"`
    Temperature float64 `json:"temperature" binding:"min=0,max=2"`
}

func CreateChatModelReqBind(c *gin.Context, u *rtutil.RtUtil) (CreateChatModelReq, rtres.CreateChatModelRes, bool) {
    ok := true
    req := CreateChatModelReq{}
    res := rtres.CreateChatModelRes{Errors: []rtres.Err{}}
    if err := c.ShouldBindJSON(&req); err != nil {
        res.Errors = u.GetValidationErrs(err)
        ok = false
    }
    // Live Test
    chatConfig := types.ChatModelConfig{
        Provider:    req.Provider,
        Model:       req.Model,
        BaseURL:     req.BaseURL,
        ApiKey:      req.ApiKey,
        MaxTokens:   req.MaxTokens,
        Temperature: req.Temperature,
    }
    if err := u.CuberService.VerifyChatModelConfiguration(c.Request.Context(), chatConfig); err != nil {
        res.Errors = append(res.Errors, rtres.Err{Field: "api_key", Message: fmt.Sprintf("Live verification failed: %s", err.Error())})
        ok = false
    }
    return req, res, ok
}

// --- 4. Update ---

type UpdateChatModelReq struct {
    ID          uint    `json:"-" binding:"gte=1"` // Path Param -> Internal
    Name        string  `json:"name" binding:"max=50"`
    Provider    string  `json:"provider" binding:"max=50"`
    Model       string  `json:"model" binding:"max=100"`
    BaseURL     string  `json:"base_url" binding:"max=255"`
    ApiKey      string  `json:"api_key" binding:"max=1024"` // Optional update
    MaxTokens   int     `json:"max_tokens" binding:"min=0"`
    Temperature *float64 `json:"temperature" binding:"omitempty,min=0,max=2"`
}

func UpdateChatModelReqBind(c *gin.Context, u *rtutil.RtUtil) (UpdateChatModelReq, rtres.UpdateChatModelRes, bool) {
    ok := true
    req := UpdateChatModelReq{ID: common.StrToUint(c.Param("chat_model_id"))}
    res := rtres.UpdateChatModelRes{Errors: []rtres.Err{}}
    if err := c.ShouldBindJSON(&req); err != nil {
        res.Errors = u.GetValidationErrs(err)
        ok = false
    }
    // Live Test for Update
    // Provider, Model, BaseURL, ApiKey が変更される場合、またはこれらが指定されている場合は
    // 既存のレコードとマージして完全な設定を作り、疎通確認を行う。
    // 指示により「req.Provider != "" || req.Model != "" || req.BaseURL != "" || req.ApiKey != ""」の時実施。
    
    needsLiveTest := req.Provider != "" || req.Model != "" || req.BaseURL != "" || req.ApiKey != ""
    
    if needsLiveTest {
         // DBから既存レコードを取得して補完する必要がある
         // ReqBind内でDB参照を行う (例外的な実装だが、バリデーションの一環として許容)
         ids := ju.IDs(!(ju.IsApx() || ju.IsFromKey()))
         var currentModel model.ChatModel
         // IDはPath Parameterから取得済み (req.ID)
         if err := u.DB.Where("id = ? AND apx_id = ? AND vdr_id = ?", req.ID, ids.ApxID, ids.VdrID).First(&currentModel).Error; err != nil {
             // 存在しない場合は404だが、Bind段階なのでValidation Errorとして返すか、falseを返してControllerでハンドリングさせるか。
             // ここではシンプルに「更新対象が見つからないため検証不能」としてエラーにする
             res.Errors = append(res.Errors, rtres.Err{Field: "id", Message: "Chat model not found for verification"})
             return req, res, false
         }

         // Config構築 (Request優先, 空ならExisting)
         targetProvider := currentModel.Provider
         if req.Provider != "" { targetProvider = req.Provider }
         
         targetModelName := currentModel.Model
         if req.Model != "" { targetModelName = req.Model }
         
         targetBaseURL := currentModel.BaseURL
         if req.BaseURL != "" { targetBaseURL = req.BaseURL }
         
         targetApiKey := ""
         if req.ApiKey != "" {
             targetApiKey = req.ApiKey
         } else {
             // 既存のKeyを復号して使う
             decrypted, err := mycrypto.Decrypt(currentModel.ApiKey, u.CryptoSecretKey)
             if err != nil {
                 res.Errors = append(res.Errors, rtres.Err{Field: "api_key", Message: "Failed to decrypt existing API key for verification"})
                 return req, res, false
             }
             targetApiKey = decrypted
         }
         
         // 検証用Config
         chatConfig := types.ChatModelConfig{
            Provider:  targetProvider,
            Model:     targetModelName,
            BaseURL:   targetBaseURL,
            ApiKey:    targetApiKey,
            // MaxTokens/Temperatureは疎通確認には本質的に不要だが渡しておく
            MaxTokens: currentModel.MaxTokens, 
            Temperature: currentModel.Temperature,
        }
        
        if err := u.CuberService.VerifyChatModelConfiguration(c.Request.Context(), chatConfig); err != nil {
             res.Errors = append(res.Errors, rtres.Err{Field: "configuration", Message: fmt.Sprintf("Live verification failed: %s", err.Error())})
             return req, res, false
        }
    }
    return req, res, true
}


// --- 5. Delete ---

type DeleteChatModelReq struct {
    ID uint `binding:"gte=1"`
}

func DeleteChatModelReqBind(c *gin.Context, u *rtutil.RtUtil) (DeleteChatModelReq, rtres.DeleteChatModelRes, bool) {
    ok := true
    req := DeleteChatModelReq{ID: common.StrToUint(c.Param("chat_model_id"))}
    res := rtres.DeleteChatModelRes{Errors: []rtres.Err{}}
    if err := c.ShouldBind(&req); err != nil {
        res.Errors = u.GetValidationErrs(err)
        ok = false
    }
    return req, res, ok
}
```

### 3.5. ビジネスロジック (`src/mode/rt/rtbl/chat_models_bl.go`)

DB操作の実装です。`ApxID` と `VdrID` によるパーティショニングを徹底します。
USRyが自分の権限範囲外のデータに触れないように、`ju.IDs(...)` を正しく使用します。

```go
package rtbl

import (
    "github.com/gin-gonic/gin"
    "github.com/t-kawata/mycute/enum/usrtype"
    "mycute/src/lib/common"
    "mycute/src/lib/mycrypto"
    "mycute/src/model"
    "mycute/src/mode/rt/rtreq"
    "mycute/src/mode/rt/rtres"
    "mycute/src/mode/rt/rtutil"
)

// --- 1. Search ---

func SearchChatModels(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.SearchChatModelsReq, res *rtres.SearchChatModelsRes) bool {
    ids := ju.IDs(!(ju.IsApx() || ju.IsFromKey()))
    var chatModels []model.ChatModel
    query := u.DB.Where("apx_id = ? AND vdr_id = ?", ids.ApxID, ids.VdrID)
    if req.Name != "" {
        query = query.Where("name LIKE ?", "%"+req.Name+"%")
    }
    if req.Provider != "" {
        query = query.Where("provider LIKE ?", "%"+req.Provider+"%")
    }
    if req.Model != "" {
        query = query.Where("model LIKE ?", "%"+req.Model+"%")
    }
    if req.BaseURL != "" {
        query = query.Where("base_url LIKE ?", "%"+req.BaseURL+"%")
    }
    if err := query.Find(&chatModels).Error; err != nil {
        return InternalServerErrorCustomMsg(c, res, err.Error())
    }
    return OK(c, rtres.SearchChatModelsResData{}.Of(&chatModels), res)
}


// --- 2. Get ---

func GetChatModel(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.GetChatModelReq, res *rtres.GetChatModelRes) bool {
    ids := ju.IDs(!(ju.IsApx() || ju.IsFromKey())) // USRだけが使用可能なので
    var m model.ChatModel
    if err := u.DB.Where("id = ? AND apx_id = ? AND vdr_id = ?", req.ID, ids.ApxID, ids.VdrID).First(&m).Error; err != nil {
        return NotFoundCustomMsg(c, res, "Chat model not found")
    }
    return OK(c, rtres.GetChatModelResData{}.Of(&m), res)
}


// --- 3. Create ---

func CreateChatModel(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.CreateChatModelReq, res *rtres.CreateChatModelRes) bool {
    ids := ju.IDs(!(ju.IsApx() || ju.IsFromKey())) // USRだけが使用可能なので
    // 暗号化
    encKey, err := mycrypto.Encrypt(req.ApiKey, u.CryptoSecretKey)
    if err != nil {
        return InternalServerErrorCustomMsg(c, res, "Failed to encrypt API key.")
    }
    m := model.ChatModel{
        Name:        req.Name,
        Provider:    req.Provider,
        Model:       req.Model,
        BaseURL:     req.BaseURL,
        ApiKey:      encKey,
        MaxTokens:   req.MaxTokens,
        Temperature: req.Temperature,
        ApxID:       *ids.ApxID,
        VdrID:       *ids.VdrID,
    }
    if err := u.DB.Create(&m).Error; err != nil {
        return InternalServerErrorCustomMsg(c, res, err.Error())
    }
    data := rtres.CreateChatModelResData{ID: m.ID}
    return OK(c, &data, res)
}


// --- 4. Update ---

func UpdateChatModel(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.UpdateChatModelReq, res *rtres.UpdateChatModelRes) bool {
    ids := ju.IDs(!(ju.IsApx() || ju.IsFromKey())) // USRだけが使用可能なので
    var m model.ChatModel
    if err := u.DB.Where("id = ? AND apx_id = ? AND vdr_id = ?", req.ID, ids.ApxID, ids.VdrID).First(&m).Error; err != nil {
        return NotFoundCustomMsg(c, res, "Chat model not found")
    }
    // Update fields if present
    if req.Name != "" { m.Name = req.Name }
    if req.Provider != "" { m.Provider = req.Provider }
    if req.Model != "" { m.Model = req.Model }
    if req.BaseURL != "" { m.BaseURL = req.BaseURL }
    if req.MaxTokens > 0 { m.MaxTokens = req.MaxTokens }
    // Temperature (Pointer check)
    if req.Temperature != nil {
        m.Temperature = *req.Temperature
    }
    if req.ApiKey != "" {
        encKey, err := mycrypto.Encrypt(req.ApiKey, u.CryptoSecretKey)
        if err != nil {
            return InternalServerErrorCustomMsg(c, res, "Failed to encrypt API key")
        }
        m.ApiKey = encKey
    }
    if err := u.DB.Save(&m).Error; err != nil {
         return InternalServerErrorCustomMsg(c, res, err.Error())
    }
    return OK(c, rtres.UpdateChatModelResData{}.Of(&m), res)
}


// --- 5. Delete ---

func DeleteChatModel(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.DeleteChatModelReq, res *rtres.DeleteChatModelRes) bool {
    ids := ju.IDs(!(ju.IsApx() || ju.IsFromKey())) // USRだけが使用可能なので
    // .Unscoped() で論理削除を解除して物理削除する
    result := u.DB.Where("id = ? AND apx_id = ? AND vdr_id = ?", req.ID, ids.ApxID, ids.VdrID).Unscoped().Delete(&model.ChatModel{})
    if result.Error != nil {
        return InternalServerErrorCustomMsg(c, res, result.Error.Error())
    }
    if result.RowsAffected == 0 {
        return NotFoundCustomMsg(c, res, "Chat model not found")
    }
    return OK(c, &rtres.DeleteChatModelResData{}, res)
}
```

### 3.6. ハンドラー (`src/mode/rt/rthandler/hv1/chat_models_handler.go`)

```go
package hv1

import (
	"github.com/gin-gonic/gin"
	"mycute/src/enum/usrtype"
	"mycute/src/mode/rt/rtbl"
	"mycute/src/mode/rt/rtreq"
	"mycute/src/mode/rt/rtutil"
)

// --- 1. Search ---

// @Tags v1 ChatModel
// @Router /v1/chat_models/search [post]
// @Summary ChatModelを検索
// @Description - USR によってのみ使用できる
// @Accept application/json
// @Param Authorization header string true "token" example(Bearer ??????????)
// @Param params body rtparam.SearchChatModelsParam true "Search Params"
// @Success 200 {object} rtres.SearchChatModelsRes "Success"
// @Failure 400 {object} rtres.ErrRes "Validation Error"
// @Failure 401 {object} rtres.ErrRes "Unauthorized"
// @Failure 500 {object} rtres.ErrRes "Internal Server Error"
func SearchChatModels(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr) {
	if rtbl.RejectUsr(c, u, ju, []usrtype.UsrType{usrtype.KEY, usrtype.APX, usrtype.VDR}) { // USRのみ使用可能
		return
	}
	if req, res, ok := rtreq.SearchChatModelsReqBind(c, u); ok {
		rtbl.SearchChatModels(c, u, ju, &req, &res)
	} else {
		rtbl.BadRequest(c, &res)
	}
}

// --- 2. Get ---

// @Tags v1 ChatModel
// @Router /v1/chat_models/{chat_model_id} [get]
// @Summary ChatModel詳細取得
// @Description - USR によってのみ使用できる
// @Param Authorization header string true "token"
// @Param chat_model_id path int true "ChatModel ID"
// @Success 200 {object} rtres.GetChatModelRes
// @Failure 400 {object} rtres.ErrRes
// @Failure 401 {object} rtres.ErrRes
// @Failure 404 {object} rtres.ErrRes
// @Failure 500 {object} rtres.ErrRes
func GetChatModel(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr) {
	if rtbl.RejectUsr(c, u, ju, []usrtype.UsrType{usrtype.KEY, usrtype.APX, usrtype.VDR}) { // USRのみ使用可能
		return
	}
	if req, res, ok := rtreq.GetChatModelReqBind(c, u); ok {
		rtbl.GetChatModel(c, u, ju, &req, &res)
	} else {
		rtbl.BadRequest(c, &res)
	}
}

// --- 3. Create ---

// @Tags v1 ChatModel
// @Router /v1/chat_models/ [post]
// @Summary ChatModel作成
// @Description - USR によってのみ使用できる
// @Param Authorization header string true "token"
// @Param json body rtparam.CreateChatModelParam true "json"
// @Success 200 {object} rtres.CreateChatModelRes
// @Failure 400 {object} rtres.ErrRes
// @Failure 401 {object} rtres.ErrRes
// @Failure 500 {object} rtres.ErrRes
func CreateChatModel(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr) {
    if rtbl.RejectUsr(c, u, ju, []usrtype.UsrType{usrtype.KEY, usrtype.APX, usrtype.VDR}) { // USRのみ使用可能
		return
	}
	if req, res, ok := rtreq.CreateChatModelReqBind(c, u); ok {
		rtbl.CreateChatModel(c, u, ju, &req, &res)
	} else {
		rtbl.BadRequest(c, &res)
	}
}

// --- 4. Update ---

// @Tags v1 ChatModel
// @Router /v1/chat_models/{chat_model_id} [patch]
// @Summary ChatModel更新
// @Description - USR によってのみ使用できる
// @Param Authorization header string true "token"
// @Param chat_model_id path int true "ChatModel ID"
// @Param json body rtparam.UpdateChatModelParam true "json"
// @Success 200 {object} rtres.UpdateChatModelRes
// @Failure 400 {object} rtres.ErrRes
// @Failure 401 {object} rtres.ErrRes
// @Failure 404 {object} rtres.ErrRes
// @Failure 500 {object} rtres.ErrRes
func UpdateChatModel(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr) {
	if rtbl.RejectUsr(c, u, ju, []usrtype.UsrType{usrtype.KEY, usrtype.APX, usrtype.VDR}) { // USRのみ使用可能
		return
	}
	if req, res, ok := rtreq.UpdateChatModelReqBind(c, u); ok {
		rtbl.UpdateChatModel(c, u, ju, &req, &res)
	} else {
		rtbl.BadRequest(c, &res)
	}
}

// --- 5. Delete ---

// @Tags v1 ChatModel
// @Router /v1/chat_models/{chat_model_id} [delete]
// @Summary ChatModel削除
// @Description - USR によってのみ使用できる
// @Param Authorization header string true "token"
// @Param chat_model_id path int true "ChatModel ID"
// @Success 200 {object} rtres.DeleteChatModelRes
// @Failure 400 {object} rtres.ErrRes
// @Failure 401 {object} rtres.ErrRes
// @Failure 404 {object} rtres.ErrRes
// @Failure 500 {object} rtres.ErrRes
func DeleteChatModel(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr) {
	if rtbl.RejectUsr(c, u, ju, []usrtype.UsrType{usrtype.KEY, usrtype.APX, usrtype.VDR}) { // USRのみ使用可能
		return
	}
	if req, res, ok := rtreq.DeleteChatModelReqBind(c, u); ok {
		rtbl.DeleteChatModel(c, u, ju, &req, &res)
	} else {
		rtbl.BadRequest(c, &res)
	}
}
```

### 3.7. ルーティング (`src/mode/rt/request_mapper.go`)

`v1.Group` を使用してルーティングを定義します。
`GetUtil` によるUtil取得とエラーハンドリングラッパーを実装します。

```go
		// Chat Models
		cms := v1.Group("/chat_models")
		// 1. Search (POST /search)
		cms.POST("/search", func(c *gin.Context) {
			u, ju, ok := GetUtil(c)
			if !ok {
				c.JSON(http.StatusForbidden, nil)
				return
			}
			hv1.SearchChatModels(c, u, ju)
		})
		// 2. Get (GET /:chat_model_id)
		cms.GET("/:chat_model_id", func(c *gin.Context) {
			u, ju, ok := GetUtil(c)
			if !ok {
				c.JSON(http.StatusForbidden, nil)
				return
			}
			hv1.GetChatModel(c, u, ju)
		})
		// 3. Create (POST /)
		cms.POST("/", func(c *gin.Context) {
			u, ju, ok := GetUtil(c)
			if !ok {
				c.JSON(http.StatusForbidden, nil)
				return
			}
			hv1.CreateChatModel(c, u, ju)
		})
		// 4. Update (PATCH /:chat_model_id)
		cms.PATCH("/:chat_model_id", func(c *gin.Context) {
			u, ju, ok := GetUtil(c)
			if !ok {
				c.JSON(http.StatusForbidden, nil)
				return
			}
			hv1.UpdateChatModel(c, u, ju)
		})
		// 5. Delete (DELETE /:chat_model_id)
		cms.DELETE("/:chat_model_id", func(c *gin.Context) {
			u, ju, ok := GetUtil(c)
			if !ok {
				c.JSON(http.StatusForbidden, nil)
				return
			}
			hv1.DeleteChatModel(c, u, ju)
		})
```

## 4. Cube 操作 API の改修

Cube の操作を行う API において、使用する `ChatModel` を指定できるように改修します。
これにより、システムデフォルトの LLM ではなく、ユーザーが設定した（あるいは指定した）任意のモデルを使用して RAG や要約を実行できるようになります。

**対象 API:**
1.  `AbsorbCube` (Knowledge Absorption)
2.  `QueryCube` (RAG Query)
3.  `MemifyCube` (Self-Correction / Knowledge Graph Management)

以下に、各層ごとの具体的な変更内容を示します。

### 4.1. パラメータ定義 (`src/mode/rt/rtparam/cubes_param.go`)

Swagger ドキュメント生成用のパラメータ構造体に `chat_model_id` を追加します。既存のフィールドは維持し、追記してください。

```go
// AbsorbCubeParam
type AbsorbCubeParam struct {
    // ... 既存フィールド ...
    ChatModelID uint   `json:"chat_model_id" swaggertype:"integer" example:"1" binding:"required"` // [NEW]
}

// QueryCubeParam
type QueryCubeParam struct {
    // ... 既存フィールド ...
    ChatModelID uint   `json:"chat_model_id" swaggertype:"integer" example:"1" binding:"required"` // [NEW]
}

// MemifyCubeParam
type MemifyCubeParam struct {
    CubeID      uint `json:"cube_id" binding:"required" example:"1"`
    ChatModelID uint `json:"chat_model_id" swaggertype:"integer" example:"1" binding:"required"` // [NEW]
}
```

### 4.2. リクエスト処理 (`src/mode/rt/rtreq/cubes_req.go`)

Binding 用のリクエスト構造体にも `ChatModelID` を追加します。

```go
type AbsorbCubeReq struct {
    // ... 既存フィールド ...
    ChatModelID uint `json:"chat_model_id" binding:"required,gte=1"` // [NEW]
}

type QueryCubeReq struct {
    // ... 既存フィールド ...
    ChatModelID uint `json:"chat_model_id" binding:"required,gte=1"` // [NEW]
}

type MemifyCubeReq struct {
    CubeID      uint `json:"cube_id" binding:"required,gte=1"`
    ChatModelID uint `json:"chat_model_id" binding:"required,gte=1"` // [NEW]
}
```

### 4.3. ビジネスロジック実装 (`src/mode/rt/rtbl/cubes_bl.go`)

`ChatModel` を取得し、復号化した API キーを用いて `ChatModelConfig` を作成し、Service に渡すロジックを実装します。
`ApxID` と `VdrID` のチェックにより、他人のモデルを使用できないように制御します。

**各関数 (`AbsorbCube`, `QueryCube`, `MemifyCube`) に共通して追加する処理:**

1.  `ju.IDs(...)` でアクセス元の `ApxID`, `VdrID` を取得。
2.  指定された `ChatModelID` のレコードを DB から取得（`ApxID`, `VdrID` 条件必須）。
3.  `mycrypto.Decrypt` で API キーを復号。
4.  `types.ChatModelConfig` を作成。
5.  `u.CuberService` のメソッドに Config を渡す。

**実装例 (`AbsorbCube`):**

```go
func AbsorbCube(c *gin.Context, u *rtutil.RtUtil, ju *rtutil.JwtUsr, req *rtreq.AbsorbCubeReq, res *rtres.AbsorbCubeRes) bool {
    // 1. 権限とID取得
    ids := ju.IDs(!(ju.IsApx() || ju.IsFromKey()))

    // 2. ChatModel 取得 (ApxID/VdrID チェック付き)
    var chatModel model.ChatModel
    // 自分のApx/Vdrに属するモデルのみ使用可能
    if err := u.DB.Where("id = ? AND apx_id = ? AND vdr_id = ?", req.ChatModelID, ids.ApxID, ids.VdrID).First(&chatModel).Error; err != nil {
        return BadRequestCustomMsg(c, res, "Invalid chat_model_id")
    }

    // 3. API Key 復号
    decryptedApiKey, err := mycrypto.Decrypt(chatModel.ApiKey, u.CryptoSecretKey)
    if err != nil {
        return InternalServerErrorCustomMsg(c, res, "Failed to decrypt chat model API key")
    }

    // 4. Config 作成
    chatConfig := types.ChatModelConfig{
        Provider:    chatModel.Provider,
        Model:       chatModel.Model,
        BaseURL:     chatModel.BaseURL,
        ApiKey:      decryptedApiKey,
        MaxTokens:   chatModel.MaxTokens,
        Temperature: chatModel.Temperature,
    }

    // --- 既存の Cube 処理 ---
    // (ここで既存のCube取得や権限チェック等を行う)
    // ...
    
    // 5. Service 呼び出し (Config を渡すように変更)
    // u.CuberService.Absorb(c.Request.Context(), ..., chatConfig) 
    
    // ... レスポンス返却 ...
}
```

`QueryCube`, `MemifyCube` についても同様に、冒頭で `ChatModel` を解決し、Config を構築して Service 関数へ渡すように修正してください。

## 5. サービス層の改修 (`CuberService`)

`src/pkg/cuber/cuber.go` の `CuberService` を、**ステートレスな LLM 管理** に移行するための具体的な手順を示します。
各リクエストごとに LLM インスタンスを生成・破棄するように変更し、グローバルな状態としての LLM 保持を廃止します。

### 5.1. 構造体の変更 (`src/pkg/cuber/cuber.go`)

`CuberService` 構造体から `LLM` フィールドを削除し、`NewCuberService` での初期化ロジックも除去します。

```go
type CuberService struct {
    StorageMap map[string]*StorageSet
    mu         sync.RWMutex
    // LLM        model.ToolCallingChatModel // [DELETE] 削除
    Config     types.CuberConfig
    S3Client   *s3client.S3Client
    closeCh    chan struct{}
}

func NewCuberService(config types.CuberConfig) (*CuberService, error) {
    // ... (設定のデフォルト値適用など) ...

    // [DELETE] LLM初期化ロジックを削除
    // ctx := context.Background()
    // chatConfig := providers.ProviderConfig{ ... }
    // chatModel, err := providers.NewChatModel(ctx, chatConfig)
    // ...

    service := &CuberService{
        StorageMap: make(map[string]*StorageSet),
        // LLM:        chatModel, // [DELETE] 削除
        Config:     config,
        S3Client:   s3Client,
        closeCh:    closeCh,
    }
    // ...
    return service, nil
}
```

### 5.2. `createTempChatModel` メソッドの実装 (`src/pkg/cuber/cuber.go`)

指定された設定 (`types.ChatModelConfig`) に基づいて LLM インスタンスを生成するヘルパーメソッドを追加します。
`Embedding` における `createTempEmbedder` と同様の役割です。

```go
// createTempChatModel creates a temporary chat model instance for a specific operation.
func (s *CuberService) createTempChatModel(ctx context.Context, config types.ChatModelConfig) (model.ToolCallingChatModel, error) {
    pConfig := providers.ProviderConfig{
        Type:      providers.ProviderType(config.Provider),
        APIKey:    config.ApiKey,
        BaseURL:   config.BaseURL,
        ModelName: config.Model,
        MaxTokens: config.MaxTokens,
        Temperature: config.Temperature, // 追加
    }
    // providers.NewChatModel を使用してインスタンス生成
    return providers.NewChatModel(ctx, pConfig)
}
```

### 5.3. メソッドシグネチャと実装の変更 (`src/pkg/cuber/cuber.go`)

`Absorb`, `Query`, `Memify` などの主要メソッドの引数に `chatModelConfig types.ChatModelConfig` を追加し、内部で `createTempChatModel` を使用するように変更します。
生成した `chatModel` インスタンスは、各タスク (`tasks/*`) やツール (`tools/query/*`) に渡します。

**例: `Absorb` メソッド**

```go
func (s *CuberService) Absorb(ctx context.Context, cubeDbFilePath string, memoryGroup string, filePaths []string, cognifyConfig types.CognifyConfig, embeddingModelConfig types.EmbeddingModelConfig, chatModelConfig types.ChatModelConfig) (types.TokenUsage, error) {
    // ...
    
    // Create temp chat model
    chatModel, err := s.createTempChatModel(ctx, chatModelConfig) // [NEW]
    if err != nil {
         return totalUsage, fmt.Errorf("Absorb: Failed to create chat model: %w", err)
    }

    // Pass chatModel to cognify
    usage2, err := s.cognify(ctx, cubeDbFilePath, memoryGroup, cognifyConfig, embeddingModelConfig, embedder, chatModel) // [MODIFIED]
    // ...
}
```

**例: `cognify` 内部メソッド**

```go
func (s *CuberService) cognify(..., embedder storage.Embedder, chatModel model.ToolCallingChatModel) (types.TokenUsage, error) { // [MODIFIED]
    // ...
    // s.LLM の代わりに 引数の chatModel を使用してタスク初期化
    graphTask := graph.NewGraphExtractionTask(chatModel, s.Config.CompletionModel, memoryGroup) // [MODIFIED]
    summarizationTask := summarization.NewSummarizationTask(st.Vector, chatModel, embedder, memoryGroup, s.Config.CompletionModel) // [MODIFIED]
    // ...
}
```

**例: `Query` メソッド**

```go
func (s *CuberService) Query(..., chatModelConfig types.ChatModelConfig) (...) { // [MODIFIED]
    // ...
    // Create temp chat model
    chatModel, err := s.createTempChatModel(ctx, chatModelConfig) // [NEW]
    if err != nil { ... }

    // s.LLM の代わりに chatModel を使用
    searchTool := query.NewGraphCompletionTool(..., chatModel, ...) // [MODIFIED]
    // ...
}
```

※ `metacognition` パッケージ（`memify`で使用）内のタスク生成部分も同様に `s.LLM` から引数の `chatModel` を使うように修正が必要です。

## 6. 作業手順

以下の順序で実装することを推奨します。

1.  **Config クリーンアップ (準備)**: `types` から削除、`MainOfRT` から削除。（ビルドが通らなくなるので一気にやる）
2.  **DB モデル追加**: `ChatModel` 追加。
3.  **Service 層改修**: `CuberService` の LLM フィールド削除、`createTempChatModel` 追加、各メソッドの引数変更。
    *   この段階で、呼び出し元 (`rtbl`) はコンパイルエラーになるが、Service内部の整合性を先に取る。
4.  **REST API (ChatModel CRUD) 実装**: `ChatModel` を登録できるようにする。
    *   `rthandler`, `rtreq`, `rtres`, `rtbl` を一式実装。Live Test もここで。
5.  **Cube 操作 API 改修**:
    *   `rtparam`, `rtreq` に `ChatModelID` 追加。
    *   `rtbl` で ChatModel 取得・復号・Service呼び出しの実装。
6.  **検証**: `make swag`, `make build`, `make build-linux-amd64`。

---
**完了条件:**
- 環境変数による LLM 設定がコードベースから完全に消去されていること。
- `ChatModel` CRUD が正常に動作し、登録時に Live Test が行われること。
- Cube の操作 (`Absorb`, `Query`, `Memify`) が、リクエストで指定された `ChatModel` を使用して実行されること。

