---
ticket_id: 51
title: "M0-3: PRNG駆動型擬似提案スコア（Confidence）による結果多様性シミュレーション"
slug: m0-3-prngconfidence
status: reviewed
created_at: 2026-05-23
updated_at: 2026-05-23
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0051-m0-3-prngconfidence/observation-20260523-143303.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0051-m0-3-prngconfidence/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0051-m0-3-prngconfidence/review.md
---

# M0-3: PRNG駆動型擬似提案スコア（Confidence）による結果多様性シミュレーション

## Summary

擬似乱数（PRNG）を用いて `CompositionPlan` の confidence 値を `[0.30, 0.95]` の範囲でバラつかせる Mock 提案器を実装する。この Mock 提案器は、低 confidence 時に探索状態機械が `Refine` へ遷移し、高 confidence 時に `Finalize` へ遷移する決定論的分岐を、確率的入力下で統計的に観測可能にするテスト基盤である。

さらに、判定ロジックに入力されるプランの内部信頼度ベクトル **C = (c_s, c_v, c_h)** に対し微小摂動 δ**C**(0) = 10⁻⁶ を加えたツイン軌道を実行し、リアプノフ指数 λ ≤ 0（非カオス・局所収束安定性）を検証する計装を含む。

## Background

### 対象不変条件 / 規範

- **RFC §13.3**: `CompositionPlan` は `confidence: f32` フィールドを持つ — 組成計画の信頼度スコア。
- **RFC §13.5**: 状態機械の遷移規則 — `Evaluate → Finalize`（REUSE/PATCH が十分）、`Evaluate → Refine`（候補不足・policy 改善が必要）、`Compose → Finalize`（COMPOSE 成立）、`Compose → Refine`（compose 不成立）。
- **RFC §16.1 Empirical Claim**: システム全体の収束安定性 — 微小摂動に対する軌道発散が非カオスであること（リアプノフ指数 λ ≤ 0）。
- **チケット M0-1 実装**: `CompositionPlan` が `confidence: f32` フィールドを持つ完全構造体として `src/types.rs:4372-4401` に定義済み。
- **チケット M0-2 実装**: `guard.rs` に副作用プロファイルに基づく GenerateNew 安全ガードが実装済み。本チケットは confidence 値に基づく状態遷移の分岐判定を追加する。

### 内部信頼度ベクトル C = (c_s, c_v, c_h)

判定ロジックはプランの単一 `confidence` スカラー値ではなく、内部で以下の3次元信頼度ベクトルを保持する：

| 成分 | 名称 | 意味 |
|------|------|------|
| c_s | Semantic Validity | 意味論的な提案の妥当性（LLM 自己評価相当） |
| c_v | Variable Consistency | 変数スコープ整合性（V-03/V-04 通過の確信度） |
| c_h | Heuristic Alignment | ヒューリスティック評価との整合性 |

全体 confidence は `C_agg = w_s·c_s + w_v·c_v + w_h·c_h` の重み付き線形結合として算出し、これが閾値判定に用いられる。重みは `constants.rs` の Calibration Candidate として定義する。

### 現状のコード

- `CompositionPlan { confidence: f32 }` は `src/types.rs:4372-4401` で定義済み。
- `SearchState` の `Refine` と `Finalize` は `src/types.rs:231-252` で定義済み、合法遷移行列は同ファイル 255-290 行で定義。
- PRNG パターン（`StdRng::seed_from_u64(12345)`）は `src/search/simulated_ranker.rs` で確立済み。
- `TEST_PRNG_SEED = 12345` は `src/constants.rs:67` で定義済み。
- 信用割引関数 `apply_self_conf_discount` は `src/types.rs:4349` で定義済み。

### 過去の観察レポートからの知見

- `tickets/context/0050-m0-2-generatenew/observation-20260523-134545.md` — M0-2 安全ガード実装。ガード関数のパターン（check → route）の参考になる。
- `tickets/context/0049-m0-1-compositionplan/observation-20260523-132423.md` — CompositionPlan 型拡張と静的バリデータ。Erdős–Rényi ランダムグラフ生成 + PRNG パターンの実装参考。
- `tickets/context/0045-m-05-2/observation-20260523-103051.md` — ランクドリフトシミュレーションにおけるガウスノイズ注入と統計的観測のパターン。`simulated_ranker.rs` の PRNG 駆動テスト手法が直接の参考になる。

## Scope

### 実装スコープ

1. **`ConfidenceVector` 構造体の定義**（`src/types.rs`）
   - フィールド: `c_s: f64`（semantic validity）, `c_v: f64`（variable consistency）, `c_h: f64`（heuristic alignment）
   - 全成分 `[0.0, 1.0]` の範囲を不変条件とする
   - `#[derive(Debug, Clone, Copy, PartialEq)]`
   - メソッド: `aggregate(&self) -> f64` — 重み付き線形結合で統合 confidence を算出
   - `fn perturb(&self, delta: f64, rng: &mut impl rand::Rng) -> Self` — ツイン軌道用摂動

2. **定数追加**（`src/constants.rs`）
   - `CONFIDENCE_C_S_WEIGHT: f64` — c_s の重み (Calibration Candidate, Default: 0.40)
   - `CONFIDENCE_C_V_WEIGHT: f64` — c_v の重み (Calibration Candidate, Default: 0.35)
   - `CONFIDENCE_C_H_WEIGHT: f64` — c_h の重み (Calibration Candidate, Default: 0.25)
   - `CONFIDENCE_REFINE_THRESHOLD: f64` — Refine へ分岐する confidence 上限 (Calibration Candidate, Default: 0.50)
   - `CONFIDENCE_FINALIZE_THRESHOLD: f64` — Finalize へ分岐する confidence 下限 (Calibration Candidate, Default: 0.70)
   - `MOCK_PROPOSER_CONFIDENCE_MIN: f64` — Mock 提案器の confidence 最小値 (Calibration Candidate, Default: 0.30)
   - `MOCK_PROPOSER_CONFIDENCE_MAX: f64` — Mock 提案器の confidence 最大値 (Calibration Candidate, Default: 0.95)
   - `LYAPUNOV_DELTA_C0: f64` — ツイン軌道初期摂動 (Safety Invariant, Default: 1e-6)
   - `CONFIDENCE_VECTOR_DIM: usize` — 信頼度ベクトル次元 (Safety Invariant, Default: 3)

3. **Mock 提案器の実装**（`src/search/mock_proposer.rs` 新規ファイル）
   - `pub struct MockProposer { rng: StdRng }` — 固定シード PRNG を保持
   - `pub fn new() -> Self` — `StdRng::seed_from_u64(TEST_PRNG_SEED)` で初期化
   - `pub fn generate_confidence(&mut self) -> ConfidenceVector` — 各成分を `[MOCK_PROPOSER_CONFIDENCE_MIN, MOCK_PROPOSER_CONFIDENCE_MAX]` の一様乱数で生成
   - `pub fn set_seed(&mut self, seed: u64)` — 再現性テスト用にシード変更可能
   - 決定論的再現性保証: 同一シード → 同一系列（全テストで検証）

4. **Confidence 判定関数の実装**（`src/search/mock_proposer.rs`）
   - `pub fn decide_composition_fate(confidence_vector: &ConfidenceVector) -> CompositionDecision`
   - `CompositionDecision` enum:
     - `Refine { reason: String }` — 低 confidence: `C_agg < CONFIDENCE_REFINE_THRESHOLD`
     - `Finalize { reason: String }` — 高 confidence: `C_agg >= CONFIDENCE_FINALIZE_THRESHOLD`
     - `Uncertain { reason: String }` — 中間領域: 閾値の間に位置（デフォルトは Refine 相当として扱うが観測用に区別）
   - uncertainty 領域（`C_agg ∈ [REFINE_THRESHOLD, FINALIZE_THRESHOLD)`）の挙動は観測テストで分布を記録

5. **リアプノフ指数計装**（`src/search/mock_proposer.rs` 観測テスト内）
   - ツイン軌道: 基準軌道（摂動なし）と摂動軌道（|δC(0)| = 10⁻⁶）を並行実行
   - 各イテレーション t で `|δC(t)|` を計測
   - リアプノフ指数: `λ(t) = (1/t) · ln(|δC(t)| / |δC(0)|)`
   - 観測: `λ(t)` の時間発展をプロット、`lim_{t→∞} λ(t) ≤ 0` を検証

6. **`src/lib.rs` への登録**
   - 公開 API re-export: `MockProposer`, `ConfidenceVector`, `CompositionDecision`, `decide_composition_fate`

### Non-scope

- ComposeExisting の実際のパッチ生成ロジックは本チケットの対象外。
- 信頼伝播（TrustProfile への統合）は M-0.5-2 以降の対象。
- 実 LLM による自己評価スコア c_s の生成は M2 以降。
- `SearchTrace` への confidence 判定記録は M2.5-1 の対象。
- Darvium Facade への統合（`Darvium::decide` 等）は本チケットの対象外。

## Investigation

### コードベース調査結果（2026-05-23）

| 発見事項 | ファイル | 行 |
|---------|----------|-----|
| `CompositionPlan.confidence` 既存（f32） | `src/types.rs` | 4382 |
| PRNG パターン確立済み（StdRng + seed_from_u64） | `src/search/simulated_ranker.rs` | 19-26 |
| `TEST_PRNG_SEED = 12345` 定義済み | `src/constants.rs` | 67 |
| 状態機械 Refine/Finalize 定義済み | `src/types.rs` | 231-252 |
| 合法遷移行列定義済み | `src/types.rs` | 255-290 |
| 閾値判定パターン（evaluate_candidates） | `src/types.rs` | 4329-4343 |
| `apply_self_conf_discount` 定義済み | `src/types.rs` | 4349 |
| search モジュールファイル一覧 | `src/search/` | simulated_ranker.rs, applicability.rs, mod.rs |
| MockProposer 未実装 | — | — |

### アーキテクチャ上の決定

- Mock 提案器は `src/search/mock_proposer.rs` に新規ファイルとして実装する（シミュレーション基盤と同じ `search` モジュール配下に配置）。
- confidence 判定関数は `src/search/mock_proposer.rs` 内に実装する（Mock 提案器と判定ロジックは密結合）。
- `ConfidenceVector` 型は `src/types.rs` に定義する（crate 全体で参照可能にするため）。
- リアプノフ指数の計装は観測テスト内で `println!` + `--nocapture` 経由で行う。
- 中程度の uncertainty 領域（閾値間）はデフォルトで Refine にルーティングするが、観測テストでは比率を記録する。

## Test Plan

### ユニットテスト計画（`src/search/mock_proposer.rs` 内 `mod tests`）

#### T1: `ConfidenceVector` 構造体の正常構築

- **条件**: 各成分 `c_s=0.8, c_v=0.7, c_h=0.6` で構築
- **期待**: 各フィールドに値が正しく設定される
- **検証**: `assert_eq!(cv.c_s, 0.8)` 等

#### T2: `ConfidenceVector` 成分範囲違反

- **条件**: `c_s = 1.5`（範囲超過）
- **期待**: コンストラクタで clamp され `[0.0, 1.0]` に収まる
- **検証**: `assert!((cv.c_s - 1.0).abs() < 1e-12)`

#### T3: `aggregate` 正常計算

- **条件**: `c_s=0.8, c_v=0.7, c_h=0.6`、デフォルト重み `w_s=0.40, w_v=0.35, w_h=0.25`
- **期待**: `C_agg = 0.40×0.8 + 0.35×0.7 + 0.25×0.6 = 0.32 + 0.245 + 0.15 = 0.715`
- **検証**: `assert!((cv.aggregate() - 0.715).abs() < 1e-12)`

#### T4: `MockProposer` の生成値範囲

- **条件**: `MockProposer::new()` で 1000 回 generate を呼び出し
- **期待**: 全成分が `[MOCK_PROPOSER_CONFIDENCE_MIN, MOCK_PROPOSER_CONFIDENCE_MAX]` の範囲内
- **検証**: 全サンプルの最小値・最大値が範囲内であることを確認

#### T5: `MockProposer` の決定論的再現性

- **条件**: 同一シードで2つの `MockProposer` を生成、同じ回数 generate
- **期待**: 全生成値が完全一致
- **検証**: `assert_eq!(vec1, vec2)` で系列全体を比較

#### T6: 異種シードで異なる系列

- **条件**: シード 12345 と 54321 で生成した最初の 10 ベクトルを比較
- **期待**: 少なくとも1つ以上のベクトルが異なる
- **検証**: `assert_ne!(vec1, vec2)`

#### T7: `decide_composition_fate` — 高 confidence で Finalize

- **条件**: `ConfidenceVector { c_s: 0.9, c_v: 0.85, c_h: 0.8 }`（aggregate ≈ 0.855）
- **期待**: `CompositionDecision::Finalize`
- **検証**: `assert!(matches!(decision, CompositionDecision::Finalize { .. }))`

#### T8: `decide_composition_fate` — 低 confidence で Refine

- **条件**: `ConfidenceVector { c_s: 0.3, c_v: 0.25, c_h: 0.2 }`（aggregate ≈ 0.2575）
- **期待**: `CompositionDecision::Refine`
- **検証**: `assert!(matches!(decision, CompositionDecision::Refine { .. }))`

#### T9: `decide_composition_fate` — 中間 uncertainty

- **条件**: `ConfidenceVector { c_s: 0.6, c_v: 0.5, c_h: 0.5 }`（aggregate ≈ 0.54）
- **期待**: `CompositionDecision::Uncertain`
- **検証**: `assert!(matches!(decision, CompositionDecision::Uncertain { .. }))`

#### T10: `perturb` による摂動

- **条件**: 基準ベクトルを `perturb(1e-6, &mut rng)` で摂動
- **期待**: 摂動前後の aggregate 差が 1e-6 オーダー
- **検証**: `let d = (cv.aggregate() - perturbed.aggregate()).abs(); assert!(d < 1e-5 && d > 1e-8)`

#### T11: `set_seed` による系列再設定

- **条件**: 生成 → 5回 generate → `set_seed(12345)` → 最初から再生成
- **期待**: 再設定後の最初の値が新規インスタンスの最初の値と一致
- **検証**: `assert_eq!(regen, fresh)`

### 観測テスト（OTS）

#### OTS-1: 500 回擬似提案ループ — confidence 分布と状態分岐の観測

- **計装**: `MockProposer` で 500 回 `generate_confidence()` → `decide_composition_fate()` を実行
- **観測**:
  - confidence 値（aggregate）のヒストグラム（期待: `[0.30, 0.95]` の一様分布に近い）
  - Refine / Finalize / Uncertain の各分岐比率
- **出力**: `println!("OTS-1: refine={} finalize={} uncertain={} total={}")`
- **試行数**: 500（チケット仕様準拠）

#### OTS-2: ツイン軌道リアプノフ指数 λ の観測

- **計装**: 基準軌道 `C` と摂動軌道 `C' = C.perturb(1e-6, rng)` を 500 イテレーション並行実行。各 t で `δC(t) = C'(t) - C(t)` の L2 ノルムを計測し `λ(t) = (1/t)·ln(|δC(t)|/|δC(0)|)` を計算。
- **観測**:
  - 最終 λ < 0.01（非カオス判定の実用的閾値）
  - 発散（|δC(t)| > 0.1）が発生しないこと
- **出力**: `println!("OTS-2: final_lyapunov={:.6e} max_divergence={:.6e}")`

#### OTS-3: 不確実性領域（Uncertainty Zone）分布

- **計装**: 0.01 刻みで sweep（66 点）、各点で 100 回試行 = 6600 サンプル
- **観測**: 閾値境界付近の分岐確率遷移
- **出力**: CSV（`c_agg, refine_prob, finalize_prob, uncertain_prob`）

## 計装方法・観測対象

### 計装方法

- `src/search/mock_proposer.rs` の `mod tests` 内に全テストを実装
- 計装プローブ: `println!` + `--nocapture` で構造化出力（CSV 形式）
- 固定シード: `StdRng::seed_from_u64(TEST_PRNG_SEED)`（OTS-1/OTS-3）
- ツイン軌道: 基準と摂動で同一のイテレーションカウンタを使用、PRNG 消費位置を同期

### 観測対象

| ID | 統計量 | 説明 | 期待値 | 種別 |
|----|--------|------|--------|------|
| OTS-1 | 分岐比率 | Refine/Finalize/Uncertain の割合 | 分布が入力範囲に応じ妥当 | 観測 |
| OTS-1 | confidence 分布 | 生成値の一様性 | `[0.30, 0.95]` 内に分布 | 観測 |
| OTS-2 | 最終リアプノフ指数 λ(500) | 摂動軌道発散率 | λ < 0.01 | 検証 |
| OTS-2 | 最大発散 max\|δC(t)\| | ツイン軌道の最大乖離 | max < 0.1 | 検証 |
| OTS-3 | 境界通過確率 | 閾値近傍の分岐確率遷移 | 明瞭なステップ関数 | 観測 |

### 較正計画

本チケットで導入する Calibration Candidate 定数：

| 定数 | デフォルト | 感度分析範囲 | 目的 |
|------|-----------|-------------|------|
| `CONFIDENCE_C_S_WEIGHT` | 0.40 | 0.20–0.60 | c_s の寄与率調整 |
| `CONFIDENCE_C_V_WEIGHT` | 0.35 | 0.15–0.55 | c_v の寄与率調整 |
| `CONFIDENCE_C_H_WEIGHT` | 0.25 | 0.05–0.45 | c_h の寄与率調整 |
| `CONFIDENCE_REFINE_THRESHOLD` | 0.50 | 0.30–0.70 | Refine 分岐閾値 |
| `CONFIDENCE_FINALIZE_THRESHOLD` | 0.70 | 0.50–0.90 | Finalize 分岐閾値 |
| `MOCK_PROPOSER_CONFIDENCE_MIN` | 0.30 | 0.10–0.50 | Mock 生成範囲下限 |
| `MOCK_PROPOSER_CONFIDENCE_MAX` | 0.95 | 0.70–1.00 | Mock 生成範囲上限 |

初期較正ループでは、閾値間の uncertainty 領域幅（`FINALIZE_THRESHOLD - REFINE_THRESHOLD = 0.20`）を保持しつつ、分岐比率が以下の範囲になることを観測する：
- Refine ≈ 30-50%（低 confidence 領域）
- Finalize ≈ 30-50%（高 confidence 領域）
- Uncertain ≈ 10-20%（閾値間領域、Mock の範囲が閾値を跨ぐため）

## Boy Scout Rule — 翻訳可能性計画

### 新規コード

- **関数名は動詞句**: `generate_confidence`（「信頼度を生成する」）、`decide_composition_fate`（「組成の運命を判定する」）、`compute_lyapunov_exponent`（「リアプノフ指数を計算する」）、`perturb`（「摂動する」）
- **変数名はドメイン概念**: `confidence_vector`, `aggregate_confidence`, `twin_trajectory`, `lyapunov_estimate`
- **一関数一責務**: Mock 生成器は生成のみ、判定関数は判定のみ、計装は観測テスト内
- **ハードコード値の禁止**: 全閾値・重みは `constants.rs` の名前付き定数
- **エラー握りつぶし禁止**: 範囲違反は clamp または Result で明示的に対処

### 既存コードの改善（Boy Scout Rule）

- `src/constants.rs` のカテゴリコメントに `// === Confidence / Mock Proposer ===` セクションを追加

## Acceptance Criteria

- [ ] `ConfidenceVector` 構造体（c_s, c_v, c_h）が定義され、`aggregate()` で統合 confidence を算出できる
- [ ] `MockProposer` が固定シード PRNG で再現可能な confidence 系列を生成する
- [ ] `decide_composition_fate` が confidence 値に基づいて Refine/Finalize/Uncertain を分岐する
- [ ] T1-T11 の全ユニットテストが PASS
- [ ] OTS-1/OTS-2/OTS-3 の全観測テストが PASS
- [ ] ツイン軌道リアプノフ指数 λ(500) < 0.01（非カオス安定性検証）
- [ ] RFC §13.3 および §16.1 との無矛盾確認完了
- [ ] 既存の全テストが通過している（後退なし）
- [ ] 翻訳可能性（関数名は動詞句、変数名はドメイン概念、一関数一責務）を満たしている

## Notes

- `plan_path`: /plan-ticket が plan.md 作成後に frontmatter に更新する
- `implementation_path`: /start-ticket が implementation.md 作成後に frontmatter に更新する
- `review_report_path`: /review-ticket が review.md 作成後に frontmatter に更新する
- `observation_report_path`: /start-ticket が observation-YYYYMMDD-HHmmss.md 作成後に frontmatter に最新パスを更新する

### 成果物

- 計画: context/0051-m0-3-prngconfidence/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0051-m0-3-prngconfidence/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0051-m0-3-prngconfidence/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0051-m0-3-prngconfidence/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
