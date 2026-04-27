use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MyCuteVersionRes {
    #[schema(example = "v0.1.0")]
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct MyCuteHomeDirRes {
    #[schema(example = "/Users/username/.mycute")]
    pub home_dir: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SetLangRes {
    #[schema(example = "Language updated successfully")]
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SetSttEngineRes {
    #[schema(example = "STT engine updated successfully")]
    pub message: String,
}

// SetLlmsRes / GetMycuteLlmsRes は LMGW 移行に伴い廃止済み

/// GET /mycute/catoken/verify のレスポンス型。検証結果を返す。
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VerifyCaTokenRes {
    pub success: bool,
    #[schema(example = "CA Cert is valid")]
    pub message: String,
    /// 署名が正当な場合に、CA任命証内に含まれている CA 公開鍵を返す
    pub ca_pubkey: Option<String>,
    /// CA任命証の有効期限（Unix TS）
    pub expire_at: Option<u64>,
    /// 権限内容 (JSON)
    pub permissions: Option<serde_json::Value>,
}

// ============================================================
// ライセンス管理
// ============================================================

/// 1 件のライセンスのパース済みサマリー。フロントエンドのカード表示に使用する。
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LicenseSummary {
    /// 自身の保持リスト内での識別子（ライセンス文字列の SHA-256 先頭 16 文字）
    pub id: String,
    /// ライセンスを発行した CA の公開鍵 (Hex)
    pub ca_pubkey: String,
    /// ライセンスの有効期限（Unix TS ms）
    pub expire_at: u64,
    /// 権限内容 (Base64 デコード前の JSON 文字列)
    pub permissions: serde_json::Value,
    /// ライセンスが現在有効かどうか
    pub is_valid: bool,
    /// 元のライセンス文字列（登録・削除に使用）
    pub raw: String,
}

/// GET /mycute/license/list レスポンス。
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ListLicensesRes {
    pub licenses: Vec<LicenseSummary>,
}

/// POST /mycute/license/register レスポンス。
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct RegisterLicenseRes {
    pub success: bool,
    pub message: String,
    pub summary: Option<LicenseSummary>,
}

/// POST /mycute/license/unregister レスポンス。
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UnregisterLicenseRes {
    pub success: bool,
    pub message: String,
}

/// POST /mycute/license/verify レスポンス。
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VerifyLicenseRes {
    pub success: bool,
    pub message: String,
    pub summary: Option<LicenseSummary>,
}
