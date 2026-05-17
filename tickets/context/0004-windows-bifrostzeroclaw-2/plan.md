# 実装計画: #4 Windows 完全初期化時の Bifrost/ZeroClaw 終了とディレクトリ削除問題の代替アプローチ

## 要件の再確認

Windows の `reset_and_exit` で MYCUTE_HOME ディレクトリ削除が確実に完了しない問題に対し、「アプリ終了後に独立したバッチファイルがクリーンアップを引き継ぐ」方式に方向転換する。既存の同期クリーンアップはベストエフォートとして維持し、その直後にバッチファイルを生成・起動してから `app_handle.exit(0)` を呼ぶ。

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|----------|------|------|
| `src/tauri_cmd/system.rs` | 変更 | `reset_and_exit` に Phase 9.5 追加（`spawn_cleanup_bat` 呼び出し）、`#[cfg(windows)]` ガード |
| `src/tauri_cmd/system.rs` (test) | 追加 | バッチファイル生成ロジックの単体テスト（内容検証・構文検証・パスクォート検証・ロックファイル・リトライ上限・パス検証） |
| `src/utils/process.rs` | 変更 | taskkill 全4箇所の `.status()` → `.output()` 変更（エラー診断改善） |
| 新規: `resources/cleanup_template.bat` | 追加 | バッチファイルテンプレート（`include_str!` で埋め込み、Rust 側からパラメータ置換） |

## Boy Scout 改善（スコープ外の翻訳可能性修正）

| ファイル | 対象 | 修正内容 |
|----------|------|----------|
| `src/utils/process.rs:81` | `BackendProcessGuard::kill_process` の taskkill | `.status()` → `.output()`（エラー診断改善） |
| `src/utils/process.rs:170` | `kill_pids` 内 taskkill | `.status()` → `.output()`（同上） |
| `src/utils/process.rs:268` | `drop` 内 taskkill | `.status()` → `.output()`（同上） |
| `src/utils/process.rs:291` | `kill_process_by_image_name` 内 taskkill | `.status()` → `.output()`（同上） |
| `src/tauri_cmd/system.rs:838-970` | test 内のハードコード値 | マジックナンバーを名前付き定数に抽出 |

## 実装手順

### Step 1: リソースディレクトリの確認とテンプレート作成

`resources/` ディレクトリの存在確認。存在しなければ作成。
`resources/cleanup_template.bat` を作成。以下の置換マーカーを使用:

| マーカー | 意味 | 置換元 |
|----------|------|--------|
| `__TARGET_DIR__` | 削除対象ディレクトリ | `home_dir` |
| `__LOG_FILE__` | ログファイルパス | `%TEMP%\mycute_cleanup_<RANDOM>.log` |
| `__LOCK_FILE__` | ロックファイルパス | `%TEMP%\mycute_cleanup_<RANDOM>.lock` |
| `__MAX_RETRIES__` | 最大リトライ回数 | 定数 `BAT_MAX_RETRIES` (= 30) |
| `__RETRY_DELAY_SEC__` | リトライ間隔（秒） | 定数 `BAT_RETRY_DELAY_SEC` (= 2) |

テンプレートでは全パスを二重引用符でクォートする。

### Step 2: `spawn_cleanup_bat()` の実装

`src/tauri_cmd/system.rs` に `#[cfg(windows)]` でガードされた関数を追加:

```rust
/// Windows: バッチファイルを生成し、バックグラウンドで起動する。
///
/// アプリ終了後に独立して MYCUTE_HOME ディレクトリを削除するための
/// クリーンアップバッチファイルを %TEMP% に生成し、CREATE_NO_WINDOW で起動する。
/// 起動後、呼び出し元は速やかに app_handle.exit(0) を呼ぶこと。
#[cfg(windows)]
fn spawn_cleanup_bat(home_dir: &Path) -> Result<(), String> {
    // cf. resources/cleanup_template.bat
    const BAT_MAX_RETRIES: u32 = 30;
    const BAT_RETRY_DELAY_SEC: u32 = 2;

    // パス検証
    let path_str = home_dir.to_str().ok_or("Invalid UTF-8 in home_dir path")?;
    if path_str.len() < 5 {
        return Err("home_dir path is suspiciously short".to_string());
    }
    if path_str.contains("..") {
        return Err("home_dir path contains '..' traversal".to_string());
    }

    // %TEMP% フォールバック
    let temp_dir = std::env::var("TEMP")
        .or_else(|_| std::env::var("WINDIR").map(|w| format!("{}\\Temp", w)))
        .unwrap_or_else(|_| "C:\\Windows\\Temp".to_string());

    // ログファイル名（一意性確保）
    let timestamp = chrono::Local::now().format("%Y%m%d%H%M%S");
    let random_suffix: u32 = rand::random::<u32>() % 100000;
    let log_file = format!("{}\\mycute_cleanup_{}_{}.log", temp_dir, timestamp, random_suffix);
    let lock_file = format!("{}\\mycute_cleanup_{}_{}.lock", temp_dir, timestamp, random_suffix);
    let bat_file = format!("{}\\mycute_cleanup_{}_{}.bat", temp_dir, timestamp, random_suffix);

    // テンプレート読み込みと置換
    let template = include_str!("../../resources/cleanup_template.bat");
    let content = template
        .replace("__TARGET_DIR__", &escape_bat_string(path_str))
        .replace("__LOG_FILE__", &escape_bat_string(&log_file))
        .replace("__LOCK_FILE__", &escape_bat_string(&lock_file))
        .replace("__MAX_RETRIES__", &BAT_MAX_RETRIES.to_string())
        .replace("__RETRY_DELAY_SEC__", &BAT_RETRY_DELAY_SEC.to_string());

    // 書き出し
    std::fs::write(&bat_file, content)
        .map_err(|e| format!("Failed to write cleanup batch file: {}", e))?;

    // 独立プロセスとして起動（start /b で親の終了影響を受けない）
    let status = std::process::Command::new("cmd")
        .args(["/c", "start", "/b", "", &bat_file])
        .creation_flags(winapi::um::winbase::CREATE_NO_WINDOW)
        .status()
        .map_err(|e| format!("Failed to launch cleanup batch: {}", e))?;

    if !status.success() {
        return Err(format!("Cleanup batch launch failed with exit code: {:?}", status.code()));
    }

    log::info!("Cleanup batch launched: {}", bat_file);
    Ok(())
}
```

#### `escape_bat_string()` 補助関数

```rust
/// バッチファイル内で安全な文字列にエスケープする（`^` のみエスケープが必要）
#[cfg(windows)]
fn escape_bat_string(s: &str) -> String {
    s.replace("^", "^^")
}
```

### Step 3: `reset_and_exit` に Phase 9.5 挿入

Phase 9（既存削除）の直後、Phase 10（exit）の直前に追加:

```rust
// ---- Phase 9.5: Windows: 後処理バッチファイルの生成と起動 ----
#[cfg(windows)]
{
    let home_dir_for_bat = home_dir.clone();
    if let Err(e) = spawn_cleanup_bat(&home_dir_for_bat) {
        log::error!("RESET_AND_EXIT: Failed to spawn cleanup batch: {}", e);
    } else {
        log::info!("RESET_AND_EXIT: Cleanup batch spawned successfully.");
    }
}

// ---- Phase 10: アプリ終了 ----
```

### Step 4: taskkill `.status()` → `.output()` 改善

`src/utils/process.rs` の taskkill 全4箇所について、`.status()` を `.output()` に変更し、stderr をログ出力するように修正。

各箇所のパターン:
```rust
// Before:
let _ = Command::new("taskkill")
    .arg("/F")
    .arg("/PID")
    .arg(pid.to_string())
    .hide_window_if_windows()
    .status();

// After:
match Command::new("taskkill")
    .arg("/F")
    .arg("/PID")
    .arg(pid.to_string())
    .hide_window_if_windows()
    .output()
{
    Ok(output) => {
        if output.status.success() {
            log::info!("<Process> Successfully terminated process {}.", pid);
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            log::warn!("<Process> taskkill /PID {} failed (exit: {:?}): {}", pid, output.status.code(), stderr);
        }
    }
    Err(e) => log::error!("<Process> Error executing taskkill for {}: {}", pid, e),
}
```

### Step 5: テスト追加

`system.rs` の `mod tests` に以下を追加:

| テスト名 | 内容 |
|----------|------|
| `test_spawn_cleanup_bat_content` | 生成されたバッチファイルのテキストが期待通りかを検証 |
| `test_spawn_cleanup_bat_all_paths_quoted` | テンプレート内の全パス参照に二重引用符があることを確認 |
| `test_spawn_cleanup_bat_lock_file` | ロックファイル存在時にスキップするロジックの検証（テンプレート確認） |
| `test_spawn_cleanup_bat_retry_limit` | リトライ回数上限（30回）の検証（テンプレート確認） |
| `test_spawn_cleanup_bat_invalid_path` | `..` を含むパスで `Err` を返すことの検証 |
| `test_spawn_cleanup_bat_short_path` | 長さ5未満のパスで `Err` を返すことの検証 |
| `test_spawn_cleanup_bat_temp_fallback` | `TEMP` 未設定時の代替パス動作 |
| `test_delete_home_directory_no_regression` | 既存テストの継続通過確認 |

## リスクと対応

### 対応済みのリスク

| リスク | レベル | 対応 |
|--------|--------|------|
| バッチファイルの子プロセス強制終了 | Medium | `start /b` で独立プロセスグループを作成し、親（Tauri）終了の影響を受けないようにする |
| `include_str!` のパス解決 | Low | テンプレート配置を `resources/cleanup_template.bat` とし、system.rs からの相対パス `../../resources/cleanup_template.bat` を事前に検証 |
| バッチファイルパスのスペース | Low | テンプレート内の全パスを二重引用符でクォートする設計を徹底 |
| taskkill `.output()` 変更の影響 | Low-Medium | `Command::output()` も `Command::status()` と同様に同期ブロッキング。変更は安全。既存のエラーハンドリングパターンを踏襲 |
| バッチファイルの削除漏れ | Low | `%TEMP%` に配置するため再起動時に自動クリーンアップされる。削除成功時はバッチ自身が `del "%~f0"` で自己削除 |
| 連続実行時の競合 | Low | ロックファイルにより二重実行防止。ロックファイルも `%TEMP%` 配置で自動クリーンアップ対象 |

### 未然に防ぐ設計

- **`#[cfg(windows)]` ガード**: 全新規コードは `#[cfg(windows)]` で保護。macOS ビルドへの影響ゼロ
- **テストカバレッジ**: バッチファイル生成ロジックは文字列操作のみでモック不要。全分岐をカバー
- **エラーハンドリング**: `spawn_cleanup_bat()` のエラーは `log::error!` で記録し、`reset_and_exit` の進行を止めない

## 物理的レビュー方法

1. **`run-quality-checks.js`**: 変更ファイル（`src/tauri_cmd/system.rs`、`src/utils/process.rs`）を対象に実行
2. **grep 翻訳可能性チェック**:
   ```bash
   # 名詞始まり関数のチェック（動詞句であるべき）
   grep -n '^\s*fn [a-z]' src/tauri_cmd/system.rs src/utils/process.rs
   # 1文字変数
   grep -n 'let [a-z] =' src/tauri_cmd/system.rs src/utils/process.rs
   # マジックナンバー（4桁以上）
   grep -nP '\b\d{4,}\b' src/tauri_cmd/system.rs src/utils/process.rs
   ```
3. **`#[cfg(windows)]` ガード確認**: 全新規コードが `#[cfg(windows)]` で保護されていることを目視確認
4. **`cargo build --all-targets` + `cargo test` 全通過確認**
