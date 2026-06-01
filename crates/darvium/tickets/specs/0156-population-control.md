---
ticket_id: 156
title: 人口安定化のための動的淘汰圧制御（Population Control）
slug: population-control
status: reviewed
created_at: 2026-06-01
updated_at: 2026-06-01
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0156-population-control/plan.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0156-population-control/implementation.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0156-population-control/observation-20260601-164229.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0156-population-control/review.md
---
# 人口安定化のための動的淘汰圧制御（Population Control）

## Summary

シミュレーション実行中に人口（生存個体数）が目標値を超えた場合に淘汰圧（GC Hazard）を自動的に上昇させ、目標値を下回った場合に圧を戻す動的制御機構を実装する。これにより、人口爆発を人工的なキャップではなくGCメカニズムの自然な延長として抑制する。

既存の `Arc<RwLock<SimulationParams>>` パイプライン（フロントエンド→WebSocket→サーバー→シミュレーションループ）に `target_population` フィールドを追加し、フロントエンドのスライダーでリアルタイム調整可能にする。

## Background

**人口爆発の観測**: #146（出生意味論の実装）以降、HELP/GMR/自己抽象化の3経路で新個体が正しく登録されるようになり、出生が死亡を上回る人口爆発が観測可能となった（[[birth-semantics-confirmed]]）。

**現在の課題**: GCメカニズム自体は存在するが、淘汰圧は静的パラメータ（`lambda_gc_base=1.0`, `gamma_child_protect=8.0` 等）で固定されており、人口が増えても自動的に圧が上がらない。以下の選択肢がある：

1. **人工的な人口上限（ハードキャップ）** — 生態系として不自然。Kind World 創発の検証に影響を与える可能性。
2. **静的なパラメータ調整** — GC パラメータを全体的に強くすると初期の人口形成が阻害される。
3. **動的淘汰圧制御（本チケット）** — 人口に応じてGC Hazard のベースラインを動的に変化させる。最も生態系的に自然。

**既存パイプライン**: すでに `Arc<RwLock<SimulationParams>>`（`movement_distance`, `chief_attraction_strength`, `min_approach_distance`）をフロントエンドからリアルタイム変更できる基盤が整っている（`simulation.rs:1078-1086`, `server.rs:52-53`）。`SimCommand::UpdateParam` ハンドラ（`server.rs:106-118`）と `syncSettingsToBackend`（`script.js:668`）が既存の配線パターンである。

## Scope

1. **`ReciprocitySimulatorConfig` に人口制御設定フィールドを追加**（`simulation.rs`）
   - `target_population: Option<usize>` — 目標人口（None=無効）
   - `target_hysteresis: f64` — ヒステリシス幅（デフォルト0.05=5%）
   - `pressure_lambda_high: f32` — 高圧時の `lambda_gc_base`
   - `pressure_lambda_low: f32` — 低圧時の `lambda_gc_base`
   - `pressure_gamma_child_low: f32` — 高圧時の `gamma_child_protect`
   - `pressure_gamma_child_high: f32` — 低圧時の `gamma_child_protect`

2. **`compute_adjusted_policy` 関数を実装**（`simulation.rs`）
   - 生存人口 vs 目標人口の比較
   - ヒステリシス帯による発振防止
   - 調整済み `ReciprocityLifecyclePolicy` を返す

3. **`SimulationParams` に `target_population` フィールドを追加**（`simulation.rs`, `#[cfg(feature = "server")]`）
   - フロントエンドからリアルタイム変更可能

4. **`SimCommand` に `UpdateTargetPop` variant を追加**（`server.rs`）

5. **フロントエンドに target_population スライダーを追加**（`index.html`, `script.js`）
   - range スライダー（0=無効〜500）
   - `syncSettingsToBackend` に同期処理を追加

6. **全シミュレーションループの `phase4_gc_survival` 呼び出し前に圧力調整を挿入**
   - KW-REAL ループ（4箇所）
   - `run_simulation` 内の `run_lifecycle_gc`（1箇所）
   - `run_evaluation_simulation_with_channel`（1箇所）

## Non-scope

- `ReciprocityLifecyclePolicy` 自体の変更（F-7 式の改変は行わない）
- Resource Pressure モード自動判定（RFC §15.8 Normal/Constrained/Emergency）の実装
- 比例制御やPID制御の導入（今回は閾値＋ヒステリシスの単純制御）
- 出生率への直接介入（あくまで淘汰圧のみを操作して間接的に制御）
- 本番コード（非シミュレーション）のGC動作変更

## Investigation

### 物理的証拠

**証拠1: GC Hazard 計算式（`src/reciprocity.rs:305-316`）**
```rust
pub fn compute_gc_hazard(
    lifecycle_score: f32,
    benevolence_score: f32,
    child_protection: f32,
    policy: &ReciprocityLifecyclePolicy,
) -> f32 {
    let inner = policy.lambda_gc_base
        - policy.gamma_lifecycle * lifecycle_score
        - policy.gamma_benevolence * benevolence_score
        - policy.gamma_child_protect * child_protection;
    softplus(inner)
}
```
→ `lambda_gc_base` と `gamma_child_protect` は `ReciprocityLifecyclePolicy` 経由で外部から変更可能。

**証拠2: 生存判定（`src/simulation.rs:3324-3327`）**
```rust
let survival_prob = compute_survival_probability(hazard as f32, 1);
if ctx.rng.random::<f64>() >= survival_prob {
    ctx.population[id].alive = false;
    gc_events += 1;
}
```
→ 確率判定に基づく淘汰。hazard が上がれば生存確率が下がり死亡数が増加。

**証拠3: 既存動的パラメータパイプライン（`simulation.rs:1078-1086`, `server.rs:52-53`）**
```rust
pub struct SimulationParams {
    pub movement_distance: f64,
    pub chief_attraction_strength: f64,
    pub min_approach_distance: f64,
}
```
→ `Arc<RwLock<SimulationParams>>` として共有。フロントエンドからリアルタイム変更可能。

**証拠4: 既存 WebSocket ハンドラ（`server.rs:106-118`）**
```javascript
Some("update_param") => {
    if let Some(md) = config.get("movement_distance").and_then(|v| v.as_f64()) {
        let _ = cmd_tx.send(SimCommand::UpdateParam(md));
    }
    // ... chief_attraction_strength, min_approach_distance も同様
}
```
→ 1フィールドにつき「SimCommand variant追加 → ハンドラ追加 → フロントエンドスライダー」のパターンで配線可能。

**証拠5: シミュレーションループの構造（`simulation.rs:1921-1927`）**
```rust
let gc_events = if tick % config.gc_interval == 0 {
    phase4_gc_survival(&mut ctx, &config.policy, &alive_ids)
} else {
    0
};
```
→ 現在は `&config.policy` を直接渡している。ここで調整済みポリシーに差し替える。

### 参照観察レポート

- tickets/context/0155-skip-child-search/observation-20260601-155605.md — skip_child_search 後のグラフ成長の観測
- tickets/context/0154-untitled-6/observation-20260601-150339.md — 各種パラメータ較正の状態
- ファイル記憶: [[birth-semantics-confirmed]] — #146 出生意味論完了、人口爆発が観測可能に

## Test Plan

### ユニットテスト（`simulation.rs` 内 `mod tests`）

| # | テスト名 | 内容 | 種別 |
|---|---------|------|------|
| TC1 | `test_adjusted_policy_no_target` | `target_population=None` で元のポリシーがそのまま返る | 正常系 |
| TC2 | `test_adjusted_policy_below_target` | 生存数 < 目標で圧力 LOW（デフォルト値が維持される） | 正常系 |
| TC3 | `test_adjusted_policy_above_target` | 生存数 >= 目標+ヒステリシスで圧力 HIGH（λ₀上昇, γ_C低下） | 正常系 |
| TC4 | `test_adjusted_policy_hysteresis_band` | ヒステリシス帯内では前回状態が維持される | 境界値 |
| TC5 | `test_adjusted_policy_hysteresis_edge` | ちょうど閾値での挙動確認 | 境界値 |
| TC6 | `test_adjusted_policy_zero_hysteresis` | `target_hysteresis=0` でヒステリシスなしの動作 | 境界値 |

### 観測テスト（既存のシミュレーションテストに統合）

既存の `test_kw_real_simulation` 等に `target_population` を設定したケースを追加：

| # | テスト名 | 内容 |
|---|---------|------|
| OT1 | `test_population_stabilization` | 目標人口100でシミュレーション実行し、定常状態での人口変動幅が目標±20%以内に収まることを観測 |
| OT2 | `test_population_control_off` | `target_population=None` で従来通りの動作（非回帰） |

### 依存関係

- `rand::Rng`（生存判定に使用）
- `StdRng::seed_from_u64(12345)`（固定シードで再現性保証）
- `SimulationContext`（`alive_ids()` メソッドで生存者数を取得）
- `ReciprocityLifecyclePolicy`（Clone可能であること）

## 計装方法・観測対象

### 計装方法

- `println!` で各 tick の `(tick, alive_count, target, pressure_state, lambda_gc_base, gamma_child_protect)` を CSV 形式で出力
- `--nocapture` で観測可能
- 固定シード `StdRng::seed_from_u64(12345)` で完全再現性

### 観測対象

| 統計量 | 取得方法 | 期待値 |
|--------|---------|--------|
| 定常人口の平均値 | 後半50tickの平均 | 目標人口 ± ヒステリシス帯内 |
| 定常人口の標準偏差 | 後半50tickの標準偏差 | 目標の 10% 未満（安定） |
| 圧力状態の遷移回数 | 状態変化カウンタ | 発振時のみ大。安定時は 2 回以下 |
| λ₀ の実効値 | 各 tick の設定値 | HIGH時 3.0、LOW時 1.0 |
| GC イベント数/tick | `gc_events` | 人口過剰時に増加確認 |

### 較正計画

- `target_hysteresis`: 0.05（5%）を初期値。発振が観測された場合は増加。
- `pressure_lambda_high`: 3.0 を初期値。人口抑制が弱ければ増加。
- `pressure_gamma_child_low`: 2.0 を初期値。子供の死亡率が高すぎる場合は調整。

## Boy Scout Rule — 翻訳可能性計画

### 新規実装対象（翻訳可能性を確保）

- `compute_adjusted_policy` → 関数名は「調整済みポリシーを計算する」と読める
- 内部変数名: `target`, `hysteresis`, `threshold_high`, `threshold_low`, `adjusted` — すべてドメイン概念を表現
- 一関数一責務: 圧力計算とポリシー生成のみ。観測出力は別関数。

### 既存コードの改善（範囲内）

現在の `phase4_gc_survival`（`simulation.rs:3299-3332`）は改修時に関数内部の child_prot 算出部分に以下の改善を行う：
- `PHASE4_CHILD_PROT_VALUE` と `PHASE4_CHILD_PROT_ADULT` の役割を既存コメントに補足
- 子供判定の `is_child` 計算がインラインなため、必要なら `is_child_workflow()` ヘルパー抽出を検討

## Acceptance Criteria

- [ ] `compute_adjusted_policy` が `target_population=None` で元のポリシーを変更せず返す
- [ ] `target_population=Some(N)` で生存数 >= N+hysteresis のとき λ₀ が上昇し γ_C が低下する
- [ ] ヒステリシス帯内で圧力状態が維持される（発振防止）
- [ ] フロントエンドのスライダー変更が WebSocket 経由でシミュレーションに反映される
- [ ] 既存テストがすべて通過（cargo test: 1389 passed, 0 failed）
- [ ] 観測テストで人口が目標値付近で安定することを確認

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

- 計画: context/0156-population-control/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0156-population-control/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0156-population-control/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0156-population-control/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
