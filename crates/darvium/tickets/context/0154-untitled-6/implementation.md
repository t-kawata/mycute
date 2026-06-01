# 実装サマリ: #154 評判再計算の間隔設定可能化

## 変更ファイル一覧

| # | ファイル | 種別 | 内容 |
|---|---------|------|------|
| 1 | src/simulation.rs | 変更 | `ReciprocitySimulatorConfig.reputation_recompute_interval: u64 = 1` 追加 |
| 2 | src/simulation.rs | 変更 | 3箇所の Phase 3.5 呼び出しを `tick % interval == 0` で条件分岐 |
| 3 | src/server.rs | 変更 | 設定パースに `reputation_recompute_interval` 追加 (val.max(1) クランプ) |
| 4 | web/cube/observation/index.html | 変更 | スライダー追加 |
| 5 | web/cube/observation/script.js | 変更 | start コマンドに値を含める |

## 実装内容
- k-means (#152) と同一パターンの間引き制御
- 3箇所の run_*_simulation 関数すべてに対応
- Default: 1（従来動作と完全互換）
- `val.max(1)` で 0 指定防止

## 検証結果
- cargo test: ✅ 1390 passed, 0 failed, 71 ignored
- cargo check: ✅ PASS
