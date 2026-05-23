# M1-1 実装計画

## 要件
SearchWorkflow から `SearchOutcome::NeedsHumanReview` 発行時にミッションを `HumanReviewQueue` へ隔離し、`HumanChannel` 経由の人間応答が到着するまで自動実行ラインから絶対に復帰させない。

**中核: $P_{leak} = 0$ の隔離障壁 + $L_q(t) = \lambda t$ の線形成長観測**

## RFC 既存実装状態検証
- HumanReviewQueuePolicy: 全5フィールド未実装 (src/内に定義なし)
- SearchOutcome::NeedsHumanReview: ✅ types.rs:4222 に既存
- HumanChannel/InteractionHandle/HumanRequest/HumanDecision: ✅ 全型既存
- 定数: HUMAN_REVIEW_TIMEOUT_SECS(3600), ESCALATION(14400), MAX_BATCH(20) — 未定義

## 変更ファイル一覧
| # | ファイル | 種別 | 内容 |
|---|----------|------|------|
| 1 | src/constants.rs | 追加 | 3定数 (Environment Policy Knobs) |
| 2 | src/types.rs | 追加 | HumanReviewQueuePolicy struct |
| 3 | src/human_review_queue.rs | 新規 | QueuedReview/QueuedReviewStatus/HumanReviewQueue + T1-T10 |
| 4 | src/lib.rs | 修正 | pub mod + pub use |
| 5 | tests/m1_1.rs | 新規 | OTS-1〜OTS-4 |

## 実装手順
1. constants.rs に3定数追加
2. types.rs に HumanReviewQueuePolicy 追加
3. human_review_queue.rs 新規作成（キュー本体 + 不変条件テスト T1-T10）
4. lib.rs にモジュール登録
5. tests/m1_1.rs 作成（観測テスト OTS-1〜OTS-4）

## レビュー方法
1. run-quality-checks.js で spec 一致確認
2. cargo test --test m1_1 -- --nocapture で観測出力確認
3. 翻訳可能性 grep: 名詞始まり関数・汎用変数名・マジックナンバー

## リスク
- T8/T9 スレッドテストの経過時間が長い → Barrier + タイムアウト
- OTS-1 実時間待機 (λ=1 で 10秒) → テスト全体が 30秒超
