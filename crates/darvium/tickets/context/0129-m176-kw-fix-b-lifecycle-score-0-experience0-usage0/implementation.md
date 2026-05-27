# 変更したファイル一覧と実装内容の概要

## 変更ファイル

### `src/constants.rs`
- `EXPERIENCE_NORMALIZATION_OFFSET: f64 = 1.0` を追加 — compute_experience_normalization に加算するオフセット
- `GC_HAZARD_GAMMA_CHILD_PROTECT: 10.0 → 5.0` — lifecycle_score 正常化に伴い緩和

### `src/reciprocity.rs`
- `compute_experience_normalization` の計算式を変更:
  - Before: `1.0 - exp(-experience / scale)`
  - After: `1.0 - exp(-(experience + offset) / scale)`
- TC8 テスト期待値を 0.0 → ~0.095 に更新
- FIX-B1 テスト追加: experience=0 で usage ∈ (0.05, 0.15) を確認

### `src/lifecycle.rs`
- FIX-B3 テスト追加: usage=0.095 + 他成分 0.8 → lifecycle_score > 0.3 を確認

### `src/simulation.rs`
- `check_convergence` 関数の引数を config 経由にリファクタリング（8→7引数、clippy too_many_arguments 対応）
- `tick % KW4_OBSERVATION_INTERVAL` を `tick.is_multiple_of(...)` に変更（clippy manual_is_multiple_of 対応）
- FIX-B5 観測テスト追加: 子供/成人別 lifecycle_score 分布出力
- FIX-B6 観測テスト追加: 子供/成人別 usage 5 数要約出力
- FIX-B7 観測テスト追加: 子供 vs 成人 GC hazard 比較

## Boy Scout 改善
- check_convergence 引数削減による clippy 警告修正
- is_multiple_of への置き換えによる clippy 警告修正
