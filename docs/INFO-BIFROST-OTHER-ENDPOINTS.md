Bifrostはcompletion以外にも多様なAI推論エンドポイントを提供しています。

## 統一APIエンドポイント（/v1/*）

### Embeddings（埋め込み）
- `POST /v1/embeddings` - テキストの埋め込みベクトルを生成 [1](#7-0) 

### Audio（音声）
- `POST /v1/audio/speech` - テキスト読み上げ（TTS） [2](#7-1) 
- `POST /v1/audio/transcriptions` - 音声文字起こし（STT） [3](#7-2) 

### Images（画像）
- `POST /v1/images/generations` - 画像生成 [4](#7-3) 
- `POST /v1/images/edits` - 画像編集 [5](#7-4) 
- `POST /v1/images/variations` - 画像バリエーション生成 [6](#7-5) 

### Videos（動画）
- `POST /v1/videos` - 動画生成 [7](#7-6) 
- `GET /v1/videos/{video_id}` - 動画情報取得 [8](#7-7) 
- `GET /v1/videos/{video_id}/content` - 動画ダウンロード [9](#7-8) 
- `POST /v1/videos/{video_id}/remix` - 動画リミックス [10](#7-9) 

### その他のエンドポイント
- `POST /v1/rerank` - ドキュメントの再ランク付け [11](#7-10) 
- `POST /v1/responses/input_tokens` - トークン数カウント [12](#7-11) 
- `GET /v1/models` - 利用可能なモデル一覧 [13](#7-12) 

### Files & Containers（ファイル管理）
- `POST /v1/files` - ファイルアップロード [14](#7-13) 
- `GET /v1/files` - ファイル一覧取得
- `GET /v1/files/{file_id}` - ファイル情報取得
- `GET /v1/files/{file_id}/content` - ファイルコンテンツ取得
- `DELETE /v1/files/{file_id}` - ファイル削除
- `POST /v1/containers` - コンテナ作成 [15](#7-14) 
- `GET /v1/containers` - コンテナ一覧取得

### Batch（バッチ処理）
- `POST /v1/batches` - バッチジョブ作成 [16](#7-15) 
- `GET /v1/batches` - バッチ一覧取得
- `GET /v1/batches/{batch_id}` - バッチ情報取得
- `POST /v1/batches/{batch_id}/cancel` - バッチキャンセル [17](#7-16) 
- `GET /v1/batches/{batch_id}/results` - バッチ結果取得 [18](#7-17) 

## 非同期エンドポイント（/v1/async/*）

上記のエンドポイントのほとんどは非同期バージョンも利用可能です： [19](#7-18) 

- `POST /v1/async/embeddings`
- `POST /v1/async/audio/speech`
- `POST /v1/async/audio/transcriptions`
- `POST /v1/async/images/generations`
- `POST /v1/async/images/edits`
- `POST /v1/async/images/variations`
- `POST /v1/async/rerank`

## プロバイダー固有エンドポイント

各プロバイダーのネイティブAPI形式でもアクセス可能です： [20](#7-19) 

- `/openai/v1/embeddings`
- `/openai/v1/audio/speech`
- `/openai/v1/audio/transcriptions`
- `/openai/v1/images/generations`
- `/anthropic/v1/messages`（チャットのみ）
- `/genai/v1beta/models/...`（Gemini形式）
- `/bedrock/...`（AWS Bedrock形式）

## Notes

- 全てのエンドポイントで `stream: true` パラメータを指定するとストリーミング対応（可能な場合）
- 非同期エンドポイントではストリーミングはサポートされていません [21](#7-20) 
- 各プロバイダーでサポートされている機能は異なります（例：MistralはSpeechをサポートせず、Embeddingsをサポート） [22](#7-21) 

Wiki pages you might want to explore:
- [API Handlers and Endpoints (maximhq/bifrost)](/wiki/maximhq/bifrost#3.3)
- [Drop-in Replacement Endpoints (maximhq/bifrost)](/wiki/maximhq/bifrost#3.4)
- [Middleware and Authentication (maximhq/bifrost)](/wiki/maximhq/bifrost#3.5)

### Citations

**File:** docs/openapi/openapi.yaml (L22-29)
```yaml
    ### Provider Integration APIs
    Native provider-format APIs for drop-in compatibility:
    - `/openai/*` - OpenAI-compatible API
    - `/anthropic/*` - Anthropic-compatible API
    - `/genai/*` - Google GenAI (Gemini) compatible API
    - `/bedrock/*` - AWS Bedrock compatible API
    - `/cohere/*` - Cohere compatible API

```

**File:** docs/openapi/openapi.yaml (L133-134)
```yaml
  /v1/models:
    $ref: './paths/inference/models.yaml#/models'
```

**File:** docs/openapi/openapi.yaml (L141-142)
```yaml
  /v1/rerank:
    $ref: './paths/inference/rerank.yaml#/rerank'
```

**File:** docs/openapi/openapi.yaml (L143-144)
```yaml
  /v1/embeddings:
    $ref: './paths/inference/embeddings.yaml#/embeddings'
```

**File:** docs/openapi/openapi.yaml (L145-146)
```yaml
  /v1/audio/speech:
    $ref: './paths/inference/audio.yaml#/speech'
```

**File:** docs/openapi/openapi.yaml (L147-148)
```yaml
  /v1/audio/transcriptions:
    $ref: './paths/inference/audio.yaml#/transcriptions'
```

**File:** docs/openapi/openapi.yaml (L149-150)
```yaml
  /v1/images/generations:
    $ref: './paths/inference/images.yaml#/image-generation'
```

**File:** docs/openapi/openapi.yaml (L151-152)
```yaml
  /v1/images/edits:
    $ref: './paths/inference/images.yaml#/image-edit'
```

**File:** docs/openapi/openapi.yaml (L153-154)
```yaml
  /v1/images/variations:
    $ref: './paths/inference/images.yaml#/image-variation'
```

**File:** docs/openapi/openapi.yaml (L155-156)
```yaml
  /v1/videos:
    $ref: './paths/inference/videos.yaml#/video-generation'
```

**File:** docs/openapi/openapi.yaml (L157-158)
```yaml
  /v1/videos/{video_id}:
    $ref: './paths/inference/videos.yaml#/video-by-id'
```

**File:** docs/openapi/openapi.yaml (L159-160)
```yaml
  /v1/videos/{video_id}/content:
    $ref: './paths/inference/videos.yaml#/video-download'
```

**File:** docs/openapi/openapi.yaml (L161-162)
```yaml
  /v1/videos/{video_id}/remix:
    $ref: './paths/inference/videos.yaml#/video-remix'
```

**File:** docs/openapi/openapi.yaml (L163-164)
```yaml
  /v1/responses/input_tokens:
    $ref: './paths/inference/count-tokens.yaml#/count-tokens'
```

**File:** docs/openapi/openapi.yaml (L165-166)
```yaml
  /v1/batches:
    $ref: './paths/inference/batches.yaml#/batches'
```

**File:** docs/openapi/openapi.yaml (L169-170)
```yaml
  /v1/batches/{batch_id}/cancel:
    $ref: './paths/inference/batches.yaml#/batches-cancel'
```

**File:** docs/openapi/openapi.yaml (L171-172)
```yaml
  /v1/batches/{batch_id}/results:
    $ref: './paths/inference/batches.yaml#/batches-results'
```

**File:** docs/openapi/openapi.yaml (L173-174)
```yaml
  /v1/files:
    $ref: './paths/inference/files.yaml#/files'
```

**File:** docs/openapi/openapi.yaml (L179-180)
```yaml
  /v1/containers:
    $ref: './paths/inference/containers.yaml#/containers'
```

**File:** transports/bifrost-http/handlers/asyncinference.go (L27-38)
```go
var AsyncPathToTypeMapping = map[string]schemas.RequestType{
	"/v1/async/completions":          schemas.TextCompletionRequest,
	"/v1/async/chat/completions":     schemas.ChatCompletionRequest,
	"/v1/async/responses":            schemas.ResponsesRequest,
	"/v1/async/embeddings":           schemas.EmbeddingRequest,
	"/v1/async/audio/speech":         schemas.SpeechRequest,
	"/v1/async/audio/transcriptions": schemas.TranscriptionRequest,
	"/v1/async/images/generations":   schemas.ImageGenerationRequest,
	"/v1/async/images/edits":         schemas.ImageEditRequest,
	"/v1/async/images/variations":    schemas.ImageVariationRequest,
	"/v1/async/rerank":               schemas.RerankRequest,
}
```

**File:** docs/features/async-inference.mdx (L40-40)
```text
Streaming is not supported on async endpoints.
```

**File:** docs/providers/supported-providers/mistral.mdx (L25-29)
```text
| Image Generation | ❌ | ❌ | - |
| Text Completions | ❌ | ❌ | - |
| Speech (TTS) | ❌ | ❌ | - |
| Files | ❌ | ❌ | - |
| Batch | ❌ | ❌ | - |
```
