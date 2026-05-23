---
ticket_id: 11
title: M-1-1: 静的閾値による `EvaluateCandidatesStep` 決定エンジンの実装
slug: m-1-1-evaluatecandidatesstep
status: reviewed
created_at: 2026-05-22
updated_at: 2026-05-22
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0011-m-1-1-evaluatecandidatesstep/plan.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0011-m-1-1-evaluatecandidatesstep/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0011-m-1-1-evaluatecandidatesstep/review.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0011-m-1-1-evaluatecandidatesstep/observation-20260522-203657.md
---
# M-1-1: 静的閾値による `EvaluateCandidatesStep` 決定エンジンの実装

## Summary

検索候補（`CandidateSet`）に含まれる各候補のスコアを静的閾値で評価し、`SearchOutcome` を決定する純粋判定関数 `evaluate_candidates` を実装する。本チケットは M-1 shadow-first（Fake policy evaluator）の第1弾であり、後段で自己評価割引・validator weight switch・patch confidence 計算へ滑らかに接続される簡約モデルとして位置づける。

## Background

Darvium SearchWorkflow は `Evaluate` 状態において、取得された候補群を評価し、以下の outcome からひとつを選択する必要がある：
- `ReuseExisting`: 既存ワークフローをそのまま再利用（スコアが閾値以上）
- `PatchExisting`: 既存ワークフローにパッチを適用（スコアが閾値未満）
- `ComposeExisting`: 複数既存ワークフローの組成
- `GenerateNew`: 新規ワークフロー生成
- `AbortSearch`: 探索中断

RFC §13.4 は SearchOutcome の最終選択を `EvaluateCandidatesStep` / `RefineSearchPolicyStep` による bounded heuristic policy として扱うと規定している。M-1-1 ではこのうち REUSE / PATCH の二値分岐を静的閾値で実装し、後続チケット (M-1-2, M-1-3, M-2-3 等) の評価基盤を提供する。

## Scope

- `SearchOutcome` enum の実装（RFC §13.3 の型定義をコード化）
- 静的閾値 0.50 による `evaluate_candidates` 純粋関数の実装
  - スコア >= 0.50 → `SearchOutcome::ReuseExisting`
  - スコア < 0.50 → `SearchOutcome::PatchExisting`
- `EVALUATION_THRESHOLD` 定数の追加（`constants.rs`）
- 自己評価割引適用のための `apply_self_conf_discount` 補助関数
- 網羅的なユニットテスト（正常系・境界値・決定論性）
- 観測テスト（ノイズ注入による決定境界近傍の選択確率分布計測）

## Non-scope

- `RefineSearchPolicyStep` の実装（M-1 後続チケット）
- `ComposeExisting` / `GenerateNew` / `AbortSearch` の分岐ロジック
- `SearchBudget` / `RecursionGuard` のガード条件（M-1-2, M-1-3）
- 実際の `RetrievalPrimitive` との結合
- 人間レビューキューイング（M1）
- Knowledge-Aware Candidate Evaluation（§16.4）

## Investigation

### RFC 交叉参照

**RFC §13.3 (SearchWorkflow データモデル)**:
- `SearchOutcome` enum が RFC 上で定義されているが、コード上には未実装である
- 必要なバリアント: `ReuseExisting`, `PatchExisting`, `ComposeExisting`, `GenerateNew`, `AbortSearch`, `NeedsHumanReview`
- M-1-1 では `ReuseExisting` と `PatchExisting` の 2 バリアントのみ使用
- 型 `WorkflowGraphId`, `GraphPatch`, `CompositionPlan` も RFC 定義のみでコード未実装
- `WorkflowGraphId` は `String` 型エイリアスとして簡易実装可能

**RFC §13.4-13.5 (RetrievalPrimitive 契約, 状態遷移規則)**:
- `EvaluateCandidatesStep` は `Evaluate` 状態で動作し、`Finalize`, `Compose`, `Refine`, `Abort` の 4 方向遷移を決定する入り口
- RFC では「A ≥ threshold かつ単独候補で十分 → REUSE / PATCH」と規定

**RFC §12.2-12.3 (自己評価割引, validator 重み切り替え)**:
- 自己評価スコア `c_s` に `SELF_CONF_DISCOUNT (0.85)` を乗算
- `c_s * 0.85 < 0.50` の場合に validator 側重み `w_v` を 0.40 → 0.50 へ引き上げる
- M-1-1 では割引適用関数の実装まで（重み切り替えは M-2-3 以降で接続）

### 既存コード調査

**`src/constants.rs`**:
- `SELF_CONF_DISCOUNT: f64 = 0.85` (27行目) — 既存、LLM 自己信頼ディスカウント率
- `EVALUATION_THRESHOLD` は未定義 → 追加が必要
- `APPLICABILITY_THRESHOLD` も未定義（RFC §12 では 0.50 と規定）

**`src/types.rs`**:
- `SearchState` enum (8 状態) 実装済み
- `SearchBudget`, `SearchBudgetSnapshot`, `RecursionGuard` 実装済み
- `OscillationDetector` 実装済み (M-1.5-3)
- `RetrievalPrimitive` trait 実装済み
- `SearchOutcome` enum — **未実装**（追加が必要）
- `RankedCandidate.blended_score: f64` (139行目) — 評価関数の主な入力となるスコアフィールド
- `TerminalTransitionReason` enum 実装済み

**`src/error.rs`**:
- `DarviumError` enum に以下が既存:
  - `SearchValidation(String)` — 遷移違反
  - `TerminalStateViolation` — 終端状態違反
  - `SearchBudgetExceeded` — 予算超過
  - `SearchRecursionExceeded` — 再帰超過
  - `SearchPolicyOscillation` — 発振検出
- 不足: `DarviumError::InvalidScore` — スコア値が [0.0, 1.0] 範囲外の場合のエラー（追加推奨）

**`src/lib.rs`**:
- `pub mod types` で `OscillationDetector`, `RecursionGuard`, `SearchBudget`, `SearchBudgetSnapshot`, `SearchState`, `TerminalTransitionReason` を re-export 済み
- `SearchOutcome` は未 re-export → 追加が必要
- `evaluate_candidates` 関数は未実装 → `types.rs` または新規モジュールへの追加が必要

**`src/mock.rs`**:
- `MockEmptyRetrievalPrimitive` / `MockErrorRetrievalPrimitive` 実装済み
- M-1-1 では新しい Mock は不要（純粋関数なので）

**テストファイル**:
- `tests/m_minus1/` ディレクトリは未作成
- ユニットテストは M-1-1 で作成する `evaluate_candidates` にインライン追加する方針
- 観測テストは別ファイルまたは同一ファイル内の `#[cfg(test)]` に追加

## Test Plan

### 実装対象関数

1. **`evaluate_candidates(best_score: f64) -> SearchOutcome`** — 純粋関数
   - 入力: 最良候補の blended_score
   - 出力: `SearchOutcome` (ReuseExisting / PatchExisting)
   - 閾値: `EVALUATION_THRESHOLD = 0.50`
   - 不変条件: `best_score` は `[0.0, 1.0]` の範囲を仮定（範囲外は `InvalidScore` エラー）

2. **`apply_self_conf_discount(raw_score: f64) -> f64`** — 純粋関数（後段接続の足場）
   - 入力: LLM 自己評価スコア `c_s`
   - 出力: `c_s * SELF_CONF_DISCOUNT`
   - 不変条件: 返値は `[0.0, 1.0]` でクランプされる

### ユニットテスト一覧

#### T1: 正常系 — 閾値判定
| ID | スコア | 期待結果 |
|----|--------|---------|
| T1-a | 0.51 | `ReuseExisting` |
| T1-b | 0.49 | `PatchExisting` |
| T1-c | 1.00 | `ReuseExisting`（上限） |
| T1-d | 0.00 | `PatchExisting`（下限） |
| T1-e | 0.50 | `ReuseExisting`（境界値・閾値以上） |

#### T2: 異常系 — スコア範囲外
| ID | スコア | 期待結果 |
|----|--------|---------|
| T2-a | -0.01 | `Err(InvalidScore)` |
| T2-b | 1.01 | `Err(InvalidScore)` |
| T2-c | f64::NEG_INFINITY | `Err(InvalidScore)` |
| T2-d | f64::INFINITY | `Err(InvalidScore)` |
| T2-e | f64::NAN | `Err(InvalidScore)` |

#### T3: 決定論性
| ID | 内容 |
|----|------|
| T3-a | 同一スコアで 2 回呼び出した結果が完全一致 |
| T3-b | 異なるスコアでは結果が異なる（0.51 vs 0.49） |

#### T4: 自己評価割引（auxiliary）
| ID | 生スコア | 割引後 | 判定結果 | 備考 |
|----|---------|--------|---------|------|
| T4-a | 0.90 | 0.765 | `ReuseExisting` | 割引後も閾値超過 |
| T4-b | 0.45 | 0.3825 | `PatchExisting` | 割引後も閾値未満 |
| T4-c | 0.60 | 0.51 | `ReuseExisting` | 割引後ぎりぎり閾値超過 |
| T4-d | 0.58 | 0.493 | `PatchExisting` | 割引後ぎりぎり閾値未満 |
| T4-e | SELF_CONF_DISCOUNT が 1.0 の場合、割引が恒等写像になる |

#### T5: SearchOutcome enum の網羅性
| ID | 内容 |
|----|------|
| T5-a | 全 6 バリアントが Debug, Clone, PartialEq を実装 |
| T5-b | 全バリアントが網羅的マッチング可能 |

### 観測テスト

#### OTS-1: ノイズ注入による決定境界近傍の選択確率分布
- 固定シード PRNG (`StdRng::seed_from_u64(12345)`) を使用
- スコア 0.50 ± ε の周辺に平均 0、分散 σ²_noise のガウスノイズを付加
- ノイズ分散を sweep しながら REUSE / PATCH 選択確率のシグモイド曲線を観測
- サンプルサイズ: 各条件 n >= 10,000
- 観測量: 選択確率、曲線傾斜（感受率）、シグモイド中心値

#### OTS-2: 決定境界の幾何学的曲率
- ノイズ分散 σ² の関数としての境界超曲面の平均曲率を実測
- スケーリング則 β = 1/σ² の検証

## 計装方法・観測対象

### 計装方法
- 全テストは `src/types.rs` 内の `#[cfg(test)] mod tests` に追加（既存の M-1.5 テスト群と同じパターン）
- `StdRng::seed_from_u64(12345)` (constants::TEST_PRNG_SEED) を使用
- 観測出力は `println!` で JSON/CSV 形式の構造化テキストを `--nocapture` 経由で標準出力
- 観測テストは `tests/m_minus1/` ディレクトリに分離してもよい

### 観測対象
| 観測量 | テスト | サンプルサイズ | 統計量 |
|--------|--------|--------------|--------|
| 決定境界における選択確率分布 | OTS-1 | 10,000/条件 | 平均確率、シグモイド傾斜 |
| 幾何学的曲率とスケーリング則 | OTS-2 | 10,000/条件 | 曲率、β 比例係数 |
| 境界値近傍の安全率 | T1 | 手動 | PASS/FAIL |

### 較正計画
- 調整する定数: `EVALUATION_THRESHOLD (0.50)` — 将来の Calibration Candidate
- SELF_CONF_DISCOUNT (0.85) — 既存、変更範囲外
- 目的関数 J(θ): 誤検出率（False Reuse / False Patch）の合成

## Boy Scout Rule — 翻訳可能性計画

### 新規コードの方針

1. **関数名は動詞句**: `evaluate_candidates`, `apply_self_conf_discount` — 関数呼び出しが「候補を評価する」「自己評価割引を適用する」と逐語訳できる
2. **定数は名前付き**: 閾値 0.50 をハードコードせず `EVALUATION_THRESHOLD` として `constants.rs` に定義
3. **一関数一責務**: `evaluate_candidates` は閾値判定のみ、`apply_self_conf_discount` は割引計算のみ
4. **エラー握りつぶし禁止**: 範囲外スコアは `Result::Err` で明示的に伝播、パニック・サイレントデフォルトは禁止
5. **範囲外入力は型で防げないならエラーで弾く**: `f64` は [-∞, +∞] 全域を許容するため、[0.0, 1.0] の範囲検証を関数内で実施

### 既存コードの改善

- `types.rs` 内の `SearchOutcome` 未実装部分を補完
- `lib.rs` の re-export に `SearchOutcome` を追加
- RFC 上の型定義とコード上の型定義の不一致を解消

## Acceptance Criteria

- [ ] `SearchOutcome` enum が RFC §13.3 の定義を満たして実装されている
- [ ] `evaluate_candidates(0.51)` が `Ok(SearchOutcome::ReuseExisting)` を返す
- [ ] `evaluate_candidates(0.49)` が `Ok(SearchOutcome::PatchExisting)` を返す
- [ ] `evaluate_candidates(0.50)` が `Ok(SearchOutcome::ReuseExisting)` を返す（境界値）
- [ ] 範囲外スコア（-0.01, 1.01, NaN, ±Inf）が `Err(InvalidScore)` を返す
- [ ] `apply_self_conf_discount(0.90)` が 0.765 を返す
- [ ] 全ユニットテスト（T1〜T5）が通過
- [ ] 観測テスト（OTS-1, OTS-2）が構造化出力を生成し、不変条件を満たす
- [ ] `cargo test` が全て通過（既存テスト含む）
- [ ] `cargo clippy -- -D warnings` が通過
- [ ] `cargo fmt` が通過
- [ ] 翻訳可能性の検証: 関数名が動詞句、定数が名前付き、エラーが伝播されている

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

- 計画: context/0011-m-1-1-evaluatecandidatesstep/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0011-m-1-1-evaluatecandidatesstep/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0011-m-1-1-evaluatecandidatesstep/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0011-m-1-1-evaluatecandidatesstep/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
