---
ticket_id: 58
title: M1.5-1: 実フォーマット形状ベクトル（1536次元等）のメモリ内 HNSW インデックス検索（Stage 2a/2b）Mockの検証
slug: m15-1-1536-hnsw-stage-2a2bmock
status: reviewed
created_at: 2026-05-23
updated_at: 2026-05-23
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0058-m15-1-1536-hnsw-stage-2a2bmock/observation-20260523-193052.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0058-m15-1-1536-hnsw-stage-2a2bmock/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0058-m15-1-1536-hnsw-stage-2a2bmock/review.md
---
# M1.5-1: 実フォーマット形状ベクトル（1536次元等）のメモリ内 HNSW インデックス検索（Stage 2a/2b）Mockの検証

## Summary

実フォーマット形状ベクトル（1536次元等）を保持するメモリ内 HNSW インデックス Mock を実装し、クエリベクトルに対するコサイン類似度上位 k 件の擬似空間検索関数を提供する。Stage 2a（SQLite/ベクトル検索）・Stage 2b（LadybugDB/グラフ検索）の Dual Retrieval パスを模擬するテスト基盤として機能する。

## Background

RFC §12.2 において、検索処理は Stage 2a（埋め込みベクトルによるセマンティック検索）と Stage 2b（グラフ構造に基づく検索）の2経路を持つ Dual Retrieval として設計されている。本チケットはこの2経路のうち、特にベクトル空間検索（Stage 2a/2b に共通するコサイン類似度計算）を Mock として実装し、後続の統合テストで使用するための基盤を提供する。

現状のコードベースでは:
- `src/store/graph_store.rs` に `InMemoryGraphStore` が存在し、`semantic_search()` で線形探索のコサイン類似度検索を実装している（`cosine_similarity()` は同ファイルの private 関数）
- `src/llm/mod.rs` に `EmbeddingProvider` トレイト + `FakeEmbeddingProvider` が存在し、決定論的疑似埋め込みベクトルを生成可能
- `src/mock.rs` に Mock パターン（`MockEmptyRetrievalPrimitive` 等）が存在し、`invocation_count` による計装パターンが確立されている
- 定数 `FAKE_EMBEDDING_DEFAULT_DIMENSION = 384` が定義されている（Calibration Candidate, 感度分析範囲 64-1536）

本チケットではこれらを活用し、特に**1536次元**（OpenAI text-embedding-3-small 等の実フォーマット）を第一対象とした HNSW 擬似インデックスを新規モジュールとして実装する。

## Scope

- **新規モジュール `src/vector_index.rs`** の作成:
  - `MockHnswIndex` 構造体: キーとベクトルのペアをメモリ内に保持し、HNSW 擬似グラフ空間におけるコサイン類似度検索を提供する
  - `insert(key: &str, vector: &[f32])` — ベクトルを登録する（同一キーで再登録可、上書き）
  - `search(query: &[f32], k: usize)` — クエリベクトルとのコサイン類似度上位 k 件を `Vec<(String, f64)>` で返す（ソート済み）
  - `invocation_count()` — 検索呼び出し回数を返す（Mock パターンに準拠）
  - `reset_count()` — 呼び出し回数カウンタをリセットする
- **定数の追加**: `constants.rs` に `HNSW_MOCK_DEFAULT_DIMENSION = 1536` を Safety Invariant として追加（実フォーマット形状のデフォルト値）
- **モジュール登録**: `src/lib.rs` に `pub mod vector_index` を追加
- **テスト**: 単体テスト（不変条件 assert） + 観測テスト（統計的性質の観測）の両方を同ファイル内 `#[cfg(test)] mod tests` に実装

## Non-scope

- 真の HNSW グラフ構築（近似最近傍探索アルゴリズム）— 本 Mock は線形探索で十分であり、近似処理を導入しない
- Real HNSW ライブラリ（`hnswlib` 等）への依存 — 外部クレートは追加しない
- 永続化・ストレージ連携 — 全てメモリ内完結
- スレッドセーフ対応 — シングルスレッドテスト用 Mock とする
- `GraphStore` トレイトとの統合 — 本 Mock は独立したコンポーネントとして提供する
- 量子化・次元圧縮 — フル精度 `f32` ベクトルのみ扱う

## Investigation

### ソースコード調査結果

**1. `src/store/graph_store.rs` — 既存のコサイン類似度検索**

- `InMemoryGraphStore` 構造体（`embeddings: RefCell<HashMap<String, Vec<f32>>>`）が線形探索の `semantic_search()` を提供している（L128-167）
- `cosine_similarity(a: &[f32], b: &[f32]) -> f64` は private 関数（L218-231）
- テスト `semantic_search_identical_vector`（L283-311）で同一ベクトル検索による類似度 1.0 確認パターンが確立済み

**2. `src/llm/mod.rs` — EmbeddingProvider トレイト階層**

- `EmbeddingProvider` トレイト: `embed()` + `embed_dimension()`
- `FakeEmbeddingProvider`: FNV-1a ハッシュベースの決定論的疑似埋め込み生成
- `ConstantEmbeddingProvider`: 常に同一ベクトルを返す
- デフォルト次元 `FAKE_EMBEDDING_DEFAULT_DIMENSION = 384`

**3. `src/mock.rs` — Mock パターン規範**

- `AtomicU64` による `invocation_count` / `reset_count` の計装パターン
- Box\<dyn RetrievalPrimitive\> のオブジェクト安全性確認

**4. `src/types.rs`** および `src/constants.rs`

- `QueryRepresentation` が `task_embedding: Vec<f32>` を保持
- `TEST_PRNG_SEED: u64 = 12345` が Safety Invariant として定義済み

### 参照観察レポート

- `tickets/context/0057-m1-3-debounce/observation-20260523-191416.md` — M1-3 Debounce 完了
- `tickets/context/0056-m1-2-adminfasttrack-trustauditlog/observation-20260523-182912.md` — M1-2 AdminFastTrack 完了
- `tickets/context/0055-m1-1-human-review-queue/observation-20260523-181239.md` — M1-1 HumanReviewQueue 完了

→ M1 系全チケット完了。M1.5 の最初のチケットとして新規 `src/vector_index.rs` モジュールを立ち上げる。

## Test Plan

### テスト対象モジュール

`src/vector_index.rs` 内の:
- `MockHnswIndex::new(dimension: usize)`
- `MockHnswIndex::insert(&self, key: &str, vector: &[f32])`
- `MockHnswIndex::search(&self, query: &[f32], k: usize) -> Result<Vec<(String, f64)>>`
- `MockHnswIndex::invocation_count(&self) -> u64`
- `MockHnswIndex::reset_count(&self)`

### 単体テスト（不変条件 assert）

| ID | テスト名 | 種別 | 内容 |
|---|---------|------|------|
| T1 | `exact_match_top_1` | 正常系 | ベクトルを1件登録し同一ベクトルで検索 → 類似度 1.0 で最上位 |
| T2 | `sort_invariant_descending` | 不変条件 | 複数登録後、検索結果が常に類似度降順であること |
| T3 | `top_k_respects_k` | 境界値 | k < n, k = n, k > n の3通りで k が正しく適用される |
| T4 | `empty_index_returns_empty` | 異常系 | 未登録状態で検索 → 空の結果 |
| T5 | `dimension_mismatch_error` | 異常系 | 異次元ベクトルで search → エラー |
| T6 | `insert_overwrite_same_key` | 正常系 | 同一キー再登録 → 後勝ち上書き |
| T7 | `zero_vector_and_orthogonal` | 境界値 | ゼロベクトルクエリ → 類似度 0.0 |
| T8 | `invocation_counting` | 計装 | search 呼び出し回数の一致確認 |
| T9 | `reset_count` | 計装 | リセット後 0 に戻る |
| T10 | `multiple_independent_instances` | 分離性 | 2インスタンスが独立 |

### 観測テスト（統計的性質の観測）

| ID | テスト名 | サンプルサイズ | 観測対象 |
|---|---------|--------------|---------|
| OTS-1 | `triangle_inequality_on_unit_hypersphere` | n = 10,000 | 1536次元単位超球上の三角不等式充足率 → 100% |
| OTS-2 | `sort_universality_across_random_queries` | n = 1,000 | 全試行でソート不変条件維持 → 100% |
| OTS-3 | `noise_perturbation_bounded` | n = 500 | ノイズ付加時 Jaccard 係数の有界性観測 |

## 計装方法・観測対象

### 計装方法

- 全観測テストは `src/vector_index.rs` 内 `#[cfg(test)] mod tests` に実装
- 全テスト: `StdRng::seed_from_u64(12345)` の固定シード PRNG 使用
- 観測出力: `println!` + `--nocapture` 経由の key=value 形式
- サンプルサイズ: OTS-1 n = 10,000（分布同定）、OTS-2/OTS-3 n = 1,000（ドリフト検出）

### 観測対象

| 観測量 | 期待値 | 検証方法 |
|--------|--------|---------|
| 三角不等式充足率 | 100%（数学的必然） | OTS-1 で全サンプル assert |
| ソート不変条件維持率 | 100%（実装要件） | OTS-2 で全試行 assert |
| Jaccard 係数 | ノイズ強度に応じた有界変動 | OTS-3 で平均・分散を観測記録 |
| invocation_count | 呼び出しごとに +1 | T8 で単調増加 assert |

### 較正計画

本チケットでは新規較正パラメータを導入しない。`HNSW_MOCK_DEFAULT_DIMENSION = 1536` は RFC §25 の実フォーマット形状仕様に基づく Safety Invariant。

## Boy Scout Rule — 翻訳可能性計画

1. **`cosine_similarity` の独立実装**: `src/vector_index.rs` 内に独立した `cosine_similarity` を実装する。
2. **翻訳可能な関数名**: `search`（"検索する"）、`insert`（"登録する"）等、動詞句で命名。
3. **一関数一責務**: 検索・ソート・カウントはそれぞれ独立した責務として分割。
4. **ハードコード値の定数化**: デフォルト次元 1536 → `constants.rs` の `HNSW_MOCK_DEFAULT_DIMENSION` として定義。
5. **エラー握りつぶし禁止**: 次元不一致・空ベクトルは `Result::Err` で適切に報告。

## Acceptance Criteria

- [ ] `src/vector_index.rs` が新規作成され、`MockHnswIndex` 構造体が実装されている
- [ ] `insert()` / `search()` が実装され、同一ベクトル検索で類似度 1.0 が返る
- [ ] search 結果が常に類似度降順でソートされている
- [ ] 次元不一致・空ベクトルがエラーとして報告される
- [ ] T1〜T10 の全単体テストが通過する
- [ ] OTS-1〜OTS-3 の全観測テストが通過する
- [ ] `cargo test` が既存テストも含めて全件通過する
- [ ] `cargo clippy` が新規警告を発しない

## Notes

<!--
注: このコメントは人間向けの説明である。AI は以下の手順に従うこと。

- plan_path: /plan-ticket が plan.md を作成後に frontmatter に更新する
- implementation_path: /start-ticket が implementation.md を作成後に frontmatter に更新する
- review_report_path: /review-ticket が review.md を作成後に frontmatter に更新する
- observation_report_path: /start-ticket が observation-YYYYMMDD-HHmmss.md を作成後に frontmatter に最新パスを更新する

各コマンドのワークフロー手順が frontmatter 更新の正しい手順である。
-->

### 成果物

- 計画: context/0058-m15-1-1536-hnsw-stage-2a2bmock/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0058-m15-1-1536-hnsw-stage-2a2bmock/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0058-m15-1-1536-hnsw-stage-2a2bmock/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0058-m15-1-1536-hnsw-stage-2a2bmock/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
