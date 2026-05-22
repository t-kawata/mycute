---
ticket_id: 4
title: EmbeddingProvider 抽象トレイトの定義
slug: embeddingprovider
status: reviewed
created_at: 2026-05-22
updated_at: 2026-05-22
implementation_path: /Users/shyme01/shyme/mycute/crates/darvium/tickets/context/0004-embeddingprovider/implementation.md
review_report_path: /Users/shyme01/shyme/mycute/crates/darvium/tickets/context/0004-embeddingprovider/review.md
observation_report_path: tickets/context/0004-embeddingprovider/observation.md
---

# EmbeddingProvider 抽象トレイトの定義

## Summary

埋め込みベクトル生成を抽象化する `EmbeddingProvider` トレイト、固定シード PRNG 駆動の `FakeEmbeddingProvider`、およびテスト用 `ConstantEmbeddingProvider` を定義する。これにより M-0.5〜M1.5 ではメモリ内疑似埋め込みを、M1.5 以降では本物の埋め込み API を使用するという段階的結合を、呼び出し元コードの修正なしで実現する。

## Background

Darvium の GMR Retrieval Core は、`task_embedding` と `workflow_design_embedding` の二系統の埋め込みベクトルに基づく Dual Retrieval（Stage 2a/2b）を行う。M-0.5 までは固定シード PRNG による疑似埋め込みでロジック検証を行い、M1.5 以降では本物の embedding API（OpenAI/text-embedding-3-small 等）に接続する計画である。

しかし、現在のコードベースには埋め込み生成の抽象化レイヤが存在しない。本チケットでは最初にトレイトを定義し、全埋め込み利用コードをこのトレイトに対するプログラミングにすることで、後段での差し替えをシームレスにする。

### 関連RFCセクション

- §12.2 Stage 2a, 2b Dual Retrieval — semantic (`task_embedding`) および structural proxy (`workflow_design_embedding`) の二経路検索
- §9.4 QueryDesignEmbedding — query design text からの埋め込み生成
- AG-06 / AG-07 — embedding model version / template version 不整合検出ゲート
- §17.5 M1.5 — Real embedding provider 実装計画

RFC 内に `EmbeddingProvider` トレイトの具体的なリファレンス実装は現時点で存在しない。本チケットのトレイト設計は Darvium-Tickets-v2.3.md の仕様を正本とする。

### 他チケットとの依存関係

- 本チケットは **M-2-1.6（LLMClient 抽象トレイト）の完了を前提とする**（`llm/` モジュールの同階層または内部に配置するため）
- **M-0.5-1（メモリ内デュアルストア）の前提条件**: M-0.5-1 は本 `EmbeddingProvider` トレイトに対する FakeImpl を利用して候補抽出をテストする
- **M-0.5-3（AG-06/AG-07 ハードゲート）の前提条件**: 本トレイトの `embed_dimension()` がバージョン不整合検出に使用される
- **M-1.5-1（実フォーマット形状ベクトル HNSW Mock）の前提条件**: M-1.5-1 は本トレイトの型定義を継承した実インデックス検索 Mock を実装する
- **M2-1（BuildQueryStep）**: 本トレイトに対する `RealEmbeddingProvider` を追加する

## Scope

1. **`EmbeddingProvider` トレイトの定義**:
   - `fn embed(&self, text: &str) -> Result<Vec<f32>, DarviumError>` — テキストから埋め込みベクトルを生成
   - `fn embed_dimension(&self) -> usize` — 生成されるベクトルの次元数を返す
   - 同期メソッドとして定義（非同期ラッパーは上位レイヤの責務）
   - `Send + Sync` を境界とし、スレッドセーフを保証
   - `Box<dyn EmbeddingProvider>` のオブジェクト安全性を確認

2. **`FakeEmbeddingProvider` の定義**:
   - 固定シード `StdRng::seed_from_u64(12345)` を使用
   - テキストのハッシュをシードに疑似埋め込みベクトルを生成
   - 次元数はコンストラクタ指定可能、デフォルト 384
   - 同一テキストに対して常に同一ベクトルを返す（決定論性）
   - `constants.rs` に `FAKE_EMBEDDING_DEFAULT_DIMENSION: usize = 384` を追加

3. **`ConstantEmbeddingProvider` の定義**:
   - 常に同じベクトルを返す（テスト用、決定論的挙動の確認に使用）

4. **エラー型の拡張**:
   - `DarviumError::Embedding(String)` — 埋め込み生成一般エラー（新規追加）
   - `DarviumError::EmbeddingDimensionMismatch { expected: usize, actual: usize }` — **既存**（`src/error.rs:35-36`）。変更不要

5. **テスト完全性**（本チケット内で完結）:
   - トレイト境界の充足をコンパイル時検証
   - 同一テキストの再現性（決定論性）
   - 異種テキストの非衝突性（衝突率 < 1e-6）
   - 次元数の一致確認
   - 空文字列埋め込みの挙動
   - オブジェクト安全性確認

## Non-scope

- `RealEmbeddingProvider` の実装（M1.5-1 または M2-1 で実施）
- `async` 対応（必要に応じて M1.5 以降で導入検討）
- HNSW インデックス検索（M1.5-1 で実施）
- embedding API クレートの依存追加（本チケットでは不要）
- バージョン不整合検出ロジック（M-0.5-3 で実施）
- キャッシュ機構（M1.5 以降で実施）

## Investigation

### 調査日時: 2026-05-22

#### 既存コードの Embedding 関連実装状況

**src/ ディレクトリ構造:**
```
src/
├── constants.rs      ← 51行、TEST_PRNG_SEED(12345) が既存
├── error.rs          ← EmbeddingDimensionMismatch は既存、Embedding(String) は未定義
├── lib.rs            ← pub mod llm; が既存、EmbeddingProvider は未実装
├── llm/mod.rs        ← LLMClient トレイト + FakeLlmClient（M-2-1.6 完了済み）
├── store/            ← Dual-Store トレイト群
└── types.rs          ← 基本型
```

**確認された既存表現（ファイル名:行番号）:**

1. **`src/constants.rs:46-47`** — `TEST_PRNG_SEED: u64 = 12345` が既存（`FakeEmbeddingProvider` のシードとして使用可能）。

2. **`src/error.rs:35-36`** — `DarviumError::EmbeddingDimensionMismatch { expected: usize, actual: usize }` が**既に存在する**。本チケットでは追加不要。

3. **`src/error.rs:32-33`** — `DarviumError::EmbeddingVersionMismatch(String)` が既に存在する（AG-06/AG-07 関連）。本チケットでは利用しない。

4. **`src/error.rs:99-104`** — 汎用 `Embedding(String)` バリアントが**未定義**。本チケットで `// === Embedding ===` セクションとして追加する必要がある。LLM エラーセクション（`// === LLM ===`）の直後に配置する。

5. **`src/llm/mod.rs:57-68`** — `LLMClient` トレイトが `Send + Sync` 境界を持ち、`Box<dyn LLMClient>` としてオブジェクト安全的に使用されている。`EmbeddingProvider` トレイトも同様のパターンに従う。

6. **`Darvium-Tickets-v2.3.md:114-132`** — 本チケットの仕様が記載済み。実装スコープの正本。

7. **`Darvium-RFC-0001-Unified-v2.3-final.md:324-327`** — アーキテクチャ概要で `EmbeddingProvider` が Layer 1 Executor/Provider Ports の一部として列出されている。現時点ではトレイト定義のみで、RFC 内にリファレンス実装は存在しない。

8. **`Darvium-RFC-0001-Unified-v2.3-final.md:557-559`** — `MemoizedGraph` のフィールドとして `task_embedding: Vec<f32>` と `workflow_design_embedding: Vec<f32>` が定義されている。これらは本トレイトの `embed()` メソッドの出力を使用する。

#### 結論

LLMClient と同様に、EmbeddingProvider トレイトの実装は一切存在しない。本チケットでは `src/llm/mod.rs` に `EmbeddingProvider` トレイト・`FakeEmbeddingProvider`・`ConstantEmbeddingProvider` を追加し、エラー型 `DarviumError::Embedding(String)` を `src/error.rs` に追加する。定数 `FAKE_EMBEDDING_DEFAULT_DIMENSION` を `src/constants.rs` に追加する。

## 計装方法・観測対象

### 計装方法
埋め込み生成の完全決定論性（同一ハッシュ入力に対する出力ベクトルのビットレベル完全一致）。`FakeEmbeddingProvider` の生成する疑似埋め込みベクトル空間におけるコサイン類似度の分布が、高次元超球面上の一様分布と統計的に区別できないこと（カイ二乗検定、$p > 0.05$）。

### 観測対象
- 埋め込みベクトルのビットレベル再現性
- コサイン類似度の分布形状（平均 $0$、標準偏差 $1/\sqrt{d}$）
- 高次元超球面上の一様分布との統計的一致性

### 較正計画
`FAKE_EMBEDDING_DEFAULT_DIMENSION` (`src/constants.rs:65`) が Calibration Candidate。感度分析推奨範囲: 64-1536。次元数変更がコサイン類似度分布の標準偏差に与える影響を観測する。

## Test Plan

### テスト対象: `EmbeddingProvider` トレイト

| #  | テストケース | 種別 | 内容 |
|----|-------------|------|------|
| T1 | トレイト境界充足のコンパイル時検証 | 正常系 | `FakeEmbeddingProvider` が `EmbeddingProvider` トレイトを実装していることを型アサーションで確認 |
| T2 | オブジェクト安全性 | 正常系 | `Box<dyn EmbeddingProvider>` がコンパイル可能であること |
| T3 | Send + Sync 境界 | 正常系 | `Box<dyn EmbeddingProvider + Send + Sync>` がスレッド間移動可能であること |

### テスト対象: `FakeEmbeddingProvider` — 決定論性

| #  | テストケース | 種別 | 内容 |
|----|-------------|------|------|
| T4 | 同一テキストの再現性 | 正常系 | 同じテキストを 2 回 embed し、ビットレベルで同一のベクトルが返る（決定論性） |
| T5 | 異種テキストの非衝突性 | 正常系 | 異なるテキストを embed し、ベクトルが異なることを確認（サンプリング n=10,000） |
| T6 | デフォルト次元数 | 正常系 | `embed_dimension()` が `FAKE_EMBEDDING_DEFAULT_DIMENSION`（384）と一致すること |
| T7 | カスタム次元数 | 正常系 | コンストラクタで指定した次元数が `embed_dimension()` と一致すること |
| T8 | 空文字列 embed | 境界値 | 空文字列を embed してもエラーにならず、指定次元数のベクトルが返ること |
| T9 | 長大テキスト embed | 境界値 | 10,000 文字以上のテキストを embed してもエラーにならず、指定次元数のベクトルが返ること |

### テスト対象: `ConstantEmbeddingProvider`

| #  | テストケース | 種別 | 内容 |
|----|-------------|------|------|
| T10 | 常に同一ベクトル | 正常系 | 異なるテキストに対しても同一ベクトルが返る |
| T11 | 次元数の一致 | 正常系 | コンストラクタで指定した次元数が `embed_dimension()` と一致する |

### テスト対象: エラー型

| #  | テストケース | 種別 | 内容 |
|----|-------------|------|------|
| T12 | `DarviumError::Embedding` のメッセージ確認 | 正常系 | エラーメッセージが正しいこと |
| T13 | `DarviumError::EmbeddingDimensionMismatch` のメッセージ確認 | 正常系 | エラーメッセージが正しいこと（既存バリアントの確認） |
| T14 | エラーの `PartialEq` 比較 | 正常系 | 同一エラー値の等価性比較が成立すること |

### テスト対象: 計装（観測）

| #  | テストケース | 種別 | 内容 |
|----|-------------|------|------|
| T15 | 埋め込みベクトルの分布 | 観測 | `FakeEmbeddingProvider` の生成する疑似埋め込みベクトル空間におけるコサイン類似度の分布を出力。高次元超球面上の一様分布と統計的に区別できないこと（カイ二乗検定、p > 0.05、n=1,000） |

## Boy Scout Rule — 翻訳可能性計画

本チケットで拡張する `src/llm/mod.rs`（および `src/error.rs`, `src/constants.rs`）において、以下の翻訳可能性を確保する：

1. **関数名は動詞句**: `embed` は「埋め込む」、`embed_dimension` は「埋め込み次元数を返す」と翻訳可能。

2. **変数名はドメイン概念**: `text` / `dimension` など、埋め込みドメインの標準用語を使用し、汎用的な変数名を避ける。

3. **一関数一責務**: `embed` は「テキスト → ベクトル」という 1 つの責務のみを持つ。

4. **ハードコード値は名前付き定数**: デフォルト次元数 `384` は `FAKE_EMBEDDING_DEFAULT_DIMENSION` として `constants.rs` に定義する。テストシードは既存の `TEST_PRNG_SEED`（12345）を再利用する。

5. **エラー握りつぶし禁止**: FakeImpl 内でエラーを握りつぶさず、`DarviumError::Embedding` を適切に伝播する。

6. **既存コード改善**: `src/error.rs` に `Embedding(String)` バリアントを追加する際、`EmbeddingDimensionMismatch` および `EmbeddingVersionMismatch` とともに `// === Embedding ===` セクションとしてグループ化する。

## Acceptance Criteria

- [ ] `EmbeddingProvider` トレイトが `Send + Sync` 境界を持ち、`Box<dyn EmbeddingProvider>` のオブジェクト安全性が確認できる
- [ ] `FakeEmbeddingProvider` が固定シード PRNG 駆動の疑似埋め込みベクトルを生成し、同一テキストに対して決定論的である
- [ ] `ConstantEmbeddingProvider` が常に同一ベクトルを返す
- [ ] `DarviumError::Embedding(String)` が追加されている（`EmbeddingDimensionMismatch` は既存）
- [ ] `FAKE_EMBEDDING_DEFAULT_DIMENSION` 定数が `constants.rs` に追加されている
- [ ] T1〜T15 の全テストケースが通過する
- [ ] 既存テストがすべて通過する
- [ ] `cargo build` が通る

## Notes

<!--
注: このコメントは人間向けの説明である。AI は以下の手順に従うこと。

- plan_path: /plan-ticket が plan.md を作成後に frontmatter に更新する
- implementation_path: /start-ticket が implementation.md を作成後に frontmatter に更新する
- review_report_path: /review-ticket が review.md を作成後に frontmatter に更新する

各コマンドのワークフロー手順が frontmatter 更新の正しい手順である。
-->

### 成果物

- 計画: context/0004-embeddingprovider/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0004-embeddingprovider/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0004-embeddingprovider/review.md（未作成、/review-ticket 全チェック通過後に作成）
