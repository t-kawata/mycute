# 計画: チケット #74 — HELP プロトコル状態機械の実装

## 要件
RFC §41B.4-41B.9 の HELP 5段階プロトコルを純粋状態機械として `src/help.rs` に実装。

## 変更ファイル一覧
| ファイル | 種別 | 内容 |
|----------|------|------|
| `src/help.rs` | 新規 | HelpState, HelpSession, 全構造体, 遷移関数, emit 関数, mod tests |
| `src/lib.rs` | 修正 | `pub mod help;` + `pub use` 追加 |

## 遷移行列
- Proposal → Offered (HelpOffered)
- Offered → Accepted (HelpAccepted)
- Offered → Rejected (HelpRejected)
- Accepted → Executing (HelpExecuted)
- Executing → Succeeded (HelpSucceeded)
- Executing → Failed (HelpAbandoned)
- 終端状態: Rejected, Succeeded, Failed（再遷移禁止）

## 計装・観測
- テスト: src/help.rs mod tests 内, T-1〜T-10 + T-O1〜T-O3
- PRNG: StdRng::seed_from_u64(12345)
- サンプル: T-O1 n=10,000, T-O2 n=5,000, T-O3 n=1,000

## 実装手順
1. src/help.rs — 型定義
2. is_legal_help_transition 純粋関数
3. HelpSession::transition_to ガード
4. emit_help_event EventBus publish
5. lib.rs モジュール登録
6. テストコード
7. cargo test + clippy

## レビュー方法
- 遷移行列総当たりテスト (T-1) で機械的検証
- EventBus publish テスト (T-6) で一致確認
- run-quality-checks.js
- cargo clippy -- -D warnings

## リスク
- 低（既存コード非改変）
