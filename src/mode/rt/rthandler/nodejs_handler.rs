use crate::mode::rt::rtbl::nodejs_bl;
use crate::mode::rt::rterr::rterr;
use crate::mode::rt::rtreq::nodejs_req::{NodeFileExecReq, NodeRawExecReq};
use crate::mode::rt::rtres::errs_res::ApiError;
use crate::mode::rt::rtres::nodejs_res::{NodeFileExecRes, NodeRawExecRes};
use crate::nodejs::NodeManager;
use axum::{http::StatusCode, Extension, Json};
use garde::Validate;
use std::sync::Arc;

const TAG: &str = "v1 NodeJS";

// ============================================================
// Raw Execution
// ============================================================
const RAW_DESC: &str = r#"
### ⚫︎ 概要
- 送信された JavaScript コードを Node.js の標準入力経由で実行します。
- 短いスクリプトや即時の動作確認に適しています。

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `script` | string | required | 実行する JavaScript コード |

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `stdout` | string | 標準出力の内容 |
| `stderr` | string | 標準エラー出力の内容 |
| `exit_code` | number | 終了コード（成功時は0） |
"#;

#[utoipa::path(
    tag = TAG,
    post,
    path = "/nodejs/node/raw",
    summary = "JavaScript コードを生の文字列として実行する。",
    description = RAW_DESC,
    request_body = NodeRawExecReq,
    responses(
        (status = 200, description = "Success", body = NodeRawExecRes),
        (status = 422, description = "Validation Error", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn exec_node_raw(
    Extension(node_manager): Extension<Arc<NodeManager>>,
    Json(req): Json<NodeRawExecReq>,
) -> Result<Json<NodeRawExecRes>, ApiError> {
    req.validate().map_err(|e| ApiError::from_garde(e))?;

    let res = nodejs_bl::execute_raw(&node_manager, req.script)
        .await
        .map_err(|e| {
            ApiError::new_system(
                StatusCode::INTERNAL_SERVER_ERROR,
                rterr::ERR_UNEXPECTED,
                format!("NodeJS raw execution failed: {}", e),
            )
        })?;

    Ok(Json(NodeRawExecRes {
        stdout: res.stdout,
        stderr: res.stderr,
        exit_code: res.exit_code,
    }))
}

// ============================================================
// File Execution
// ============================================================
const FILE_DESC: &str = r#"
### ⚫︎ 概要
- サーバー上の特定のディレクトリ (`MYCUTE_HOME/scripts`) にある JavaScript ファイルを実行します。
- すでに配置されているスクリプトファイルや、大規模なプログラムの実行に適しています。

### セキュリティ
- 実行可能なファイルは `MYCUTE_HOME/scripts` 配下に限定されます。
- ディレクトリ・トラバーサル（`..` 等による境界外へのアクセス）は遮断されます。
- 絶対パスの指定は無効化され、常にベースディレクトリからの相対パスとして解釈されます。

### ⚫︎ Request
| KEY | TYPE | VALIDATION | DESCRIPTION |
| --- | --- | --- | --- |
| `path` | string | required | `MYCUTE_HOME/scripts` からの相対パス |

### ⚫︎ Response
| KEY | TYPE | DESCRIPTION |
| --- | --- | --- |
| `stdout` | string | 標準出力の内容 |
| `stderr` | string | 標準エラー出力の内容 |
| `exit_code` | number | 終了コード（成功時は0） |
"#;

#[utoipa::path(
    tag = TAG,
    post,
    path = "/nodejs/node/file",
    summary = "JavaScript ファイルをパス指定で実行する。",
    description = FILE_DESC,
    request_body = NodeFileExecReq,
    responses(
        (status = 200, description = "Success", body = NodeFileExecRes),
        (status = 422, description = "Validation Error", body = ApiError),
        (status = 500, description = "Internal Server Error", body = ApiError)
    )
)]
pub async fn exec_node_file(
    Extension(node_manager): Extension<Arc<NodeManager>>,
    Json(req): Json<NodeFileExecReq>,
) -> Result<Json<NodeFileExecRes>, ApiError> {
    req.validate().map_err(|e| ApiError::from_garde(e))?;

    let res = nodejs_bl::execute_file(&node_manager, req.path)
        .await
        .map_err(|e| {
            ApiError::new_system(
                StatusCode::INTERNAL_SERVER_ERROR,
                rterr::ERR_UNEXPECTED,
                format!("NodeJS file execution failed: {}", e),
            )
        })?;

    Ok(Json(NodeFileExecRes {
        stdout: res.stdout,
        stderr: res.stderr,
        exit_code: res.exit_code,
    }))
}
