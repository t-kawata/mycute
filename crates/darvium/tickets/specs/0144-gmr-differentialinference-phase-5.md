---
ticket_id: 144
title: GMR DifferentialInference 実装 — Phase 5 差分推論
slug: gmr-differentialinference-phase-5
status: reviewed
created_at: 2026-05-29
updated_at: 2026-05-29
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0144-gmr-differentialinference-phase-5/observation-20260529-093257.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0144-gmr-differentialinference-phase-5/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0144-gmr-differentialinference-phase-5/review.md
---
# GMR DifferentialInference 実装 — Phase 5 差分推論

## Summary

`try_gmr_diffusion` に本物の差分推論ロジックを実装し、GMR 機構がワークフロー複雑化に実質的に寄与するようにする。具体的には、helper のグラフから接続された AgentStep サブグラフ（2〜4 ノード）を抽出し、GraphPatch として helpee のグラフに適用する。DeterminismScore が抽出するサブグラフのサイズを制御する。TODO コメントで本来の完全な実装（セマンティックな差分推論、ApplicabilityScore 評価、Stage5 の 5 方向分岐）への拡張ポイントを記載する。

## Background

現在の `try_gmr_diffusion`（`src/simulation.rs:2954-2972`）は、`DeterminismScore` を計算するがその結果を実質的に使わず、乱数による確率分岐だけで `0` または `1` を返すだけのスタブである。RFC §14（GraphPatch / GraphPatchSet）および §4A.3 Mechanism 17-18 では、GMR の差分推論が DeterminismScore と ApplicabilityScore に基づいて GraphPatch を生成し、既存ワークフローに適用することで能力拡散を行うことが規定されている。現在の実装ではこの機構が完全に欠落している。

### AgentStep 1 つ追加の問題点

元の spec では「AgentStep ノードを 1 つ追加する」としていたが、1 ノード追加では複雑化への寄与が小さすぎる（#143 のグラフコピーが一度に 5〜11 ノード伝搬するのに対し、GMR が 1 ノードでは無視できる）。また、単一ノードの追加は「差分推論」の概念に合わない — 差分推論とは helper と helpee のグラフの構造的な差異から意味のあるサブグラフを抽出する操作である。

### 設計方針: サブグラフ抽出 + GraphPatch 適用

本実装では、以下の方針で GMR 差分推論をシミュレーションスコープで実装する：

1. helper の AgentStep ノードからランダムに開始ノードを選ぶ
2. BFS で接続された AgentStep ノードを 2〜4 ノード収集し、サブグラフを構成する
3. サブグラフのノードとエッジから `GraphPatch`（`AddNode` + `AddEdge` 操作列）を構築する
4. `apply_patch_atomic` で helpee のグラフに適用する
5. DeterminismScore が抽出サイズを制御する（高 determinism → 多ノード、低 determinism → 少ノードまたはスキップ）

これにより GMR 1 イベントあたり 2〜4 ノードの追加が可能になり、複雑化に実質的に寄与する。

## Scope

- helper のグラフから **接続されたサブグラフ（2〜4 AgentStep ノード + エッジ）** を抽出するロジックを実装する
- 抽出したサブグラフを `GraphPatch`（`PatchOperation::AddNode` + `PatchOperation::AddEdge`）に変換し、`apply_patch_atomic` で helpee のグラフに適用する
- `DeterminismScore` の計算結果を基に、抽出サイズを動的に決定する（高 determinism → 多ノード、低 determinism → 少ノード or スキップ）
- `_helper_id` / `_helpee_id` のアンダースコアプリフィックスを削除し、実際に使用する
- 呼び出し元で戻り値を破棄せず、実際の追加ノード数を返すように修正する
- TODO コメントで本来の完全な実装（セマンティック差分推論、ApplicabilityScore による Stage5 5 方向分岐）への拡張ポイントを記載する

## Non-scope

- セマンティックな差分推論（LLM ベースの差分解釈） — TODO で拡張ポイントを示す
- ApplicabilityScore による Stage5 5 方向分岐（REUSE/PATCH/COMPOSE/NEW/ABORT） — TODO
- サブグラフ抽出戦略の高度化（構造的重要度・エッジ密度に基づく選択）
- HELP 成功時ワークフロー伝搬（チケット #143）
- SearchWorkflow の変更（チケット #142）

## Investigation

### 証拠 1: try_gmr_diffusion が実質的なスタブ

`src/simulation.rs:2954-2972`（Spec 修正時点）：
```rust
fn try_gmr_diffusion(ctx: &mut SimulationContext, _helper_id: PersonId, _helpee_id: PersonId) -> usize {
    let det_values: Vec<f64> = (0..5).map(|_| ctx.rng.random::<f64>() * 0.5 + 0.5).collect();
    let det_score = DeterminismScore::compute(&det_values, SOFT_MIN_BETA);
    if det_score > DETERMINISM_THRESHOLD && ctx.rng.random::<f64>() < GMR_DIFFUSION_PROBABILITY {
        1
    } else {
        0
    }
}
```

引数の `_helper_id` / `_helpee_id` は未使用。乱数で DeterminismScore をでっち上げ、閾値判定と確率分岐のみで `1` を返す。実際のワークフロー操作は一切行われない。

### 証拠 2: 呼び出し元と戻り値の使われ方

`phase5_capability_diffusion`（2906行目）から GMR 有効時に呼ばれる：
```rust
if ctx.use_gmr {
    let _ = try_gmr_diffusion(ctx, helper_id, helpee_id);
}
```

戻り値 `usize` は破棄されている（`let _ =`）。加えて、`phase5_capability_diffusion` の戻り値 `diffusions` も呼び出し元（`run_kw_real_simulation`）で使われているか確認が必要。もし破棄されている場合、GMR の効果がシミュレーション結果に一切反映されない。

### 証拠 3: 単一 AgentStep ノード追加の不十分性

#143 の観測結果（tickets/context/0143-help-phase-5/observation-20260529-091631.md）から、HELP 成功時のグラフコピーは一度に 5〜11 ノードを伝搬することが確認されている。これに対して AgentStep 1 ノード追加は桁違いに小さく、GMR 機構の存在意義が希薄化する。

RFC §4A.3 Mechanism 17（Differential Inference）は「既存 WorkflowGraph からの微小変異で新しい WorkflowGraph を生成する」と定義されており、「微小変異」は 1 ノードではなく構造的な差異（複数ノード + エッジ）の抽出と適用を意図している。

### 参照観察レポート

- tickets/context/0141-compose/observation-20260528-162907.md — GMR 機構が未実装であることが示唆されている
- tickets/context/0143-help-phase-5/observation-20260529-091631.md — グラフコピーが 5〜11 ノード伝搬することを確認。GMR はこれを補完する形で 2〜4 ノードの構造的追加を行う

## Test Plan

### 不変条件テスト

1. **T1: try_gmr_diffusion が helpee のグラフに複数ノード（2〜4）を追加する** — 十分なノード数を持つ helper グラフで実行後、helpee のノード数が 2 以上増加していることを確認
2. **T2: DeterminismScore が抽出サイズを制御する** — 高い det_values（高 determinism）と低い det_values（低 determinism）で追加ノード数が異なることを確認（高 determinism で多ノード、低 determinism で少ノードまたは 0）
3. **T3: DAG 性が維持される** — サブグラフ追加後もグラフが DAG であることを `toposort` で確認（`apply_patch_atomic` が保証）
4. **T4: helper のグラフが空でもパニックしない** — 空グラフの helper でもクラッシュせず 0 を返す
5. **T5: サブグラフ抽出が helper の元グラフを変更しない** — 抽出後、helper のグラフが不変であることを確認
6. **T6: 既存テスト回帰なし** — `cargo test` 全パス

### 観測テスト

- **観測 1**: GMR 有効時の 1 イベントあたり追加ノード数分布（最小・最大・平均）
- **観測 2**: DeterminismScore と追加ノード数の相関
- **観測 3**: GMR 拡散発生率（全 HELP 成功中の何%で GMR が発動するか）

## 計装方法・観測対象

### 計装方法

- `try_gmr_diffusion` 内にサブグラフ抽出と適用時の `println!` 計装
  - 抽出ノード数、適用ノード数、DeterminismScore 値を出力
- 固定シード `StdRng::seed_from_u64(12345)` 使用

### 観測対象

- GMR 拡散発生率（全 HELP 成功中の何%で GMR が追加ノードを生成するか）
- GMR 1 イベントあたりの追加ノード数（最小・最大・平均・分布）
- DeterminismScore の値域と追加ノード数の相関
- サブグラフ抽出成功率（helper に十分な AgentStep ノードがある割合）

### 較正計画

本チケットでは新たな較正パラメータは導入しない。DeterminismScore の閾値（`DETERMINISM_THRESHOLD`）など既存定数の調整は観測後に判断する。

## Boy Scout Rule — 翻訳可能性計画

- `_helper_id` / `_helpee_id` のアンダースコアプリフィックスを削除（実際に使用するため）
- `try_gmr_diffusion` の戻り値を呼び出し元で活用するよう修正
- `try_gmr_diffusion` 内の `det_values` 乱数生成を、実際のグラフ構造に基づく DeterminismScore 計算に置き換える
- サブグラフ抽出ロジックは `extract_connected_subgraph` のような名前付き関数に分離し、翻訳可能性を確保する

## Acceptance Criteria

- [ ] try_gmr_diffusion が helper のグラフから接続サブグラフ（2〜4 ノード + エッジ）を抽出する
- [ ] 抽出したサブグラフが GraphPatch（AddNode + AddEdge）経由で helpee に適用される
- [ ] DeterminismScore が抽出サイズを制御している（高 determinism → 多ノード、低 determinism → 少ノード/スキップ）
- [ ] `_helper_id` / `_helpee_id` が実際に使用されている（アンダースコア削除）
- [ ] 呼び出し元で戻り値が破棄されず、追加ノード数が伝播される
- [ ] ノード追加後も DAG 性が維持される（T3）
- [ ] T1〜T6 の不変条件テストが全通過している
- [ ] 観測テストで 1 イベントあたり平均 2 ノード以上の追加が確認できる

## Notes

### 成果物

- 計画: context/0144-gmr-differentialinference-phase-5/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0144-gmr-differentialinference-phase-5/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0144-gmr-differentialinference-phase-5/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0144-gmr-differentialinference-phase-5/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
