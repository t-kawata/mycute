# 計画: M1.76-KW-MTR-C (ticket #123)

## 要件
- SimulationContext に total_gc_collections, total_help_attempts, total_help_successes 追加
- シミュレーションループで phase3/phase4 戻り値を各カウンターに加算
- collect_final_metrics の 2 ハードコードを実測値で置き換え

## 変更ファイル一覧
| ファイル | 種別 | 内容 |
| simulation.rs | 修正 | SimulationContext フィールド追加 + 初期化 + ループカウンター |
| kind_world.rs | 修正 | compute_execution_success_rate / compute_cost_efficiency + collect_final_metrics + C1-C7 テスト |

## 実装手順
1. simulation.rs: SimulationContext に 3 フィールド追加 + new() で 0 初期化
2. simulation.rs: run_evaluation_simulation ループにカウンター累積 (3 行)
3. kind_world.rs: compute_execution_success_rate と compute_cost_efficiency 実装
4. kind_world.rs: collect_final_metrics の cost_efficiency/execution_success_rate 置き換え
5. kind_world.rs: C1-C7 テスト追加
6. cargo build + cargo test + cargo clippy

## リスク
- 旧パス compute_cost_efficiency (kind_world.rs:782) との名前衝突 → 新関数は非公開関数として同ファイルトップレベルに配置
- total_gc_collections が usize → u64 キャスト
