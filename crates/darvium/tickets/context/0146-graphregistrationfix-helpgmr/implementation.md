# 変更したファイル一覧と実装内容の概要

## 修正内容（監査後）

### src/simulation.rs
1. **Import追加**: `WorkflowGraphId` を types から追加 import
2. **run_self_refinement_for_population**: on_new_individual コールバックを接続
   - グラフを std::mem::take で一時取り出し、借用分割で registry/population を別フィールドとして扱う
   - コールバック内で新個体を population に push（親の属性継承付き）
   - 実行後グラフを書き戻し
3. **propose_subgraph_and_accept**: register_graph_only 呼び出しを追加
4. **try_gmr_diffusion**: register_graph_only 呼び出しを追加
5. **unwrap_or(0) → unwrap_or_else**: 2箇所のエラーケースに eprintln! 警告を追加
6. **O3 観測テスト追加**: 統合シミュレーションフルスタックテスト

## ビルド・テスト結果
- cargo check: 成功（1 warning: propose_subgraph_and_accept はテスト専用関数のため無害）
- cargo test: 1358 passed, 0 failed, 65 ignored（統合テスト含む全件）
