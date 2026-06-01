---
ticket_id: 155
title: skip_child_search時のレジストリランダムサンプリングによる擬似成長
slug: skip-child-search
status: done
created_at: 2026-06-01
updated_at: 2026-06-01
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0155-skip-child-search/plan.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0155-skip-child-search/implementation.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0155-skip-child-search/observation-20260601-155605.md
---

# skip_child_search時のレジストリランダムサンプリングによる擬似成長

## Summary

`skip_child_search = true` では `generate_workflow_for_child` が SearchWorkflow（意味検索＋SQLite＋GED＋評価）をスキップし、毎回 `generate_new_workflow` でゼロからグラフを生成する。その結果、親のグラフのノード数が子に継承されず、世代を経てもグラフが成長しない。本チケットでは、検索をスキップしたままでもレジストリからランダムサンプリングした既存グラフを親として子に継承させることで、擬似的な成長を実現する。

## Background

`skip_child_search = false` のパスでは `SearchWorkflow::execute()` が以下を行う：
1. 意味検索で類似グラフを探索
2. `ReuseExisting` で最も適した既存グラフをクローン
3. これにより親世代のノード数が子に継承され、世代を超えてグラフが成長

`skip_child_search = true` では上記がすべてスキップされ、子は常に `generate_new_workflow`（複雑度 2〜4）の新鮮な小グラフを得る。親のノード数を引き継がないため、世代を経てもグラフサイズが停滞する。

これを解決するには SearchWorkflow（高コストな検索パイプライン）を呼ばずに、**レジストリからランダムにグラフを選んでクローンする** ことで、検索コストをかけずに成長を再現する。

## Scope

1. **`generate_workflow_for_child` の skip パス改造**: レジストリからランダムに既存グラフを選び、クローンして子に割り当てる
2. **空レジストリのフォールバック**: レジストリが空の場合は `generate_new_workflow` にフォールバック

## Non-scope

- SearchWorkflow 自体の変更（本番コード非変更を維持）
- Phase 5（能力拡散）の改造
- グラフ変異アルゴリズムの導入（クローンのみで変異なし）

## Investigation

### 物理的証拠

**証拠1**: #153 実装後の `generate_workflow_for_child`（`simulation.rs`）
```rust
if ctx.skip_child_search {
    let max_cx = crate::constants::WORKFLOW_GENERATION_MAX_COMPLEXITY;
    let complexity = ctx.rng.random_range(2..=max_cx);
    return generate_new_workflow(mission, &mut ctx.rng, complexity);
}
```
毎回 `generate_new_workflow` を呼ぶため、親のグラフ構造は一切継承されない。

**証拠2**: `skip_child_search = false` の SearchWorkflow パスでは `ReuseExisting` がグラフ成長の主因
- `search_workflow.rs` の `execute()` で semantic_search → スコア評価
- `SEARCH_FINALIZE_THRESHOLD` 以上のスコアで `finalize_reuse` が呼ばれ、レジストリ内の既存グラフがクローンされる
- これにより親世代から蓄積されたノード数が子に継承される

**証拠3**: レジストリ（`WorkflowRegistry`）はシミュレーション内で抽象化・能力拡散により常に更新されている
- Phase 3.6 の `run_self_refinement_for_population` でグラフが登録される
- Phase 5 の `phase5_capability_diffusion` で新しいグラフが追加される
- レジストリが空になることは事実上ない

### 参照観察レポート

- `tickets/context/0153-generate-workflow-for-child/observation-20260601-145905.md` — skip_child_search の実装詳細
- `tickets/context/0152-k-means/observation-20260601-142517.md` — Phase 1 が 81.4% のボトルネックだった証拠

### 参照チケット
- #153: skip_child_search の初回実装
- #154: 評判再計算の間隔設定可能化

## Test Plan

### T1: ランダムサンプリングの動作確認
- `skip_child_search = true` でシミュレーションを実行
- 子ワークフローがレジストリから取得したグラフを持つことを確認
- **正常系**: `cached_node_count` が親と同等以上であること

### T2: 空レジストリのフォールバック
- レジストリが空の場合、`generate_new_workflow` にフォールバックする
- **正常系**: 空レジストリでも Panic せず、有効なグラフが生成される

### T3: ノード数成長の観測（観測テスト）
- `skip_child_search = true/false` で同一設定のシミュレーションを実行
- 各世代の平均ノード数を比較
- **期待**: `true` でも `false` と同程度の成長曲線が得られる

### T4: 実行時間の確認（観測テスト）
- ランダムサンプリングが SearchWorkflow より大幅に高速であることを確認
- **期待**: サンプリングは O(1) であり、検索パイプラインより 100 倍以上高速

## 計装方法・観測対象

### 計装方法
- `println!` + `--nocapture` で世代ごとの平均ノード数を出力
- 固定シード `StdRng::seed_from_u64(12345)` で決定論的実行
- `ctx.registry` のエントリ数を tick ごとに計測し、レジストリが空でないことを確認

### 観測対象
- **統計量**: 世代別の平均ノード数（`cached_node_count` の平均値）
- **サンプルサイズ**: 同一設定で 3 回実行
- **期待される現象**: `skip_child_search = true` でも、世代を経るごとに `cached_node_count` が増加する

### 較正計画
- 本チケットでは較正不要（ランダムサンプリングの有無が本質）

## Implementation Plan

### 注意: この実装はシミュレーター専用の便宜的措置である

実装時にコードに記載するコメント（日本語）は以下の内容を必ず含める：

```
// ---- シミュレーター専用 疑似成長パス ----
//
// 通常パス（skip_child_search = false）では SearchWorkflow による
// Compose（複数グラフの合成）や ReuseExisting（既存グラフの再利用）が
// 世代間のグラフ成長を実現する。
//
// このパスは SearchWorkflow を完全にスキップする代わりに、
// レジストリからのランダムサンプリング + DAG 断片の追加により
// 擬似的なグラフ成長をエミュレートする。
//
// これはあくまでシミュレーションのための便宜的実装である。
// 本来の MYCUTE システムでは、ワークフローの継承は意味検索・評価・合成
// （SearchWorkflow のフルパイプライン）を経て行われるべきものであり、
// ランダムサンプリングはそのプロセスを代替しない。
```

同様のコメントを `sample_graph_from_registry` 関数にも付与する。

### Step 1: `sample_graph_from_registry` 関数追加（simulation.rs）

```rust
/// レジストリからランダムに1件のグラフをサンプリングし、クローンして返す。
///
/// # 注意
///
/// この関数はシミュレーター専用の便宜的実装である。本来の MYCUTE システムにおいて
/// 子ワークフローのグラフを決定するプロセスは SearchWorkflow による意味検索・評価・
/// 合成（Compose）を経るものであり、ランダムサンプリングはその代替ではない。
/// ...
fn sample_graph_from_registry(
    registry: &crate::workflow_registry::WorkflowRegistry,
    rng: &mut impl rand::Rng,
) -> Option<WorkflowGraph> {
    let count = registry.graph_count();
    if count == 0 { return None; }
    let idx = rng.random_range(0..count);
    registry.all_graphs().nth(idx).map(|m| m.graph.clone())
}
```

### Step 2: `generate_workflow_for_child` の skip パス改造（simulation.rs）

```rust
if ctx.skip_child_search {
    // ---- 上記「シミュレーター専用 疑似成長パス」コメント ----
    if let Some(mut child_graph) = sample_graph_from_registry(&ctx.registry, &mut ctx.rng) {
        // 変異追加: クローンした親グラフに DAG 断片を追記することで、
        // Compose 疑似効果を得る。本来の Compose は複数候補を意味的に
        // 評価・選択・結合するが、ここでは単純な追記のみ行う。
        crate::workflow_generation::generate_dag(&mut child_graph, mission, &mut ctx.rng);
        return child_graph;
    }
    // レジストリが空の場合（初期 tick など）は新規生成にフォールバック
    let max_cx = crate::constants::WORKFLOW_GENERATION_MAX_COMPLEXITY;
    let complexity = ctx.rng.random_range(2..=max_cx);
    return generate_new_workflow(mission, &mut ctx.rng, complexity);
}
```

### Step 3: `generate_dag` の公開設定変更（workflow_generation.rs）

`generate_dag` は現在 `fn`（非公開）。`pub(crate) fn generate_dag` に変更する。

## Boy Scout Rule — 翻訳可能性計画

- `generate_workflow_for_child` 内で「シミュレーター専用」「本来の実装ではない」というコメントが冒頭の早期リターン直後にあり、関数を読んだ人が即座に意図を理解できるようにする

## Acceptance Criteria

- [ ] `skip_child_search = true` でレジストリからランダムサンプリングされる
- [ ] サンプリングしたグラフに `generate_dag` で DAG 断片が追加される（成長）
- [ ] 空レジストリ時は `generate_new_workflow` にフォールバックする
- [ ] 世代を経るごとにノード数が成長する（Phase 5 との重畳効果）
- [ ] コード内に「シミュレーター専用の便宜的実装である」旨のコメントが記載されている
- [ ] 実行時間が SearchWorkflow パスより大幅に短い
- [ ] 既存テストがすべて通過する

## Notes

### 成果物

- 計画: context/0155-skip-child-search/plan.md
- 実装サマリ: context/0155-skip-child-search/implementation.md
- レビュー報告書: context/0155-skip-child-search/review.md
- 観察レポート: context/0155-skip-child-search/observation-YYYYMMDD-HHmmss.md
