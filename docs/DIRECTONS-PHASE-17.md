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
    Temperature float64 `gorm:"not null;default:0.7"`        // 生成温度
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
- `COMPLETION_MODEL`

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

**検証内容:**
- 指定された Provider/APIKey/BaseURL/Model で簡単なチャット完了リクエスト ("Hello" 等) を送り、エラーが返らないことを確認する。

## 4. Cube 操作 API の改修

Cube の操作を行う以下の API リクエストパラメータに `chat_model_id` (必須) を追加し、ロジックを修正します。

**対象 API:**
1.  `AbsorbCube` (Knowledge Absorption)
2.  `QueryCube` (RAG Query)
3.  `MemifyCube` (Self-Correction / Knowledge Graph Management)

### 4.1. パラメータ変更 (`rtparam` & `rtreq`)

各 `Req` 構造体に `ChatModelID uint` を追加します。

```go
type AbsorbCubeReq struct {
    // ...
    // [NEW] 使用するChatモデルのID
    ChatModelID uint `json:"chat_model_id" binding:"required"` 
}
```
(`QueryCubeReq`, `MemifyCubeReq` も同様)

### 4.2. ビジネスロジック変更 (`rtbl`)

各 `rtbl` 関数 (`AbsorbCube`, `QueryCube`, `MemifyCube`) の処理フローを変更します。

1.  **ChatModel の取得**: リクエストされた `ChatModelID` を使い、DBから `ChatModel` レコードを取得します。
    *   **重要**: 必ず `ApxID` と `VdrID` が一致するものだけを取得すること（他人のモデルを使わせない）。
2.  **API Key の復号**: `mycrypto.Decrypt` で API キーを復号します。
3.  **Config 構造体の作成**: `types` パッケージに `ChatModelConfig` (仮) 構造体を定義するか、既存の `EmbeddingModelConfig` に似た設定オブジェクトを作成し、復号済みキーを含めます。
4.  **CuberService 呼び出し**: `u.CuberService.Absorb(...)` 等の引数に、上記の `ChatModelConfig` を渡します。

```go
// rtbl/cubes_bl.go 例
chatModel := model.ChatModel{}
if err := u.DB.Where("id = ? AND apx_id = ? AND vdr_id = ?", req.ChatModelID, *ids.ApxID, *ids.VdrID).First(&chatModel).Error; err != nil {
    return BadRequestCustomMsg(c, res, "Invalid chat_model_id")
}
decryptedKey, err := mycrypto.Decrypt(chatModel.ApiKey, u.CryptoSecretKey)
// ...
chatConfig := types.ChatModelConfig{
    Provider: chatModel.Provider,
    Model:    chatModel.Model,
    BaseURL:  chatModel.BaseURL,
    ApiKey:   decryptedKey,
    // ...
}
u.CuberService.Absorb(..., chatConfig)
```

## 5. サービス層の改修 (`CuberService`)

`src/pkg/cuber/cuber.go` の `CuberService` を、**ステートレスな LLM 管理** に移行します。

### 5.1. 構造体と初期化
- `CuberService` から `LLM` (およびそれに類するフィールド) を削除。
- `NewCuberService` から LLM 初期化ロジックを削除。

### 5.2. `createTempLLM` メソッドの実装
指定された設定 (`types.ChatModelConfig`) に基づいて LLM インスタンスを生成するヘルパーメソッドを実装します。

```go
func (s *CuberService) createTempLLM(ctx context.Context, config types.ChatModelConfig) (llm.ChatModel, error) {
    // providers パッケージ等を使用して Eino/LangChain 互換の ChatModel を生成
    // Embedding同様、ProviderTypeへの変換やFactory利用を行う
}
```

### 5.3. メソッドシグネチャの変更
- `Absorb`, `Query`, `Memify` の引数に `chatModelConfig` を追加。
- 内部で `createTempLLM` を呼び出し、生成された LLM インスタンスを各タスク (`IngestTask`, `QueryTask`, `MemifyTask` 等) に渡す。

**注意**:
タスク内部(`src/pkg/cuber/task/...`)やツール(`src/pkg/cuber/tools/...`)でも、これまで `CuberService` から渡されていた（あるいはグローバルな）LLM インスタンスではなく、**引数として渡された LLM インスタンス** を使用するように修正が必要になる可能性があります。影響範囲を確認し、バケツリレーを実装してください。

`src/pkg/cuber/task/summarization/summarization_task.go` や `src/pkg/cuber/tools/query/query_tool.go` などが主な修正対象です。

## 6. 作業手順

以下の順序で実装することを推奨します。

1.  **Config クリーンアップ (準備)**: `types` から削除、`MainOfRT` から削除。（ビルドが通らなくなるので一気にやる）
2.  **DB モデル追加**: `ChatModel` 追加。
3.  **Service 層改修**: `CuberService` の LLM フィールド削除、`createTempLLM` 追加、各メソッドの引数変更。
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
