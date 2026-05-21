# 音声入力 UX 変更: RealTime → Buffer & Flush (Clipboard Paste)

## 0. 以下の計画書の前提
- v0.24.29 というバージョンのソースコードを前提として計画している
- 行番号は v0.24.29 のソースコードを基準として記載

## 1. 背景と目的

### 1.1 現状の UX

- **開始**: Option (macOS) / Alt (Windows) キーのダブルタップ → 開始音 → 音声認識開始
- **入力方式**: 認識結果がリアルタイムで `KeyboardInjector::input_diff()` によりカーソル位置に逐次打鍵される（macOS は CGEvent Unicode、Windows はクリップボード経由 Ctrl+V）
- **終了**: 録音中に他のキー入力やマウスクリックが発生すると自動的に `HotkeyAction::Commit` が発行され、認識が終了する
- **補正**: PostCorrection（LLMによる最終補正）実行中は `→ Correcting …` という文字列が装飾として打ち込まれ、補正完了後に差し替えられる

### 1.2 望ましい UX

- **開始**: Option/Alt ダブルタップ → 開始音 → 音声認識開始（バッファリングモード）
- **入力方式**: 認識結果はキーボードに注入せず、メモリ上のバッファに蓄積されるのみ。オーバーレイUIへの表示は継続
- **終了**: **一度の Option/Alt キー押下**（シングルタップ）で、蓄積された全文をカーソル位置に一気にフラッシュし、認識を終了する（終了音再生）
- **補正中の安全性**: LLM補正が実行中の場合はフラッシュを待機し、補正完了後に補正済み全文をフラッシュする
- **自動コミットの廃止**: マウスクリックや他キー入力による自動コミットは一切行わない

### 1.3 クリップボード方式への統一（Mac 新規導入）

- macOS 側も Windows と同様に、フラッシュ時にクリップボードにテキストをセット → Cmd+V でペーストする方式に変更する
- **フラッシュ前にユーザーのクリップボード内容を必ず退避し、フラッシュ後に復元する**
- 既存の Mac `type_text()`（CGEvent 一字打鍵）は使わなくなる

---

## 2. 現状のコード解析

### 2.1 関係するファイル一覧

| ファイル | 役割 | 変更要否 |
|----------|------|---------|
| `src/types.rs` | `HotkeyAction`, `InputMode`, `AppState` 列挙型定義 | 変更不要 |
| `src/constants.rs` | 各種定数 | Low（不要定数削除） |
| `src/hotkey_mac.rs` | macOS CGEventTap によるホットキー検出 | **要変更** |
| `src/hotkey_win.rs` | Windows rdev によるホットキー検出 | **要変更** |
| `src/mycute_manager.rs` | `MycuteManager` 状態管理 | **要変更** |
| `src/mycute_settings.rs` | `HotkeyConfig` 設定 | 変更不要 |
| `src/tauri_cmd/system.rs` | ホットキーアクションハンドラループ | **要変更** |
| `src/mode/cl/main_of_cl.rs` | STTイベントブリッジ | **要変更** |
| `src/tools/audio.rs` | 開始音・終了音の再生 | 変更不要 |
| `src/input/keyboard_mac.rs` | macOS キーボード注入 | **要変更**（削除中心） |
| `src/input/keyboard_win.rs` | Windows キーボード注入 | 変更不要（既にクリップボード方式） |
| `src/input/clipboard.rs` | クリップボード操作 | 変更不要 |

### 2.2 既存の Buffer 関連コード（未使用だが存在する）

**`InputMode::Buffered`** (types.rs:119-124): 定義済みだが、どのコードパスからも使われていない。

```rust
pub enum InputMode {
    RealTime,
    Buffered,  // ← 未使用
}
```

**`MycuteManager::start_recording(mode)`** (mycute_manager.rs:34): `mode == InputMode::Buffered` で `buffer.clear()` する分岐は存在する。呼び出し元は常に `InputMode::RealTime` を渡している。

**`HotkeyAction::BufferStart` / `BufferFlush`** (types.rs:63-64): 列挙型定義済み。system.rs の match では `_ =>` で握り潰され未処理。

### 2.3 バッファ変数の分析（これをそのまま流用する）

`MycuteManager` のフィールド:

| フィールド | 説明 | フラッシュ時の役割 |
|-----------|------|-------------------|
| `current_text: String` | 最新の認識結果テキスト（1回の Partial/Final 結果） | 最新未確定部分 |
| `buffer: String` | 確定済み(Final)テキスト断片の累積蓄積 | 確定済み全文 |
| `is_post_correcting: bool` | PostCorrection 実行中フラグ | フラッシュ待機判定用 |

**フラッシュする全文**: `format!("{}{}", mgr.buffer, mgr.current_text)`

これは既に `overlay_full_text`（main_of_cl.rs:623）としてオーバーレイUIに送信されている値と同じものである。新しい変数は不要。

### 2.4 自動コミットの発行元

**macOS** (hotkey_mac.rs:231-239): `event_tap_callback` 内、`BUFFER_MODE_ACTIVE` が false かつ KeyDown/マウスクリック時に `HotkeyAction::Commit` を送信。

**Windows** (hotkey_win.rs:296-301): `handle_event` 内、修飾キーなし KeyPress / ButtonPress 時に `HotkeyAction::Commit` を送信。

### 2.5 PostCorrection の現状の流れ

1. 認識エンジン側が沈黙検知 → `SttEvent::PostCorrectionStarted` 発行
2. イベントブリッジ (main_of_cl.rs:550-569): `mgr.is_post_correcting = true` + 装飾文字 `→ Correcting …` をキーボード注入（打鍵）
3. LLM 補正実行（非同期的）
4. `SttEvent::PostCorrectionFinished` 発行 → `mgr.is_post_correcting = false`
5. 補正結果が `FinalResult` として後続イベントで届く

---

## 3. 変更設計

### 3.1 全体フロー

```
[Idle 状態]
  Alt/Option ダブルタップ（HOTKEY_DOUBLE_TAP_MAX_MS 以内の連続押下）
    ↓
  start_recording(InputMode::Buffered)
  SttEvent::Ready → play_ready_sound()（開始音 piro.wav）
    ↓
[Recording / Buffered 状態]
  認識結果 → mgr.buffer + mgr.current_text に蓄積のみ（キーボード注入なし）
  オーバーレイ UI への SttUpdate イベント（全文表示）は継続
    ↓
  Alt/Option シングルタップ
    ↓
  → mgr.is_post_correcting ?
    → Yes: mgr.pending_flush = true → PostCorrectionFinished で自動発動
    → No : 即座にフラッシュ実行
    ↓
  フラッシュ手順:
    1. saved = clipboard::get_clipboard()          // ユーザーのクリップボード退避
    2. clipboard::set_clipboard(flush_text)        // 全文をセット
    3. KeyboardInjector::send_cmd_v()              // Cmd+V / Ctrl+V
    4. sleep(50ms)                                 // ペースト反映待機
    5. clipboard::set_clipboard(saved)             // クリップボード復元
    ↓
  play_commit_sound()
  stop_recording()
  SttCommit / AppState:Idle イベントを emit
    ↓
[Idle 状態]
```

### 3.2 ホットキー検出の変更

#### 設計方針: Recording状態をホットキースレッドに伝達する

「ダブルタップの1回目」と「フラッシュ用シングルタップ」は物理的イベントとしては区別不可能。
時間差による判定（`HOTKEY_DOUBLE_TAP_MAX_MS` との比較）を使うと、マウスクリックや他キー押下によるタイマーリセット（`LAST_OPTION_PRESS_TIME = 0`）が行われるたびにシングルタップ検出が狂う。

**解決策: `AtomicBool` で Recording 状態をホットキースレッドに伝える。**

- `hotkey_mac.rs` / `hotkey_win.rs` に `static RECORDING_ACTIVE: AtomicBool` を追加
- `pub fn set_recording_active(active: bool)` を公開
- system.rs が Start 処理時に `set_recording_active(true)`、Flush 処理時に `set_recording_active(false)` を呼ぶ
- **Recording 中の Option/Alt 押下 → 即座に `HotkeyAction::BufferFlush` を送信**
- **Idle 中の Option/Alt 押下 → 従来通りのダブルタップ検出のみ**（時間差判定）
- Recording 中のマウスクリック/他キーによるタイマーリセットは無視してよい（フラッシュは `RECORDING_ACTIVE` で判定するため）

#### macOS (hotkey_mac.rs): RECORDING_ACTIVE 追加 + シングルタップ検出

`event_tap_callback` の `FLAGS_CHANGED` 処理内：

```rust
// --- 追加: グローバル AtomicBool ---
use std::sync::atomic::{AtomicBool, Ordering};
// static mut 領域の隣などに追加
static RECORDING_ACTIVE: AtomicBool = AtomicBool::new(false);

pub fn set_recording_active(active: bool) {
    RECORDING_ACTIVE.store(active, Ordering::SeqCst);
}
```

```rust
// event_tap_callback 内の FLAGS_CHANGED 処理
if event_type == K_CG_EVENT_FLAGS_CHANGED {
    let flags = CGEventGetFlags(event);
    CONTROL_KEY_DOWN = (flags & K_CG_EVENT_FLAG_MASK_CONTROL) != 0;

    let is_option_down = (flags & K_CG_EVENT_FLAG_MASK_ALTERNATE) != 0;
    if is_option_down && !OPTION_KEY_DOWN {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis();

        let diff = now.saturating_sub(LAST_OPTION_PRESS_TIME);

        // ★ Recording 中は単発の Option 押下で即フラッシュ
        if RECORDING_ACTIVE.load(Ordering::SeqCst) {
            if let Some(ref sender) = HOTKEY_SENDER {
                let _ = sender.try_send(HotkeyAction::BufferFlush);
            }
            // LAST_OPTION_PRESS_TIME は更新しない（次の押下を独立して扱うため）
        } else if diff > (HOTKEY_DOUBLE_TAP_MIN_MS as u128)
            && diff < (HOTKEY_DOUBLE_TAP_MAX_MS as u128)
        {
            // ダブルタップ: 録音開始
            if let Some(ref sender) = HOTKEY_SENDER {
                let _ = sender.try_send(HotkeyAction::Start);
            }
            LAST_OPTION_PRESS_TIME = 0;
        } else {
            LAST_OPTION_PRESS_TIME = now;
        }
    }
    OPTION_KEY_DOWN = is_option_down;

    return event;
}
```

合わせて以下を削除:
- `BUFFER_MODE_ACTIVE` static 変数（109行）と関数 `set_buffer_mode()`（116-119行）
- `IS_TYPING` static 変数（111行）と関数 `set_typing_mode()`（123-126行）
- 自動 Commit 発行ブロック（231-240行）全体
- マウスクリック時の `LAST_OPTION_PRESS_TIME = 0` リセット（228行）— RECORDING_ACTIVE 方式では不要
- 他キー押下時の `LAST_OPTION_PRESS_TIME = 0` リセット（187行）— 同上

**【第2次検証で追加】** KeyDown 内の BufferStart/Flush ホットキー照合（200-209行）を削除:
- 現在 `Option+B → BufferStart`, `Option+F → BufferFlush` が照合されると `return ptr::null_mut()` でイベントが**ブロック**される（217行）
- 新しい UX では Option+F は通常のキーとしてアプリに届くべき（フラッシュは修飾キー単独押下で行うため）
- Option+B も意味を持たないため、これら2つの照合分岐（`else if` ブロック全体）を削除する
- この削除により `ACTIVE_HOTKEYS` 構造体の `buffer_start_key` / `buffer_start_flags` / `buffer_flush_key` / `buffer_flush_flags` フィールド、および `start()` 内のパース代入（280-281行）と初期化（290-293行）は到達不能になる。Step 6 のデッドコード整理で削除する。

さらにイベントマスク（319-325行）を簡略化:
- 自動コミット廃止により KEY_UP / マウスイベントを処理する必要がなくなった
- `events_of_interest` を `K_CG_EVENT_KEY_DOWN | K_CG_EVENT_FLAGS_CHANGED` のみに縮小する
- マウスイベント定数定義（`K_CG_EVENT_LEFT_MOUSE_DOWN` 等、29-32行）も併せて削除

#### Windows (hotkey_win.rs): RECORDING_ACTIVE 追加 + シングルタップ検出

```rust
// --- 追加: グローバル AtomicBool ---
static RECORDING_ACTIVE: AtomicBool = AtomicBool::new(false);

pub fn set_recording_active(active: bool) {
    RECORDING_ACTIVE.store(active, Ordering::SeqCst);
}
```

`handle_event` の `KeyPress(Alt)` 内:

```rust
Key::Alt | Key::AltGr => {
    let old_mods = CURRENT_MODIFIERS.fetch_or(MOD_ALT, Ordering::SeqCst);
    if (old_mods & MOD_ALT) == 0 {
        // ★ Recording 中は単発の Alt 押下で即フラッシュ
        if RECORDING_ACTIVE.load(Ordering::SeqCst) {
            if let Ok(guard) = HOTKEY_SENDER.lock() {
                if let Some(ref sender) = *guard {
                    let _ = sender.try_send(HotkeyAction::BufferFlush);
                }
            }
            return;
        }

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;
        let last = LAST_ALT_PRESS_TIME.load(Ordering::SeqCst);
        let diff = now.saturating_sub(last);
        if diff > HOTKEY_DOUBLE_TAP_MIN_MS && diff < HOTKEY_DOUBLE_TAP_MAX_MS {
            // ダブルタップ検出（KeyRelease で発動）
            PENDING_ALT_START.store(true, Ordering::SeqCst);
            LAST_ALT_PRESS_TIME.store(0, Ordering::SeqCst);
        } else {
            LAST_ALT_PRESS_TIME.store(now, Ordering::SeqCst);
        }
    }
    return;
}
```

KeyRelease の PENDING_ALT_START 発動ロジックはそのまま維持（ダブルタップ検出のため）。

合わせて自動 Commit 発行ロジックをすべて削除:
- `BUFFER_MODE_ACTIVE` (21行) / `IS_TYPING` (22行) の AtomicBool と関数 `set_buffer_mode()` (33-35行) / `set_typing_mode()` (38-40行)
- `handle_event` 内の Commit 送信: KeyPress（296-301行）と ButtonPress（337-343行）の両方
- マウスクリック時の `LAST_ALT_PRESS_TIME.store(0, ...)`（335行）— RECORDING_ACTIVE 方式では不要
- 他キー押下時の `LAST_ALT_PRESS_TIME.store(0, ...)`（248行）— 同上

**【第2次検証で追加】** KeyPress 内の BufferStart/Flush ホットキー照合（267-270行）を削除:
- macOS と同様、`Option+B → BufferStart`, `Option+F → BufferFlush` の照合分岐を削除する
- 新しい UX ではこれらのキーコンビネーションは通常のキーとしてOSに届くべき

**【第2次検証で追加】** ButtonPress アーム（334-343行）を削除:
- `EventType::ButtonPress(_)` の match アーム全体を削除（自動コミット廃止により不要）
- rdev の `listen()` でマウスイベントは引き続き取得されるが、処理は行わない

### 3.3 MycuteManager の変更 (mycute_manager.rs)

```rust
pub struct MycuteManager {
    pub recognizer: Arc<Mutex<SpeechRecognizer>>,
    pub state: AppState,
    pub input_mode: InputMode,
    pub current_text: String,
    pub buffer: String,
    pub pending_flush: bool,     // ← 追加: PostCorrection 完了待ちフラグ
    pub locale: LocaleCode,
    pub last_stt_seq: u64,
    pub is_post_correcting: bool,
}
```

メソッド追加:

```rust
impl MycuteManager {
    /// フラッシュする全文を構築する。
    pub fn build_flush_text(&self) -> String {
        format!("{}{}", self.buffer, self.current_text)
    }
}
```

**stop_recording() に `self.buffer.clear()` + `self.pending_flush = false` を追加**:

従来 `stop_recording()` は `current_text` のみクリアして `buffer` をクリアしていなかった。
フラッシュ方式では確実にバッファを初期化する必要があるため、以下を追加する。

```rust
pub fn stop_recording(&mut self) {
    self.recognizer.lock().stop();
    self.state = AppState::Idle;
    self.current_text.clear();
    self.buffer.clear();          // ← 追加
    self.last_stt_seq = 0;
    self.is_post_correcting = false;
    self.pending_flush = false;   // ← 追加
}
```

これにより `clear_flush()` のような別メソッドは不要になる。
`stop_recording()` を経由すれば常にバッファと pending_flush がクリアされる。

`new()` での初期化: `pending_flush: false`
`start_recording()` の先頭で `self.pending_flush = false` にリセット

### 3.4 system.rs: ホットキーハンドラループの変更 (tauri_cmd/system.rs)

**Start ハンドラ** (159行):

```rust
// 変更前
mgr.start_recording(InputMode::RealTime);
// 変更後
mgr.start_recording(InputMode::Buffered);
#[cfg(target_os = "macos")]
hotkey_mac::set_recording_active(true);
#[cfg(windows)]
hotkey_win::set_recording_active(true);
```

**Commit ハンドラ** (175-197行): 削除（自動コミットの廃止）

**BufferFlush ハンドラ**（新規）:

```rust
HotkeyAction::BufferFlush => {
    let mut mgr = manager_for_hk.lock();
    if mgr.state != MgrAppState::Recording {
        continue; // Idle 状態では無視（RECORDING_ACTIVE ガードもあるが二重安全）
    }
    if mgr.is_post_correcting {
        // 補正中は保留
        mgr.pending_flush = true;
        continue;
    }
    // 空バッファガード
    let flush_text = mgr.build_flush_text();
    if flush_text.is_empty() {
        mgr.stop_recording();
        #[cfg(target_os = "macos")]
        hotkey_mac::set_recording_active(false);
        #[cfg(windows)]
        hotkey_win::set_recording_active(false);
        audio::play_commit_sound();
        let _ = handle_for_hk.emit(TauriEvent::SttCommit.as_str(), ());
        let _ = handle_for_hk.emit(TauriEvent::AppState.as_str(), AppStatePayload {
            state: APP_STATE_IDLE.to_string(),
        });
        continue;
    }
    // フラッシュ実行（ロックを一時解放してブロッキング操作を行う）
    mgr.stop_recording();
    drop(mgr); // ← ロック解放！これが重要

    #[cfg(target_os = "macos")]
    hotkey_mac::set_recording_active(false);
    #[cfg(windows)]
    hotkey_win::set_recording_active(false);

    // クリップボードに保存・設定・ペースト・復元
    let saved = clipboard::get_clipboard().unwrap_or_default();
    if let Err(e) = clipboard::set_clipboard(&flush_text) {
        log::error!("[Flush] Failed to set clipboard: {}", e);
    }
    KeyboardInjector::send_cmd_v();
    std::thread::sleep(std::time::Duration::from_millis(50));
    if let Err(e) = clipboard::set_clipboard(&saved) {
        log::warn!("[Flush] Failed to restore clipboard: {}", e);
    }

    audio::play_commit_sound();
    let _ = handle_for_hk.emit(TauriEvent::SttCommit.as_str(), ());
    let _ = handle_for_hk.emit(TauriEvent::AppState.as_str(), AppStatePayload {
        state: APP_STATE_IDLE.to_string(),
    });
}
```

**注意**: `clipboard` モジュールは既に system.rs で `use` されている（7行目）。`audio` も使用済み。`hotkey_mac` / `hotkey_win` の `use` は既にあることを確認すること。

### 3.5 STT イベントブリッジの変更 (main_of_cl.rs)

**PostCorrectionStarted ハンドラ** (550-569行):

現状の装飾文字 `KeyboardInjector::type_text(decoration)` 打鍵を削除。代わりに:
- `mgr.is_post_correcting = true` だけでよい
- `mgr.current_text` への `push_str(decoration)` も不要
- `SttPartial` の emit は継続（オーバーレイ更新用）

簡略化後のコード:

```rust
SttEvent::PostCorrectionStarted => {
    let mut mgr = manager.lock();
    if mgr.state == MgrAppState::Recording && !mgr.is_post_correcting {
        mgr.is_post_correcting = true;
        // SttPartial はオーバーレイ更新のために送信（decoration なしで current_text を送る）
        let _ = handle.emit(
            TauriEvent::SttPartial.as_str(),
            SttPayload {
                text: mgr.current_text.clone(),
                seq: mgr.last_stt_seq,
            },
        );
    }
}
```

**PostCorrectionFinished ハンドラ** (571-583行):

`mgr.is_post_correcting = false` にした後、以下の保留フラッシュ処理を追加:

```rust
SttEvent::PostCorrectionFinished => {
    let mut mgr = manager.lock();
    if mgr.is_post_correcting {
        mgr.is_post_correcting = false;

        // 保留中のフラッシュを確認
        if mgr.pending_flush {
            mgr.pending_flush = false;

            let flush_text = mgr.build_flush_text();
            mgr.stop_recording();       // ← 内部で buffer.clear() + pending_flush = false
            drop(mgr); // ロック解放

            let handle_clone = handle.clone();
            tokio::task::spawn_blocking(move || {
                // ★ RECORDING_ACTIVE は spawn_blocking 内部の先頭で false にする
                //    (system.rs と同じ「stop → set_recording_active(false) → clipboard」の順序)
                #[cfg(target_os = "macos")]
                hotkey_mac::set_recording_active(false);
                #[cfg(windows)]
                hotkey_win::set_recording_active(false);

                if flush_text.is_empty() {
                    audio::play_commit_sound();
                    let _ = handle_clone.emit(TauriEvent::SttCommit.as_str(), ());
                    let _ = handle_clone.emit(TauriEvent::AppState.as_str(), AppStatePayload {
                        state: APP_STATE_IDLE.to_string(),
                    });
                    return;
                }

                let saved = clipboard::get_clipboard().unwrap_or_default();
                if let Err(e) = clipboard::set_clipboard(&flush_text) {
                    log::error!("[Flush] Failed to set clipboard: {}", e);
                }
                KeyboardInjector::send_cmd_v();
                std::thread::sleep(std::time::Duration::from_millis(50));
                if let Err(e) = clipboard::set_clipboard(&saved) {
                    log::warn!("[Flush] Failed to restore clipboard: {}", e);
                }

                audio::play_commit_sound();
                let _ = handle_clone.emit(TauriEvent::SttCommit.as_str(), ());
                let _ = handle_clone.emit(TauriEvent::AppState.as_str(), AppStatePayload {
                    state: APP_STATE_IDLE.to_string(),
                });
            });
            // set_recording_active(false) は spawn_blocking 内部に移動したためここには不要
        }
    }
}
```

**注**:
- 空バッファ時の emit を `AppState` + `APP_STATE_IDLE` に統一した（旧 `AppStatus` + `APP_STATUS_STOPPED`）。
  これは system.rs の BufferFlush ハンドラと同じパターンであり、フロントエンドに
  「録音が終了して Idle 状態になった」ことを伝える。`app-status` イベントは STT エンジン停止通知
  （SttEvent::Stopped）専用として残る。
- system.rs の BufferFlush ハンドラと共通するクリップボード操作 + Cmd+V のシーケンスは、
  将来 `clipboard::replace_selected_text()` に相当する汎用関数として抽出することも可能だが、
  マイグレーションの範囲を超えるため今回はインラインで実装してよい。

**キーボード注入分岐の変更確認** (631行):

```rust
// 変更前:
if mgr.state == MgrAppState::Recording && mgr.input_mode == InputMode::RealTime {
    KeyboardInjector::input_diff(&injected_text, &text);
    injected_text = text.clone();
}
// 変更後: この分岐自体は Buffered では実行されないのでコード変更不要
// ただし InputMode::RealTime を列挙型から削除しない限り、動作としては正しい。
```

### 3.6 macOS keyboard_mac.rs の整理 (keyboard_mac.rs)

フラッシュ方式への移行に伴い、以下の関数は使われなくなる:
- `type_text()` / `type_text_inner()` — CGEvent 一字打鍵
- `input_diff()` — 差分更新
- `send_backspaces()` / `send_backspaces_inner()` — バックスペース
- 関連する静的変数: `DELETION_DEADLINES`, `INPUT_LOCK`

ただし **削除は安全に行うこと**:
1. `make check` を実行し、未使用関数の警告（`dead_code`）を確認
2. 他から参照されていない関数のみ削除
3. `send_cmd_c()` / `send_cmd_v()` / `send_cmd_key()` はフラッシュ時にも使用するので維持

**安全策**: デッドコード削除は必須ではない。`#[allow(dead_code)]` を付与して後日整理することも可能。
ただし本計画書の「Boy Scout Rule」の精神に従い、触ったコードは整理すること。

### 3.7 フロントエンドへの影響

**必要な変更なし**。以下のイベントは現状と同じ形式で送信され続ける:
- `stt-partial` / `stt-final`: 認識結果の逐次送信
- `stt-update`: `mgr.buffer + mgr.current_text` の全文表示
- `stt-commit`: フラッシュ/終了時のオーバーレイ消去
- `app-state`: Recording / Idle 状態遷移

---

## 4. ステップバイステップ実装手順

**重要: 各ステップの後で `make check` を実行し、コンパイルが通ることを確認すること。**

### Step 1: MycuteManager に pending_flush を追加

**ファイル**: `src/mycute_manager.rs`

- `MycuteManager` 構造体に `pub pending_flush: bool` 追加
- `new()` で初期化: `pending_flush: false`
- `start_recording()` の先頭に `self.pending_flush = false` 追加
- `stop_recording()` に以下を追加:
  - `self.buffer.clear()` — 従来漏れていたバッファクリア
  - `self.pending_flush = false` — 念のためリセット
- メソッド追加: `build_flush_text() -> String`（`clear_flush()` は不要 — `stop_recording()` がバッファをクリアするため）

**確認**: `make check` → OK

### Step 2: 自動コミットを削除（hotkey_mac.rs / hotkey_win.rs）

**ファイル**: `src/hotkey_mac.rs`

削除対象:
- `static BUFFER_MODE_ACTIVE: bool` (109行) と `set_buffer_mode()` (116-119行)
- `static IS_TYPING: bool` (111行) と `set_typing_mode()` (123-126行)
- `event_tap_callback` 内の Commit 送信ブロック（231-240行）全体
- マウスクリック時の `LAST_OPTION_PRESS_TIME = 0` リセット（228行）— RECORDING_ACTIVE 方式では不要
- 他キー押下（KeyDown）時の `LAST_OPTION_PRESS_TIME = 0`（187行）— 同上
- **KeyDown 内の BufferStart/Flush ホットキー照合（200-209行）** — Option+F/B ブロック防止（3.2 参照）
- **イベントマスクから KEY_UP と全 MOUSE_DOWN を削除**（320-325行）— 不要なイベント処理削減
- **マウスイベント定数定義**（`K_CG_EVENT_LEFT_MOUSE_DOWN` 等、29-32行）— 使用しなくなるため削除

追加:
- `static RECORDING_ACTIVE: AtomicBool` と `set_recording_active()`（3.2 参照）

**ファイル**: `src/hotkey_win.rs`

削除対象:
- `static BUFFER_MODE_ACTIVE: AtomicBool` (21行) と `set_buffer_mode()` (33-35行)
- `static IS_TYPING: AtomicBool` (22行) と `set_typing_mode()` (38-40行)
- `static RECORDING_ACTIVE: AtomicBool` を新規追加（3.2 参照）
- `handle_event` 内の Commit 送信:
  - KeyPress 内: `if !BUFFER_MODE_ACTIVE.load(...) && !IS_TYPING.load(...)` ブロック（296-301行）
  - ButtonPress 内: 同様のブロック（337-343行）
- マウスクリック時の `LAST_ALT_PRESS_TIME.store(0, ...)`（335行）
- 他キー押下時の `LAST_ALT_PRESS_TIME.store(0, ...)`（248行）
- **KeyPress 内の BufferStart/Flush ホットキー照合（267-270行）** — Option+F/B ブロック防止
- **ButtonPress アーム全体（334-343行）** — マウスクリック処理の全廃止
- 上記に関連する `use` 文の整理

**依存削除: `src/input/keyboard_win.rs` の TypingGuard**

`TypingGuard`（77-88行）は `crate::hotkey::set_typing_mode()` を呼んでいる。`set_typing_mode()` を削除するとコンパイルエラーになる。

**必ず同時に削除すること:**
- `TypingGuard` 構造体全体（77-88行）
- `type_text()` 内の `let _guard = TypingGuard::new();`（108行）
- `send_backspaces()` 内の `let _guard = TypingGuard::new();`（242行）
- `input_diff()` 内の `let _guard = TypingGuard::new();`（319行）
- `send_ctrl_key()` 内の `let _guard = TypingGuard::new();`（379行）

**確認**: `make check` → OK

### Step 3: RECORDING_ACTIVE フラグとシングルタップ検出を追加

**注意**: この Step は、3.2 で説明した `RECORDING_ACTIVE` AtomicBool の追加と、
それを用いたシングルタップ検出ロジックの実装である。Step 2 では削除のみ行い、
ここで新規ロジックを追加する（削除 → 追加の順で進めるため）。

**ファイル**: `src/hotkey_mac.rs`

追加:
- `static RECORDING_ACTIVE: AtomicBool = AtomicBool::new(false);`
- `pub fn set_recording_active(active: bool) { ... }`
- `event_tap_callback` 内の `FLAGS_CHANGED` 処理: 既存のダブルタップ判定の前に
  `if RECORDING_ACTIVE.load(...) { sender.try_send(BufferFlush); return event; }` を挿入
  （詳細なコードは 3.2 参照）

**ファイル**: `src/hotkey_win.rs`

追加:
- `static RECORDING_ACTIVE: AtomicBool = AtomicBool::new(false);`
- `pub fn set_recording_active(active: bool) { ... }`
- `handle_event` の `KeyPress(Alt)` 内: 時間差判定に入る前に
  `if RECORDING_ACTIVE.load(...) { sender.try_send(BufferFlush); return; }` を挿入
  （詳細なコードは 3.2 参照）

**確認**: `make check` → OK

### Step 4: system.rs ハンドラループを変更

**ファイル**: `src/tauri_cmd/system.rs`

1. `Start` の `InputMode::RealTime` → `InputMode::Buffered` + `set_recording_active(true)` 呼び出し追加
2. `Commit` アーム全体を削除（match から削除）
3. `BufferFlush` アームを追加し、フラッシュ処理を実装 + `set_recording_active(false)` 呼び出し追加
4. **`_ =>` フォールバックは残す** — `HotkeyAction::BufferStart` は列挙型に残るが未使用のため、引き続き `_ =>` で握り潰す。削除しないこと。

```rust
// 変更後の match 構造:
match action {
    HotkeyAction::Start => { ... }
    HotkeyAction::Correct => { ... }
    HotkeyAction::Summarize => { ... }
    HotkeyAction::BufferFlush => { ... }       // 新規
    _ => {                                      // 維持（BufferStart 用）
        log::debug!("Hotkey received but unhandled in cl mode: {:?}", action);
    }
}
```

フラッシュ処理の実装詳細は [3.4 system.rs: ホットキーハンドラループの変更](#34-systemrs-ホットキーハンドラループの変更-tauri_cmdsystemrs) を参照。

**注意**: `clipboard` モジュールの関数（`get_clipboard`, `set_clipboard`）は system.rs の先頭で `use crate::input::clipboard;` として既にインポート済み。

**注意**: `hotkey_mac::set_recording_active()` を使用するため、system.rs 上部の `use crate::hotkey_mac;` が既にあることを確認（22行目で使用済み）。同様に `hotkey_win` も条件付きで使用済み。

**確認**: `make check` → OK

### Step 5: STT イベントブリッジを変更

**ファイル**: `src/mode/cl/main_of_cl.rs`

1. `PostCorrectionStarted` ハンドラ: 装飾打鍵コードを削除し、`is_post_correcting = true` のみに
2. `PostCorrectionFinished` ハンドラ:
   - `pending_flush` チェックと spawn_blocking でのフラッシュ実行を追加
   - Emit は `AppState` + `APP_STATE_IDLE` を使用（`AppStatus` + `APP_STATUS_STOPPED` ではない）
   - `set_recording_active(false)` は spawn_blocking 内部の先頭で呼び出す
   - 新しい `use` の追加: `APP_STATE_IDLE`, `AppStatePayload`
3. `use` 文の整理:

   - `use crate::input::keyboard::KeyboardInjector;` は他で使われていなければ削除
   - ただし `_ => {}` のアームは残す（網羅性のため）

**確認**: `make check` → OK

### Step 6: デッドコードの整理（安全に）

**ファイル**: `src/input/keyboard_mac.rs`

- `type_text()` 他が `pub fn` として他から参照されているか確認:
  - Grep: `KeyboardInjector::type_text` が他で使われていないことを確認
  - Grep: `KeyboardInjector::input_diff` が他で使われていないことを確認
  - Grep: `KeyboardInjector::send_backspaces` が他で使われていないことを確認
- 完全に未使用なら削除。ただし `send_cmd_c()` / `send_cmd_v()` / `send_cmd_key()` は維持
- 静的変数 `DELETION_DEADLINES`, `INPUT_LOCK` も同様に未使用なら削除

**ファイル**: `src/constants.rs`

- Grep で各定数が参照されているか確認:
  - `KEY_DELAY_MS_MAC` → 不要なら削除
  - `DELETION_COOLDOWN_MS_MAC` → 不要なら削除
  - `DELETION_WEIGHT_MS_MAC` → 不要なら削除
  - `POST_CORRECTION_DECORATION` → 不要なら削除

**ファイル**: `src/hotkey_mac.rs`

- `ActiveHotkeys` 構造体から `buffer_start_key` / `buffer_start_flags` / `buffer_flush_key` / `buffer_flush_flags` フィールドを削除（Step 2 で KeyDown 照合を削除したため到達不能）
- `start()` 内の上記フィールドに対するパース代入（280-281行）と初期化リテラル（290-293行）を削除

**ファイル**: `src/hotkey_win.rs`

- `IS_TYPING` 関連削除後、`input::keyboard` の `use` が不要なら削除
- `ActiveHotkeys` 構造体から `buffer_start` / `buffer_flush` フィールドを削除（Step 2 で KeyPress 照合を削除したため到達不能）
- `ActiveHotkeys::from_config()` 内の上記フィールド初期化（119-120行）を削除

**確認**: `make check` → OK（dead_code 警告が消えたことを確認）

### Step 7: 最終確認

```bash
make check
make test
```

両方ともパスすること。

---

## 5. エッジケースと注意点

### 5.1 フラッシュ中に別の Alt 押下が来た場合

フラッシュ処理は数十msで完了する。完了後すぐに Idle に遷移するため、後続の Alt 押下は無視される（BufferFlush は Recording 状態でなければ何もしない）。

万が一クリップボード操作が競合した場合も、`arboard::Clipboard::new()` が内部で適切に排他する。

### 5.2 空バッファでのフラッシュ

認識開始直後のフラッシュでは `flush_text` が空文字列になる。この場合:
- クリップボード操作をスキップ
- `stop_recording()` のみ実行
- 終了音とオーバーレイ消去は通常通り

コード参照: [3.4 の空バッファガード](#34-systemrs-ホットキーハンドラループの変更-tauri_cmdsystemrs)

### 5.3 PostCorrection 中の Recording 強制終了

アプリ終了等で `stop_recording()` が呼ばれた場合、`pending_flush` を `false` にリセットすること（3.3 で実装済み）。

### 5.4 Windows IME 制御は継続

`Start` 時の `disable_ime()` とフラッシュ時の `restore_ime()` は引き続き必要。
フラッシュ前に IME が ON だと Cmd+V が正しく機能しない可能性がある。

### 5.5 PostCorrection 完了後のフラッシュ実行コンテキスト

`PostCorrectionFinished` は STT イベントブリッジ（tauri async runtime）内で受信される。
このコンテキストで `clipboard::set_clipboard()` を呼ぶとブロッキングになるため、`tokio::task::spawn_blocking` を使用すること（3.5 で実装）。

---

## 6. 検証手順書（別AIに渡す用）

```bash
# 1. コンパイル確認
make check

# 2. 全テスト実行
make test

# 3. 手動テスト（macOS）
# 3-a. ダブルタップ開始: Option を2回素早く押す
#       → 開始音（piro.wav）が鳴り、オーバーレイに認識テキストが表示されること
#       → キーボードに文字が注入されていないこと
# 3-b. シングルタップフラッシュ: Option を1回押す（録音中）
#       → カーソル位置に認識全文がペーストされること
#       → 終了音（commit.wav）が鳴ること
#       → オーバーレイが消えること
# 3-c. クリップボード復元: 事前に任意テキストをコピー → 録音 → フラッシュ
#       → フラッシュ後にコピーしたテキストがクリップボードに残っていること
# 3-d. 自動コミット廃止: 録音中にマウスクリックやキー入力
#       → 録音が終了しないこと（フラッシュまたは次のダブルタップのみで終了）
# 3-e. 空バッファフラッシュ: 録音後即座に Option 単押し
#       → 空文字がペーストされず、正常終了すること
# 3-f. Idle 状態の Option 単押し: 録音していない状態で Option 単押し
#       → 何も起こらないこと（無視される）

# 4. 手動テスト（Windows）
# 同上。Windows の場合は Ctrl+V ペーストであることを確認。
```

---

## 7. ファイル変更サマリー

| # | ファイル | 変更種類 | 変更内容 |
|---|---------|---------|---------|
| 1 | `src/mycute_manager.rs` | 追加/編集 | `pending_flush` フィールド、`build_flush_text()`、`stop_recording()` に `buffer.clear()` + `pending_flush = false` 追加（`clear_flush()` は不要） |
| 2 | `src/hotkey_mac.rs` | 編集 | `RECORDING_ACTIVE` 追加、自動Commit削除、マウスクリックリセット削除、シングルタップ検出追加、KeyDown BufferStart/Flush照合削除、イベントマスク簡略化 |
| 3 | `src/hotkey_win.rs` | 編集 | `RECORDING_ACTIVE` 追加、自動Commit削除、マウスクリックリセット削除、シングルタップ検出追加、KeyPress BufferStart/Flush照合削除、ButtonPressアーム削除 |
| 4 | `src/tauri_cmd/system.rs` | 編集 | Start: Buffered+set_recording_active(true)、Commit削除、BufferFlush実装+set_recording_active(false) |
| 5 | `src/mode/cl/main_of_cl.rs` | 編集 | PostCorrection 装飾削除、保留フラッシュ処理追加、emit を `AppState`+`APP_STATE_IDLE` に修正、`set_recording_active(false)` を spawn_blocking 内部に移動 |
| 6 | `src/input/keyboard_mac.rs` | 削除(任意) | type_text/input_diff/send_backspaces （デッドコード） |
| 7 | `src/input/keyboard_win.rs` | 削除(必須) | TypingGuard 削除（set_typing_mode 削除に伴うコンパイルエラー回避） |
| 8 | `src/constants.rs` | 削除(任意) | 不要定数（デッドコード） |

## 8. 補足: 影響を受けない/引き継がれる既存動作

### 8.1 HotkeyAction::BufferStart は未使用に、Option+B は通常キーに

`BufferStart` 列挙型バリアントは削除せず残るが、system.rs の `_ =>` で握り潰される。macOS の KeyDown 照合（200-204行）は削除済みのため、`Option+B` は通常のキーとしてアプリケーションに届く。紛らわしさを避けるため、将来 `BufferStart` 列挙型バリアントごと削除してもよい。

### 8.2 BufferFlush キーコンビネーション（Option+F）の二重経路は解消

第2次検証で macOS/Windows 双方の KeyDown/KeyPress ホットキー照合から BufferStart/Flush の分岐を削除した。そのため:
- フラッシュは **単独 Option/Alt 押下（FLAGS_CHANGED / KeyPress）** という単一経路のみで発動する
- `Option+F` は通常のキーとしてアプリケーションに届く（F キーの文字が入力される）
- 従来の `buffer_flush` ホットキー設定は設定ファイルに残っていても無視される

### 8.3 ホットキー設定（HotkeyConfig）の buffer_start / buffer_flush

`mycute_settings.rs` の `HotkeyConfig` 構造体と `settings.json` のホットキー設定は変更不要。
従来の設定が残っていても新しい動作に影響しない。

### 8.4 フロントエンド（オーバーレイUI）への影響

**必要な変更は一切ない**。送信される Tauri イベントは従来と同じ:

| イベント | 従来 | 変更後 |
|---------|------|--------|
| `stt-partial` / `stt-final` | 認識結果を送信 | 同じ（継続） |
| `stt-update` | buffer + current_text を送信 | 同じ（継続） |
| `stt-commit` | 終了時に送信 | 同じ（継続） |
| `app-state` | Recording / Idle | 同じ（継続） |
| `app-status` | Stopped（SttEvent::Stopped 時のみ） | 同じ（継続、PostCorrection フラッシュでは不使用） |

### 8.5 TauriEvent の命名改善（本変更とは別タスクとして推奨）

本計画のレビューで `TauriEvent::AppState` / `AppStatus` の名前が紛らわしいことが判明した。
このマイグレーションとは独立して、将来的に以下のリネームを推奨する。

| 現在 | 変更後 | 理由 |
|------|--------|------|
| `TauriEvent::AppState` → `"app-state"` | `TauriEvent::RecordingState` → `"recording-state"` | 録音状態機械の遷移であることを明示 |
| `TauriEvent::AppStatus` → `"app-status"` | `TauriEvent::SttStatus` → `"stt-status"` | STTエンジンの状態であることを明示 |
| `AppStatePayload` | `RecordingStatePayload` | 上記に合わせる |
| `AppStatusPayload` | `SttStatusPayload` | 上記に合わせる |
| `APP_STATE_IDLE` / `APP_STATE_RECORDING` | `RECORDING_STATE_IDLE` / `RECORDING_STATE_RECORDING` | 定数名も明確に |
| `APP_STATUS_STOPPED` | `STT_STATUS_STOPPED` | 定数名も明確に |

**影響ファイル**:
- `src/types.rs` — 列挙型バリアント名、Payload構造体名、`as_str()` マッピング
- `src/constants.rs` — イベント文字列定数、状態値定数
- `src/mycute_manager.rs` — `use` のエイリアス（`AppState as MgrAppState`）
- `src/mode/cl/main_of_cl.rs` — インポートと全使用箇所
- `src/tauri_cmd/system.rs` — インポートと全使用箇所
- `web/src/consts/generated_constants.ts` — フロントエンド定数

**注意**: イベント文字列自体を変更するため、フロントエンドの listen 箇所も同時に更新する必要がある。
ただし 2026-05-08 時点で `App.vue` は `app-state` / `app-status` を購読していないため、
rename による破壊的影響は現状発生しない。将来オーバーレイUIが購読する場合に備えて
明確な名前にしておく価値がある。
