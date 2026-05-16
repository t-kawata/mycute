---
ticket_id: 1
title: "Windows: 音声認識中・補正中に Alt キーの排他が効かずフラッシュ先アプリにキーが奪われる"
slug: windows-alt
status: done
created_at: 2026-05-16
updated_at: 2026-05-16
---
# Windows: 音声認識中・補正中に Alt キーの排他が効かずフラッシュ先アプリにキーが奪われる

## Summary

Windows 環境で音声認識中または AI 補正中に Alt キーを 1 回押してフラッシュ（確定テキストの貼り付け）を試みると、WH_KEYBOARD_LL フックによる Alt キーのブロックが効かず、フラッシュ先アプリケーションのキーバインドに Alt が奪われる問題を修正する。

## Background

### 期待される動作

Alt ダブルタップで録音を開始後、`RECORDING_ACTIVE = true` の間、WH_KEYBOARD_LL フックが Alt キーの押下イベントを捕捉し `1` を返してブロックする。フラッシュ先アプリには Alt イベントが一切到達しない。Alt シングルタップで `BufferFlush` アクションが発行され、認識テキストをクリップボード経由でフラッシュ先アプリに貼り付ける。

### 問題の観測

- **macOS**: `CGEventTap` が CoreGraphics レベルで全キーボードイベントを捕捉・ブロックする。認識中・補正中の区別なく常に排他が効く。
- **Windows**: 認識中・補正中に限り Alt キーがフラッシュ先アプリに漏れる。認識/補正が完了した状態（`is_stt_pending == false && is_post_correcting == false`）では正しく排他できる。

### コード解析による状態の確認

`RECORDING_ACTIVE` は以下のタイミングでのみ変更される:
- `set_recording_active(true)` — `HotkeyAction::Start` ハンドラ（`system.rs:209`）
- `set_recording_active(false)` — 即時 BufferFlush 完了時（`system.rs:337`）、または遅延 BufferFlush 完了時（`main_of_cl.rs:714, 882`）

認識中・補正中は `RECORDING_ACTIVE = true` が維持されているため、コード上のロジックだけを見れば WH_KEYBOARD_LL フックはブロックすべき状態にある。

### 根本原因の詳細

**WH_KEYBOARD_LL フックは WM_SYSKEYDOWN/WM_KEYDOWN の VK_MENU (Alt) のみをブロックする。しかし、これは以下の2つの経路でバイパスされる:**

1. **Raw Input API (`WM_INPUT`)**: モダンな Windows アプリケーション（VS Code、Windows Terminal、Chrome、Electron アプリ等）は `RegisterRawInputDevices` を使用してキーボード入力を直接受信する。これらのアプリは `RIDEV_NOLEGACY` フラグを設定することで、WM_SYSKEYDOWN/WM_KEYDOWN のレガシーメッセージをそもそも受信しない。WH_KEYBOARD_LL が VK_MENU をブロック（戻り値 1）しても、Raw Input 経由で配送される `WM_INPUT` はブロックできない。Alt キーの押下情報は OS レベルで認識されており、アプリケーションは WM_INPUT 経由で Alt の押下状態を検出できる。

2. **WM_SYSKEYDOWN のオートリピート漏れ**: `process_alt_down()` 内の `HOOK_ALT_REPEAT` ガードにより、Alt キーのオートリピート WM_SYSKEYDOWN はブロックされずに通過する（`return CallNextHookEx(...)`）。ユーザーが Alt を押し続ける時間が Windows のキーリピート閾値（標準約 500ms）を超えると、リピートメッセージがフラッシュ先アプリに到達し、Alt メニュー等を起動する可能性がある。

3. **フックの自動削除（Windows のタイムアウト）**: Windows は低レベルフックが所定時間内に応答しない場合、フックをサイレントに解除する（MSDN: "The system may pass the message to the target window procedure if the hook procedure does not return within a certain time"）。認識中・補正中は AI API 呼び出しや音声処理等によりシステム全体の負荷が高まり、フックスレッドのメッセージポンプが遅延する可能性がある。フックが解除された後は Alt イベントは完全に素通りする。

4. **`start_hook()` の戻り値握りつぶし**: `SetWindowsHookExW` が失敗しても `start_hook()` は常に `true` を返す。呼び出し元（`system.rs:183`）でも戻り値をチェックしていない。フックのインストール自体に失敗した場合、Alt ブロック機構が全く存在しない状態で録音が開始される。

### 検証（ユーザー報告とコードの整合性）

| 状態 | `RECORDING_ACTIVE` | フックによるブロック | 観測結果 |
|------|-------------------|---------------------|---------|
| 録音中（認識/補正なし） | true | 期待通り動作 | ブロック成功 ✓ |
| 録音中（認識処理中） | true | Raw Input/リピート/タイムアウトでバイパス | ブロック失敗 ✗ |
| 録音中（補正処理中） | true | Raw Input/リピート/タイムアウトでバイパス | ブロック失敗 ✗ |
| 録音完了後 | false | ブロック不要 | — |

「認識/補正完了時はブロックできる」理由: 認識/補正が完了した直後はシステム負荷が低下し、フックが正常応答する。また、ユーザーが Alt を押すタイミングがフックのタイムアウト圏外になる。

## Scope

### 1. 防御的 Alt UP インジェクション（WH_KEYBOARD_LL フック内）

WH_KEYBOARD_LL フックの `process_alt_down()` で `RECORDING_ACTIVE == true` を検出した際、`SendInput` による **Alt UP の強制注入** を行う。これにより:
- 万が一 Alt DOWN がフラッシュ先アプリに漏れた場合でも、即座に Alt UP が注入される
- 結果としてフラッシュ先アプリは「Alt が一瞬押された」ように見えるだけで、Alt 系ショートカットは発動しない
- WH_KEYBOARD_LL によるブロック（戻り値 1）との二重防御となる

SendInput で注入するイベントには `dw_extra_info` にマーカー値（`0x4D594355 = MYCUTE`）を設定し、フック内で自己イベントをスキップする（再帰防止）。

### 2. フックライフサイクルの堅牢化

- `start_hook()` の戻り値を `bool` または `Result<bool, Error>` に変更
- 呼び出し元 `enable_hotkey_standby()` で戻り値をチェックし、フックインストール失敗時はログにエラー出力＋録音開始を拒否
- `HOOK_ACTIVE` を定期的に監視するヘルスチェック機構の追加（15秒間隔で確認）
- ヘルスチェックでフックが死んでいることを検出した場合、自動再インストールを試みる

### 3. ホットキーイベントの合成/実イベント区別

- `Input.dw_extra_info` フィールドにマジックナンバー `0x4D594355` を設定するユーティリティ関数を実装
- `KeyboardInjector::send_ctrl_key_inner()` 他、SendInput を使用する全関数でマーカーを設定
- WH_KEYBOARD_LL フック内で `dw_extra_info` をチェックし、自己生成イベントはスキップ

### 4. state管理の検証（ログ出力強化）

- `RECORDING_ACTIVE` の状態変化を debug ログに出力
- `PENDING_ALT_FLUSH` と連動する状態変化のログ強化
- `is_stt_pending` / `is_post_correcting` の状態と `RECORDING_ACTIVE` の一貫性を検証可能にする

## Non-scope

- **macOS 側の排他機構**: macOS では問題が発生していないため対象外
- **フラッシュ以外の機能**: Correct / Summarize ホットキーコンボの排他ロジックは変更しない
- **`disable_ime()` / `restore_ime()`**: IME 制御ロジックは変更しない
- **rdev / GetAsyncKeyState パスの変更**: これらの検出専用パスはブロック能力を持たないため、変更不要

## Implementation Plan

### Step 1: 合成イベントマーカーの導入

**対象ファイル**: `src/input/keyboard_win.rs`

SendInput 構造体に `dw_extra_info` マーカーを追加:
- 定数 `MYCUTE_EVENT_TAG: usize = 0x4D594355` を定義
- `Input` 構造体の `ki.dw_extra_info` にマーカーを設定する補助関数 `fn mark_mycute_event(input: &mut Input)` を実装
- `send_ctrl_key_inner()` 内の全 4 つの Input にマーカーを設定
- `type_text_sendinput()` 内の全 Input にマーカーを設定
- `send_backspaces_inner()` 内の全 Input にマーカーを設定

WH_KEYBOARD_LL フック内で `dw_extra_info == MYCUTE_EVENT_TAG` の場合は `CallNextHookEx` にパススルーする。

### Step 2: 防御的 Alt UP インジェクション

**対象ファイル**: `src/hotkey_win_hook.rs`

`process_alt_down()` 内で `RECORDING_ACTIVE == true` を検出した直後、`SendInput` による Alt UP 注入:
```rust
unsafe fn inject_alt_up() {
    let mut input: Input = std::mem::zeroed();
    input.input_type = INPUT_KEYBOARD;
    input.ki.w_vk = VK_MENU;  // 0x12
    input.ki.dw_flags = KEYEVENTF_KEYUP;
    input.ki.dw_extra_info = MYCUTE_EVENT_TAG;
    SendInput(1, &input, size_of::<Input>() as i32);
}
```

ただし WH_KEYBOARD_LL フックのファイルは現在 raw Windows API のみを使用しており、`Input` / `KeybdInput` / `SendInput` / `KEYEVENTF_KEYUP` の定義がない。これらを追加する必要がある。あるいは、同じ `user32` FFI 宣言内に SendInput を追加する。

設計判断: `hotkey_win_hook.rs` 内に `SendInput` / `KEYEVENTF_KEYUP` / `VK_MENU` / `Input` / `KeybdInput` を追加する（`keyboard_win.rs` の型定義と重複するが、依存関係の循環を避けるため）。

### Step 3: フックの結果を呼び出し元で検証

**対象ファイル**: `src/hotkey_win_hook.rs`, `src/tauri_cmd/system.rs`

変更:
- `start_hook()` → `Result<(), String>` を返すように変更
- `enable_hotkey_standby()` 内で戻り値をチェック:
```rust
#[cfg(windows)]
if let Err(e) = hotkey_win_hook::start_hook() {
    log::error!("Critical: WH_KEYBOARD_LL hook failed to install: {}. Alt blocking will be unavailable.", e);
    // フックが無くても録音は可能（機能制限付き）
}
```

### Step 4: ヘルスチェックの追加

**対象ファイル**: `src/hotkey_win_hook.rs`

- `HOOK_ACTIVE` を監視する `check_hook_health()` 関数を追加
- この関数はホットキーハンドラループ（`system.rs` の `while let Some(action) = hk_rx.recv().await` 内）から定期的に呼び出す
- `HOOK_ACTIVE == false` かつ `is_hotkey_active == true` の場合、再インストールを試みる

## Architecture Considerations

### なぜ SendInput の Alt UP 注入が Raw Input 問題を回避できるのか

Raw Input でキーボード入力を受信するアプリケーションは、Alt キーの押下状態を `WM_INPUT` 経由で取得する。WH_KEYBOARD_LL で WM_SYSKEYDOWN/VK_MENU をブロックしても、Raw Input 経由の Alt 押下情報はアプリに到達する。しかし `SendInput` による Alt UP は OS レベルのキー状態も変更するため、アプリが Raw Input 経由で参照するキー状態も Alt UP に更新される。結果として:
- アプリは Alt が押されたことを検知する時間的猶予がほぼない
- Alt UP が直後に注入されるため、アプリのキーバインド処理（例: Alt → メニュー活性化）は開始されない
- Hook のブロック（戻り値 1）と合わせて二重防御となる

### 自己イベントの再帰防止

`dw_extra_info` マーカーにより、SendInput が生成したイベントが再び WH_KEYBOARD_LL フックのコールバックに到達した際に自己イベントと識別し、無限ループを防止する。

## Boy Scout Rule — 翻訳可能性計画

1. **`process_alt_down()` 関数の責務分割**:
   - `is_alt_repeat()` — リピート検出
   - `handle_recording_alt()` — 録音中の Alt ブロック + Alt UP 注入
   - `handle_double_tap_detection()` — ダブルタップ検出
   - `update_modifier_state()` — 修飾子状態の更新

2. **`start_hook()` の戻り値型変更**: `bool` → `Result<(), String>`

3. **ハードコード値の定数化**: `0x11`(VK_CONTROL), `0x10`(VK_SHIFT), `0x5B/0x5C`(VK_LWIN/VK_RWIN) を名前付き定数に

4. **`Input` 構造体の重複定義**: `keyboard_win.rs` と `hotkey_win_hook.rs` で同じ構造体を定義することになる。共通の `windows_types.rs` または既存モジュールへの抽出を検討する。

## Acceptance Criteria

- [ ] Windows 環境で音声認識中に Alt キーを押してもフラッシュ先アプリに Alt イベントが漏れない（Raw Input 使用アプリ含む）
- [ ] Windows 環境で AI 補正中に Alt キーを押してもフラッシュ先アプリに Alt イベントが漏れない（Raw Input 使用アプリ含む）
- [ ] フラッシュ（clipboard paste）は認識/補正完了後に正しく実行される
- [ ] macOS 側に影響がない（従来通り動作）
- [ ] フックインストール失敗時に関数が握りつぶさずにログエラーを出力する
- [ ] 自己生成イベント（合成 Alt UP / 合成 Ctrl+V）が WH_KEYBOARD_LL フックで誤ブロックされない
- [ ] `cargo build --release` が Windows ターゲットで通る
- [ ] 既存テストが通過している

## Known Risks / Mitigations

| リスク | 影響 | 緩和策 |
|--------|------|--------|
| `dw_extra_info` マーカーが他アプリの入力と衝突 | 誤検出によるブロック漏れ | マーカー値 `0x4D594355` はASCII "MYCU" に相当し偶然一致する可能性は極めて低い |
| SendInput の Alt UP 注入によりシステムキー状態が不整合 | アプリの Alt メニュー動作異常 | 注入するのは録音中のみ。録音終了後は `VK_MENU` のキー状態は通常のユーザー入力でリセットされる |
| フックの自動再インストールによる性能低下 | 無視できる程度 | ヘルスチェック間隔は最小 15 秒に設定 |

## Notes

### 調査で発見した関連コードパス

- `process_alt_down()` — `src/hotkey_win_hook.rs:249-283`（Alt DOWN 処理、ブロック判定）
- `process_alt_up()` — `src/hotkey_win_hook.rs:287-309`（Alt UP 処理、アクション発火）
- `start_hook()` — `src/hotkey_win_hook.rs:120-171`（フックのインストール、戻り値握りつぶし）
- `KeyboardInjector::send_ctrl_key_inner()` — `src/input/keyboard_win.rs:366-392`（Ctrl+V の SendInput）
- `BufferFlush` ハンドラ — `src/tauri_cmd/system.rs:299-349`（遅延フラッシュ分岐）
- 遅延フラッシュ実行 — `src/mode/cl/main_of_cl.rs:677-730`（PostCorrectionFinished）, `845-894`（SttCompleted）
