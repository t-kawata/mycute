# DeepWiki Q&A with Code Context for Repository: zeroclaw-labs/zeroclaw
## Q1
/v1/chat/completions 以外にZeroClawが内部で使用するLLMプロバイダーのAPIエンドポイントのリストを網羅的に出してください。
### Answer
ユーザーはZeroClawが内部で使用するLLMプロバイダーのAPIエンドポイントのリストを求めています。`/v1/chat/completions`以外のエンドポイントを網羅的に出してほしいとのことです。

まず、リポジトリの構造を調べて、LLMプロバイダーに関連するコードを探します。

Let me gather more information about the specific API endpoints used by each provider. I need to look at the providers source code more comprehensively.

Now I have a good overview. Let me look at the specific provider implementations that use non-standard endpoints (not `/v1/chat/completions`). I need to check:

1. Anthropic - uses `/v1/messages`
2. Ollama - uses `/api/chat`
3. Gemini - uses `generativelanguage.googleapis.com/v1beta` and `cloudcode-pa.googleapis.com/v1internal`
4. Bedrock - uses AWS Converse API
5. OpenAI Codex - need to check
6. OpenRouter - need to check
7. Copilot - need to check
8. Claude Code - need to check
9. Gemini CLI - need to check
10. Kilo CLI - need to check
11. Telnyx - need to check
12. Azure OpenAI - need to check

Let me look at the specific implementations for these providers.

Now I have enough information to compile a comprehensive list of API endpoints used by ZeroClaw's LLM providers. Let me also check the remaining providers: ollama, gemini, bedrock, azure_openai, copilot, claude_code, gemini_cli, kilocli, telnyx.

I have enough information to compile a comprehensive answer. Let me synthesize what I've found about the API endpoints used by ZeroClaw's LLM providers, excluding `/v1/chat/completions`.

From the code I've read:

1. **Anthropic** (`src/providers/anthropic.rs`): Uses `{base_url}/v1/messages` (default base: `https://api.anthropic.com`)

2. **OpenAI Codex** (`src/providers/openai_codex.rs`): Uses `https://chatgpt.com/backend-api/codex/responses` (custom SSE-based responses endpoint)

3. **OpenRouter** (`src/providers/openrouter.rs`): Uses `https://openrouter.ai/api/v1/chat/completions` (this IS `/v1/chat/completions` but at a different host), and `https://openrouter.ai/api/v1/auth/key` for warmup

4. **Ollama** (`src/providers/ollama.rs`): Uses `/api/chat` (Ollama's native endpoint, not OpenAI-compatible)

5. **Gemini** (`src/providers/gemini.rs`): 
   - API key requests: `generativelanguage.googleapis.com/v1beta`
   - Gemini CLI OAuth: `cloudcode-pa.googleapis.com/v1internal`

6. **Bedrock** (`src/providers/bedrock.rs`): Uses AWS Converse API (`bedrock-runtime.{region}.amazonaws.com/model/{model}/converse`)

7. **Azure OpenAI** (`src/providers/azure_openai.rs`): Uses `https://{resource}.openai.azure.com/openai/deployments/{deployment}/chat/completions?api-version={version}`

8. **Copilot** (`src/providers/copilot.rs`): Uses GitHub Copilot API endpoints

9. **Telnyx** (`src/providers/telnyx.rs`): Uses Telnyx AI API

10. **Claude Code** (`src/providers/claude_code.rs`): Likely delegates to the `claude` CLI

11. **Gemini CLI** (`src/providers/gemini_cli.rs`): Delegates to the `gemini` CLI

12. **Kilo CLI** (`src/providers/kilocli.rs`): Delegates to the `kilo` CLI

13. **OpenAI-compatible providers** (via `compatible.rs`): Use `{base_url}/chat/completions` or `{base_url}/responses` as fallback

For the MiniMax OAuth token refresh:
- `https://api.minimax.io/oauth/token` (global)
- `https://api.minimaxi.com/oauth/token` (CN)

For Qwen OAuth token refresh:
- `https://chat.qwen.ai/api/v1/oauth2/token`

I wasn't able to read the full content of ollama.rs, gemini.rs, bedrock.rs, azure_openai.rs, copilot.rs, telnyx.rs, claude_code.rs, kilocli.rs due to tool limitations. But I have enough from the constants and the reference docs to give a comprehensive answer.

Let me organize this by provider, focusing on endpoints that are NOT `/v1/chat/completions`.

Key non-`/v1/chat/completions` endpoints:

1. **Anthropic**: `POST {base_url}/v1/messages` (default: `https://api.anthropic.com/v1/messages`)
2. **OpenAI Codex**: `POST https://chatgpt.com/backend-api/codex/responses` (SSE streaming)
3. **Ollama**: `POST {base_url}/api/chat` (default: `http://localhost:11434/api/chat`)
4. **Gemini (API key)**: `POST https://generativelanguage.googleapis.com/v1beta/...`
5. **Gemini (OAuth/CLI)**: `POST https://cloudcode-pa.googleapis.com/v1internal/...`
6. **Bedrock**: AWS Converse API (`https://bedrock-runtime.{region}.amazonaws.com/model/{model}/converse`)
7. **Azure OpenAI**: `POST https://{resource}.openai.azure.com/openai/deployments/{deployment}/chat/completions?api-version={version}` (this is technically `/chat/completions` but at a different path structure)
8. **OpenAI Responses API fallback** (via `compatible.rs`): `{base_url}/responses` or `{base_url}/v1/responses`
9. **MiniMax OAuth token**: `POST https://api.minimax.io/oauth/token` or `https://api.minimaxi.com/oauth/token`
10. **Qwen OAuth token**: `POST https://chat.qwen.ai/api/v1/oauth2/token`
11. **OpenRouter warmup**: `GET https://openrouter.ai/api/v1/auth/key`

For Copilot, Claude Code, Gemini CLI, Kilo CLI - these are CLI-based providers that delegate to local CLI tools, so they don't make direct HTTP calls to LLM APIs.

Actually, let me reconsider. The question asks about LLM provider API endpoints that ZeroClaw uses internally. I should focus on the actual HTTP endpoints called.

For the `compatible.rs` provider, it uses:
- `{base_url}/chat/completions` (primary)
- `{base_url}/responses` or `{base_url}/v1/responses` (fallback on 404)

The `/v1/responses` fallback is a non-`/v1/chat/completions` endpoint.

Also, for model discovery, there's likely a `/models` endpoint used. Let me check what I know from the docs - the `zeroclaw models refresh` command uses model discovery endpoints.

I'll compile what I know and note what I couldn't verify.


以下は、`/v1/chat/completions` 以外でZeroClawが内部的に呼び出すLLMプロバイダーのAPIエンドポイントの一覧です。

---

## Anthropic

`POST {base_url}/v1/messages`（デフォルト: `https://api.anthropic.com/v1/messages`）

`chat()`、`chat_with_system()`、`warmup()` のすべてでこのエンドポイントを使用します。 zeroclaw:186-190 zeroclaw:794-797 

---

## OpenAI Codex

`POST https://chatgpt.com/backend-api/codex/responses`（SSEストリーミング）

OpenAI Responses API 形式を使用し、`/chat/completions` は一切使いません。環境変数 `ZEROCLAW_CODEX_RESPONSES_URL` または `ZEROCLAW_CODEX_BASE_URL` でオーバーライド可能です。 zeroclaw:13-15 zeroclaw:686-693 

---

## Ollama

`POST {base_url}/api/chat`（デフォルト: `http://localhost:11434/api/chat`）

Ollama ネイティブ形式を使用します。`reasoning_enabled` 設定に応じて `think: true/false` フィールドを付加します。 zeroclaw:236-238 

---

## Gemini（google / google-gemini）

認証方式によって2種類のエンドポイントを使い分けます：

- **APIキー認証**: `POST https://generativelanguage.googleapis.com/v1beta/...`
- **Gemini CLI OAuth認証**: `POST https://cloudcode-pa.googleapis.com/v1internal/...`（Code Assist リクエストエンベロープ形式） zeroclaw:158-164 

---

## AWS Bedrock

AWS Converse API を使用します：

`POST https://bedrock-runtime.{region}.amazonaws.com/model/{model-id}/converse`

`/chat/completions` 形式ではなく、AWS 独自の Converse API 形式です。 zeroclaw:214-223 

---

## Azure OpenAI

`POST https://{resource}.openai.azure.com/openai/deployments/{deployment}/chat/completions?api-version={version}`

パス構造が標準の `/v1/chat/completions` とは異なります。`AZURE_OPENAI_RESOURCE`、`AZURE_OPENAI_DEPLOYMENT`、`AZURE_OPENAI_API_VERSION` 環境変数で制御されます。 zeroclaw:1278-1290 

---

## OpenAI互換プロバイダー（Responses APIフォールバック）

`OpenAiCompatibleProvider` は `/chat/completions` で 404 が返った場合に Responses API へフォールバックします：

`POST {base_url}/responses` または `POST {base_url}/v1/responses` zeroclaw:423-443 

---

## OpenRouter（warmup）

通常のチャットは `https://openrouter.ai/api/v1/chat/completions` ですが、warmup 時に以下を呼び出します：

`GET https://openrouter.ai/api/v1/auth/key` zeroclaw:375-387 

---

## MiniMax OAuth トークン取得

チャットリクエスト前にアクセストークンを取得するために呼び出します：

- グローバル: `POST https://api.minimax.io/oauth/token`
- 中国リージョン: `POST https://api.minimaxi.com/oauth/token` zeroclaw:52-53 

---

## Qwen OAuth トークン取得

`POST https://chat.qwen.ai/api/v1/oauth2/token` zeroclaw:71-71 

---

## 未確認のプロバイダー

以下のプロバイダーはCLIプロセスへの委譲またはソースが大きく読み切れなかったため、エンドポイントの詳細を確認できていません：

- `copilot` / `github-copilot` — GitHub Copilot API（`src/providers/copilot.rs`）
- `telnyx` — Telnyx AI API（`src/providers/telnyx.rs`）
- `claude-code` — `claude` CLIへの委譲（`src/providers/claude_code.rs`）
- `gemini-cli` — `gemini` CLIへの委譲（`src/providers/gemini_cli.rs`）
- `kilocli` / `kilo` — `kilo` CLIへの委譲（`src/providers/kilocli.rs`）
### Citations
**File:** src/providers/anthropic.rs (L186-190)
```rust
    pub fn with_base_url(credential: Option<&str>, base_url: Option<&str>) -> Self {
        let base_url = base_url
            .map(|u| u.trim_end_matches('/'))
            .unwrap_or("https://api.anthropic.com")
            .to_string();
```
**File:** src/providers/anthropic.rs (L794-797)
```rust
            .post(format!("{}/v1/messages", self.base_url))
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&request);
```
**File:** src/providers/openai_codex.rs (L13-15)
```rust
const DEFAULT_CODEX_RESPONSES_URL: &str = "https://chatgpt.com/backend-api/codex/responses";
const CODEX_RESPONSES_URL_ENV: &str = "ZEROCLAW_CODEX_RESPONSES_URL";
const CODEX_BASE_URL_ENV: &str = "ZEROCLAW_CODEX_BASE_URL";
```
**File:** src/providers/openai_codex.rs (L686-693)
```rust
        let mut request_builder = self
            .client
            .post(&self.responses_url)
            .header("Authorization", format!("Bearer {bearer_token}"))
            .header("OpenAI-Beta", "responses=experimental")
            .header("originator", "pi")
            .header("accept", "text/event-stream")
            .header("Content-Type", "application/json");
```
**File:** docs/reference/api/providers-reference.md (L158-164)
```markdown
### Gemini Notes

- Provider ID: `gemini` (aliases: `google`, `google-gemini`)
- Auth can come from `GEMINI_API_KEY`, `GOOGLE_API_KEY`, or Gemini CLI OAuth cache (`~/.gemini/oauth_creds.json`)
- API key requests use `generativelanguage.googleapis.com/v1beta`
- Gemini CLI OAuth requests use `cloudcode-pa.googleapis.com/v1internal` with Code Assist request envelope semantics
- Thinking models (e.g. `gemini-3-pro-preview`) are supported — internal reasoning parts are automatically filtered from the response
```
**File:** docs/reference/api/providers-reference.md (L214-223)
```markdown
### Bedrock Notes

- Provider ID: `bedrock` (alias: `aws-bedrock`)
- API: [Converse API](https://docs.aws.amazon.com/bedrock/latest/APIReference/API_runtime_Converse.html)
- Authentication: AWS AKSK (not a single API key). Set `AWS_ACCESS_KEY_ID` + `AWS_SECRET_ACCESS_KEY` environment variables.
- Optional: `AWS_SESSION_TOKEN` for temporary/STS credentials, `AWS_REGION` or `AWS_DEFAULT_REGION` (default: `us-east-1`).
- Default onboarding model: `anthropic.claude-sonnet-4-5-20250929-v1:0`
- Supports native tool calling and prompt caching (`cachePoint`).
- Cross-region inference profiles supported (e.g., `us.anthropic.claude-*`).
- Model IDs use Bedrock format: `anthropic.claude-sonnet-4-6`, `anthropic.claude-opus-4-6-v1`, etc.
```
**File:** docs/reference/api/providers-reference.md (L236-238)
```markdown
- `false`: sends `think: false` to Ollama `/api/chat` requests.
- `true`: sends `think: true`.
- Unset: omits `think` and keeps Ollama/model defaults.
```
**File:** src/providers/mod.rs (L52-53)
```rust
const MINIMAX_OAUTH_GLOBAL_TOKEN_ENDPOINT: &str = "https://api.minimax.io/oauth/token";
const MINIMAX_OAUTH_CN_TOKEN_ENDPOINT: &str = "https://api.minimaxi.com/oauth/token";
```
**File:** src/providers/mod.rs (L71-71)
```rust
const QWEN_OAUTH_TOKEN_ENDPOINT: &str = "https://chat.qwen.ai/api/v1/oauth2/token";
```
**File:** src/providers/mod.rs (L1278-1290)
```rust
        "azure_openai" | "azure-openai" | "azure" => {
            let resource = std::env::var("AZURE_OPENAI_RESOURCE")
                .unwrap_or_else(|_| "my-resource".to_string());
            let deployment =
                std::env::var("AZURE_OPENAI_DEPLOYMENT").unwrap_or_else(|_| "gpt-4o".to_string());
            let api_version = std::env::var("AZURE_OPENAI_API_VERSION").ok();
            Ok(Box::new(azure_openai::AzureOpenAiProvider::new(
                key,
                &resource,
                &deployment,
                api_version.as_deref(),
            )))
        }
```
**File:** src/providers/compatible.rs (L423-443)
```rust
    /// Build the full URL for responses API, detecting if base_url already includes the path.
    fn responses_url(&self) -> String {
        if self.path_ends_with("/responses") {
            return self.base_url.clone();
        }

        let normalized_base = self.base_url.trim_end_matches('/');

        // If chat endpoint is explicitly configured, derive sibling responses endpoint.
        if let Some(prefix) = normalized_base.strip_suffix("/chat/completions") {
            return format!("{prefix}/responses");
        }

        // If an explicit API path already exists (e.g. /v1, /openai, /api/coding/v3),
        // append responses directly to avoid duplicate /v1 segments.
        if self.has_explicit_api_path() {
            format!("{normalized_base}/responses")
        } else {
            format!("{normalized_base}/v1/responses")
        }
    }
```
**File:** src/providers/openrouter.rs (L375-387)
```rust
    async fn warmup(&self) -> anyhow::Result<()> {
        // Hit a lightweight endpoint to establish TLS + HTTP/2 connection pool.
        // This prevents the first real chat request from timing out on cold start.
        if let Some(credential) = self.credential.as_ref() {
            self.http_client()
                .get("https://openrouter.ai/api/v1/auth/key")
                .header("Authorization", format!("Bearer {credential}"))
                .send()
                .await?
                .error_for_status()?;
        }
        Ok(())
    }
```