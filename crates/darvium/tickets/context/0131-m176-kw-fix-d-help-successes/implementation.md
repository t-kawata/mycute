# 変更したファイル一覧と実装内容の概要

## 変更ファイル

### `src/simulation.rs`

1. **`run_kw_real_simulation` 関数（line 1266-1324）**:
   - `help_successes` 累積変数（`Vec<(NodeId, NodeId)>`）を削除
   - `help_successes.extend(new_successes)` を削除
   - `phase5_capability_diffusion` の第2引数を `&help_successes` → `&new_successes` に変更
   - これにより各 tick の新規成功のみが処理されるようになった

2. **`run_evaluation_simulation` 関数（line 1505-1561）**:
   - 同一の修正を適用

3. **FIX-D テスト追加（mod tests）**:
   - `test_fixd_single_help_single_exp` (D1): 1 回の HELP 成功 → 経験値 +1 を確認
   - `test_fixd_two_ticks_separate_exp` (D2): 2 tick 連続 HELP → 各 helpee の experience == 1、再処理なしを確認
   - `test_fixd_observe_avg_exp` (D3): 修正後平均経験値を観測出力
   - `test_fixd_existing_tests_pass` (D4): シミュレーション正常動作確認

## 修正の本質

- **バグ**: `help_successes` が関数スコープで宣言され各 tick で extend されるがクリアされない。`phase5_capability_diffusion` に累積全件が渡され、過去の success が毎 tick 再処理される。経験値が tick 数倍に膨張。
- **修正**: 累積変数を削除し、`new_successes`（当 tick 分の新規成功）のみを直接 phase5 に渡す。
- **影響範囲**: 2 つのシミュレーション関数（run_kw_real_simulation, run_evaluation_simulation）。従来の `run_simulation` は無影響。

## 検証結果

- `cargo build`: ✅ 成功
- `cargo test`: ✅ 全 1311 テスト PASS（0 failed, 5 ignored）
- `cargo clippy -D warnings`: ✅ クリーン
