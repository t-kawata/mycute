---
ticket_id: 129
title: M1.76-KW-FIX-B: 子供ノードの lifecycle_score = 0 問題の修正 — experience=0 による usage=0 の解決
slug: m176-kw-fix-b-lifecycle-score-0-experience0-usage0
status: reviewed
created_at: 2026-05-27
updated_at: 2026-05-27
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0129-m176-kw-fix-b-lifecycle-score-0-experience0-usage0/plan.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0129-m176-kw-fix-b-lifecycle-score-0-experience0-usage0/observation-20260527-182358.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0129-m176-kw-fix-b-lifecycle-score-0-experience0-usage0/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0129-m176-kw-fix-b-lifecycle-score-0-experience0-usage0/review.md
---
# M1.76-KW-FIX-B: 子供ノードの lifecycle_score = 0 問題の修正 — experience=0 による usage=0 の解決

## Summary

子供ノードは初期化時に `experience=0` で開始されるため、`compute_experience_normalization(0) = 1.0 - exp(0) = 0.0` となり、lifecycle_score の usage 成分が 0 になる。幾何平均は 1 成分でも 0 なら全体が 0 になるため、ライフサイクルスコアが 0 に固定される。この結果、GC hazard の lifecycle 項が全く効かず、`gamma_child_protect=10.0` という極端な定数で強引に子供を保護している状態を解消する。

## Background

- RFC §15.9.2 の s_growth 因子、j_lifecycle 成分は lifecycle_score の平均値を入力とする。子供ノードが lifecycle_score=0 のままでは j_lifecycle が常に低く、s_growth が過小評価される。
- RFC §41A.5: lifecycle_score は 5 成分（freshness, success, trust, usage, reputation）の幾何平均。
- GC hazard (F-7): λ_gc = softplus(λ0 - γ_lifecycle·L - γ_benevolence·B - γ_child_protect·P)。lifecycle_score=0 のため lifecycle 項が 0 になり、gamma_child_protect=10.0 だけが子供の生存を担保している。この値は生命表から見て非現実的に高い。
- j_child_survival 指標も lifecycle_score に依存しており（s_growth の構成要素）、子供の lifecycle_score が正常化されればより正確な評価が可能になる。

## Scope

- 子供ノードの lifecycle_score が非ゼロになる修正を 1 つ採用
- gamma_child_protect 定数の調整（緩和の余地がある場合）
- 修正前後の子供 lifecycle_score 分布の観測

## Non-scope

- RFC §41A.5 の幾何平均定義自体の変更（usage 除外はオプションとして検討対象）
- HELP プロトコルの構造的変更（FIX-C で対応）
- j_pop_growth 計算式の修正（FIX-A で対応済み）

## Investigation

### 現象の再現手順

`run_evaluation_simulation` において、初期ノードの experience 設定（simulation.rs:1484）:
```rust
node_experiences.insert(id, if id < init_child_count { 0 } else { 10 });
```

子供ノード（id < init_child_count）には `experience=0` が設定される。

### 原因特定（物理的証拠）

**証拠 1: experience=0 で usage=0.0 が確定**

`phase4_gc_survival`（simulation.rs:1922-1923）:
```rust
let usage = compute_experience_normalization(node_experiences.get(&id).copied().unwrap_or(0));
```

`compute_experience_normalization(0)`（reciprocity.rs:298-300）:
```rust
pub fn compute_experience_normalization(experience_count: u64) -> f64 {
    let scale = crate::constants::EXPERIENCE_NORMALIZATION_SCALE; // = 10.0
    1.0 - (-(experience_count as f64) / scale).exp()
}
```

`experience=0` → `1.0 - exp(0) = 1.0 - 1.0 = 0.0`。

**証拠 2: usage=0 → lifecycle_score=0**

lifecycle_score は 5 成分の幾何平均（lifecycle.rs:40-43）:
```rust
pub fn compute_lifecycle_score(score: &LifecycleScore) -> f64 {
    let product = score.freshness * score.success * score.trust * score.usage * score.reputation;
    product.powf(1.0 / 5.0)
}
```

usage=0 → product=0 → lifecycle_score=0。既存テスト `test_lifecycle_score_one_zero`（lifecycle.rs:87-101）で確認済み。

**証拠 3: lifecycle_score=0 → GC hazard が lifecycle 項を無視**

`compute_gc_hazard`（reciprocity.rs:303-314）:
```rust
let inner = policy.lambda_gc_base
    - policy.gamma_lifecycle * lifecycle_score   // = 0.5 * 0.0 = 0
    - policy.gamma_benevolence * benevolence_score
    - policy.gamma_child_protect * child_protection;  // = 10.0 * 0.5 = 5.0
```

lifecycle_score=0 により `gamma_lifecycle` 項が完全に無効化。`gamma_child_protect=10.0`（constants.rs:726）のみが子供を保護。

**証拠 4: 現行 TC8 テストが experience=0 → 0.0 をアサート**

reciprocity.rs:5322-5328:
```rust
let r = compute_experience_normalization(0);
assert!((r - 0.0).abs() < f64::EPSILON, "experience=0 must be 0.0 (got {})", r);
```

オプション c（offset 導入）を選択した場合、このテストは更新が必要。

### 3 修正オプションの評価

| オプション | 影響範囲 | 副作用 | 実装難易度 |
|------------|---------|--------|-----------|
| a. usage を幾何平均から除外 | lifecycle.rs compute_lifecycle_score | 全ノードで usage が lifecycle 評価に効かなくなる | 低（2行） |
| b. 子供に minimum usage (0.1) | simulation.rs phase4_gc_survival | 子供限定、成人に影響なし | 中（条件分岐） |
| c. offset 項導入 | reciprocity.rs compute_experience_normalization | 全ノードの usage 計算が変わる。TC8 更新必要 | 中（定数追加 + テスト更新） |

### 参照観察レポート

- tickets/context/0128-m176-kw-fix-a-j-pop-growth-alive-count/observation-20260527-174849.md — FIX-A 観察結果。j_pop_growth バグ修正後のベースライン
- tickets/context/0127-m176-kw4-kind-world-j-kw-social-j-kw-s-speed/observation-20260527-163702.md — 較正ループ結果。J_kw_social が 0.061 で頭打ち、j_lifecycle が 0 に近い

## Test Plan

### ユニットテスト

1. **FIX-B1**: experience=0 の子供ノードで lifecycle_score > 0 になること
   - 修正選択肢に応じてアサーションを設定
2. **FIX-B2**: experience 増加に伴い usage が単調増加すること（既存 TC8 と重複）
3. **FIX-B3**: lifecycle_score 正常化後も子供保護が適切に機能すること
   - gamma_child_protect 緩和後も子供の GC 生存率が許容範囲内であることを確認
4. **FIX-B4**: 既存テスト全 PASS（regression 確認）

### 観測テスト（--nocapture）

5. **FIX-B5**: 修正前後の子供 lifecycle_score 分布（最小値・平均・分散）を観測
6. **FIX-B6**: usage 値、幾何平均 5 成分ごとの内訳を出力
7. **FIX-B7**: GC hazard 値（子供 vs 成人）の変化を追跡

## 計装方法・観測対象

### 計装方法
- `phase4_gc_survival` 内に lifecycle_score の 5 成分内訳を `println!` で出力
- 修正前後の子供/成人別 lifecycle_score 平均を観測テストで出力
- 固定シード `StdRng::seed_from_u64(12345)` を使用

### 観測対象
- 子供ノードの lifecycle_score 最小値（修正前 0.0 → 修正後 > 0.0）
- usage 値の子供/成人別分布
- GC hazard 値の子供/成人別比較
- gamma_child_protect 緩和後の子供生存率変化

### 較正計画
- 修正対象定数: `GC_HAZARD_GAMMA_CHILD_PROTECT`（現行 10.0）
- 緩和目標: lifecycle_score 正常化後、gamma_child_protect を 10.0 → 2.0-5.0 程度に低減
- 適正値は子供の GC 生存率が成人と同等〜やや高めになる範囲で決定

## Boy Scout Rule — 翻訳可能性計画

- `phase4_gc_survival` 内の lifecycle_score 計算ブロック（simulation.rs:1914-1935）は 5 成分の組立と GC hazard 計算が混在している。責務分離を検討
- `compute_mean_lifecycle_score`（kind_world.rs:2155）が GcEvent 状態ベースのプロキシ値を使用しており、真の lifecycle_score（5 成分幾何平均）と乖離している可能性。必要に応じて本物の lifecycle_score に置き換え

## Acceptance Criteria

- [ ] experience=0 の子供ノードで lifecycle_score > 0 になる（FIX-B1）
- [ ] experience 増加に伴い usage が単調増加する（FIX-B2）
- [ ] lifecycle_score 正常化後も子供保護が適切に機能する（FIX-B3）
- [ ] 既存テスト全 PASS（FIX-B4）
- [ ] 翻訳可能性の検証が通っている
- [ ] 観測テストで修正前後の lifecycle_score 分布が記録されている

## Notes

- plan_path: context/0129-m176-kw-fix-b-lifecycle-score-0-experience0-usage0/plan.md（未作成）
- implementation_path: context/0129-m176-kw-fix-b-lifecycle-score-0-experience0-usage0/implementation.md（未作成）
- review_report_path: context/0129-m176-kw-fix-b-lifecycle-score-0-experience0-usage0/review.md（未作成）
- observation_report_path: context/0129-m176-kw-fix-b-lifecycle-score-0-experience0-usage0/observation-YYYYMMDD-HHmmss.md（未作成）

### 成果物

- 計画: context/0129-m176-kw-fix-b-lifecycle-score-0-experience0-usage0/plan.md（未作成）
- 実装サマリ: context/0129-m176-kw-fix-b-lifecycle-score-0-experience0-usage0/implementation.md（未作成）
- レビュー報告書: context/0129-m176-kw-fix-b-lifecycle-score-0-experience0-usage0/review.md（未作成）
- 観察レポート: context/0129-m176-kw-fix-b-lifecycle-score-0-experience0-usage0/observation-YYYYMMDD-HHmmss.md（未作成）
