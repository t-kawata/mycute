---
ticket_id: 6
title: M-2-2: SearchBudget 及び RecursionGuard 初期化仕様の検証
slug: m-2-2-searchbudget-recursionguard
status: reviewed
created_at: 2026-05-22
updated_at: 2026-05-22
plan_path: /Users/shyme01/shyme/mycute/crates/darvium/tickets/context/0006-m-2-2-searchbudget-recursionguard/plan.md
implementation_path: /Users/shyme01/shyme/mycute/crates/darvium/tickets/context/0006-m-2-2-searchbudget-recursionguard/implementation.md
review_report_path: /Users/shyme01/shyme/mycute/crates/darvium/tickets/context/0006-m-2-2-searchbudget-recursionguard/review.md
---

# M-2-2: SearchBudget 及び RecursionGuard 初期化仕様の検証

## Summary

`SearchBudget`（検索予算）と `RecursionGuard`（再帰ガード）の構造体定義を RFC §13.3 に完全準拠させ、デフォルト値コンストラクタ・サチュレーティングインクリメント演算・境界値テストを実装する。加えて、使用量を追跡する `SearchBudgetSnapshot` を新規定義する。

- **関連 RFC**: §13.3（データモデル制約）、§13.6（ガード条件）
- **対応チケット**: M-2-2（Darvium-Tickets-v2.3.md L149-L156）

## Background

`SearchBudget` は `SearchWorkflow` が消費するトークン・検索呼び出し・反復・実時間の上限を束ねる bounded search 制約であり、`RecursionGuard` は SearchWorkflow が自身を再帰的に呼び出す際の深さ制限を制御する。これらの型は M-1.5 以降の state machine テストで実際に使用されるため、M-2 フェーズで正しく定義されテストされている必要がある。

現状の実装には以下の問題があり、RFC 準拠のために修正が必要である：

1. **フィールドの不一致**: 現行の `SearchBudget` は RFC で定義されていない `max_depth`/`current_depth`（本来 `RecursionGuard` の責務）を持ち、RFC 定義の `max_iterations`/`max_retrieval_calls`/`max_wall_clock_ms` が欠落している。
2. **`SearchBudgetSnapshot` 未定義**: RFC §13.3 で定義される使用量スナップショット型が存在しない。現在は使用量を `SearchBudget` 自身が持つ設計になっているが、RFC は予算上限（`SearchBudget`）と使用量（`SearchBudgetSnapshot`）を分離している。
3. **`RecursionGuard` の不完全性**: `allow_reentrant` フィールドが欠落している。また型が `usize` になっており、RFC の `u32` と異なる。
4. **impl ブロック不在**: コンストラクタ・インクリメント演算が一切実装されておらず、フィールドを直接操作する必要がある。
5. **テスト不在**: 境界値テストが一切存在しない。

## Scope

1. `SearchBudget` のフィールドを RFC §13.3 に完全準拠させる（`max_iterations: u32`, `max_retrieval_calls: u32`, `max_prompt_tokens: u64`, `max_wall_clock_ms: u64`）
2. `SearchBudgetSnapshot` を RFC §13.3 に従い新規定義（`iterations_used: u32`, `retrieval_calls_used: u32`, `prompt_tokens_used: u64`, `wall_clock_ms_used: u64`）
3. `RecursionGuard` のフィールドを RFC §13.3 に準拠させる（`max_depth: u32`, `current_depth: u32`, `allow_reentrant: bool`）
4. デフォルト値コンストラクタの実装（`SearchBudget::default()`、`RecursionGuard::default()`）
5. サチュレーティングインクリメント演算の実装:
   - `SearchBudget::try_consume_iteration(&self) -> Result<SearchBudgetSnapshot, DarviumError>`
   - `SearchBudget::try_consume_retrieval_call(&self) -> Result<SearchBudgetSnapshot, DarviumError>`
   - `SearchBudget::try_consume_prompt_tokens(&self, tokens: u64) -> Result<SearchBudgetSnapshot, DarviumError>`
   - `RecursionGuard::try_increment_depth(&mut self) -> Result<(), DarviumError>`
   - `RecursionGuard::decrement_depth(&mut self)`（再帰からの復帰時）
6. 境界値テスト
7. `constants.rs` へのデフォルト予算定数追加

## Non-scope

- `SearchBudgetSnapshot` の `wall_clock_ms_used` に対する実時間計測（Clock トレイトとの結合は実装段階で別途対応）
- SearchWorkflow 状態機械との統合（M-1.5）
- `SearchPolicyOscillation` 検出（M-1.5-3）
- 複数スレッドからの予算操作（楽観的並行性制御は M3-3）
- `SearchTrace` への `SearchBudgetSnapshot` 統合

## Investigation

### ファイル: `src/types.rs` L381-L393 — 現行定義

```rust
#[derive(Debug, Clone)]
pub struct SearchBudget {
    pub max_prompt_tokens: u64,
    pub prompt_tokens_used: u64,
    pub max_depth: usize,
    pub current_depth: usize,
}

#[derive(Debug, Clone)]
pub struct RecursionGuard {
    pub max_depth: usize,
    pub current_depth: usize,
}
```

**問題点**:
- `SearchBudget` に `prompt_tokens_used`（RFC 非存在、`SearchBudgetSnapshot` に属する）、`max_depth`/`current_depth`（RFC 非存在、`RecursionGuard` に属する）が含まれている
- `SearchBudget` に `max_iterations`、`max_retrieval_calls`、`max_wall_clock_ms` が欠落
- `RecursionGuard` に `allow_reentrant: bool` が欠落
- 型が `usize` だが RFC は `u32`

### ファイル: `src/error.rs` L39-L43 — エラー型（既存、修正不要）

```rust
#[error("Search budget exceeded")]
SearchBudgetExceeded,
#[error("Search recursion exceeded")]
SearchRecursionExceeded,
```

### ファイル: `src/constants.rs` L41-L42 — 予算関連定数

```rust
pub const MAX_PROMPT_TOKENS: u64 = 16_384;
```

現状この定数はどこからも参照されていない。`SearchBudget::default()` で使用する。

### ファイル: `Darvium-RFC-0001-Unified-v2.3-final.md` L1572-L1604 — 規範定義

RFC §13.3 の正規型定義:

```rust
struct SearchBudget {
    max_iterations:      u32,
    max_retrieval_calls: u32,
    max_prompt_tokens:   u64,
    max_wall_clock_ms:   u64,
}

struct RecursionGuard {
    max_depth:       u32,
    current_depth:   u32,
    allow_reentrant: bool,
}

struct SearchBudgetSnapshot {
    iterations_used:      u32,
    retrieval_calls_used: u32,
    prompt_tokens_used:   u64,
    wall_clock_ms_used:   u64,
}
```

### 証拠: 現行コードで RFC 準拠のフィールド名が一切使われていない

`max_iterations`、`max_retrieval_calls`、`max_wall_clock_ms`、`allow_reentrant` の 4 フィールドは `src/` 内に出現しない（grep 確認済み）。また `SearchBudgetSnapshot` も未定義。

### 証拠: impl ブロック不在

`impl SearchBudget` および `impl RecursionGuard` は `src/` 内に存在しない。全メソッドを新規実装する必要がある。

### 証拠: テスト不在

`SearchBudget` および `RecursionGuard` に対するテストは一切存在しない。

## Test Plan

### テスト対象: `src/types.rs` 内の `#[cfg(test)] mod tests`

#### T1: デフォルト値検証
- `SearchBudget::default()` が定数から適切なデフォルト値を設定することを確認
- `RecursionGuard::default()` が適切なデフォルト値（`max_depth=8`, `current_depth=0`, `allow_reentrant=false`）を持つことを確認

#### T2: サチュレーティングインクリメント境界値テスト
- `try_consume_iteration` で `iterations_used` が `max_iterations` 未満の場合に成功することを確認（正常系）
- `try_consume_iteration` で `iterations_used` が `max_iterations` 以上の場合に `SearchBudgetExceeded` を返すことを確認（異常系: 上限超過）
- `try_consume_retrieval_call` の同値類テスト（正常系＋異常系）
- `try_consume_prompt_tokens` で累積トークンが `max_prompt_tokens` を超えた場合に `SearchBudgetExceeded` を返すことを確認
- `try_consume_prompt_tokens` で累積トークンが上限ぴったりの場合に成功することを確認（境界値: 上限一致）

#### T3: RecursionGuard 境界値テスト
- `try_increment_depth` で `current_depth` が `max_depth` 未満の場合に成功（正常系）
- `try_increment_depth` で `current_depth` が `max_depth` 以上の場合に `SearchRecursionExceeded` を返す（異常系）
- `try_increment_depth` 成功後に `current_depth` が 1 増加することを確認（副作用検証）
- `decrement_depth` で `current_depth` が 1 減少することを確認
- `decrement_depth` でアンダーフローが発生しないことを確認（`current_depth=0` で呼んでもパニックしない）

#### T4: allow_reentrant フラグ検証
- `allow_reentrant = false` の状態で `try_increment_depth` を呼ぶとエラーになることを確認（初回でも）
- `allow_reentrant = true` の状態で `try_increment_depth` が `max_depth` まで正常動作することを確認

#### T5: SearchBudgetSnapshot 生成検証
- `try_consume_iteration` が正しい `iterations_used` を持つスナップショットを返すことを確認
- 複数回の消費呼び出しでスナップショットの値が累積されることを確認

### 観測テスト（観測ベース検証ファースト）

**OTS-1: 初期予算アンサンブル緩和時間計測**
- シード固定 PRNG で `SearchBudget` の初期値を変動させた 10,000 個のアンサンブルを生成
- 各アンサンブルに対し、ランダムな量のトークン/イテレーション/コールを消費する軌道をシミュレート
- 全軌道が上限境界に到達するまでの平均緩和時間 τ_relax を計測（出力形式: JSON）
- サチュレーション演算呼び出し後の状態ベクトル変化率がゼロに即時収束することを検証

## Boy Scout Rule — 翻訳可能性計画

1. **責務の分割**: 現行の `SearchBudget` は「予算上限」と「使用量追跡」と「再帰深度」という 3 責務が混在している。RFC に従い `SearchBudget`（上限のみ）・`SearchBudgetSnapshot`（使用量）・`RecursionGuard`（再帰深度）に分割する。
2. **関数名を動詞句に**: インクリメントメソッドは `increment_depth()` ではなく `try_increment_depth()` とし、Result を返すことを名前に含める。デクリメントはパニックしない設計のため `decrement_depth()` とする。
3. **マジックナンバーの定数化**: デフォルト予算値（`max_iterations` のデフォルト等）は `constants.rs` に定数として定義する。
4. **コメントで「なぜ」を説明**: サチュレーション演算がオーバーフローでなく `max_depth` との比較で停止する理由（RFC §13.6 ガード条件との対応）をコメントに記載する。

## Acceptance Criteria

- [ ] RFC §13.3 の `SearchBudget`・`RecursionGuard`・`SearchBudgetSnapshot` 定義と完全に一致するフィールドを持つ
- [ ] デフォルト値コンストラクタが適切な初期値を設定する
- [ ] サチュレーティングインクリメント演算が上限超過時に正しくエラーを返す
- [ ] 境界値テストが全パターン通過する
- [ ] 既存テストがすべて通過する
- [ ] `cargo clippy -- -D warnings` が通過する
- [ ] `cargo fmt` で整形済み

## Notes

### 成果物

- 計画: `context/0006-m-2-2-searchbudget-recursionguard/plan.md`
- 実装サマリ: `context/0006-m-2-2-searchbudget-recursionguard/implementation.md`
- レビュー報告書: `context/0006-m-2-2-searchbudget-recursionguard/review.md`
