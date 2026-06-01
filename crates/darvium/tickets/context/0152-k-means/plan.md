# 計画: チケット #152 k-means実行間隔の設定可能化

## 要件
ReciprocitySimulatorConfig に village_recluster_interval を追加し、k-means クラスタリングの実行間隔を制御可能にする。

## 変更ファイル一覧

| # | ファイル | 種別 | 内容 |
|---|---------|------|------|
| 1 | src/simulation.rs | 要変更 | Config + Context 拡張、Voronoi 関数追加、ループ分岐、テスト更新 |
| 2 | src/server.rs | 要変更 | 設定パース追加 |
| 3 | src/constants.rs | 追加 | KMEANS_PLUSPLUS_SEED, KMEANS_MAX_ITERATIONS |
| 4 | web/cube/observation/index.html | 要変更 | スライダー追加 |
| 5 | web/cube/observation/script.js | 要変更 | start コマンドに値を含める |

## 実装手順

1. constants.rs に定数追加
2. euclidean_distance_3d(), compute_village_count() 抽出
3. SimulationContext に last_centroids, kmeans_distance_computations 追加
4. ReciprocitySimulatorConfig に village_recluster_interval 追加
5. phase2_village_clustering 改造（centroid 保存）
6. phase2_voronoi_assign() 新規作成
7. 4箇所のループ分岐
8. 9箇所のテスト設定更新
9. server.rs 設定パース追加
10. フロントエンド UI
11. cargo test 確認

## レビュー方法

cargo check → cargo test → cargo clippy -- -D warnings → cargo fmt --check

## リスク

- テスト設定更新漏れ（コンパイルエラーで発覚）
- Voronoi 空の村（最低1人保証ロジック）
