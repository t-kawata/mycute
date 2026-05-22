---
ticket_id: 8
title: "M-1.5-1: SearchState 合法状態遷移マトリクスの実装"
slug: searchstate
status: reviewed
created_at: 2026-05-22
updated_at: 2026-05-22
plan_path: /Users/shyme01/shyme/mycute/crates/darvium/tickets/context/0008-searchstate/plan.md
implementation_path: /Users/shyme01/shyme/mycute/crates/darvium/tickets/context/0008-searchstate/implementation.md
review_report_path: /Users/shyme01/shyme/mycute/crates/darvium/tickets/context/0008-searchstate/review.md
---

# M-1.5-1: SearchState 合法状態遷移マトリクスの実装

## Summary

`SearchState` Enum（8 状態）および現状態と次状態のペアが合法か否かを判定する純粋関数 `is_legal_transition(current, next) -> bool` を実装する。RFC §13.5 で定義された有向状態機械の遷移規則をコードとして形式化し、総当たりマトリクステストで全合法経路・違法経路を検証する。また、候補単体の failure が発生しても残候補が存在する限り状態機械が `Abort` ではなく継続可能状態へ留まることを、状態系列の設計原理として明文化する。

- **関連 RFC**: §13.5（状態遷移規則）、§13.6（ガード条件）
- **対応チケット**: M-1.5-1（Darvium-Tickets-v2.3.md L169-L175）

## Background

SearchWorkflow は GMR Retrieval Core を呼び出してワークフロー候補を収集・評価し、最終的に REUSE / PATCH / COMPOSE / NEW のいずれかの outcome に至るメタワークフローである。その実行過程は `SearchState` 列挙型で表現される有限状態機械としてモデル化され、RFC §13.5 は以下の有向遷移規則を規定する：

```text
Init -> Retrieve -> Evaluate
Evaluate -> Finalize        (REUSE / PATCH が十分)
Evaluate -> Compose         (単独候補では不十分だが組成候補あり)
Evaluate -> Refine          (候補不足・policy 改善が必要)
Compose -> Finalize         (COMPOSE 成立)
Compose -> Refine           (compose 不成立)
Refine -> Retrieve          (requery)
Refine -> ProposeNew        (既存候補再利用の期待値が低い)
ProposeNew -> Finalize      (NEW 採択)
任意状態 -> Abort           (budget / recursion / unsafe transition)
```

加えて、`Finalize` と `Abort` は終端状態であり、終端後に再遷移してはならない (MUST NOT)。

現状、`SearchState` は未実装である。このチケットでは上記の状態機械を純粋関数として実装し、後続チケット（M-1.5-2 終端状態非再入、M-1.5-3 発振検出、M-1 ポリシー評価）の基盤を提供する。

## Scope

1. **`SearchState` Enum の実装**: RFC §13.5 に基づく 8 バリアント（`Init`, `Retrieve`, `Evaluate`, `Refine`, `Compose`, `ProposeNew`, `Finalize`, `Abort`）
   - `#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]` を付与
   - `src/types.rs` に配置（既存の検索関連型と同じモジュール）
2. **`is_legal_transition(current: SearchState, next: SearchState) -> bool` 純粋関数の実装**:
   - RFC §13.5 の遷移規則を完全にコード化
   - `Finalize` / `Abort` からの遷移は全て違法（終端状態の再遷移禁止）
   - `任意状態 -> Abort` を全ての状態から許可
3. **総当たりマトリクステスト**: 8×8 = 64 通りの遷移ペア全てについて `is_legal_transition` の戻り値を検証
4. **SearchState の公開**: `src/lib.rs` からの `pub use types::SearchState;` による公開

## Non-scope

- 終端状態の非再入ガードロジック（`transition_to` メソッド） — チケット M-1.5-2
- 発振検出エンジン — チケット M-1.5-3
- `SearchPolicyOscillation` による `AbortSearch` 強制ダウングレード — チケット M-1.5-3
- `SearchStep` Enum（BuildQueryStep / RetrieveCandidatesStep 等） — 後続チケット
- `SearchOutcome` Enum — 後続チケット
- `FakeExecutor` との統合 — チケット M-1
- マルコフ連鎖ストレステストの本格実装 — 観測テスト範囲内で最小限実施

## Investigation

### RFC §13.5 状態遷移規則（2026-05-22 確認）

**ソース: Darvium-RFC-0001-Unified-v2.3-final.md L1556-L1670**

`SearchState` は RFC L1556-L1566 で以下のように定義されている：

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SearchState {
    Init,
    Retrieve,
    Evaluate,
    Refine,
    Compose,
    ProposeNew,
    Finalize,
    Abort,
}
```

遷移規則は RFC L1657-L1668 の有向グラフとして形式化されている。これを遷移マトリクスに展開すると以下の 8×8 表となる（`✅` = 合法, `❌` = 違法）：

| from \ to    | Init | Retrieve | Evaluate | Refine | Compose | ProposeNew | Finalize | Abort |
|-------------|------|----------|----------|--------|---------|------------|----------|-------|
| Init        | ❌   | ✅       | ❌       | ❌     | ❌      | ❌         | ❌       | ✅    |
| Retrieve    | ❌   | ❌       | ✅       | ❌     | ❌      | ❌         | ❌       | ✅    |
| Evaluate    | ❌   | ❌       | ❌       | ✅     | ✅      | ❌         | ✅       | ✅    |
| Refine      | ❌   | ✅       | ❌       | ❌     | ❌      | ✅         | ❌       | ✅    |
| Compose     | ❌   | ❌       | ❌       | ✅     | ❌      | ❌         | ✅       | ✅    |
| ProposeNew  | ❌   | ❌       | ❌       | ❌     | ❌      | ❌         | ✅       | ✅    |
| Finalize    | ❌   | ❌       | ❌       | ❌     | ❌      | ❌         | ❌       | ❌    |
| Abort       | ❌   | ❌       | ❌       | ❌     | ❌      | ❌         | ❌       | ❌    |

### 現状調査（2026-05-22）

1. **SearchState 未実装**: `src/types.rs` に SearchState の定義は存在しない。`grep SearchState src/` で 0 件。
2. **SearchBudget / RecursionGuard は実装済み**: `src/types.rs` L779-L949 に SearchBudget、SearchBudgetSnapshot、RecursionGuard の実装が完了している（チケット M-2-2）。
3. **エラー型定義済み**: `src/error.rs` L33-L34 に `DarviumError::SearchValidation(String)`、L36-L37 に `DarviumError::TerminalStateViolation` が定義されている。
4. **公開 API パターン**: `src/lib.rs` L28 で `pub use types::{RecursionGuard, SearchBudget, SearchBudgetSnapshot};` と同様の要領で SearchState も公開する想定。
5. **テストパターン**: 既存の `#[cfg(test)] mod tests` 内のテスト（mock_retrieval_primitive_invocation 等）と同様のスタイルで `types.rs` 内に `mod tests` を追加する。

## Test Plan

### テスト構成

テストは `src/types.rs` の `#[cfg(test)] mod tests` 内に追加する。以下のテストを実装する：

#### T1: 全合法遷移の網羅的検証（T1-a〜T1-p）

遷移マトリクスの合法エントリを個別にテストする。

| ID    | 遷移              | 根拠                         |
|-------|-------------------|------------------------------|
| T1-a  | Init -> Retrieve  | Init からの唯一の正規遷移      |
| T1-b  | Init -> Abort     | 任意状態からの Abort の一部    |
| T1-c  | Retrieve -> Evaluate | 検索結果の評価へ              |
| T1-d  | Retrieve -> Abort | 任意状態からの Abort           |
| T1-e  | Evaluate -> Refine | 候補不足によるポリシー改善     |
| T1-f  | Evaluate -> Compose | 組成候補あり                 |
| T1-g  | Evaluate -> Finalize | REUSE/PATCH が十分          |
| T1-h  | Evaluate -> Abort | 任意状態からの Abort           |
| T1-i  | Refine -> Retrieve | requery                     |
| T1-j  | Refine -> ProposeNew | 既存候補再利用の期待値が低い |
| T1-k  | Refine -> Abort   | 任意状態からの Abort           |
| T1-l  | Compose -> Finalize | COMPOSE 成立                |
| T1-m  | Compose -> Refine | compose 不成立               |
| T1-n  | Compose -> Abort  | 任意状態からの Abort           |
| T1-o  | ProposeNew -> Finalize | NEW 採択                  |
| T1-p  | ProposeNew -> Abort | 任意状態からの Abort         |

#### T2: 全違法遷移の網羅的検証

違法遷移のカテゴリごとにテストする。複数ペアをループで一括検証する。

| ID    | 違反カテゴリ              | 含まれる遷移ペア                              | 件数 |
|-------|--------------------------|----------------------------------------------|------|
| T2-a  | 終端状態からの全遷移       | Finalize->*, Abort->*                         | 14   |
| T2-b  | Init からの違法遷移       | Init->Evaluate, Init->Refine, Init->Compose, Init->ProposeNew, Init->Finalize | 5 |
| T2-c  | Retrieve からの違法遷移   | Retrieve->Init, Retrieve->Refine, Retrieve->Compose, Retrieve->ProposeNew, Retrieve->Finalize | 5 |
| T2-d  | Evaluate からの違法遷移   | Evaluate->Init, Evaluate->Retrieve, Evaluate->ProposeNew | 3 |
| T2-e  | Refine からの違法遷移     | Refine->Init, Refine->Evaluate, Refine->Compose, Refine->Finalize | 4 |
| T2-f  | Compose からの違法遷移    | Compose->Init, Compose->Retrieve, Compose->Evaluate, Compose->ProposeNew | 4 |
| T2-g  | ProposeNew からの違法遷移 | ProposeNew->Init, ProposeNew->Retrieve, ProposeNew->Evaluate, ProposeNew->Refine, ProposeNew->Compose | 5 |

#### T3: 全ペア総当たりマトリクス確認

1 つのテスト関数内で 8×8 = 64 ペアすべてをループで検証し、合法 = 16 ペア、違法 = 48 ペアであることをアサートする。`println!` で全ペアの判定結果を構造化出力する。

#### T4: 境界値テスト

| ID   | 内容 |
|------|------|
| T4-a | SearchState のメモリサイズが 1 バイトであることを確認（u8 表現のコンパクト保証） |

### 外部依存

Mock / Stub は不要。`is_legal_transition` は純粋関数であり、外部状態に依存しない。

## 計装方法・観測対象

### 計装方法

1. **総当たり遷移マトリクス観測**: 8×8 = 64 通りの全ペアを `println!` で構造化出力（CSV 形式）し、`--nocapture` 経由で観測する
2. **スペクトル半径観測**: 有効遷移確率行列 $P_{actual}$ のスペクトル半径 $\rho(P_{actual})$ を計測する。全ての合法遷移に等確率を割り当てた行列の固有値から、スペクトル半径が 1 未満であること（全軌道が有限ステップで終端状態へ到達すること）を確認する

### 観測対象

- **OTS-1: 遷移マトリクス完全性**: 64 ペアすべての判定結果を構造化出力として観測し、欠損がないことを確認
- **OTS-2: スペクトル半径計測**: 有効遷移確率行列の最大固有値（スペクトル半径）を計算し $\rho < 1$ であること（全軌道が有限吸収）を確認
- **OTS-3: 平均自由行程**: 全状態から一様ランダムに合法遷移を繰り返した場合の、終端状態（Finalize/Abort）到達までの平均ステップ数の有限性を実証

### 較正計画

本チケットに較正対象の定数は存在しない。状態遷移規則は Safety Invariant であり、RFC 改訂なしでは変更禁止。

## Boy Scout Rule — 翻訳可能性計画

本チケットで新規実装するコードは以下の方針に従う：

1. **関数名は動詞句**: `is_legal_transition` — 「この遷移は合法か？」と散文として読める
2. **変数名はドメイン概念**: `from`, `to`（または `current`, `next`）— 状態遷移のドメインを素直に表現
3. **一関数一責務**: `is_legal_transition` は遷移判定のみ。遷移の実行（`transition_to`）は M-1.5-2 に分離
4. **ハードコードは none**: 状態バリアントの列挙は Rust の Enum として表現し、マジックナンバーを排除
5. **エラー握りつぶし禁止**: `is_legal_transition` は純粋関数として bool を返し、エラーは呼び出し元で処理
6. **翻訳可能性**: 遷移マトリクスは `match (from, to)` の網羅的パターンマッチで表現し、「from が Init で to が Retrieve なら true、...」と日本語に逐語訳できる構造を維持する
7. **既存コード改善**: スコープ内に翻訳可能性を損なう既存コードは存在しない（新規実装のため）

## Acceptance Criteria

- [ ] `SearchState` Enum が RFC §13.5 の 8 バリアントを完全に網羅している
- [ ] `is_legal_transition(current, next) -> bool` が全ての合法遷移で `true` を返す
- [ ] 全 64 ペア（8×8）の総当たりマトリクステストが PASS する
- [ ] 終端状態からの遷移が全て違法（`false`）である
- [ ] `任意状態 -> Abort` が常に合法である
- [ ] `cargo test` が全テストを PASS する
- [ ] 翻訳可能性の検証が通っている（`match (from, to)` が散文として読める）
- [ ] OTS-2: スペクトル半径 $\rho < 1$ が確認されている
- [ ] 既存テストが通過している

## Notes

- plan_path: /plan-ticket が plan.md を作成後に frontmatter に更新する
- implementation_path: /start-ticket が implementation.md を作成後に frontmatter に更新する
- review_report_path: /review-ticket が review.md を作成後に frontmatter に更新する
- observation_report_path: /start-ticket が observation-YYYYMMDD-HHmmss.md を作成後に frontmatter に最新パスを更新する

### 成果物

- 計画: context/0008-searchstate/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0008-searchstate/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0008-searchstate/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0008-searchstate/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
