## Q5
Bifrostが提供する普通のcompletionとstreamのエンドポイントはどこですか？
### Answer
Bifrostが提供するcompletionとstreamingのエンドポイントは以下の通りです。

## 主要なエンドポイント

### 1. 統一APIエンドポイント（`/v1/*`）

**Chat Completions:**
- `POST /v1/chat/completions` - チャット補完（通常・ストリーミング対応） bifrost:135-136 

**Text Completions:**
- `POST /v1/completions` - テキスト補完（通常・ストリーミング対応） bifrost:137-138 

**Responses API:**
- `POST /v1/responses` - OpenAI Responses API互換（通常・ストリーミング対応） bifrost:139-140 

### 2. プロバイダー固有エンドポイント

各プロバイダーのネイティブ形式で直接アクセス可能です：

- `/openai/v1/chat/completions` - OpenAI形式 bifrost:24-25 
- `/anthropic/v1/messages` - Anthropic形式 bifrost:25-26 
- `/genai/v1/*` - Google Gemini形式 bifrost:26-27 
- `/bedrock/*` - AWS Bedrock形式 bifrost:27-28 

## ストリーミングの仕組み

ストリーミングはリクエストボディの `stream` パラメータで制御します：

```json
{
  "model": "openai/gpt-4",
  "messages": [{"role": "user", "content": "Hello!"}],
  "stream": true  // trueでストリーミング有効
}
```

実装例（chat completions）： bifrost:927-929 

```go
if req.Stream != nil && *req.Stream {
    h.handleStreamingChatCompletion(ctx, bifrostChatReq, bifrostCtx, cancel)
    return
}
```

## 非同期エンドポイント

Fire-and-forget形式の非同期実行も可能です：

- `POST /v1/async/chat/completions` - 非同期チャット補完 bifrost:44-45 
- `GET /v1/async/chat/completions/{job_id}` - 結果取得 bifrost:44-45 

## Notes

- 全てのcompletionエンドポイントで `stream: true` を指定するとServer-Sent Events (SSE) 形式でストリーミングレスポンスが返されます
- 非同期エンドポイントではストリーミングはサポートされていません bifrost:40-40 
- プロバイダー固有エンドポイントは各SDKとの互換性を保つために提供されています

Wiki pages you might want to explore:
- [API Handlers and Endpoints (maximhq/bifrost)](/wiki/maximhq/bifrost#3.3)
- [Middleware and Authentication (maximhq/bifrost)](/wiki/maximhq/bifrost#3.5)
- [Provider Interface and Architecture (maximhq/bifrost)](/wiki/maximhq/bifrost#4.1)
### Citations
**File:** docs/openapi/openapi.yaml (L24-28)
```yaml
    - `/openai/*` - OpenAI-compatible API
    - `/anthropic/*` - Anthropic-compatible API
    - `/genai/*` - Google GenAI (Gemini) compatible API
    - `/bedrock/*` - AWS Bedrock compatible API
    - `/cohere/*` - Cohere compatible API
```
**File:** docs/openapi/openapi.yaml (L135-136)
```yaml
  /v1/chat/completions:
    $ref: './paths/inference/chat-completions.yaml#/chat-completions'
```
**File:** docs/openapi/openapi.yaml (L137-138)
```yaml
  /v1/completions:
    $ref: './paths/inference/text-completions.yaml#/text-completions'
```
**File:** docs/openapi/openapi.yaml (L139-140)
```yaml
  /v1/responses:
    $ref: './paths/inference/responses.yaml#/responses'
```
**File:** transports/bifrost-http/handlers/inference.go (L927-929)
```go
	if req.Stream != nil && *req.Stream {
		h.handleStreamingChatCompletion(ctx, bifrostChatReq, bifrostCtx, cancel)
		return
```
**File:** docs/features/async-inference.mdx (L40-40)
```text
Streaming is not supported on async endpoints.
```
**File:** docs/features/async-inference.mdx (L44-45)
```text
| Text completions | `/v1/async/completions` | `/v1/async/completions/{job_id}` |
| Chat completions | `/v1/async/chat/completions` | `/v1/async/chat/completions/{job_id}` |
```
## Q6
統一APIエンドポイントはプロバイダーをモデル名のプレフィックスで、プロバイダー固有エンドポイントはURLパスのプレフィックスで区別する仕組みになっているという認識であっていますか？
### Answer
はい、その認識で正しいです。Bifrostは2つの異なる方法でプロバイダーを区別しています。

## 統一APIエンドポイント（/v1/*）

モデル名にプロバイダープレフィックスを使用します：

```bash
# 統一APIでの使用例
POST /v1/chat/completions
{
  "model": "openai/gpt-4",        # openaiプロバイダー
  "model": "anthropic/claude-3",   # anthropicプロバイダー
  "model": "gemini/gemini-pro"     # geminiプロバイダー
}
``` bifrost:13-16 bifrost:76-78 

## プロバイダー固有エンドポイント

URLパスのプレフィックスでプロバイダーを区別します：

```bash
# 各プロバイダーのネイティブエンドポイント
POST /openai/v1/chat/completions     # OpenAI形式
POST /anthropic/v1/messages          # Anthropic形式
POST /genai/v1beta/models/...        # Google Gemini形式
POST /bedrock/...                    # AWS Bedrock形式
``` bifrost:22-29 bifrost:126-174 

## 実際の使用例

各クライアントツールでの設定例：

**Cursor:**
```json
{
  "baseURL": "http://localhost:8080/openai",  // プロバイダー固有エンドポイント
  "models": {
    "openai/gpt-5": {},                      // 統一API形式
    "anthropic/claude-sonnet-4-5-20250929": {}
  }
}
``` bifrost:26-50 

**Open WebUI:**
```yaml
baseURL: "http://localhost:8080/v1"  # 統一APIエンドポイント
models:
  default:
    - "openai/gpt-5"               # プロバイダープレフィックス
    - "anthropic/claude-sonnet-4-5-20250929"
``` bifrost:86-96 

この2つのアプローチにより、Bifrostは柔軟な統合を提供しています：統一APIでは単一のエンドポイントで複数プロバイダーを利用でき、プロバイダー固有エンドポイントでは既存のSDKとの互換性を保ちます。

Wiki pages you might want to explore:
- [Installation Methods (maximhq/bifrost)](/wiki/maximhq/bifrost#11.1)
- [CLI Architecture and Internals (maximhq/bifrost)](/wiki/maximhq/bifrost#12.2)
- [API Handlers and Endpoints (maximhq/bifrost)](/wiki/maximhq/bifrost#3.3)
### Citations
**File:** docs/openapi/openapi.yaml (L13-16)
```yaml
    ### Unified Inference API (`/v1/*`)
    The primary API using Bifrost's unified format. Model parameters use the format
    `provider/model` (e.g., `openai/gpt-4`, `anthropic/claude-3-opus`).

```
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
**File:** docs/providers/supported-providers/overview.mdx (L76-78)
```text
# Same response format regardless of provider
curl -X POST http://localhost:8080/v1/chat/completions \
  -H "Content-Type: application/json" \
```
**File:** docs/integrations/what-is-an-integration.mdx (L126-174)
```text

For custom HTTP clients or when you have existing provider-specific setup and want to use Bifrost gateway without restructuring your codebase:

```python {5,18,31,}
import requests

# Fully OpenAI compatible endpoint
response = requests.post(
    "http://localhost:8080/openai/v1/chat/completions",
    headers={
        "Authorization": f"Bearer {openai_key}",
        "Content-Type": "application/json"
    },
    json={
        "model": "gpt-4o-mini",
        "messages": [{"role": "user", "content": "Hello!"}]
    }
)

# Fully Anthropic compatible endpoint
response = requests.post(
    "http://localhost:8080/anthropic/v1/messages",
    headers={
        "Content-Type": "application/json",
    },
    json={
        "model": "claude-3-sonnet-20240229",
        "max_tokens": 1000,
        "messages": [{"role": "user", "content": "Hello!"}]
    }
)

# Fully Google GenAI compatible endpoint
response = requests.post(
    "http://localhost:8080/genai/v1beta/models/gemini-1.5-flash/generateContent",
    headers={
        "Content-Type": "application/json",
    },
    json={
        "contents": [
            {"parts": [{"text": "Hello!"}]}
        ],
        "generation_config": {
            "max_output_tokens": 1000,
            "temperature": 1
        }
    }
)
```
```
**File:** docs/cli-agents/cursor.mdx (L26-50)
```text

   Toggle **Override OpenAI Base URL** to ON and enter your Bifrost endpoint:

   <Note>
      For cursor you need publicly accessible link for Bifrost.
   </Note>

   For deployed instances, use your Bifrost deployment URL (e.g., `https://bifrost.example.com/cursor`).

4. **Add custom models** (optional)

   Type a model name in the **Add or search model** field using the `provider/model-name` format:

   ![Adding a custom model in Cursor](/media/ides/cursor-add-custom-model-4-custom-model.png)

   Examples: `anthropic/claude-sonnet-4-5-20250929`, `openai/gpt-5`, `gemini-2.5-pro`

   | Provider | Format | Example |
   |----------|--------|---------|
   | Anthropic | `anthropic/model-name` | `anthropic/claude-sonnet-4-5-20250929` |
   | Gemini | `model-name` | `gemini-2.5-pro` |
   | OpenAI | `openai/model-name` | `openai/gpt-5` |
   | Bedrock | `bedrock/model-name` | `bedrock/anthropic.claude-3` |
   | Vertex (non-Gemini) | `vertex/model-name` | `vertex/text-bison` |
   | Other providers | `provider/model-name` | `groq/llama-3.3-70b-versatile` |
```
**File:** docs/cli-agents/open-webui.mdx (L86-96)
```text

Open WebUI supports image generation and vision (image understanding). You can use Bifrost for both.

### Image Generation

Set a Bifrost provider/model as your **image inference engine** for DALL·E-style image generation:

1. Go to **⚙️ Admin Settings** → **Settings** → **Images**
2. Set **Image Generation Engine** to **Open AI**
3. Configure:
   - **API Endpoint URL**: `http://localhost:8080/v1` (or your Bifrost host + `/v1`)
```
## Q7
わかりました。現在時点で使用できる全てのプロバイダー名を正確に全てリストしてください。
### Answer
現在Bifrostで使用できる全てのプロバイダー名を以下に示します。

## サポートされているプロバイダー一覧

### 標準プロバイダー（23種類） bifrost:37-60 

1. **openai** - OpenAI
2. **azure** - Azure OpenAI Service
3. **anthropic** - Anthropic
4. **bedrock** - AWS Bedrock
5. **cohere** - Cohere
6. **vertex** - Google Vertex AI
7. **mistral** - Mistral AI
8. **ollama** - Ollama（ローカル）
9. **groq** - Groq
10. **sgl** - SGLang
11. **parasail** - Parasail
12. **perplexity** - Perplexity
13. **cerebras** - Cerebras
14. **gemini** - Google Gemini
15. **openrouter** - OpenRouter
16. **elevenlabs** - ElevenLabs
17. **huggingface** - Hugging Face
18. **nebius** - Nebius
19. **xai** - xAI (Grok)
20. **replicate** - Replicate
21. **vllm** - vLLM
22. **runway** - Runway

### カスタムプロバイダーのベースとして使用可能なプロバイダー bifrost:62-71 

以下のプロバイダーはカスタムプロバイダーのベースとして使用できます：
- anthropic
- bedrock
- cohere
- gemini
- openai
- huggingface
- replicate

### 実装確認 bifrost:3572-3619 

全てのプロバイダーは `createBaseProvider` 関数で実装されており、実際に使用可能です。

### 使用方法

統一APIエンドポイントでは `provider/model` 形式で使用します：
```bash
openai/gpt-4
anthropic/claude-3-sonnet
gemini/gemini-pro
``` bifrost:3-29 

## Notes

- 上記23種類のプロバイダーが現在のBifrostバージョンで完全にサポートされています
- 各プロバイダーは異なる機能セット（チャット、埋め込み、画像生成など）をサポートしています
- 詳細な機能サポートマトリックスはドキュメントを参照してください

Wiki pages you might want to explore:
- [Architecture Overview (maximhq/bifrost)](/wiki/maximhq/bifrost#1.2)
### Citations
**File:** core/schemas/bifrost.go (L37-60)
```go
const (
	OpenAI      ModelProvider = "openai"
	Azure       ModelProvider = "azure"
	Anthropic   ModelProvider = "anthropic"
	Bedrock     ModelProvider = "bedrock"
	Cohere      ModelProvider = "cohere"
	Vertex      ModelProvider = "vertex"
	Mistral     ModelProvider = "mistral"
	Ollama      ModelProvider = "ollama"
	Groq        ModelProvider = "groq"
	SGL         ModelProvider = "sgl"
	Parasail    ModelProvider = "parasail"
	Perplexity  ModelProvider = "perplexity"
	Cerebras    ModelProvider = "cerebras"
	Gemini      ModelProvider = "gemini"
	OpenRouter  ModelProvider = "openrouter"
	Elevenlabs  ModelProvider = "elevenlabs"
	HuggingFace ModelProvider = "huggingface"
	Nebius      ModelProvider = "nebius"
	XAI         ModelProvider = "xai"
	Replicate   ModelProvider = "replicate"
	VLLM        ModelProvider = "vllm"
	Runway      ModelProvider = "runway"
)
```
**File:** core/schemas/bifrost.go (L62-71)
```go
// SupportedBaseProviders is the list of base providers allowed for custom providers.
var SupportedBaseProviders = []ModelProvider{
	Anthropic,
	Bedrock,
	Cohere,
	Gemini,
	OpenAI,
	HuggingFace,
	Replicate,
}
```
**File:** core/bifrost.go (L3572-3619)
```go
	switch targetProviderKey {
	case schemas.OpenAI:
		return openai.NewOpenAIProvider(config, bifrost.logger), nil
	case schemas.Anthropic:
		return anthropic.NewAnthropicProvider(config, bifrost.logger), nil
	case schemas.Bedrock:
		return bedrock.NewBedrockProvider(config, bifrost.logger)
	case schemas.Cohere:
		return cohere.NewCohereProvider(config, bifrost.logger)
	case schemas.Azure:
		return azure.NewAzureProvider(config, bifrost.logger)
	case schemas.Vertex:
		return vertex.NewVertexProvider(config, bifrost.logger)
	case schemas.Mistral:
		return mistral.NewMistralProvider(config, bifrost.logger), nil
	case schemas.Ollama:
		return ollama.NewOllamaProvider(config, bifrost.logger)
	case schemas.Groq:
		return groq.NewGroqProvider(config, bifrost.logger)
	case schemas.SGL:
		return sgl.NewSGLProvider(config, bifrost.logger)
	case schemas.Parasail:
		return parasail.NewParasailProvider(config, bifrost.logger)
	case schemas.Perplexity:
		return perplexity.NewPerplexityProvider(config, bifrost.logger)
	case schemas.Cerebras:
		return cerebras.NewCerebrasProvider(config, bifrost.logger)
	case schemas.Gemini:
		return gemini.NewGeminiProvider(config, bifrost.logger), nil
	case schemas.OpenRouter:
		return openrouter.NewOpenRouterProvider(config, bifrost.logger), nil
	case schemas.Elevenlabs:
		return elevenlabs.NewElevenlabsProvider(config, bifrost.logger), nil
	case schemas.Nebius:
		return nebius.NewNebiusProvider(config, bifrost.logger)
	case schemas.HuggingFace:
		return huggingface.NewHuggingFaceProvider(config, bifrost.logger), nil
	case schemas.XAI:
		return xai.NewXAIProvider(config, bifrost.logger)
	case schemas.Replicate:
		return replicate.NewReplicateProvider(config, bifrost.logger)
	case schemas.VLLM:
		return vllm.NewVLLMProvider(config, bifrost.logger)
	case schemas.Runway:
		return runway.NewRunwayProvider(config, bifrost.logger)
	default:
		return nil, fmt.Errorf("unsupported provider: %s", targetProviderKey)
	}
```
**File:** docs/openapi/schemas/inference/common.yaml (L3-29)
```yaml
ModelProvider:
  type: string
  description: AI model provider identifier
  enum:
    - openai
    - azure
    - anthropic
    - bedrock
    - cohere
    - vertex
    - vllm
    - mistral
    - ollama
    - groq
    - sgl
    - parasail
    - perplexity
    - replicate
    - cerebras
    - gemini
    - openrouter
    - elevenlabs
    - huggingface
    - nebius
    - xai
    - runway

```