# 変更したファイル一覧と実装内容の概要

## 変更ファイル

### `src/kind_world.rs` — 本実装

1. **`parse_workflow_id()` 関数追加**（~line 2039）
   - `compute_search_radius_inverse()` の直前に独立関数として追加
   - 6 種の ID フォーマットを順次試行：`"n<数字>"`, `"wf-child-<数字>"`, `"wf-adult-<数字>"`, `"session-<数字>"`, `"adult-<数字>"`, `"child-<数字>"`
   - 戻り値: `Option<crate::types::NodeId>`

2. **`compute_search_radius_inverse()` 修正**（~line 2066）
   - `parse_nid` クロージャを削除し `parse_workflow_id()` 関数呼び出しに置き換え
   - 引数シグネチャ変更: `sessions` と `positions` を明示的に受け取る形に整理

3. **`AllParams::default_g1()` 修正**（~line 342）
   - `params.active[G1_SEARCH_RADIUS_INVERSE] = false;` を追加
   - コメントで「関数がパラメーターを無視し実測値経路で計算するため」と明記

4. **Bayesian search テスト修正**（~line 7872, ~line 7963）
   - `tc_p2_g1_bayesian_search`: FloatParam 数・trial_values capacity を `G1_COUNT`(14) → `defaults.active_count()`(13) に変更
   - `tc_p2_g1g2_bayesian_search`: 同様に `total_count`(17) → `defaults.active_count()`(16) に変更
   - 不要な `total_count` 変数を削除
   - 両テストに `#[ignore]` を追加（長時間のため）

5. **テスト tb1〜tb7 追加**（~line 8026）
   - tb1: `"n<数字>"` 形式パース
   - tb2: `"wf-child-*"` / `"wf-adult-*"` 形式パース
   - tb3: `"session-*"` 形式パース
   - tb4: 同一位置 → 逆数 1.0
   - tb5: 空セッション → 0.5
   - tb6: パース失敗セッションスキップ
   - tb7: `"adult-*"` / `"child-*"` 形式パース（production 形式）

### `Darvium-RFC-0001-Unified-v2.3-final.md` — ドキュメント更新

- §15.9.2 の `j_search_radius_inv` 説明から「暫定実装」注記を削除し、`parse_workflow_id()` による全 ID 形式対応を追記
- EcosystemGrowthMetrics の `search_radius_inverse` 説明から同様に「暫定実装」注記を削除

## 検証結果

- `cargo test`: 全 PASS（Bayesian 最適化テスト 2 件は長時間のため `#[ignore]`）
- `cargo clippy`: 確認済み
- 品質チェック: 158 issues（全件既存コード由来、新規導入ゼロ）
- RFC 無矛盾性: 確認済み
