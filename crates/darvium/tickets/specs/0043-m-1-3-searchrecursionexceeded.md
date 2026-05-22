---
ticket_id: 43
title: "M-1-3: SearchRecursionExceeded 深さ制限ガードの強制"
slug: m-1-3-searchrecursionexceeded
status: reviewed
created_at: 2026-05-23
updated_at: 2026-05-23
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0043-m-1-3-searchrecursionexceeded/plan.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0043-m-1-3-searchrecursionexceeded/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0043-m-1-3-searchrecursionexceeded/review.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0043-m-1-3-searchrecursionexceeded/observation-20260523-003310.md
---

# M-1-3: `SearchRecursionExceeded` 深さ制限ガードの強制

## Summary

SearchWorkflow の再入呼び出し時に `RecursionGuard` の深さ上限超過を事前検査し、超過時は `Err(SearchRecursionExceeded)` を返して状態を `Abort` へ遷移させるインターセプタ `guard_recursion_or_abort` を実装する。また、深さ上限遮断時のメモリアロケーションゼロ性（アロケーション増分累積値 $\sum \Delta A_{bytes} = 0$）を global_allocator 計装により検証する観測テストを含む。

## Background

RFC §13.6 は「RecursionGuard の深さ超過時は `SearchRecursionExceeded` を返し、SearchWorkflow は SearchWorkflow を再入してはならない」と規定している。M-2-2 で実装済みの `try_increment_depth` / `decrement_depth` は RecursionGuard の個別操作メソッドである。これに加えて、**サーチエンジンループの事前ガード**として、深さ超過時の状態遷移（`Abort`）を一貫して行うインターセプタと、統合テストが必要である。

M-1-3 の位置づけ：
- M-1-2（`SearchBudgetExceeded`）は 4 次元の予算超過を事前検査するハードガード
- M-1-3 は再帰深さ制限を検査するハードガード
- 両者合わせて SearchWorkflow ループの安全性基盤（Safety Invariant）を形成する

### 深さ制限ガードの特性

RecursionGuard は以下の 3 つの Safety Invariant を持つ：

1. **allow_reentrant = false 時は全拒否**: 再入が許可されていない場合、最初の 1 回目の呼び出しも `SearchRecursionExceeded` となる
2. **深さ飽和時の遮断**: `current_depth >= max_depth` に達した以降のすべてのインクリメント試行は `SearchRecursionExceeded` となる
3. **メモリアロケーションゼロ性**: 深さ上限に達した状態で遮断ロジックが発動する場合、追加のメモリアロケーションが発生してはならない（スタックフレーム生成以外のヒープアロケーションゼロ）

3 番目の特性は特に重要である。深さ制限ガードが「ガードとして機能しているにもかかわらず、そのガード自体が新たなメモリプレッシャーを生む」という逆説的状況を防止する。`SearchRecursionExceeded` の遮断ロジックは、以下の不変条件を満たさなければならない：

$$\sum \Delta A_{bytes} = 0 \quad \text{(遮断発動中のアロケーション増分累積値ゼロ)}$$

これは、深さ上限到達後に $10^4$ 回の連続遮断発動を行った場合でも、ヒープアロケーションが一切発生しないことを意味する。ガードロジックはスタック上のレジスタのみで閉じていなければならない。

## Scope

- `guard_recursion_or_abort(guard: &mut RecursionGuard, state: &mut SearchState) -> Result<(), DarviumError>` インターセプタ関数の実装
  - `guard.try_increment_depth()` を呼び出し
  - Err の場合: `*state = SearchState::Abort` で状態を変更してから Err を伝播
  - 終端状態からの呼び出しは `TerminalStateViolation` として拒否（M-1.5-2 不変条件との整合性）
- 網羅的なユニットテスト（正常系・異常系・境界値・状態遷移・副作用ゼロ）
- 観測テスト（global_allocator 計装によるアロケーションゼロ性検証 + スタックフレーム変位測定）

## Non-scope

- `RecursionGuard` 構造体のフィールド定義（M-2-2 で既存）
- `try_increment_depth` / `decrement_depth` メソッドの修正（M-2-2 で既存）
- `check_recursion_exceeded` の独立関数（`try_increment_depth` がメソッドとして既に存在するため不要）
- SearchBudget 関連のガード（M-1-2）
- 発振検出（M-1.5-3）
- `attempt_transition` への recursion ガード組み込み（上位レイヤーの責務）

## Investigation

### RFC 交叉参照

**RFC §13.6 (ガード条件)**:
- 「RecursionGuard の深さ超過時は `SearchRecursionExceeded` を返し、SearchWorkflow は SearchWorkflow を再入してはならない (MUST)」
- 超過の定義: `current_depth >= max_depth`
- `allow_reentrant = false` の場合は常に超過

**RFC §13.3 (データモデル)**:
- `RecursionGuard` の 3 フィールド: `max_depth: u32`, `current_depth: u32`, `allow_reentrant: bool`
- `RecursionGuard::default()` → `max_depth = DEFAULT_RECURSION_MAX_DEPTH (8)`, `current_depth = 0`, `allow_reentrant = false`
- `SearchRecursionExceeded` エラー型は実装済み

**RFC §15.1 (SearchWorkflow 全体フロー)**:
- SearchWorkflow は `RecursionGuard` を入力として受け取る
- 再帰的呼び出しのたびに `try_increment_depth` を呼び出し、上限超過時は即座にガード遮断

### 既存コード調査

**`src/types.rs`**:

- `RecursionGuard` 構造体 (3464行) — `max_depth`, `current_depth`, `allow_reentrant`、実装済み
- `RecursionGuard::new()` (3636行) — 全フィールド指定コンストラクタ、実装済み
- `RecursionGuard::default()` (3624行) — DEFAULT_RECURSION_MAX_DEPTH = 8, current_depth = 0, allow_reentrant = false
- `RecursionGuard::try_increment_depth()` (3649行) — 深さインクリメント + 上限検査、実装済み
- `RecursionGuard::decrement_depth()` (3664行) — saturating_sub による安全なデクリメント、実装済み
- `SearchState::transition_to()` (268行) — 状態遷移（`Abort` への遷移を含む）
- `attempt_transition()` (388行) — 発振検出付き遷移ヘルパー
- `TerminalTransitionReason::RecursionExceeded` (241行) — 深さ超過理由、実装済み
- `can_terminate_with(RecursionExceeded)` → `true` (実装済み)

**`src/error.rs`**:
- `DarviumError::SearchRecursionExceeded` (43行) — 実装済み

**`src/constants.rs`**:
- `DEFAULT_RECURSION_MAX_DEPTH` (63行) — 8 (Safety Invariant、変更禁止)

**`src/lib.rs`**:
- `RecursionGuard` は re-export 済み
- `TerminalTransitionReason` は re-export 済み
- `guard_recursion_or_abort` は未実装 → `types.rs` への追加が必要

**既存のユニットテスト**:
以下のテストは M-2-2 で既に実装済み（types.rs 690行〜）：
- `recursion_guard_default_values` — デフォルト値検証
- `increment_depth_normal` — 正常インクリメント
- `increment_depth_exceeded` — 上限超過
- `decrement_depth_reduces` — デクリメント
- `decrement_depth_no_underflow` — アンダーフロー防止
- `reentrant_disabled_fails_always` — allow_reentrant = false の全拒否
- `reentrant_enabled_allows_depth` — allow_reentrant = true の深さ許容

**不足点**:
- `guard_recursion_or_abort` 関数 — 未実装（M-1-3 で追加）
- 深さ上限遮断時のアロケーションゼロ性観測テスト — 未実装（M-1-3 で追加）
- スタックフレーム変位追跡テスト — 未実装（M-1-3 で追加）
- 統合テスト（サーチエンジンループ内での RecursionGuard 動作検証）— 未実装（M-1-3 で追加）

## Test Plan

### 実装対象関数

1. **`guard_recursion_or_abort(guard: &mut RecursionGuard, state: &mut SearchState) -> Result<(), DarviumError>`** — 深さ超過時の状態遷移交携版インターセプタ
   - 入力: `RecursionGuard`（可変参照）+ `SearchState`（可変参照）
   - 出力: `Ok(())`（深さ上限未満 / 正常）/ `Err(SearchRecursionExceeded)`（超過）/ `Err(TerminalStateViolation)`（終端状態から呼び出し）
   - 処理順序:
     1. 終端状態（Finalize / Abort）からの呼び出しは TerminalStateViolation
     2. `guard.try_increment_depth()` を呼び出し
     3. Err の場合: 状態を `Abort` に変更してから Err を伝播
     4. Ok の場合: そのまま Ok を伝播
   - 復帰後は呼び出し元が `guard.decrement_depth()` を呼ぶ責任を負う

### ユニットテスト一覧

#### T1: 正常系 — 深さ上限未満での通過

| ID | allow_reentrant | max_depth | current_depth(事前) | 期待結果 |
|----|-----------------|-----------|--------------------|---------|
| T1-a | true | 8 | 0 | `Ok(())`, current_depth = 1 |
| T1-b | true | 8 | 3 | `Ok(())`, current_depth = 4 |
| T1-c | true | 8 | 7 | `Ok(())`, current_depth = 8（上限ぴったり） |

#### T2: 異常系 — 深さ超過・再入禁止

| ID | allow_reentrant | max_depth | current_depth(事前) | 期待結果 |
|----|-----------------|-----------|--------------------|---------|
| T2-a | true | 3 | 3 | `Err(SearchRecursionExceeded)`, current_depth = 3（不変） |
| T2-b | true | 3 | 4 | `Err(SearchRecursionExceeded)`, current_depth = 4（不変） |
| T2-c | false | 8 | 0 | `Err(SearchRecursionExceeded)`, current_depth = 0（不変） |
| T2-d | true | 0 | 0 | `Err(SearchRecursionExceeded)`, current_depth = 0（不変） |

#### T3: guard_recursion_or_abort 状態遷移

| ID | 内容 | 期待結果 |
|----|------|---------|
| T3-a | 深さ超過なし → 状態不変 | `Ok(())`, state は元のまま, current_depth = 1 |
| T3-b | 深さ超過 → 状態が Abort に | `Err(SearchRecursionExceeded)`, state == Abort, current_depth 不変 |
| T3-c | 終端状態から超過 → TerminalStateViolation 優先 | `Err(TerminalStateViolation)`, state 不変 |
| T3-d | allow_reentrant=false → 直ちに Abort | `Err(SearchRecursionExceeded)`, state == Abort |
| T3-e | 超過時の状態変更確認 | state が Abort に設定された後に Err 伝播 |

#### T4: 副作用ゼロ検証（current_depth 変更以外）

| ID | 内容 |
|----|------|
| T4-a | `try_increment_depth` Err 時は current_depth が不変 |
| T4-b | `guard_recursion_or_abort` Err 時は guard の全フィールドが不変 |
| T4-c | 正常系でも呼び出し前後で SearchState が不変 |

#### T5: 決定論性

| ID | 内容 |
|----|------|
| T5-a | 同一入力で 2 回呼び出した結果が完全一致 |
| T5-b | 異なる超過量（current_depth = max_depth + 1 〜 +1000）でもガード結果が同一 |
| T5-c | 超過量 ΔD を sweep しても戻り値の型は不変 |

#### T6: 統合テスト — サーチエンジンループ模擬

| ID | 内容 | 期待結果 |
|----|------|---------|
| T6-a | max_depth=3 で 3 回の呼び出し → 全成功 | Ok × 3, current_depth = 3 |
| T6-b | max_depth=3 で 4 回目の呼び出し → ブロック | 4 回目のみ Err, state == Abort |
| T6-c | ブロック後の decrement_depth → アンダーフローなし | 3→2→1→0→0...（安全） |
| T6-d | 呼び出し→復帰のペアで current_depth が元に戻る | depth: 0→1→0→1→0→... |
| T6-e | allow_reentrant=false で 1 回目すら通らない | Err, state == Abort |

### 観測テスト

#### OTS-1: 深さ上限遮断時のアロケーション増分累積値ゼロ検証

`SearchRecursionExceeded` 遮断ロジックが連続 $10^4$ 回発動した状態におけるヒープアロケーション量を計測し、$\sum \Delta A_{bytes} = 0$ を検証する。

- **計装方法**: `std::alloc::GlobalAlloc` をラップするカスタムアロケータ（`CountingAllocator`）を `#[global_allocator]` に設定し、アロケーション呼び出し（`alloc` / `dealloc`）をフックする
- **テスト設定**: `RecursionGuard { max_depth: 3, current_depth: 3, allow_reentrant: true }`（既に上限到達）
- **発動回数**: n = 10,000 回の連続 `guard_recursion_or_abort` 呼び出し
- **観測量**:
  - アロケーション総回数 $N_{alloc}$（期待値: 0）
  - アロケーション増分累積値 $\sum \Delta A_{bytes}$（期待値: 0）
  - ディアロケーション総回数 $N_{dealloc}$（期待値: 0）
- **不変条件**: $N_{alloc} = 0$ かつ $\sum \Delta A_{bytes} = 0$（ヒープアロケーション絶対禁止）

#### OTS-2: スタックフレーム変位のカットオフ境界測定

深さ $d = d_{max}$ に到達しガードが発動した前後で、スタックフレームの成長率が不連続に 0 になることを確認する。

- **計装方法**: 各深度レベル（$d = 0, 1, 2, 3, 4$）におけるスタックフレームポインタのアドレスをローカル変数のアドレスとして間接的にサンプリング
- **計測手順**:
  1. $d = 0$（初期状態）でスタック上の変数アドレスを記録
  2. $d = 1, 2, 3$（正常呼び出し）で各深度の変数アドレスを記録
  3. $d = 3$（上限到達）以降でガード遮断
  4. 各深度間のスタックフレーム変位 $\Delta SP(d) = SP(d) - SP(d-1)$ を計算
- **観測量**:
  - 各深度間のスタックフレーム変位 $\Delta SP(d)$
  - $d_{max}$ を超えた後の $\Delta SP(d)$ の変化率（期待値: 0 への不連続カットオフ）
- **不変条件**: $d > d_{max}$ における呼び出しツリートポロジー階層の成長率が完全に 0

## 計装方法・観測対象

### 計装方法
- ユニットテスト: `src/types.rs` 内の `#[cfg(test)] mod tests` に追加（既存の M-2-2 / M-1.5 / M-1-1 / M-1-2 テスト群と同じパターン）
- 観測テスト: 同一ファイル内に追加（OTS-1 は CountingAllocator を `#[global_allocator]` で注入）
- `StdRng::seed_from_u64(12345)` (constants::TEST_PRNG_SEED) を使用
- 観測出力は `println!` で CSV 形式の構造化テキストを `--nocapture` 経由で標準出力
- アロケーション計測は `CountingAllocator`（GlobalAlloc ラッパー）で実現

### 観測対象

| 観測量 | テスト | サンプルサイズ | 統計量 |
|--------|--------|--------------|--------|
| アロケーション増分累積値 $\sum \Delta A_{bytes}$ | OTS-1 | 10,000 回発動 | 総和・平均・回数（いずれも 0） |
| アロケーション回数 $N_{alloc}$ | OTS-1 | 10,000 回発動 | 0 であることの確認 |
| スタックフレーム変位 $\Delta SP(d)$ | OTS-2 | 深度 0 〜 8 | 各深度間の変位の実測値 |
| カットオフ境界の不連続性 | OTS-2 | 上限境界前後 | 成長率の段差 |

### 較正計画
- 調整する定数: なし（本チケットは Safety Invariant の実装であり、較正対象ではない）
- `DEFAULT_RECURSION_MAX_DEPTH = 8` は Safety Invariant（変更禁止）
- 観測テストは較正ではなく不変条件検証（ガードの結晶化特性確認）が目的

## Boy Scout Rule — 翻訳可能性計画

### 新規コードの方針

1. **関数名は動詞句**: `guard_recursion_or_abort` — 「再帰をガードしてアボートする」と逐語訳できる
2. **一関数一責務**: `guard_recursion_or_abort` は深さ検査 + 状態遷移交携のみ
3. **副作用の明確化**: 深さ超過時は状態を Abort に変更することを明示。正常時は `current_depth` のみ変更
4. **エラー握りつぶし禁止**: Err は全て呼び出し元に伝播、サイレントデフォルトは禁止
5. **早期 return の明示**: 終端状態チェック、allow_reentrant チェックの順で即座に return（最悪時間有界性）
6. **decrement_depth との対称性**: guard_recursion_or_abort を通った場合、呼び出し元は必ず decrement_depth を呼ぶこと（RAII 的な使用パターンを文書化）

### 既存コードの改善

- `types.rs` 内に `guard_recursion_or_abort` を追加（RecursionGuard 実装の直後）
- 既存の RecursionGuard テストに M-1-3 のテストを追加（同じテストモジュール内）
- 既存の `decrement_depth` のアンダーフロー防止安全性は維持（saturating_sub）

## Acceptance Criteria

- [ ] `guard_recursion_or_abort` が深さ上限超過を事前検査する
- [ ] T1: 正常系（深さ上限未満）で `Ok(())` を返し current_depth が +1 される
- [ ] T2: 異常系（深さ超過・再入禁止）で `Err(SearchRecursionExceeded)` を返し current_depth が不変
  - [ ] T2-a: current_depth >= max_depth で超過
  - [ ] T2-c: allow_reentrant = false で全拒否
  - [ ] T2-d: max_depth = 0 で全拒否
- [ ] T3: `guard_recursion_or_abort` が深さ超過時に状態を `Abort` に変更する
  - [ ] T3-c: 終端状態からの呼び出しは `TerminalStateViolation`
- [ ] T4: 副作用ゼロ（Err 時は current_depth が不変）
- [ ] T5: 決定論性が保証されている
- [ ] T6: 統合テスト（サーチエンジンループ模擬）で RecursionGuard が正しく機能
- [ ] OTS-1: 深さ上限遮断時のアロケーション増分累積値 $\sum \Delta A_{bytes} = 0$
- [ ] OTS-2: スタックフレーム変位のカットオフ境界が確認できる
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
- observation_report_path: 観測テスト実行後に observation.md を作成して frontmatter に更新する
-->
