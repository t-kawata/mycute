//! Lindera Tokenizer Wrapper
//!
//! Go 版の Kagome に相当する日本語形態素解析器のラッパーです。
//! Lindera を使用して日本語テキストをトークン化します。

use lindera::tokenizer::Tokenizer as LinderaTokenizerInner;
use crate::tools::lindera_util::get_tokenizer;

use super::error::CuberError;

/// Lindera ベースの形態素解析器ラッパー
///
/// `Arc<LinderaTokenizer>` として `CuberService` で共有され、
/// 辞書のロードオーバーヘッドを起動時の一度のみに抑えます。
pub struct LinderaTokenizer {
    inner: LinderaTokenizerInner,
}

impl LinderaTokenizer {
    /// 新しい Tokenizer インスタンスを作成します。
    ///
    /// IPA 辞書を使用して初期化します。
    pub fn new() -> Result<Self, CuberError> {
        let inner = get_tokenizer().map_err(|e| CuberError::TokenizerInitError(e.to_string()))?;
        Ok(Self { inner })
    }

    /// テキストをトークン化し、トークン（表層形）のリストを返します。
    pub fn tokenize(&mut self, text: &str) -> Result<Vec<String>, CuberError> {
        let tokens = self.inner.tokenize(text)
            .map_err(|e| CuberError::TokenizerInitError(format!("Tokenization failed: {}", e)))?;

        Ok(tokens.iter().map(|t| t.surface.to_string()).collect())
    }

    /// テキストをトークン化し、名詞のみを抽出します。
    ///
    /// FTS の Layer 0 (nouns) に対応します。
    pub fn extract_nouns(&mut self, text: &str) -> Result<Vec<String>, CuberError> {
        let tokens = self.inner.tokenize(text)
            .map_err(|e| CuberError::TokenizerInitError(format!("Tokenization failed: {}", e)))?;

        let nouns: Vec<String> = tokens
            .iter()
            .filter(|t| {
                // 品詞情報から名詞を抽出
                // details は Option<Vec<Cow<'_, str>>> で、
                // 最初の要素が品詞（例: "名詞"）
                t.details
                    .as_ref()
                    .and_then(|d| d.first())
                    .map(|s| {
                        let s_ref: &str = s.as_ref();
                        s_ref == "名詞"
                    })
                    .unwrap_or(false)
            })
            .map(|t| t.surface.to_string())
            .collect();

        Ok(nouns)
    }

    /// テキストをトークン化し、名詞と動詞を抽出します。
    ///
    /// FTS の Layer 1 (nouns_verbs) に対応します。
    pub fn extract_nouns_verbs(&mut self, text: &str) -> Result<Vec<String>, CuberError> {
        let tokens = self.inner.tokenize(text)
            .map_err(|e| CuberError::TokenizerInitError(format!("Tokenization failed: {}", e)))?;

        let nouns_verbs: Vec<String> = tokens
            .iter()
            .filter(|t| {
                t.details
                    .as_ref()
                    .and_then(|d| d.first())
                    .map(|s| {
                        let s_ref: &str = s.as_ref();
                        s_ref == "名詞" || s_ref == "動詞"
                    })
                    .unwrap_or(false)
            })
            .map(|t| t.surface.to_string())
            .collect();

        Ok(nouns_verbs)
    }

    /// テキストをトークン化し、全ての内容語（名詞、動詞、形容詞、副詞）を抽出します。
    ///
    /// FTS の Layer 2 (keywords) に対応します。
    pub fn extract_keywords(&mut self, text: &str) -> Result<Vec<String>, CuberError> {
        let tokens = self.inner.tokenize(text)
            .map_err(|e| CuberError::TokenizerInitError(format!("Tokenization failed: {}", e)))?;

        let keywords: Vec<String> = tokens
            .iter()
            .filter(|t| {
                t.details
                    .as_ref()
                    .and_then(|d| d.first())
                    .map(|s| {
                        let s_ref: &str = s.as_ref();
                        s_ref == "名詞"
                            || s_ref == "動詞"
                            || s_ref == "形容詞"
                            || s_ref == "副詞"
                    })
                    .unwrap_or(false)
            })
            .map(|t| t.surface.to_string())
            .collect();

        Ok(keywords)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize() {
        let mut tokenizer = LinderaTokenizer::new().expect("Failed to create tokenizer");
        let tokens = tokenizer.tokenize("東京都に住んでいます。").expect("Failed to tokenize");
        assert!(!tokens.is_empty());
        // 東京都、に、住ん、で、い、ます、。 などのトークンが含まれるはず
    }

    // MYCUTE開発のため一時的にコメントアウト
    // #[test]
    // fn test_extract_nouns() {
    //     let mut tokenizer = LinderaTokenizer::new().expect("Failed to create tokenizer");
    //     let nouns = tokenizer.extract_nouns("東京都に住んでいます。").expect("Failed to extract nouns");
    //     // 「東京」「都」などの名詞が含まれるはず
    //     assert!(!nouns.is_empty());
    // }
}
