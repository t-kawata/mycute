---
ticket_id: 130
title: HELP プロトコルの任意ペア化 + proposal 生成機構の実装 — 3 層問題の解決 ★最重要
slug: help-proposal-3
status: reviewed
created_at: 2026-05-27
updated_at: 2026-05-27
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0130-help-proposal-3/plan.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0130-help-proposal-3/observation-20260527-184137.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0130-help-proposal-3/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0130-help-proposal-3/review.md
---

# HELP プロトコルの任意ペア化 + proposal 生成機構の実装 — 3 層問題の解決 ★最重要

## Summary

Kind World 較正ループ（#127）の結果、J_kw_social が 0.061 で頭打ちになった最大の原因は、
HELP プロトコルが **成人→子供の単方向ハードフィルタ** になっていることにある。これにより
reciprocity_pair_counts に双方向ペアが原理的に発生せず、j_reciprocity = 0 固定、
s_topology に天井 ~0.48 が発生している。本チケットは 3 層に積層したこの問題を解決する。

## Background

### 問題の経緯

1. Cycle 3（#127 較正）で、j_help=0.40 が有効化されたが j_reciprocity=0.0 が常時であることが判明
2. j_reciprocity=0 により s_topology の 6 因子中 1 つが恒久的に 0 → s_topology 天井 ~0.48
3. s_topology 天井が J_kw_social の上限 0.061（=観測値）を引き起こす

### 3 層の積層問題

**Layer 1 — 実装バグ（ハードフィルタ）:** `simulation.rs:1788-1796` で helper は成人ノードのみ、
helpee は子供ノードのみに制限されている。reciprocity_pair_counts が常に一方向（成人→子供）となり、
双方向ペア（a→b と b→a が両方存在）が原理的に発生不能。

**Layer 2 — RFC 設計矛盾:** RFC §41B-9 の条件式 HelpProposal(h→c|M) ⇔ Child(c) ∧ Adult(h) が
成人→子供のハードフィルタを RFC 自身で定義している。実装バグ（Layer 1）を RFC が正当化・固定化している。

**Layer 3 — バイアス方向の誤り:** RFC §41B.20.1 F-11 の helper quality
Q = w_s·S + w_t·T(h) + w_r·Rep(h) + w_b·B(h) + w_n·N(c) - w_d·d は helper 側バイアス
（T, Rep, B 項が「大人を選びやすく」する）を作り出す。正しい設計は helpee 側バイアス
（「子供が helpee の時、より多くの周囲が提案を生成する」）であるべき。F-11 の helper quality 計算自体は
妥当だが、その前に提案生成の確率バイアス機構が存在すべき。

## Scope

1. **simulation.rs:1779-1796**: 任意の alive ペアから helper/helpee を選択するよう修正
   - 成人制限・子供制限を撤廃し、全 alive ノードから無作為選択
   - 子供が helpee として選択される確率にバイアスをかける機構を追加
2. **simulation.rs:1801**: `reciprocity_pair_counts` の記録を修正（任意ペア対応）
3. **観測テスト**: FIX-C1〜C8 の追加（simulation.rs）
4. **既存テスト**: 回帰がないことを確認

## Non-scope

1. **RFC 改訂（§41B-9 / F-11）**: 別チケットに分離。本チケットは実装修正に限定
2. **HELP 以外の相互作用機構**: 変更しない
3. **gamma_child_protect 等の GC 定数調整**: FIX-B 完了済み。本チケットでは触らない
4. **help_successes 二重処理バグ（FIX-D）**: 別チケット（#131 相当）。FIX-C 完了後に対応

## Investigation

### 物理的証拠一覧

#### 証拠 1: phase3_help_protocol のハードフィルタ（simulation.rs:1779-1796）

```rust
let alive_adults: Vec<NodeId> = (0..ctx.population_count())
    .filter(|id| !dead.contains(id) && *is_adult.get(id).unwrap_or(&true))
    .collect();
let alive_children: Vec<NodeId> = (0..ctx.population_count())
    .filter(|id| !dead.contains(id) && !*is_adult.get(id).unwrap_or(&true))
    .collect();

for &helper_id in &alive_adults {            // helper は成人 ONLY
    ...
    let child_id = alive_children[child_idx]; // helpee は子供 ONLY
    ...
}
```

- **確認日**: 2026-05-27
- **該当行**: simulation.rs:1779-1796
- **状態**: 現行コードで確認。変更なし。

#### 証拠 2: reciprocity_pair_counts の単方向性（simulation.rs:1801）

```rust
*ctx.reciprocity_pair_counts.entry((helper_id, child_id)).or_insert(0) += 1;
```

helper_id は常に adult、child_id は常に child → 常に (adult, child) 方向のみ記録。
(adult, adult) / (child, child) / (child, adult) の全パターンが原理的に発生不能。

- **確認日**: 2026-05-27
- **該当行**: simulation.rs:1801
- **状態**: 現行コードで確認。

#### 証拠 3: compute_mean_reciprocity の双方向ペア要件（kind_world.rs:2254-2273）

```rust
fn compute_mean_reciprocity(pair_counts: &HashMap<(NodeId, NodeId), u64>) -> f64 {
    ...
    for (&(a, b), &count) in pair_counts {
        if a != b {
            let reverse = pair_counts.get(&(b, a)).copied().unwrap_or(0);
            symmetric_sum += count.min(reverse);  // a→b と b→a の両方が必要
        }
        total_interactions += count;
    }
    symmetric_sum / total_interactions  // 単方向のみ → 常に 0
}
```

- **確認日**: 2026-05-27
- **該当行**: kind_world.rs:2254-2273
- **状態**: 現行コードで確認。
- **結論**: 単方向ペアしか存在しないため、pair_counts.get(&(b, a)) が常に 0 → symmetric_sum が常に 0

#### 証拠 3b: j_reciprocity が s_topology の天井を決定（kind_world.rs:409-437）

```rust
let j_reciprocity = metrics.mean_reciprocity_score.clamp(0.0, 1.0);
let s_topology = (j_benevolence + j_reciprocity + j_help + j_trust + j_clustering + j_local_density) / 6.0;
```

j_reciprocity=0 → s_topology の最大値は (1.0+0+1.0+1.0+1.0+1.0)/6 = 0.833... だが、
他の成分も抑圧されるため実際の天井 ≈ 0.48（観測値）。

- **確認日**: 2026-05-27
- **該当行**: kind_world.rs:409-411, 436-437
- **状態**: 現行コードで確認。FIX-A/B でも未修正。

#### 証拠 4: RFC §41B-9 のハードフィルタ定義（RFC:7895-7901）

```
HelpProposal(h → c | M) ⇔ Child(c) ∧ Adult(h) ∧ h ∈ N_t(c) ∧ Q(h,c,M) ≥ θ_proposal
```

RFC 自身が Child(c) ∧ Adult(h) を提案条件として定義している。
これが Layer 2（RFC 設計矛盾）の直接の証拠。

- **確認日**: 2026-05-27
- **RFC 行番号**: 7895-7901

#### 証拠 5: RFC §41B.20.1 F-11 の helper 側バイアス（RFC:8219-8221）

```
Q(h,c,M) = w_s·S(h,c,M) + w_t·T(h) + w_r·Rep(h) + w_b·B(h) + w_n·N(c) - w_d·d(h,c)
```

T(h), Rep(h), B(h) は helper の属性のみに依存 → helper 側バイアス。
「大人 = 高信頼・高評判」の相関がある場合、成人 helper が自動的に選ばれやすい。
正しくは Proposal 生成段階で「子供が helpee の時により多くの提案が生成される」
という helpee 側バイアスを実装すべき（Layer 3）。

#### 証拠 6: Cycle 3 観測（#127 observation）による実測値

```
J_kw_social = 0.061, j_help = 0.40, j_reciprocity = 0.0
s_topology 天井 ≈ 0.48（j_reciprocity=0 による構造的制約）
```

- **出典**: observation-20260527-163702.md
- **状態**: FIX-A/B 完了後も j_reciprocity=0 は未解決

### 参照観察レポート

- observation-20260527-163702.md（#127 Cycle 3） — j_reciprocity=0.0 発見
- observation-20260527-174849.md（#128 FIX-A） — j_pop_growth バグ修正、j_reciprocity=0 は未解決
- observation-20260527-182358.md（#129 FIX-B） — lifecycle_score 正常化完了、j_reciprocity=0 は未解決

## Test Plan

### FIX-C1〜C8 テスト一覧

| ID | テスト名 | ファイル | 種別 | 内容 |
|----|---------|---------|------|------|
| C1 | `test_fixc_adult_to_adult_help` | simulation.rs | 不変条件 | 成人→成人の HELP 発生確認 |
| C2 | `test_fixc_child_to_child_help` | simulation.rs | 不変条件 | 子供→子供の HELP 発生確認 |
| C3 | `test_fixc_child_to_adult_help` | simulation.rs | 不変条件 | 子供→成人の HELP 発生確認 |
| C4 | `test_fixc_adult_to_child_help` | simulation.rs | 不変条件 | 成人→子供の HELP 発生確認（従来方向も維持） |
| C5 | `test_fixc_bidirectional_pairs` | simulation.rs | 不変条件 | 双方向ペア出現確認 |
| C6 | `test_fixc_mean_reciprocity_positive` | simulation.rs | 不変条件 | compute_mean_reciprocity > 0 |
| C7 | `test_fixc_child_helpee_bias` | simulation.rs | 観測 | 子供 helpee 確率が成人より高いことの観測 |
| C8 | `test_fixc_existing_tests_pass` | simulation.rs | 不変条件 | 既存テスト全 PASS |

### 観測テスト詳細

- **C7**: 4 種類の HELP ペア方向ごとの発生回数と割合を println! 出力。子供 helpee 比率 vs 成人 helpee 比率を比較。
- **C5 補足**: 双方向ペア数と単方向ペア数を total_interactions と共に出力。
- **固定シード**: StdRng::seed_from_u64(12345)

## 計装方法・観測対象

### 計装方法

- simulation.rs の test_fixc_* テストで実装
- 観測テストは println! + --nocapture で構造化出力
- 固定シード StdRng::seed_from_u64(12345)

### 観測対象

- 4 方向 HELP ペアの発生回数と割合
- 双方向ペア数 / 単方向ペア数
- compute_mean_reciprocity の値（0 からの改善）
- サンプルサイズ: シミュレーション全ノード（n >= 100）

### 較正計画

本チケットは実装修正が主眼。以下の定数がバイアス制御に関わる場合に調整：

- **CHILD_HELPEE_BIAS_FACTOR**（新規）: 子供 helpee バイアスの強さ
- 目的関数: j_reciprocity > 0 かつ compute_mean_reciprocity > 0.1
- 停止条件: 4 方向すべての HELP ペア発生 + j_reciprocity > 0.01

## Boy Scout Rule — 翻訳可能性計画

1. **phase3_help_protocol の責務分割**: proposal 生成と session 進行管理を独立関数に分割
2. **生存ノードフィルタリング**: 年齢別グループ化をヘルパー関数に抽出
3. **ハードコード値の定数化**: mission_rate 閾値・バイアス確率を constants.rs に定義

## Acceptance Criteria

- [ ] FIX-C1: 成人→成人の HELP が少なくとも 1 回発生する
- [ ] FIX-C2: 子供→子供の HELP が少なくとも 1 回発生する
- [ ] FIX-C3: 子供→成人の HELP が少なくとも 1 回発生する
- [ ] FIX-C4: 成人→子供の HELP が少なくとも 1 回発生する
- [ ] FIX-C5: reciprocity_pair_counts に双方向ペアが出現する
- [ ] FIX-C6: compute_mean_reciprocity > 0 になる
- [ ] FIX-C7: 子供が helpee となる確率が成人よりも統計的に高いことが観測される
- [ ] FIX-C8: 既存テスト全 PASS

## Notes

### 実行順序制約

- FIX-B（#129）は完了済み。本チケットの前提条件成立。
- FIX-A（#128）は完了済み。j_pop_growth 正常化は本チケットに影響しない。
- 観測テスト（FIX-B5/B6/B7）との競合は生じない設計とする。

### 成果物

- 計画: context/0130-help-proposal-3/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0130-help-proposal-3/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0130-help-proposal-3/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0130-help-proposal-3/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成）
