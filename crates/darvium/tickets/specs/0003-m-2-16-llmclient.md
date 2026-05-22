---
ticket_id: 3
title: チケット M-2-1.6: LLMClient 抽象トレイトの定義
slug: m-2-16-llmclient
status: reviewed
created_at: 2026-05-22
updated_at: 2026-05-22
implementation_path: /Users/shyme01/shyme/mycute/crates/darvium/tickets/context/0003-m-2-16-llmclient/implementation.md
review_report_path: /Users/shyme01/shyme/mycute/crates/darvium/tickets/context/0003-m-2-16-llmclient/review.md
---
# チケット M-2-1.6: LLMClient 抽象トレイトの定義

## Summary

LLM 呼び出しを抽象化する `LLMClient` トレイト、スキーマ定義 `LlmSchema`、および決定論的ダミー実装 `FakeLlmClient` を定義する。これにより M-2-3（Mock クライアント）以降の全チケットは LLM 接続なしでロジック層のテストが可能となり、M2 以降で `RealLlmClient` を追加するだけで本物の LLM API に差し替え可能になる。

## Background

Darvium RFC-0001 v2.3 では、M-0.5 で Fake LLM adapter、M2〜M3 で本物の LLM API に段階的に接続する計画である。しかし、現在のコードベースには LLM 呼び出しの抽象化レイヤが一切存在しない。本チケットでは最初にトレイトを定義し、全 LLM 利用コードをこのトレイトに対するプログラミングにすることで、後段での差し替えをシームレスにする。

### 関連RFCセクション

- §14.2 構造化出力要求契約 — LLM からの JSON スキーマ準拠出力を要求する契約
- §13A LLM adapter interface — トレイト境界としての LLM 抽象化（RFC §17.4-17.5）
- §12.2 GraphPatchGenerator — LLM 自己評価スコア cₛ とパッチ生成パイプライン
- §17.4 M-0.5 — Fake LLM adapter の位置づけ
- §17.5 M2 — RealLlmClient 実装計画

### 他チケットとの依存関係

- 本チケットは **M-2-1（RetrievalPrimitive）の完了を前提とする**（既存の `DarviumError` 型を拡張するため）
- M-2-1.5（Dual-Store トレイト）とは独立して並行実装可能
- **M-2-3（Mock クライアント）の前提条件**: M-2-3 は本 LLMClient トレイトに対する FakeImpl をテストで利用する
- M-0.5-1（スクリプト化 Fake LLM）は本 FakeLlmClient を継承・拡張する形で実装される
- M2-1（BuildQueryStep）は本トレイトに対する `RealLlmClient` を追加する

## Scope

1. **`LLMClient` トレイトの定義**:
   - `fn generate_structured(&self, prompt: &str, schema: &LlmSchema) -> Result<String, DarviumError>`
   - 同期メソッドとして定義（非同期ラッパーは上位レイヤの責務）
   - `Send + Sync` を境界とし、スレッドセーフを保証
   - `Box<dyn LLMClient>` のオブジェクト安全性を確認

2. **`LlmSchema` 列挙型の定義**:
   - `QueryDesignText` — クエリ設計テキスト生成
   - `PatchOperations` — パッチ操作列生成（RFC §12.2 の JSON スキーマに相当）
   - `SelfScore` — LLM 自己評価スコア cₛ 出力（RFC §12.2）
   - `FreeText` — 自由文形式（スキーマ制約なし）
   - 各バリアントにヒント文字列を保持可能にする（例: JSON schema description）

3. **`FakeLlmClient` ダミー実装の定義**:
   - **固定文字列モード**（デフォルト）: コンストラクタで指定された固定文字列を常に返す
   - **乱数モード**: 指定確率で不正フォーマット（空文字列、不正 JSON、指定外文字列）を返す
   - `call_count: Arc<AtomicUsize>` で呼び出し回数を計測可能
   - テスト用ヘルパーコンストラクタ: `default_pass()`、`returns_empty()`、`returns_malformed()` 等

4. **エラー型の拡張**:
   - `DarviumError::Llm(String)` — LLM 呼び出し一般エラー
   - `DarviumError::LlmMalformedJson(String)` — LLM 応答の JSON パース失敗

5. **テスト完全性**（本チケット内で完結）:
   - トレイト境界の充足をコンパイル時検証
   - 固定文字列モードの出力正確性
   - 乱数モードの不正フォーマット注入確率検証
   - エラー型の正しい伝播
   - オブジェクト安全性確認

## Non-scope

- `RealLlmClient` の実装（M2-1 で実施）
- `async` 対応（必要に応じて M2 以降で `async_trait` 導入を検討）
- LLM API クレートの依存追加（本チケットでは不要）
- `PatchGenerationContext` の定義（M2-1 で実施）
- プロンプトテンプレートの管理（M2-1 で実施）
- リトライ・サーキットブレーカーロジック（M2-2 以降で実施）

## Investigation

### 調査日時: 2026-05-22

#### 既存コードの LLM 関連実装状況

**src/ ディレクトリ構造:**
```
src/
├── constants.rs      ← 47行、LLM 関連定数として SELF_CONF_DISCOUNT(0.85) が既存
├── error.rs          ← DarviumError に LLM エラーバリアントなし
├── lib.rs            ← DarviumConfig は空、LLMClient 未定義
├── types.rs          ← 基本型のみ、LlmSchema 未定義
├── store/
│   ├── mod.rs        ← Dual-Store トレイトのモジュール構成
│   ├── graph_store.rs← GraphStore トレイト + InMemoryGraphStore
│   └── metadata_store.rs ← MetadataStore トレイト + InMemoryMetadataStore
```

**確認された既存表現（ファイル名:行番号）:**

1. `src/constants.rs:27` — `SELF_CONF_DISCOUNT: f64 = 0.85` が既存（LLM 自己信頼ディスカウント率）。これは LLMClient トレイトの直接の定数ではないが、LLM 周辺の調整定数として参照される可能性がある。

2. `src/error.rs:10-101` — `DarviumError` は 20 バリアントを持つが、`Llm(String)` および `LlmMalformedJson(String)` は未定義。本チケットで追加する必要がある。

3. `Darvium-RFC-0001-Unified-v2.3-final.md:2556-2564` — RFC §17 で LLM トレイトのリファレンス実装が記載:
   ```rust
   #[async_trait]
   pub trait LlmClient: Send + Sync {
       async fn generate_patch(
           &self,
           ctx: &PatchGenerationContext,
       ) -> Result<(f32, Vec<PatchOperation>), LlmError>;
   }
   ```
   ただし、本チケットで定義する `LLMClient` トレイトはより汎用的な `generate_structured` メソッドを持ち、`LlmSchema` による出力形式の切り替えを可能にする。RFC の `generate_patch` は本トレイトを内部で利用する高レベルメソッドとして位置づけられる。

4. `Darvium-RFC-0001-Unified-v2.3-final.md:2590-2613` — RFC §17 で `FakeLlmClient` のリファレンス実装が記載（`self_confidence` と `ops` を返す）。本チケットの `FakeLlmClient` はより汎用的な `generate_structured` に対応し、固定文字列モードと乱数モードを両立する。

5. `Cargo.toml:13-22` — 現在の依存関係に `rand` が dev-dependencies として存在。`FakeLlmClient` の乱数モードで利用可能。

6. `Darvium-Tickets-v2.3.md:98-112` — 本チケットの仕様が記載済み。以下の実装スコープが明示:
   - `LlmSchema` 列挙型: `QueryDesignText`, `PatchOperations`, `SelfScore`, `FreeText`
   - `LLMClient` トレイト: `fn generate_structured(&self, prompt: &str, schema: &LlmSchema) -> Result<String, DarviumError>`
   - `FakeLlmClient`: 固定文字列モード + 乱数モード
   - エラー型: `DarviumError::Llm(String)` および `DarviumError::LlmMalformedJson(String)`

#### 結論

既存コードに LLMClient 関連の実装は一切存在しない。本チケットは新規に `src/llm/mod.rs`（または `src/llm_client.rs`）を作成し、トレイト・列挙型・Fake 実装・エラー型拡張を一括で定義する必要がある。

## Test Plan

### テスト対象: `LLMClient` トレイト

| # | テストケース | 種別 | 内容 |
|---|-------------|------|------|
| T1 | トレイト境界充足のコンパイル時検証 | 正常系 | `FakeLlmClient` が `LLMClient` トレイトを実装していることを型アサーションで確認 |
| T2 | オブジェクト安全性 | 正常系 | `Box<dyn LLMClient>` がコンパイル可能であること |
| T3 | Send + Sync 境界 | 正常系 | `Box<dyn LLMClient + Send + Sync>` がスレッド間移動可能であること |

### テスト対象: `FakeLlmClient` — 固定文字列モード

| # | テストケース | 種別 | 内容 |
|---|-------------|------|------|
| T4 | 指定文字列が正確に返る | 正常系 | コンストラクタで指定した固定文字列が `generate_structured` の戻り値と一致する |
| T5 | 同一インスタンスの複数回呼び出しで同一出力 | 正常系 | 同じインスタンスを 3 回呼び出し、すべて同じ文字列が返る |
| T6 | LlmSchema のバリアント問わず同一出力 | 正常系 | `QueryDesignText` / `PatchOperations` / `SelfScore` / `FreeText` の全スキーマで同一の固定文字列が返る |
| T7 | 空文字列モード | 境界値 | `returns_empty()` で `generate_structured` を呼び出し、空文字列が返る |

### テスト対象: `FakeLlmClient` — 乱数モード

| # | テストケース | 種別 | 内容 |
|---|-------------|------|------|
| T8 | 指定確率での不正フォーマット注入 | 正常系 | 確率 0.3 で設定し、100 回呼び出して約 30 回程度が不正フォーマットであることを統計的に確認（二項分布、95%信頼区間内） |
| T9 | 確率 0.0 では常に正常出力 | 境界値 | 確率 0.0 で `generate_structured` が常に正常文字列を返す |
| T10 | 確率 1.0 では常に不正出力 | 境界値 | 確率 1.0 で `generate_structured` が常に不正フォーマットを返す |
| T11 | 不正フォーマットの種類 | 正常系 | 空文字列、不正 JSON、指定外文字列の 3 種類が確率的に出現する |

### テスト対象: エラー型

| # | テストケース | 種別 | 内容 |
|---|-------------|------|------|
| T12 | `DarviumError::Llm` のメッセージ確認 | 正常系 | `DarviumError::Llm("API error".into())` のエラーメッセージが正しいこと |
| T13 | `DarviumError::LlmMalformedJson` のメッセージ確認 | 正常系 | `LlmMalformedJson` が JSON パース失敗を正しく表現すること |
| T14 | エラーの `PartialEq` 比較 | 正常系 | 同一エラー値の等価性比較が成立すること |

### テスト対象: `LlmSchema` 列挙型

| # | テストケース | 種別 | 内容 |
|---|-------------|------|------|
| T15 | 全バリアントのデバッグ表示 | 正常系 | 4 バリアントすべてが `Debug` トレイトを実装し、意味のある表示を持つ |
| T16 | 全バリアントの Clone 可能性 | 正常系 | 4 バリアントすべてが `Clone` 可能であること |

## Boy Scout Rule — 翻訳可能性計画

本チケットで新規作成する `src/llm/mod.rs`（または `src/llm_client.rs`）および拡張する `src/error.rs` において、以下の翻訳可能性を確保する：

1. **関数名は動詞句**: `generate_structured` は「構造化出力を生成する」と翻訳可能。`FakeLlmClient` のメソッドも同様に動詞句＋ドメイン名詞で統一する。

2. **変数名はドメイン概念**: `prompt` / `schema` / `malformed_probability` など、LLM ドメインの標準用語を使用し、`x` / `data` / `tmp` などの汎用的な変数名を避ける。

3. **一関数一責務**: `generate_structured` は「prompt + schema → 文字列出力」という 1 つの責務のみを持つ。パース・検証は呼び出し元の責務とする。

4. **ハードコード値は名前付き定数**: `FakeLlmClient` の乱数モードにおけるデフォルト確率値は `pub const FAKE_LLM_DEFAULT_MALFORMED_PROB: f64 = 0.0` として `constants.rs` に定義する。

5. **エラー握りつぶし禁止**: `FakeLlmClient` 内でエラーを握りつぶさず、`DarviumError::LlmMalformedJson` を適切に伝播する。

6. **既存コード改善**: 本チケットのスコープ外ではあるが、`src/error.rs` に LLM エラーバリアントを追加する際、既存エラーのコメントが日本語であることを確認し、一貫性を保つ。

## Acceptance Criteria

- [ ] `LLMClient` トレイトが `Send + Sync` 境界を持ち、`Box<dyn LLMClient>` のオブジェクト安全性が確認できる
- [ ] `LlmSchema` 列挙型が 4 バリアント（`QueryDesignText`, `PatchOperations`, `SelfScore`, `FreeText`）を持つ
- [ ] `FakeLlmClient` が固定文字列モードと乱数モードの両方をサポートする
- [ ] `DarviumError::Llm(String)` および `DarviumError::LlmMalformedJson(String)` が追加されている
- [ ] T1〜T16 の全テストケースが通過する
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

- 計画: context/0003-m-2-16-llmclient/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0003-m-2-16-llmclient/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0003-m-2-16-llmclient/review.md（未作成、/review-ticket 全チェック通過後に作成）
