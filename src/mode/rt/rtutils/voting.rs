use crate::constants::{ERR_SERIALIZE_VOTES, ST_INTERNAL_SERVER_ERROR};
use crate::mode::rt::rtres::errs_res::ApiError;

/// 投票時の署名対象ペイロード文字列を生成する。
/// フォーマット: "app_id:vote:vote_allocated:timestamp:ticket_json"
pub fn format_vote_payload(
    app_id: &str,
    vote: i32,
    vote_allocated: i32,
    timestamp: &str,
    ticket: &serde_json::Value,
) -> Result<String, ApiError> {
    let ticket_json_str = serde_json::to_string(ticket).map_err(|e| {
        ApiError::new_system(ST_INTERNAL_SERVER_ERROR, ERR_SERIALIZE_VOTES, e.to_string())
    })?;
    Ok(format!(
        "app_id:{},vote:{},vote_allocated:{},timestamp:{},ticket:{}",
        app_id, vote, vote_allocated, timestamp, ticket_json_str
    ))
}

/// CAが投票処理結果（領収書）に署名するためのペイロードを作成する
/// 形式: "vote_allocated:{},timestamp:{},node_sig:{}"
pub fn format_vote_receipt_payload(vote_allocated: i32, timestamp: i64, node_sig: &str) -> String {
    format!(
        "vote_allocated:{},timestamp:{},node_sig:{}",
        vote_allocated, timestamp, node_sig
    )
}
