# WIRE-D 実装計画

## 要件再確認

`offer_help_sessions()` に村内/村外の距離を考慮した遠隔探索機構を導入する。村内ノードには通常確率（`OFFER_HELP_BASE + epsilon_remote`）、村外ノードには epsilon_remote 確率のみ（探索的）で offer する。`child_need` を村内の子供割合から動的に計算する。

## RFC §4A.5 F-13 既存実装状態検証

`ReciprocityLifecyclePolicy`（event.rs:596-603）:

| フィールド | RFC 記号 | 現行コードの型 | 状態 |
|---|---|---|---|
| `epsilon_remote_base` | ε₀ | f32 | ✅ 一致 |
| `epsilon_remote_max` | ε_max | f32 | ✅ 一致 |
| `epsilon_remote_need_coeff` | a₁ | f32 | ✅ 一致 |
| `epsilon_remote_benevolence_coeff` | a₂ | f32 | ✅ 一致 |

`compute_benevolence_aware_remote_exploration()`（reciprocity.rs:527-535）は RFC 式 (F-13) を正確に実装済み。WIRE-D ではこれを「村別」に利用する差分のみ。

**評価サマリ**: 全フィールド一致、乖離なし。

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|---|---|---|
| `src/constants.rs` | 追加 | `LOCAL_HELP_BOOST: f64 = 1.0` |
| `src/kind_world.rs` | 追加 | G5 グループ（G5_COUNT=1, G5_LOCAL_HELP_BOOST=28）, default_g1g2g4() G5, to_sim_config_g1g2g4() G5 |
| `src/simulation.rs` | 修正 | Config.local_help_boost, compute_village_mean_benevolence, compute_child_need_in_village, offer_help_sessions village_assignments 対応, run_simulation None, テスト D1-D7 |

## 計装・観測の実装計画

- **テストコード**: simulation.rs の `mod tests` に test_d1〜test_d6 追加
- **D1-D5**: 固定 seed（StdRng::seed_from_u64(12345)）の assert テスト
- **D6**: 純粋関数の境界値テスト
- **D7**: cargo test 全テスト通過確認
- 観測出力: println! + --nocapture で村内/村外別の offer 確率分布を出力
- D3: n >= 1,000 試行の epsilon_remote_max=0 検証

## Boy Scout 改善

- `child_need: f32 = 0.0` プレースホルダを実測値に置き換え（WIRE-A の技術的負債返済）
- `compute_village_mean_benevolence` による責務分割の明確化

## 実装手順

1. constants.rs: LOCAL_HELP_BOOST 追加
2. kind_world.rs: G5 定数追加
3. kind_world.rs: default_g1g2g4() G5 セクション
4. kind_world.rs: to_sim_config_g1g2g4() G5 伝播
5. simulation.rs: Config.local_help_boost 追加
6. simulation.rs: compute_village_mean_benevolence()
7. simulation.rs: compute_child_need_in_village()
8. simulation.rs: offer_help_sessions() シグネチャ＋ロジック変更
9. simulation.rs: run_simulation() None 呼び出し
10. テスト D1-D7
11. cargo build / test / clippy

## 物理的レビュー方法

```bash
_R=$(cat DARVIUM_PLUGIN_ROOT.md)
node "$_R/scripts/tickets/review/run-quality-checks.js" src/constants.rs src/kind_world.rs src/simulation.rs | node "$_R/scripts/tickets/review/generate-report.js"
```

翻訳可能性チェック: 関数名動詞句確認、マジックナンバー確認、デバッグ出力確認。

## リスク

- 小規模村での child_need 推定ノイズ（村人数 < 5 で全人口平均フォールバックを検討）
- phase3_help_protocol() は対象外（KW-REAL パス独立）
- village_assignments=None で後方互換維持
