---
ticket_id: 4
title: "Windows: 完全初期化時のBifrost/ZeroClaw終了とディレクトリ削除問題の代替アプローチ"
slug: windows-bifrostzeroclaw-2
status: approved
created_at: 2026-05-17
updated_at: 2026-05-17
---
# Windows: 完全初期化時のBifrost/ZeroClaw終了とディレクトリ削除問題の代替アプローチ

## Summary

チケット #2 で実装した同期型クリーンアップ（taskkill /F /T → 待機 → remove_dir_all リトライ → cmd rmdir フォールバック）では Windows の Bifrost/ZeroClaw プロセス終了と MYCUTE_HOME ディレクトリ削除が完了しない。原因は Windows の必須ファイルロックとプロセス終了後のカーネルハンドル解放遅延にある。本チケットでは「アプリ終了後に独立したプロセスがクリーンアップを引き継ぐ」アプローチに方向転換し、バッチファイル方式による確実な削除を実現する。

## Background

### チケット #2 の現状と限界

チケット #2 では以下の対策を実装した:
1. `BackendProcessGuard::drop()` での `taskkill /F /T /PID` 結果のログ出力（握りつぶし解消）
2. `kill_process_by_image_name()` によるイメージ名ベースの追い打ち（Phase 7.5）
3. `wait_for_process_exit()` による 5 秒間のアクティブポーリング（Phase 8）
4. `delete_home_directory()` のリトライ + OS フォールバック（Phase 9）
5. `release_and_delete()` によるロック解放→削除の順序保証

しかしテストの結果、これらの対策でも削除が完了しないことが確認された。

### 根本原因の分析

**原因1: プロセス階層の非対称性**

```
CL (Tauri)
  └── RT (mycute-server)    ← taskkill /F /T で kill
        ├── Bifrost          ← RT の子。RT が死ぬと orphan
        └── ZeroClaw         ← RT の子。RT が死ぬと orphan
```

`taskkill /F /T /PID <rt_pid>` は RT のプロセスツリーを強制終了する。しかし /T の動作は **親プロセスが生存していることが前提** であり、RT が先に死んで Bifrost/ZeroClaw が orphan 化した場合、/T の対象から漏れる。Phase 7.5 のイメージ名ベースの追い打ちはこの安全網だが、taskkill がプロセスを見つけても kill 自体が失敗する場合がある（アクセス拒否、プロセスが既に死んでいる等）。

**原因2: Windows の必須ファイルロック**

Windows ではプロセスがファイルを開いている限り、そのファイルは削除不可能（mandatory locking）。macOS では開いているファイルでも削除可能（inode ベース）なため問題が顕在化しない。具体的な妨害要因:

| ファイル | 保持プロセス | 状況 |
|----------|-------------|------|
| SQLite DB (`db/mycute.sqlite`) + WAL/SHM | CL 自身 | Phase 4 で `drop(pools)` しても非同期プール解放の完了タイミングが非決定的 |
| ログファイル (`log/*.log`) | RT, Bifrost, ZeroClaw | プロセス強制終了後も OS カーネル内のハンドル解放に遅延 |
| Bifrost 内部 SQLite | Bifrost プロセス | Bifrost が強制終了されてもファイルハンドル解放は非同期 |
| DLL/ローダーロック | OS ローダー | MYCUTE_HOME 配下の DLL をロードしている場合、プロセス生存中は削除不可 |

**原因3: タイミングの非決定性**

`wait_for_process_exit` の 5 秒ポーリング + 1 秒スリープはヒューリスティックであり、OS のカーネルハンドルクリーンアップが完了するまでの時間を保証できない。高負荷時やセキュリティソフトのスキャン中はさらに遅延する可能性がある。

**原因4: taskkill の出力が診断不可能**

すべての taskkill 呼び出しは `.status()`（stdout/stderr 未キャプチャ）を使用しており、失敗理由（アクセス拒否 / プロセス不在 / 特権不足）がログから診断不可能。

### 結論: 同期的単一プロセスアプローチの限界

上記の分析から、同一プロセス内で同期的に全ファイルハンドルを解放しディレクトリを削除することは Windows では **構造的に 100% 信頼できない** と結論する。必要なのは「アプリ終了後に独立して動作するクリーンアップ機構」である。

## Scope

1. **バッチファイル方式の導入**（プライマリ）: `reset_and_exit` の最終フェーズで、MYCUTE_HOME 削除専用のバッチファイルを生成し、バックグラウンド起動後に `app_handle.exit(0)` を呼ぶ
2. **既存の同期クリーンアップは維持**: バッチファイルは最終手段であり、従来の同期クリーンアップはベストエフォートとして継続
3. **バッチファイルのエラーハンドリング**: 削除失敗時のリトライ、タイムアウト、ログ出力
4. **Windows 専用**: macOS では既存の同期削除で問題ないため対象外
5. **ユニットテスト**: バッチファイル生成ロジックのテスト

## Non-scope

- フロントエンド（Vue.js / Quasar）の変更
- macOS/Linux 向けのコード変更
- Bifrost / ZeroClaw 自体の終了処理ロジックの改良
- 別途実行可能バイナリ（`.exe`）の作成（バッチファイルで十分）
- `MoveFileEx + MOVEFILE_DELAY_UNTIL_REBOOT`（再起動が必要で UX を損なうため不採用）

## Investigation

### バッチファイル方式の設計

**基本動作:**

```
1. reset_and_exit が Phase 8（プロセス待機）まで実行
2. Phase 9: 既存の delete_home_directory() をベストエフォートで実行
3. Phase 9.5 [NEW]: バッチファイル生成 + バックグラウンド起動
4. Phase 10: app_handle.exit(0) — アプリ終了
5. [アプリ終了後] バッチファイルが独立して削除をリトライ
6. 削除成功後、バッチファイルが自身を削除
```

**バッチファイルの内容（設計案）:**

```batch
@echo off
setlocal

set TARGET_DIR=%USERPROFILE%\.mycute
set MAX_RETRIES=30
set RETRY_COUNT=0
set LOG_FILE=%TEMP%\mycute_cleanup_%RANDOM%.log

echo [%DATE% %TIME%] MYCUTE cleanup started > "%LOG_FILE%"
echo [%DATE% %TIME%] Target: %TARGET_DIR% >> "%LOG_FILE%"

:LOOP
if not exist "%TARGET_DIR%" goto SUCCESS
echo [%DATE% %TIME%] Attempt %RETRY_COUNT% / %MAX_RETRIES% >> "%LOG_FILE%"

rmdir /s /q "%TARGET_DIR%" 2>> "%LOG_FILE%"
if not exist "%TARGET_DIR%" goto SUCCESS

set /a RETRY_COUNT+=1
if %RETRY_COUNT% geq %MAX_RETRIES% goto FAILED

timeout /t 2 /nobreak >nul
goto LOOP

:SUCCESS
echo [%DATE% %TIME%] SUCCESS: Directory deleted >> "%LOG_FILE%"
del "%~f0"
exit /b 0

:FAILED
echo [%DATE% %TIME%] FAILED: Directory still exists after %MAX_RETRIES% retries >> "%LOG_FILE%"
exit /b 1
```

**重要な設計ポイント:**

- `timeout /t 2` で 2 秒間隔のリトライ（合計最大 60 秒）
- アプリ終了後も独立して動作するため、OS カーネルのハンドル解放を十分に待てる
- ログファイルを `%TEMP%` に出力（`%RANDOM%` で一意性確保）
- 最終的に自身を削除（痕跡を残さない）
- **全パスは必ず二重引用符でクォートする**: `"%TARGET_DIR%"`、`"%LOG_FILE%"` — Windows のパスにスペースが含まれていても安全
- **ログファイル名にはタイムスタンプを含める**: `mycute_cleanup_%DATE:/=-%_%TIME::=-%_%RANDOM%.log` — 1秒以内の連続起動でも衝突しない

### 代替案: Windows Job Object（将来の拡張）

より根本的な解決として、RT プロセスの生成時に Windows Job Object を作成し `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` を設定する方法がある。Job Object が閉じられると全子プロセスが確実に強制終了される。しかしこれは CL の RT 起動ロジックの変更が必要であり、チケット #4 のスコープを超える。

### プロセス生存確認の診断改善

taskkill の失敗理由を特定するために、`.status()` から `.output()` への変更も併せて実施する:
- 現状: `Command::new("taskkill").args([...]).status()` — stdout/stderr 未キャプチャ
- 改善: `Command::new("taskkill").args([...]).output()` — stderr をキャプチャしてログ出力

### バッチファイルの配置場所

`%TEMP%` ディレクトリに `mycute_cleanup_<TIMESTAMP>_<RANDOM>.bat` として生成する。`%TEMP%` は再起動時に自動クリーンアップされるため、バッチファイルが削除漏れしても問題ない。

### 安全性の補強

#### 1. パスクォーティング

バッチファイル内の **すべてのパスは二重引用符でクォートする**。Windows では `%USERPROFILE%` にスペースが含まれることが一般的（`C:\Users\John Doe`）であり、クォート漏れはスクリプトの誤動作の最大の原因になる。

```batch
set TARGET_DIR="%USERPROFILE%\.mycute"
set LOG_FILE="%TEMP%\mycute_cleanup_%RANDOM%.log"
set LOCK_FILE="%TEMP%\mycute_cleanup.lock"
```

#### 2. 複数バッチプロセスの競合防止（ロックファイル）

ユーザーが「完全初期化」を連続して実行した場合、複数のバッチファイルが同時に動作する可能性がある。`%TEMP%` にロックファイルを置き、先に起動したバッチだけが処理を実行する:

```batch
if exist "%LOCK_FILE%" (
    echo [%DATE% %TIME%] Another cleanup instance is already running. Exiting. >> "%LOG_FILE%"
    del "%~f0"
    exit /b 0
)
echo %DATE% %TIME% > "%LOCK_FILE%"
```

ロックファイルはバッチ終了時に削除する。バッチが強制終了された場合にロックファイルが残るが、`%TEMP%` は再起動時にクリアされるため問題にならない。

#### 3. グローバルタイムアウト

`rmdir /s /q` がファイルシステムの問題でハングする可能性に備え、バッチ全体の実行時間に上限を設ける。並行して起動した watchdog が上限超過時にバッチ自身を強制終了する:

```batch
:: 並行 watchdog プロセス（60秒後にこのバッチを kill）
start /b cmd /c "timeout /t 60 /nobreak >nul & taskkill /F /PID %~1 2>nul"
```

ただしこの方法は PID の受け渡しが必要になる。より簡易な方法として、`rmdir` のリトライ回数制限（30回）で実質的なタイムアウトを担保する。30回 × 2秒 = 60秒で自動停止する設計のため、バッチが永久に動き続けることはない。

#### 4. `%TEMP%` フォールバック

環境変数 `%TEMP%` が未設定の場合、`%WINDIR%\Temp` を代替パスとして使用する:

```batch
if not defined TEMP set TEMP=%WINDIR%\Temp
if not defined TEMP set TEMP=C:\Windows\Temp
```

`%WINDIR%` も未設定の場合は `C:\Windows\Temp` をハードコードする。このケースは現実的には起こり得ないが、防御的プログラミングとして記述する。

#### 5. パス検証（Rust 側）

`spawn_cleanup_bat()` 内で、`home_dir` が `%USERPROFILE%` 配下であることを軽度に検証する。パスが異常に短い場合や `..` を含む場合はエラーを返す:

```rust
fn spawn_cleanup_bat(home_dir: &Path) -> Result<(), String> {
    let path_str = home_dir.to_str().ok_or("Invalid UTF-8 in home_dir path")?;
    if path_str.len() < 5 {
        return Err("home_dir path is suspiciously short".to_string());
    }
    // パストラバーサルの簡易チェック
    if path_str.contains("..") {
        return Err("home_dir path contains '..' traversal".to_string());
    }
    // ...バッチ生成処理
}
```

#### 6. macOS への非影響（再確認）

全変更は `#[cfg(windows)]` でガードされる。バッチファイルは `.bat` 拡張子であり、macOS で実行されることは決してない。macOS 向けビルドでは `spawn_cleanup_bat` はコンパイルされず、`reset_and_exit` の Phase 9.5 ブロックも存在しない。

## Test Plan

### テスト対象

| テスト | 分類 | 内容 |
|--------|------|------|
| バッチファイルの内容検証 | 単体 | 生成されたバッチファイルのテキストが期待通りか |
| バッチファイルの構文検証 | 単体 | 生成されたバッチが Windows の cmd.exe で解釈可能か |
| 全パスがクォートされていることの検証 | 単体 | テンプレート内の全パス参照に二重引用符があることを確認 |
| ロックファイルによる多重起動防止 | 単体 | ロックファイル存在時にバッチがスキップするロジックの検証 |
| リトライ回数上限（30回）の検証 | 単体 | 削除できないディレクトリに対して30回で打ち切りになること |
| `spawn_cleanup_bat()` のエラー処理 | 単体 | バッチファイル書き込み失敗時のエラーハンドリング |
| `temp` ディレクトリ不在時のフォールバック | 単体 | TEMP 環境変数が未設定の場合の代替パス |
| パス検証（`..` トラバーサル） | 単体 | 不正なパスが渡されたときに `Err` を返すこと |
| パス検証（短すぎるパス） | 単体 | 長さ5未満のパスが渡されたときに `Err` を返すこと |
| 既存 `reset_and_exit` の変更なし確認 | 回帰 | Phase 1-8/10/11 の動作が変更されていないこと |
| 既存 `delete_home_directory` テストの継続通過 | 回帰 | 同期削除がまだベストエフォートとして機能すること |

### テスト方針

- バッチファイル生成ロジックは文字列操作のため、モック不要でテスト可能
- 実際のプロセス起動テストは `#[cfg(windows)]` でガードし、テスト用バッチが正しく実行されることを確認
- バッチファイルの内容は `include_str!` またはテンプレート文字列として保持し、テストで期待値と比較
- 既存の同期削除テスト（system.rs の `test_delete_home_directory_*`）は全て維持

## Boy Scout Rule — 翻訳可能性計画

1. **バッチファイル生成を関数として抽出**: `spawn_cleanup_bat(home_dir: &Path) -> Result<(), String>` とし、関数名で意図を語らせる
2. **マジックナンバーの排除**: リトライ間隔（2秒）、最大リトライ回数（30回）を名前付き定数として定義
3. **エラー握りつぶしの解消**: 既存の `.status()` → `.output()` 変更により taskkill のエラー詳細をログ出力可能にする
4. **README への追記**: バッチファイル方式の存在をドキュメント化し、運用者がクリーンアップの仕組みを理解できるようにする

## Acceptance Criteria

### 機能
- [ ] `reset_and_exit` の最終フェーズで MYCUTE_HOME 削除用バッチファイルが生成される
- [ ] 生成されたバッチファイルが `CREATE_NO_WINDOW` でバックグラウンド起動される
- [ ] `app_handle.exit(0)` がバッチファイル起動後に呼ばれる（アプリが終了する）
- [ ] アプリ終了後、バッチファイルが MYCUTE_HOME を完全に削除する
- [ ] 削除成功後、バッチファイルが自身を削除する（痕跡を残さない）
- [ ] 削除失敗時はログを `%TEMP%` に出力する
- [ ] 既存の同期クリーンアップ（delete_home_directory）はベストエフォートとして継続される
- [ ] Windows で「完全に初期化して終了」を実行し、次回起動時に MYCUTE_HOME が存在しない

### 安全性
- [ ] 生成されるバッチファイル内の全パスが二重引用符でクォートされている
- [ ] 連続して「完全初期化」を実行した場合、ロックファイルにより二重実行が防止される
- [ ] `rmdir` のリトライは30回（約60秒）で打ち切られる（永久ループしない）
- [ ] `spawn_cleanup_bat()` がパストラバーサル（`..`）を含むパスを拒否する
- [ ] `%TEMP%` が未設定の場合、`%WINDIR%\Temp` にフォールバックする
- [ ] taskkill のエラー出力がログで確認可能になる（`.output()` 化）

### 環境分離
- [ ] `cargo test`（全環境）が通過する
- [ ] macOS 向けコードには一切変更が加えられていない
- [ ] 全変更が `#[cfg(windows)]` でガードされている

## Notes

### バッチファイル方式の限界

- バッチファイルが起動後、ユーザーが手動で `taskkill` した場合、クリーンアップは中断される
- Windows のファイルロックが 60 秒以上継続する場合は削除失敗（システム障害レベル）
- バッチファイルが生成するログファイルは `%TEMP%` に残る（再起動で自動削除）

### バッチファイル生成コードの配置

`src/tauri_cmd/system.rs` に追加する。`reset_and_exit` の Phase 9（既存の削除試行）の直後、Phase 10（アプリ終了）の直前に挿入する:

```rust
// ---- Phase 9.5: Windows: 後処理バッチファイルの生成と起動 ----
#[cfg(windows)]
{
    spawn_cleanup_bat(&home_dir);
}

// ---- Phase 10: アプリ終了 ----
log::warn!("RESET_AND_EXIT: Exiting application.");
app_handle.exit(0);
```

### 参考: macOS では問題ない理由

macOS では削除対象ファイルが開いていても `remove_dir_all` で削除可能（Unix の inode ベースファイルシステムの特性）。Fate-Sharing により Bifrost/ZeroClaw も確実に終了する。したがって本チケットの変更は全量 `#[cfg(windows)]` でガードする。
