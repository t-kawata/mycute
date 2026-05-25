# 実装サマリ: M1.75-11 Village Calibration Loop Harness (\(J_{village}\))

## 変更したファイル一覧

| ファイル | 種別 | 内容 |
|----------|------|------|
| `src/calibration.rs` | 新規作成 | 較正ハーネス本体（~860行） |
| `src/lib.rs` | 編集 | `pub mod calibration;` の追加（1行） |
| `src/constants.rs` | 編集 | 目的関数重み5定数 + スイープ設定3定数の追加（8行） |
| `src/village.rs` | 編集 | `VillageMetricsSnapshot` に `Default` derive 追加（1行） |

## 実装内容の概要

### データ型
- `ParameterRange`: パラメータ名・最小値・最大値・ステップ数
- `SweepMode`: OFAT / Grid / LHS の3モード
- `VillageCalibrationConfig`: パラメータ範囲・重み・スイープ設定
- `VillageCalibrationResult`: パラメータ値・目的関数値・内訳・トレース
- `CalibrationReport`: 全結果・最良結果・設定のサマリ
- `VillageCalibrationHarness`: Facade 構造体

### コア関数
- `compute_village_objective()`: 純粋関数、J 値を [0.0, 1.0] にクランプ
- `run_sweep_ofat()`: n_params × (steps+1) 評価
- `run_sweep_grid()`: デカルト積
- `run_sweep_lhs()`: 層化ランダムサンプリング

### テスト（全12件 PASS）
- C-1〜C-5: 目的関数の境界値・決定論性・感度・劣化・重み感度
- C-6〜C-10: スイープモード3種 + 整合性 + レポート形式
- C-11〜C-12: 不変条件非侵害 + 空パラメータ
