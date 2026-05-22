---
ticket_id: 10
title: SearchPolicyOscillation（無限往復暴走）検出エンジンの検証
slug: searchpolicyoscillation
status: reviewed
created_at: 2026-05-22
updated_at: 2026-05-22
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0010-searchpolicyoscillation/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0010-searchpolicyoscillation/review.md
---

# SearchPolicyOscillation（無限往復暴走）検出エンジンの検証

## Summary

状態機械が `Refine -> Retrieve -> Refine` の往復を閾値回数以上繰り返す「ポリシー発振（Policy Oscillation）」を検出し、強制的に `Abort` 状態へ遷移させる検出エンジン `OscillationDetector` を新規実装する。あわせて、発振検出時の選択肢（`AbortSearch` / `NeedsHumanReview` へのダウングレード）を `TerminalTransitionReason` に追加する。

- **関連 RFC**: §13.5（状態遷移規則 — 発振検出義務）、§13.6（ガード条件）
- **対応チケット**: M-1.5-3（Darvium-Tickets-v2.3.md L183-L192）

## Background

RFC §13.5 は以下の MUST を規定する:

> `Refine -> Retrieve -> Refine` が閾値回数を超えて往復する場合、実装は `SearchPolicyOscillation` として検出し `AbortSearch` または `NeedsHumanReview` に落とさなければならない (MUST)。

SearchWorkflow が以下の経路で無限ループに陥る可能性がある:

```
Refine → Retrieve → Evaluate → Refine → Retrieve → Evaluate → Refine → ...
```

この往復（oscillation）は、検索ポリシーが「候補不足 → requery → 評価 → やはり不足 → requery」のループから抜け出せない状態であり、`SearchBudget` 上限超過とは独立したメカニズムで検出・遮断する必要がある。予算上限に達するまで待つのではなく、**発振パターンそのものを検出して早期に異常終了**させるのが本チケットの目的である。

### ポリシー発振と Budget 超過の違い

| 観点 | Policy Oscillation | Budget Exceeded |
|------|-------------------|-----------------|
| 検出対象 | 状態遷移パターンの周期性 | リソース消費量の閾値超過 |
| 検出タイミング | 発振パターン検出時に早期遮断 | budget 上限到達時に遮断 |
| 発振カウンタ | 状態遷移履歴ベースの専用カウンタ | 汎用的な消費量カウンタ |
| 復旧戦略 | `AbortSearch` / `NeedsHumanReview` | `Abort` のみ |

## Scope

1. **`OscillationDetector` 構造体の新規実装**（`src/types.rs`）
   - 遷移履歴を追跡し、`Refine ↔ Retrieve` の往復回数をカウントする
   - 発振カウンタが閾値を超えた場合に `is_oscillating() -> bool` を返す
   - 非発振遷移（Refine/Retrieve 以外への遷移）ではカウンタをリセットする
   - カウンタは saturated 加算（上限超過によるパニック防止）

2. **発振検出閾値定数の追加**（`src/constants.rs`）
   - `OSCILLATION_MAX_COUNT: u32` — 最大発振カウント（Calibration Candidate として分類）
   - デフォルト値: 3

3. **`TerminalTransitionReason` への `OscillationDetected` バリアント追加**
   - `can_terminate_with(OscillationDetected) -> true` として終端許可
   - `DarviumError::SearchPolicyOscillation` エラーとの対応付け

4. **`attempt_transition` ヘルパー関数の実装**
   - `transition_to` 呼び出し前に `detector.record_transition()` で履歴記録
   - 発振検出時は `Err(SearchPolicyOscillation)` を返す
   - 正常時は `transition_to` の結果をそのまま伝播

5. **公開 API の更新**: `src/lib.rs` で `OscillationDetector` を `pub use`

## Non-scope

- `SearchOutcome` Enum の実装 — 後続チケット
- `NeedsHumanReview` 状態への実際のルーティング — M1 チケット
- スペクトル半径・吸収時間の観測 — M-1.5-1 で実施済み
- マルチスレッド並行パルス注入（oscillation 版）— 単純な逐次テストで十分
- `SearchTrace` への発振検出ログ記録 — M2.5 チケット
- 既存の `transition_to` / `can_terminate_with` の変更 — 追加のみ

## Investigation

### 現状調査（2026-05-22）

**1. SearchState Enum は実装済み**（`src/types.rs:167-184`）
- 8 バリアント Init / Retrieve / Evaluate / Refine / Compose / ProposeNew / Finalize / Abort
- 発振検出に必要なペアは `(Refine, Retrieve)` と `(Retrieve, Refine)`

**2. `is_legal_transition` 純粋関数**（`src/types.rs:202-222`）
- `Refine -> Retrieve` のみ legal
- `Retrieve -> Refine` は illegal（Retrieve からは Evaluate または Abort のみ）
- したがって、RFC の「`Refine -> Retrieve -> Refine` の往復」は模式的記述であり、実際には `Refine -> Retrieve -> Evaluate -> Refine` の 4 状態サイクル

**3. 発振検出器は未実装**
- `OscillationDetector` → grep 0 件
- OSCILLATION 関連定数 → `constants.rs` に未定義
- `DarviumError::SearchPolicyOscillation` → `src/error.rs:45-46` に定義済みだが未使用

**4. 発振検出アルゴリズムの決定**

RFC の模式的記述「`Refine -> Retrieve -> Refine` が閾値回数を超えて往復」を満たすため、位相ベースの交互遷移カウンタを採用:

- `phase` 状態を `Option<SearchState>` で保持
- `ExpectingRetrieve`（Refine の直後）の状態で `Retrieve` が来ると counter++
- `ExpectingRefine`（Retrieve の直後）の状態で `Refine` が来ると counter++
- それ以外 → リセット（`counter = 0`, `phase = None`）
- `counter >= OCCILLATION_MAX_COUNT` で `is_oscillating() = true`

```text
初期状態: phase = None, counter = 0

Init -> Retrieve:       期待と不一致 → リセット, phase = ExpectingRefine
Retrieve -> Evaluate:   期待と不一致 → リセット, phase = None
...
Refine -> Retrieve:     期待(ExpectingRetrieve)と一致 → counter=1, phase = ExpectingRefine
Retrieve -> Refine:     期待(ExpectingRefine)と一致 → counter=2, phase = ExpectingRetrieve
Refine -> Retrieve:     期待(ExpectingRetrieve)と一致 → counter=3 → is_oscillating() = true
```

**5. エラー型は準備済み**（`src/error.rs:45-46`）
```rust
#[error("Search policy oscillation detected")]
SearchPolicyOscillation,
```

**6. 公開 API の現状**（`src/lib.rs:28-30`）
- `OscillationDetector` の追加が必要
- `TerminalTransitionReason` は既に公開

## Test Plan

テストは `src/types.rs` の既存 `#[cfg(test)] mod tests` 内に追加する。

### T1: 発振検出の基本動作

| ID | 遷移系列 | 発振カウント | is_oscillating |
|----|---------|------------|----------------|
| T1-a | `Refine→Retrieve→Refine→Retrieve` | 3 | `true` |
| T1-b | `Refine→Retrieve` | 1 | `false` |
| T1-c | `Refine→Retrieve→Refine` | 2 | `false`（max=3） |

### T2: 非発振パターンでのリセット

| ID | 遷移系列 | 発振カウント |
|----|---------|------------|
| T2-a | `Refine→Retrieve→Evaluate→Refine` | 0（Evaluate でリセット） |
| T2-b | `Refine→Retrieve→Init` | 0（Init でリセット） |
| T2-c | `Init→Retrieve→Evaluate` | 0（発振未検出） |

### T3: `attempt_transition` 統合テスト

| ID | 遷移系列 | 期待 |
|----|---------|------|
| T3-a | 非発振系列 `Init→Retrieve→Evaluate→Finalize` | Ok(()) |
| T3-b | 発振系列（閾値3）`Refine→Retrieve→Refine→Retrieve→Refine→Retrieve` | Err(SearchPolicyOscillation) |
| T3-c | 発振検出後の状態が Abort になっている | confirmed |

### T4: `can_terminate_with(OscillationDetected)`

T4-a: `OscillationDetected` で `true` を返す

### T5: 発振カウンタの飽和安全性

`u32::MAX` 近傍からの連続発振遷移でオーバーフローせず飽和する。

### 外部依存: Mock/Stub 不要

## 計装方法・観測対象

### 計装方法

1. **発振検出マトリクス観測**: 全発振/非発振系列におけるカウント推移を CSV 構造化出力
2. **`record_transition` レイテンシ**: 各呼び出しの処理時間を計測

### 観測対象

- **OTS-1**: 発振系列/非発振系列の全パターンにおける発振カウント推移を観測
- **OTS-2**: `attempt_transition` のエラー発生レイテンシと正常遷移レイテンシの比較

### 較正計画

- **調整する定数**: `OSCILLATION_MAX_COUNT: u32`（`constants.rs`）
- **分類**: Calibration Candidate
- **デフォルト**: 3
- **目的関数 J(θ)**: 発振検出潜時と false positive 率のトレードオフ

## Boy Scout Rule — 翻訳可能性計画

1. **関数名は動詞句**: `record_transition`（遷移を記録する）、`is_oscillating`（発振しているか）、`attempt_transition`（遷移を試行する）
2. **変数名はドメイン概念**: `oscillation_count`（発振カウント）、`phase`（位相）
3. **一関数一責務**: `OscillationDetector` は発振検出のみ。`transition_to` は変更しない
4. **ハードコード値の定数化**: 閾値は `OSCILLATION_MAX_COUNT` として `constants.rs` に定義
5. **エラー握りつぶし禁止**: `attempt_transition` は `Result` 伝播

## Acceptance Criteria

- [ ] `OscillationDetector` 構造体が実装されている
- [ ] `record_transition` が発振遷移を正しくカウントする
- [ ] `is_oscillating()` が閾値超過時に `true` を返す
- [ ] 非発振遷移でカウンタがリセットされる
- [ ] `attempt_transition` が発振検出時に `Err(SearchPolicyOscillation)` を返す
- [ ] `OscillationDetected` が `can_terminate_with` で `true` を返す
- [ ] `cargo test` が全テストを PASS する
- [ ] 既存テスト（M-1.5-1, M-1.5-2）が通過している

## Notes

### 成果物

- 計画: context/0010-searchpolicyoscillation/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0010-searchpolicyoscillation/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0010-searchpolicyoscillation/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0010-searchpolicyoscillation/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
