//! LMGW専用 LLM クライアント。
//!
//! `async-openai` を内部エンジンとして使用し、ローカルの LMGW (Bifrost Proxy) に対して
//! テキスト補正・要約などの completions 系リクエストを送る。
//!
//! # 設計方針
//! - `LmgwClient` は「LMGW というローカルゲートウェイを知っている」ことだけを責務として持つ。
//! - プロバイダーの管理・負荷分散・APIキー管理は全て LMGW (Bifrost) 側に委任する。
//! - OpenAI の仕様変更への追随コストは `async-openai` クレートへ外部委託する。

use crate::constants::{IP_LOCALHOST, LLM_MIMICRY_DELAY_MS, PATH_LMGW_OPENAI_V1};
use crate::mycute_settings::LocaleCode;
use crate::stt::stats::UsageStats;
use crate::types::LlmAction;
use crate::utils::time::sleep_ms;
use async_openai::{
    config::OpenAIConfig,
    types::chat::{
        ChatCompletionRequestMessage, ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs,
    },
    Client as OpenAIClient,
};

/// LMGW (Bifrost Proxy) への専用クライアント。
///
/// 内部に `async-openai` クライアントを保持し、ベースURLを LMGW に固定する。
/// 呼び出し元は LMGW の存在を意識せず、補正・要約のメソッドを呼ぶだけでよい。
pub struct LmgwClient {
    /// 内部の async-openai クライアント（ベースURLが LMGW に設定済み）
    inner: OpenAIClient<OpenAIConfig>,
    /// LMGWへのリクエストに使用するモデル名
    model: String,
    /// RT サーバーのポート番号（ASRバックエンドが独自の OpenAIClient を作成する際に参照する）
    rt_port: u16,
    /// 内部通信用 JWT（ASRバックエンドが独自の OpenAIClient を作成する際に参照する）
    jwt_token: String,
}

impl LmgwClient {
    /// 新しい LmgwClient を作成する。
    ///
    /// # 引数
    /// - `rt_port`: RT サーバーのポート番号。LMGW のベースURLを組み立てるのに使用する。
    /// - `jwt`: RT サーバーとの内部通信に使用する JWT トークン。
    /// - `model`: 使用するモデル名（例: `"gpt-4o-mini"`）。Bifrost 経由で解決される。
    pub fn new(rt_port: u16, jwt: &str, model: &str) -> Self {
        // LMGW の completions エンドポイントをベースURLとして設定する。
        // async-openai は このURL配下に `/chat/completions` 等のパスを自動付与する。
        let base_url = format!(
            "http://{}:{}{}",
            IP_LOCALHOST,
            rt_port,
            PATH_LMGW_OPENAI_V1
        );

        let config = OpenAIConfig::new()
            .with_api_base(base_url)
            .with_api_key(jwt);

        Self {
            inner: OpenAIClient::with_config(config),
            model: model.to_string(),
            rt_port,
            jwt_token: jwt.to_string(),
        }
    }

    /// RT サーバーのポート番号を返す。
    ///
    /// `OpenAIBackend` が ASR 専用の `async-openai` クライアントを組み立てる際に参照する。
    pub fn base_url_port(&self) -> u16 {
        self.rt_port
    }

    /// 内部通信用 JWT を返す。
    ///
    /// `OpenAIBackend` が ASR 専用の `async-openai` クライアントを組み立てる際に参照する。
    pub fn jwt(&self) -> &str {
        &self.jwt_token
    }

    /// テキストを書き言葉として補正する。
    ///
    /// LMGW 経由で chat completions API を呼び出し、結果から XML タグを抽出して返す。
    /// トークン使用量は自動的に `UsageStats` に記録される。
    /// LMGW が未起動などで通信に失敗した場合は元のテキストをそのまま返す（模擬モード）。
    pub async fn correct_text(&self, text: &str, locale: LocaleCode) -> Result<String, String> {
        let system_prompt = if locale == LocaleCode::En {
            crate::llm::prompts::SYSTEM_PROMPT_EN
        } else {
            crate::llm::prompts::SYSTEM_PROMPT_JA
        };

        let user_content = if locale == LocaleCode::En {
            format!(
                "Please correct the following text:\n<text>\n{}\n</text>",
                text
            )
        } else {
            format!(
                "以下のテキストを補正してください：\n<text>\n{}\n</text>",
                text
            )
        };

        self.call_completions(system_prompt, &user_content).await
    }

    /// テキストを要約し、Markdown形式で再構成する。
    ///
    /// LMGW 経由で chat completions API を呼び出し、結果から XML タグを抽出して返す。
    /// トークン使用量は自動的に `UsageStats` に記録される。
    /// LMGW が未起動などで通信に失敗した場合は元のテキストをそのまま返す（模擬モード）。
    pub async fn summarize_text(&self, text: &str, locale: LocaleCode) -> Result<String, String> {
        let system_prompt = if locale == LocaleCode::En {
            crate::llm::prompts::SYSTEM_PROMPT_SUMMARIZE_EN
        } else {
            crate::llm::prompts::SYSTEM_PROMPT_SUMMARIZE_JA
        };

        let user_content = if locale == LocaleCode::En {
            format!(
                "Please summarize and restructure the following text:\n<text>\n{}\n</text>",
                text
            )
        } else {
            format!(
                "以下のテキストを要約・再構成してください：\n<text>\n{}\n</text>",
                text
            )
        };

        self.call_completions(system_prompt, &user_content).await
    }

    /// アクションの種類に応じて補正または要約を実行する。
    pub async fn execute(
        &self,
        action: LlmAction,
        text: &str,
        locale: LocaleCode,
    ) -> Result<String, String> {
        match action {
            LlmAction::Correct => self.correct_text(text, locale).await,
            LlmAction::Summarize => self.summarize_text(text, locale).await,
        }
    }

    /// LMGW の chat completions エンドポイントに対してリクエストを送信する内部メソッド。
    ///
    /// # 処理の流れ
    /// 1. システムプロンプトとユーザーメッセージからリクエストを組み立てる。
    /// 2. `async-openai` クライアントを使ってリクエストを送信する。
    /// 3. レスポンスの `usage` フィールドを確認し、`UsageStats::record_llm` に記録する。
    /// 4. 最初の choice のテキストを取り出し、XMLタグ抽出を経て返す。
    async fn call_completions(
        &self,
        system_prompt: &str,
        user_content: &str,
    ) -> Result<String, String> {
        let messages: Vec<ChatCompletionRequestMessage> = vec![
            ChatCompletionRequestSystemMessageArgs::default()
                .content(system_prompt)
                .build()
                .map_err(|e| format!("Failed to build system message: {}", e))?
                .into(),
            ChatCompletionRequestUserMessageArgs::default()
                .content(user_content)
                .build()
                .map_err(|e| format!("Failed to build user message: {}", e))?
                .into(),
        ];

        let request = CreateChatCompletionRequestArgs::default()
            .model(&self.model)
            .messages(messages)
            .build()
            .map_err(|e| format!("Failed to build completion request: {}", e))?;

        log::debug!("[LmgwClient] Sending chat completion request to LMGW.");

        let response = self.inner.chat().create(request).await.map_err(|e| {
            // LMGW未起動などの接続失敗は一定の遅延の後に模擬モードで処理を継続させる
            log::warn!("[LmgwClient] LMGW request failed (Mimicry Mode): {}", e);
            e.to_string()
        })?;

        // トークン使用量を UsageStats に記録する（LMGW側の集計とは独立して行う）
        if let Some(usage) = &response.usage {
            if let Err(e) = UsageStats::record_llm(
                &response.model,
                usage.prompt_tokens as u64,
                usage.completion_tokens as u64,
            ) {
                log::error!("[LmgwClient] Failed to record LLM usage stats: {}", e);
            }
        }

        // 最初の choice からテキストを取り出す
        let raw_text = response
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message.content)
            .ok_or_else(|| {
                "No completion choices returned or unexpected response type".to_string()
            })?;

        log::debug!("[LmgwClient] Received response from LMGW.");
        Ok(Self::extract_result(&raw_text))
    }

    /// LMGW クライアントが利用可能か（接続先設定が存在するか）を確認する。
    ///
    /// 現在の設計では LMGW への接続情報は常に存在するため、常に `true` を返す。
    /// 将来的にヘルスチェックを行う場合はここに実装する。
    pub fn is_available(&self) -> bool {
        true
    }

    /// 模擬モード（LMGW未接続時）用のフォールバック処理。
    ///
    /// 意図的な遅延を挿入した上で元のテキストをそのまま返す。
    /// 呼び出し元のロジックを変えずにフォールバック動作を実現するために使用する。
    pub async fn fallback(text: &str) -> Result<String, String> {
        log::info!("[LmgwClient] LMGW unavailable. Falling back to mimicry mode.");
        sleep_ms(LLM_MIMICRY_DELAY_MS);
        Ok(text.to_string())
    }

    /// レスポンステキストの最初の XML 系タグペア内の内容を抽出する。
    ///
    /// LLM が `<result>補正後テキスト</result>` の形式で応答した場合、
    /// タグ内のテキストのみを取り出す。タグが存在しない場合は元のテキストを返す。
    ///
    /// # 例
    /// - `"Here is the result: <result>Corrected text</result>"` → `"Corrected text"`
    /// - `"No tags here"` → `"No tags here"`
    fn extract_result(text: &str) -> String {
        use once_cell::sync::Lazy;
        use regex::Regex;

        // Rust の regex クレートは後方参照（`\1`）をサポートしないため、
        // 最初の開きタグを見つけ、対応する閉じタグを手動で探す方式を採用する。
        static OPEN_TAG_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"<([a-zA-Z0-9_-]+)>").unwrap());

        if let Some(caps) = OPEN_TAG_RE.captures(text) {
            if let (Some(full_match), Some(tag_name)) = (caps.get(0), caps.get(1)) {
                let tag_name = tag_name.as_str();
                let opening_tag_end = full_match.end();
                let closing_tag = format!("</{}>", tag_name);

                if let Some(closing_tag_start) = text[opening_tag_end..].find(&closing_tag) {
                    let content_start = opening_tag_end;
                    let content_end = opening_tag_end + closing_tag_start;
                    let extracted = text[content_start..content_end].trim();

                    if !extracted.is_empty() {
                        // 入れ子になったタグがある場合はさらに除去する
                        // 例: `<result><text>...</text></result>` の内部タグも除去
                        static TAG_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"<[^>]+>").unwrap());
                        let cleaned = TAG_RE.replace_all(extracted, "").trim().to_string();
                        if !cleaned.is_empty() {
                            return cleaned;
                        }
                        return extracted.to_string();
                    }
                }
            }
        }
        text.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// extract_result メソッドの動作を検証するユニットテスト。
    /// LLM の応答形式（XMLタグありなし・入れ子など）に対する正確な動作を担保する。
    #[test]
    fn test_extract_result() -> Result<(), Box<dyn std::error::Error>> {
        // 推奨タグがある場合
        assert_eq!(
            LmgwClient::extract_result(
                "Here is the result: <result>Corrected text</result> some noise"
            ),
            "Corrected text"
        );

        // 異なるタグ名の場合
        assert_eq!(
            LmgwClient::extract_result("Alternative tag: <output>Some content</output>"),
            "Some content"
        );

        // タグが複数ある場合（最初のタグを優先）
        assert_eq!(
            LmgwClient::extract_result("<first>First</first> <second>Second</second>"),
            "First"
        );

        // タグが存在しない場合（そのまま返す）
        assert_eq!(
            LmgwClient::extract_result("No tags here at all"),
            "No tags here at all"
        );

        // タグが空（またはタグ除去後に空）の場合（そのまま返す）
        assert_eq!(
            LmgwClient::extract_result("Empty tags: <result></result>"),
            "Empty tags: <result></result>"
        );

        // 入れ子になったタグの場合
        assert_eq!(
            LmgwClient::extract_result("<result><text>Nested content</text></result>"),
            "Nested content"
        );

        // 複数の入れ子タグがある場合
        assert_eq!(
            LmgwClient::extract_result("<output><text>Multiple</text><note>Tags</note></output>"),
            "MultipleTags"
        );

        // 改行と入れ子タグを含む場合
        assert_eq!(
            LmgwClient::extract_result(
                "Prefix\n<result>\n  <text>\n    Indented and nested\n  </text>\n</result>"
            ),
            "Indented and nested"
        );

        Ok(())
    }
}
