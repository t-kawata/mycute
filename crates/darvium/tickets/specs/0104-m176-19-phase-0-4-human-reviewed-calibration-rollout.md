---
ticket_id: 104
title: M1.76-19: 較正フェーズ (Phase 0-4) 実装＋human-reviewed calibration rollout
slug: m176-19-phase-0-4-human-reviewed-calibration-rollout
status: reviewed
created_at: 2026-05-26
updated_at: 2026-05-26
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0104-m176-19-phase-0-4-human-reviewed-calibration-rollout/plan.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0104-m176-19-phase-0-4-human-reviewed-calibration-rollout/observation-20260526-140153.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0104-m176-19-phase-0-4-human-reviewed-calibration-rollout/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0104-m176-19-phase-0-4-human-reviewed-calibration-rollout/review.md
---

# M1.76-19: 較正フェーズ (Phase 0-4) 実装＋human-reviewed calibration rollout

## Summary

M1.76-3〜M1.76-10 の純粋関数（F-1〜F-15）、M1.76-13 の決定論的リプレイ機構、M1.76-14 の摂動テストスイート、M1.76-16 の多目的較正目的関数 F-16 + 較正ハーネス、M1.76-17 の合成村シミュレーター、M1.76-18 の拡張運用メトリクス観測パイプラインを統合し、RFC §15.10.9 で定義された 5 段階の較正フェーズ（Phase 0: PureFunctionValidation → Phase 1: DeterministicReplay → Phase 2: SmallPerturbation → Phase 3: SyntheticEcosystem → Phase 4: HumanReviewed）を実装する。

最終的な係数更新は **human-reviewed** でなければならず（MUST NOT auto-update to production）、rollout は canary → production の 2 段階で進行する。Event Architecture の較正候補（`EVENTBUS_DEFAULT_TIMEOUT_MS` 等）も本較正ループの対象に含む。本チケットは RFC §41C.3 の **M4.x（human-reviewed calibration rollout）** の中核 + 全 Phase 統合に対応する。

## Background

RFC §15.10.9 は 5 段階の較正フェーズ（Phase 0-4）を定義し、係数更新の安全性を段階的に検証するプロトコルを要求している。M1.76-16 で ReciprocityCalibrationHarness の基盤が構築され、M1.76-17 で合成村シミュレーターが完成し、M1.76-18 で拡張運用メトリクス観測パイプラインが整備された。これらの成果物を統合し、自動化された較正パイプラインとして完成させる必要がある。

**過去の観測レポートからの知見**:
- M1.76-16 の観察レポートでは「ReciprocityCalibrationHarness.evaluate() は外部から metrics を受け取る設計」と指摘されており、本チケットでシミュレーターと統合する
- M1.76-17 の観察レポートでは「lambda_gc_base=1.0 が高すぎて全滅が早い」と指摘されており、本チケットの Phase 3 sweep で lambda_gc_base の低減（0.3〜0.5）と gamma_benevolence の増強（0.3〜0.5）を検証する
- M1.76-18 の観察レポートでは「ExtendedOperationalMetrics + ReciprocityMetricsObserver のシミュレーター統合は完了しているが、デフォルトパラメータでは拡張指標が全て 0 となる」と報告されており、本チケットの Phase 3 で適切なパラメータ発見が必要
- 既存カウンターパート 1063 テストは全 PASS 状態

## Scope

### 実装するもの

1. **`CalibrationPhase` 列挙型**: 5 段階のフェーズを表現
   - `Phase0(PureFunctionValidation)` — 純粋関数の出力値域・単調性・非負性検証
   - `Phase1(DeterministicReplay)` — 同一 event stream + policy version での再現性検証
   - `Phase2(SmallPerturbation)` — 摂動に対する安定性 regression 検証
   - `Phase3(SyntheticEcosystem)` — 合成村シミュレーターによる創発的性質の観測
   - `Phase4(HumanReviewed)` — 候補係数セット生成 → 差分レポート生成 → human review queue 配送

2. **`Phase0Runner`**: 以下の検証を一括実行
   - M1.76-3〜M1.76-10 の全純粋関数（F-1〜F-15）の出力値域チェック（全出力が NaN/Inf でないこと）
   - 単調性検証（パラメータ増加に対する出力の単調非減少性）
   - 非負性検証（全出力が 0.0 以上であること）
   - 境界値検証（ゼロ入力・最大入力での安定性）

3. **`Phase1Runner`**: 以下の検証を一括実行
   - M1.76-13 のリプレイ機構を使用し、同一 event stream + policy version での結果のビットレベル一致を検証
   - 複数シード（12345, 67890, 11111, 22222, 99999 の 5 シード）でのリプレイ再現性
   - policy version 変更時の結果不一致を検出可能なこと（リグレッション検出の実証）

4. **`Phase2Runner`**: 以下の検証を一括実行
   - M1.76-14 の perturbation suite を実行
   - 各摂動軸（embedding noise, trust delta, usage increment）での J(θ) 変動が許容範囲内であることを検証
   - チャーン率 P95 増加が `PERTURB_CHURN_MAX_P95_INCREASE` 以下であること
   - Jensen-Shannon Divergence が `PERTURB_JSD_MAX_P95_INCREASE` 以下であること

5. **`Phase3Runner`**: 以下の検証を一括実行
   - M1.76-17 の合成村シミュレーターを seed 固定で実行
   - M1.76-16 の ReciprocityCalibrationHarness にシミュレーション結果（ReciprocityMetricsObserver の出力 → ReciprocityOperationalMetrics 変換）を自動投入
   - パラメータ sweep（OFAT または grid）による目的関数 J(θ) の最大化
   - sweep 結果から最適パラメータセットを特定
   - Kind World 創発の有無を survival_advantage 系列で観測

6. **`Phase4Runner`**: 以下の処理を実行
   - Phase 0-3 の全検証通過を前提条件としてアサート（全ガード）
   - 候補係数セット（ReciprocityCalibrationResult の上位 N 件）を生成
   - 現行 production 係数との差分レポート（CalibrationRolloutReport）を生成
   - 差分レポートを human review queue（HumanReviewQueue）へ配送
   - human review 承認後に policy version 更新を提案（自動適用はしない）
   - canary environment policy 分離（環境別 policy version の管理）

7. **`CalibrationRolloutReport` 構造体**:
   - `candidate_coefficients: Vec<HashMap<String, f64>>` — 候補係数セット一覧
   - `evaluation_results: Vec<ReciprocityCalibrationResult>` — 各候補の評価結果
   - `diff_from_production: HashMap<String, (f64, f64)>` — 現行値からの差分（係数名 → (旧値, 新値)）
   - `human_review_ticket: Option<String>` — 配送先 human review チケット ID
   - `policy_version_update: Option<String>` — 提案 policy version

8. **Human review queue 連携**: Phase 4 の差分レポートを `HumanReviewQueue`（M1-1）または `FakeHumanChannel` 経由で配送可能にする adapter

9. **Canary environment policy 分離**: 環境タグ（`"canary"`, `"production"`）による policy version 管理。canary での検証通過後に production へ昇格する 2 段階 rollout

10. **Event Architecture 較正候補の統合**: `constants.rs` の Event Architecture 定数（`EVENTBUS_DEFAULT_TIMEOUT_MS`, `EVENTBUS_CHANNEL_RECONNECT_BASE_DELAY_MS` 等）の sweep を Phase 3 で実施可能にする拡張

### 実装しないもの

- 新しい純粋関数（F-1〜F-15 の実装は M1.76-3〜M1.76-10 で完了済み）
- 新しいシミュレーションロジック（M1.76-17 で完了済み）
- 新しい拡張メトリクス（M1.76-18 で完了済み）
- 実際の production 係数更新（Phase 4 は human review queue 配送まで）
- canary 環境の実際のデプロイ基盤（policy version 管理は本チケットだが、デプロイ機構は別）
- 実験レポート生成と系列管理の統合（M1.76-20 で実施）

## Investigation

### 物理的証拠（ソースコード解析）

#### Constants.rs の較正候補定数（較正対象の全定数）

較正候補となる ReciprocityLifecyclePolicy 関連定数（constants.rs 内の命名と値）

| 定数名 | 現行値 | 分類 | 属する関数 |
|--------|--------|------|------------|
| LIFECYCLE_WEIGHT_BENEVOLENCE | 0.15 | Calibration Candidate | F-4, F-7, F-8 |
| GC_HAZARD_LAMBDA_0 | 1.0 | Calibration Candidate | F-7 |
| GC_HAZARD_GAMMA_LIFECYCLE | 0.5 | Calibration Candidate | F-7 |
| GC_HAZARD_GAMMA_BENEVOLENCE | 0.10 | Calibration Candidate | F-7, F-8 |
| GC_HAZARD_GAMMA_CHILD_PROTECT | 0.20 | Calibration Candidate | F-10 |
| CHILD_PROTECT_ETA1 | 0.50 | Calibration Candidate | F-10 |
| CHILD_PROTECT_ETA2 | 0.30 | Calibration Candidate | F-10 |
| CHILD_PROTECT_ETA3 | 0.20 | Calibration Candidate | F-10 |
| HELP_WEIGHT_BENEVOLENCE | 0.20 | Calibration Candidate | F-11 |
| HELP_SOFTMAX_TAU | 1.0 | Calibration Candidate | F-12 |
| REMOTE_EXPLORATION_BASE | 0.05 | Calibration Candidate | F-13 |
| REMOTE_EXPLORATION_MAX | 0.20 | Calibration Candidate | F-13 |
| CHILD_GROWTH_MU_MISSION_SUCCESS | 0.60 | Calibration Candidate | F-14 |
| CHILD_GROWTH_MU_HELP_SUCCESS | 0.40 | Calibration Candidate | F-14 |
| MATURATION_NU_BIAS | -2.0 | Calibration Candidate | F-15 |
| REPUTATION_THETA_DIR | 0.35 | Calibration Candidate | F-4 |
| REPUTATION_THETA_IND | 0.35 | Calibration Candidate | F-4 |
| REPUTATION_THETA_EXP | 0.20 | Calibration Candidate | F-4 |
| REPUTATION_THETA_INHERIT | 0.10 | Calibration Candidate | F-4 |
| F16_LAMBDA_AUC | 0.30 | Calibration Candidate | F-16 |
| F16_LAMBDA_HELP_SUCCESS | 0.25 | Calibration Candidate | F-16 |
| F16_LAMBDA_CHURN | 0.15 | Calibration Candidate | F-16 |
| F16_LAMBDA_FALSE_NEW | 0.10 | Calibration Candidate | F-16 |
| F16_LAMBDA_REVIEW_LOAD | 0.10 | Calibration Candidate | F-16 |
| F16_LAMBDA_INSTABILITY | 0.10 | Calibration Candidate | F-16 |

Event Architecture 較正候補:

| 定数名 | 現行値 | 分類 |
|--------|--------|------|
| EVENTBUS_DEFAULT_TIMEOUT_MS | 5000 | Calibration Candidate |
| EVENTBUS_CHANNEL_RECONNECT_BASE_DELAY_MS | 1000 | Calibration Candidate |
| EVENTBUS_CHANNEL_RECONNECT_MAX_DELAY_MS | 30000 | Calibration Candidate |
| EVENTBUS_PROJECTION_ERROR_BACKOFF_MS | 5000 | Calibration Candidate |

#### 既存 ReciprocityCalibrationHarness（calibration.rs:852-915）

`ReciprocityCalibrationHarness` は現在、外部から `ReciprocityOperationalMetrics` を受け取る設計。本チケットでは M1.76-17 の `run_simulation` と統合し、シミュレーション実行 → メトリクス計算 → 目的関数評価を自動化する。

```rust
pub fn evaluate(
    &self,
    params: &HashMap<String, f64>,
    metrics: &ReciprocityOperationalMetrics,
) -> ReciprocityCalibrationResult
```

#### 既存 run_simulation（simulation.rs:895）

`ReciprocitySimulatorConfig` を受け取り、固定 seed でシミュレーションを実行する。

```rust
pub fn run_simulation(config: &ReciprocitySimulatorConfig) -> ReciprocitySimulationResult
```

#### 既存 HumanReviewQueue（human_review_queue.rs:56-70）

`QueuedReview` のキューイングが可能な隔離レビューキュー。本チケットの Phase 4 で差分レポートをキューイングする。

#### 既存観測テスト

M1.76-16（較正ハーネス）の 11 テスト、M1.76-17（シミュレーター）の 9 テスト、M1.76-18（拡張メトリクス）の 10 テストが既に実装済み。全テストが再現性とロジック正確性を検証している。

### 各 Phase の依存関係（直列性）

Phase 0-3 の全検証通過が Phase 4 の候補係数生成の前提条件であることは、RFC の要求事項である。各 Phase は完全に直列であり、前段階の PASS なしに次段階に進めない。

```text
Phase0 → Phase1 → Phase2 → Phase3 → Phase4 → human review → canary rollout → production rollout
```

### ソースコード調査で判明した設計上の注意点

#### 1. 較正目的関数の二重構造（F-16 vs J_kw）

現行の `ReciprocityCalibrationHarness::evaluate()` は `ReciprocityOperationalMetrics`（6 成分、`compute_calibration_objective` / F-16）を入力として J(θ) を計算する。RFC §15.10.9 Phase 3 は J_kw (Kind World 目的関数、§15.9.2) を目的関数として定義しているが、J_kw の実装は M1.76-KW1 に依存する。本チケットの Phase 3 では **F-16 を一次目的関数として実装し、J_kw 対応は将来の拡張ポイントとして設計する**。

ただし、Phase 3 の sweep パラメータは M1.76-KW1 で定義される MagnificentSevenParams（gamma_benevolence, lambda_gc_base, direct_reciprocity_weight, indirect_reciprocity_weight, softmax_temperature, gc_interval, child_ratio）と一致させる。これにより、KW チケット完了後もシームレスに移行できる。

#### 2. Simulation → Metrics 変換パスの不在

`run_simulation()` は `ReciprocitySimulationResult`（metric_series + final_state）を返す。一方、`ReciprocityCalibrationHarness::evaluate()` は `ReciprocityOperationalMetrics`（6 成分）を受け取る。両者を変換する関数が存在しない。

```
// 現在: 手動変換が必要
let result = run_simulation(&config);
let metrics = simulation_result_to_operational_metrics(&result);
harness.evaluate(&params, &metrics);
```

本チケットで `simulation_result_to_operational_metrics()` 関数を実装する。変換ロジック: `ReciprocityMetricsObserver` の出力 (`ExtendedOperationalMetrics`) から AUC/benevolent_survival・help_success_rate・village_churn_p95・false_new_rate・review_load・instability_penalty の 6 成分を計算。ただしシミュレーション内では false_new_rate と review_load はシミュレーション前提に適した近似値を使用する（exact 値は本番パイプラインのみ）。

#### 3. Phase 0 はユニットテスト再実行ではなく純粋関数直接呼び出し

既存のユニットテスト（M1.76-3〜M1.76-10 の mod tests）は個別ファイルのテストとして既に存在する。Phase 0 ではこれらを `cargo test` 経由で再実行するのではなく、各純粋関数に合成入力を直接与えて出力値域・単調性・非負性を検証するラッパー関数を実装する。これにより、テスト環境と較正パイプラインが独立する。

#### 4. Phase 2 の摂動テストは M1.76-14 の関数を直接呼び出し

既存の `VillageCalibrationHarness`（M1.75-11）は village 用の sweep 機構であり、M1.76-14 の摂動テストスイートとは異なる。Phase 2 では `src/calibration.rs` に新規の perturbation validation 関数を実装し、embedding noise・trust delta・usage increment の各摂動軸に対して churn P95 / JSD の変動を検証する（既存の VillageCalibrationHarness は使用しない）。

#### 5. Phase 3 の Config 型は ReciprocitySimulatorConfig

`VillageCalibrationConfig`（M1.75-11）は village パラメータ（beta, epsilon, top_k 等）を対象としており、Phase 3 で sweep する Reciprocity パラメータ（gamma_benevolence, lambda_gc_base 等）とは目的が異なる。Phase 3 独自の sweep 設定として、`ReciprocitySimulatorConfig` の該当フィールドを `ParameterRange` 経由で変化させるラッパーを実装する。

#### 6. Event Architecture 較正候補の sweep は擬似的対応

`EVENTBUS_DEFAULT_TIMEOUT_MS` 等の Event Architecture 定数はランタイム定数であり、シミュレーター実行に直接影響しない。Phase 3 ではこれらの sweep を `ParameterRange` のリストに含めることのみ可能とし、評価関数はダミー値を返すスタブとする（実際の影響評価は M1.76-22 の Event Architecture 運用メトリクス統合で実施）。

#### 7. Phase ゲート機構

Phase 0-3 の全検証通過を Phase 4 の前提条件とするため、各 Phase の PASS/FAIL 状態を保持する `PhaseGate` 構造体を実装する。各 Phase Runner は完了時に `PhaseGate::record_pass(phase)` を呼び出し、Phase 4 は `PhaseGate::assert_all_preceding_passed(phase_4)` でガードする。

### 参照観察レポート

- `tickets/context/0101-m176-16-f-16/observation-20260526-115247.md` — 較正ハーネス基盤完了。J(θ) の決定論的再現性確認。`evaluate()` が外部 metrics 依存であることを確認。
- `tickets/context/0102-m176-17-phase-3-synthetic-ecosystem-simulation/observation-20260526-121203.md` — 合成村シミュレーター完了。Kind World 創発の初期シグナル確認。lambda_gc_base 低減の必要性指摘。
- `tickets/context/0103-m176-18-additional-operational-metrics/observation-20260526-123144.md` — 拡張メトリクス観測基盤完了。デフォルトパラメータでは拡張指標が全て 0 になる問題確認。`ExtendedOperationalMetrics` と `ReciprocityOperationalMetrics` の間の変換パスが未実装であることも確認。

## Test Plan

### Phase 0 テスト

**T1: Phase0Runner が全純粋関数の出力値域検証をパスすること**

`Phase0Runner::run()` を実行し、M1.76-3〜M1.76-10 の各関数について以下の検証が PASS することを確認:
- `compute_direct_reciprocity` の出力が [0, 1] 範囲
- `compute_indirect_reciprocity` の出力が [0, 1] 範囲
- `compute_benevolence_score` の出力が [0, 1] 範囲
- `recompute_reputation` の成分が全て [0, 1] 範囲
- `compute_gc_hazard` の出力が [0, ∞) 範囲
- `compute_survival_probability` の出力が [0, 1] 範囲
- 全出力が NaN/Inf でないこと

**T2: Phase0Runner が単調性検証をパスすること**

各パラメータを段階的に増加させたとき、対応する出力が単調非減少であることを検証:
- lambda_gc_base 増加 → gc_hazard 非減少
- gamma_benevolence 増加 → benevolent_survival_probability 非減少
- child_protect_eta1 増加 → child_death_probability 非減少

**T3: Phase0Runner が非負性検証をパスすること**

全純粋関数にゼロ入力を与えた場合の出力が全て 0.0 以上であることを確認。

**T4: Phase0Runner が境界値検証をパスすること**

極端な入力（f64::MAX, f64::MIN_POSITIVE, 0.0）に対して全関数が panic せず有限値を返すことを確認。

### Phase 1 テスト

**T5: Phase1Runner が同一 seed でビットレベル一致を確認すること**

固定 seed（12345）で 3 回実行し、全結果が同一であることを検証。

**T6: Phase1Runner が複数シードでの再現性を確認すること**

5 シード（12345, 67890, 11111, 22222, 99999）それぞれで 2 回実行し、各シード内での一致を確認。シード間で結果が異なることは許容（異なるシミュレーション経路のため）。

**T7: Phase1Runner が policy version 変更時の結果不一致を検出できること**

異なる policy version を指定して実行し、結果が異なる（regression 検出）ことを確認。

### Phase 2 テスト

**T8: Phase2Runner が embedding noise 摂動に対して安定性 bounds 内であることを確認すること**

`PERTURB_EMBEDDING_NOISE_SIGMA_DEFAULT ± 50%` の範囲で JSD が `PERTURB_JSD_MAX_P95_INCREASE` 以下であることを確認。

**T9: Phase2Runner が trust delta 摂動に対して安定性 bounds 内であることを確認すること**

trust delta ± 0.05 の摂動で churn P95 増加が `PERTURB_CHURN_MAX_P95_INCREASE` 以下であることを確認。

**T10: Phase2Runner が usage increment 摂動に対して安定性 bounds 内であることを確認すること**

`PERTURB_USAGE_INCREMENT_DEFAULT ± 5` の範囲で ranking 変動が許容範囲内であることを確認。

### Phase 3 テスト

**T11: Phase3Runner が seed 固定で再現可能な simulation 結果を出力すること**

同一 seed（12345）で 2 回 `Phase3Runner::run_sweep()` を実行し、全 sweep 結果がビットレベル一致することを確認。

**T12: Phase3Runner が OFAT sweep で各パラメータの影響を観測できること**

OFAT（One-Factor-At-a-Time）sweep で各較正候補パラメータを `SWEEP_OFAT_DEFAULT_STEPS` 段階で変化させ、J(θ) の変化を記録可能であることを確認。

**T13: Phase3Runner が lambda_gc_base 低減による生存率向上を観測できること**

lambda_gc_base を 1.0 → 0.5 → 0.3 と低減したとき、survival_advantage が改善する（より正の値になる）ことを観測。レポートに CSV 出力を含める。

**T14: Phase3Runner が gamma_benevolence 増強による Kind World 創発促進を観測できること**

gamma_benevolence を 0.1 → 0.3 → 0.5 と増強したとき、benevolent_survival_advantage と child_survival_rate が改善することを観測。

**T15: Phase3Runner の sweep 結果から最適パラメータセットが特定できること**

sweep 結果の中から J(θ) 最大のパラメータセットを自動選択できること。選択結果を CalibrationRolloutReport の candidate_coefficients に含められること。

### Phase 4 テスト

**T16: Phase4Runner が Phase 0-3 の全 PASS を前提条件としてアサートすること**

Phase 0-3 のいずれかが未実行/FAIL の場合、Phase 4 がエラーを返すこと。

**T17: Phase4Runner が差分レポートを生成できること**

現行 constants.rs の値との差分（diff_from_production）を含む `CalibrationRolloutReport` を生成できること。

**T18: Phase4Runner の差分レポートが human review queue へ配送可能であること**

`HumanReviewQueue` または `FakeHumanChannel` を使用して、`CalibrationRolloutReport` を review queue に投入できること。

**T19: auto-update が production へ即時反映されないこと（MUST NOT 確認）**

Phase 4 実行後も constants.rs の値が変更されていないことを確認。

### 統合テスト

**T20: 全 Phase を直列実行する統合テストが 1 サイクル完走できること**

Phase 0 → 1 → 2 → 3 → 4 を連続実行し、各 Phase の PASS/FAIL ステータスと実行時間を出力できること。

**T21: CalibrationRolloutReport の canary/production policy version 分離**

report に canary environment tag が含まれ、production とは異なる policy version が提案されることを確認。

**T22: Event Architecture 較正候補の sweep が実施可能であること**

`EVENTBUS_DEFAULT_TIMEOUT_MS` 等の Event Architecture 定数を sweep パラメータに含められることを確認。

**T23: 既存全テストとの後方互換性**

`cargo test` で既存 1063 テストが全て PASS すること。

## 計装方法・観測対象

### 計装方法

- `CalibrationPhase` 列挙型、各 Phase Runner、`CalibrationRolloutReport` は `src/calibration.rs` に実装（ファイルサイズに応じて `src/calibration_rollout.rs` に分離も検討）
- 各 Phase の通過/不通過ステータス、実行時間、検出された異常件数を構造化テキスト（JSON）で `println!` + `--nocapture` 経由で標準出力に書き出す
- Phase 3 の sweep 結果は CSV 形式で標準出力に書き出す
- 固定シード PRNG（`StdRng::seed_from_u64(12345)`）を使用
- 各 Phase Runner は `run_phase0()`, `run_phase1()` 等の個別関数 + `run_all_phases()` 統合関数の形で実装
- Phase 4 の human review queue 連携は `FakeHumanChannel` を使用（実際のキュー配送はシミュレーション）
- canary/production policy version 管理は `HashMap<String, String>`（環境名 → policy version）で簡易実装

### 観測対象

- **Phase 0**: 全純粋関数の出力値域（[min, max]）、単調性検定の成否、異常値検出件数
- **Phase 1**: 同一 seed 内の最大誤差（ビットレベル）、シード間の JSD、policy version 不一致検出の有無
- **Phase 2**: 各摂動軸における churn P95 増加量・JSD 値、bounds 超過有無
- **Phase 3**: sweep パラメータ × J(θ) の応答曲面、最適パラメータ位置、survival_advantage 系列、child_survival_rate 系列
- **Phase 4**: 候補係数セット数、現行値との差分ベクトル、human review ticket 生成成功/失敗、配送レイテンシ

### 較正計画

- **調整する定数**: 上記「較正候補定数」一覧の全 25 定数（Reciprocity 22 + Event Architecture 3）。Phase 3 の sweep は特に MagnificentSevenParams（M1.76-KW1 定義の 7 パラメータ: gamma_benevolence, lambda_gc_base, direct_reciprocity_weight, indirect_reciprocity_weight, softmax_temperature, gc_interval, child_ratio）に焦点を当てる
- **目的関数**: 既存 F-16（`compute_calibration_objective` / `calibration.rs:773`）を一次目的関数として流用。J_kw 対応は KW チケット完了後に拡張。デフォルト重みは F16_LAMBDA_*（現行 constants.rs 値）を使用
- **sweep 方法**: まず OFAT（One-Factor-At-a-Time）で各パラメータの影響度を観測し、影響の大きいパラメータに対して grid sweep または LHS（Latin Hypercube Sampling）を実施
- **停止条件**: Phase 3 の sweep で J(θ) の改善が 1% 未満になった時点、または SWEEP_OFAT_DEFAULT_STEPS（=5）を全パラメータで完了した時点

## Boy Scout Rule — 翻訳可能性計画

### 新規実装

- **関数名は動詞句**: `run_phase0`, `run_phase1`, `generate_rollout_report`, `deliver_to_review_queue` 等
- **変数名はドメイン概念**: `calibration_phase`, `candidate_coefficients`, `diff_report`, `review_ticket` — `data`, `items`, `list` を避ける
- **一関数一責務**: 各 Phase Runner は 1 つの Phase のみ実行。Phase 間の依存関係は呼び出し側が管理
- **ハードコード値禁止**: sweep step 数・grid 分割数は `constants.rs` の `SWEEP_OFAT_DEFAULT_STEPS`, `SWEEP_GRID_DEFAULT_DIVISIONS`, `SWEEP_LHS_DEFAULT_SAMPLES` を使用
- **エラー握りつぶし禁止**: `unwrap()` 不使用、結果は `Result<CalibrationRolloutReport, DarviumError>` で伝播
- **コメントの日本語化**: 公開関数の rustdoc は日本語、実行ログは英語

### 既存コード改善（Boy Scout Rule）

本チケットで触る既存コードに対して以下を改善する：

- `calibration.rs` の `ReciprocityCalibrationHarness` 関連コード: 既存コメントが英語のままの箇所は日本語に修正。`//` コメントではなく `///` rustdoc が使用されていない箇所があれば修正
- `calibration.rs` の `ReciprocityCalibrationResult` と `ReciprocityCalibrationReport`: 既存の rustdoc は日本語化済みだが、新規フィールド追加時に一貫性を維持
- `simulation.rs` の `run_simulation`: Phase 3 の sweep 呼び出しに対応するため、`ReciprocitySimulatorConfig` から `ReciprocityOperationalMetrics` への変換パスを最小差分で整備

### 翻訳可能性の検証

実装完了後に以下の観点で自己レビューする：
- 新規関数の呼び出しチェーンが日本語の文章として逐語訳可能か
- 型名・フィールド名の一貫性（同一概念に同一名称が使われているか）
- 既存の命名規則との矛盾がないか

## Acceptance Criteria

- [ ] `CalibrationPhase` 列挙型が 5 フェーズ（Phase0-PureFunctionValidation, Phase1-DeterministicReplay, Phase2-SmallPerturbation, Phase3-SyntheticEcosystem, Phase4-HumanReviewed）を正しく定義している
- [ ] `Phase0Runner` が全純粋関数（F-1〜F-15）の出力値域・単調性・非負性を検証できる
- [ ] `Phase1Runner` が同一 seed でのビットレベル再現性と複数シードでの再現性を検証できる
- [ ] `Phase2Runner` が摂動に対する安定性 bounds 超過を検出できる
- [ ] `Phase3Runner` が seed 固定で再現可能な simulation sweep を実行し、最適パラメータを特定できる
- [ ] `Phase4Runner` が Phase 0-3 の全 PASS を前提条件としてアサートする
- [ ] `CalibrationRolloutReport` が候補係数セット・評価結果・production 差分を含む
- [ ] Phase 4 の差分レポートが human review queue へ配送可能である
- [ ] auto-update が production へ即時反映されないこと（MUST NOT 確認）
- [ ] canary/production の 2 段階 policy version 管理が実装されている
- [ ] Event Architecture 較正候補の sweep が Phase 3 で実施可能
- [ ] 全 Phase 直列実行が 1 サイクル完走する
- [ ] 既存テスト（1063+）が全て PASS する
- [ ] 翻訳可能性の検証が通っている

## Notes

- plan_path: context/0104-m176-19-phase-0-4-human-reviewed-calibration-rollout/plan.md（未作成、/plan-ticket 承認後に作成）
- implementation_path: context/0104-m176-19-phase-0-4-human-reviewed-calibration-rollout/implementation.md（未作成、/start-ticket 実装完了後に作成）
- review_report_path: context/0104-m176-19-phase-0-4-human-reviewed-calibration-rollout/review.md（未作成、/review-ticket 全チェック通過後に作成）
- observation_report_path: context/0104-m176-19-phase-0-4-human-reviewed-calibration-rollout/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）

### 成果物

- 計画: context/0104-m176-19-phase-0-4-human-reviewed-calibration-rollout/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0104-m176-19-phase-0-4-human-reviewed-calibration-rollout/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0104-m176-19-phase-0-4-human-reviewed-calibration-rollout/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0104-m176-19-phase-0-4-human-reviewed-calibration-rollout/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
