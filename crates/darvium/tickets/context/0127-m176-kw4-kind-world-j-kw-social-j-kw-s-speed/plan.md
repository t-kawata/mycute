# M1.76-KW4: Kind World 較正ループ実行（J_kw_social = J_kw × s_speed）

## 要件の再確認

KW-REAL 基盤上で Nelder-Mead 最適化を実行し、目的関数を J_kw から J_kw_social = J_kw × s_speed に変更する。これにより「状態の質」と「到達速度」を同時最適化する。

## RFC 既存実装状態検証

### RFC §15.9.2 KindWorldAssessment vs 現行コード
全 28 フィールドが RFC 定義と一致。閾値 `j_kw > 0.8` は診断用として継続。

| 項目 | RFC | 現行コード | 状態 |
|------|-----|-----------|------|
| evaluate_single 戻り値 | J_kw_social (§15.9.2) | J_kw | ❌ 要修正 |
| OptimizationReport.best_j_kw | best_j_kw_social | best_j_kw | ❌ 要追加 |
| tick_to_convergence/s_speed | §15.9.2 定義 | 未実装 | ❌ 要追加 |
| is_kind_world (診断) | j_kw > 0.8 (§15.9.2) | j_kw > 0.8 | ✅ 診断用として継続 |
| is_kind_world (較正) | J_kw_social > 0.64 (§15.9.2) | 未実装 | ❌ 評価関数変更時に解決 |
| MagnificentSevenParams | 7 fields (§15.9.1) | 7 fields | ✅ 一致 |

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|---------|------|------|
| src/constants.rs | 追加 | KW4_OBSERVATION_INTERVAL (10), KW4_CONVERGENCE_THRESHOLD (0.8) |
| src/simulation.rs | 修正 | run_evaluation_simulation に mid-simulation サンプリング + tick_to_convergence 計算 |
| src/kind_world.rs | 修正 | evaluate_single J_kw_social 対応, OptimizationReport 拡張, NelderMeadOptimizer::run 更新, テスト更新 |

## 計装・観測の実装計画

1. compute_s_speed(tick_to_convergence, total_ticks) -> f64 — 速度因子計算関数
2. run_evaluation_simulation の戻り値拡張: (KindWorldMetricsInput, u64)
3. mid-simulation サンプリング: tick ループ内で tick % KW4_OBSERVATION_INTERVAL == 0 のタイミングで s_growth 成分 + j_cov を計算
4. s_speed 計算、evaluate_single が J_kw_social を返すよう変更
5. OptimizationReport に best_j_kw_social, tick_to_convergence, s_speed を追加
6. CSV/JSON 出力更新 (tc6_kw4_optimize_run)

## 実装手順

1. constants.rs に 2 定数追加
2. simulation.rs の run_evaluation_simulation に mid-simulation サンプリング機構 + tick_to_convergence 計算を追加、戻り値型拡張
3. kind_world.rs に compute_s_speed 関数追加
4. evaluate_single を J_kw_social 対応に変更
5. OptimizationReport にフィールド追加
6. NelderMeadOptimizer::run のレポート作成更新
7. テスト更新（TC1e〜TC8e）
8. cargo test + cargo clippy 全 PASS 確認

## 物理的レビュー方法

- run-quality-checks.js で変更ファイル（simulation.rs, kind_world.rs, constants.rs）を静的解析
- 翻訳可能性 grep: fn [A-Z]（名詞始まり関数）、\b(tmp|data|info)\b（汎用変数名）
- cargo test 全 PASS + cargo clippy -- -D warnings

## リスク

- mid-simulation サンプリングの計算量: KW4_OBSERVATION_INTERVAL=10 で 100tick 中 10 回、許容範囲
- 既存テストとの互換性: run_evaluation_simulation の戻り値型が変わるため呼び出し元を全て確認
- history の f64 値意味変化: J_kw → J_kw_social。CSV ヘッダー変更で対処
