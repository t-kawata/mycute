# 計画: ワークフロー複雑化メカニズム完全活性化

## RFC 既存実装状態検証

**RFC §10.2 DeterminismScore D(G)** (gmr.rs:185-214):
- 数式は `DeterminismScore::compute()` として実装済み
- 現状の問題: 入力値が乱数ベース（simulation.rs:3103-3105）
- 評価: DeterminismScore 関数自体は ✅ 一致。呼び出し側の入力が ❌ 乖離（乱数 vs 実グラフ値）

**RFC §4A.5 Mechanism 25-26 (HELP Success)**:
- RFC は「支援完了と能力伝搬」を定義するが、具体的なグラフコピーアルゴリズムは規定せず
- 提案+確率的受入モデルへの置換は RFC と矛盾しない

**評価サマリ**: 実装上の乖離は DeterminismScore への入力が乱数である点のみ。本チケットで修正する。

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|---------|------|------|
| src/constants.rs | 変更 | WORKFLOW_GENERATION_MAX_COMPLEXITY: 2→4。新規定数追加 |
| src/simulation.rs | 変更 | use_gmr config追加、3パスのハードコードfalse削除、_diffusions修正、copy_graph_if_more_complex→propose_subgraph_and_accept置換、DeterminismScoreグラフ構造ベース化 |
| src/search_workflow.rs | 変更 | try_patch_existingを複数AddNode対応に拡張 |
| src/server.rs | 変更（最小） | parse_configにuse_gmrのJSONパース追加 |

## 計装・観測の実装計画

**テストコード**: src/simulation.rsのmod tests内に新規テスト関数を追加

**観測出力**: println!パターンで[PHASE5]/[GMR]/[BIRTH]タグ付き出力

**較正定数**: HELP_PROPOSAL_ACCEPT_BASE=0.30, HELP_PROPOSAL_MIN_NODES=2, HELP_PROPOSAL_MAX_NODES=4, WORKFLOW_GENERATION_MAX_COMPLEXITY=2→4

## 実装手順

Step A: constants.rsの変更（定数変更+追加）
Step B: ReciprocitySimulatorConfigにuse_gmr追加 + 全パス修正
Step C: _diffusions修正
Step D: DeterminismScoreグラフ構造ベース化
Step E: try_patch_existing複数ノード対応
Step F: HELP提案+確率的受入モデル
Step G: テスト実装（T1-T8 + 観測テスト）
Step H: cargo test検証

## 物理的レビュー方法

run-quality-checks.js + 翻訳可能性grep

## リスク

- copy_graph_if_more_complex削除で#143テストに影響。代替テストでカバー
- ReciprocitySimulatorConfigフィールド追加はDefaultトレイト経由で自動フォロー
