---
ticket_id: 12
title: M-1-2: `SearchBudgetExceeded` ハードガードの遮断アサーション
slug: m-1-2-searchbudgetexceeded
status: reviewed
created_at: 2026-05-22
updated_at: 2026-05-22
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0012-m-1-2-searchbudgetexceeded/plan.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0012-m-1-2-searchbudgetexceeded/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0012-m-1-2-searchbudgetexceeded/review.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0012-m-1-2-searchbudgetexceeded/observation-20260522-230022.md
---
# M-1-2: `SearchBudgetExceeded` ハードガードの遮断アサーション

## Summary

SearchWorkflow のループ実行前に全 4 次元（iterations, retrieval_calls, prompt_tokens, wall_clock_ms）の SearchBudget 上限超過を事前検査するインターセプタ `check_budget_exceeded` を実装する。1 つでも上限超過が検出された場合、即座に `Err(SearchBudgetExceeded)` を返し、状態を `Abort` へ遷移させる純粋関数として設計する。

## Background

RFC §13.6 は「SearchBudget の上限超過時は `SearchBudgetExceeded` を返し、`Abort` へ遷移すること」と規定している。M-2-2 で実装済みの `try_consume_iteration` / `try_consume_retrieval_call` / `try_consume_prompt_tokens` は個別リソースの消費時点での事後チェックである。これに加えて、**ループ開始前**の事前検査として全次元をまとめてチェックするインターセプタが必要である。

ハードガードの目的は以下の 2 つ：
1. **Safety**: 使用量が既に上限に達している状態で新たなリソース消費に入らせない
2. **Determinism**: ガード遮断に要する命令ステップ数が入力値に依存せず一定であること（最悪時間有界性）

M-1-1 の `EvaluateCandidatesStep` が候補評価の決定論的閾値判定を担当するのに対し、M-1-2 は**ループ事前遮断**という異なる責務を負う。後続の M-1-3（`SearchRecursionExceeded`）と共に安全性ガードの基盤を形成する。

## Scope

- `check_budget_exceeded(budget, snapshot)` 純粋関数の実装
  - 4 次元すべての事前チェック（iterations, retrieval_calls, prompt_tokens, wall_clock_ms）
  - 1 つでも超過 → `Err(SearchBudgetExceeded)`
  - 使用量の消費は行わない（pure check）
- `guard_budget_or_abort` 高階インターセプタ（状態遷移送携版）
  - `check_budget_exceeded` の結果が Err の場合、状態を `Abort` に変更
- 網羅的なユニットテスト（正常系・異常系・境界値 4 次元 × 3 水準）
- 観測テスト（ガード遮断命令ステップ数の分散測定）

## Non-scope

- `try_consume_*` メソッドの修正（M-2-2 で既存、変更不要）
- `RecursionGuard` の深さ制限ガード（M-1-3）
- 実時間 wall_clock_ms の測定（Clock トレイト統合時に別途対応）
- `attempt_transition` への budget ガード組み込み（上位レイヤーの責務）

## Investigation

### RFC 交叉参照

**RFC §13.6 (ガード条件)**:
- 「SearchBudget の上限超過時は SearchBudgetExceeded を返し、Abort へ遷移すること」
- 4 次元の上限はそれぞれ独立に評価される
- 超過の定義: `used >= max`（iterations, retrieval_calls は saturated）、`used > max`（prompt_tokens, wall_clock_ms）
- `TerminalTransitionReason::BudgetExceeded` による終端理由の正当化は M-1.5-2 で実装済み

**RFC §13.3 (データモデル)**:
- `SearchBudget` の 4 フィールド: `max_iterations: u32`, `max_retrieval_calls: u32`, `max_prompt_tokens: u64`, `max_wall_clock_ms: u64`
- `SearchBudgetSnapshot` の 4 フィールド: `iterations_used: u32`, `retrieval_calls_used: u32`, `prompt_tokens_used: u64`, `wall_clock_ms_used: u64`
- `SearchBudgetExceeded` エラー型は実装済み

### 既存コード調査

**`src/types.rs`**:
- `SearchBudget` 構造体 (2806行) — 4 次元の上限値、実装済み
- `SearchBudgetSnapshot` 構造体 (2818行) — 4 次元の使用量、実装済み
- `try_consume_iteration` (2870行) — イテレーション消費 + 事後チェック
- `try_consume_retrieval_call` (2885行) — 検索呼び出し消費 + 事後チェック
- `try_consume_prompt_tokens` (2900行) — トークン消費 + 事後チェック
- `snapshot` (2919行) — 現在の使用量スナップショット取得
- `SearchState::transition_to` (268行) — 状態遷移（`Abort` への遷移を含む）
- `attempt_transition` (388行) — 発振検出付き遷移ヘルパー
- `TerminalTransitionReason::BudgetExceeded` (240行) — 予算超過理由、実装済み
- `can_terminate_with(BudgetExceeded)` → `true` (実装済み)

**`src/error.rs`**:
- `DarviumError::SearchBudgetExceeded` (40行) — 実装済み

**`src/constants.rs`**:
- `MAX_PROMPT_TOKENS` (51行) — 16,384
- `DEFAULT_MAX_ITERATIONS` (54行) — 100
- `DEFAULT_MAX_RETRIEVAL_CALLS` (57行) — 50
- `DEFAULT_MAX_WALL_CLOCK_MS` (60行) — 30,000

**`src/lib.rs`**:
- `SearchBudget`, `SearchBudgetSnapshot` は re-export 済み
- `TerminalTransitionReason` は re-export 済み
- `check_budget_exceeded` は未実装 → `types.rs` への追加が必要

**不足点**:
- `check_budget_exceeded` 関数 — 未実装（M-1-2 で追加）
- `guard_budget_or_abort` 関数 — 未実装（M-1-2 で追加）

## Test Plan

### 実装対象関数

1. **`check_budget_exceeded(budget: &SearchBudget, snapshot: &SearchBudgetSnapshot) -> Result<(), DarviumError>`** — 純粋検査関数
   - 入力: SearchBudget（上限値） + SearchBudgetSnapshot（現在使用量）
   - 出力: `Ok(())`（上限超過なし）/ `Err(SearchBudgetExceeded)`（1 つでも超過）
   - 不変条件: 使用量の消費を行わない（副作用ゼロ）
   - チェック順序: iterations → retrieval_calls → prompt_tokens → wall_clock_ms（早期 return 最適化）

2. **`guard_budget_or_abort(state: &mut SearchState, budget: &SearchBudget, snapshot: &SearchBudgetSnapshot) -> Result<(), DarviumError>`** — 状態遷移送携版インターセプタ
   - `check_budget_exceeded` を呼び出し
   - Err の場合: `*state = SearchState::Abort` で状態を変更してから Err を伝播
   - Ok の場合: そのまま Ok を伝播

### ユニットテスト一覧

#### T1: 正常系 — 上限未満での通過

| ID | 次元 | 使用量 | 上限 | 期待結果 |
|----|------|--------|------|---------|
| T1-a | iterations | 0 | 100 | `Ok(())` |
| T1-b | retrieval_calls | 0 | 50 | `Ok(())` |
| T1-c | prompt_tokens | 0 | 16384 | `Ok(())` |
| T1-d | wall_clock_ms | 0 | 30000 | `Ok(())` |
| T1-e | 全次元 | 半量 | 全上限 | `Ok(())` |

#### T2: 異常系 — 各次元の単独超過

| ID | 次元 | 使用量 | 上限 | 期待結果 |
|----|------|--------|------|---------|
| T2-a | iterations | 100 | 100 | `Err(SearchBudgetExceeded)`（飽和） |
| T2-b | retrieval_calls | 50 | 50 | `Err(SearchBudgetExceeded)`（飽和） |
| T2-c | prompt_tokens | 16385 | 16384 | `Err(SearchBudgetExceeded)`（超過） |
| T2-d | prompt_tokens | 16384 | 16384 | `Ok(())`（境界値・上限ぴったり） |
| T2-e | wall_clock_ms | 30001 | 30000 | `Err(SearchBudgetExceeded)`（超過） |
| T2-f | wall_clock_ms | 30000 | 30000 | `Ok(())`（境界値・上限ぴったり） |

#### T3: 複数次元同時超過

| ID | iterations | retrieval_calls | prompt_tokens | wall_clock_ms | 期待結果 |
|----|-----------|----------------|--------------|--------------|---------|
| T3-a | 超過 | 正常 | 正常 | 正常 | `Err(SearchBudgetExceeded)` |
| T3-b | 正常 | 超過 | 正常 | 正常 | `Err(SearchBudgetExceeded)` |
| T3-c | 正常 | 正常 | 超過 | 正常 | `Err(SearchBudgetExceeded)` |
| T3-d | 正常 | 正常 | 正常 | 超過 | `Err(SearchBudgetExceeded)` |
| T3-e | 超過 | 超過 | 超過 | 超過 | `Err(SearchBudgetExceeded)` |

#### T4: guard_budget_or_abort 状態遷移

| ID | 内容 | 期待結果 |
|----|------|---------|
| T4-a | 超過なし → 状態不変 | `Ok(())`, state は元のまま |
| T4-b | 超過あり → 状態が Abort に | `Err(SearchBudgetExceeded)`, state == Abort |
| T4-c | 終端状態から超過 → TerminalStateViolation 優先 | `Err(TerminalStateViolation)` |
| T4-d | 超過時の状態変更確認 | state が Abort に設定された後に Err 伝播 |

#### T5: 副作用ゼロ検証

| ID | 内容 |
|----|------|
| T5-a | `check_budget_exceeded` 呼び出し前後で snapshot の値が不変 |
| T5-b | `check_budget_exceeded` 呼び出しが budget の値を変更しない |

#### T6: 決定論性

| ID | 内容 |
|----|------|
| T6-a | 同一入力で 2 回呼び出した結果が完全一致 |
| T6-b | 異なる超過量でもガード結果が同一 |
| T6-c | 超過量 ΔB を sweep しても戻り値の型は不変 |

### 観測テスト

#### OTS-1: ガード遮断命令ステップ数の分散測定

`check_budget_exceeded` の呼び出しに要する処理時間を測定し、超過量 $\Delta B$ に依存しないことを確認する。

- 固定シード PRNG (`StdRng::seed_from_u64(12345)`) を使用
- $\Delta B$ を 1 から 10,000 まで対数 sweep（10 水準）
- 各水準で n = 10,000 回のガード呼び出し
- 観測量: 処理時間（ns）の平均・分散・最大・最小
- 不変条件: $\sigma^2(S_{inst}) = 0$（完全な最悪時間有界性）

#### OTS-2: guard_budget_or_abort レイテンシ分布

`guard_budget_or_abort` の状態遷移レイテンシを測定し、正常系と超過系で有意差がないことを確認する。

- 正常系 n = 10,000: 超過なしで通過
- 超過系 n = 10,000: T4-b の超過パターンで Abort 遷移
- 観測量: 両系統のレイテンシ分布（平均・分散・分位数）

## 計装方法・観測対象

### 計装方法
- 全テストは `src/types.rs` 内の `#[cfg(test)] mod tests` に追加（既存の M-2-2 / M-1.5 / M-1-1 テスト群と同じパターン）
- `StdRng::seed_from_u64(12345)` (constants::TEST_PRNG_SEED) を使用
- 観測出力は `println!` で CSV 形式の構造化テキストを `--nocapture` 経由で標準出力
- 処理時間計測は `std::time::Instant` を使用（物理クロックではなく命令ステップ数のプロキシ）

### 観測対象
| 観測量 | テスト | サンプルサイズ | 統計量 |
|--------|--------|--------------|--------|
| ガード命令ステップ数分散 | OTS-1 | 10,000/水準 | 平均・分散・最大・最小時間 |
| guard_budget_or_abort レイテンシ | OTS-2 | 10,000/系統 | 平均・分散・P50/P95/P99 |
| 4 次元 × 3 水準境界値 | T1〜T3 | 手動 | PASS/FAIL |
| 状態遷移整合性 | T4 | 手動 | PASS/FAIL |

### 較正計画
- 調整する定数: なし（本チケットは Safety Invariant の実装であり、較正対象ではない）
- ガードのチェック順序（iterations → retrieval_calls → prompt_tokens → wall_clock_ms）は性能に影響しないが、早期 return 最適化として文書化する

## Boy Scout Rule — 翻訳可能性計画

### 新規コードの方針

1. **関数名は動詞句**: `check_budget_exceeded`, `guard_budget_or_abort` — 関数呼び出しが「予算超過をチェックする」「予算をガードしてアボートする」と逐語訳できる
2. **一関数一責務**: `check_budget_exceeded` は純粋検査のみ、`guard_budget_or_abort` は状態遷移送携のみ
3. **副作用の明確化**: `check_budget_exceeded` は副作用ゼロを明示し、消費と検査を分離する
4. **エラー握りつぶし禁止**: Err は全て呼び出し元に伝播、サイレントデフォルトは禁止
5. **早期 return の明示**: 最初に超過を検出した次元で即座に return（最悪時間有界性の保障）

### 既存コードの改善

- `types.rs` 内に `check_budget_exceeded` を追加（SearchBudget 実装の直後）
- `lib.rs` の re-export に `check_budget_exceeded` を追加（必要に応じて）

## Acceptance Criteria

- [ ] `check_budget_exceeded` が全 4 次元の上限超過を事前検査する
- [ ] T1: 正常系（上限未満）で `Ok(())` を返す
- [ ] T2: 各次元の単独超過で `Err(SearchBudgetExceeded)` を返す
  - [ ] iterations: used >= max で超過
  - [ ] retrieval_calls: used >= max で超過
  - [ ] prompt_tokens: used > max で超過
  - [ ] wall_clock_ms: used > max で超過
- [ ] T2-d, T2-f: 上限ぴったりの場合は通過（prompt_tokens, wall_clock_ms）
- [ ] T3: 複数次元同時超過でも正しく検出
- [ ] T4: `guard_budget_or_abort` が状態を `Abort` に変更する
- [ ] T5: `check_budget_exceeded` が副作用ゼロである
- [ ] T6: 決定論性が保証されている
- [ ] OTS-1: ガード命令ステップ数の分散 σ² = 0 を確認
- [ ] OTS-2: guard_budget_or_abort レイテンシ分布を観測
- [ ] `cargo test` が全て通過（既存テスト含む）
- [ ] `cargo clippy -- -D warnings` が通過
- [ ] `cargo fmt` が通過
- [ ] 翻訳可能性の検証: 関数名が動詞句、副作用が明示、エラーが伝播されている

## Notes

<!--
注: このコメントは人間向けの説明である。AI は以下の手順に従うこと。

- plan_path: /plan-ticket が plan.md を作成後に frontmatter に更新する
- implementation_path: /start-ticket が implementation.md を作成後に frontmatter に更新する
- review_report_path: /review-ticket が review.md を作成後に frontmatter に更新する
- observation_report_path: /start-ticket が observation-YYYYMMDD-HHmmss.md を作成後に frontmatter に最新パスを更新する

各コマンドのワークフロー手順が frontmatter の正しい更新手順である。
-->

### 成果物

- 計画: context/0012-m-1-2-searchbudgetexceeded/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0012-m-1-2-searchbudgetexceeded/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0012-m-1-2-searchbudgetexceeded/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0012-m-1-2-searchbudgetexceeded/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
