# M1.76-16 実装サマリー

## 変更したファイル一覧

| ファイル | 種別 | 内容 |
|---------|------|------|
| `src/constants.rs` | 追加 | 6 つの F-16 λ 重み定数（F16_LAMBDA_AUC 〜 F16_LAMBDA_INSTABILITY） |
| `src/calibration.rs` | 追加 | 全データ構造体、純粋関数、ReciprocityCalibrationHarness、テスト 11 件 |

## 実装内容の概要

### データ構造体
- `SurvivalPair` — AUC 計算の基本単位
- `ReciprocityOperationalMetrics` — 式 F-16 の 6 成分メトリクス
- `ReciprocityCalibrationConfig` — 較正設定（λ 重み + パラメータ範囲）
- `ReciprocityCalibrationResult` — 単一パラメータ設定の評価結果
- `ReciprocityCalibrationReport` — 完全レポート（実験ID + 全結果）
- `ReciprocityCalibrationHarness` — 統合ハーネス

### 純粋関数
- `compute_auc_benevolent_survival` — Mann-Whitney U 統計量による AUC 計算
- `compute_calibration_objective` — 式 F-16 合成値（[0, 1] クランプ）

### 実験系列管理
- `exp-{yyyymmdd}-{seq}` 形式の実験ID
- 親実験ID の追跡（レポート生成時に指定可能）

### テスト (11 件)
| テスト | 内容 | 結果 |
|--------|------|------|
| T1 | ランダム ranking AUC ≈ 0.5 | PASS (0.490367) |
| T2 | 完全分離 ranking AUC ≈ 1.0 | PASS (1.0) |
| T2b | 全員同一 AUC = 0.5 | PASS (0.5) |
| T2c | 空スライス AUC = 0.5 | PASS (0.5) |
| T3 | 全 λ = 0 で J = 0 | PASS |
| T4 | 決定論的再現性 | PASS (0.328) |
| T5 | 極値パラメータで NaN/Inf 回避 | PASS |
| T6 | 空パラメータセット graceful | PASS |
| T7 | 同一 θ で決定論的 J(θ) | PASS (0.312) |
| T8 | 各成分の分離検証 | PASS |
| T9 | 実験ID 形式検証 | PASS |

### 検証結果
- 全 1044 テスト PASS（既存テスト含む）
- `cargo clippy --lib -- -D warnings` PASS
