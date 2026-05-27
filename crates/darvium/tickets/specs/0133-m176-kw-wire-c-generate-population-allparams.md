---
ticket_id: 133
title: M1.76-KW-WIRE-C: 生成時定数のパラメーター化 — generate_population のハードコード値を AllParams 結合
slug: m176-kw-wire-c-generate-population-allparams
status: reviewed
created_at: 2026-05-28
updated_at: 2026-05-28
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0133-m176-kw-wire-c-generate-population-allparams/plan.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0133-m176-kw-wire-c-generate-population-allparams/implementation.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0133-m176-kw-wire-c-generate-population-allparams/observation-20260528-073548.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0133-m176-kw-wire-c-generate-population-allparams/review.md
---
# M1.76-KW-WIRE-C: 生成時定数のパラメーター化 — generate_population のハードコード値を AllParams 結合

## Summary

`generate_population()`（simulation.rs:445-505）にハードコードされた 3 つの値（子供 trust 上限 0.3、成人 trust 下限 0.3、benevolent 分類閾値 0.5）を constants.rs の定数化 + AllParams G4 グループとしてパラメーター化する。同時に `ReciprocitySimulatorConfig` を 3 フィールド拡張し、`to_sim_config` 経路で G4 値がシミュレーション初期化に反映されるようにする。これにより初期信頼分布と慈悲分類閾値が較正ループから制御可能になる。

## Background

RFC §41C はシミュレーションの全初期条件を較正対象として露出することを要求している。しかし `generate_population()` は子供ノードの初期 trust を `[0, 0.3]`、成人ノードの初期 trust を `[0.3, 0.8]` の一様分布に固定している。これらの値は constants.rs にも AllParams にも経路を持たず、較正不能である。人口の初期信頼分布は GC hazard・HELP プロトコル・村形成の基盤であり、s_growth/s_topology の初期条件として無視できない影響を持つ。また `observe_tick()` の benevolent 分類閾値 0.5 は s_fairness の j_penalty 計算を左右するが、同様に固定値である。

4A.0 カタログ再検査（2026-05-27）ではこれら 3 値が未収録のハードコード値として確認された。カタログ方針（「主要 6 フェーズ + メトリクス計算パス」の範囲外）により追加保留となっていたが、WIRE 系列の一環として本チケットで対処する。

## Scope

1. constants.rs に 3 定数を追加
2. kind_world.rs に AllParams G4 グループを追加（G4_CHILD_TRUST_MAX, G4_ADULT_TRUST_MIN, G4_BENEVOLENT_THRESHOLD）
3. simulation.rs の `generate_population()` と `observe_tick()` を定数参照に変更
4. simulation.rs の `ReciprocitySimulatorConfig` を 3 フィールド拡張
5. kind_world.rs に `to_sim_config_g1g2g4()` を追加（G1+G2+G4 の全値を config に伝播）
6. テスト C1-C6 の追加

## Non-scope

- 他のハードコード値（offer_help_probability の 0.3/0.4、advance_help_sessions の 6 値）→ WIRE-A で対処
- REMOTE_EXPLORE_* 4 定数のシミュレーション実装 → WIRE-D で対処
- 残余ハードコード値の全数パラメーター化 → WIRE-E で対処
- AllParams G3 グループ（WIRE-A 用、8 パラメーター）の定義

## Investigation

### 発見 1: generate_population の子供 trust 上限（simulation.rs:467）

```rust
// simulation.rs:467（generate_population 関数内）
trust: rng.random::<f32>() * 0.3,
```

子供ノードの初期 trust が `[0, 0.3]` の一様分布に固定されている。`0.3` はマジックナンバーであり、constants.rs にも AllParams にも対応する定数が存在しない。

### 発見 2: generate_population の成人 trust 下限（simulation.rs:492）

```rust
// simulation.rs:492（generate_population 関数内）
trust: rng.random::<f32>() * 0.5 + 0.3,
```

成人ノードの初期 trust が `[0.3, 0.8]` の一様分布に固定されている。`0.3` は下限値であり、`0.5` は幅である。本チケットでは下限値 `0.3` をパラメーター化する（幅 `0.5` は構造的性質のため固定維持）。

### 発見 3: observe_tick の benevolent 分類閾値（simulation.rs:864-880）

```rust
// simulation.rs:864-880（observe_tick 関数内）
.filter(|w| w.initial_benevolence > 0.5)   // 865, 875 行
.filter(|w| w.initial_benevolence <= 0.5)  // 869, 879 行
```

benevolent 分類閾値 `0.5` が 4 箇所にハードコードされている。この閾値は benevolent_survival_rate、non_benevolent_survival_rate、survival_advantage の全計算に影響する。

### 発見 4: ReciprocitySimulatorConfig の現行フィールド（simulation.rs:54-69）

```rust
pub struct ReciprocitySimulatorConfig {
    pub population_size: usize,
    pub child_ratio: f64,
    pub mission_rate: f64,
    pub max_ticks: u64,
    pub gc_interval: u64,
    pub policy: ReciprocityLifecyclePolicy,
    pub seed: u64,
}
```

7 フィールド。本チケットで 3 フィールド追加が必要：`child_trust_max: f64`、`adult_trust_min: f64`、`benevolent_threshold: f64`。

### 発見 5: AllParams G4 グループの位置づけ（kind_world.rs:316-355）

AllParams の現行グループ構成：

| グループ | パラメーター数 | インデックス範囲 | 内容 |
|---------|---------------|----------------|------|
| G1 | 14 | 0-13 | 検索・探索系 |
| G2 | 3 | 14-16 | GC・生存系 |
| G3（WIRE-A） | 8 | 17-24 | offer/advance 確率（未実装） |
| G4（本チケット） | 3 | 25-27 | 生成時定数 |

G4 の各定数インデックス：
- `G4_CHILD_TRUST_MAX = G1_COUNT + G2_COUNT + G3_COUNT + 0 = 25`
- `G4_ADULT_TRUST_MIN = G1_COUNT + G2_COUNT + G3_COUNT + 1 = 26`
- `G4_BENEVOLENT_THRESHOLD = G1_COUNT + G2_COUNT + G3_COUNT + 2 = 27`

注: G3_COUNT は WIRE-A で 8 と定義される予定。本チケットでは将来の G3 追加を考慮して `G3_COUNT = 8` を先に定義しておく（または WIRE-A 完了後に G4 インデックスを調整）。いずれにせよ G4 は G1+G2+G3 の直後に配置される。

### 発見 6: to_sim_config の拡張パターン（kind_world.rs:463-507）

既存の `to_sim_config_g1()` および `to_sim_config_g1g2()` が G1/G2 値を ReciprocitySimulatorConfig に反映するパターンが確立されている。本チケットでは `to_sim_config_g1g2g4()` を追加し、G1+G2+G4 の全値が config に反映される経路を構築する。

```rust
pub fn to_sim_config_g1g2g4(&self, seed: u64) -> ReciprocitySimulatorConfig {
    let mut config = self.to_sim_config_g1g2(seed);
    config.child_trust_max = self.values[G4_CHILD_TRUST_MAX];
    config.adult_trust_min = self.values[G4_ADULT_TRUST_MIN];
    config.benevolent_threshold = self.values[G4_BENEVOLENT_THRESHOLD];
    config
}
```

### 発見 7: constants.rs の空き領域（constants.rs:1074-1101）

KW4 関連定数（`KW4_SIMULATION_TICKS` 等）は constants.rs:1074-1101 に集約されている。WIRE-C の 3 定数はこのブロックの直後（`// ============================================================================` 区切り内）に追加するのが適切。

### 参照観察レポート

- 本チケットは WIRE 系列（B→C→A→D→E）の 2 番目。WIRE-B（#132）の観察レポートにて「simulation 内で HELP セッションが発生しないため compute_search_radius_inverse が 0.5 固定」という制約が確認されている。WIRE-C の定数化はこの制約の解決に直接関与しないが、初期条件の制御可能性を高める基盤となる。

## Test Plan

### C1: SIMULATION_CHILD_TRUST_MAX = 0.0 で子供 trust が全員 0.0
`generate_population()` の子供ノード全員の `trust` フィールドが 0.0 であることを検証する。

### C2: SIMULATION_ADULT_TRUST_MIN = 1.0 で成人 trust が全員 1.0
`generate_population()` の成人ノード全員の `trust` フィールドが 1.0（上限 clamp）であることを検証する。

### C3: SIMULATION_BENEVOLENT_THRESHOLD = 1.0 で全員非 benevolent
`observe_tick()` の結果、`total_benevolent = 0` になることを検証する。

### C4: SIMULATION_BENEVOLENT_THRESHOLD = 0.0 で全員 benevolent
`observe_tick()` の結果、`total_non_benevolent = 0` になることを検証する。

### C5: 定数変更後も決定論的再現性維持
同一 seed で 2 回 `generate_population()` を呼び出し、全ノードの全フィールドがビットレベルで一致することを検証する。

### C6: 既存テスト全 PASS
修正後も既存のテストスイートが全て通過すること。

## 計装方法・観測対象

### 計装方法
- simulation.rs の `mod tests` に C1-C5 のユニットテストを追加
- C1-C5 は固定値の assert テスト（観測不要、PASS/FAIL のみ）
- C6 は `cargo test` で全テスト通過を確認
- 観測テストとして、生成された population の trust 分布（子供/成人別ヒストグラム）を println! + --nocapture で出力

### 観測対象
- 生成された population の trust 分布（子供/成人別 5 数要約）
- benevolent 分類比率の閾値変更に対する感度
- 初期信頼分布と GC hazard の相関

### 較正計画
本チケットは定数化（パラメーター経路の確立）が目的であり、較正は後続の WIRE 全チケット完了後に行う。本チケット単独では較正しない。

## Boy Scout Rule — 翻訳可能性計画

1. **generate_population の 0.3/0.5 マジックナンバー**: 名前付き定数に置き換え、コードの自己文書化を促進する。
2. **observe_tick の 4 箇所の 0.5 比較**: 名前付き定数に置き換え、閾値の意味を明示する。
3. **ReciprocitySimulatorConfig の Default impl**: 新フィールド追加時にデフォルト値を既存の constants.rs 値と一致させる。

## Acceptance Criteria

- [ ] C1: SIMULATION_CHILD_TRUST_MAX = 0.0 で子供 trust が全員 0.0
- [ ] C2: SIMULATION_ADULT_TRUST_MIN = 1.0 で成人 trust が全員 1.0
- [ ] C3: SIMULATION_BENEVOLENT_THRESHOLD = 1.0 で全員非 benevolent
- [ ] C4: SIMULATION_BENEVOLENT_THRESHOLD = 0.0 で全員 benevolent
- [ ] C5: 定数変更後も決定論的再現性維持
- [ ] C6: 既存テスト全 PASS
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

- 計画: context/0133-m176-kw-wire-c-generate-population-allparams/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0133-m176-kw-wire-c-generate-population-allparams/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0133-m176-kw-wire-c-generate-population-allparams/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0133-m176-kw-wire-c-generate-population-allparams/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
