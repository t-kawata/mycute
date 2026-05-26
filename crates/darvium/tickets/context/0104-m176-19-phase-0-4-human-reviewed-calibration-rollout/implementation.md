# 変更したファイル一覧と実装内容の概要

## src/constants.rs
- PHASE_GATE_MAX_PHASES: usize = 5 — 較正パイプラインの最大 Phase 数
- CANARY_ENVIRONMENT_TAG / PRODUCTION_ENVIRONMENT_TAG — 2段階ロールアウト用環境タグ
- SWEEP_MAGNIFICENT_PARAM_NAMES — MagnificentSevenParams の7パラメータ名リスト

## src/calibration.rs
### 新規データ構造
- CalibrationPhase enum (Phase0〜Phase4) + idx() + all()
- PhaseStatus enum (Pending/Pass/Fail)
- PhaseGate (HashMap<CalibrationPhase, PhaseStatus>) — ゲート機構
- CalibrationRolloutReport — ロールアウトレポート
- ReciprocityOperationalMetrics — 運用メトリクス6成分

### 新規関数
- apply_params_to_sim_config() — パラメータ名→シミュレーション設定反映
- simulation_result_to_operational_metrics() — シミュレーション結果→メトリクス変換
- default_replay_scenario(seed) — 最小 VillageReplayScenario 生成
- evaluate_simulation_params() — パラメータ評価
- run_phase0() — 純粋関数検証（値域・単調性・空入力）
- run_phase1() — 決定論的リプレイ（全シード同一性検証）
- run_phase2() — 摂動テスト（churn/JSD bounds）
- run_phase3() — OFAT sweep（7パラメータ×4ステップ = 28候補）
- run_phase4() — 上位5候補選択 + human_review_queue + canary/production分離
- run_all_phases() — 5 Phase 直列統合パイプライン

### テスト
- mod phase_rollout_tests (T10-T23, 14 tests) 追加済み

## src/lib.rs
- CalibrationPhase, PhaseGate, PhaseStatus 等の公開
- Darvium::run_calibration_pipeline()  Facade メソッド追加
