# 計画: #155 skip_child_search時のレジストリランダムサンプリングによる擬似成長

## 要件
skip_child_search=true パスでレジストリからランダムサンプリング + generate_dag変異追加により擬似成長を実現

## 変更ファイル一覧
| ファイル | 種別 | 内容 |
|---------|------|------|
| src/simulation.rs | 追加 | sample_graph_from_registry関数 + コメント |
| src/simulation.rs | 変更 | generate_workflow_for_child のskipパス |
| src/workflow_generation.rs | 変更 | generate_dag を pub(crate) に |

## 実装手順
1. generate_dag を pub(crate) に
2. sample_graph_from_registry 追加（シミュレーター専用コメント付き）
3. skip_child_searchパスを clone + generate_dag に変更

## レビュー方法
cargo check → cargo test → cargo clippy -- -D warnings
