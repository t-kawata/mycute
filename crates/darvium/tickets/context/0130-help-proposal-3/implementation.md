# 変更したファイル一覧と実装内容の概要

## 変更ファイル

| ファイル | 変更種別 | 内容 |
|----------|---------|------|
| src/constants.rs | 追加 | CHILD_HELPEE_BIAS_FACTOR = 2.0 定数追加 |
| src/simulation.rs | 修正 | phase3_help_protocol のハードフィルタ撤去 + 任意ペア HELP 化 |
| src/simulation.rs | 追加 | FIX-C1〜C7 テスト追加（extract_help_directions, extract_pair_counts, test_fixc_*） |

## 実装の概要

### constants.rs
- `CHILD_HELPEE_BIAS_FACTOR` (f64 = 2.0): 子供が helpee として選択される確率バイアス。Calibration Candidate。

### simulation.rs — phase3_help_protocol (lines ~1779-1842)
- 成人→子供のハードフィルタを撤去し、全 alive ノードから任意の helper/helpee を選択するよう修正
- 子供 helpee バイアス: 重み付き確率選択（child_weight = n_children × BIAS_FACTOR, adult_weight = n_adults × 1.0）
- 自己 HELP 禁止: helper_id == helpee_id の場合はスキップ
- reciprocity_pair_counts の記録方向を (adult, child) 固定から任意 (helper, helpee) に変更

### simulation.rs — FIX-C テスト (lines ~3930-4175)
- C1: adult→adult HELP 発生確認（50回）
- C2: child→child HELP 発生確認（2794回）
- C3: child→adult HELP 発生確認（129回）
- C4: adult→child HELP 発生確認（292回、従来方向維持）
- C5: 双方向ペア出現確認（668種類）
- C6: compute_mean_reciprocity > 0 確認（0.210）
- C7: 子供 helpee バイアス観測（94.5% > 65.0%）

## 設計判断
- RFC §41B-9 の Child(c)∧Adult(h) 条件は意図的に逸脱。RFC 改訂は別チケット。
- RFC §41B.20.1 F-11 の helper 品質スコアは変更せず、helpee 側バイアスを提案生成段階で追加。
- セッション進行ロジック（Proposal→Offered 等）は変更なし。

## 検証結果
- cargo test: 1307 passed, 0 failed, 5 ignored
- cargo clippy: 警告なし
- cargo build: 成功
