# M1.76-KW1: Kind World 成立条件定数 + J_kw 目的関数実装 — 実装計画

## RFC 既存実装状態検証

### RFC §15.9.1 Kind World 成立条件 — 定数テーブル (8 Safety Invariants + 2 村定数)
全 10 定数が未実装。新規追加が必要。

### RFC §15.9.1 MagnificentSevenParams
構造体そのものが未定義。SWEEP_MAGNIFICENT_PARAM_NAMES 定数配列のみ存在。

### RFC §15.9.2 重み係数 (6 Calibration Candidates)
全 6 定数が未実装。

**評価サマリ**: 全 18 項目が未実装。全て新規実装となる。

## 要件

Kind World の成立を定義する 8 つの成立条件定数 + 6 つの J_kw 重み係数を constants.rs に追加し、KindWorldAssessment 構造体と compute_kind_world_objective() 純粋関数を新規 kind_world.rs に実装する。

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|---------|------|------|
| src/constants.rs | 追加 (16定数) | KW 条件閾値 8個 + 村定数 2個 + J_kw 重み 6個 |
| src/kind_world.rs | 新規 | KindWorldAssessment, KindWorldMetricsInput, MagnificentSevenParams + compute_kind_world_objective, compute_village_health_score + 全10テストケース |
| src/lib.rs | 変更 | pub mod kind_world; を追加 |

## 計装・観測の実装計画

| 項目 | 内容 |
|------|------|
| テストファイル | src/kind_world.rs 内 mod tests |
| 観測テスト | --nocapture で構造化 JSON 出力 |
| 観測統計量 | J_kw の平均/最小/最大/NaN出現率 |
| サンプルサイズ | n=10,000 ランダム入力 |
| PRNG シード | StdRng::seed_from_u64(12345) |

## テストケース (10件)

1. kw_all_conditions_met — 全8条件成立
2. kw_all_conditions_not_met — 全8条件不成立
3. kw_j_kw_range_random — n=10,000 ランダム入力で範囲検証
4. kw_j_pop_monotonic — J_pop 単調性
5. kw_weight_sum_one — Σα_i == 1.0 静的アサート
6. kw_empty_input_no_panic — 空入力で panic しない
7. kw_penalty_benevolent_inferior — 慈悲劣位で J_penalty > 0
8. kw_penalty_benevolent_equal — 慈悲同等で J_penalty = 0
9. kw_boundary_threshold — 境界値 ±0.001 試験
10. kw_json_roundtrip — serde ラウンドトリップ

## 実装手順

1. src/constants.rs に 16 定数を追加
2. src/kind_world.rs を新規作成 (構造体 + 関数 + テスト)
3. src/lib.rs に pub mod kind_world; を追加
4. cargo test で全テスト通過確認
5. cargo clippy で警告なし確認

## 物理的レビュー方法

```
_R=$(cat DARVIUM_PLUGIN_ROOT.md)
node "$_R/scripts/tickets/review/run-quality-checks.js" src/constants.rs src/kind_world.rs src/lib.rs
cargo test -- --nocapture
cargo clippy -- -D warnings
```

## リスク

| リスク | 確率 | 対策 |
|--------|------|------|
| 浮動小数点演算による境界値誤差 | 低 | clamp + f64::EPSILON 使用 |
| serde フィールド追加忘れ | 低 | ラウンドトリップテストで確認 |
| 重み合計 ≠ 1.0 | 低 | 静的アサートでコンパイル時検証 |
