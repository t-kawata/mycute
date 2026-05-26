# 変更したファイル一覧と実装内容の概要

## 変更ファイル

| ファイル | 種別 | 内容 |
|----------|------|------|
| `src/reciprocity.rs` | 変更 | M1.76-15 proptest 不変条件ファジング実装 |

## 実装内容の概要

### 新規追加コード

`src/reciprocity.rs` の mod tests 末尾に以下を追加:

1. **`pbt_runner()`** — TestRunner ファクトリ関数（PROPTEST_DEFAULT_CASES=10000、failure_persistence=無効）

2. **T1: `test_pbt_benevolence_monotonicity`** — 善意スコア単調性（d1, i1, r1 の 3 入力次元 × ±Δ で 6 方向）
3. **T2: `test_pbt_hazard_non_negativity`** — GC hazard 非負性（lc, b, cp の 3 次元ランダム）
4. **T3: `test_pbt_probability_boundedness`** — GC 確率・生存確率の [0,1] 有界性
5. **T4: `test_pbt_no_negative_reputation`** — 評判スコア非負（[-0.1, 1.1] 拡張範囲）
6. **T5: `test_pbt_no_silent_overflow_nan`** — NaN/Inf 未発生（±1e6 範囲）
7. **T6: `test_pbt_grace_period_child_protection`** — 子供保護による hazard 低減
8. **T6b: `test_pbt_grace_period_statistical`** — Welch t-test（n=10000, StdRng 固定種）
9. **E1-E3**: 極値ケース（全係数ゼロ / 全係数 max / 人口端値）

### テスト結果

- 全 6 不変条件 × 10,000 ケース: violations = 0
- 全 3 極値ケース: PASS
- Welch t-test: 保護あり 0.975 vs 保護なし 1.103（t=-92.54）
- 1033 テスト中 0 失敗、後方互換性完全維持

### アーキテクチャ上の決定

- proptest! マクロ不使用 → TestRunner::run() パターン採用（ケース数制御・エラー処理のため）
- 観測テスト出力は `println!` + `--nocapture` 経由（Darvium 観測ベース検証方式）
- 統計的検証（T6b）は独立した StdRng ベース（proptest 非依存）
