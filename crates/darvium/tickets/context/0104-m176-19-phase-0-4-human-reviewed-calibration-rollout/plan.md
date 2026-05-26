# 計画: M1.76-19 Phase 0-4 Calibration Rollout

## RFC 既存実装状態検証
RFC §15.10.9: 全 Phase で個別部品（run_simulation, HumanReviewQueue, perturbation 関数, sweep 関数）は揃っているが、Phase Runner 統合層（CalibrationPhase, PhaseGate, Phase0〜4Runner, CalibrationRolloutReport, simulation_result_to_operational_metrics）は全件未実装。

## 変更ファイル一覧
| ファイル | 種別 | 内容 |
|----------|------|------|
| src/calibration.rs | 編集 | CalibrationPhase, PhaseGate, Phase0〜4Runner, CalibrationRolloutReport, simulation_result_to_operational_metrics を追記 |
| src/constants.rs | 編集 | 新規定数追加（PHASE_GATE_MAX_PHASES 等） |
| src/lib.rs | 編集 | 公開 API run_calibration_pipeline() 追加 |

## 実装手順
1. CalibrationPhase 列挙型 + PhaseGate 構造体
2. simulation_result_to_operational_metrics() 変換関数
3. Phase0Runner — 純粋関数検証
4. Phase1Runner — 決定論的リプレイ
5. Phase2Runner — 摂動テスト
6. Phase3Runner — シミュレーション sweep
7. CalibrationRolloutReport + Phase4Runner
8. run_all_phases() 統合 + 公開 API
9. テスト T1〜T23 + 定数追加

## リスク
- Phase 3 シミュレーション全滅（lambda_gc_base 0.3〜0.5 の範囲で開始）
- simulation_result_to_operational_metrics の近似値誤差
- calibration.rs 2600 行超過時は calibration_rollout.rs に分割
