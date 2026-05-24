# 実装計画: adult HELP offer policy と child consent policy の純粋判定器実装

## 要件
RFC §41B.6-41B.7 に基づく純粋判定器の実装：
1. Adult 側 offer policy (式 41B-10): should_offer_help()
2. Child 側 need score (式 41B-12): child_need_score()
3. Child 側 consent policy (式 41B-13): decide_help_offer()

## RFC 既存実装状態検証
- HelpState/HelpProposal/HelpOffer/HelpDecision: ✅ 既存
- HelpRejectionReason (6→10 variant): ⚠️ 拡張必要
- AdultHelpOfferPolicy/ChildHelpAcceptancePolicy/OfferScoreBreakdown: ❌ 新規
- should_offer_help/child_need_score/decide_help_offer: ❌ 新規
- HELP 定数11個: ❌ 新規

## 変更ファイル一覧
| ファイル | 種別 | 内容 |
|---------|------|------|
| src/constants.rs | 追加 | 12の HELP policy 定数 |
| src/help.rs | 追加 | 6新規型 + 4判定関数 + 11テスト関数 |

## 実装手順
1. constants.rs に HELP policy 定数群を追加
2. help.rs に新規型定義 (HelpRejectionReason 拡張, OfferScoreBreakdown, OfferDecision, ChildDecision, AdultHelpOfferPolicy, ChildHelpAcceptancePolicy)
3. help.rs に判定純粋関数 (should_offer_help, child_need_score, compute_offer_score_breakdown, decide_help_offer)
4. 既存テスト影響確認
5. 不変条件テスト T-1〜T-8 追加
6. 観測テスト T-O1〜T-O3 追加
7. 全テスト実行確認

## 物理的レビュー方法
1. run-quality-checks.js src/help.rs src/constants.rs
2. 翻訳可能性 grep (名詞始まり関数、ハードコード数値、汎用変数名)
3. cargo check
4. cargo test

## リスク
- HelpRejectionReason variant 追加による既存 match 網羅性: #[non_exhaustive] で対応
- 浮動小数点 decision jitter: 比較イプシロン 1e-9
