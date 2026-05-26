---
ticket_id: 102
title: M1.76-17: 合成村シミュレーター（Phase 3: Synthetic ecosystem simulation）
slug: m176-17-phase-3-synthetic-ecosystem-simulation
status: reviewed
created_at: 2026-05-26
updated_at: 2026-05-26
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0102-m176-17-phase-3-synthetic-ecosystem-simulation/observation-20260526-121203.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0102-m176-17-phase-3-synthetic-ecosystem-simulation/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0102-m176-17-phase-3-synthetic-ecosystem-simulation/review.md
---

# M1.76-17: 合成村シミュレーター（Phase 3: Synthetic ecosystem simulation）

## Summary

Training Plane の safe sandbox scope で、合成 population（child/adult）を走らせて HELP プロトコル（offer → accept/reject → execute → succeed/fail）をシミュレートし、優しい世界（benevolent な workflow が非 benevolent な workflow より生存率が高い状態）が emergent に成立するかを検証するシミュレーターを実装する。シミュレーターは production path を汚染せず、fake execution path に限定する（MUST）。

本チケットの実装により、M1.76-16 で構築した `ReciprocityCalibrationHarness` が受け取るべき `ReciprocityOperationalMetrics` の各成分を実際に計算する実行基盤が完成する。これにより、パラメータ θ から J(θ) までの自動パイプラインが実現する。

## Background

RFC §15.10.9 Phase 3 では、純粋関数層の検証（Phase 0）→ プロパティベース不変条件ファジング（Phase 1-2）を経て、Phase 3 で合成村シミュレーターによる創発的性質の検証を定義している。RFC §41C.3 M3.x はこの Phase 3 に対応し、以下のコンポーネントを要求する：

- Synthetic population generator (child/adult)
- Mission stream generator (一定 rate で mission を生成)
- Help interaction simulator (benevolence-biased HELP protocol)
- Trust/reputation recompute loop (tick ごと)
- Lifecycle/GC loop (tick ごと)
- Tick observer (全 tick の状態記録)

M1.75-8（決定論的リプレイ）の `run_replay_scenario` は既存の `VillageReplayScenario` ベースで village 構成・HELP プロトコル・位置移動をシミュレートするが、benevolence bias が組み込まれておらず、Reciprocity モジュール（M1.76-1〜M1.76-10）の各関数（F-1〜F-15）は純粋関数として実装されているが、それらを tick ごとに連続駆動する合成シミュレーターが存在しない。本チケットはそのギャップを埋める。

M1.76-16 で構築した `ReciprocityCalibrationHarness.evaluate()` は外部から `ReciprocityOperationalMetrics` を受け取る設計であり、metrics の生成元を持たない。本シミュレーターがその生成元となる。

### 優しい世界（Benevolent World）の創発とは

善良な workflow（benevolence 高）が非善良な workflow（benevolence 低）よりも生存率が有意に高い状態を「優しい世界が emergent に成立した」と定義する。具体的には：

- `AUC_benevolent > nonbenevolent > 0.5` かつ信頼区間の下限が 0.5 を超える
- 善良群の平均生存率が非善良群より統計的に有意に高い
- この差がパラメータ sweep の広い範囲で安定的に観測される

## Scope

### 実装するもの

1. **`ReciprocitySimulatorConfig` 構造体**: `{ population_size, child_ratio, mission_rate, max_ticks, policy, seed }`
2. **`SyntheticPopulationGenerator`**: child/adult population を固定シード PRNG で生成。child_ratio に従って child:adult 比を決定。各 workflow に初期位置・経験値・信頼値・評判値を割り当てる
3. **`MissionStreamGenerator`**: 一定 rate で mission（child のニーズ）を生成。mission_rate（0-1）に従って各 tick で生成確率を判定
4. **`HelpInteractionSimulator`**: HELP プロトコルを benevolence-biased でシミュレート。benevolence の高い adult ほど offer を出しやすく、実行成功率も高い。内部で M1.76-4（F-2）の間接互恵性、M1.76-8（F-11/F-12）の helper quality/selection を利用
5. **`TrustReputationRecomputeLoop`**: tick ごとに trust/reputation を再計算。内部で `compute_direct_reciprocity`（F-1）、`compute_indirect_reciprocity`（F-2）、`recompute_reputation`（F-4/F-5）を呼び出す
6. **`LifecycleGcLoop`**: tick ごとに GC hazard を再計算。内部で `compute_gc_hazard`（F-7/F-8）を呼び出し、`compute_survival_probability` で生存判定を実施
7. **`TickObserver`**: 各 tick の状態（profiles, hazards, villages, helper assignments）を記録し、`ReciprocitySimulationResult.metric_series` として時系列データを蓄積
8. **`ReciprocitySimulationResult`**: `{ metric_series, final_state, experiment_id }` — 全 tick の時系列メトリクス・最終状態・実験ID を保持
9. **`run_simulation(config: &ReciprocitySimulatorConfig) -> ReciprocitySimulationResult`**: 上記全コンポーネントを統合するエントリポイント

### 実装しないもの

- 実際のパラメータ sweep（M1.76-19 で実施）
- 運用メトリクス観測パイプライン（M1.76-18 で別チケット）
- 既存 VillageReplayScenario / run_replay_scenario の改変（独立モジュールとして実装）
- 外部ストア（MetadataStore / GraphStore）への永続化 — 純粋なインメモリシミュレーションに限定
- 既存 calibration.rs の ReciprocityCalibrationHarness 統合（連携は M1.76-19 で行う）

## Investigation

### 物理的証拠（ソースコード解析）

#### 既存 Replay 基盤（replay.rs）

- **ファイル**: `src/replay.rs`
- **`VillageReplayScenario`** (32行目): PRNG seed + workflow 初期設定 + mission 仕様 + clock schedule + policy bundle を持つ。本シミュレーターのベースパターン。
- **`run_replay_scenario`** (283行目): 各 tick で位置更新 → village 構成 → helper weight 計算 → mission 処理 → 経験値増加 を実行。本シミュレーターはこれに benevolence bias を追加した独立版として実装する。
- **`PolicyBundle`** (73行目): `offer_policy`, `accept_policy`, `selection_policy` を持つ。本シミュレーターでも使用可能。
- **`ReplayTrace`** (92行目): `space_positions`, `villages`, `helper_weights`, `help_sessions`, `child_growth_events` を持つ。本シミュレーターはこれに加えて reciprocity metrics の時系列を含む拡張を実装する。

#### 既存 Reciprocity 関数群（reciprocity.rs & event.rs）

- **ファイル**: `src/reciprocity.rs`
- `compute_direct_reciprocity(events, now, policy) -> f32` (74行目) — F-1
- `compute_indirect_reciprocity(graph_metrics, policy) -> f32` — F-2
- `logistic_sigmoid(x) -> f32` (45行目) — F-1内で使用
- **ファイル**: `src/event.rs`
- `recompute_reputation(profile, direct, indirect, policy) -> ReputationProfile` — F-4, F-5
- `compute_gc_hazard(memo, policy, now, clock) -> f32` — F-7
- `ReputationProfile` 構造体 — final_score, benevolence, direct_score, indirect_score, experience 等
- `ReciprocityLifecyclePolicy` 構造体 — 全較正パラメータ（theta_dir, gamma_benevolence, lambda_gc_base 等）

#### 既存較正ハーネス（calibration.rs）

- **ファイル**: `src/calibration.rs`
- `SurvivalPair` (679行目): AUC 計算の基本単位（workflow_id, benevolence_score, survived）
- `ReciprocityOperationalMetrics` (690行目): AUC, help_success_rate, churn_p95, false_new_rate, review_load, instability_penalty の 6 成分
- `compute_auc_benevolent_survival(profiles) -> f64` (725行目): Mann-Whitney U 統計量による AUC 計算
- `ReciprocityCalibrationHarness` (852行目): `evaluate(params, metrics)` で J(θ) を計算。metrics の生成元を持たない

#### 既存定数（constants.rs）

- 全 Calibration Candidate 定数が既存。本チケットでは新規定数は最小限に抑える
- `REPLAY_POSITION_DELTA_SIGMA: f64 = 0.1` — 位置更新ランダムウォーク幅（流用可能）
- `CHILD_GROWTH_MU_*` — 成長係数（流用可能）
- シミュレーション制御用の定数（max_ticks 上限等）を追加する可能性あり

#### 既存 Village Metrics（village.rs 等）

- `VillageMetricsSnapshot` — churn_p95, helper_jsd_p95, child_survival_rate 等を保持
- 本シミュレーターでは独自の metrics 時系列を `ReciprocitySimulationResult` として定義する

### 参照観察レポート

- `tickets/context/0101-m176-16-f-16/observation-20260526-115247.md` — AUC 計算・J(θ) 合成の正しさ確認。次チケットへの示唆として「M1.76-17 では ReciprocityOperationalMetrics の各成分を実際に計算する実装が必要」と明記
- `tickets/context/0100-m176-15-should-property-based-test/observation-20260526-113421.md` — 全不変条件 violations=0 を確認。ファジング範囲内での数値的安全性は確立済み
- `tickets/context/0099-m176-14-perturbation-test-suite/observation-20260526-120000.md` — survival_drift, flip_rate, churn_delta の観測。本シミュレーターの metrics 時系列と比較可能

## Test Plan

### T1: 決定論的リプレイ — 同一 seed で 2 回実行し全 tick の状態がビットレベルで一致

- `run_simulation` を同一設定で 2 回呼び出し、`metric_series` の全エントリが一致することを確認
- 全ワークフローの生存状態・位置・スコアが全て一致することを確認
- 固定シード PRNG（`StdRng::seed_from_u64(12345)`）を使用

### T2: child_ratio = 0 で child-support 関連指標が全て 0

- `ReciprocitySimulatorConfig { child_ratio: 0.0, .. }` で実行
- child が存在しないため、village churn, help_success_rate が全て 0 または適切なデフォルト値になること
- child survival rate が 0（生存すべき child がいない）になること

### T3: max_ticks = 0 で空の metric series

- `ReciprocitySimulatorConfig { max_ticks: 0, .. }` で実行
- `metric_series` が空の Vec であること
- `final_state` が初期状態と一致すること

### T4: 善良群と非善良群の生存率差が正であること（優しい世界の創発）

- 善良（benevolence 高）な workflow 群と非善良（benevolence 低）群を合成 population として生成
- シミュレーション後の生存率を両群で比較し、善良群の生存率が非善良群以上であることを確認
- すなわち `AUC_benevolent > nonbenevolent >= 0.5`
- 注: これは創発の必要条件であり、十分条件ではない（統計的有意性は観測対象）

### T5: policy.lambda_gc_base = 0 で GC が一切発生しない

- `ReciprocityLifecyclePolicy { lambda_gc_base: 0.0, .. }` で実行
- 死亡 workflow 数が 0 であること（全員生存）
- GC hazard の累積値が全 tick で 0 になること

### T6: 各種 metrics のレンジ検証

- `benevolence_score_p50/p95` が [0, 1] 範囲に収まる
- `direct_reciprocity_p50/p95`, `indirect_reciprocity_p50/p95` が [0, 1] 範囲に収まる
- `reputation_final_p50/p95` が [0, 1] 範囲に収まる
- `harmful_gc_rate` が [0, 1] 範囲に収まる

### T7: 境界値パラメータで panic しない

- `population_size = 1`, `child_ratio = 0.0`, `child_ratio = 1.0` で実行
- `mission_rate = 0.0`, `mission_rate = 1.0` で実行
- いずれも panic せず正常終了すること

### T8: SimulationResult の experiment_id 形式検証

- `exp-{yyyymmdd}-{seq}` 形式に従っていること
- 親実験 ID が正しく伝播すること

### T9: 既存テストとの後方互換性

- `cargo test` で既存 1033+ テストが全て PASS すること

## 計装方法・観測対象

### 計装方法

- 新規ファイル `src/simulation.rs` として実装（独立モジュール）
- 全コンポーネントは固定シード PRNG（`StdRng::seed_from_u64(12345)`）で駆動し、完全再現を保証
- `println!` + `--nocapture` で以下の構造化データを標準出力に書き出す：
  - 各 tick 終了時の metrics snapshot（JSON-like: `{"tick":N, "benevolent_survival":X, "nonbenevolent_survival":Y}`）
  - 最終 metrics series の CSV 形式出力
  - AUC_benevolent_survival 値
- 観測テストは `cargo test -- --nocapture` で実行

### 観測対象

- **時系列メトリクス** (tick ごと):
  - `benevolence_score_p50/p95` — 善良性スコアの分布
  - `direct_reciprocity_p50/p95` — 直接互恵性の分布
  - `indirect_reciprocity_p50/p95` — 間接互恵性の分布
  - `reputation_final_p50/p95` — 最終評判の分布
  - `benevolent_survival_advantage` — 善良群と非善良群の生存率差
  - `harmful_gc_rate` — 有害 GC 発生率
- **創発検証**:
  - T4 の生存率差が正であること (MUST)
  - 善良群と非善良群の生存 ranking AUC (観測対象)
- **副作用モニタリング**:
  - village churn、false-new rate、review-load への影響を同時観測

### 較正計画

- 本チケットでは較正パラメータの sweep は実施しない（ハーネスに統合する metrics 生成基盤の構築が目的）
- シミュレーターのパラメータ（`population_size`, `mission_rate` 等）はテスト内で指定可能とし、デフォルト値を `ReciprocitySimulatorConfig::default()` で提供
- `gamma_benevolence` を sweep するための準備として、ReciprocityLifecyclePolicy を config 経由で注入可能にする

## Boy Scout Rule — 翻訳可能性計画

本チケットで新規作成する `src/simulation.rs` に対して以下を徹底する：

- **関数名は動詞句**: `generate_population`, `generate_mission_stream`, `simulate_help_interaction`, `recompute_trust_reputation`, `run_lifecycle_gc`, `observe_tick`, `run_simulation`
- **変数名はドメイン概念**: `benevolence_scores`, `survival_flags`, `population`, `mission_queue`, `help_session_registry` — 抽象名を避ける
- **一関数一責務**: pop 生成、mission 生成、HELP シミュレーション、trust 再計算、GC ループ、tick 観測はそれぞれ別関数（または別メソッド）
- **ハードコード値禁止**: シミュレーションパラメータのデフォルト値は `ReciprocitySimulatorConfig` に集約
- **エラー握りつぶし禁止**: `unwrap()` 不使用、`Result` 伝播または `expect("reason")` に限定
- **コメントの日本語化**: 公開 API の rustdoc は日本語。内部関数の意図説明も日本語

既存コードへの影響：
- 既存 `run_replay_scenario` の改変は行わない
- 既存 `calibration.rs` の `ReciprocityCalibrationHarness` は本チケットでは変更しない

## Acceptance Criteria

- [ ] `ReciprocitySimulatorConfig` が正しく定義されている
- [ ] `SyntheticPopulationGenerator` が固定シードで決定論的に population を生成する
- [ ] `MissionStreamGenerator` が指定 rate で mission を生成する
- [ ] `HelpInteractionSimulator` が benevolence-biased で HELP プロトコルをシミュレートする
- [ ] `TrustReputationRecomputeLoop` が既存 F-1〜F-5 関数を正しく呼び出す
- [ ] `LifecycleGcLoop` が既存 F-7/F-8 関数を正しく呼び出す
- [ ] `TickObserver` が全 tick の状態を記録する
- [ ] `ReciprocitySimulationResult` が metric_series + final_state + experiment_id を持つ
- [ ] 同一 seed で決定論的リプレイがビットレベルで一致する
- [ ] child_ratio = 0 で child 関連指標が全て 0 になる
- [ ] max_ticks = 0 で空系列が返る
- [ ] 善良群と非善良群の生存率差が正である（創発の必要条件充足）
- [ ] lambda_gc_base = 0 で GC が発生しない
- [ ] 全 metrics が [0, 1] 範囲に収まる
- [ ] 境界値パラメータで panic しない
- [ ] 実験 ID 形式が正しい
- [ ] 既存テスト（1033+）が全て PASS する
- [ ] 翻訳可能性の検証が通っている

## Notes

- 本シミュレーターは src/simulation.rs に独立モジュールとして実装する
- 既存の `run_replay_scenario` （replay.rs）は改変しない
- `ReciprocityCalibrationHarness`（calibration.rs）の統合は M1.76-19 で行う
- 観測出力は後続の較正フェーズ（M1.76-19）での目的関数 J(θ) 計算の入力として利用される
- plan_path: context/0102-m176-17-phase-3-synthetic-ecosystem-simulation/plan.md（未作成、/plan-ticket 承認後に作成）
- implementation_path: context/0102-m176-17-phase-3-synthetic-ecosystem-simulation/implementation.md（未作成、/start-ticket 実装完了後に作成）
- review_report_path: context/0102-m176-17-phase-3-synthetic-ecosystem-simulation/review.md（未作成、/review-ticket 全チェック通過後に作成）
- observation_report_path: context/0102-m176-17-phase-3-synthetic-ecosystem-simulation/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）

### 成果物

- 計画: context/0102-m176-17-phase-3-synthetic-ecosystem-simulation/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0102-m176-17-phase-3-synthetic-ecosystem-simulation/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0102-m176-17-phase-3-synthetic-ecosystem-simulation/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0102-m176-17-phase-3-synthetic-ecosystem-simulation/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
