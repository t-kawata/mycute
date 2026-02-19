use axum::{Extension, Json, response::IntoResponse};
use crate::constants::ST_OK;
use crate::mode::rt::rtevent::{ProxyLeakPayload, LeakSeverity, LeakSource};
use tauri::{AppHandle, Emitter};
use crate::mode::rt::rtreq::mycute_proxy_leaks_req::{CreateCspReportReq, CreateSwLeakReq};
use crate::mode::rt::rtres::errs_res::ApiError;
use crate::constants::EVENT_PROXY_LEAK;
use crate::utils::time;

const TAG: &str = "v1 ProxyLeak";
const PROXY_LEAK_TAG: &str = "[MYCUTE PROXY LEAK]";

// ============================================================
// CSP Leak Report
// ============================================================
const CSP_DESC: &str = r#"
### ⚫︎ 概要
- ブラウザ(WebView)がプロキシを介さない通信を検知した際に送信される標準レポートを受信します。
- 受信した内容は開発者ログに出力され、同時にWebViewのコンソールにもリレーされます。
- `Content-Security-Policy-Report-Only` ヘッダーによってトリガーされます。

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `csp-report` | object | required | CSPレポート本体 |
"#;
#[utoipa::path(
    tag = TAG,
    post,
    path = "/mycute_proxy_leak/csp",
    summary = "CSP違反レポートを受信する。",
    description = CSP_DESC,
    request_body = CreateCspReportReq,
    responses(
        (status = 200, description = "Success"),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn create_csp_leak_report(
    Extension(app_handle): Extension<AppHandle>,
    Json(req): Json<CreateCspReportReq>,
) -> impl IntoResponse {
    let report = req.csp_report;
    let url = report.document_uri.clone();
    let msg = format!(
        "CSP Violation: Blocked URI: {}, Directive: {}",
        report.blocked_uri.as_deref().unwrap_or("unknown"),
        report.violated_directive.as_deref().unwrap_or("unknown")
    );

    // 開発者ログ
    log::error!("{} {}", PROXY_LEAK_TAG, msg);

    // MycuteEventBus: Emit System Event
    let payload = ProxyLeakPayload {
        severity: LeakSeverity::Warning,
        source: LeakSource::Csp,
        url,
        message: msg,
        timestamp: time::now_ts(),
    };
    
    // 全てのウィンドウ (Shell) に対してイベントを発火
    // Shell Bridge がこれを拾い、iframe へ転送する
    if let Err(e) = app_handle.emit(EVENT_PROXY_LEAK, &payload) {
        log::error!("Failed to emit proxy leak event: {}", e);
    }

    ST_OK
}

// ============================================================
// Service Worker Leak Report
// ============================================================
const SW_DESC: &str = r#"
### ⚫︎ 概要
- Service Worker や SDK インターセプターが異常な通信（プロキシ漏れ）を検知した際に送信されます。
- 受信した内容は開発者ログに出力され、同時にWebViewのコンソールにもリレーされます。

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `url` | string | required | 漏洩したURL |
| `message` | string | option | エラーメッセージ |
| `source_file` | string | option | 検知元ファイル |
| `line_number` | number | option | 行番号 |
"#;
#[utoipa::path(
    tag = TAG,
    post,
    path = "/mycute_proxy_leak/sw",
    summary = "SW/SDKからのリーク検知レポートを受信する。",
    description = SW_DESC,
    request_body = CreateSwLeakReq,
    responses(
        (status = 200, description = "Success"),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn create_sw_leak_report(
    Extension(app_handle): Extension<AppHandle>,
    Json(req): Json<CreateSwLeakReq>,
) -> impl IntoResponse {
    let msg = format!(
        "SW/SDK Leak: {}",
        req.message.as_deref().unwrap_or("")
    );

    log::error!("{} {}", PROXY_LEAK_TAG, msg);

    let payload = ProxyLeakPayload {
        severity: LeakSeverity::Critical, // SW検知はコードレベルの違反なので重大
        source: LeakSource::ServiceWorker,
        url: req.url.clone(),
        message: msg,
        timestamp: time::now_ts(),
    };

    if let Err(e) = app_handle.emit(EVENT_PROXY_LEAK, &payload) {
        log::error!("Failed to emit proxy leak event: {}", e);
    }

    ST_OK
}
