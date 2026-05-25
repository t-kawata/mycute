# M1.75-11: village calibration loop harness と目的関数 J_village(θ) の実装

## RFC 既存実装状態検証

| チェック項目 | 結果 |
|---|---|
| `VillageMetricsSnapshot` のフィールド | RFC §41B.15 の metrics と整合。churn/jsd/survival は利用可能 |
| `false_new_count` の有無 | ❌ 未実装（全コードベースで不在）。本チケットでは 0 固定で仮対応 |
| `review_load_count` の有無 | ❌ 未実装（全コードベースで不在）。本チケットでは 0 固定で仮対応 |
| `run_replay_scenario` → `ReplayTrace` | ✅ 利用可能。後処理で VillageMetricsSnapshot を再度計算可能 |

**評価サマリ**: false_new / review_load は v2.3 Fake 実行環境では観測困難なため目的関数内では 0 固定。

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|---|---|---|
| `src/calibration.rs` | 新規 | 較正ハーネス全体（データ型、目的関数、sweep 3 モード、ハーネス） |
| `src/lib.rs` | 編集 | `pub mod calibration;` を追加 |
| `src/constants.rs` | 編集 | 目的関数重み a₁〜a₅、sweep パラメータ定数を追加 |

## 詳細設計

### constants.rs に追加する定数
- OBJECTIVE_WEIGHT_CHURN = 0.35
- OBJECTIVE_WEIGHT_JSD = 0.25
- OBJECTIVE_WEIGHT_SURVIVAL = 0.25
- OBJECTIVE_WEIGHT_FALSE_NEW = 0.10
- OBJECTIVE_WEIGHT_REVIEW_LOAD = 0.05
- SWEEP_OFAT_DEFAULT_STEPS = 5
- SWEEP_GRID_DEFAULT_DIVISIONS = 3
- SWEEP_LHS_DEFAULT_SAMPLES = 20

### calibration.rs モジュール構成
- VillageCalibrationConfig — 較正対象パラメータの束と sweep 設定
- ParameterRange — パラメータ名・範囲・デフォルト値
- VillageCalibrationResult — 目的関数値 + 成分値
- CalibrationReport — 全結果 + メタデータ
- SweepMode — Ofat / Grid / LatinHypercube 列挙型
- compute_village_objective — 純粋関数: &VillageMetricsSnapshot, &[f64; 5] → f64
- run_sweep_ofat / run_sweep_grid / run_sweep_lhs — 各 sweep モード
- VillageCalibrationHarness — 統合ハーネス（config → sweep → report）

### compute_village_objective の処理
- churn = w[0] * (1.0 - snapshot.village_churn_p95)
- jsd = w[1] * (1.0 - snapshot.helper_jsd_p95)
- survival = w[2] * snapshot.child_survival_rate
- false_new = w[3] * 0.0 (現状 0 固定)
- review_load = w[4] * 0.0 (現状 0 固定)
- J = churn + jsd + survival - false_new - review_load
- clamp: [0.0, 1.0]

## 計装・観測の実装計画
- テスト: src/calibration.rs 内 mod tests (C-1〜C-12)
- 観測出力: println! + cargo test -- --nocapture
- サンプルサイズ: 目的関数テスト n≥10 / sweep テスト n≥100

## Boy Scout 改善
- 既存コードの翻訳可能性問題は確認の上、該当箇所のみ修正

## 物理的レビュー方法
1. run-quality-checks.js で全 spec 整合性チェック
2. 翻訳可能性 grep: 名詞始まり関数、1文字変数、ハードコード値
3. cargo clippy -- -D warnings
4. テスト観測出力確認

## リスク
- run_replay_scenario 内の PRNG が calibration 側から seed 制御可能であること要確認
- LHS は自前実装のため stratified random sampling で代用可能性
