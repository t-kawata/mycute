---
ticket_id: 5
title: チケット M-2-1.8: Clock / VirtualClock 抽象トレイトの定義
slug: m-2-18-clock-virtualclock
status: reviewed
created_at: 2026-05-22
updated_at: 2026-05-22
plan_path: /Users/shyme01/shyme/mycute/crates/darvium/tickets/context/0005-m-2-18-clock-virtualclock/plan.md
implementation_path: /Users/shyme01/shyme/mycute/crates/darvium/tickets/context/0005-m-2-18-clock-virtualclock/implementation.md
review_report_path: /Users/shyme01/shyme/mycute/crates/darvium/tickets/context/0005-m-2-18-clock-virtualclock/review.md
observation_report_path: tickets/context/0005-m-2-18-clock-virtualclock/observation.md
---
# チケット M-2-1.8: Clock / VirtualClock 抽象トレイトの定義

## Summary

`Clock` 抽象トレイトとその具象実装（`VirtualClock`, `SystemClock`, `FrozenClock`）を定義する。時間を抽象化することで、`SearchBudget`（チケット M-2-2）の時間計測を決定論的にし、M2.5-2 の deterministic replay を可能にする。

関連RFC: §v1.7（Human Time / Virtual Time 二軸モデル）、§13.6（SearchBudget ガード条件）、§18.2（タイムアウト処理）

## Background

`SearchBudget` は `wall_clock_ms_used` を持ち、実時間に依存するとテストが非決定論的になる。`WorkflowGraph` のタイムアウト処理も同様に実時間依存のため、ユニットテストで再現性が保証できない。

既存のトレイト定義（`LLMClient`, `EmbeddingProvider`）は `Send + Sync` を境界とし、`Box<dyn Trait>` によるオブジェクト安全性を保証するパターンを採用している。Clock も同一パターンに従う。

なお、全ての Human Time（`SystemTime` 経由の時間）は UTC を強制する (MUST)。`SystemClock::now_ms()` が返す `u64` は UTC 起点のミリ秒であり、タイムゾーンの概念は一切含まない。

## Scope

1. `src/clock/mod.rs` の新規作成（`src/llm/mod.rs` と同様のディレクトリ構成）
2. `src/lib.rs` への `pub mod clock;` 追加
3. 以下の型の実装:
   - `Clock` トレイト: `fn now_ms(&self) -> u64` + `fn advance(&mut self, delta_ms: u64)`
   - `VirtualClock`: 内部 `u64` カウンタ + `advance()` のみで進行（完全決定論的）
   - `SystemClock`: `SystemTime::now()` ラップ。`advance()` は no-op
   - `FrozenClock`: 常に一定値。`advance()` は no-op（テスト用）
4. `#[cfg(test)] mod tests` 内での全テスト実装

## Non-scope

- `Instant` / `Duration` 型の直接公開（内部で使用は可）
- `SystemTime::now()` 以外の時間取得（`Instant::now()` 等）の抽象化
- 非同期タイムアウト（`tokio::time::timeout`）の抽象化
- タイムゾーン・日時処理
- `SearchBudget` 自体の実装（チケット M-2-2）

## Investigation

### 既存トレイト定義パターン（`src/llm/mod.rs`）

`LLMClient` トレイトと `EmbeddingProvider` トレイトは以下の共通パターンを持つ:

```rust
// トレイト定義: Send + Sync を境界
pub trait LLMClient: Send + Sync {
    fn generate_structured(&self, prompt: &str, schema: &LlmSchema)
        -> Result<String, DarviumError>;
}

// FakeImpl: 決定論的ダミー実装
pub struct FakeLlmClient { ... }

// テスト: 同一ファイル内の `#[cfg(test)] mod tests`
// - T1: トレイト境界充足のコンパイル時検証
// - T2: Box<dyn Trait> のオブジェクト安全性
// - T3: Send + Sync 境界の検証
// - T4〜: 各 impl の振る舞い検証
```

### エラー型（`src/error.rs`）

DarviumError は `thiserror` で定義され、現在 22 バリアントを持つ。Clock に関連するエラーは未定義。`advance()` の失敗は現状 `Internal` で代替可能だが、専用バリアントを追加する方が望ましい。

### 定数定義（`src/constants.rs`）

Calibration Candidates / Safety Invariants / Environment Policy Knobs の3分類で管理されている。Clock 関連の定数（デフォルト開始時刻等）は未定義。

### テストパターン（`src/llm/mod.rs:278-794`）

516 行のテストが以下の構成で記述されている:
- コンパイル時検証: `fn assert_trait(_: &impl Trait) {}`
- オブジェクト安全性: `Box<dyn Trait>::new(...)`
- Send + Sync: `fn assert_send_sync<T: Send + Sync>(_t: &T) {}`
- 振る舞い検証: 正常系・異常系・境界値
- 観測テスト: 分布の統計的検証（埋め込みベクトル等）

Clock トレイトも同一パターンでテスト可能。

## 計装方法・観測対象

### 計装方法
`VirtualClock` の単調増加性（巻き戻し禁止）のアサーション。`Clock` トレイトを通して観測される時間の流れが、実時間または仮想時間のいずれかで一貫していることの検証。

### 観測対象
- `VirtualClock` の単調増加性
- `SystemClock` と実時間の乖離（誤差 < 1秒）
- `FrozenClock` の恒常性

### 較正計画
`CLOCK_DEFAULT_START_MS` (`src/constants.rs:69`) は Safety Invariant（変更禁止）。較正不要。

## Test Plan

### 全実装共通（T1-T3）

| ID | テスト内容 | 種別 |
|---|---|---|
| T1 | 全 3 実装が `Clock` トレイト境界を充足することのコンパイル時検証 | コンパイル時 |
| T2 | `Box<dyn Clock>` のオブジェクト安全性 | 正常系 |
| T3 | `Box<dyn Clock + Send + Sync>` がスレッド間移動可能 | 正常系 |

### VirtualClock（T4-T9）

| ID | テスト内容 | 種別 |
|---|---|---|
| T4 | 初期値が指定値または 0 であること | 正常系 |
| T5 | `advance(100)` で `now_ms()` が正確に 100ms 進行すること | 正常系 |
| T6 | 複数回の advance の累積性（100+200 = 300） | 正常系 |
| T7 | advance(0) で値が変化しないこと | 境界値 |
| T8 | 最大値付近からの advance でオーバーフローしないこと（wrapping または飽和） | 異常系/境界値 |
| T9 | 単調増加性のアサーション（巻き戻し禁止の不変条件） | 不変条件 |

### SystemClock（T10-T12）

| ID | テスト内容 | 種別 |
|---|---|---|
| T10 | `now_ms()` が実時間と大きく乖離しないこと（誤差 < 1秒） | 正常系 |
| T11 | `advance()` が no-op (パニックせず値が変化しないこと) | 正常系 |
| T12 | 連続呼び出しで値が単調増加すること | 正常系 |

### FrozenClock（T13-T15）

| ID | テスト内容 | 種別 |
|---|---|---|
| T13 | コンストラクタで指定した値を常に返すこと | 正常系 |
| T14 | 複数回呼び出しで同一値が返ること | 正常系 |
| T15 | advance() が no-op (値が変化しないこと) | 正常系 |

### 計装・観測（T16）

| ID | テスト内容 | 種別 |
|---|---|---|
| T16 | VirtualClock の経過時間分布観測（advance 100回で期待値通りの累積時間） | 観測 |

## Boy Scout Rule — 翻訳可能性計画

新規作成ファイル `src/clock/mod.rs` のため、初期状態から翻訳可能性を維持する:

- **関数名**: `now_ms`, `advance` — 動詞句として自明
- **型名**: `Clock`（名詞）、`VirtualClock`（名詞+修飾）、`SystemClock`、`FrozenClock`
- **一関数一責務**: 各 Clock impl は単一の時間軸を表現
- **ハードコード値**: デフォルト開始時刻は `constants.rs` の定数として抽出（`CLOCK_DEFAULT_START_MS: u64 = 0`）
- **エラー握りつぶし禁止**: `SystemTime::now()` のエラーは `DarviumError::Internal` で伝播
- **コメント**: 「なぜ advance が SystemClock で no-op なのか」の意図説明のみ
- **スコープ外の既存コードは触れない**（本チケットは新規ファイルのみ）

## Acceptance Criteria

- [ ] 実装要件を満たしている
- [ ] 翻訳可能性の検証が通っている
- [ ] 既存テストが通過している

## Notes

<!--
注: このコメントは人間向けの説明である。AI は以下の手順に従うこと。

- plan_path: /plan-ticket が plan.md を作成後に frontmatter に更新する
- implementation_path: /start-ticket が implementation.md を作成後に frontmatter に更新する
- review_report_path: /review-ticket が review.md を作成後に frontmatter に更新する

各コマンドのワークフロー手順が frontmatter 更新の正しい手順である。
-->

### 成果物

- 計画: context/0005-m-2-18-clock-virtualclock/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0005-m-2-18-clock-virtualclock/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0005-m-2-18-clock-virtualclock/review.md（未作成、/review-ticket 全チェック通過後に作成）
