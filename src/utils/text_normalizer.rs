//! Text Normalizer Utilities
//!
//! Ported from Go: mycute-go/src/pkg/cuber/utils/normalize.go
//! Provides text normalization functions for Vector, Graph, and Search operations.

use once_cell::sync::Lazy;
use regex::Regex;

// ============================================================
// Regex Patterns
// ============================================================

// Markdown 関連
static CODE_BLOCK_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?s)````+.*?````+|```.*?```").unwrap());
static INLINE_CODE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"`([^`]*)`").unwrap());
static LINK_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\[([^\]]+)\]\([^\)]+\)").unwrap());
static IMAGE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"!\[([^\]]*)\]\([^\)]+\)").unwrap());
static HEADING_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^#+\s+").unwrap());
static LIST_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^[\*\-\+]\s+").unwrap());
static NUMBERED_LIST_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^\d+\.\s+").unwrap());
static QUOTE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^>\s*").unwrap());
static HR_DASH_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^-{3,}$").unwrap());
static HR_STAR_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^\*{3,}$").unwrap());
static HR_UNDER_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?m)^_{3,}$").unwrap());

// HTML 関連
static SCRIPT_STYLE_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?is)<script[^>]*?>.*?</script>").unwrap());
static STYLE_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?is)<style[^>]*?>.*?</style>").unwrap());
static COMMENT_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"(?s)<!--.*?-->").unwrap());
static TAG_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"<[^>]+>").unwrap());

// 空白・改行関連
static CONSECUTIVE_SPACES_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"[ \t]+").unwrap());
static CONSECUTIVE_NEWLINES_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"\n{3,}").unwrap());
static TRAILING_SPACES_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"[ \t]+\n").unwrap());

// 制御文字・絵文字
static CONTROL_RE: Lazy<Regex> = Lazy::new(|| Regex::new(r"[\x00-\x1F\x7F-\x9F]").unwrap());
static EMOJI_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"[\u{1F600}-\u{1F64F}]|[\u{1F300}-\u{1F5FF}]|[\u{1F680}-\u{1F6FF}]|[\u{2600}-\u{26FF}]|[\u{2700}-\u{27BF}]").unwrap()
});

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
        || TAG_RE.is_match(header)
}

fn extract_from_html(text: &str) -> String {
    let mut result = SCRIPT_STYLE_RE.replace_all(text, "").to_string();
    result = STYLE_RE.replace_all(&result, "").to_string();
    result = COMMENT_RE.replace_all(&result, "").to_string();
    result = TAG_RE.replace_all(&result, " ").to_string();
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
    let mut result = CODE_BLOCK_RE.replace_all(text, "").to_string();
    result = INLINE_CODE_RE.replace_all(&result, "$1").to_string();
    result = LINK_RE.replace_all(&result, "$1").to_string();
    result = IMAGE_RE.replace_all(&result, "$1").to_string();
    result = HEADING_RE.replace_all(&result, "").to_string();
    result = LIST_RE.replace_all(&result, "").to_string();
    result = NUMBERED_LIST_RE.replace_all(&result, "").to_string();
    result = QUOTE_RE.replace_all(&result, "").to_string();
    result = HR_DASH_RE.replace_all(&result, "").to_string();
    result = HR_STAR_RE.replace_all(&result, "").to_string();
    result = HR_UNDER_RE.replace_all(&result, "").to_string();
    result
}

fn normalize_whitespace(text: &str) -> String {
    let mut result = text.replace("\\n", "\n");
    result = result.replace("\\r", "\r");
    result = result.replace("\\t", "\t");
    result = result.replace("\r\n", "\n");
    result = result.replace('\r', "\n");
    result = CONSECUTIVE_SPACES_RE.replace_all(&result, " ").to_string();
    result = CONSECUTIVE_NEWLINES_RE
        .replace_all(&result, "\n\n")
        .to_string();
    result = TRAILING_SPACES_RE.replace_all(&result, "\n").to_string();
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
    let result = CONTROL_RE.replace_all(text, "");
    let result = CONSECUTIVE_SPACES_RE.replace_all(&result, " ");
    result.trim().to_string()
}

/// NormalizeForGraph は決定論的なエンティティ解決を確保します。
pub fn normalize_for_graph(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let mut result = text.to_lowercase();
    result = EMOJI_RE.replace_all(&result, "").to_string();
    result = CONSECUTIVE_SPACES_RE.replace_all(&result, " ").to_string();
    result.trim().to_string()
}

/// NormalizeForSearch は全文検索（FTS）の精度を最大化します。
pub fn normalize_for_search(text: &str) -> String {
    if text.is_empty() {
        return String::new();
    }
    let mut result = text.to_lowercase();
    result = CONTROL_RE.replace_all(&result, "").to_string();
    result = EMOJI_RE.replace_all(&result, "").to_string();
    result = CONSECUTIVE_SPACES_RE.replace_all(&result, " ").to_string();
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
