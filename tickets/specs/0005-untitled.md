---
ticket_id: 5
title: クリップボードフラッシュ処理の競合状態修正
slug: untitled
status: reviewed
created_at: 2026-05-17
updated_at: 2026-05-17
---
# クリップボードフラッシュ処理の競合状態修正

## Summary

音声入力のバッファフラッシュ（クリップボード経由のペースト）において、`save_paste_and_restore` への複数スレッドからの同時アクセスによる競合状態を修正する。
修正対象は `src/input/clipboard.rs` のクリップボード操作関数群に対する排他制御の追加。

## Background

音声入力のバッファフラッシュは、クリップボード経由でテキストをカーソル位置にペーストする仕組みである。
フラッシュ処理は以下のパターンで実装されている（`save_paste_and_restore`, `src/input/clipboard.rs:55-68`）：

1. 現在のクリップボード内容を変数に退避
2. 音声認識テキストをクリップボードにセット
3. Ctrl+V / Cmd+V でペースト
4. 退避した内容をクリップボードに復元

この基本パターンは正しいが、以下の問題により競合状態が発生する：

- フラッシュを実行するコードパスが2つの異なるスレッド（ホットキーハンドラスレッド / STTイベントループ）に分岐している
- `save_paste_and_restore` 全体を保護する排他制御がないため、両スレッドが同時にクリップボード操作を行うと競合する
- Windows では Mac より頻繁に「もともとクリップボードにあった内容がフラッシュされる」現象が報告されている（OSのクリップボードAPIの排他動作の差による）

## Scope

- `src/input/clipboard.rs` にクリップボード操作用のMutex（排他ロック）を追加する
- `save_paste_and_restore` 全体を Mutex で保護し、複数スレッドからの同時アクセスを直列化する
- `replace_selected_text` にも退避/復元パターンを追加する（現在は退避なしでクリップボードを上書きしている）
- `get_selected_text` も Mutex 保護対象とする（同関数もクリップボードを操作するため）

## Non-scope

- `pending_flush` の設計変更（遅延フラッシュ自体の仕組みは変更しない）
- フラッシュの経路統合（ホットキーハンドラとSTTイベントループの分岐は維持）
- フロントエンドの変更
- ホットキー検出ロジックの変更

## Investigation

### フラッシュを実行する3つのコードパス

すべての経路は最終的に `clipboard::save_paste_and_restore(&flush_text)` を呼び出す。

**経路①: HotkeyAction::BufferFlush** (`src/tauri_cmd/system.rs:311-360`)
- スレッド: ホットキーハンドラスレッド（`tauri::async_runtime::spawn`）
- `save_paste_and_restore` 呼び出し: 328行目

**経路②: PostCorrectionFinished + pending_flush** (`src/mode/cl/main_of_cl.rs:678-744`)
- スレッド: STT イベントループ（main_of_cl.rs の async コンテキスト）
- `save_paste_and_restore` 呼び出し: 692行目

**経路③: SttCompleted + pending_flush** (`src/mode/cl/main_of_cl.rs:846-912`)
- スレッド: STT イベントループ（同上）
- `save_paste_and_restore` 呼び出し: 858行目

### 競合が発生するタイムライン

経路①（ホットキーハンドラ）と経路②/③（STTイベントループ）は**別スレッド**で動作する。

```
前提: ユーザーが他アプリからテキストコピー → クリップボード = "元のテキスト"
  ↓
Alt ダブルタップ（録音開始）→ 発話 → STT完了 → pending_flush
  ↓
経路③発動: save_paste_and_restore("認識テキスト") 開始
  ├─ saved = "元のテキスト"              ← 退避
  ├─ set_clipboard("認識テキスト")       ← セット
  │                                      ← ★ここで Alt ダブルタップ！
  │                                        別スレッドが経路①を実行
  │  [別スレッド]
  │  save_paste_and_restore("認識テキスト2") 開始
  │    ├─ saved2 = "認識テキスト"        ← 経路③がセットした値を保存！
  │    ├─ set_clipboard("認識テキスト2")
  ├─ send_cmd_v()                       ← "認識テキスト" をペースト（一見OK）
  ├─ sleep(50ms)
  ├─ set_clipboard(saved)               ← "元のテキスト" を復元
  │                                      ← ★クリップボードが "元のテキスト" に
  │  [別スレッド]
  │    ├─ send_cmd_v()                  ← ★"元のテキスト" がペーストされる = バグ！
  │    ├─ sleep(50ms)
  │    ├─ set_clipboard(saved2)         ← "認識テキスト" を復元
```

### 関連問題

**`replace_selected_text`** (`src/input/clipboard.rs:71-78`):
退避/復元なし。`Correct` / `Summarize` ホットキーから呼ばれ、クリップボードをそのまま残す。

**`get_selected_text`** (`src/input/clipboard.rs:29-51`):
クリップボードを退避 → Ctrl+C → 取得。選択なしの場合のみ復元する。`save_paste_and_restore` と同時実行時に競合する。

## Test Plan

**テスト対象**: `src/input/clipboard.rs` の全公開関数

**注意**: クリップボード操作は OS のグローバルリソースであり、単体テストでの完全な分離は困難。
`arboard` 自体の動作をモック化するか、あるいは関数分離によりロジック部分のみをテストする方針とする。

### テストケース一覧

| # | ケース | 種別 | 内容 |
|---|--------|------|------|
| 1 | Mutex排他: 逐次アクセス | 正常系 | `save_paste_and_restore` を1スレッドで呼び出し、退避→セット→復元が正しく機能することを確認 |
| 2 | Mutex排他: 並行アクセス | 異常系 | 2スレッドから同時に `save_paste_and_restore` を呼び出し、競合状態が発生しないことを確認（順番は不定でも内容の不整合がないこと） |
| 3 | `replace_selected_text` 退避/復元 | 正常系 | 退避/復元追加後、クリップボードが呼び出し前と同じ内容に戻ることを確認 |
| 4 | `get_selected_text` 排他 | 正常系 | `save_paste_and_restore` と `get_selected_text` の同時実行で競合しないことを確認 |

**テスト方針**: `arboard` のモックラッパーを作成し、内部の退避/復元ロジックと Mutex 動作を検証する。実 OS のクリップボードに依存するテストは min とする。

## Boy Scout Rule — 翻訳可能性計画

- `clipboard.rs` の全公開関数に排他制御（Mutex）を追加することで、責務として「スレッドセーフなクリップボード操作」を明確にし、呼び出し側でロックを意識する必要をなくす
- `replace_selected_text` に `save_paste_and_restore` と同じ退避/復元パターンを適用し、「選択テキストを置換する」という関数名の責務と動作を一致させる（現在はクリップボードを汚染する副作用がある）
- 関数コメントを「何を」から「なぜ退避/復元が必要か」に更新する

## Acceptance Criteria

- [ ] `clipboard.rs` に Mutex が追加され、`save_paste_and_restore` / `get_selected_text` / `replace_selected_text` の全クリップボード操作が排他制御されている
- [ ] `replace_selected_text` が退避/復元パターンで実装され、呼び出し後にクリップボードを汚染しない
- [ ] 2スレッドからの同時 `save_paste_and_restore` 呼び出しで競合が発生しないことをユニットテストで確認
- [ ] 既存の全テストが通過している（`make test` パス）
- [ ] `make check` でコンパイルが通る

## Notes
