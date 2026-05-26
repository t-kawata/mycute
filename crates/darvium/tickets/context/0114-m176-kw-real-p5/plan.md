# Plan: M1.76-KW-REAL-P5 ライフサイクル・成熟機構

## 要件の再確認
P1 (SimulationContext) 完了後に実装する P5。P4 (6 フェーズループ) の GC 処理で使用される全機構を提供する。5 機構が未実装、1 機構が部分実装。

## 変更ファイル一覧
| ファイル | 種別 | 内容 |
|---------|------|------|
| src/event.rs | 変更 | GcEvent に Protected/Active 追加 + transition_gc_state |
| src/lifecycle.rs | 新規追加 | LifecycleScore 構造体 + compute_lifecycle_score |
| src/trust.rs | 変更 | inherit_trust / inherit_reputation |
| src/reciprocity.rs | 変更 | compute_experience_normalization (F-5) |
| src/clock/mod.rs | 変更 | compute_blended_freshness (F_time) |
| src/lib.rs | 変更 | 新規モジュール・公開関数のエクスポート |
| src/simulation.rs | 変更 | 観測テスト + 旧 inline LifecycleScore 参照更新 |

## 計装・観測の実装計画
- 観測テスト: src/simulation.rs mod tests → test_p5_lifecycle_instrumentation
- 出力: CSV (tick/node_id/lifecycle_score/gc_state/maturity/experience_count) + JSON (経験値分布)
- 固定シード: StdRng::seed_from_u64(12345)
- サンプル: n=50 pop × 20 ticks = 1,000 観測点
- 較正: 本チケットでは行わない。全テスト PASS を完了条件とする。

## Boy Scout 改善
1. simulation.rs:729-731 inline LifecycleScore → 新規関数に抽出
2. MIN_SURVIVAL_EXPERIENCE のドキュメント値修正 (3→5)

## 実装手順
1. 新規 src/lifecycle.rs (LifecycleScore + compute_lifecycle_score + TC2/TC3)
2. src/event.rs (GcEvent 拡張 + transition_gc_state + TC4/TC5)
3. src/trust.rs (inherit_trust / inherit_reputation + TC6/TC7)
4. src/reciprocity.rs (compute_experience_normalization + TC8)
5. src/clock/mod.rs (compute_blended_freshness + TC9)
6. src/lib.rs (エクスポート追加)
7. 観測テスト追加 (simulation.rs)
8. cargo test + clippy 確認

## 物理的レビュー方法
- run-quality-checks.js on changed files
- 翻訳可能性 grep (動詞句関数名, 1文字変数, マジックナンバー)
- validate-structure.js

## リスク
1. GcEvent::Protected と EventVisibility::Protected の同名共存 (問題なし、別 enum)
2. compute_gc_hazard の f32 引数互換性維持
3. SimWorkflowState の旧式 LifecycleScore は変更しない (P1 で新旧共存)
