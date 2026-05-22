# 計画: M-1-1 EvaluateCandidatesStep

## 要件
- SearchOutcome enum（RFC §13.3、6バリアント）実装
- evaluate_candidates(score): >=0.50 → ReuseExisting、<0.50 → PatchExisting
- apply_self_conf_discount(raw): raw * SELF_CONF_DISCOUNT(0.85)、[0.0,1.0]でクランプ
- EVALUATION_THRESHOLD = 0.50 を constants.rs に追加
- InvalidScore エラーを error.rs に追加
- 網羅的テスト（T1〜T5 + OTS-1/2）

## 変更ファイル一覧
| ファイル | 種別 | 内容 |
|---------|------|------|
| src/constants.rs | 追加 | EVALUATION_THRESHOLD (0.50) |
| src/error.rs | 追加 | InvalidScore(f64) |
| src/types.rs | 追加 | SearchOutcome, WorkflowGraphId, evaluate_candidates(), apply_self_conf_discount(), 全テスト |
| src/lib.rs | 修正 | SearchOutcome を re-export |

## 実装手順
1. constants.rs: EVALUATION_THRESHOLD 追加
2. error.rs: InvalidScore 追加
3. types.rs: 型・関数・テスト実装
4. lib.rs: re-export 追加
5. cargo test → cargo clippy → cargo fmt

## 物理的レビュー方法
- cargo test -- --nocapture
- cargo clippy -- -D warnings
- cargo fmt -- --check
- grep 'fn [a-z]' src/types.rs（関数名が名詞始まりでないか）
- grep '\b0\.50\b' src/types.rs（ハードコードされた閾値がないか）

## リスク
- CompositionPlan / GraphPatch が未実装 → スタブ構造体で仮置き
- 純粋関数のため他モジュールへの影響は限定的
