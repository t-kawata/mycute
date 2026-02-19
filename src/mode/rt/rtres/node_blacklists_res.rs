use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ReportBlacklistNodeRes {
    pub success: bool,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct SyncBlacklistNodeRes {
    /// 同期が成功したかどうか
    pub success: bool,
    /// 新しく取り込まれた証拠の件数
    pub new_items_count: u64,
}
