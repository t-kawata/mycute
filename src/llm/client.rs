//! LLM API client for text correction and processing using swarms-rs.
//!
//! Supports multiple endpoints with round-robin distribution.

use parking_lot::RwLock;
use std::sync::atomic::{AtomicUsize, Ordering};

use swarms_rs::llm::completion::{AssistantContent, Message, Text};
use swarms_rs::llm::provider::openai::OpenAI;
use swarms_rs::llm::request::CompletionRequest;
use swarms_rs::llm::Model;

use crate::constants::{DUMMY_STRING, LLM_MIMICRY_DELAY_MS};
use crate::stt::stats::UsageStats;
use crate::stt_config::{LlmEndpoint, LocaleCode};
use crate::utils::time::{self, sleep_ms};

/// Single LLM client wrapping a swarms-rs agent.
pub struct LlmClient {
    pub name: String,
    model_name: String,
    base_url: String,
    api_key: String,
}

impl LlmClient {
    pub fn model_name(&self) -> &str {
        &self.model_name
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn api_key(&self) -> &str {
        &self.api_key
    }

    /// Check if the exact endpoint contains a dummy setting or is missing critical info.
    pub fn is_valid(&self) -> bool {
        let is_dummy = |s: &str| s.to_lowercase().contains(DUMMY_STRING);

        if is_dummy(&self.name)
            || is_dummy(&self.model_name)
            || is_dummy(&self.base_url)
            || is_dummy(&self.api_key)
        {
            return false;
        }

        if self.base_url.trim().is_empty() || self.api_key.trim().is_empty() {
            return false;
        }

        true
    }

    /// Create a new LLM client from endpoint config.
    pub fn from_config(endpoint: &LlmEndpoint) -> Self {
        Self {
            name: endpoint.name.clone(),
            model_name: endpoint.model.clone(),
            base_url: endpoint.base_url.clone(),
            api_key: endpoint.api_key.clone().unwrap_or_default(),
        }
    }

    /// Correct text using swarms-rs agent.
    pub async fn correct_text(&self, text: &str, locale: LocaleCode) -> Result<String, String> {
        // Build the OpenAI provider from URL and key
        let model = OpenAI::from_url(&self.base_url, &self.api_key).set_model(&self.model_name);

        // Use unified system prompt based on language
        let system_prompt = if locale == LocaleCode::En {
            crate::llm::prompts::SYSTEM_PROMPT_EN
        } else {
            crate::llm::prompts::SYSTEM_PROMPT_JA
        };

        // Create completion request with structural wrapping
        let (prefix, text_content) = if locale == LocaleCode::En {
            ("Please correct the following text:", text)
        } else {
            ("以下のテキストを補正してください：", text)
        };
        let user_prompt = format!("{}\n<text>\n{}\n</text>", prefix, text_content);
        let request = CompletionRequest {
            prompt: Message::user(user_prompt),
            system_prompt: Some(system_prompt.to_string()),
            chat_history: vec![],
            tools: vec![],
            temperature: None,
            max_tokens: None,
        };

        log::debug!(
            "[LLM] Using swarms-rs local completion for correction on endpoint: {}",
            self.name
        );

        // Execute completion directly to get usage info
        let response = model
            .completion(request)
            .await
            .map_err(|e| format!("swarms-rs model (correction) failed: {}", e))?;

        // Extract and record token usage
        if let Some(usage) = response.raw_response.usage {
            if let Err(e) = UsageStats::record_llm(
                &self.model_name,
                usage.prompt_tokens as u64,
                usage.completion_tokens as u64,
            ) {
                log::error!("[LLM] Failed to record statistics: {}", e);
            }
        }

        // Return the first choice content and extract text within XML tags if present
        let raw_text = response
            .choice
            .get(0)
            .and_then(|c| match c {
                AssistantContent::Text(Text { text }) => Some(text.clone()),
                _ => None,
            })
            .ok_or_else(|| {
                "No completion choices returned or unexpected response type".to_string()
            })?;

        Ok(self.extract_result(&raw_text))
    }

    /// Summarize and structure text into Markdown using swarms-rs agent.
    pub async fn summarize_text(&self, text: &str, locale: LocaleCode) -> Result<String, String> {
        let model = OpenAI::from_url(&self.base_url, &self.api_key).set_model(&self.model_name);

        // Use unified system prompt based on language
        let system_prompt = if locale == LocaleCode::En {
            crate::llm::prompts::SYSTEM_PROMPT_SUMMARIZE_EN
        } else {
            crate::llm::prompts::SYSTEM_PROMPT_SUMMARIZE_JA
        };

        // Create completion request with structural wrapping
        let (prefix, text_content) = if locale == LocaleCode::En {
            ("Please summarize and restructure the following text:", text)
        } else {
            ("以下のテキストを要約・再構成してください：", text)
        };
        let user_prompt = format!("{}\n<text>\n{}\n</text>", prefix, text_content);
        let request = CompletionRequest {
            prompt: Message::user(user_prompt),
            system_prompt: Some(system_prompt.to_string()),
            chat_history: vec![],
            tools: vec![],
            temperature: None,
            max_tokens: None,
        };

        log::debug!(
            "[LLM] Using swarms-rs local completion for summarization on endpoint: {}",
            self.name
        );

        // Execute completion directly to get usage info
        let response = model
            .completion(request)
            .await
            .map_err(|e| format!("swarms-rs model (summarization) failed: {}", e))?;

        // Extract and record token usage
        if let Some(usage) = response.raw_response.usage {
            if let Err(e) = UsageStats::record_llm(
                &self.model_name,
                usage.prompt_tokens as u64,
                usage.completion_tokens as u64,
            ) {
                log::error!("[LLM] Failed to record statistics: {}", e);
            }
        }

        // Return the first choice content and extract text within XML tags if present
        let raw_text = response
            .choice
            .get(0)
            .and_then(|c| match c {
                AssistantContent::Text(Text { text }) => Some(text.clone()),
                _ => None,
            })
            .ok_or_else(|| {
                "No completion choices returned or unexpected response type".to_string()
            })?;

        Ok(self.extract_result(&raw_text))
    }

    /// Extract content from the first XML-like tag pair found in the text.
    /// If no tags are found or the extraction result is empty, returns the original text.
    fn extract_result(&self, text: &str) -> String {
        use once_cell::sync::Lazy;
        use regex::Regex;

        // Rust's regex crate does not support backreferences (like \1).
        // We find the first opening tag and then look for its corresponding closing tag.
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
                        // Further clean up the extracted content by removing ANY remaining XML tags
                        // This handles cases where the LLM might have nested tags like <result><text>...</text></result>
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

struct LlmPoolState {
    clients: Vec<LlmClient>,
    counter: AtomicUsize,
    has_valid_endpoints: bool,
}

/// Pool of LLM clients with round-robin distribution.
pub struct LlmPool {
    state: RwLock<LlmPoolState>,
}

impl LlmPool {
    /// Create a new pool from endpoint configs.
    pub fn new(endpoints: &[LlmEndpoint]) -> Self {
        let clients: Vec<LlmClient> = endpoints
            .iter()
            .map(|e| LlmClient::from_config(e))
            .filter(|c| c.is_valid())
            .collect();

        let has_valid_endpoints = !clients.is_empty();

        // システム時刻をシードにして、開始時のエンドポイントをランダム化します。
        // ナノ秒単位の大きな値をそのままカウンターの初期値として使用する意図は以下の通りです：
        // 2. カウンターのオーバーフローを含め、どの数値から始まっても正しく巡回する堅牢なラウンドロビンを実現する。
        // なお、実際のインデックスは next() メソッド内の剰余演算（% clients.len()）によって常に範囲内に収まります。
        let start_idx = time::now_ts_ms() as usize;

        Self {
            state: RwLock::new(LlmPoolState {
                clients,
                counter: AtomicUsize::new(start_idx),
                has_valid_endpoints,
            }),
        }
    }

    /// Update the endpoints dynamically.
    pub fn update_endpoints(&self, endpoints: &[LlmEndpoint]) {
        let new_clients: Vec<LlmClient> = endpoints
            .iter()
            .map(|e| LlmClient::from_config(e))
            .filter(|c| c.is_valid())
            .collect();

        let new_has_valid_endpoints = !new_clients.is_empty();
        let current_count = {
            let state = self.state.read();
            state.counter.load(Ordering::Relaxed)
        };

        let mut state = self.state.write();
        state.clients = new_clients;
        state.counter.store(current_count, Ordering::Relaxed);
        state.has_valid_endpoints = new_has_valid_endpoints;
    }

    /// Check if the pool has any endpoints.
    pub fn is_empty(&self) -> bool {
        let state = self.state.read();
        state.clients.is_empty()
    }

    /// Get the next client using round-robin.
    /// Returns None if there are no valid clients, allowing the caller to bypass LLM processing.
    pub fn next(&self) -> Option<LlmClient> {
        let state = self.state.read();
        if !state.has_valid_endpoints || state.clients.is_empty() {
            return None;
        }

        let idx = state.counter.fetch_add(1, Ordering::Relaxed) % state.clients.len();
        // Return a cloned client (since it contains mostly Strings) to keep lock lifetime extremely short
        let client = &state.clients[idx];
        Some(LlmClient {
            name: client.name.clone(),
            model_name: client.model_name.clone(),
            base_url: client.base_url.clone(),
            api_key: client.api_key.clone(),
        })
    }

    /// Correct text to proper written style using one of the available endpoints.
    pub async fn correct_text(&self, text: &str, locale: LocaleCode) -> Result<String, String> {
        let client = match self.next() {
            Some(c) => c,
            None => {
                log::info!("[LLM Pool] No valid endpoints available. Bypassing text correction (Mimicry Mode).");
                sleep_ms(LLM_MIMICRY_DELAY_MS);
                return Ok(text.to_string());
            }
        };
        client.correct_text(text, locale).await
    }

    /// Summarize text into Markdown using one of the available endpoints.
    pub async fn summarize_text(&self, text: &str, locale: LocaleCode) -> Result<String, String> {
        let client = match self.next() {
            Some(c) => c,
            None => {
                log::info!("[LLM Pool] No valid endpoints available. Bypassing text summarization (Mimicry Mode).");
                sleep_ms(LLM_MIMICRY_DELAY_MS);
                return Ok(text.to_string());
            }
        };
        client.summarize_text(text, locale).await
    }

    /// Execute specialized LLM action (correct, summarize, etc.) using one of the available endpoints.
    pub async fn execute(
        &self,
        action: crate::types::LlmAction,
        text: &str,
        locale: LocaleCode,
    ) -> Result<String, String> {
        match action {
            crate::types::LlmAction::Correct => self.correct_text(text, locale).await,
            crate::types::LlmAction::Summarize => self.summarize_text(text, locale).await,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stt_config::LlmEndpoint;

    #[test]
    fn test_extract_result() {
        let client = LlmClient::from_config(&LlmEndpoint {
            name: "test".to_string(),
            model: "test".to_string(),
            base_url: "test".to_string(),
            api_key: None,
        });

        // 推奨タグの場合
        assert_eq!(
            client.extract_result("Here is the result: <result>Corrected text</result> some noise"),
            "Corrected text"
        );

        // 異なるタグ名の場合
        assert_eq!(
            client.extract_result("Alternative tag: <output>Some content</output>"),
            "Some content"
        );

        // タグが複数ある場合（最初を優先）
        assert_eq!(
            client.extract_result("<first>First</first> <second>Second</second>"),
            "First"
        );

        // タグが存在しない場合（そのまま返す）
        assert_eq!(
            client.extract_result("No tags here at all"),
            "No tags here at all"
        );

        // タグが空（またはタグ除去後に空）の場合（そのまま返す）
        assert_eq!(
            client.extract_result("Empty tags: <result></result>"),
            "Empty tags: <result></result>"
        );

        // 入れ子になったタグの場合
        assert_eq!(
            client.extract_result("<result><text>Nested content</text></result>"),
            "Nested content"
        );

        // 複数の入れ子タグがある場合
        assert_eq!(
            client.extract_result("<output><text>Multiple</text><note>Tags</note></output>"),
            "MultipleTags"
        );

        // 改行と入れ子タグを含む場合
        assert_eq!(
            client.extract_result(
                "Prefix\n<result>\n  <text>\n    Indented and nested\n  </text>\n</result>"
            ),
            "Indented and nested"
        );
    }
}
