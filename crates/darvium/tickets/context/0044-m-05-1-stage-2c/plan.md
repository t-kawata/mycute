# 計画: M-0.5-1 Stage 2c 統合・重複排除器

## 要件
- `merge_and_deduplicate_candidates` 純粋関数の実装（RFC §12.2 Stage 2c）
- セマンティック検索結果（GraphStore）と構造検索結果（MetadataStore）を統合
- `workflow_id` による重複排除（HashMap 集約）
- 重複時のスコア選択: 高い方の `blended_score` を残す（最大値保存則）
- provenance 連結（両ストア由来を保持）
- 入力非破壊性
- テスト: T1〜T9 + OTS-1（カイ二乗検定）+ OTS-2（最大値保存則）

## 変更ファイル一覧
| ファイル | 種別 | 内容 |
|---------|------|------|
| src/store/mod.rs | 追加 | `merge_and_deduplicate_candidates` 関数 + 全テスト |
| src/lib.rs | 修正 | `merge_and_deduplicate_candidates` を re-export |

## 実装手順
1. src/store/mod.rs: `merge_and_deduplicate_candidates` 実装
2. src/lib.rs: re-export 追加
3. テスト実装（T1〜T9 + OTS-1/2）
4. cargo test → cargo clippy → cargo fmt

## 物理的レビュー方法
- cargo test -- --nocapture
- cargo clippy -- -D warnings
- cargo fmt -- --check
- 確認: 入力リストの非破壊性（clone 使用の確認）
- 確認: provenance 連結ロジックの正確性

## リスク
- 純粋関数のため他モジュールへの影響は限定的
- 大量候補（2,000件）でのパフォーマンスは OTS で確認
