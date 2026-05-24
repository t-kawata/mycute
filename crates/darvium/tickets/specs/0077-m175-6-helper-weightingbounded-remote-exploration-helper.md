---
ticket_id: 77
title: M1.75-6: helper weighting、bounded remote exploration、および helper 候補フィルタの実装
slug: m175-6-helper-weightingbounded-remote-exploration-helper
status: reviewed
created_at: 2026-05-24
updated_at: 2026-05-24
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0077-m175-6-helper-weightingbounded-remote-exploration-helper/plan.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0077-m175-6-helper-weightingbounded-remote-exploration-helper/observation-20260524-154834.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0077-m175-6-helper-weightingbounded-remote-exploration-helper/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0077-m175-6-helper-weightingbounded-remote-exploration-helper/review.md
---
# M1.75-6: helper weighting、bounded remote exploration、および helper 候補フィルタの実装

## Summary

M1.75-5 までで child-support mission の発行機構（`spawn_child_support_mission`）は実装済みだが、helper の選定は `village.adult_ids` をそのまま使用しており、重み付けや探索的多様性の制御が行われていない。本チケットでは、RFC §41B.12 に基づく helper 重み関数（式 41B-18）と bounded remote exploration（式 41B-19）を実装し、helper 候補フィルタ（ConsistencyState・maturity フィルタ）を導入する。

## Background

- 現状の `spawn_child_support_mission`（src/childsupport.rs:104）は、`village.adult_ids` をそのまま `helper_ids` として使用しており、距離減衰や信頼・レピュテーションによる重み付けがされていない
- 観察レポート（M1.75-5）では、`MAX_HELPERS_PER_MISSION=10` の境界効果が確認され、village サイズが大きい場合の helper 選定戦略（TOP-K 制限など）の必要性が示唆された
- RFC §41B.12 は、距離減衰 β、信頼 T(h)、レピュテーション R(h) に基づく正規化重み（式 41B-18）と、局所性ロックイン回避のためのε混合（式 41B-19）を規定している
- RFC §41B.3 の `filter_adult_candidates` は既存だが、`select_helpers` 内でハードフィルタ（Pending / NeedsRepair / Quarantined の排除）を確実に実行する必要がある

## Scope

1. **HelperWeight / HelperSelectionPolicy / RemoteExplorationPolicy 型定義**（src/childsupport.rs）
   - `HelperWeight { helper_id, weight, is_remote }` 構造体
   - `HelperSelectionPolicy { beta, trust_exponent, reputation_exponent, epsilon, top_k }` — 式 41B-18 の β, μ, ν と式 41B-19 の ε、および TOP-K 制限
   - `RemoteExplorationPolicy { epsilon, max_remote_fraction }` — 遠隔探索パラメータ

2. **helper 重み関数 `compute_helper_weights`**（式 41B-18）
   - `w_t(h|c) = exp(-β·d_t(h,c)) · T(h)^μ · R(h)^ν / Σ exp(-β·d_t(g,c)) · T(g)^μ · R(g)^ν`
   - L2 距離 `l2_distance` を再利用
   - 信頼 T(h) とレピュテーション R(h) は外部からパラメータとして受け取る純粋関数

3. **bounded remote exploration 混合 `mix_with_remote_exploration`**（式 41B-19）
   - `w̃_t(h|c) = (1-ε) · w_t(h|c) + ε · w^{remote}_t(h|c)`
   - ε = 0 → 100% 局所重み、ε = 1 → 100% remote 均等重み
   - remote 候補は既存の遠方 adult から均等サンプリング（距離降順の上位 max_remote_fraction 件）

4. **`select_helpers(child, village, policy) -> Vec<HelperWeight>`**
   - ハードフィルタ適用（`ConsistencyState != Committed`、repair pending、quarantined、adult maturity 未達 → 排除）
   - 距離計算 → 重み計算 → exploration 混合 → TOP-K 選抜
   - village の AdultCandidate 情報を引数に取る純粋関数

5. **定数定義**（src/constants.rs）
   - `HELPER_WEIGHT_DISTANCE_DECAY_BETA`（β, 較正候補, Default: 1.0）
   - `HELPER_WEIGHT_TRUST_EXPONENT`（μ, 較正候補, Default: 1.0）
   - `HELPER_WEIGHT_REPUTATION_EXPONENT`（ν, 較正候補, Default: 1.0）
   - `HELPER_WEIGHT_EXPLORATION_EPSILON`（ε, 較正候補, Default: 0.1）
   - `HELPER_WEIGHT_DEFAULT_TOP_K`（top_k, 較正候補, Default: 10 = MAX_HELPERS_PER_MISSION）

## Non-scope

- M1.75-7 の village stability/dynamicity metrics は本チケットでは実装しない
- M1.76 系の reciprocity/benevolence-aware helper weighting は本チケットでは導入しない（v2.3-f で additive に追加）
- 実データによる較正ループは本チケットでは行わない（定数定義と計装のみ。較正は M1.75-11 以降）
- EventBus への helper 選定イベント publish は本チケットでは実装しない

## Investigation

### 参照観察レポート

- `tickets/context/0076-m175-5-child-support-trainingmission-specialization-training-orchestrator/observation-20260524-152336.md` — MAX_HELPERS_PER_MISSION=10 の境界効果、village サイズと発行率の関係、spawn 成功率分布

### ソースコード調査結果

**現状の helper 選定（src/childsupport.rs:114-115）**:
```rust
ChildSupportMissionPayload::new(
    child_id,
    village.adult_ids.clone(),  // ← 重み付けなし全件採用
    village.clone(),
    ...
)?;
```
`select_helpers` による重み付け選定に置き換える必要がある。

**既存のフィルタ機構（src/village.rs:109-114）**:
```rust
pub fn filter_adult_candidates(candidates: Vec<AdultCandidate>) -> Vec<AdultCandidate> {
    candidates
        .into_iter()
        .filter(|c| c.consistency == ConsistencyStateTag::Committed && c.is_adult_maturity)
        .collect()
}
```
ハードフィルタの基本ロジックは確立済み。本チケットでは `select_helpers` 内でこれを確実に呼び出す。

**距離計算（src/spaceposition.rs:146-151）**:
```rust
pub fn l2_distance(a: &[f32; 3], b: &[f32; 3]) -> f64 { ... }
```
helper 重み関数で再利用可能。

**式 41B-18 の完全形（RFC §41B.12）**:
```
w_t(h|c) = exp(-β·d_t(h,c)) · T(h)^μ · R(h)^ν
         / Σ_{g∈H_t(c)} exp(-β·d_t(g,c)) · T(g)^μ · R(g)^ν
```

**式 41B-19 の完全形（RFC §41B.12）**:
```
w̃_t(h|c) = (1-ε) · w_t(h|c) + ε · w^{remote}_t(h|c)
```

**現状の定数（src/constants.rs）**:
- `MAX_HELPERS_PER_MISSION = 10`（M1.75-5 で定義済み）— この値と `HELPER_WEIGHT_DEFAULT_TOP_K` は一致させる
- 本チケットの β, μ, ν, ε 用の定数は未定義

## Test Plan

### テスト対象モジュール: `src/childsupport.rs` 内の `mod tests` に追加

以下の外部依存は関数パラメータとして注入する純粋関数設計のため、モック不要：

#### 不変条件テスト（T-1 〜 T-8）

| ID | 名称 | 内容 | 種別 |
|----|------|------|------|
| T-1 | 近距離高weight | 同一 quality なら近距離 helper の weight が遠距離 helper より必ず高い | 正常系 |
| T-2 | 高品質遠距離 > 低品質近距離 | quality が十分高ければ、適度に遠い helper が近距離低品質 helper を上回りうる | 正常系 |
| T-3 | ハードフィルタ除外 | Pending / NeedsRepair / Quarantined / non-Adult の候補が 1 件も選ばれない | 異常系 |
| T-4 | exploration ε=0 | ε = 0 で remote exploration が 0、全 helper が局所重みのみ | 境界値 |
| T-5 | exploration ε=1 | ε = 1 で常に remote sampling が発火、local weight が 0 | 境界値 |
| T-6 | 重みの正規化 | 出力 weight の総和が 1.0（±浮動小数点誤差）である | 不変条件 |
| T-7 | β=0 一様分布 | 距離減衰係数 β = 0 で距離に依存しない一様重みになる | 境界値 |
| T-8 | 空候補リスト | 空の候補リストに対して select_helpers が空リストを返す | 異常系 |

#### 観測テスト（T-O1 〜 T-O2）

| ID | 名称 | 内容 |
|----|------|------|
| T-O1 | β-ε 2次元グリッド掃引 | β ∈ {0.1, 0.5, 1.0, 2.0, 5.0}, ε ∈ {0.0, 0.1, 0.3, 0.5, 0.8, 1.0} で helper 分布エントロピー、平均 helper 距離、remote helper 混入率を計測 |
| T-O2 | 距離-品質トレードオフ相図 | 距離と品質を独立に変化させ、selected helper の特性分布を観測 |

### テスト設計方針

- 全テストは固定シード `StdRng::seed_from_u64(12345)` で再現可能
- `compute_helper_weights` は純粋関数として実装し、関数パラメータとして位置・信頼・レピュテーションを受け取る
- `select_helpers` は `AdultCandidate` のリストを受け取り、`HelperWeight` のリストを返す純粋インターフェース

## 計装方法・観測対象

### 計装方法

- `src/childsupport.rs` の `mod tests` 内に観測テストを実装
- 全観測テストは `println!` + `--nocapture` で構造化テキスト出力
- 固定シード PRNG（`StdRng::seed_from_u64(12345)`）で完全再現性を保証

### 観測対象

**T-O1: β-ε 2次元グリッド掃引**:
- 独立変数: β（距離減衰係数）, ε（exploration 率）
- 従属変数: helper 分布エントロピー（多様性指標）、平均 helper 距離、remote helper 混入率
- サンプルサイズ: β×ε の各 grid point につき 1,000 回のシミュレーション
- 期待される現象:
  - β 大 → 近距離 helper 集中（低エントロピー）
  - ε 大 → remote helper 増加（高エントロピー）
  - 適度な β, ε でエントロピーと距離のバランスが取れた sweet spot が存在

**T-O2: 距離-品質トレードオフ**:
- helper の距離と quality (trust × reputation) を独立に変化
- selected helper の分布が距離と品質のバランスを反映することを観測

### 較正計画

本チケットでは定数定義のみを行い、較正ループは M1.75-11 で実施。ただし、以下の定数を Calibration Candidate として定義する：

| 定数 | デフォルト | 推奨範囲 | 影響 |
|------|-----------|---------|------|
| `HELPER_WEIGHT_DISTANCE_DECAY_BETA` | 1.0 | 0.1–5.0 | 距離減衰の強さ |
| `HELPER_WEIGHT_TRUST_EXPONENT` | 1.0 | 0.5–2.0 | 信頼の重み指数 |
| `HELPER_WEIGHT_REPUTATION_EXPONENT` | 1.0 | 0.5–2.0 | レピュテーションの重み指数 |
| `HELPER_WEIGHT_EXPLORATION_EPSILON` | 0.1 | 0.0–1.0 | exploration 率 |
| `HELPER_WEIGHT_DEFAULT_TOP_K` | 10 | 1–50 | 選抜上限 |

## Boy Scout Rule — 翻訳可能性計画

- `compute_helper_weights` 関数は「距離減衰重みを計算する」という1つの責務に特化させる
- `mix_with_remote_exploration` 関数は「局所重みと遠隔探索を混合する」に特化
- `select_helpers` 関数は「フィルタ→重み計算→混合→TOP-K 選抜」という処理の流れが上から下に文章として読めるよう構成
- 全てのマジックナンバーは `constants.rs` の名前付き定数に集約
- `HelperWeight` の `weight` フィールドは f64 型で表現、総和 1.0 の不変条件を持つ
- `expect()` や `unwrap()` は使用せず、`Result` 伝播または純粋関数の戻り値でエラーを表現

## Acceptance Criteria

- [ ] 実装要件を満たしている
- [ ] 翻訳可能性の検証が通っている
- [ ] 既存テストが通過している

## Notes

<!--
注: このコメントは人間向けの説明である。AI は以下の手順に従うこと。

- plan_path: /plan-ticket が plan.md を作成後に frontmatter に更新する
- implementation_path: /start-ticket が implementation.md を作成後に frontmatter に更新する
- review_report_path: /review-ticket が review.md を作成後に frontmatter に更新する
- observation_report_path: /start-ticket が observation-YYYYMMDD-HHmmss.md を作成後に frontmatter に最新パスを更新する

各コマンドのワークフロー手順が frontmatter 更新の正しい手順である。
-->

### 成果物

- 計画: context/0077-m175-6-helper-weightingbounded-remote-exploration-helper/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0077-m175-6-helper-weightingbounded-remote-exploration-helper/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0077-m175-6-helper-weightingbounded-remote-exploration-helper/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0077-m175-6-helper-weightingbounded-remote-exploration-helper/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
