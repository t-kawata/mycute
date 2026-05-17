---
ticket_id: 3
title: "Windows: Altキーに加えてCtrlキーでも操作可能に / macOS: Optionキーに加えてCmdキーでも操作可能に"
slug: windows-altctrl-macos-optioncmd
status: reviewed
created_at: 2026-05-17
updated_at: 2026-05-17
---
# Windows: Altキーに加えてCtrlキーでも操作可能に / macOS: Optionキーに加えてCmdキーでも操作可能に

## Summary

Alt（Windows）/ Option（macOS）の BufferFlush に加え、Ctrl（Windows）/ Cmd（macOS）のシングルタップでも BufferFlush を実行できるようにする。Ctrl/Cmd に録音開始（ダブルタップ）機能は持たせず、BufferFlush 専用とする。また、タイトルバー右端のフローティングアクションボタンに当機能の有効/無効トグルを追加し、無効時は Ctrl/Cmd に一切干渉しない。

Alt（Windows）/ Option（macOS）の既存動作（ダブルタップ録音開始 + シングルタップ BufferFlush）は一切変更しない。

## Background

### 問題: 単一キーでは競合を回避できない

チケット #1 で Alt キーの排他制御は実装されたが、根本的に「Alt キーそのものがアプリに捕捉される」問題は解決できない。Raw Input API を使用するアプリ（VS Code、Windows Terminal、Chrome、Electron アプリ等）では WH_KEYBOARD_LL でブロックしても Alt UP 注入により被害は最小化されるが、以下の問題が残る:

- **アプリの UX が損なわれる**: ユーザーが Alt キーを意図した操作（VS Code の Alt+Click マルチカーソル、Alt+D アドレスバー移動等）に使いたい場合、MYCUTE が Alt をブロックするためアプリの機能が使えなくなる
- **単一キーでは選択肢がない**: 現状 Alt 一択のため、競合を回避する方法がない

### 解決策: Ctrl/Cmd を BufferFlush の代替キーとして追加

Ctrl（Windows）/ Cmd（macOS）を BufferFlush 専用の代替キーとして追加する。録音中に Ctrl/Cmd をシングルタップすると BufferFlush が実行される。非録音中は Ctrl/Cmd に何の効果もなく、そのままアプリに通過する。

ダブルタップ（録音開始）は持たせない。これは Ctrl がコピペ（Ctrl+C/V）等で日常的に連打される修飾子であり、ダブルタップの誤検出リスクが Alt より著しく高いためである。

さらに、タイトルバー右端のフローティングアクションボタンに有効/無効トグルを追加することで、ユーザーが Ctrl/Cmd BufferFlush を完全に無効化できるようにする。無効時は MYCUTE は Ctrl/Cmd に一切干渉せず、フック内でもスキップされる。

### 既存アーキテクチャの分析

Alt の状態マシンは 3 つの実行コンテキストに分散している（`src/hotkey_win_hook.rs` × `src/hotkey_win.rs`）。Ctrl 対応では BufferFlush 関連の処理のみを複製する。

**WH_KEYBOARD_LL フック**（`src/hotkey_win_hook.rs`）:

Alt 専用の状態変数:
- `HOOK_ALT_DOWN_BLOCKED: AtomicBool` — ブロック済み Alt DOWN に対応する UP もブロックする
- `HOOK_ALT_REPEAT: AtomicBool` — オートリピート検出ガード
- `process_alt_down()` → `is_alt_repeat()` / `update_modifier_state()` / `handle_recording_alt()` / `is_double_tap_detected()` / `confirm_double_tap()` / `update_last_press_time()`
- `process_alt_up()` → 保留アクション送信 + 状態リセット
- `inject_alt_up()` → SendInput + MYCUTE_EVENT_TAG による防御的 Alt UP 注入

Ctrl（`VK_CONTROL = 0x11`）は現状 `track_other_modifier()` 内で `CURRENT_MODIFIERS` の `MOD_CTRL` ビットを設定するのみ。フラッシュ予約・イベントブロック・UP 注入は一切行われていない。

**共有状態**（`src/hotkey_win.rs`）:
- `PENDING_ALT_START: AtomicBool` / `PENDING_ALT_FLUSH: AtomicBool` / `LAST_ALT_PRESS_TIME: AtomicU64`
- `alt_monitor_thread()` — `GetAsyncKeyState(VK_MENU)` のエッジ検出による状態管理

## Scope

### 共通: Ctrl/Cmd BufferFlush 有効/無効トグル

タイトルバー右端のフローティングアクションボタンに、Ctrl（Windows）/ Cmd（macOS）の BufferFlush 機能を有効/無効にするトグルスイッチを追加する。

- **無効（デフォルト）**: MYCUTE は Ctrl/Cmd キーに一切干渉しない。フック内でも Ctrl イベントは常に通過する。既存の `track_other_modifier()` による修飾子ビット追跡（他キーとのコンボ検出用）は継続する
- **有効**: 録音中（`RECORDING_ACTIVE == true`）の Ctrl/Cmd シングルタップで BufferFlush を実行し、Ctrl/Cmd イベントをブロックする。非録音中は Ctrl/Cmd は何もしない（通過させる）

トグルの状態は Tauri の State もしくは共有 atomic フラグとして保持し、フックや rdev リスナーから参照可能にする。

### Windows 対応

1. **共有 atomic フラグの追加**: `CTRL_FLUSH_ENABLED: AtomicBool`, `PENDING_CTRL_FLUSH: AtomicBool`, `HOOK_CTRL_DOWN_BLOCKED: AtomicBool`
   - `PENDING_ALT_START` / `LAST_ALT_PRESS_TIME` 等の録音開始関連フラグは Ctrl 用に複製しない（Ctrl にダブルタップはないため）
2. **WH_KEYBOARD_LL フックに Ctrl BufferFlush 処理を追加**
   - `hook_proc()` の `WM_KEYDOWN` 分岐に `VK_CONTROL` 検出を追加
   - 関数: `process_ctrl_down()`, `process_ctrl_up()`, `inject_ctrl_up()`
   - `process_ctrl_down()` のロジック: `CTRL_FLUSH_ENABLED` AND `RECORDING_ACTIVE` → `PENDING_CTRL_FLUSH = true` + イベントブロック + Ctrl UP 注入。それ以外 → 通過
3. **rdev リスナー（`hotkey_win.rs handle_event()`）に Ctrl BufferFlush 処理を追加**
   - ただし WH_KEYBOARD_LL フックがイベントをブロックするため、rdev パスはフックがインストールされていない場合のフォールバックとして機能
4. **フローティングアクションボタンにトグルスイッチを追加**（フロントエンド）
   - トグル状態を Tauri コマンド経由でバックエンドの `CTRL_FLUSH_ENABLED` に反映

### macOS 対応

1. **共有 atomic フラグの追加**: `CMD_FLUSH_ENABLED: AtomicBool`, `PENDING_CMD_FLUSH: AtomicBool`
2. **`CGEventTap` コールバック（`hotkey_mac.rs`）に Cmd BufferFlush 処理を追加**
   - `kVK_Command`（0x37: 左Cmd, 0x38: 右Cmd）の検出
   - `CMD_FLUSH_ENABLED` AND `RECORDING_ACTIVE` のとき BufferFlush を予約 + イベントブロック
3. **フローティングアクションボタンにトグルスイッチを追加**（Windows と共用 UI）
4. rdev リスナー（macOS）にも同様の処理を追加（CGEventTap のフォールバック用）

### 送信アクション

Ctrl/Cmd は `HotkeyAction::BufferFlush` のみを送信する。`HotkeyAction::Start` は送信しない（Ctrl/Cmd で録音開始はできない）。アクション受信側の変更は不要。

## Non-scope

- 設定画面でのキー選択 UI（カスタムキー割り当て、リマップ）
- Shift / Win キーの追加サポート
- 既存の Alt（Windows）/ Option（macOS）の動作変更（ダブルタップ録音開始 + シングルタップフラッシュは維持）
- 3 つ以上の修飾キー同時サポート
- Ctrl/Cmd での録音開始（ダブルタップ）

## Investigation

### Windows の Ctrl 処理現状

`hook_proc()` での Ctrl の扱いは以下のみ:

```rust
// WM_KEYDOWN/WM_SYSKEYDOWN の場合（Alt 以外）
track_other_modifier(kb.vk_code, true);  // 単に MOD_CTRL ビットをセット

// WM_KEYUP/WM_SYSKEYUP の場合
match kb.vk_code {
    VK_CONTROL | VK_SHIFT | VK_LWIN | VK_RWIN => {
        track_other_modifier(kb.vk_code, false);  // 単に MOD_CTRL ビットをクリア
    }
    _ => {}
}
```

Ctrl は Alt と全く異なるコードパスを通っており、ダブルタップ検出もブロックも行われない。Ctrl に Alt と同等の機能を持たせるには、Alt の全状態マシンを Ctrl 用に複製する必要がある。

### macOS の Cmd 処理現状

`hotkey_mac.rs` では `CGEventTap` コールバック（`event_tap_callback()`）で `CGEvent.getIntegerValueField(kCGKeyboardEventKeycode)` を取得し、`kVK_Option`（0x3A, 0x3D）の状態を監視している。Cmd キー（`kVK_Command` = 0x37, 0x38）に対する同様の処理は存在しない。Option と同様の状態マシンを Cmd 用に実装する必要がある。

### 二重発火のリスク分析

Alt と Ctrl の両方で同時に BufferFlush が送信されるシナリオ:
- ユーザーが Alt と Ctrl を同時に押した場合

対策:
- WH_KEYBOARD_LL フック内で先に Alt か Ctrl のどちらか一方がブロックされれば、もう一方のイベントはフックプロシージャの別の呼び出しとして処理される
- `HOTKEY_SENDER` は `Mutex<Option<SyncSender>>` ＋ `try_send()` で直列化される
- 二重 BufferFlush が発生しても、バッファが空の場合は何も貼り付けられない（アプリ側で安全）
- 念のため `send_action()` 内に短時間（〜50ms）の重複送信ガードを追加する

### トグルフラグの設計

```rust
/// Ctrl（Windows）/ Cmd（macOS）の BufferFlush 機能が有効かどうか。
/// true: 録音中に Ctrl/Cmd シングルタップで BufferFlush を実行する
/// false: Ctrl/Cmd に一切干渉しない（デフォルト）
pub static CTRL_CMD_FLUSH_ENABLED: AtomicBool = AtomicBool::new(false);
```

- デフォルトは `false`（無効）。有効にしたユーザーのみが Ctrl/Cmd BufferFlush を使う
- フローティングアクションボタンのトグル→Tauri コマンド→`CTRL_CMD_FLUSH_ENABLED` に反映
- トグルの状態はアプリ再起動でリセット（設定に保存しない）

## Test Plan

### 全体方針

Alt/Option の既存動作を変えずに Ctrl/Cmd BufferFlush が追加されることを確認する。Ctrl/Cmd は BufferFlush のみで録音開始（ダブルタップ）はないため、テストケースは Alt に比べて少ない。

### テスト対象

| テスト | 分類 | 内容 |
|--------|------|------|
| トグル OFF → Ctrl 通過（録音中） | 単体 | `CTRL_CMD_FLUSH_ENABLED=false` かつ `RECORDING_ACTIVE=true` のとき、Ctrl はブロックされず通過する |
| トグル OFF → Ctrl 通過（非録音中） | 単体 | `CTRL_CMD_FLUSH_ENABLED=false` かつ `RECORDING_ACTIVE=false` のとき、Ctrl はブロックされず通過する |
| トグル ON + 録音中 → Ctrl ブロック＋フラッシュ | 単体 | `CTRL_CMD_FLUSH_ENABLED=true` かつ `RECORDING_ACTIVE=true` で Ctrl 押下時、`PENDING_CTRL_FLUSH` がセットされイベントがブロックされる |
| トグル ON + 非録音中 → Ctrl 通過 | 単体 | `CTRL_CMD_FLUSH_ENABLED=true` かつ `RECORDING_ACTIVE=false` のとき、Ctrl はブロックされず通過する（Ctrl は録音中のみ効果を持つ） |
| Ctrl UP → BufferFlush 送信 | 単体 | `PENDING_CTRL_FLUSH=true` の UP で `HotkeyAction::BufferFlush` が送信される |
| Alt / Ctrl 二重発火防止 | 単体 | 両方から同時に BufferFlush が二重送信されない |
| Self-event スキップ | 単体 | `dw_extra_info == MYCUTE_EVENT_TAG` の Ctrl イベントはスキップされる |
| Cmd DOWN → 録音中フラッシュ | macOS単体 | `CMD_FLUSH_ENABLED=true` かつ `RECORDING_ACTIVE=true` で Cmd 押下時、BufferFlush が予約される |
| Cmd DOWN → 非録音中 → 通過 | macOS単体 | 非録音中は Cmd がブロックされず通過する |
| フローティングトグル → Tauri State 反映 | 結合 | トグルの ON/OFF が Tauri コマンド経由で `CTRL_CMD_FLUSH_ENABLED` に正しく反映される |
| 既存 Alt 動作への影響なし | 回帰 | Ctrl 対応後も Alt ダブルタップ録音開始＋シングルタップフラッシュが従来通り動作する |

### テスト方針

- atomic 変数の操作テストは `SeqCst` オーダリングの検証と共に記述
- `send_action()` は `HOTKEY_SENDER` 経由のため、テスト用レシーバを設定して送信内容を検証
- フック内関数は `pub` または `pub(crate)` に変更してテスト可能にする
- macOS テストは `#[cfg(target_os = "macos")]` でガード
- フロントエンドのトグル UI テストは本チケットのスコープ外（必要に応じて別チケット）

## Boy Scout Rule — 翻訳可能性計画

1. **関数名の対称性確保**: Ctrl/Cmd の BufferFlush 処理関数は Alt の同名関数と対称な命名にする
   - `process_alt_down()` ↔ `process_ctrl_down()`
   - `handle_recording_alt()` ↔ `handle_recording_ctrl()`
   - `inject_alt_up()` ↔ `inject_ctrl_up()`
   - Alt/Ctrl 間で引数・戻り値・副作用の一貫性を保つ

2. **BufferFlush 共通ロジックの抽出**: 録音中の修飾キー処理（`PENDING_FLUSH` セット＋イベントブロック＋UP 注入）は Alt と Ctrl で同一のため、汎用関数に抽出して重複を排除する:
   ```rust
   fn handle_recording_modifier(pending_flush: &AtomicBool, blocked: &AtomicBool, vk_code: u16) {
       pending_flush.store(true, Ordering::SeqCst);
       blocked.store(true, Ordering::SeqCst);
       inject_modifier_up(vk_code);  // 引数で VK_MENU or VK_CONTROL を切り替え
   }
   ```

3. **トグルフラグの命名**: `CTRL_FLUSH_ENABLED` ではなく `CTRL_CMD_FLUSH_ENABLED` とし、Windows と macOS で共用する意図を名前に含める

4. **マジックナンバー排除**: `kVK_Command` の値（0x37, 0x38）はコード内で直接使用せず、名前付き定数として定義する

## Acceptance Criteria

### トグル機能
- [ ] タイトルバー右端のフローティングアクションボタンに Ctrl/Cmd BufferFlush の有効/無効トグルが表示される
- [ ] トグル OFF（デフォルト）のとき、Ctrl/Cmd に MYCUTE は一切干渉しない
- [ ] トグル ON のとき、録音中 Ctrl/Cmd シングルタップで BufferFlush が実行される
- [ ] トグルの ON/OFF が Tauri コマンド経由でバックエンドの atomic フラグ `CTRL_CMD_FLUSH_ENABLED` に反映される

### Windows
- [ ] トグル ON + 録音中: Ctrl シングルタップで BufferFlush が実行される
- [ ] トグル ON + 録音中: Ctrl がフラッシュ先アプリに漏れない（Alt と同等の排他）
- [ ] トグル ON + 非録音中: Ctrl は何もせず通過する
- [ ] Alt の既存動作（ダブルタップ録音開始 + シングルタップフラッシュ）に変更がない

### macOS
- [ ] トグル ON + 録音中: Cmd シングルタップで BufferFlush が実行される
- [ ] トグル ON + 非録音中: Cmd は何もせず通過する
- [ ] Option の既存動作（ダブルタップ録音開始 + シングルタップフラッシュ）に変更がない

### 安全性
- [ ] 自己生成イベント（MYCUTE の SendInput Ctrl+V）が Ctrl 状態マシンを誤起動しない
- [ ] Alt と Ctrl（または Option と Cmd）の同時押下で二重 BufferFlush が発生しない
- [ ] トグルの ON/OFF 切り替え中にレースコンディションが発生しない
- [ ] `cargo build --release` が Windows / macOS 両方で通る
- [ ] 既存テストがすべて通過している

## Known Risks / Mitigations

| リスク | 影響 | 緩和策 |
|--------|------|--------|
| 録音中に Ctrl/Cmd がブロックされるため、ユーザーが Ctrl+C/V 等を使えなくなる | 録音中の作業効率低下 | デフォルト OFF のため、有効にしたユーザーのみ影響。フローティングトグルで即座に無効化可能。また録音時間は通常短いため実害は限定的 |
| macOS Cmd キーはシステムショートカット（Cmd+Tab, Cmd+Space）の修飾子 | 誤検出の可能性 | 単体 Cmd タップのみ検出（Cmd+他キーのコンボ中は無視）。録音中のみブロックするため非録音中は影響なし。CGEventTap のイベントフィルタで適切に制御 |
| フローティングトグルとバックエンドフラグの同期ずれ | トグルの状態と実際の動作が一致しない | Tauri コマンドは同期的に処理され、コマンド完了後にフラグが更新される。invoke の応答を待ってから UI を更新する |
| トグル ON のままアプリを終了すると、次回起動時に録音中 Ctrl が効かない | — | デフォルト OFF のため問題なし。トグル状態は永続化しないため、再起動後は常に無効状態で開始 |

## Notes

### 調査で特定した全変更箇所

| ファイル | 変更概要 | 備考 |
|----------|----------|------|
| `src/hotkey_win.rs` | `CTRL_CMD_FLUSH_ENABLED`, `PENDING_CTRL_FLUSH` の追加。`handle_event()` に Ctrl BufferFlush 処理追加 | ダブルタップ関連の状態変数（`PENDING_CTRL_START`, `LAST_CTRL_PRESS_TIME`）は不要 |
| `src/hotkey_win_hook.rs` | `HOOK_CTRL_DOWN_BLOCKED` の追加。`hook_proc()` に `VK_CONTROL` 分岐追加。`process_ctrl_down()`, `process_ctrl_up()`, `inject_ctrl_up()` の追加 | `HOOK_CTRL_REPEAT` は不要（リピート検出はブロック処理に影響しないため）。`stop_hook()` で Ctrl フラグもクリア |
| `src/hotkey_mac.rs` | `CMD_FLUSH_ENABLED`, `PENDING_CMD_FLUSH` の追加。`event_tap_callback()` に `kVK_Command` 分岐追加。Cmd BufferFlush 処理関数追加 | ダブルタップ関連の状態変数は不要 |
| `web/src/`（フロントエンド） | フローティングアクションボタンに Ctrl/Cmd BufferFlush トグルスイッチを追加。Tauri コマンド呼び出し | トグル状態の永続化は不要（再起動でリセット） |
| `src/tauri_cmd/system.rs` または新規コマンド | トグルの ON/OFF を受け取り `CTRL_CMD_FLUSH_ENABLED` に反映する Tauri コマンド | 最小限のコマンド |

### 「安全」である根拠

以下の設計判断により、Ctrl/Cmd BufferFlush は安全に動作する:

1. **デフォルト OFF**: ユーザーが明示的にトグルを ON にしない限り、Ctrl/Cmd は完全にパススルー。既存の動作に影響なし
2. **録音中のみ作用**: 非録音中は Ctrl/Cmd に何の効果もない。誤検出のリスクはゼロ
3. **ダブルタップなし**: Ctrl/Cmd の連打（Ctrl+C → Ctrl+V）による誤検出は原理的に発生しない
4. **自己イベントスキップ**: `MYCUTE_EVENT_TAG` により MYCUTE 自身が生成した Ctrl+V が Ctrl BufferFlush を呼び出すことはない
5. **Alt への影響なし**: Ctrl 処理は Alt 状態マシンと完全に独立しているため、Alt の既存動作が変化することはない

### Ctrl/Cmd 同時押下の動作

Alt + Ctrl を同時に押した場合:
- 各キーは独立した状態マシンを持つ
- 両方の DOWN が検出され、それぞれ BufferFlush が予約される可能性がある
- `send_action()` 内の短時間（〜50ms）重複ガードにより、二重 BufferFlush を防止する

### フローティングアクションボタンのトグル配置（設計案）

現状のフローティングアクションボタン群（設定アイコン等）に、Ctrl/Cmd BufferFlush トグルをアイコンとして追加:
- 無効時: `⌨️` または `Ctrl` の薄表示
- 有効時: `⌨️` または `Ctrl` のハイライト表示
- ツールチップ: "Ctrl/Cmd BufferFlush: 有効" / "Ctrl/Cmd BufferFlush: 無効"
- クリックで ON/OFF トグル
