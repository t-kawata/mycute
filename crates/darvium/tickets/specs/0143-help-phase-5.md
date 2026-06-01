---
ticket_id: 143
title: HELP 成功時ワークフロー伝搬 — Phase 5 能力拡散
slug: help-phase-5
status: reviewed
created_at: 2026-05-29
updated_at: 2026-05-29
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0143-help-phase-5/plan.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0143-help-phase-5/observation-20260529-091631.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0143-help-phase-5/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0143-help-phase-5/review.md
---
# HELP 成功時ワークフロー伝搬 — Phase 5 能力拡散

## Summary

HELP 成功時（Phase 5）に、helper のワークフローグラフを helpee に条件付きでコピーする能力拡散を実装する。コピーは helper のグラフが helpee より複雑（ノード数が多い）場合のみ行われ、常に複雑化が進む方向への伝搬を保証する。これにより HELP がワークフローの複雑化に安全に寄与する。

## Background

現在の `phase5_capability_diffusion`（`src/simulation.rs:2884`）は、HELP 成功時に信頼プロファイルと評判スコアのみを継承し、ワークフローグラフ自体は一切伝搬しない。RFC §4A.5 Mechanism 25-26（HELP Execution / HELP Success）では、支援成功時に helper の能力（ワークフロー）が helpee に伝搬されることが期待される。この伝搬が不足しているため、HELP は信頼向上にのみ寄与し、ワークフロー複雑化には貢献できていない。

### 単純コピーの問題点

単純な上書きコピーでは、helper のグラフのノード数が helpee のグラフより少ない場合、ワークフローの複雑性が減少する（回帰）。これは複雑化を促進したい設計意図に反する。そこで、ノード数比較による条件付きコピーを採用する：helper のノード数 >= helpee のノード数の場合のみコピーし、そうでなければ GMR 拡散に処理を委ねる。

## Scope

- `phase5_capability_diffusion` に helper の `graph` を helpee に**条件付きで**コピーする処理を追加する
- コピー条件: **helper のグラフのノード数 >= helpee のグラフのノード数のときのみコピーする**
- 条件不成立時はコピーをスキップし、既存の GMR 拡散（`try_gmr_diffusion`）に処理を委ねる
- TODO コメントで本来の本物実装（セマンティックマージ、競合解決等）への拡張ポイントを記載する

### なぜ条件付きコピーか

helper のグラフを helpee に常に上書きすると、helper のグラフが helpee よりも複雑性（ノード数）が低い場合にワークフローの複雑化を阻害する（回帰）。ノード数比較による条件付きコピーにより、複雑化が進む方向にのみ伝搬が発生することを保証する。

## Non-scope

- セマンティックマージ（複数ワークフローの知的な統合）— TODO で拡張ポイントを示すに留める
- ノード数以外の複雑性指標（エッジ密度、DAG 深さ等）— 現在はノード数のみで判定する
- GMR DifferentialInference（チケット #144）
- SearchWorkflow の変更（チケット #142）

## Investigation

### 証拠 1: phase5_capability_diffusion がワークフローをコピーしない

`src/simulation.rs:2884-2912` — ループ内の処理（Spec 修正時点）：
```rust
let helper_trust = ctx.population[helper_id].trust.clone();
// ... 信頼継承 ...
inherit_reputation(hr, &mut cr, PHASE5_REPUTATION_INHERIT_DECAY);
ctx.population[helpee_id].experience_count = ...
// GMR 有効時: try_gmr_diffusion
```

`ctx.population[helper_id].graph` の参照も、`ctx.population[helpee_id].graph` への代入も存在しない。

### 証拠 2: シミュレーションの位相構造

`run_kw_real_simulation` のフェーズ構成：
- Phase 1: 人口成長（出生）
- Phase 2: 村クラスタリング
- Phase 3: HELP プロトコル
- Phase 4: GC 生存
- **Phase 5: 能力拡散 ← ここに追加**
- Phase 6: J_kw 測定

Phase 5 は `phase5_capability_diffusion(ctx, &successes)` として呼ばれ、`successes: &[(PersonId, PersonId)]` は Phase 3 で成立した HELP ペアのリスト。

### 参照観察レポート

- tickets/context/0141-compose/observation-20260528-162907.md — Phase 5 にワークフロー伝搬がないことが示唆されている
- tickets/context/0139-help/observation-20260528-170856.md — HELP プロトコルの状態遷移実装確認

### 証拠 3: 単純コピーによる複雑性回帰のリスク

`MemoizedGraph.graph` は `WorkflowGraph` 型であり、`node_count()` で取得可能なノード数を複雑性の指標として利用できる。単純な上書きコピーでは、helper のノード数が helpee より少ない場合に複雑性が減少し、RFC §4A.3 の複雑化メカニズム設計に反する。

現在の出生機構（チケット #142）により新生児は最小 1 ノードからスタートし、コード: `let fallback_complexity = (tick / GENERATION_COMPLEXITY_TICK_DIVISOR) as usize;` で動的複雑度が与えられる。HELP が繰り返される後期の個体ほど複雑なグラフを持つ可能性が高く、単純上書きは回帰のリスクが無視できない。

`MemoizedGraph` の構造 (`src/trust.rs:32-68`):
```rust
pub struct MemoizedGraph {
    pub graph: WorkflowGraph,  // ワークフローグラフ本体
    pub trust: TrustProfile,
    pub reputation: ReputationProfile,
    pub experience_count: u64,
    // ...
}
```

## Test Plan

### 不変条件テスト

1. **T1: helpee より helper のグラフが複雑な場合にコピーされる** — helper のノード数 > helpee のノード数で `phase5_capability_diffusion` 実行後、`helpee.graph` が `helper.graph` と同一（クローン）であることを確認
2. **T2: helpee より helper のグラフが単純な場合はコピーされない** — helper のノード数 < helpee のノード数で実行後、`helpee.graph` が変更されていないことを確認
3. **T3: ノード数が等しい場合はコピーされる** — helper のノード数 == helpee のノード数で実行後、コピーが発生することを確認（同程度の複雑性なら置換して問題ない）
4. **T4: 既存の信頼・評判継承が維持される** — グラフコピー追加後も trust/reputation の継承が従来通り動作することを確認
5. **T5: 既存テスト回帰なし** — `cargo test` 全パス

### 観測テスト

- **観測 1**: HELP 成功ペアにおける条件成立/不成立の比率（何%のペアでコピーが発生するか）
- **観測 2**: 伝搬前後の helpee 平均ノード数の変化

## 計装方法・観測対象

### 計装方法

- `phase5_capability_diffusion` 内に伝搬発生時の `println!` 計装
- 固定シード `StdRng::seed_from_u64(12345)` 使用

### 観測対象

- HELP 成功ペア数に対するグラフコピー発生数（条件成立/不成立の内訳）
- コピー前後の helpee ノード数変化

### 較正計画

本チケットでは新たな較正パラメータは導入しない。

## Boy Scout Rule — 翻訳可能性計画

- `phase5_capability_diffusion` の責務が「信頼継承」「評判継承」「経験値増加」「GMR拡散」と混在している。グラフコピーを追加する際に関数分割を検討する

## Acceptance Criteria

- [ ] HELP 成功時に helper のワークフローが **条件付きで** helpee にコピーされる（helper ノード数 >= helpee ノード数の場合のみ）
- [ ] 条件不成立時（helper ノード数 < helpee ノード数）はコピーされず、既存処理（GMR 拡散等）に委ねられる
- [ ] TODO コメントで本来の本物実装（セマンティックマージ等）への拡張ポイントが記載されている
- [ ] T1-T5 の不変条件テストが全通過している
- [ ] 観測テストで条件付きコピーの動作が確認できる

## Notes

### 成果物

- 計画: context/0143-help-phase-5/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0143-help-phase-5/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0143-help-phase-5/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0143-help-phase-5/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
