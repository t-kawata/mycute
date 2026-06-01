# 計画: #153 generate_workflow_for_childの検索スキップ最適化

## 要件
ReciprocitySimulatorConfig.skip_child_search を追加し、true の場合に generate_workflow_for_child 内で SearchWorkflow をスキップして直接 generate_new_workflow を呼ぶ。

## 変更ファイル一覧
| ファイル | 種別 | 内容 |
|---------|------|------|
| src/constants.rs | 追加 | CHILD_MISSION_RANDOM_SUFFIX |
| src/simulation.rs | 変更 | Config + Context + generate_workflow_for_child分岐 |
| src/server.rs | 変更 | 設定パース |
| web/cube/observation/index.html | 変更 | チェックボックス |
| web/cube/observation/script.js | 変更 | startコマンド |

## 実装手順
1. constants.rs に定数追加
2. Config + Context に skip_child_search 追加
3. generate_workflow_for_child 分岐
4. run_kw_real_simulation で Config→Context コピー
5. マジックナンバー定数化
6. server.rs + フロントエンド
7. 観測テスト追加
8. cargo test 確認

## レビュー方法
cargo check → cargo test → cargo clippy -- -D warnings → cargo fmt --check

## リスク
- スキップでグラフ品質低下（T3で観測）
- 後方互換性（false デフォルトで維持）
