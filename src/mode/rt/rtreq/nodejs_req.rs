use crate::mode::rt::rterr::rterr::*;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use garde::Validate;

#[derive(Debug, Deserialize, Serialize, ToSchema, Validate)]
pub struct NodeRawExecReq {
    /// 実行する JavaScript コード
    #[garde(custom(required_simple_err(1, 1000000)))]
    pub script: String,
}

#[derive(Debug, Deserialize, Serialize, ToSchema, Validate)]
pub struct NodeFileExecReq {
    /// 実行する既存のファイルパス
    #[garde(custom(required_simple_err(1, 2048)))]
    pub path: String,
}
