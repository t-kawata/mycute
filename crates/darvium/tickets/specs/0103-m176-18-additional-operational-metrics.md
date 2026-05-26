---
ticket_id: 103
title: M1.76-18: 運用メトリクス観測パイプライン（Additional operational metrics）
slug: m176-18-additional-operational-metrics
status: reviewed
created_at: 2026-05-26
updated_at: 2026-05-26
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0103-m176-18-additional-operational-metrics/observation-20260526-123144.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0103-m176-18-additional-operational-metrics/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0103-m176-18-additional-operational-metrics/review.md
---

# M1.76-18: 運用メトリクス観測パイプライン（Additional operational metrics）

## Summary

M1.76-17 の合成村シミュレーターが出力する `SimulationTickSnapshot` から、RFC §41B.20.7 で定義された 11 の運用メトリクス（Additional operational metrics）を計算・観測するパイプラインを実装する。具体的には、`benevolent_survival_advantage`, `harmful_gc_rate`, `helper_accept_rate`, `help_abandon_rate`, `child_survival_rate` の 5 関数の純粋計算器と、シミュレーターの TickObserver に統合可能な `ReciprocityMetricsObserver` を実装する。既存の 6 つのパーセンタイル値（p50/p95）は M1.76-17 の `SimulationTickSnapshot` から既に出力されているため、本チケットではそのまま転用する。

本チケットは **RFC §41C.3 の全フェーズにまたがる横断的観測基盤**であり、M1.76-19 の較正フェーズにおける目的関数 J(θ) の成分として利用される。

## Background

RFC §41B.20.7 では、v2.3-e §41B.15 の基本メトリクスに加えて、reciprocity-awareness が導入する 11 の追加運用メトリクスの監視を要求している。これらのメトリクスは、「優しい世界（Kind World）」が実際に創発しているかどうかを多角的に評価するための観測指標である。

M1.76-17 で実装した `SimulationTickSnapshot` は 8 種類のメトリクス（benevolence_score_p50/p95, direct_reciprocity_p50/p95, indirect_reciprocity_p50/p95, reputation_final_p50/p95, survival_advantage, harmful_gc_rate）を既に含むが、以下の 3 指標が未実装である：
1. `helper_accept_rate` — ヘルプ申し出のうち受け入れられた割合
2. `help_abandon_rate` — 実行中のヘルプセッションのうち放棄された割合
3. `child_survival_rate` — 子ワークフローの生存率

また、既存の `harmful_gc_rate` と `survival_advantage` は `SimulationTickSnapshot` 内で簡易計算されているが、RFC が要求する純粋関数として切り出されていない。本チケットではこれらの関数を独立した純粋計算器として定義し、M1.76-16 の `ReciprocityCalibrationHarness.evaluate()` が直接参照可能な形に整備する。

M1.76-16 の観察レポートでは「ReciprocityCalibrationHarness の evaluate メソッドは現状、外部から metrics を受け取る設計」と指摘されており、本チケットで metrics 生成の純粋関数を提供することで、M1.76-19 での J(θ) 自動パイプライン構築の基盤を完成させる。

## Scope

### 実装するもの

1. **`ExtendedOperationalMetrics` 構造体**: RFC §41B.20.7 の 11 指標を保持する新規構造体。既存の `ReciprocityOperationalMetrics`（F-16 の 6 成分）は変更せず、新規構造体として `operational_metrics.rs` または `simulation.rs` に定義する。フィールド:
   - `benevolence_score_p50/p95` — 善良性スコアのパーセンタイル（SimulationTickSnapshot から転記）
   - `direct_reciprocity_p50/p95` — 直接互恵性のパーセンタイル（同上）
   - `indirect_reciprocity_p50/p95` — 間接互恵性のパーセンタイル（同上）
   - `reputation_final_p50/p95` — 最終評判スコアのパーセンタイル（同上）
   - `benevolent_survival_advantage` — 善良群（上位 20%）と非善良群（下位 20%）の生存率差
   - `harmful_gc_rate` — HarmfulMismatch 起因の GC 率
   - `helper_accept_rate` — Accepted / (Accepted + Rejected) 率
   - `help_abandon_rate` — Abandoned / 全終了セッション 率
   - `child_survival_rate` — 子ワークフロー生存率

2. **`compute_benevolent_survival_advantage(population: &[SimWorkflowState]) -> f64`**: 初期善良スコア上位 20% と下位 20% の生存率差。RFC 仕様の「上位/下位 20% 分割」で計算。既存 `SimulationTickSnapshot` の中央値分割版とは異なる。

3. **`compute_harmful_gc_rate(events: &[ReciprocityEvent], population: &[SimWorkflowState]) -> f64`**: HarmfulMismatch イベント数 / 全ヘルプセッション数。既存の `harmful_gc_rate`（全死亡数/全人口）とは計算方法が異なる。

4. **`compute_helper_accept_rate(sessions: &[SimHelpSession]) -> f64`**: 全ヘルプセッションのうち Accepted 状態に遷移したものの割合。分母は (Accepted + Rejected)。

5. **`compute_help_abandon_rate(sessions: &[SimHelpSession]) -> f64`**: 全完了セッションのうち Abandoned で終了したものの割合。分母は (Succeeded + HarmfulMismatch + Abandoned)。

6. **`compute_child_survival_rate(population: &[SimWorkflowState]) -> f64`**: 子ワークフロー（`is_child == true`）のうち生存しているものの割合。子ワークフローが 0 の場合は 0.0 を返す。

7. **`ReciprocityMetricsObserver` 構造体**: M1.76-17 の TickObserver に統合可能な observer。各 tick の `SimulationTickSnapshot`, `SimHelpSession`, `SimWorkflowState` を受け取り、上記 5 関数を呼び出して `ExtendedOperationalMetrics` を生成する。

8. **全 11 指標の時系列 CSV 出力器**: シミュレーション終了後に拡張 CSV 形式で全 metrics を標準出力に書き出す。

### 実装しないもの

- M1.76-16 の `ReciprocityCalibrationHarness` 統合（M1.76-19 で実施）
- 実際のパラメータ sweep や較正（M1.76-19 で実施）
- 既存 `ReciprocityOperationalMetrics` 構造体のフィールド削除
- `HelpSession`（help.rs の本物型）の改変 — シミュレーション用 `SimHelpSession` のみを対象とする
- 永続化（SQLite/LadybugDB）への書き出し

## Investigation

### 物理的証拠（ソースコード解析）

#### 既存 SimulationTickSnapshot（simulation.rs:144-172）

```rust
pub struct SimulationTickSnapshot {
    pub tick: u64,
    pub benevolence_score_p50: f32,
    pub benevolence_score_p95: f32,
    pub direct_reciprocity_p50: f32,
    pub direct_reciprocity_p95: f32,
    pub indirect_reciprocity_p50: f32,
    pub indirect_reciprocity_p95: f32,
    pub reputation_final_p50: f32,
    pub reputation_final_p95: f32,
    pub benevolent_survival_rate: f32,
    pub non_benevolent_survival_rate: f32,
    pub survival_advantage: f32,      // 中央値分割版
    pub harmful_gc_rate: f32,         // 全死亡数/全人口
}
```

既存の 8 指標（benevolence_score_p50/p95, direct_reciprocity_p50/p95, indirect_reciprocity_p50/p95, reputation_final_p50/p95）はそのまま利用可能。欠落しているのは helper_accept_rate, help_abandon_rate, child_survival_rate の 3 指標。

#### 既存 SimHelpSession 型（simulation.rs:124-141）

```rust
pub struct SimHelpSession { pub status: HelpSessionStatus, ... }
pub enum HelpSessionStatus {
    Offered, Accepted, Rejected, Executing, Succeeded, Abandoned, HarmfulMismatch,
}
```

`HelpSessionStatus::Accepted` と `HelpSessionStatus::Abandoned` のカウントで accept rate / abandon rate が計算可能。`Offered` は未応答、`Executing` は進行中として分母から除外。

#### 既存 SimWorkflowState 型（simulation.rs:65-90）

```rust
pub struct SimWorkflowState { pub is_child: bool, pub survived: bool, pub initial_benevolence: f32, ... }
```

`is_child` で子 WF 識別、`survived` で生存判定、`initial_benevolence` で善良/非善良 20% 分割が可能。

#### 既存 ReciprocityOperationalMetrics（calibration.rs:690-717）

6 成分のみ（auc_benevolent_survival, help_success_rate, village_churn_p95, false_new_rate, review_load, instability_penalty）。本チケットではこれを変更せず、新規 `ExtendedOperationalMetrics` を定義する。

#### 既存観測テスト（simulation.rs:706-982）

9 テスト（T1-T9）がシミュレーターの動作を検証済み。本チケットでは新規テストを追加する。

### 参照観察レポート

- `tickets/context/0102-m176-17-phase-3-synthetic-ecosystem-simulation/observation-20260526-121203.md` — シミュレーターの初期 tick で Kind World 創発シグナル確認。CSV 出力形式は既に整備済み。本チケットのメトリクス計算はこのシミュレーター出力を入力とする。
- `tickets/context/0101-m176-16-f-16/observation-20260526-115247.md` — `ReciprocityCalibrationHarness.evaluate()` が外部から metrics を受け取る設計であることを確認。本チケットで提供する純粋関数群はこのハーネスへの入力生成基盤となる。
- `tickets/context/0100-m176-15-should-property-based-test/observation-20260526-113421.md` — 全不変条件 violations=0。数値的安全性は確立済み。

## Test Plan

### T1: benevolent_survival_advantage が全ワークフロー同一 benevolence のとき 0 になること

`compute_benevolent_survival_advantage` に全ワークフローの `initial_benevolence` が同一値の population を入力。上位 20% と下位 20% で生存率差が存在しないため、結果が 0.0 であることを確認。

### T2: harmful_gc_rate が harmful event 0 件のとき 0 になること

`compute_harmful_gc_rate` に `HarmfulMismatch` が 1 件もない `SimHelpSession` リストを入力。全死亡が自然 GC のみの場合でも harmful GC rate は 0 であることを確認。

### T3: helper_accept_rate が全 accept のとき 1.0、全 reject のとき 0.0 となること

- 全セッション Accepted → 1.0
- 全セッション Rejected → 0.0
- 空スライス → 0.0（panic しない）
- 3/10 が Accepted → 0.3

### T4: help_abandon_rate の境界値検証

- 全セッション Succeeded → 0.0
- 全セッション Abandoned → 1.0
- 空スライス → 0.0
- 2/8 が Abandoned → 0.25

### T5: child_survival_rate が 0 child のとき 0.0 になること

- Adult のみ → 0.0
- 全 child 生存 → 1.0
- 全 child 死亡 → 0.0
- 3/10 生存 → 0.3

### T6: 空データに対する graceful ハンドリング

全 5 関数に空スライスを入力し、すべて `0.0` を返し、NaN や Inf が出ないことを確認。

### T7: ReciprocityMetricsObserver 統合テスト

`ReciprocityMetricsObserver` が `SimulationTickSnapshot` + `SimHelpSession` + `SimWorkflowState` を入力として全 11 指標を含む `ExtendedOperationalMetrics` を生成できること。

### T8: 上位/下位 20% 分割の正しさ

- 10 件の population で上位 2 件・下位 2 件が正しく選択されること
- 4 件未満の population でも panic せず 0.0 を返すこと

### T9: CSV 出力の完全性

拡張 CSV 出力が全 11 指標を含む 12 列（tick + 11 metrics）であることを確認。

### T10: 既存テストとの後方互換性

`cargo test` で既存 1053+ テストが全て PASS すること。

## 計装方法・観測対象

### 計装方法

- 全 5 関数の純粋計算器は `src/simulation.rs` の公開関数として実装（またはファイルサイズに応じて `src/operational_metrics.rs` に分離）
- 全関数は `#[cfg(not(test))]` なしで常時コンパイル可能
- `ReciprocityMetricsObserver` は `simulation.rs` に実装
- `println!` + `--nocapture` で拡張 CSV を標準出力に書き出す
- 固定シード PRNG（`StdRng::seed_from_u64(12345)`）は M1.76-17 と共通

### 観測対象

- **新規計算指標（時系列）**: helper_accept_rate, help_abandon_rate, child_survival_rate
- **再計算指標（20% 分割版）**: benevolent_survival_advantage, harmful_gc_rate
- **パーセンタイル指標（既存転記）**: benevolence_score_p50/p95, direct_reciprocity_p50/p95, indirect_reciprocity_p50/p95, reputation_final_p50/p95
- **観測期待**: 全指標が [0, 1] 範囲、空データで panic しない

### 較正計画

- 本チケットでは較正パラメータの sweep は実施しない（M1.76-19 で実施）
- 終了条件: 11 指標がすべて観測可能かつ空データに対して graceful
- 上位/下位 20% の閾値は `constants.rs` に Safety Invariant として定義

## Boy Scout Rule — 翻訳可能性計画

本チケットで実装する全関数に対して以下を徹底する：

- **関数名は動詞句**: `compute_benevolent_survival_advantage`, `compute_helper_accept_rate`, `compute_child_survival_rate` 等
- **変数名はドメイン概念**: `help_sessions`, `population`, `workflow_states` — `data`, `items`, `list` を避ける
- **一関数一責務**: 5 関数はそれぞれ 1 指標のみ計算。複数指標の同時計算は行わない
- **ハードコード値禁止**: 上位/下位 20% 閾値は `constants.rs` の名前付き定数として定義
- **エラー握りつぶし禁止**: `unwrap()` 不使用、空スライスは `0.0` early return
- **コメントの日本語化**: 公開関数の rustdoc は日本語

既存コードへの影響：
- `simulation.rs` の `observe_tick` は変更しない（拡張 metrics は別途計算）
- `calibration.rs` の `ReciprocityOperationalMetrics` は変更しない
- 既存 `SimulationTickSnapshot` のフィールドは削除しない

## Acceptance Criteria

- [ ] `compute_benevolent_survival_advantage` が RFC 仕様の上位/下位 20% 分割で正しく計算される
- [ ] `compute_helper_accept_rate` が全セッションの Accepted 率を正しく計算する
- [ ] `compute_help_abandon_rate` が全セッションの Abandoned 率を正しく計算する
- [ ] `compute_child_survival_rate` が子ワークフローの生存率を正しく計算する
- [ ] 全 5 関数が空データに対して panic せず `0.0` を返す
- [ ] `ReciprocityMetricsObserver` が全 11 指標を計算可能
- [ ] 全 11 指標の CSV 出力が正しい形式で生成される
- [ ] 既存シミュレーターとの統合テストが PASS する
- [ ] 既存テスト（1053+）が全て PASS する
- [ ] 翻訳可能性の検証が通っている

## Notes

- plan_path: context/0103-m176-18-additional-operational-metrics/plan.md（未作成、/plan-ticket 承認後に作成）
- implementation_path: context/0103-m176-18-additional-operational-metrics/implementation.md（未作成、/start-ticket 実装完了後に作成）
- review_report_path: context/0103-m176-18-additional-operational-metrics/review.md（未作成、/review-ticket 全チェック通過後に作成）
- observation_report_path: context/0103-m176-18-additional-operational-metrics/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）

### 成果物

- 計画: context/0103-m176-18-additional-operational-metrics/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0103-m176-18-additional-operational-metrics/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0103-m176-18-additional-operational-metrics/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0103-m176-18-additional-operational-metrics/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
