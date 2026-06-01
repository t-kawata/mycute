---
ticket_id: 153
title: generate_workflow_for_childの検索スキップ最適化
slug: generate-workflow-for-child
status: done
created_at: 2026-06-01
updated_at: 2026-06-01
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0153-generate-workflow-for-child/plan.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0153-generate-workflow-for-child/implementation.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0153-generate-workflow-for-child/observation-20260601-145905.md
---

# generate_workflow_for_childの検索スキップ最適化

## Summary

`phase1_population_growth` が各出生子に対して `generate_workflow_for_child` → `SearchWorkflow::execute()` を実行し、シミュレーション全時間の 81% を占めている。出生時のミッション文字列は毎回ランダムユニークであり、意味検索がほぼ常に無駄になる。SearchWorkflow のフルパイプラインをスキップし、直接 `generate_new_workflow` にフォールバックするモードを追加する。

## Background

`simulation.rs:2587` の `generate_workflow_for_child` は以下を行う：
1. `SearchWorkflow::execute()` でミッション文字列を意味検索
2. 検索結果スコアに応じて Reuse / Compose / PatchExisting / GenerateNew の 4 択

しかし、出生時のミッション文字列は `"child-of-{parent_id}-tick-{tick}-{random}"` という毎回ユニークなランダム文字列であり、既存の知識ベースにヒットする確率はほぼゼロ。実際の挙動はほぼ常に `GenerateNew` にフォールバックしている。

プロファイル結果（#152 観測より）：
```
Phase 1 (人口成長): 7,306,085 µs / 8,975,919 µs = 81.4%
```

## Scope

1. **`ReciprocitySimulatorConfig.skip_child_search` 追加**: `bool`、デフォルト `false`（現行動作維持）
2. **`generate_workflow_for_child` 分岐**: `skip_child_search=true` の場合、`SearchWorkflow::execute()` をスキップして直接 `generate_new_workflow` を呼ぶ
3. **フロントエンド連携**: トグルスイッチ + server.rs config parsing

## Non-scope

- SearchWorkflow 自体の高速化（別チケット）
- グラフ生成アルゴリズムの変更
- 出生率（child_ratio）の調整

## Investigation

### 物理的証拠

**証拠1**: `phase1_population_growth`（`simulation.rs:2694`）が各出生子に対して `generate_workflow_for_child` を呼ぶ
- `simulation.rs:2718`:
  ```rust
  let mission = format!(
      "child-of-{}-tick-{}-{}",
      parent_id, ctx.tick, ctx.rng.random_range(0..10000)
  );
  let graph = generate_workflow_for_child(ctx, &mission);
  ```
- `ctx.rng.random_range(0..10000)` により毎回異なるミッション文字列

**証拠2**: `generate_workflow_for_child`（`simulation.rs:2587`）は `SearchWorkflow::execute()` を呼ぶ
- `search_workflow.rs:86`: semantic_search → Evaluate → Finalize/Compose/Patch のパイプライン
- 実際には `SearchOutcome::GenerateNew` に常時フォールバック

**証拠3**: `generate_new_workflow`（`workflow_generation.rs`）は単純なランダムグラフ生成
- `SearchWorkflow::execute()` を通らずとも直接呼べる
- `generate_workflow_for_child` の `_ => { generate_new_workflow(...) }` が実質的な唯一の実行パス

**証拠4**: プロファイルで Phase 1 が 81.4% を占める（#152 観測より）
- 人口 1430 時点で Phase 1 累積時間 7.3秒
- Phase 2-5 の合計よりも約 4.3 倍大きい

### 参照観察レポート

- `tickets/context/0152-k-means/observation-20260601-142517.md` — Phase 1 が 81.4%、Phase 3.5 が 15.2% であることのプロファイル証拠を含む

### 参照チケット
- #152: k-means 実行間隔の設定可能化（本調査の元になったチケット）
- #154: 評判再計算の間隔設定可能化（同時発行の姉妹チケット）

## Test Plan

### T1: `skip_child_search=false` で後方互換性
- デフォルト（`false`）でシミュレーションを実行
- `skip_child_search` なし（従来コード）と同一の結果になることを検証
- **正常系**: 同一 seed → 同一のグラフ構造

### T2: `skip_child_search=true` で直接生成
- `skip_child_search=true` でシミュレーションを実行
- 子ワークフローがすべて `generate_new_workflow` 経由で生成されることを確認
- **正常系**: 全生存者がいずれかのグラフを持つ（Graph が空でない）
- **正常系**: HELP プロトコルが正常動作する

### T3: グラフ多様性の観測（観測テスト）
- `skip_child_search=true` で生成されたグラフが従来と同等の多様性を持つか
- **観測対象**: グラフのノード数分布、エッジ数分布
- **期待**: 生成方法が変わっても十分な多様性が維持される

### T4: 実行時間の削減確認（観測テスト）
- `skip_child_search=true/false` で同一設定のシミュレーションを実行
- Phase 1 の実行時間を比較
- **期待**: `skip_child_search=true` で Phase 1 の時間が 90% 以上削減される

## 計装方法・観測対象

### 計装方法
- `println!` + `--nocapture` で Phase 1 実行時間を出力
- 固定シード `StdRng::seed_from_u64(12345)` で決定論的実行
- `SimulationContext` に `phase1_cumulative_time: Duration` を追加（あるいは `Instant` で都度計測）

### 観測対象
- **統計量**: `skip_child_search=true/false` の Phase 1 実行時間比
- **サンプルサイズ**: 同一設定で 3 回実行
- **期待される現象**: `skip_child_search=true` で Phase 1 が約 1/10 に短縮

### 較正計画
- **調整するフラグ**: `skip_child_search`（bool、Config フィールド）
- **目的関数 J(θ)**: 最小実行時間を目標とする（グラフ品質の低下がなければ常に true）

## Implementation Plan

### Step 1: `ReciprocitySimulatorConfig.skip_child_search: bool` 追加
- Default: `false`（後方互換性維持）
- `SimulationContext` にも `skip_child_search: bool` 追加

### Step 2: `generate_workflow_for_child` 分岐
```rust
fn generate_workflow_for_child(ctx: &mut SimulationContext, mission: &str) -> WorkflowGraph {
    if ctx.skip_child_search {
        return generate_new_workflow(mission, &mut ctx.rng, 0);
    }
    // 従来の SearchWorkflow::execute() パス
    ...
}
```

### Step 3: `phase1_population_growth` から `ctx.skip_child_search` 設定
- `run_kw_real_simulation` で Config から `ctx.skip_child_search` にコピー

### Step 4: サーバー設定パース追加 (`server.rs`)
```rust
if let Some(val) = obj.get("skip_child_search").and_then(|v| v.as_bool()) {
    cfg.skip_child_search = val;
}
```

### Step 5: フロントエンド UI 追加
- index.html: チェックボックス `id="skipChildSearch"`
- script.js: start コマンドに値を含める

## Boy Scout Rule — 翻訳可能性計画

- `generate_workflow_for_child` の責務を明確に: 「検索して生成」と「直接生成」の分岐が関数名から自明であること
- ミッション文字列のフォーマットに使われている `ctx.rng.random_range(0..10000)` のマジックナンバー `10000` を名前付き定数 `CHILD_MISSION_RANDOM_SUFFIX` に抽出する（ついでに）

## Acceptance Criteria

- [ ] `skip_child_search=true` で検索をスキップして直接生成される
- [ ] `skip_child_search=false` で従来動作と同一
- [ ] 全生存者が有効なグラフを持つ
- [ ] 既存テストがすべて通過する
- [ ] Phase 1 の実行時間が 90% 以上削減される

## Notes

### 成果物

- 計画: context/0153-generate-workflow-for-child/plan.md
- 実装サマリ: context/0153-generate-workflow-for-child/implementation.md
- レビュー報告書: context/0153-generate-workflow-for-child/review.md
- 観察レポート: context/0153-generate-workflow-for-child/observation-YYYYMMDD-HHmmss.md
