# 実装サマリー: adult HELP offer policy と child consent policy の純粋判定器実装

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|---------|------|------|
| src/constants.rs | 追加 | 12個の HELP policy 定数（Calibration Candidates） |
| src/help.rs | 追加 | 6新規型 + 4判定関数 + 11テスト関数 |
| src/lib.rs | 更新 | 公開API再エクスポート追加 |

## 新規型定義
- AdultHelpOfferPolicy (quality_weight, load_penalty, risk_penalty, threshold) + Default impl
- ChildHelpAcceptancePolicy (gamma1-3, quality_weight, uncertainty_weight, autonomy_penalty, threshold) + Default impl
- OfferScoreBreakdown (distance_term, maturity_term, reciprocity_term, reputation_term, urgency_term)
- OfferDecision { Offer, Abstain(HelpRejectionReason) }
- ChildDecision { Accept, Reject(HelpRejectionReason), Abstain }
- HelpRejectionReason 拡張: +Unsafe, Irrelevant, Overloaded, ResourceExhausted（6→10 variant）

## 新規判定関数
- should_offer_help() — 式 41B-10: a₁Q - a₂L_load - a₃P_risk ≥ θ_offer
- compute_offer_score_breakdown() — 5項のスコア内訳計算
- child_need_score() — 式 41B-12: γ₁(1-Ẽ) + γ₂(1-T) + γ₃(1-L)
- decide_help_offer() — 式 41B-13: b₁Q + b₂U - b₃A ≥ θ_accept

## テスト結果
- 既存テスト: 754 → 771 (+17) 全て PASS
- 新規不変条件テスト 8件 (T-1~T-8): 全て PASS
- 新規観測テスト 3件 (T-O1~T-O3): 全て PASS
- 警告: 0件
