# 実装計画: M1.76-KW-MTR-A — Lifecycle & Freshness Metrics Backfill

## RFC §15.9.3 既存実装状態検証

| 指標 | RFC 定義 | 本チケット実装 | 乖離 |
|------|---------|---------------|------|
| mean_lifecycle_score | LifecycleScore L(G) 集団平均 | GcEvent 状態分布スコア平均 | ⚠️ 下流指標 (L(G) 完全一致は MTR-B/D 完了後) |
| child_survival_rate | 生存子供 / 全子供 | 同左 | ✅ 一致 |
| mean_freshness | BlendedFreshness 平均 | 同左 | ✅ 一致 |

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|---------|------|------|
| simulation.rs | 修正 | SimulationContext: node_gc_states, node_last_update_tick, total_births 追加 |
| simulation.rs | 修正 | new() / add_person() / phase1 / phase4 を ctx 対応 |
| simulation.rs | 修正 | collect_final_metrics 呼び出しに child_count 追加 |
| kind_world.rs | 追加 | 4 関数 (lifecycle_score_from_gc_state, compute_mean_lifecycle_score, compute_child_survival_rate, compute_mean_freshness) |
| kind_world.rs | 修正 | collect_final_metrics 引数 + child_count、3 指標置き換え |

## 実装手順

1. SimulationContext に 3 フィールド追加 + new() 初期化 + add_person() 拡張
2. run_evaluation_simulation: ローカル node_gc_states → ctx 移行、total_births 累積
3. phase1_population_growth: ctx.total_births インクリメント、子ノード更新 tick
4. phase4_gc_survival: ctx.node_gc_states / ctx.node_last_update_tick 使用
5. kind_world.rs: 4 関数追加 (RFC §15.9.3 コメント)
6. collect_final_metrics: 引数追加 + 3 指標置き換え
7. テスト A1-A7

## Boy Scout 改善

- phase4_gc_survival のハードコード 0.5 → DEFAULT_FRESHNESS_HUMAN_WEIGHT 定数化
- success = 0.5 スタブにコメント明記

## レビュー方法

run-quality-checks.js + 翻訳可能性 grep

## リスク

- mean_lifecycle_score が GcEvent ベース (RFC の L(G) 平均ではない)
- 旧パス run_kw_real_simulation への波及回避
