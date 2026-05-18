---
ticket_id: 7
title: Windows: 音声入力フラッシュ時に誤ったクリップボード内容が貼り付けられる問題を修正
slug: windows-2
status: reviewed
created_at: 2026-05-18
updated_at: 2026-05-18
review_report_path: context/0007-windows-2/review.md
---
# Windows: 音声入力フラッシュ時に誤ったクリップボード内容が貼り付けられる問題を修正

## Summary

Windows 環境で音声入力のバッファフラッシュを行うと、認識されたテキストではなく、ユーザーが以前にクリップボードにコピーしていた別の内容が貼り付けられることがある問題を修正する。

## Background

音声入力のバッファフラッシュ機構は以下の順序で動作する:
1. 現在のクリップボード内容を退避（`get_clipboard_inner()`）
2. 認識テキストをクリップボードにセット（`set_clipboard_inner()`）
3. Ctrl+V を送信（`KeyboardInjector::send_cmd_v()` → `SendInput`）
4. 50ms 待機
5. 退避した内容を復元（`set_clipboard_inner(&saved)`）

macOS では一切問題が起きないが、Windows では「時々」認識テキストではなく別の内容が貼り付けられる。以前の修正（コミット 6bb3399: CLIPBOARD_LOCK による Mutex 排他制御追加）ではスレッド間の競合を防止したが、依然として発生する。

## Scope

- `src/input/clipboard.rs` の修正
  - `PASTE_DELAY_MS` の Windows 向け延長（50ms → 200ms）
  - `save_paste_and_restore()` の復元前チェック追加
  - `replace_selected_text()` の復元前チェック追加（一貫性のため）

## Non-scope

- `src/input/keyboard_win.rs` の `type_text_inner()` は RealTime モード用の別パスのため対象外
- macOS/Linux の挙動変更は不要のため対象外
- テストコードの追加（クリップボード操作は OS 依存が強くユニットテスト困難なため対象外）

## Investigation

### 現象
- WindowsPC で音声入力のフラッシュ時に、認識テキストではなく「別途PC上でクリップボードにコピーした内容」が貼り付けられる
- macOS では一切発生しない
- 以前の修正（CLIPBOARD_LOCK 追加）後も「時々」発生する

### 調査で判明した事実

**関連コード:**
- `src/input/clipboard.rs` — `save_paste_and_restore()`, `replace_selected_text()`, `get_selected_text()`
- `src/input/keyboard_win.rs` — `send_ctrl_key_inner()`
- `src/input/keyboard_mac.rs` — `send_cmd_v()`

**フラッシュ呼び出しパス（3経路）:**
1. `src/tauri_cmd/system.rs:338` — HotkeyAction::BufferFlush の即時処理
2. `src/mode/cl/main_of_cl.rs:709` — PostCorrectionFinished の遅延フラッシュ
3. `src/mode/cl/main_of_cl.rs:875` — SttCompleted の遅延フラッシュ

いずれの経路も最終的に `clipboard::save_paste_and_restore(&flush_text)` を呼ぶ。

### 仮説と検証

**仮説: Windows の SendInput(Ctrl+V) は非同期配送であり、50ms の待機では対象アプリがペーストを処理するのに不十分な場合がある。**

- `save_paste_and_restore()` は CLIPBOARD_LOCK を保持したまま以下のシーケンスを実行する:
  1. `saved = get_clipboard()` → 退避
  2. `set_clipboard(flush_text)` → 認識テキストをセット
  3. `KeyboardInjector::send_cmd_v()` → SendInput(4, inputs) で Ctrl+V 発行（非同期）
  4. `sleep(50ms)` → 待機
  5. `set_clipboard(saved)` → 復元

- 本来の意図: ステップ4でアプリがペーストを処理し、ステップ5で元に戻す

- 実際の問題: アプリがビジー等の理由で 50ms 以内にペーストを処理できない場合、ステップ5で先にクリップボードが復元される。その後アプリがペーストを処理すると、復元された「元のクリップボード内容」（＝ユーザーが以前コピーした内容）が貼り付けられる。

**反証: スレッド間競合説**
- CLIPBOARD_LOCK は `save_paste_and_restore` 全体をカバーしており、他スレッドとのインターリーブは発生しない
- ただし CLIPBOARD_LOCK は同一プロセス内のみ有効であり、外部プロセスによる割り込みは防げない

**macOS で発生しない理由:**
- macOS の `CGEventPost(kCGHIDEventTap)` は WindowServer を通じて比較的同期性の高いイベント配送を行う
- Windows の `SendInput` はシステムの入力キューに非同期で投稿され、メッセージキューを経由するため、配送レイテンシの変動が大きい

### 結論

Windows の `SendInput` の非同期性が原因で、50ms の待機では対象アプリのペースト処理が完了する前にクリップボードを復元してしまうことがある。このため「認識テキストではなく退避しておいた元の内容が貼り付けられる」という現象が発生する。

## Test Plan

クリップボード操作は OS のネイティブ API（`arboard` クレート経由）に依存しており、信頼性の高いユニットテストの実装は困難。以下の代替検証手段を計画する:

1. **コンパイル確認**: `cargo check` が通ること
2. **手動テスト（Windows実機）**:
   - 任意のテキストをクリップボードにコピーしておく
   - 音声入力を開始し、発話後 Alt ダブルタップでフラッシュ
   - 確認: 音声認識されたテキストが正しく貼り付けられること
   - 確認: フラッシュ後にクリップボードの内容が元のテキストに戻っていること
   - 上記を 10 回以上繰り返し、誤った内容が貼り付けられないことを確認
3. **デバッグログの活用**:
   - `log::debug!("Clipboard changed externally after paste...)` が出力された場合、外部プロセスによるインターリーブが発生した証拠となる
   - Windows 実機でのログ収集により修正の効果をモニタリング可能

## Boy Scout Rule — 翻訳可能性計画

- `src/input/clipboard.rs` の既存コードは関数名・変数名が散文として読める状態を維持している:
  - `save_paste_and_restore`: 「退避してペーストして復元する」を体現
  - `get_selected_text`: 「選択テキストを取得する」
  - `replace_selected_text`: 「選択テキストを指定テキストで差し替える」
- 今回の修正では条件付きコンパイルで `PASTE_DELAY_MS` の値をプラットフォームごとに変える。その理由をコメントで説明する
- 復元前チェックを追加することで、「クリップボードに外部変更があっても上書きしない」という安全策を明示的にする

## Acceptance Criteria

- [ ] `cargo check` が通過すること
- [ ] Windows 実機で 10 回以上の連続フラッシュテストで誤貼り付けが発生しないこと
- [ ] macOS で既存の動作が変わっていないこと（50ms 維持）
- [ ] クリップボードが外部プロセスによって変更された場合に復元をスキップし、上書きしないこと

## Notes

<!--
注: このコメントは人間向けの説明である。AI は以下の手順に従うこと。

- plan_path: /plan-ticket が plan.md を作成後に frontmatter に更新する
- implementation_path: /start-ticket が implementation.md を作成後に frontmatter に更新する
- review_report_path: /review-ticket が review.md を作成後に frontmatter に更新する

各コマンドのワークフロー手順が frontmatter 更新の正しい手順である。
-->

### 成果物

- 計画: context/0007-windows-2/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0007-windows-2/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0007-windows-2/review.md（未作成、/review-ticket 全チェック通過後に作成）
