---
ticket_id: 1
title: M-2-1: RetrievalPrimitive 抽象インターフェース及びコアデータ型の定義
slug: m-2-1-retrievalprimitive
status: reviewed
created_at: 2026-05-21
updated_at: 2026-05-21
plan_path: /Users/shyme01/shyme/mycute/crates/darvium/tickets/context/0001-m-2-1-retrievalprimitive/plan.md
implementation_path: /Users/shyme01/shyme/mycute/crates/darvium/tickets/context/0001-m-2-1-retrievalprimitive/implementation.md
review_report_path: /Users/shyme01/shyme/mycute/crates/darvium/tickets/context/0001-m-2-1-retrievalprimitive/review.md
---
# M-2-1: RetrievalPrimitive 抽象インターフェース及びコアデータ型の定義

## Summary

`RetrievalPrimitive` トレイトおよび関連するコアデータ型（`QueryRepresentation`、`RetrievalPolicy`、`CandidateSet`、`RankedCandidate`）を定義する。SearchWorkflow が呼び出す GMR 検索の抽象インターフェースを確立し、後続の FakeImpl 実装・状態機械・ポリシー評価の土台を提供する。

## Background

Darvium RFC-0001 v2.0-final §13.4 に基づき、SearchWorkflow は GMR (Graph Memory Retrieval) の Stage 0–4 を `RetrievalPrimitive::search_workflows()` として呼び出す契約を持つ。このトレイトは以下の役割を担う：

1. SearchWorkflow と具象検索実装の間の**純粋抽象境界**を定義する
2. 検索結果としての候補集合 (`CandidateSet`) と、検索ポリシー (`RetrievalPolicy`) の型を確定する
3. クエリ表現 (`QueryRepresentation`) の canonical スキーマを固定する

現在の `src/types.rs` には `QueryRepresentation`、`RankedCandidate`、`CandidateSet` が空構造体として存在するが、フィールドが一切定義されていない。`RetrievalPrimitive` トレイトおよび `RetrievalPolicy` 構造体は未実装である。

このチケットは M-2 マイルストーンの最初のチケットであり、以降の全実装（FakeImpl、状態機械、ポリシー評価、バジェット管理）が依存する型基盤を確立する。

## Scope

1. **`QueryRepresentation` の具体化**: RFC §9.4 / §9.5 に基づき、以下のフィールドを持つ構造体に具体化する
   - `mission_text: String`
   - `task_embedding: Vec<f32>`
   - `query_design_text: String`
   - `query_design_embedding: Vec<f32>`
   - `design_template_version: String`
   - `query_type: QueryType`
   - `freshness_requirement: FreshnessRequirement`
   - `evidence_strictness: EvidenceStrictness`
   - `origin_trace_required: bool`
   - `drift_sensitivity: DriftSensitivity`

2. **`QueryType` / `FreshnessRequirement` / `EvidenceStrictness` / `DriftSensitivity` 列挙型の定義**

3. **`RetrievalPolicy` 構造体の定義**: RFC §13.4 に基づく
   - `top_k_sem: u32`
   - `top_k_struct: u32`
   - `min_trust: f32`
   - `allow_compose: bool`
   - `allow_new: bool`

4. **`RankedCandidate` 構造体の定義**: 検索結果の個別候補を表す
   - `workflow_id: String` — 候補ワークフローの識別子
   - `semantic_score: f64` — セマンティック類似度 (0.0–1.0)
   - `structural_score: f64` — 構造類似度 (0.0–1.0)
   - `blended_score: f64` — 統合スコア (0.0–1.0)
   - `trust_score: f64` — 信頼スコア (0.0–1.0)
   - `provenance: Vec<String>` — 証拠連鎖
   - `metadata: serde_json::Value` — 拡張メタデータ

5. **`CandidateSet` の具体化**: RFC §13.4 に基づく
   - `candidates: Vec<RankedCandidate>`
   - `retrieval_calls_used: u32`
   - `total_candidates_found: u32`

6. **`RetrievalPrimitive` トレイトの定義**: RFC §13.4 に基づく
   - `fn search_workflows(&self, query: &QueryRepresentation, policy: &RetrievalPolicy) -> Result<CandidateSet, DarviumError>`

7. **`RetrievalError` 関連エラーバリアントの確認**: 既存の `DarviumError::Retrieval` バリアントで十分か検証する

## Non-scope

- `RetrievalPrimitive` の具象実装（FakeImpl など） — チケット M-2-3 で扱う
- `SearchBudget` / `RecursionGuard` — チケット M-2-2 で扱う（既にフィールド定義済み）
- `SearchState` 状態機械 — チケット M-1.5-1 で扱う
- `EvaluateCandidatesStep` ポリシー評価 — チケット M-1-1 で扱う
- `serde` のシリアライゼーション derive は必要最小限にとどめる（JSON 出力が必要なもののみ）

## Investigation

### 現状確認（2026-05-21）

**ソースコード調査結果:**

1. **`src/types.rs`** — 以下の型が空構造体として既存:
   - `QueryRepresentation` (30行目): `#[derive(Debug, Clone)]` のみ、フィールドなし
   - `RankedCandidate` (33行目): 同上
   - `CandidateSet` (36行目): 同上
   - 上記3型は `#[derive(Debug, Clone)]` のみ実装
   - 一方 `SearchBudget` (40行目) と `RecursionGuard` (48行目) は既にフィールド定義済み

2. **`src/lib.rs`** — モジュール公開:
   - `pub mod types;` により全型は `darvium::types::*` でアクセス可能
   - 現状 `pub use` による再公開はなし

3. **`src/error.rs`** — エラー型:
   - `DarviumError::Retrieval(String)` が既に存在 (26行目)
   - このバリアントを `RetrievalPrimitive` のエラー型として使用可能

4. **RFC §13.4** より引用したトレイトシグネチャ:
```rust
trait RetrievalPrimitive {
    fn search_workflows(
        &self,
        query: &QueryRepresentation,
        policy: &RetrievalPolicy,
    ) -> Result<CandidateSet, RetrievalError>;
}
```

5. **RFC §9.4 / §9.5** より引用した `QueryRepresentation`:
   - v1.7 ベース: `mission_text`, `task_embedding`, `query_design_text`, `query_design_embedding`, `design_template_version`
   - v1.8 拡張: `query_type`, `freshness_requirement`, `evidence_strictness`, `origin_trace_required`, `drift_sensitivity`
   - デフォルト値: `query_type = Hybrid`, `freshness_requirement = Mixed`, `evidence_strictness = Light`, `origin_trace_required = false`, `drift_sensitivity = PreferLatest`

6. **`RankedCandidate`** のフィールド定義は RFC / 構造定義書に明示なし。ダブルステージ（セマンティック+構造）統合の文脈から必要フィールドを導出した（Scope 参照）。

## Test Plan

### コンパイル時検証（本チケットの中核）

このチケットの実装はデータ型定義のみであり、実行時ロジックを含まない。したがってテスト計画の主眼は**コンパイル時の型シグネチャ充足性検証**に置く。

| # | テスト種別 | テスト内容 | 期待結果 |
|---|-----------|-----------|---------|
| 1 | 型コンパイル | `RetrievalPrimitive` トレイトを実装するダミー構造体がコンパイルできる | コンパイル成功 |
| 2 | 型コンパイル | ダミー構造体の `search_workflows` が正しい型シグネチャで呼び出せる | コンパイル成功 |
| 3 | 型コンパイル | `QueryRepresentation` の全フィールドにアクセスできる | コンパイル成功 |
| 4 | デフォルト値 | `QueryRepresentation::default()` が RFC §9.5 のデフォルト値を満たす | 各フィールドが規定値と一致 |
| 5 | デフォルト値 | `RetrievalPolicy` の各フィールドが適切な初期値を持つ | フィールドがデフォルト設定される |
| 6 | 境界値 | `QueryType` / `FreshnessRequirement` / `EvidenceStrictness` / `DriftSensitivity` の全バリアントがマッチング可能 | 全バリアントの網羅的マッチがコンパイル可能 |
| 7 | 構築 | `CandidateSet` が空候補・複数候補の両方で構築可能 | 正常構築 |
| 8 | 構築 | `RankedCandidate` が全フィールドを指定して構築可能 | 正常構築 |
| 9 | トレイト境界 | トレイト境界の不整合を誘発する変異コードが確実にコンパイルエラーになる | 期待されたコンパイルエラーが発生 |

### テスト実装場所

- 単体テスト: `src/types.rs` 内の `#[cfg(test)] mod tests`
- トレイト境界テスト: `src/types.rs` 内のテストモジュール（`RetrievalPrimitive` のダミー実装を含む）

## Boy Scout Rule — 翻訳可能性計画

### 改善対象（既存コード）

| 箇所 | 問題 | 改善計画 |
|------|------|---------|
| `src/types.rs` L30–38 | `QueryRepresentation`, `RankedCandidate`, `CandidateSet` が空構造体で、翻訳不可能（何を表現する型か一切不明） | 本チケットでフィールドを具体化し、型自体がドメイン概念を語るようにする |
| `src/types.rs` 全般 | 全ての型がフラットに並び、責務によるグルーピングがない | 既存のコメント区切りは維持しつつ、関連型をまとめる位置に再配置しない（変更最小化）。ただし新規追加する型は適切なコメント区切り内に配置する |

### 遵守事項

- 関数名は動詞句（`search_workflows`, `rank_candidates` など）とする
- 変数名はドメイン概念を表す（`candidates` ではなく `ranked_candidates` のような明確な命名）
- 一関数一責務を徹底する
- ハードコード値は全て `src/constants.rs` の名前付き定数を参照する
- エラーの握りつぶし（`unwrap()` / `expect()` の無断使用）を禁止する

## Notes

<!--
注: このコメントは人間向けの説明である。AI は以下の手順に従うこと。

- plan_path: /plan-ticket が plan.md を作成後に frontmatter に更新する
- implementation_path: /start-ticket が implementation.md を作成後に frontmatter に更新する
- review_report_path: /review-ticket が review.md を作成後に frontmatter に更新する

各コマンドのワークフロー手順が frontmatter 更新の正しい手順である。
-->

### 成果物

- 計画: context/0001-m-2-1-retrievalprimitive/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0001-m-2-1-retrievalprimitive/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0001-m-2-1-retrievalprimitive/review.md（未作成、/review-ticket 全チェック通過後に作成）
