# 変更したファイル一覧と実装内容の概要

## 変更ファイル

### src/trust.rs
- `MemoizedGraph` に `cached_node_count: usize` フィールド追加
- `new()` `new_with_position()` 両コンストラクタで 0 初期化

### src/simulation.rs
- `compute_all_node_count` の import を削除（使用しなくなったため）
- `build_node_created_event`: `compute_all_node_count` → `person.cached_node_count` に置換
- `build_clock_advanced_event`: 同上 + 村密度計算ブロック全体を削除 + `village_densities` ペイロード削除
- `try_gmr_diffusion`: 出生後のグラフ代入直後に `cached_node_count = nodes.len()` を設定
- `propose_subgraph_and_accept`: 同上
- `run_self_refinement_for_population`: 自己抽象化による新個体登録時に `cached_node_count` を設定
- `build_node_created_event` / `build_clock_advanced_event`: `store` 引数を `_store` に変更（未使用化対応）

### src/store/coordinator.rs
- `GraphStoreCoordinator::load_memoized_graph` の `MemoizedGraph` 初期化子に `cached_node_count: 0` 追加

### web/cube/observation/script.js
- `computeVillageDensities` 関数を削除（サーバー側の密度計算と二重であり、かつフロントエンドはサーバーの密度を使用しない）
- `polygonArea` 関数を削除（computeVillageDensities専用）
- `updateStatsPanel` 内の densityList 更新コードを削除
- `clearVisualization` 内の densityList クリアコードを削除

### web/cube/observation/index.html
- `<div id="densityList">` を削除

## 未実施（将来チケット）

- `alive_ids()` の呼び出し集約（各tick先頭で1回 → 全フェーズに引数で渡す）はスコープ変更により本チケットでは実施せず
