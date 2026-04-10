use std::fs;
use std::process::Stdio;
use anyhow::{Context, Result, bail};
use crate::nodejs::NodeManager;

/// Node.js 実行の内部共通結果構造体
pub struct NodeExecResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

/// 生の JavaScript コードを標準入力経由で実行します。
pub async fn execute_raw(node_manager: &NodeManager, script: String) -> Result<NodeExecResult> {
    log::debug!("<NodeJS BL> Executing raw script via stdin.");
    
    let mut child = node_manager.node()
        .arg("-") // 標準入力から読み込み
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to spawn node process for raw execution")?;

    // 標準入力にスクリプトを書き込む
    {
        use std::io::Write;
        let mut stdin = child.stdin.take().context("Failed to open stdin")?;
        stdin.write_all(script.as_bytes()).context("Failed to write to stdin")?;
        // stdin がドロップされると書き込みが完了し、node が実行を開始する
    }

    let output = child.wait_with_output().context("Failed to wait for node process")?;

    Ok(NodeExecResult {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code().unwrap_or(-1),
    })
}

/// 指定されたパスの JavaScript ファイルを実行します。
/// 
/// セキュリティ上の理由から、パスは MYCUTE_HOME/scripts からの相対パスとして扱われ、
/// ディレクトリ・トラバーサル（.. 等によるベースディレクトリ外へのアクセス）は遮断されます。
pub async fn execute_file(
    node_manager: &NodeManager,
    path: String,
) -> Result<NodeExecResult> {
    log::debug!("<NodeJS BL> Executing script path relative to scripts dir: {}", path);

    let scripts_dir = node_manager.get_scripts_dir();
    
    // ベースディレクトリが実在し、正規化できることを確認
    let scripts_dir_abs = fs::canonicalize(&scripts_dir)
        .with_context(|| format!("Scripts directory not found: {:?}", scripts_dir))?;

    // ターゲットパスを結合
    let target_path = if path.starts_with('/') {
        // 先頭が / の場合でもベースパスからの相対として扱う（安全のため絶対パス指定を無効化）
        scripts_dir.join(path.trim_start_matches('/'))
    } else {
        scripts_dir.join(&path)
    };

    // ターゲットパスを正規化（.. 等を解決）
    // 存在しない場合はこの時点でエラーになる
    let target_path_abs = fs::canonicalize(&target_path)
        .with_context(|| format!("Script file not found or inaccessible: {}", path))?;

    // 境界チェック：正規化後のパスがベースディレクトリの配下にあるか
    if !target_path_abs.starts_with(&scripts_dir_abs) {
        bail!("Access denied: Path is outside of scripts directory: {}", path);
    }

    run_node_file(node_manager, &target_path_abs.to_string_lossy())
}

/// 内部用：指定されたパスのファイルを Node.js で実行します。
fn run_node_file(node_manager: &NodeManager, path: &str) -> Result<NodeExecResult> {
    let output = node_manager.node()
        .arg(path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .context("Failed to execute node file")?;

    Ok(NodeExecResult {
        stdout: String::from_utf8_lossy(&output.stdout).to_string(),
        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        exit_code: output.status.code().unwrap_or(-1),
    })
}
