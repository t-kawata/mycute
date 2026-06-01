---
ticket_id: 142
title: SearchWorkflow 複雑化 — 出生時ワークフロー生成パスへの複雑性向上
slug: searchworkflow
status: reviewed
created_at: 2026-05-29
updated_at: 2026-05-29
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0142-searchworkflow/plan.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0142-searchworkflow/implementation.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0142-searchworkflow/observation-20260529-090248.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0142-searchworkflow/review.md
---
# SearchWorkflow 複雑化 — 出生時ワークフロー生成パスへの複雑性向上

## Summary

出生時に SearchWorkflow を経由して生成されるワークフローが、世代を経るにつれて複雑化（ノード数増加）するよう、4 つの生成パス（PatchExisting / ComposeExisting / Differential Mutation / GenerateNew）を修正する。

## Background

シミュレーション観測の結果、ワークフローあたりのノード数は初期生成時から殆ど増加せず、max=9 程度に留まっている。RFC §4A.3 の設計では、3 つの人口増加メカニズム（COMPOSE / NEW / Differential Inference）が世代間でワークフローを複雑化するはずだが、現状は以下の理由で複雑化が機能していない：

1. `PatchExisting` は SearchWorkflow が決してこの outcome を返さないため、`generate_workflow_for_child` の対応アームは死にコード
2. `ComposeExisting` は 2 候補のみの合成でノード数の増加幅が小さい
3. `generate_differential_mutation` の add_node 確率は 20% と低く、「基本的に親より複雑になる」という前提に反する
4. `generate_workflow_for_child` の全フォールバックが `complexity=0`（1 ノード）を渡す

## Scope

以下の 4 項目を実装する：

1. **PatchExisting 経路の有効化**: SearchWorkflow が適切な条件下で `PatchExisting` を返す経路を FSM に追加する
2. **ComposeExisting の複雑化保証**: 合成結果のノード数が親グラフより増加することを保証する
3. **Differential Mutation の複雑化確率向上**: `generate_differential_mutation` の add_node 確率を引き上げる
4. **GenerateNew フォールバックの動的複雑度**: フォールバック時の complexity を tick 数に応じて増加させる

## Non-scope

- HELP 成功時のワークフロー伝搬（チケット #143）
- GMR DifferentialInference（チケット #144）
- Self-Refinement 閾値の変更
- 検索アルゴリズム自体の再設計

## Investigation

### 証拠 1: SearchWorkflow::execute() が PatchExisting を返さない

`src/search_workflow.rs:85-135` の `execute()` は以下の分岐のみ：
- 候補なし → `propose_new(None)` → `GenerateNew`
- `best_score >= 0.50` → `finalize_reuse()` → `ReuseExisting`
- 2 候補以上かつ 2位/1位 > 0.70 → `try_compose()` → `ComposeExisting`
- 上記以外 → `propose_new(Some(base))` → `GenerateNew`

`PatchExisting`, `AbortSearch`, `NeedsHumanReview` はこの FSM から決して返されない。`SearchOutcome` enum には定義されているが、到達不能。

### 証拠 2: generate_workflow_for_child のフォールバック

`src/simulation.rs:2206-2247` — 以下のアームが全て `generate_new_workflow(mission, rng, 0)` にフォールバック：
- `PatchExisting` の `apply_patch_atomic` 失敗時 (2224行目)
- `PatchExisting` の `registry.resolve` 失敗時 (2227行目)
- `ComposeExisting` の `registry.resolve` 失敗時 (2237行目)
- `ComposeExisting` の `component_graph_ids.last() == None` 時 (2239行目)
- `AbortSearch / NeedsHumanReview / Err` (2244行目)

`complexity=0` は単一 AgentStep ノードのみのグラフを生成する。

### 証拠 3: Differential Mutation の低い add_node 確率

`src/workflow_generation.rs:88-97` の確率分布：
- 30%: update_prompt（ノード数不変）
- 25%: add_edge（ノード数不変）
- **20%: add_node（ノード数+1）**
- 15%: replace_node（ノード数不変）
- 10%: remove_edge（ノード数不変）

`DIFFERENTIAL_MUTATION_MAX_ATTEMPTS = 10` 回試行するが、各試行で独立した 20% の確率でしか add_node は選択されない。

### 証拠 4: COMPOSE_CANDIDATE_COUNT の制限

`src/search_workflow.rs:26` — `const COMPOSE_CANDIDATE_COUNT: usize = 2;`
`src/search_workflow.rs:119-128` — `try_compose()` は最大 2 候補しか合成しない。
`compose_workflows()` (`src/composition.rs:151`) は 2 つのグラフを合成するのみで、サイズの増加は最大でも親グラフ 2 つのノード数の合算に過ぎない。

### 証拠 5: GENERATION_COMPLEXITY の静的特性

`src/search_workflow.rs:35` — `const GENERATION_COMPLEXITY: usize = 2;`
この値はコンパイル時定数であり、tick 経過や世代に応じて変化しない。

### 参照観察レポート

- tickets/context/0141-compose/observation-20260528-162907.md — 初期人口ノード数分布（min=4, max=9, avg=5.80）。全パイプライン接続確認。

## Test Plan

### 不変条件テスト（`assert!`）

1. **T1: PatchExisting 経路の到達性** — SearchWorkflow が特定条件で PatchExisting を返すことを確認
2. **T2: ComposeExisting のノード数増加** — 2 つのグラフ合成結果が最小ノード数を満たすことを確認
3. **T3: Differential Mutation の add_node 確率** — N 回の試行で add_node が発生する割合が閾値以上であることを確認（統計的検定）
4. **T4: GenerateNew フォールバックの動的複雑度** — 異なる tick 値で complexity が適切に変化することを確認
5. **T5: 既存テスト回帰なし** — `cargo test` が全パス

### 観測テスト（`println!` + `--nocapture`）

- **観測 1**: 複数出生（n >= 100）のノード数分布。平均・最大・分位数を出力
- **観測 2**: PatchExisting / ComposeExisting / Differential Mutation / GenerateNew の各経路の選択比率
- **観測 3**: 長期シミュレーション（200 tick）でのノード数推移

## 計装方法・観測対象

### 計装方法

- `src/simulation.rs` の `generate_workflow_for_child` 内に各 outcome の選択を `println!` で計装
- `StdRng::seed_from_u64(12345)` 固定シードで再現性を保証
- 100 回以上の出生試行でノード数分布を集計

### 観測対象

- 各出生イベントの SearchOutcome 種別（REUSE / PATCH / COMPOSE / NEW）
- 生成されたワークフローのノード数
- tick 経過に伴う平均ノード数の推移

### 較正計画

本チケットでは新たな較正パラメータは導入しない。既存定数の変更値を観測テストで確認する。

## Boy Scout Rule — 翻訳可能性計画

- `generate_workflow_for_child` の match 式: 死にアーム（`AbortSearch`, `NeedsHumanReview`）を削除または早期リターンに変更
- `generate_differential_mutation` のマジックナンバー確率（`0.30`, `0.55`, `0.75`, `0.90`）を名前付き定数に抽出
- `COMPOSE_CANDIDATE_COUNT` の定数名をロールを表現する名前に変更（候補）

## Acceptance Criteria

- [ ] PatchExisting が SearchWorkflow から返される経路が追加されている
- [ ] ComposeExisting で合成結果のノード数保証が機能している
- [ ] Differential Mutation の add_node 確率が向上している
- [ ] GenerateNew フォールバックの complexity が動的に変化する
- [ ] 既存テストが全通過している
- [ ] 観測テストでノード数の増加が確認できる

## Notes

### 成果物

- 計画: context/0142-searchworkflow/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0142-searchworkflow/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0142-searchworkflow/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0142-searchworkflow/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
