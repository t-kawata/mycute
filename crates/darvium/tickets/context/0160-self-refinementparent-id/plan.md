# 計画: 本番 self-refinement での parent_id 設定漏れ修正

## 変更ファイル
| ファイル | 内容 |
|---------|------|
| src/self_refinement.rs | register_abstracted_subworkflow で子グラフの parent_id 設定 |
| src/lib.rs | parent_alive_map の ID パースを hex 対応に修正 |

## 実装手順
1. lib.rs: parent_alive_map の ID パースを strip_prefix + from_str_radix(16) に修正
2. self_refinement.rs: register_graph_only 後に resolve_mut で parent_id 設定
3. cargo check → cargo test
