# 実装サマリ: HELP プロトコル状態機械 (M1.75-3 / Ticket #74)

## 変更ファイル
| ファイル | 種別 | 内容 |
|----------|------|------|
| src/help.rs | 新規 | HELP 状態機械全体（型定義、遷移関数、EventBus publish、13テスト） |
| src/lib.rs | 修正 | pub mod help; 追加 + pub use で全公開型を再公開 |
| src/error.rs | 修正 | DarviumError::HelpTransitionViolation 追加 |

## 実装した機能
- HelpState 7状態 (Proposal/Offered/Accepted/Rejected/Executing/Succeeded/Failed)
- HelpSession 状態機械コンテナ + transition_to ガード
- is_legal_help_transition 純粋関数（6通りの合法遷移）
- transition_to_event マッピング（遷移種別→ReciprocityEvent variant）
- emit_help_event EventBus publish
- 全支援構造体（HelpProposal/HelpOffer/HelpDecision/HelpExecution/HelpSuccess/HelpFailure）
- RFC §41B.4 準拠の HelpOfferState / HelpMode 列挙型
- HelpRejectionReason / HelpFailureReason 列挙型

## テスト結果
- 全760テスト PASS（既存747 + 新規13）
- clippy 通過（-D warnings）
- 観測テスト: T-O1 違法遷移フラックス 87%（期待値 87.8% と一致）
- 観測テスト: T-O2 吸収状態分布 33%/34%/33%（等確率ランダムウォークの期待値と一致）
- 観測テスト: T-O3 EventBus 一貫性 2,494 遷移中不一致 0 件
