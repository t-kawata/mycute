use crate::constants::{
    MYCUTE_PROXY_SUFFIX,
    MYCUTE_SDK_FILENAME, HEADER_CSP_REPORT_ONLY,
    SCHEME_PREFIX_HTTP, PROTOCOL_HTTPS, DOMAIN_LOCALHOST, PATH_PROXY_LEAK_CSP,
    MYCUTE_PROXY_PORT,
};
use lol_html::{element, HtmlRewriter, Settings};

/// ホスト名からサフィックス方式のプロキシ対象かどうかを判定し、
/// 対象であればサフィックスを除去したオリジナルのホスト名を返します。
/// プロキシ対象でなければ、元のホスト名をそのまま返します。
/// 
/// # Arguments
/// * `host` - 検査対象のホスト名 (e.g., "example.com.mc.shyme.net")
/// 
/// # Returns
/// * `String` - サフィックスを除去したホスト名、または元のホスト名
pub fn extract_original_host(host: &str) -> String {
    // ポート番号が含まれている場合は除去して判定する
    let host_only = host.split(':').next().unwrap_or(host);
    
    if host_only.ends_with(MYCUTE_PROXY_SUFFIX) {
        let original_len = host_only.len() - MYCUTE_PROXY_SUFFIX.len();
        let base = &host_only[..original_len];
        // If the base ends with a dot (e.g. google.com.), it might be an issue, but usually not.
        
        // Double-hyphen decoding: "--" -> "."
        base.replace("--", ".")
    } else {
        host.to_string()
    }
}

/// 指定されたホスト名がプロキシ対象のサフィックスを持っているか判定します。
pub fn is_proxied_host(host: &str) -> bool {
    host.ends_with(MYCUTE_PROXY_SUFFIX)
}

/// ホスト名をダブルハイフン形式でエンコードし、サフィックスを付与します。
/// 
/// # Arguments
/// * `host` - 元のホスト名 (e.g., "google.com")
/// 
/// # Returns
/// * `String` - エンコードされたホスト名 (e.g., "google--com.mc.shyme.net")
pub fn encode_host(host: &str) -> String {
    if host.is_empty() { return String::new(); }
    if host.ends_with(MYCUTE_PROXY_SUFFIX) { return host.to_string(); }
    
    // Replace dots with double-hyphens
    let encoded_base = host.replace('.', "--");
    format!("{}{}", encoded_base, MYCUTE_PROXY_SUFFIX)
}

/// プロキシ応答のヘッダーを加工します (Vec<(String, String)> 版)
/// reqwest(http 0.2) と axum(http 1.1) のバージョン競合を回避するため、文字列ベースで処理します。
pub fn process_proxy_headers(
    headers: Vec<(String, String)>, 
    sw_port: u16
) -> Vec<(String, String)> {
    let mut new_headers = Vec::new();

    for (name, value) in headers {
        let name_lower = name.to_lowercase();

        // 1. セキュリティ制限ヘッダーおよび転送制御ヘッダーの除去
        if name_lower == "x-frame-options"
            || name_lower == "content-security-policy"
            || name_lower == "frame-options" 
            || name_lower == "content-length"  // body変更時に再計算するため除去
            || name_lower == "content-encoding" // reqwestが自動解凍するため、圧縮ヘッダーは除去必須
            || name_lower == "transfer-encoding" // chunked等の転送設定は再構築時に不整合になるため除去
            { 
            continue;
        }

        // 2. Cookie の正規化 (Set-Cookie)
        if name_lower == "set-cookie" {
            let tokens: Vec<&str> = value.split(';').map(|s| s.trim()).collect();
            if let Some(first_token) = tokens.first() {
                let mut new_tokens = vec![*first_token];
                for &token in tokens.iter().skip(1) {
                    let token_l = token.to_lowercase();
                    if !token_l.starts_with("samesite") 
                        && !token_l.eq("secure") 
                        && !token_l.eq("partitioned") 
                        && !token_l.starts_with("domain") {
                        new_tokens.push(token);
                    }
                }
                new_tokens.push("SameSite=None");
                new_tokens.push("Secure");
                new_tokens.push("Partitioned");
                new_headers.push((name, new_tokens.join("; ")));
                continue;
            }
        }

        new_headers.push((name, value));
    }

    // 3. CSP (Allow All)
    new_headers.push((
        "Content-Security-Policy".to_string(),
        "default-src * 'unsafe-inline' 'unsafe-eval' data: blob: filesystem: about: ws: wss:;".to_string()
    ));

    // 4. Report-Only CSP
    let report_uri = format!("{}{}:{}{}", SCHEME_PREFIX_HTTP, DOMAIN_LOCALHOST, sw_port, PATH_PROXY_LEAK_CSP);
    let wildcard_domain = format!("*{}", MYCUTE_PROXY_SUFFIX);
    let csp_ro = format!(
        "default-src * 'unsafe-inline' 'unsafe-eval' data: blob: filesystem: about: ws: wss:; connect-src 'self' mycute: mycutes: ws: wss: {}{}:* {}://{} data: blob:; report-uri {}", 
        SCHEME_PREFIX_HTTP, DOMAIN_LOCALHOST, PROTOCOL_HTTPS, wildcard_domain, report_uri
    );
    new_headers.push((HEADER_CSP_REPORT_ONLY.to_string(), csp_ro));

    // 5. Location Header Rewriting (Redirect following)
    // 3xx リダイレクトの際、Location ヘッダーが外部ドメインを指していたらエンコードする
    for i in 0..new_headers.len() {
        if new_headers[i].0.eq_ignore_ascii_case("Location") {
            let val = &new_headers[i].1;
            if val.starts_with("http") { // http:// or https://
                // Parse and encode host
                if let Ok(mut url) = reqwest::Url::parse(val) {
                    if let Some(host) = url.host_str() {
                        let encoded_host = encode_host(host);
                        if encoded_host != host {
                            let _ = url.set_host(Some(&encoded_host));
                            let _ = url.set_port(Some(MYCUTE_PROXY_PORT));
                            new_headers[i].1 = url.to_string();
                        }
                    }
                }
            }
        }
    }

    new_headers
}

/// Helper function to rewrite URLs in HTML attributes
fn rewrite_url_in_attribute(url_str: &str) -> String {
    if url_str.starts_with("http://") || url_str.starts_with("https://") {
        if let Ok(mut url) = reqwest::Url::parse(url_str) {
            if let Some(host) = url.host_str() {
                let encoded_host = encode_host(host);
                if encoded_host != host {
                    let _ = url.set_host(Some(&encoded_host));
                    let _ = url.set_port(Some(MYCUTE_PROXY_PORT));
                    return url.to_string();
                }
            }
        }
    }
    url_str.to_string()
}

/// HTML ボディを書き換えます（SDK注入 + リンク書き換え）。
/// lol_html を使用してストリーミングフレンドリー（今回はバッファ処理だが）にパースします。
pub fn rewrite_html(body: Vec<u8>) -> Vec<u8> {
    let mut output = Vec::new();
    
    // SDK Script Tag to inject
    let script_tag = format!(r#"<script src="/{}" type="module" async="false"></script>"#, MYCUTE_SDK_FILENAME);

    let mut rewriter = HtmlRewriter::new(
        Settings {
            element_content_handlers: vec![
                // 1. Inject SDK into <head>
                element!("head", |el| {
                    el.prepend(&script_tag, lol_html::html_content::ContentType::Html);
                    Ok(())
                }),
                // 2. Rewrite links (a[href])
                element!("a[href], link[href], area[href]", |el| {
                    if let Some(val) = el.get_attribute("href") {
                        el.set_attribute("href", &rewrite_url_in_attribute(&val))?;
                    }
                    Ok(())
                }),
                // 3. Rewrite sources (img[src], script[src], etc.)
                element!("img[src], script[src], iframe[src], embed[src], source[src], track[src], video[src], audio[src], input[src]", |el| {
                    if let Some(val) = el.get_attribute("src") {
                         el.set_attribute("src", &rewrite_url_in_attribute(&val))?;
                    }
                    Ok(())
                }),
                // Rewrite srcset (img[srcset], source[srcset])
                element!("img[srcset], source[srcset]", |el| {
                    if let Some(val) = el.get_attribute("srcset") {
                         // split by comma, rewrite each url
                         let new_srcset = val.split(',')
                            .map(|part| {
                                let parts: Vec<&str> = part.trim().split_whitespace().collect();
                                if let Some(url) = parts.first() {
                                    let new_url = rewrite_url_in_attribute(url);
                                    // Reconstruct part (url + descriptors)
                                    let rest = parts[1..].join(" ");
                                    if rest.is_empty() {
                                        new_url
                                    } else {
                                        format!("{} {}", new_url, rest)
                                    }
                                } else {
                                    part.to_string()
                                }
                            })
                            .collect::<Vec<String>>()
                            .join(", ");
                         el.set_attribute("srcset", &new_srcset)?;
                    }
                    Ok(())
                }),
                // 4. Form actions
                element!("form[action]", |el| {
                     if let Some(val) = el.get_attribute("action") {
                         el.set_attribute("action", &rewrite_url_in_attribute(&val))?;
                     }
                     Ok(())
                }),
                // 5. Meta refresh? <meta http-equiv="refresh" content="0; url=...">
                 // Too complex to parse content string properly with simple attribute check, skip for now.

                // 6. Remove CSP Meta tags
                element!("meta", |el| {
                    if let Some(val) = el.get_attribute("http-equiv") {
                        if val.eq_ignore_ascii_case("Content-Security-Policy") {
                            el.remove();
                        }
                    }
                    Ok(())
                }),
            ],
            ..Settings::default()
        },
        |c: &[u8]| {
            output.extend_from_slice(c);
        }
    );

    if rewriter.write(&body).is_ok() && rewriter.end().is_ok() {
        return output;
    }

    // If rewriting fails, return original
    body
}

/// CSS コンテンツ内の URL を書き換えます。
/// url(...) および @import "..." のパターンを検出してプロキシURLに置換します。
pub fn rewrite_css_content(content: &str) -> String {
    use regex::Regex;
    use std::sync::OnceLock;

    // Pattern 1: url(...)
    // Matches: url('http...'), url("http..."), url(http...)
    // Capture groups: 1=quote, 2=url, 3=quote
    static URL_REGEX: OnceLock<Regex> = OnceLock::new();
    let url_re = URL_REGEX.get_or_init(|| {
        Regex::new(r#"url\(\s*(['"]?)(https?://[^)'"]+)(['"]?)\s*\)"#).unwrap()
    });

    // Pattern 2: @import "..." or @import '...'
    // Matches: @import "http...", @import 'http...'
    static IMPORT_REGEX: OnceLock<Regex> = OnceLock::new();
    let import_re = IMPORT_REGEX.get_or_init(|| {
        Regex::new(r#"@import\s+(['"])(https?://[^'"]+)(['"])"#).unwrap()
    });

    let step1 = url_re.replace_all(content, |caps: &regex::Captures| {
        let quote1 = &caps[1];
        let url = &caps[2];
        let quote2 = &caps[3];
        
        // Rewrite URL
        let new_url = rewrite_url_in_attribute(url); // Re-use existing helper
        format!("url({}{}{})", quote1, new_url, quote2)
    });

    let step2 = import_re.replace_all(&step1, |caps: &regex::Captures| {
        let quote1 = &caps[1];
        let url = &caps[2];
        let quote2 = &caps[3];
        
        let new_url = rewrite_url_in_attribute(url);
        format!("@import {}{}{}", quote1, new_url, quote2)
    });

    step2.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_original_host() {
        let suffix = MYCUTE_PROXY_SUFFIX;

        // Standard case with double-hyphen encoding
        // google.com -> google--com.mc.shyme.net
        let input = format!("google--com{}", suffix);
        assert_eq!(extract_original_host(&input), "google.com");

        // Subdomain case
        // sub.example.com -> sub--example--com.mc.shyme.net
        let input_sub = format!("sub--example--com{}", suffix);
        assert_eq!(extract_original_host(&input_sub), "sub.example.com");
        
        // Host with actual hyphen
        // my-lat.orb -> my-lat--orb.mc.shyme.net
        let input_hyphen = format!("my-lat--orb{}", suffix);
        assert_eq!(extract_original_host(&input_hyphen), "my-lat.orb");

        // No suffix
        assert_eq!(extract_original_host("example.com"), "example.com");

        // Broken suffix
        let input_broken = format!("example--com{}.other", suffix);
        assert_eq!(extract_original_host(&input_broken), input_broken);
    }

    #[test]
    fn test_encode_host() {
        let suffix = MYCUTE_PROXY_SUFFIX; // ".mc.shyme.net"
        
        assert_eq!(encode_host("google.com"), format!("google--com{}", suffix));
        assert_eq!(encode_host("api-server.org"), format!("api-server--org{}", suffix));
        assert_eq!(encode_host("sub.example.co.jp"), format!("sub--example--co--jp{}", suffix));
        
        // Idempotency
        let encoded = format!("already--encoded{}", suffix);
        assert_eq!(encode_host(&encoded), encoded);
        
        // Empty
        assert_eq!(encode_host(""), "");
    }

    #[test]
    fn test_is_proxied_host() {
        assert!(is_proxied_host(&format!("test{}", MYCUTE_PROXY_SUFFIX)));
        assert!(!is_proxied_host("test.com"));
    }
}
