use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct NodeRawExecRes {
    /// 標準出力 (stdout)
    pub stdout: String,
    /// 標準エラー出力 (stderr)
    pub stderr: String,
    /// 終了コード
    pub exit_code: i32,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct NodeFileExecRes {
    /// 標準出力 (stdout)
    pub stdout: String,
    /// 標準エラー出力 (stderr)
    pub stderr: String,
    /// 終了コード
    pub exit_code: i32,
}
