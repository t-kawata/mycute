---
ticket_id: 116
title: M1.76-KW-REAL-P2: GMR抽象化層
slug: m176-kw-real-p2-gmr
status: reviewed
created_at: 2026-05-26
updated_at: 2026-05-26
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0116-m176-kw-real-p2-gmr/plan.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0116-m176-kw-real-p2-gmr/observation-20260526-204244.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0116-m176-kw-real-p2-gmr/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0116-m176-kw-real-p2-gmr/review.md
---
# M1.76-KW-REAL-P2: GMR抽象化層

## Summary

RFC §4A.3 に定義された GMR（Goal-Mediated Reasoning）機構のうち、未実装の 7 機構（DeterminismScore、ApplicabilityScore AG-01〜AG-05、Stage5分岐、COMPOSE、NEW、Differential Inference、ApplicabilityChannel / CapabilityGenerator トレイト）を abstract 実装する。既存の REAL コンポーネント（GraphPatch, apply_patch_atomic, AG-06, AG-07）はそのまま流用する。

## Background

P4（6 フェーズシミュレーションループ）の観測結果では、Phase5（能力拡散）の diffusion イベント数が 0 であった。これは GMR 機構が未実装のため、シミュレーション内で能力拡散が発生しない状態である。P2 はこのギャップを埋め、P4 で動作可能な形で GMR 抽象層を提供する。

本チケットは KW-REAL シリーズ 6 チケットの第 4 弾であり、P4 の人口成長フェーズ（Phase1）と能力拡散フェーズ（Phase5）で使用される GMR 機構を実装する。監査の結果、AG-06/AG-07 は REAL、残り 7 機構は MISSING である。「シミュレーションはツールであって目的ではない」 — 不足機構は trait で抽象化し、将来の本実装（ANN 検索パイプライン等）に置き換え可能にする。シミュレーション用の簡略化された代用実装で理論検証を可能にする。

## Scope

### 新規作成: src/gmr.rs

1. **DeterminismScore 構造体** — `fn compute(&self, graph: &WorkflowGraph) -> f64`
   - 各 WorkflowNode::AgentStep の determinism 値を SoftMin 合成
   - シミュレーション内では全 AgentStep の determinism 平均値で代用

2. **ApplicabilityScore 構造体** — AG-01〜AG-05 を abstract 実装:
   - AG-01 RewardSignalChannel: 履歴成功率で代用
   - AG-02 UtilityChannel: 期待効用で代用
   - AG-03 NoveltyChannel: Embedding 間コサイン距離で代用（既存 cosine_similarity 利用）
   - AG-04 UrgencyChannel: デッドライン残り tick 数で代用
   - AG-05 SafetyChannel: リスクスコアで代用
   - 既存の check_ag06 / check_ag07（applicability.rs）は流用

3. **Stage5Branch 列挙型**:
   ```rust
   pub enum Stage5Branch { Reuse, Patch, Compose, New, Abort }
   ```

4. **Stage5Decision 構造体** — `fn decide(candidate: &ApplicabilityOutcome) -> Stage5Branch`
   - スコアベースの確率的選択（高スコア→REUSE/COMPOSE、低スコア→ABORT）

5. **DifferentialInference 構造体** — `fn infer(&self, source: &WorkflowGraph, target: &mut WorkflowGraph, rng: &mut StdRng) -> Vec<GraphPatch>`
   - 不足 AgentStep を特定し GraphPatch（patch.rs:102, REAL）として差分生成
   - apply_patch_atomic（patch.rs:273, REAL）で適用可能なパッチを生成

6. **ApplicabilityChannel トレイト**:
   ```rust
   pub trait ApplicabilityChannel { fn score(&self, candidate: &ApplicabilityCandidate) -> f64; }
   ```

7. **CapabilityGenerator トレイト**:
   ```rust
   pub trait CapabilityGenerator { fn generate(&self, seed: &WorkflowGraph, rng: &mut StdRng) -> WorkflowGraph; }
   ```

### 拡張: src/composition.rs

8. **compose_workflows 関数** — `fn compose(a: &WorkflowGraph, b: &WorkflowGraph) -> WorkflowGraph`
   - 2 つの WorkflowGraph のノードを統合し、共通部分を結合
   - 既存の validate_composition_plan / detect_frontier_leakage を検証に使用

### 拡張: src/simulation.rs

9. Phase1（人口成長）と Phase5（能力拡散）で GMR コンポーネントを使用するよう拡張
   - スタブモードから abstract GMR 呼び出しに切り替え
   - SimulationContext に GMR 関連フィールドを追加（Option ラップ）

### エクスポート: src/lib.rs

10. pub mod gmr を追加し、全公開型をエクスポート

## Non-scope

- AG-06 / AG-07 の修正（既存流用、変更不要）
- ANN 検索パイプライン（Stage 1-4）の実装 — シミュレーションでは省略
- 本番 GMR パイプライン — trait の具象実装は後続チケット
- src/patch.rs の修正 — GraphPatch / apply_patch_atomic は流用のみ
- src/search/applicability.rs の修正 — 既存テスト全 PASS が条件

## Investigation

### 調査日時: 2026-05-26

以下の物理的証拠に基づき、RFC §4A.3 の 8 機構の実装状態を検証した。

#### REAL（既存実装、そのまま流用可能）

| 機構 | ファイル | 行 | 状態 |
|------|---------|----|------|
| GraphPatch 構造体 | src/patch.rs | 102 | 5 フィールド完全実装 |
| apply_patch_atomic 関数 | src/patch.rs | 273 | WorkflowGraph 引数、Result 戻り値 |
| AG-06 (Semantic Channel) | src/search/applicability.rs | 86 | check_ag06 — EmbeddingChannelVersion 比較 |
| AG-07 (Structural Proxy) | src/search/applicability.rs | 107 | check_ag07 — EmbeddingChannelVersion 比較 |
| ApplicabilityOutcome | src/search/pipeline.rs | 38 | 6 フィールド（semantic〜trust_score） |
| cosine_similarity | src/search/pipeline.rs | 60 | f32 ベクトル間コサイン類似度 |
| WorkflowGraph = DiGraph<WorkflowNode, EdgeMeta> | src/types.rs | 91 | type alias |
| WorkflowNode enum (AgentStep variant) | src/types.rs | 53 | agent / prompt_template / inputs / output_var |

#### MISSING（未実装、本チケットで作成）

| 機構 | 理由 | 備考 |
|------|------|------|
| DeterminismScore 構造体 | grep -rn "struct DeterminismScore" → 該当なし | Rust 構造体として新規作成 |
| ApplicabilityScore 構造体 | grep -rn "struct ApplicabilityScore" → 該当なし | AG-01〜AG-05 を含む |
| Stage5Branch 列挙型 | grep -rn "enum Stage5Branch" → 該当なし | 5 方向分岐 |
| Stage5Decision 構造体 | grep -rn "struct Stage5Decision" → 該当なし | decide メソッド |
| compose_workflows 関数 | grep -rn "fn compose_workflows" → 該当なし | composition.rs 未実装 |
| new_workflow_from 関数 | grep -rn "fn new_workflow_from" → 該当なし | NEW 機構の中核 |
| DifferentialInference 構造体 | grep -rn "struct DifferentialInference" → 該当なし | infer メソッド |
| ApplicabilityChannel トレイト | grep -rn "trait ApplicabilityChannel" → 該当なし | trait + 5 実装 |
| CapabilityGenerator トレイト | grep -rn "trait CapabilityGenerator" → 該当なし | NEW 機構の抽象化 |

#### AG-01〜AG-05 個別チェック

```bash
grep -rn "RewardSignalChannel" src/  → 該当なし
grep -rn "UtilityChannel" src/       → 該当なし
grep -rn "NoveltyChannel" src/       → 該当なし
grep -rn "UrgencyChannel" src/       → 該当なし
grep -rn "SafetyChannel" src/        → 該当なし
```

全 5 チャネルとも未実装である。

#### 既存 composition.rs 機能

composition.rs の公開関数は validate_composition_plan, detect_frontier_leakage のみで、compose_workflows は存在しない。

#### 参照観察レポート

- tickets/context/0115-m176-kw-real-p4-6/observation-20260526-201837.md — P4 観測結果: Phase5（能力拡散）の diffusion=0。GMR 未実装により能力拡散が発生していない。J_kw=0.3625。
- tickets/context/0114-m176-kw-real-p5/observation-20260526-195519.md — P5 観測: パラメータ較正結果。
- tickets/context/0113-m176-kw-real-p1-simulationcontext/observation-20260526-191047.md — P1 観測: SimulationContext 基盤の検証。
- tickets/context/0112-m176-kw4-kind-world/observation-20260526-171302.md — KW4 観測: Kind World シミュレーション全体の J_kw 最適化結果。

## Test Plan

### ユニットテスト（src/gmr.rs 内 mod tests）

1. **DeterminismScore::compute** — 全 AgentStep determinism = 1.0 で 1.0、全 0.0 で 0.0 を返すこと。空グラフで NaN にならないこと。
2. **AG-01〜AG-05 各チャネル** — 各チャネルの score() が [0, 1] 範囲を返すこと。境界値（空履歴、最大効用、同一 Embedding 等）でクラッシュしないこと。
3. **Stage5Decision::decide** — 高スコア ApplicabilityOutcome（total_score=0.9）に REUSE または COMPOSE を割り当てること。低スコア（total_score=0.1）に ABORT を割り当てること。PATCH が中間スコアで出現すること。
4. **compose_workflows** — 2 つの単純な WorkflowGraph（各 1 AgentStep）を統合し、ノード数 = 2 になること。同一 AgentStep を含むグラフは重複排除されること。
5. **NEW 機構 (CapabilityGenerator)** — 生成された WorkflowGraph が seed と同一構造ではないこと。生成結果が空グラフにならないこと。
6. **DifferentialInference::infer** — 生成される GraphPatch が apply_patch_atomic で適用可能であること。差分がない場合、空の Vec が返ること。
7. **既存の search/applicability.rs テスト** — 全 PASS すること（回帰テスト、変更不可）。

### E2E テスト（src/simulation.rs 内）

8. **simulation_with_gmr** — P4 の 6 フェーズループに P2 GMR を接続し、Phase5 diffusions > 0 を確認する（観測ベース）。

## 計装方法・観測対象

### 計装方法

- P2 専用テスト関数: src/gmr.rs 内に mod tests を追加し、各機構の単体テストを実装
- 観測テスト: --nocapture で以下の構造化出力を行う
- 固定シード: StdRng::seed_from_u64(12345) で全テスト再現保証

### 観測対象

| 観測対象 | 内容 | サンプルサイズ |
|---------|------|--------------|
| AG チャネルスコア分布 | AG-01〜AG-05 の各スコア値を JSON 配列として出力 | n >= 100 |
| Stage5 分岐確率 | REUSE/PATCH/COMPOSE/NEW/ABORT の出現割合 | n >= 1,000 判断 |
| GraphPatch サイズ分布 | 生成されるパッチの operations 長の分布 | n >= 100 |
| Phase5 diffusion 数 | GMR 接続後のシミュレーション内拡散イベント数 | 観測値 |

### 較正計画

本チケットでは GMR 機構の abstract 実装が主目的であり、較正は最小限とする。

- 調整候補: AG チャネルの重みパラメータ（constants.rs に追加予定）
- 目的関数 J(θ): シミュレーション内の Phase5 diffusions 数（最大化）と J_kw 値の変化
- 停止条件: 全テスト PASS + GMR 機構が期待通り動作することの確認

## Boy Scout Rule — 翻訳可能性計画

### 本チケットで新規作成するコード

- src/gmr.rs: 全ての公開関数名・変数名を動詞句＋ドメイン名に統一。trait 名は ApplicabilityChannel / CapabilityGenerator など目的を明確に命名。マジックナンバーはすべて constants.rs の定数参照とする。

### 修正する既存コード

- src/simulation.rs Phase1/Phase5: 既存の TODO コメントを削除し、実際の GMR 呼び出しに置き換え。関数分割済みのため大規模変更は不要。
- src/lib.rs: pub mod gmr の追加のみ。
- src/composition.rs: compose_workflows 追加。既存の関数定義スタイルに合わせる。
- src/constants.rs: GMR 関連定数（AG 重み等）を追加。分類コメント（Calibration Candidate / Safety Invariant）を明記。

## Acceptance Criteria

- [ ] DeterminismScore::compute が全 determinism=1.0 で 1.0、全 0.0 で 0.0 を返す（TC1）
- [ ] AG-01〜AG-05 の各チャネルが [0, 1] 範囲のスコアを返す（TC2）
- [ ] Stage5Decision::decide が高スコア候補に REUSE/COMPOSE を、低スコアに ABORT を割り当てる（TC3）
- [ ] compose_workflows が 2 つの WorkflowGraph を正しく統合する（TC4）
- [ ] NEW 機構で生成された WorkflowGraph が seed と同一構造ではない（TC5）
- [ ] DifferentialInference::infer が生成する GraphPatch が apply_patch_atomic で適用可能（TC6）
- [ ] 既存の search/applicability.rs テストが全 PASS する（TC7）
- [ ] GMR 接続後、シミュレーション Phase5 の diffusion 数が 0 より大きくなる（観測ベース、TC8）
- [ ] 翻訳可能性の検証が通っている（関数名・変数名・定数化・責務分割）
- [ ] 既存テスト全 PASS + cargo clippy 警告ゼロ

## Notes

### 依存関係

- P1 (SimulationContext 基盤) の型定義を使用するが、独立開発可能
- P4 (6 フェーズシミュレーションループ) はスタブモードで動作可能なため、実装順序の制約なし
- AG-06/AG-07 は既存の search/applicability.rs に依存（変更不要、テスト PASS 必須）

### 成果物

- 計画: context/0116-m176-kw-real-p2-gmr/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0116-m176-kw-real-p2-gmr/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0116-m176-kw-real-p2-gmr/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0116-m176-kw-real-p2-gmr/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
