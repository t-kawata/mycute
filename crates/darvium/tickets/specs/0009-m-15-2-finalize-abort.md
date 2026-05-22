---
ticket_id: 9
title: M-1.5-2: 終端状態（Finalize / Abort）非再入不変条件の強制
slug: m-15-2-finalize-abort
status: reviewed
created_at: 2026-05-22
updated_at: 2026-05-22
plan_path: /Users/shyme01/shyme/mycute/crates/darvium/tickets/context/0009-m-15-2-finalize-abort/plan.md
implementation_path: /Users/shyme01/shyme/mycute/crates/darvium/tickets/context/0009-m-15-2-finalize-abort/implementation.md
review_report_path: /Users/shyme01/shyme/mycute/crates/darvium/tickets/context/0009-m-15-2-finalize-abort/review.md
---
# M-1.5-2: 終端状態（Finalize / Abort）非再入不変条件の強制

## Summary

`SearchState` Enum に状態変更メソッド `transition_to(&mut self, next: SearchState) -> Result<(), DarviumError>` を実装し、終端状態（`Finalize` / `Abort`）からの再遷移をランタイムで阻止するガードロジックを追加する。また、終端状態への遷移を許可する正当な理由（予算超過・再帰超過・明示的 Abort・正常完了）と、許可しない理由（候補単体 failure）を区別する補助判定器 `can_terminate_with` を設計に含める。

- **関連 RFC**: §13.5（終端状態非再入不変条件）、§13.6（ガード条件）
- **対応チケット**: M-1.5-2（Darvium-Tickets-v2.3.md L176-L183）

## Background

RFC §13.5 は「`Finalize` と `Abort` は終端状態であり、終端後に再遷移してはならない (MUST NOT)」と規定する。M-1.5-1 では `is_legal_transition` 純粋関数により「どの遷移が合法か」を定義したが、これは**クエリ（問い合わせ）** に過ぎず、実際の状態変更時に違法遷移を阻止する**ガード（実行時強制）** は存在しない。

また v2.3 では、終端状態への遷移は以下の正当な理由でのみ許可される：
- **SearchBudget 上限超過**（§13.6）
- **RecursionGuard 深さ超過**（§13.6）
- **明示的 Abort 理由**（unsafe transition 検出等）
- **正常完了**（REUSE/PATCH 十分、COMPOSE 成立、NEW 採択）

これに対し、「候補単体の failure」（例：1件の候補が無効だった）は終端理由として不十分であり、残候補が存在する限り SearchWorkflow は継続可能状態に留まらなければならない。

本チケットでは以下を実装する：
1. **`transition_to` メソッド**: 現在状態が終端状態の場合に `TerminalStateViolation` を返す。合法遷移の場合は状態を更新する。
2. **`TerminalTransitionReason` Enum + `can_terminate_with` 補助判定器**: どのような理由であれば終端状態への遷移を許可するかを明示する。

## Scope

1. **`transition_to(&mut self, next: SearchState) -> Result<(), DarviumError>` メソッドの実装**
   - `SearchState` に impl ブロックで追加（`src/types.rs`）
   - 現在状態が `Finalize` または `Abort` の場合 → `Err(DarviumError::TerminalStateViolation)`
   - 遷移が違法（`!is_legal_transition(current, next)`）の場合 → `Err(DarviumError::SearchValidation("..."))`
   - 合法遷移の場合 → `self = next` し `Ok(())`

2. **`TerminalTransitionReason` Enum の実装**
   - `src/types.rs` に追加（SearchState の近傍）
   - バリアント: `BudgetExceeded`, `RecursionExceeded`, `ExplicitAbort`, `NormalCompletion`, `SingleCandidateFailure`
   - `#[derive(Debug, Clone, Copy, PartialEq, Eq)]`

3. **`fn can_terminate_with(reason: TerminalTransitionReason) -> bool` の実装**
   - `BudgetExceeded` → `true`（§13.6 ガード条件）
   - `RecursionExceeded` → `true`（§13.6 ガード条件）
   - `ExplicitAbort` → `true`（unsafe transition 等）
   - `NormalCompletion` → `true`（REUSE/PATCH/COMPOSE/NEW 正常成立）
   - `SingleCandidateFailure` → `false`（単一候補の failure では終端しない）

4. **公開 API の更新**: `src/lib.rs` で `TerminalTransitionReason` を `pub use`

## Non-scope

- `SearchPolicyOscillation` 検出エンジン — チケット M-1.5-3
- `SearchStep` Enum の実装 — 後続チケット
- `SearchOutcome` Enum の実装 — 後続チケット
- マルチスレッド並行パルス注入の本格実装 — 観測テスト範囲内で最小限実施
- すでに実装済みの `is_legal_transition` の変更

## Investigation

### 現状調査（2026-05-22）

**1. SearchState Enum は実装済み**（`src/types.rs:167-184`）
- 8 バリアント（Init, Retrieve, Evaluate, Refine, Compose, ProposeNew, Finalize, Abort）を完全に網羅
- `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]` 付与済み
- 終端状態（Finalize, Abort）のコメントで「この状態からの遷移は全て違法」と明記

**2. `is_legal_transition` 純粋関数は実装済み**（`src/types.rs:202-222`）
- 全 16 合法遷移を `matches!` マクロで網羅
- 終端状態からの全遷移を違法として正しく判定する
- ただしこれは **クエリ（判定）のみ** であり、状態変更のガードではない

**3. `transition_to` メソッドは未実装**
- `grep transition_to src/` で 0 件
- SearchState に対する impl ブロック自体が存在しない

**4. エラー型は準備済み**（`src/error.rs:36-37`）
```rust
#[error("Terminal state violation")]
TerminalStateViolation,
```
- このバリアントは現在どこからも参照されていない（未使用）
- 加えて `SearchValidation(String)`（`src/error.rs:33-34`）も一般検証エラーとして利用可能

**5. SearchState は公開 API としてエクスポート済み**（`src/lib.rs:28`）
```rust
pub use types::{RecursionGuard, SearchBudget, SearchBudgetSnapshot, SearchState};
```

**6. 既存テスト（M-1.5-1）**
- 全合法遷移 16 個の個別テスト（T1-a〜T1-p）
- 違法遷移 7 グループの網羅的テスト（T2-a〜T2-g）
- 総当たり 8×8 = 64 ペアマトリクステスト（T3）
- 境界値テスト（SearchState のメモリサイズ）
- 観測テスト OTS-1/OTS-2/OTS-3（スペクトル半径・平均吸収時間）
- 全て `#[cfg(test)] mod tests` 内（`src/types.rs:840-1340`）

**7. RFC §13.5 終端状態非再入（L1655）**
> `Finalize` と `Abort` は終端状態であり、終端後に再遷移してはならない (MUST NOT)。

**8. RFC §13.6 ガード条件（L1674-L1679）**
> - SearchBudget の上限超過時は `SearchBudgetExceeded` を返し、`Abort` へ遷移すること (MUST)。
> - RecursionGuard の深さ超過時は `SearchRecursionExceeded` を返し...
> - side-effect safety invariant に反する SearchStep 遷移...は `UnsafeSearchTransition` として拒否すること (MUST)。

## Test Plan

テストは `src/types.rs` の既存 `#[cfg(test)] mod tests` 内に追加する。以下のテストを実装する：

### T1: `transition_to` 終端状態ガード（T1-a〜T1-b）

終端状態からの遷移試行が `TerminalStateViolation` を返すことを個別に検証する。

| ID | 初期状態 | 遷移先 | 期待結果 |
|----|---------|--------|---------|
| T1-a | `Finalize` | `Init` | `Err(TerminalStateViolation)` |
| T1-b | `Abort` | `Init` | `Err(TerminalStateViolation)` |

### T2: `transition_to` 終端状態ガード全網羅（T2-a〜T2-b）

終端状態からの**全種類**の遷移試行（各 8 通り × 2 終端状態）が `TerminalStateViolation` を返すことをループで一括検証する。

| ID | 終端状態 | 試行遷移先 | 件数 |
|----|---------|-----------|------|
| T2-a | `Finalize` | 全 8 状態 | 8 |
| T2-b | `Abort` | 全 8 状態 | 8 |

### T3: `transition_to` 合法遷移成功（T3-a〜T3-p）

合法遷移の各パターンで `transition_to` が成功し、状態が正しく更新されることを検証する。

| ID | 遷移 | 期待 |
|----|------|------|
| T3-a | Init -> Retrieve | Ok(()) |
| T3-b | Init -> Abort | Ok(()) |
| T3-c | Retrieve -> Evaluate | Ok(()) |
| T3-d | Retrieve -> Abort | Ok(()) |
| T3-e | Evaluate -> Refine | Ok(()) |
| T3-f | Evaluate -> Compose | Ok(()) |
| T3-g | Evaluate -> Finalize | Ok(()) |
| T3-h | Evaluate -> Abort | Ok(()) |
| T3-i | Refine -> Retrieve | Ok(()) |
| T3-j | Refine -> ProposeNew | Ok(()) |
| T3-k | Refine -> Abort | Ok(()) |
| T3-l | Compose -> Finalize | Ok(()) |
| T3-m | Compose -> Refine | Ok(()) |
| T3-n | Compose -> Abort | Ok(()) |
| T3-o | ProposeNew -> Finalize | Ok(()) |
| T3-p | ProposeNew -> Abort | Ok(()) |

### T4: `transition_to` 違法遷移拒否

非終端状態からの違法遷移が `Err(DarviumError::SearchValidation(...))` を返すことを検証する。
M-1.5-1 の T2-a〜T2-g と同じ違法ペアを使用するが、`is_legal_transition` の代わりに `transition_to` を呼び出す。

### T5: `can_terminate_with` 判定の正当性

| ID | 理由 | 期待 |
|----|------|------|
| T5-a | `BudgetExceeded` | `true` |
| T5-b | `RecursionExceeded` | `true` |
| T5-c | `ExplicitAbort` | `true` |
| T5-d | `NormalCompletion` | `true` |
| T5-e | `SingleCandidateFailure` | `false` |

### T6: 状態更新の正確性

`transition_to` 成功後に `self` が正しい状態に変更されていることを確認する。

### 外部依存

Mock / Stub は不要。`transition_to` は純粋メソッドであり、外部状態に依存しない。

## 計装方法・観測対象

### 計装方法

1. **終端状態ガード計装**: 全 16 通りの終端状態遷移試行（Finalize からの 8 通り + Abort からの 8 通り）を `println!` で構造化出力（CSV 形式）し、全てが `TerminalStateViolation` で拒否されることを観測する。

2. **マルチスレッドパルス注入（簡易版）**: 終端状態に固定された `Arc<Mutex<SearchState>>` に対し、10 スレッドから各 10,000 回（計 100,000 回）の `transition_to` 呼び出しを注入する。全試行が `TerminalStateViolation` を返し、状態が終端状態のまま維持されることを確認する。

### 観測対象

- **OTS-1: 終端状態維持率**: 100,000 回のパルス注入後も状態が終端（`Finalize` / `Abort`）のまま維持されている率。期待値: 100%。
- **OTS-2: ガードレイテンシ分布**: パルス注入各試行の処理時間 $\tau_{gate}$ を計測し、平均・最大・最小を観測する。異常なレイテンシ（ガードロジックのバグを示唆する極端値）の有無を確認。
- **OTS-3: `can_terminate_with` 判定表**: 全 5 理由の判定結果を構造化出力し、`SingleCandidateFailure` のみ `false` であることを確認。

### 較正計画

本チケットに較正対象の定数は存在しない。終端状態非再入不変条件は Safety Invariant であり、RFC 改訂なしでは変更禁止。

## Boy Scout Rule — 翻訳可能性計画

本チケットで新規実装するコードは以下の方針に従う：

1. **関数名は動詞句**: `transition_to` — 「次の状態へ遷移する」、`can_terminate_with` — 「この理由で終端できるか」と散文として読める
2. **変数名はドメイン概念**: `self`（現在状態）、`next`（遷移先）、`reason`（終端理由）— 状態機械のドメインを素直に表現
3. **一関数一責務**: `transition_to` は状態変更とガードのみ。終端理由の判定は `can_terminate_with` に分離
4. **ハードコードは none**: エラーメッセージのみ文字列リテラルを使用（デバッグ可能性のため）
5. **エラー握りつぶし禁止**: `transition_to` は `Result` を返し、エラーは呼び出し元で処理
6. **翻訳可能性**: `transition_to` のガードロジックは「もし現在状態が終端なら TerminalStateViolation を返す」と逐語訳できる構造を維持
7. **既存コード改善**: 既存の `is_legal_transition` 関数は触らない。本チケットはその上位レイヤー（ランタイムガード）を追加する

## Acceptance Criteria

- [ ] `transition_to(&mut self, next: SearchState) -> Result<(), DarviumError>` が実装されている
- [ ] 終端状態（`Finalize` / `Abort`）からの `transition_to` が全て `Err(TerminalStateViolation)` を返す
- [ ] 合法遷移の `transition_to` が成功し、状態が正しく更新される
- [ ] 違法遷移（非終端状態からの違法ペア）の `transition_to` が `Err(SearchValidation(...))` を返す
- [ ] `TerminalTransitionReason` Enum が 5 バリアントを網羅している
- [ ] `can_terminate_with` が `SingleCandidateFailure` のみ `false` を返す
- [ ] マルチスレッドパルス注入テスト（10スレッド × 10,000回 = 100,000回）で終端状態維持率 100%
- [ ] `cargo test` が全テストを PASS する
- [ ] 既存テスト（M-1.5-1）が通過している

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

- 計画: context/0009-m-15-2-finalize-abort/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0009-m-15-2-finalize-abort/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0009-m-15-2-finalize-abort/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0009-m-15-2-finalize-abort/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
