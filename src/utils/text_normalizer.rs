//! Text Normalizer Utilities
//!
//! Ported from Go: mycute-go/src/pkg/cuber/utils/normalize.go
//! Provides text normalization functions for Vector, Graph, and Search operations.

use once_cell::sync::Lazy;
use regex::Regex;

macro_rules! safe_lazy_re {
    ($re:expr) => {
        Lazy::new(|| Regex::new($re))
    };
}

// ============================================================
// Regex Patterns
// ============================================================

// Markdown 関連
static CODE_BLOCK_RE: Lazy<Result<Regex, regex::Error>> = safe_lazy_re!(r"(?s)````+.*?````+|```.*?```");
static INLINE_CODE_RE: Lazy<Result<Regex, regex::Error>> = safe_lazy_re!(r"`([^`]*)`");
static LINK_RE: Lazy<Result<Regex, regex::Error>> = safe_lazy_re!(r"\[([^\]]+)\]\([^\)]+\)");
static IMAGE_RE: Lazy<Result<Regex, regex::Error>> = safe_lazy_re!(r"!\[([^\]]*)\]\([^\)]+\)");
static HEADING_RE: Lazy<Result<Regex, regex::Error>> = safe_lazy_re!(r"(?m)^#+\s+");
static LIST_RE: Lazy<Result<Regex, regex::Error>> = safe_lazy_re!(r"(?m)^[\*\-\+]\s+");
static NUMBERED_LIST_RE: Lazy<Result<Regex, regex::Error>> = safe_lazy_re!(r"(?m)^\d+\.\s+");
static QUOTE_RE: Lazy<Result<Regex, regex::Error>> = safe_lazy_re!(r"(?m)^>\s*");
static HR_DASH_RE: Lazy<Result<Regex, regex::Error>> = safe_lazy_re!(r"(?m)^-{3,}$");
static HR_STAR_RE: Lazy<Result<Regex, regex::Error>> = safe_lazy_re!(r"(?m)^\*{3,}$");
static HR_UNDER_RE: Lazy<Result<Regex, regex::Error>> = safe_lazy_re!(r"(?m)^_{3,}$");

// HTML 関連
static SCRIPT_STYLE_RE: Lazy<Result<Regex, regex::Error>> = safe_lazy_re!(r"(?is)<script[^>]*?>.*?</script>");
static STYLE_RE: Lazy<Result<Regex, regex::Error>> = safe_lazy_re!(r"(?is)<style[^>]*?>.*?</style>");
static COMMENT_RE: Lazy<Result<Regex, regex::Error>> = safe_lazy_re!(r"(?s)<!--.*?-->");
static TAG_RE: Lazy<Result<Regex, regex::Error>> = safe_lazy_re!(r"<[^>]+>");

// 空白・改行関連
static CONSECUTIVE_SPACES_RE: Lazy<Result<Regex, regex::Error>> = safe_lazy_re!(r"[ \t]+");
static CONSECUTIVE_NEWLINES_RE: Lazy<Result<Regex, regex::Error>> = safe_lazy_re!(r"\n{3,}");
static TRAILING_SPACES_RE: Lazy<Result<Regex, regex::Error>> = safe_lazy_re!(r"[ \t]+\n");

// 制御文字・絵文字
static CONTROL_RE: Lazy<Result<Regex, regex::Error>> = safe_lazy_re!(r"[\x00-\x1F\x7F-\x9F]");
static EMOJI_RE: Lazy<Result<Regex, regex::Error>> = safe_lazy_re!(r"[\u{1F600}-\u{1F64F}]|[\u{1F300}-\u{1F5FF}]|[\u{1F680}-\u{1F6FF}]|[\u{2600}-\u{26FF}]|[\u{2700}-\u{27BF}]");

// ============================================================
// Common Normalize
// ============================================================

/// CommonNormalize は、入力テキストから HTML/Markdown 記法を除去しノイズをクリーニングします。
pub fn common_normalize(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }

    // 空白の正規化
    let text = normalize_whitespace(text);

    // HTML の検出と処理
    if detect_html(&text) {
        extract_from_html(&text)
    } else {
        extract_from_markdown(&text)
    }
}

fn detect_html(text: &str) -> bool {
    let header = &text[..std::cmp::min(text.len(), 1000)];
    let h_lower = header.to_lowercase();
    h_lower.contains("<!doctype html")
        || h_lower.contains("<html")
        || h_lower.contains("<head")
        || h_lower.contains("<body")
        || TAG_RE.as_ref().ok().map(|re| re.is_match(header)).unwrap_or(false)
}

fn extract_from_html(text: &str) -> String {
    let mut result = SCRIPT_STYLE_RE.as_ref().ok().map(|re| re.replace_all(text, "").to_string()).unwrap_or_else(|| text.to_string());
    result = STYLE_RE.as_ref().ok().map(|re| re.replace_all(&result, "").to_string()).unwrap_or(result);
    result = COMMENT_RE.as_ref().ok().map(|re| re.replace_all(&result, "").to_string()).unwrap_or(result);
    result = TAG_RE.as_ref().ok().map(|re| re.replace_all(&result, " ").to_string()).unwrap_or(result);
    decode_html_entities(&result)
}

fn decode_html_entities(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&nbsp;", " ")
        .replace("&ndash;", "–")
        .replace("&mdash;", "—")
        .replace("&hellip;", "…")
        .replace("&copy;", "©")
        .replace("&reg;", "®")
        .replace("&trade;", "™")
}

fn extract_from_markdown(text: &str) -> String {
    let mut result = CODE_BLOCK_RE.as_ref().ok().map(|re| re.replace_all(text, "").to_string()).unwrap_or_else(|| text.to_string());
    result = INLINE_CODE_RE.as_ref().ok().map(|re| re.replace_all(&result, "$1").to_string()).unwrap_or(result);
    result = LINK_RE.as_ref().ok().map(|re| re.replace_all(&result, "$1").to_string()).unwrap_or(result);
    result = IMAGE_RE.as_ref().ok().map(|re| re.replace_all(&result, "$1").to_string()).unwrap_or(result);
    result = HEADING_RE.as_ref().ok().map(|re| re.replace_all(&result, "").to_string()).unwrap_or(result);
    result = LIST_RE.as_ref().ok().map(|re| re.replace_all(&result, "").to_string()).unwrap_or(result);
    result = NUMBERED_LIST_RE.as_ref().ok().map(|re| re.replace_all(&result, "").to_string()).unwrap_or(result);
    result = QUOTE_RE.as_ref().ok().map(|re| re.replace_all(&result, "").to_string()).unwrap_or(result);
    result = HR_DASH_RE.as_ref().ok().map(|re| re.replace_all(&result, "").to_string()).unwrap_or(result);
    result = HR_STAR_RE.as_ref().ok().map(|re| re.replace_all(&result, "").to_string()).unwrap_or(result);
    result = HR_UNDER_RE.as_ref().ok().map(|re| re.replace_all(&result, "").to_string()).unwrap_or(result);
    result
}

fn normalize_whitespace(text: &str) -> String {
    let mut result = text.replace("\\n", "\n");
    result = result.replace("\\r", "\r");
    result = result.replace("\\t", "\t");
    result = result.replace("\r\n", "\n");
    result = result.replace('\r', "\n");
    result = CONSECUTIVE_SPACES_RE.as_ref().ok().map(|re| re.replace_all(&result, " ").to_string()).unwrap_or(result);
    result = CONSECUTIVE_NEWLINES_RE
        .as_ref().ok().map(|re| re.replace_all(&result, "\n\n").to_string())
        .unwrap_or(result);
    result = TRAILING_SPACES_RE.as_ref().ok().map(|re| re.replace_all(&result, "\n").to_string()).unwrap_or(result);
    result.trim().to_string()
}

// ============================================================
// Specialized Normalizers
// ============================================================

/// NormalizeForVector は意味を保持しつつノイズを除去します。
pub fn normalize_for_vector(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let result = CONTROL_RE.as_ref().ok().map(|re| re.replace_all(text, "").to_string()).unwrap_or_else(|| text.to_string());
    let result = CONSECUTIVE_SPACES_RE.as_ref().ok().map(|re| re.replace_all(&result, " ").to_string()).unwrap_or(result);
    result.trim().to_string()
}

/// NormalizeForGraph は決定論的なエンティティ解決を確保します。
pub fn normalize_for_graph(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let mut result = text.to_lowercase();
    result = EMOJI_RE.as_ref().ok().map(|re| re.replace_all(&result, "").to_string()).unwrap_or(result);
    result = CONSECUTIVE_SPACES_RE.as_ref().ok().map(|re| re.replace_all(&result, " ").to_string()).unwrap_or(result);
    result.trim().to_string()
}

/// NormalizeForSearch は全文検索（FTS）の精度を最大化します。
pub fn normalize_for_search(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let mut result = text.to_lowercase();
    result = CONTROL_RE.as_ref().ok().map(|re| re.replace_all(&result, "").to_string()).unwrap_or(result);
    result = EMOJI_RE.as_ref().ok().map(|re| re.replace_all(&result, "").to_string()).unwrap_or(result);
    result = CONSECUTIVE_SPACES_RE.as_ref().ok().map(|re| re.replace_all(&result, " ").to_string()).unwrap_or(result);
    result.trim().to_string()
}

// ============================================================
// Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_markdown_heading() {
        let input = "# Heading\nText";
        let result = extract_from_markdown(input);
        assert!(result.contains("Heading"));
        assert!(!result.contains("#"));
    }

    #[test]
    fn test_remove_markdown_link() {
        let input = "[Link Text](https://example.com)";
        let result = extract_from_markdown(input);
        assert_eq!(result.trim(), "Link Text");
    }

    #[test]
    fn test_remove_markdown_code_block() {
        let input = "Before\n```rust\nlet x = 1;\n```\nAfter";
        let result = extract_from_markdown(input);
        assert!(result.contains("Before"));
        assert!(result.contains("After"));
        assert!(!result.contains("let x = 1"));
    }

    #[test]
    fn test_normalize_for_vector() {
        let input = "Hello   World\u{0000}!";
        let result = normalize_for_vector(input);
        assert_eq!(result, "Hello World!");
    }

    #[test]
    fn test_normalize_for_graph() {
        let input = "Hello WORLD";
        let result = normalize_for_graph(input);
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_normalize_for_search() {
        let input = "  HELLO   WORLD  ";
        let result = normalize_for_search(input);
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_common_normalize_markdown() {
        let input = "# Title\n\n- Item 1\n- Item 2\n\n> Quote";
        let result = common_normalize(input);
        assert!(result.contains("Title"));
        assert!(result.contains("Item 1"));
        assert!(!result.contains("#"));
        assert!(!result.contains("-"));
        assert!(!result.contains(">"));
    }

    #[test]
    fn test_common_normalize_html() {
        let input = "<html><body><p>Hello</p><script>alert('x')</script></body></html>";
        let result = common_normalize(input);
        assert!(result.contains("Hello"));
        assert!(!result.contains("script"));
        assert!(!result.contains("alert"));
    }
}
