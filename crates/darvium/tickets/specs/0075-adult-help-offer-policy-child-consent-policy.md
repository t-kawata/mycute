---
ticket_id: 75
title: adult HELP offer policy と child consent policy の純粋判定器実装
slug: adult-help-offer-policy-child-consent-policy
status: reviewed
created_at: 2026-05-24
updated_at: 2026-05-24
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0075-adult-help-offer-policy-child-consent-policy/plan.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0075-adult-help-offer-policy-child-consent-policy/observation-20260524-145607.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0075-adult-help-offer-policy-child-consent-policy/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0075-adult-help-offer-policy-child-consent-policy/review.md
---

# adult HELP offer policy と child consent policy の純粋判定器実装

## Summary

M1.75-4 は Child Support Villages / HELP Consensus（v2.3-e）マイルストーンの第4チケットとして、RFC §41B.6（Adult HelpOffer Policy）および §41B.7（Child consent / acceptance policy）で定義された純粋判定器を実装する。Adult 側の offer policy は式 41B-10 に基づき quality / load / risk の3軸で支援提案の妥当性を判定し、Child 側の consent policy は式 41B-12 / 41B-13 に基づき needs / quality / autonomy の3軸でオファーの受入可否を決定する。policy 層は M1.75-3 の状態機械に直結し、Proposal → Offered 遷移のガード条件として機能する。

## Background

M1.75-3（状態機械実装）は完了し、HELP プロトコルの7状態遷移（Proposal → Offered → Accepted/Rejected → Executing → Succeeded/Failed）が EventBus publish 付きで実装された。しかし現状の状態機械は **policy によるガードを持たない** 純粋な遷移行列であり、以下の重要な gap が存在する：

1. **Proposal → Offered 遷移に Adult 側の判断が存在しない**: 現状は `is_legal_help_transition` が真なら即座に Offered に遷移する。RFC §41B.6 では Adult 側の offer policy（負荷・リスク・適合性の評価）を通過して初めて Offer が成立する。
2. **Offered → Accepted 遷移に Child 側の判断が存在しない**: 現状は Accepted/Rejected を決定するロジックがなく、呼び出し元が任意に選択するだけ。RFC §41B.7 では Child 側の consent policy（ニーズ・品質・自律性の評価）を通過して初めて Acceptance が成立する。
3. **拒否理由コードが未整備**: 現状の `HelpRejectionReason`（6 variant）は policy の細分化された拒否理由（`Unsafe`, `Irrelevant`, `Overloaded` 等）を表現できない。

本チケットはこれらの gap を埋める純粋判定器を実装する。判定器は純粋関数として設計され、副作用を持たない（EventBus publish は呼び出し元の責任とする）。

### 参照観察レポート

- tickets/context/0074-help-helpproposalhelpofferhelpdecisionhelpexecutionhelpsuccess/observation-20260524-143958.md — HELP 状態機械 全13テスト PASS。違法遷移フラックス 86.98%、吸収状態分布均等。T-O2 の観測では「実運用では Policy（M1.75-4）により分布が偏る」と明記。
- tickets/context/0073-m175-2-child-adult-maturity-local-village/observation-20260524-141956.md — 村構成ロジック全テスト PASS。AdultCandidate フィルタリング実装済み。
- tickets/context/0072-m175-1-spacepositionembedding-villageposition/observation-20260524-140442.md — 位置更新ダイナミクス全テスト PASS。

## Scope

以下の実装を含む：

1. **`AdultHelpOfferPolicy` 構造体定義**: RFC §41B-10 のパラメータ（a₁, a₂, a₃, θ_offer）を保持
2. **`should_offer_help` 純粋関数**: `(child, adult, context, &AdultHelpOfferPolicy) -> Result<OfferDecision, DarviumError>`
   - 式 41B-10: `O(h,c,M) = 1{a₁Q(h,c,M) - a₂L_load(h) - a₃P_risk(M) ≥ θ_offer}`
   - OfferDecision は { offer, abstain } の2値 + 理由コード
3. **`ChildHelpAcceptancePolicy` 構造体定義**: RFC §41B-12 / 41B-13 のパラメータ（γ₁, γ₂, γ₃, b₁, b₂, b₃, θ_accept）を保持
4. **`child_need_score` 純粋関数**: 式 41B-12 `N(c) = γ₁(1-Ẽ(c)) + γ₂(1-T(c)) + γ₃(1-L(c))`
5. **`decide_help_offer` 純粋関数**: `(child, &HelpOffer, &ChildHelpAcceptancePolicy) -> Result<ChildDecision, DarviumError>`
   - 式 41B-13: `Accept(c,h,M) = 1{b₁Q(h,c,M) + b₂U(c,M) - b₃A(c,h) ≥ θ_accept}`
   - ChildDecision は { accept, reject, abstain } の3値 + 理由コード
6. **`OfferScoreBreakdown` 記録構造体**: `{ distance_term, maturity_term, reciprocity_term, reputation_term, urgency_term }`
7. **理由コード体系（拡張 `HelpRejectionReason`）**: `Unsafe`, `Irrelevant`, `Overloaded`, `InsufficientSimilarity`, `InsufficientTrust`, `DistanceExceeded`, `AutonomyLossRisk`, `NeedMismatch`, `ResourceExhausted`, `Other`
8. **較正定数追加**: `constants.rs` に `HELP_OFFER_*` および `HELP_ACCEPT_*` 定数群を追加

以下の実装は含まない：
- M1.75-3 の状態機械そのものの変更（政策情報の注入は policy 構造体として外付け）
- EventBus publish（判定結果の publish は呼び出し元が行う）
- TrainingMission 統合（M1.75-5 のスコープ）

## Investigation

### 既存コード分析

**help.rs**（M1.75-3 完了済み）:

現在の `HelpDecision` 構造体と `HelpRejectionReason` 列挙型は policy 用の細分化を想定していない。具体的には：

1. `HelpDecision` は Child 側の決定のみを表現（adult 側の abstain が表現できない）
2. `HelpRejectionReason` は6 variant（`InsufficientSimilarity`, `InsufficientTrust`, `DistanceExceeded`, `AutonomyLossRisk`, `NeedMismatch`, `Other`）だが、RFC の offer policy で使われる `Unsafe`, `Irrelevant`, `Overloaded` が不足している
3. `HelpOffer` 構造体は policy 判定結果を保持するフィールドを持たない（`offer_score_breakdown` がない）

**constants.rs**:

村関連の定数は `SPACE_POSITION_*` および `E_ADULT_THRESHOLD` / `T_ADULT_THRESHOLD` / `R_ADULT_THRESHOLD` まで実装済み。HELP policy 用の定数は未実装。

**修正方針**:

- `HelpRejectionReason` に `Unsafe`, `Irrelevant`, `Overloaded`, `ResourceExhausted` を追加（後方互換性を維持）
- 新規判定器の型（`OfferDecision`, `ChildDecision`, `OfferScoreBreakdown`）は help.rs に追加
- policy 構造体は help.rs に追加（専用モジュール分割は過剰）
- 定数は constants.rs に `HELP_OFFER_*` / `HELP_ACCEPT_*` として追加

## Test Plan

### 不変条件テスト

| ID | テスト名 | 内容 | 種別 |
|----|---------|------|------|
| T-1 | `adult_policy_false_no_execution` | Adult policy が false の場合、offer 不成立で終わること（execution path に進まない） | 不変条件 |
| T-2 | `child_consent_reject_blocks_execution` | Child consent が reject の場合、execution path が完全遮断されること | 不変条件 |
| T-3 | `offer_score_monotonicity` | 近距離・高成熟・高信頼な Adult が遠距離・低成熟・低信頼 Adult より高い offer score を持つこと | 単調性 |
| T-4 | `offer_score_breakdown_structure` | OfferScoreBreakdown の全項が非負で、期待される範囲内であること | 構造検証 |
| T-5 | `child_need_score_bounds` | child_need_score が [0.0, 1.0] の範囲内であること | 境界値 |
| T-6 | `accept_decision_correctness` | 各チューニングシナリオで accept/reject/abstain が期待通りに出ること | 決定論 |
| T-7 | `rejection_reason_mapping` | `Unsafe`, `Irrelevant`, `Overloaded` 等の reject reason が期待どおりに出ること | 理由コード |
| T-8 | `serde_roundtrip` | 新規型（OfferDecision, ChildDecision, OfferScoreBreakdown）の serde ラウンドトリップ | シリアライズ |

### 観測テスト

| ID | テスト名 | 内容 | サンプル |
|----|---------|------|---------|
| T-O1 | `offer_acceptance_phase_diagram` | child-adult ペア空間上に距離・信頼・評判・緊急度のパラメータグリッドを形成し、offer 発火率と accept 率の相図を計測 | n >= 10,000 |
| T-O2 | `acceptance_decision_surface_jitter` | acceptance decision surface の等高線を追跡し、閾値境界近傍での decision jitter を測定 | n >= 5,000 |
| T-O3 | `policy_boundary_sensitivity` | 各 policy パラメータ（θ_offer, θ_accept, 重み係数）の微小変動に対する判定結果の感度を測定 | n >= 3,000 |

### 検証方法

```bash
cargo test -- help::tests --nocapture
```

## Acceptance Criteria

1. ✅ `AdultHelpOfferPolicy` 構造体が定義され、`should_offer_help` が RFC 式 41B-10 に従って offer/abstain を判定する
2. ✅ `ChildHelpAcceptancePolicy` 構造体が定義され、`child_need_score` が RFC 式 41B-12 に従って child need を計算する
3. ✅ `decide_help_offer` が RFC 式 41B-13 に従って accept/reject/abstain を判定する
4. ✅ `OfferScoreBreakdown` が5項（distance/maturity/reciprocity/reputation/urgency）を保持する
5. ✅ T-1〜T-8 の全不変条件テストがパスする
6. ✅ T-O1〜T-O3 の全観測テストがパスする
7. ✅ `constants.rs` に HELP offer/accept 用の較正定数が追加されている
8. ✅ 既存の state machine テスト（T-1〜T-10, T-O1〜T-O3）に影響を与えない

## Risks and Mitigations

| リスク | 影響 | 緩和策 |
|--------|------|--------|
| 既存 `HelpRejectionReason` への variant 追加による既存コードの破壊 | Medium | `#[non_exhaustive]` 付き variant 追加で後方互換を維持 |
| Policy パラメータの不適切なデフォルト値 | Medium | RFC §41B.6-41B.7 の推奨値をデフォルトに設定し、Calibration Candidate として明示 |
| 浮動小数点比較による decision jitter | Low | threshold 比較に ε（1e-9）を使用し、T-O2 で jitter を測定 |

## Boy Scout Rule — 翻訳可能性計画

本チケットで新規作成する関数はすべて動詞句（`should_offer_help`, `child_need_score`, `decide_help_offer`）とし、構造体はドメイン名詞（`AdultHelpOfferPolicy`, `ChildHelpAcceptancePolicy`, `OfferScoreBreakdown`）とする。一関数一責務を厳守し、ハードコード値は名前付き定数に抽出する。エラーは `Result` で伝播し、握りつぶし禁止。
