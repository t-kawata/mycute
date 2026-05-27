---
ticket_id: 135
title: M1.76-KW-WIRE-D: REMOTE_EXPLORE_* 定数のシミュレーション実装 — 遠隔探索機構の導入（実装順序: 4 番目）
slug: m176-kw-wire-d-remote-explore-4
status: reviewed
created_at: 2026-05-28
updated_at: 2026-05-28
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0135-m176-kw-wire-d-remote-explore-4/plan.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0135-m176-kw-wire-d-remote-explore-4/observation-20260528-082905.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0135-m176-kw-wire-d-remote-explore-4/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0135-m176-kw-wire-d-remote-explore-4/review.md
---
# M1.76-KW-WIRE-D: REMOTE_EXPLORE_* 定数のシミュレーション実装 — 遠隔探索機構の導入（実装順序: 4 番目）

## Summary

`offer_help_sessions()`（simulation.rs:608）に村内/村外の距離を考慮した遠隔探索機構を導入する。WIRE-A で epsilon_remote が offer 確率に組み込まれたが、現在は全ノード一律の確率（村内も村外も同じ式）で offer を送る。本チケットでは `village_assignments` に基づき、村内ノードには通常確率（OFFER_HELP_BASE + epsilon_remote）、村外ノードには epsilon_remote 確率のみ（探索的）で offer するロジックに変更する。同時に `child_need` を村ごとの子供割合から動的に計算する `compute_child_need_in_village()` を実装し、WIRE-A で暫定 0.0 固定だった child_need を実測値化する。

## Background

RFC §4A.5（F-13 benevolence-aware remote exploration）は「局所的な助け合いが基本であり、村外への HELP は低確率で探索的に行われる」と規定する。現在の `offer_help_sessions()` は全 alive ノードを絶対評価し、最も遠いノードにも一定確率でオファーを送る — 村内も村外も一律の確率である。このため s_topology 因子の j_clustering や j_local_density がシミュレーションの HELP パターンと乖離した値を取りうる。

WIRE-A では epsilon_remote の計算式（base, need_coeff, benevolence_coeff）を offer_help_probability() に組み込んだが、child_need は暫定 0.0 固定のまま残された。WIRE-A 観察レポート（observation-20260528-080417.md）も「WIRE-D で child_need の実装が必要。現在 0.0 固定の child_need をエージェント状態から動的に計算する」と示唆している。

WIRE 系列の 4 番目として、本チケットで村構造と HELP 発動確率の結合を実現する。

## Scope

1. **constants.rs に LOCAL_HELP_BOOST 定数を追加**: 村内 offer 確率のブースト係数（デフォルト 1.0、boost なし）。
2. **kind_world.rs に G5 グループ追加**: G5_COUNT=1, G5_LOCAL_HELP_BOOST (28)。default_g1g2g4() に G5 セクション追加（constants.rs の値で初期化、active=true）。to_sim_config_g1g2g4() に G5→sim_config 伝播追加。
3. **simulation.rs: ReciprocitySimulatorConfig に `local_help_boost: f64` フィールド追加**。
4. **simulation.rs: `compute_village_mean_benevolence()` 補助関数追加**: 指定村の平均慈悲スコアを計算（None の場合は全人口平均）。
5. **simulation.rs: `compute_child_need_in_village()` 補助関数追加**: 指定村内の is_child 割合を計算。
6. **simulation.rs: `offer_help_sessions()` に `village_assignments` 引数追加**: `Option<&HashMap<String, Option<usize>>>` として受け取り、None の場合は全ノードを村内扱い（後方互換）。
7. **offer_help_sessions() の offer ロジック変更**: 村内ノード → `base_prob + epsilon_remote`（従来通り）、村外ノード → `(OFFER_HELP_BASE + epsilon_remote) * local_help_boost` かつ epsilon 制限（探索的）。
8. **`run_simulation()` の呼び出し元修正**: `offer_help_sessions()` に None（village_assignments なし）を渡す。
9. **テスト D1-D7 追加**。

## Non-scope

- `phase3_help_protocol()`（simulation.rs:1847）の村内/村外ロジック → こちらは既に ctx.village_assignments を持ち、FIX-C で任意ペアの HELP を生成している。本チケットは `offer_help_sessions()`（簡易シミュレーションパス）が対象。
- G5 の追加探索パラメーター（LOCAL_HELP_BOOST 以外） → WIRE-E で対処。
- 村形成ロジック自体の修正 → 既存の P1 実装をそのまま利用。
- s_topology/j_clustering への直接的な影響評価 → 後続 WIRE-E 完了後の統合較正で実施。

## Investigation

### 発見 1: offer_help_sessions の現在の実装（simulation.rs:608-642）

```rust
fn offer_help_sessions(
    missions: &[SimMission],
    population: &[SimWorkflowState],
    existing_sessions: &mut Vec<SimHelpSession>,
    tick: u64,
    rng: &mut StdRng,
    session_counter: &mut u64,
    config: &ReciprocitySimulatorConfig,
    village_mean_benevolence: f32,     // ← 全人口の平均（村別ではない）
) {
    let policy = &config.policy;
    let child_need: f32 = 0.0; // WIRE-D 未実装のため一時的に 0.0
    for mission in missions {
        for wf in population.iter().filter(|w| w.survived && w.id != mission.requester_id) {
            if rng.random::<f64>() < offer_help_probability(
                wf.benevolence, child_need, village_mean_benevolence, policy, config,
            ) {
                // 全人口一律の確率で offer を生成
                ...
            }
        }
    }
}
```

- `village_mean_benevolence` は全人口の平均（global）で村別ではない
- `child_need` は 0.0 固定
- 村内/村外の区別が全くない
- `village_assignments` を引数に持たない

### 発見 2: VillageAssignment 型と SimulationContext（simulation.rs:286, 305）

```rust
pub type VillageAssignment = Option<usize>;
// None = どの村にも所属していない

pub struct SimulationContext {
    ...
    pub village_assignments: HashMap<NodeId, VillageAssignment>,
}
```

`SimulationContext` は `village_assignments` を保持するが、`offer_help_sessions()` の呼び出し元である `run_simulation()`（simulation.rs:1225）は `SimulationContext` ではなく `ReciprocitySimulatorConfig` を使用する。したがって `run_simulation()` 経路には村情報が存在しない。このため `village_assignments` は optional 引数とし、None の場合は全ノードを村内扱い（従来動作）とする必要がある。

### 発見 3: compute_benevolence_aware_remote_exploration は既に正しい（reciprocity.rs:527-535）

```rust
pub fn compute_benevolence_aware_remote_exploration(
    child_need: f32, local_benevolence_mean: f32, policy: &ReciprocityLifecyclePolicy,
) -> f32 {
    let raw = policy.epsilon_remote_base + policy.epsilon_remote_need_coeff * child_need
        - policy.epsilon_remote_benevolence_coeff * local_benevolence_mean;
    raw.clamp(0.0, policy.epsilon_remote_max)
}
```

関数自体は既に正しい。WIRE-D ではこれを `child_need` 実測値（村内子供割合）で呼び出す必要がある。

### 発見 4: G3/G4 の実装パターン（kind_world.rs:356-382）

```rust
pub const G3_COUNT: usize = 8;
pub const G4_COUNT: usize = 3;

// 現在 G5 は未定義。G3 インデックス = G1_COUNT + G2_COUNT
// G4 インデックス = G1_COUNT + G2_COUNT + G3_COUNT
// G5 インデックス = G1_COUNT + G2_COUNT + G3_COUNT + G4_COUNT
```

G3 および G4 のパターンに従って G5 を追加する。`G5_COUNT = 1`（LOCAL_HELP_BOOST のみ）。

### 発見 5: WIRE-A 観察レポートの示唆

WIRE-A 観察レポートは以下の示唆を残している：
- 「child_need は現在 0.0 固定（WIRE-D 未実装）のため、当面の影響は epsilon_remote_base と benevolence_coeff に限定されるが、WIRE-D 完了後は child_need に応じた動的な発動率変動が期待できる」
- 「AllParams G3 グループの残りパラメーター（REMOTE_EXPLORATION_*）と G3 の整合性を WIRE-D で検証する必要がある」

### 参照観察レポート
- tickets/context/0134-m176-kw-wire-a-offer-help-probability-epsilon-remote-allparams-3/observation-20260528-080417.md — 「WIRE-D では child_need の実装が必要」

## Test Plan

### D1: 村内ノードには通常確率（OFFER_HELP_BASE + epsilon_remote）で offer
村内ペアを作成し、offer_help_sessions が通常確率で offer を生成することを検証。

### D2: 村外ノードには epsilon_remote 確率のみで offer
村外ペアを作成し、村内よりも有意に低い offer 確率になることを検証。

### D3: epsilon_remote_max = 0 で村外への offer が全く発生しない
epsilon_remote_max=0, local_help_boost=1.0 の config で村外ノードへの offer が 0 であることを検証（n >= 1,000 試行）。

### D4: LOCAL_HELP_BOOST を増加すると村内 offer 確率が上がる
LOCAL_HELP_BOOST を 1.0 → 2.0 に変更すると村内 offer 確率が 2 倍になることを検証。

### D5: 村が 1 つも形成されていない場合、全ノードが村内扱い
village_assignments = None（全ノード未所属）の場合に全ノードが通常確率で offer されることを検証。

### D6: compute_child_need_in_village の境界値
子供 0 で 0.0、全員子供で 1.0 を返すことを検証。

### D7: 既存テスト全 PASS
修正後も既存のテストスイートが全て通過すること。

## 計装方法・観測対象

### 計装方法
- simulation.rs の `mod tests` に D1-D6 のユニットテストを追加
- D1-D5 は固定 seed の assert テスト
- D6 は純粋関数の境界値テスト
- D7 は `cargo test` で全テスト通過を確認
- 村内/村外別の offer 発生数と確率を println! + --nocapture で出力

### 観測対象
- 村内/村外別の offer 確率の分布比較（n=10,000 random sampling）
- epsilon_remote の 4 成分値と offer 確率の関係
- child_need（村内子供割合）と offer 確率の相関
- LOCAL_HELP_BOOST の感度分析
- 村間 HELP 比率（VHELP）の基本値

### 較正計画
本チケットは遠隔探索機構の導入（ロジック追加）が目的。パラメーター値（LOCAL_HELP_BOOST, REMOTE_EXPLORATION_*）の較正は WIRE-E 完了後の統合較正で実施。

## Boy Scout Rule — 翻訳可能性計画

1. **offer_help_sessions() の責務分割**: 現在の offer_help_sessions は offer 生成 + 村判定の 2 責務を持つが、村判定は compute_village_mean_benevolence や compute_child_need_in_village に分割することで翻訳可能性を向上
2. **child_need = 0.0 暫定値の解消**: WIRE-A で残された「WIRE-D 未実装のため一時的に 0.0」を実測値に置き換え
3. **関数名の一貫性**: compute_village_mean_benevolence, compute_child_need_in_village は動詞句＋名詞句で翻訳可能

## Acceptance Criteria

- [ ] D1: 村内ノードには通常確率で offer
- [ ] D2: 村外ノードには epsilon_remote 確率のみで offer
- [ ] D3: epsilon_remote_max = 0 で村外への offer が 0
- [ ] D4: LOCAL_HELP_BOOST 変更で村内 offer 確率が変化
- [ ] D5: 村なし状態では全ノードが村内扱い
- [ ] D6: compute_child_need_in_village 境界値正解
- [ ] D7: 既存テスト全 PASS
- [ ] 翻訳可能性の検証が通っている

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

- 計画: context/0135-m176-kw-wire-d-remote-explore-4/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0135-m176-kw-wire-d-remote-explore-4/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0135-m176-kw-wire-d-remote-explore-4/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0135-m176-kw-wire-d-remote-explore-4/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
