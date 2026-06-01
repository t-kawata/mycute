# 計画: #154 評判再計算の間隔設定可能化

## 要件
ReciprocitySimulatorConfig.reputation_recompute_interval を追加し、
recompute_reputation_for_population の呼び出し間隔を設定可能にする。

## 変更ファイル一覧
| ファイル | 種別 | 内容 |
|---------|------|------|
| src/simulation.rs | 変更 | Config + ループ分岐 |
| src/server.rs | 変更 | 設定パース |
| web/cube/observation/index.html | 変更 | スライダー |
| web/cube/observation/script.js | 変更 | startコマンド |

## 実装手順
1. Config に reputation_recompute_interval 追加 (Default: 1)
2. 3箇所の Phase 3.5 呼び出しを条件分岐
3. server.rs 設定パース追加
4. フロントエンド UI
5. cargo test 確認

## レビュー方法
cargo check → cargo test → cargo clippy -- -D warnings

## リスク
- GC誤動作（T3で観測）
- 後方互換性（interval=1デフォルトで維持）
