---
ticket_id: 2
title: Windows: 完全初期化時にBifrost/ZeroClawプロセス終了とホームディレクトリ削除が失敗する問題
slug: windows-bifrostzeroclaw
status: reviewed
created_at: 2026-05-17
updated_at: 2026-05-17
---
# Windows: 完全初期化時にBifrost/ZeroClawプロセス終了とホームディレクトリ削除が失敗する問題

## Summary

SettingsApp の「完全に初期化して終了」ボタンをクリックした際、Windows で Bifrost / ZeroClaw のバックエンドプロセスが正しく終了されず、MYCUTE_HOME ディレクトリ（`%USERPROFILE%\.mycute`）の削除に失敗する問題を解決する。

macOS では正常動作しているため、修正は Windows のみに適用し、macOS の動作に一切影響を与えない。

## Background

- SettingsApp の「完全に初期化して終了」ボタン（`web/src/apps/SettingsApp.vue:514`）をクリックすると、Tauri コマンド `reset_and_exit` が呼び出される（`src/tauri_cmd/system.rs:550`）
- 同コマンドは 17 のフェーズから構成され、DB リセット → Bifrost クリーンアップ → バックエンドプロセス終了 → MYCUTE_HOME 削除 → アプリ終了 を順次実行する
- macOS では全く問題なく動作しているが、Windows では Bifrost / ZeroClaw のプロセスが終了されず、ディレクトリ削除に失敗する
- 現状、関連するユニットテストが存在しないため、E2E テストに依存せず安全に正しさを検証できる体制が必要

## Scope

1. Windows における `reset_and_exit` のプロセス終了処理の修正（原因特定と対策）
2. ディレクトリ削除のロバスト性向上（リトライ / フォールバック戦略の改善）
3. ユニットテストの追加（プロセス終了・ディレクトリ削除のテスト）
4. すべての変更は `#[cfg(windows)]` でガードし、macOS/Unix への影響をゼロにする

## Non-scope

- フロントエンド（Vue.js / Quasar）の変更は含まない
- `reset_and_exit` 以外のリセット機能（`reset_application`）の動作変更は含まない
- macOS/Linux 向けのコードは一切変更しない
- Bifrost / ZeroClaw 自体の終了処理ロジックの改良は含まない（あくまで CL 側の管理方法の改善）

## Investigation

### アーキテクチャ調査

#### プロセス階層
```
mycute (CL / Tauri)
  └── mycute-server (RT daemon)  ← BackendProcessGuard がこの PID を管理
        ├── bifrost-http.exe      ← RT の子プロセス（バイナリはサードパーティ製）
        └── zeroclaw.exe          ← RT の子プロセス（バイナリはサードパーティ製）
```

- バイナリ名（Windows）: `bifrost-http.exe`（`src/bifrost/executor.rs:9`）、`zeroclaw.exe`（`src/zeroclaw/executor.rs:7`）
- 両方とも `hide_window_if_windows()`（`CREATE_NO_WINDOW` フラグ）付きで起動（`src/bifrost/executor.rs:62`、`src/zeroclaw/executor.rs:48`）

#### reset_and_exit の実行フロー（`src/tauri_cmd/system.rs:537-793`）

| Phase | 処理 | 行 | Windows固有 | 備考 |
|-------|------|-----|------------|------|
| 1 | UsageStats::delete_all_stats() | 557-560 | なし | |
| 2 | do_reset_db() | 562-595 | なし | DB全テーブル削除 + provider_names保存 |
| 3 | Bifrost HTTP クリーンアップ | 597-622 | なし | RT経由でProvider削除APIを呼出 |
| 4 | DB プール解放 | 624-627 | なし | |
| 5 | シャットダウンフラグ | 629-633 | なし | watchdog抑制 |
| 6 | BackendProcessGuard::drop() | 635-640 | **あり** | 内部: taskkill /F /T + 1500ms sleep + ポートクリーンアップ |
| 7 | cleanup_all_backend_ports() | 642-643 | **あり** | netstat + taskkill /F /PID |
| 7.5 | taskkill /F /IM 追い打ち | 645-658 | **Windowsのみ** | bifrost-http.exe / zeroclaw.exe をイメージ名指定 |
| 8 | 待機 3000ms | 660-669 | **Windowsのみ** | macOSは500ms |
| 8.5 | release_lock() | 671-680 | **Windowsのみ** | アプリロックファイル解放 |
| 9 | ディレクトリ削除 | 682-777 | **フォールバック** | remove_dir_all x4 → cmd /c rmdir |
| 10 | 削除確認 | 779-785 | なし | |
| 11 | app_handle.exit(0) | 788-793 | なし | |

#### BackendProcessGuard::drop() の Windows 動作（`src/utils/auth.rs:124-143`）

1. stdin パイプを drop（graceful shutdown 信号）
2. `taskkill /F /T /PID <rt_pid>` — RT プロセスツリー全体を強制終了
3. `std::thread::sleep(1500)` — 1.5秒待機
4. `cleanup_all_backend_ports("Fate-Sharing")` — ポートベースクリーンアップ
5. 中間ログファイル削除

`taskkill` の結果は `let _ = ` で無視されており、失敗しても検知されない。

### 現状のテスト状況

- `reset_and_exit` に対するユニットテスト・統合テストは**一切存在しない**
- `reset_application`（DBリセットのみの旧コマンド）に対するテストも存在しない
- `src/mycute_settings.rs:1752-1761` に `protected_settings_keys` のテストが1件あるのみ

### 問題の仮説

#### 仮説A: taskkill /F /T が Bifrost/ZeroClaw に届いていない（可能性: 中）

**根拠**:
- `CREATE_NO_WINDOW` フラグで起動された子プロセスが process tree の /T 対象から漏れる可能性
- ただし Microsoft のドキュメント上は /T は「プロセスツリー」全体を kill するため、CREATE_NO_WINDOW の有無は影響しない

#### 仮説B: taskkill /F /T 自体が失敗している（可能性: 中）

**根拠**:
- `let _ = ` でエラーが握りつぶされている（`src/utils/auth.rs:131-137`）
- **Windows では taskkill /F の成功・失敗が一切確認されていない**
- `hide_window_if_windows()` が taskkill に適用されるが、これが `creation_flags` 設定エラーを引き起こす可能性

#### 仮説C: プロセスは死んでいるが OS のファイルハンドル解放が完了していない（可能性: 高）

**根拠**:
- Windows の mandatory locking では、プロセス終了後も OS カーネル内でファイルハンドルのクリーンアップに時間がかかる
- 現在の 3000ms 待機 + リトライ [0, 200, 500, 1000]ms では不十分な可能性
- `remove_dir_all` → `cmd /c rmdir /s /q` のフォールバックも同じ理由で失敗する

#### 仮説D: アプリ自身が MYCUTE_HOME 内のファイルを開いたままにしている（可能性: 中）

**根拠**:
- Phase 4 で DB プールを drop しているが、それ以外のファイルハンドル（ログファイル、設定ファイル、キャッシュ等）が解放されていない可能性
- Phase 8.5 の `release_lock()` はロックファイルを解放するが、ロックファイルが `%APPDATA%\.mycute\` にある場合は MYCUTE_HOME（`%USERPROFILE%\.mycute\`）とは異なるため、削除に影響しない

#### 仮説E: バックエンド再起動時の watchdog との競合（可能性: 低）

**根拠**:
- Phase 5 で `is_shutting_down = true` をセットしているため、watchdog による Fate-Sharing 自爆は抑制される
- ただし、`drop(guard)` 後の 1500ms ブロッキング中に watchdog スレッドが動作する可能性はある

### 検証方法

上記仮説の検証には、以下のアプローチが必要：

1. **Phase 6 の taskkill 結果をログ出力する**: 現在 `let _ = ` で無視されている exit status を `log::warn!` で出力
2. **Phase 7.5 の taskkill 結果をログ出力する**: 同様に exit status を確認
3. **Phase 8〜9 の前にプロセス生存確認**: Bifrost/ZeroClaw のプロセスが実際に生きているか `tasklist` で確認
4. **削除失敗時のエラー詳細**: `remove_dir_all` のエラーに含まれる OS エラーコード（例: `ERROR_ACCESS_DENIED`, `ERROR_SHARING_VIOLATION`）を確認すると、原因がファイルハンドルなのか権限なのか切り分け可能

## Test Plan

### 全体方針

`reset_and_exit` から以下の関数を抽出し、それぞれに**網羅的なユニットテスト**を書く。抽出関数が全てユニットテストで正しさを保証できれば、E2E テストは「ボタンを押したときの UI の反応」だけの確認に留められる。

```
抽出関数                                           テスト数(目安)  備考
──────────────────────────────────────────────────────────────────────────
delete_home_directory(path)                    →  8〜10ケース    リトライ・フォールバック
kill_process_on_port(port)                     →  4〜5ケース    正常系・異常系
kill_process_by_image_name(name)               →  3〜4ケース    新規抽出。taskkill /IM のラッパー
release_and_delete(lock_path, home_path)       →  4ケース       Windows: ロック解放→削除の結合
wait_for_process_exit(pids, timeout_ms)        →  3ケース       新規抽出。プロセス終了確認＋ポーリング
reset_and_exit_integration (軽量結合テスト)     →  1〜2ケース    ダミー子プロセスを使った結合検証
```

### 関数抽出とテスト詳細

#### 1. `delete_home_directory(path: &Path) -> Result<(), String>`

**抽出元**: `src/tauri_cmd/system.rs:682-777`（Phase 9 + Phase 10）

**責務**: MYCUTE_HOME の物理削除。リトライロジックと OS フォールバックを内包する。

**テストケース（`src/tauri_cmd/system.rs` 内 `#[cfg(test)] mod tests`）**:

| # | ケース | 分類 | 検証内容 |
|---|--------|------|----------|
| 1 | 空ディレクトリの削除 | 正常系 | `tempdir` 作成→削除→`exists() == false` |
| 2 | ファイル・サブディレクトリを含む削除 | 正常系 | 複数階層のファイルを作成→削除→存在確認 |
| 3 | 読み取り専用ファイルを含む削除 | 正常系 | Windows: `readonly`属性ファイル→削除可能か確認 |
| 4 | 存在しないパスの削除 | 異常系 | 不存在パス→`Ok(())` を返す（べき等） |
| 5 | 空文字パス | 異常系 | エラーを返す |
| 6 | リトライ上限到達時のエラー（モック代替） | 異常系 | 削除できないパス（仮想的にロック）→エラー文字列 |
| 7 | Windows: `release_lock()` 後に削除成功 | 統合 | ロックファイルが内部→解放→削除の流れ |
| 8 | フォールバック（`cmd /c rmdir`）の動作確認 | Windows特有 | 特殊パス文字を含むディレクトリ（参考: 現状のフォールバックが機能するか） |

注意: 
- 全テストは `tempfile::TempDir` または `std::env::temp_dir()` 配下で実施
- MYCUTE_HOME 実体には絶対に触れない
- テスト後は必ず後処理を行う（TempDir の Drop に任せる）

#### 2. `kill_process_by_image_name(image_name: &str) -> Result<(), String>`（新規抽出）

**抽出元**: `src/tauri_cmd/system.rs:649-658`（Phase 7.5）

**責務**: Windows の `taskkill /F /IM <image_name>` をラップし、終了コードとエラーメッセージを返す。

**配置**: `src/utils/process.rs` の公開関数として追加

**テストケース（`src/utils/process.rs` 内 `#[cfg(test)] mod tests`）**:

| # | ケース | 検証内容 |
|---|--------|----------|
| 1 | 存在しないプロセス名を指定 | エラーにならない（taskkill は「プロセスが見つからない」を返すが握りつぶすか、あるいは warn ログを出す） |
| 2 | 存在するプロセス名を指定 | テスト用子プロセス（`notepad.exe` 等ではなく単体 exe で代用）を起動→kill→プロセス終了確認 |
| 3 | 空文字 | `Err` を返す |

**補足**: `kill_process_on_port()` も同様に exit status を返すようにリファクタリングする。ただし現時点では exit status の返却にとどめ、呼び出し側でログ出力する形とする。

#### 3. `wait_for_process_exit(pids: &[u32], timeout: Duration) -> Vec<u32>`（新規抽出）

**抽出元**: `src/tauri_cmd/system.rs:660-669`（Phase 8）に相当する機能

**責務**: 指定された PID 群のプロセス終了をポーリングし、タイムアウト後に未終了の PID リストを返す。Windows では `tasklist /FI "PID eq ..."` で生存確認を行う。

**テストケース（`src/utils/process.rs` 内 `#[cfg(test)] mod tests`）**:

| # | ケース | 検証内容 |
|---|--------|----------|
| 1 | 既に終了した PID | 空リストが返る |
| 2 | タイムアウト内に終了するプロセス | 短命プロセス（`timeout /t 1` 等）→空リストが返る |
| 3 | タイムアウトしても終了しないプロセス | 永続プロセス→その PID がリストに含まれる |
| 4 | 空の PID リスト | 即座に空リストが返る |

#### 4. `release_and_delete(lock_path: &Path, target_path: &Path) -> Result<(), String>`（Windows のみ）

**抽出元**: `src/tauri_cmd/system.rs:671-777`（Phase 8.5 + Phase 9 + Phase 10）

**責務**: アプリロックファイルを解放し、ディレクトリを削除する。Windows では「ロック解放→削除」の順序が重要。

**テストケース（`system.rs` 内 `#[cfg(windows)]` テスト）**:

| # | ケース | 検証内容 |
|---|--------|----------|
| 1 | ロックファイルが削除対象ディレクトリ外 | 何も解放せず削除成功 |
| 2 | ロックファイルが削除対象ディレクトリ内 → release → 削除 | 解放後に削除成功 |
| 3 | ロックファイルが削除対象ディレクトリ内で release 忘れ | 削除失敗（エラー） |

#### 5. 軽量 Integration Test（`src/tauri_cmd/system.rs` 内 `#[cfg(test)]`）

**目標**: ダミーの子プロセス（テスト用に自作した最小バイナリ、または `ping -t` 等の標準コマンド）を起動し、抽出した関数群を使って「プロセス終了→ディレクトリ削除成功」までを一貫検証する。

| # | ケース | 検証内容 |
|---|--------|----------|
| 1 | ダミープロセスを起動→`kill_process_by_image_name` で kill→`wait_for_process_exit` で終了確認→`delete_home_directory` で削除成功 | プロセス終了→削除の流れ全体 |
| 2 | ダミープロセスが終了しない場合→`wait_for_process_exit` がタイムアウト→削除が失敗する | 異常系の確認 |

注意:
- このテストは `#[cfg(windows)]` のみで有効
- ダミープロセスとしてどのような手段を使うかは `/plan-ticket` で具体化する（例: `cmd /c "start /b ping -t 127.0.0.1"` と PID 指定 kill、または単体のテスト補助バイナリ）

### テストの隔離原則

1. ファイルシステム操作はすべて `std::env::temp_dir()` 配下の一意なディレクトリで実施
2. 既存の MYCUTE_HOME（`~/.mycute`）には一切触れない
3. 並列実行による競合を避けるため、Integration Test は `#[serial_test::serial]` で逐次実行する
4. 各テストの `tempdir` は `Drop` ハンドラで自動削除される
5. プロセス生成を伴うテストは、テスト終了時に必ず子プロセスが絶命していることを保証する（`Drop` ガード or `std::process::Command::output` で同期実行）

### テストでは検証しない領域

- Tauri ランタイムの起動（`app_handle.exit(0)`）
- 実際の Bifrost / ZeroClaw バイナリを使ったプロセス生成と終了
- フロントエンド（Vue.js）の UI レンダリング
- 実際の DB 操作（`do_reset_db()`）

## Boy Scout Rule — 翻訳可能性計画

- 削除リトライロジック（`src/tauri_cmd/system.rs:692-777`）を `delete_home_directory` 関数として抽出し、関数名で意図を語らせる
- `reset_and_exit` から責務を分割することで、「翻訳可能な散文」としての可読性を高める
- Phase 7.5 の `let _ = ` によるエラー握りつぶしを廃止し、`log::warn!` によるエラーログを追加する
- リトライ遅延の `[0, 200, 500, 1000]` を名前付き定数として定義する
- マジックナンバー `3000`（Phase 8 の待機時間）と `500` を名前付き定数にする
- Phase 9 のディレクトリ削除と Phase 10 の確認を一つの責務として関数化する

## Acceptance Criteria

- [ ] 下記5つの関数が `src/tauri_cmd/system.rs` および `src/utils/process.rs` から抽出され、ユニットテストで完全に検証されている
  - `delete_home_directory()` — リトライロジックとOSフォールバック
  - `kill_process_by_image_name()` — taskkill /F /IM のラッパー
  - `wait_for_process_exit()` — プロセス終了ポーリング
  - `release_and_delete()` — ロック解放→削除（Windowsのみ）
  - `kill_process_on_port()` — exit status 返却対応
- [ ] 軽量 Integration Test（ダミー子プロセス→kill→ディレクトリ削除）が1件以上追加され、通過している
- [ ] 追加されたテストの合計が **およそ 25〜35 ケース** に達している
- [ ] 全テストが `#[cfg(windows)]` でガードされ、macOS/Unix ではコンパイルされない（テスト対象外）
- [ ] macOS の既存コード（`src/tauri_cmd/system.rs` の Unix ブロック、`src/utils/auth.rs` の Unix ブロック）に一切変更が加えられていない
- [ ] Windows で「完全に初期化して終了」を手動実行した際、Bifrost / ZeroClaw が確実に終了し、MYCUTE_HOME ディレクトリが完全に削除される
- [ ] 全ての `taskkill` 呼び出しで exit status がログ出力され、エラー握りつぶしが解消されている
- [ ] `cargo test`（全環境）が通過する

## Notes

- 本チケットは Windows 専用である。すべてのコード変更は `#[cfg(windows)]` / `#[cfg(not(windows))]` でガードし、macOS（`#[cfg(target_os = "macos")]`）や Linux（`#[cfg(target_os = "linux")]`）には一切影響を与えないこと
- 修正後の検証手順:
  1. `cargo test` で既存テストが通過すること（全環境）
  2. 追加したユニットテストが通過すること（Windows）
  3. SettingsApp →「完全に初期化して終了」→ 確認ダイアログ → 実行 でホームディレクトリが完全に削除されアプリが終了すること（Windows 手動確認）
  4. 手動確認後、ログファイルにエラーが出力されていないことを確認
