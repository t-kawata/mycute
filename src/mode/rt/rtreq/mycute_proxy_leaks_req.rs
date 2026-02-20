use garde::Validate;
use serde::Deserialize;
use utoipa::ToSchema;

// note: 厳格ルールでは src/mode/rt/rterr/ を使用すべきだが、
// 該当モジュールは現在非公開(または未実装)の可能性があるため、
// 今回は単純なバリデーションのみを適用する。
// 将来的にはカスタムアダプタを使用する形にリファクタリングする。

// ============================================================
// CSP Leak Report
// ============================================================
#[derive(Deserialize, Validate, ToSchema, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct CreateCspReportReq {
    #[garde(dive)]
    pub csp_report: CspReportBody,
}

#[derive(Deserialize, Validate, ToSchema, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct CspReportBody {
    #[schema(example = "https://example.com/page")]
    #[garde(skip)]
    pub document_uri: String,

    #[schema(example = "https://example.com/page")]
    #[garde(skip)]
    pub referrer: Option<String>,

    #[schema(example = "https://malicious.com/script.js")]
    #[garde(skip)]
    pub blocked_uri: Option<String>,

    #[schema(example = "script-src")]
    #[garde(skip)]
    pub violated_directive: Option<String>,

    #[schema(example = "default-src 'self'")]
    #[garde(skip)]
    pub original_policy: Option<String>,
}

// ============================================================
// Service Worker Leak Report
// ============================================================
#[derive(Deserialize, Validate, ToSchema, Debug)]
pub struct CreateSwLeakReq {
    #[schema(example = "https://leak-example.com/api/data")]
    #[garde(length(min = 1))]
    #[serde(default)]
    pub url: String,

    #[schema(example = "https://mycute.app/")]
    #[garde(skip)]
    pub scope: Option<String>,

    #[schema(example = "Fetch to non-proxy origin detected")]
    #[garde(skip)]
    pub message: Option<String>,
}
