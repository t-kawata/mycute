# 計画: チケット M-2-1.8 — Clock / VirtualClock 抽象トレイトの定義

## 要件
- Clock トレイト（now_ms / advance, Send+Sync, オブジェクト安全）
- 3 実装: VirtualClock（決定論的カウンタ）, SystemClock（実時間ラップ）, FrozenClock（固定値）
- テスト T1-T16

## 変更ファイル
| ファイル | 種別 | 内容 |
|---|---|---|
| src/clock/mod.rs | 新規 | Clock トレイト + 3実装 + テスト |
| src/lib.rs | 編集 | pub mod clock; 追加 |
| src/constants.rs | 編集 | CLOCK_DEFAULT_START_MS 追加 |

## 実装手順
1. constants.rs に定数追加
2. clock/mod.rs 作成（トレイト、実装、テスト）
3. lib.rs にモジュール登録
4. cargo test で確認

## レビュー方法
- cargo test -- --nocapture
- 翻訳可能性 grep
- run-quality-checks.js
