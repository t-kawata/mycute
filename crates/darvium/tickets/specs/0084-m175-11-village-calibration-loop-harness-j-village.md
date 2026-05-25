---
ticket_id: 84
title: "M1.75-11: village calibration loop harness と目的関数 J_village(θ) の実装"
slug: m175-11-village-calibration-loop-harness-j-village
status: reviewed
created_at: 2026-05-25
updated_at: 2026-05-25
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0084-m175-11-village-calibration-loop-harness-j-village/plan.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0084-m175-11-village-calibration-loop-harness-j-village/implementation.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0084-m175-11-village-calibration-loop-harness-j-village/observation-20260525-161942.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0084-m175-11-village-calibration-loop-harness-j-village/review.md
---
# M1.75-11: village calibration loop harness と目的関数 \(J_{village}(\theta)\) の実装

## Summary

village-help パラメータ群（alpha, beta, k, epsilon, maturity thresholds 等）を calibration candidate として管理し、目的関数 \(J_{village}(\theta)\) に基づく系統的較正ループ（one-factor-at-a-time sweep / grid sweep / Latin hypercube sampling の 3 モード）を実装する。結果は `CalibrationReport` として保存し、パラメータ感度分析と安定高原の可視化を可能にする。

## Background

village-help の各パラメータは RFC §41B で calibration candidate と指定されている（SPACEPOSITIONUPDATEALPHA や maturity thresholds など）。M1.75-7〜M1.75-10 では village の安定性測定・摂動実験・プロパティベースファジングを実装したが、以下の領域は未着手である：

1. **目的関数の定義と実装**: churn, JSD, survival, false-new, review-load を統合する合成目的関数 \(J_{village}(\theta)\) が未実装
2. **系統的パラメータ探索**: 3 モード（one-factor-at-a-time / grid sweep / Latin hypercube sampling）の sweep 機構が存在しない
3. **較正ループの統合ハーネス**: 全パラメータを束として受け取り replay → metrics 算出 → \(J(\theta)\) 評価 → 記録 を自動化するハーネスがない
4. **結果の永続化と比較**: `CalibrationReport` によるバージョン管理された結果保存がない

既存の `VillageMetrics`（village.rs:355-380）は churn P95, JSD P95, survival rate 等の観測値を提供しており、これらを入力として目的関数を計算する。

## Scope

1. **`VillageCalibrationConfig` 構造体**: 較正対象パラメータの束（`SpacePositionUpdateAlpha` 等）と sweep 設定（モード・ステップ数・範囲）を保持
2. **`VillageCalibrationHarness`**: シミュレーション実行 → metrics 収集 → 目的関数評価 を自動化するループ制御器
3. **`VillageCalibrationResult`**: パラメータ設定・目的関数値・成分値（churn/JSD/survival/false-new/review-load）を保持
4. **目的関数 \(J_{village}(\theta)\)**:
   \[
   J_{village}(\theta) = a_1 (1 - churn_{p95}) + a_2 (1 - jsd_{p95}) + a_3 survival - a_4 false\_new - a_5 review\_load
   \]
5. **3 種の sweep モード**: one-factor-at-a-time sweep / grid sweep / Latin hypercube sampling
6. **`CalibrationReport`**: 結果を保存しバージョン管理する出力器
7. **定数定義**: 目的関数重み \(a_1\)〜\(a_5\) および各 sweep パラメータを `src/constants.rs` に追記

## Non-scope

- M1.75-12（実験レポート生成と系列管理）— 本チケットは `CalibrationReport` の保存形式のみ定義し、Markdown レポート生成や実験系列管理には踏み込まない
- M1.76 の reciprocity calibration — reciprocity パラメータの較正は M1.76-16 で扱う
- 実 LLM 結合 — すべて FakeExecutor + FakeLlmClient 前提
- 自動パラメータ最適化（勾配降下・ベイズ最適化等）— 本チケットは sweep ベースの粗探査に限定

## Investigation

### 物理的証拠

1. **既存の metrics 構造体**: `VillageMetrics`（village.rs:355-380）が以下を提供:
   - `village_churn_p50`, `village_churn_p95`
   - `helper_jsd_p50`, `helper_jsd_p95`
   - `child_survival_rate`
   - `maturation_rate`
   - `helper_count_mean`, `helper_count_median`
   - `total_help_sessions`, `total_help_success`

2. **既存の replay scenario 型**: `VillageReplayScenario`（replay.rs:32）が seed, workflows, policy_bundle, clock_schedule を保持

3. **既存の simulation runner**: `replay.rs:deterministic_replay` 関数が metrics を返す。これを calibration harness がラップして利用する

4. **既存 constants**（constants.rs）:
   - `VILLAGE_STABILITY_MAX_CHURN_P95`（450行目）— churn 警告閾値
   - `VILLAGE_DYNAMICITY_MIN_LONG_TERM_CHANGE`（454行目）— JSD 警告閾値
   - `PERTURB_CHURN_MAX_P95_INCREASE`, `PERTURB_JSD_MAX_P95_INCREASE`
   - 新規定数: \(a_1\)〜\(a_5\) の重み、sweep ステップ数等が必要

5. **Latin hypercube sampling 実装**: 現状の依存関係（rand のみ）では不足。既存の `rand::Rng` に加え、必要なら LHS の自前実装か `rand_distr` の活用を検討

6. **calibration-loop.md の欠如**: `rules/darvium/calibration-loop.md` は存在しない。本チケットで原則を確立するため、ハーネス設計に合わせて最小限のルール文書を生成する

### 依存関係

- [`replay.rs`]: `deterministic_replay` 関数が metrics を返すシグネチャを持つこと（確認: 現在 `run_deterministic_replay_scenario` が `VillageMetrics` を返す）
- [`village.rs`]: `VillageMetrics` に `false_new_count` と `review_load_count` がない場合、本チケットで追加が必要
- [`constants.rs`]: 目的関数重みと sweep 設定定数の追加

## Test Plan

### テスト構成

`src/calibration.rs` 新規作成（`mod calibration` として module 化）。テストは同一ファイル内の `mod tests` に配置する。

### 目的関数テスト

| ID | テスト名 | 内容 |
|----|---------|------|
| C-1 | `objective_deterministic_identical_params` | 同一パラメータ束で複数回評価したとき目的関数値が完全一致する |
| C-2 | `objective_extreme_params_degradation` | 極端なパラメータ（高 alpha、高 beta、低 k）が churn/JSD を悪化させ目的関数が低下する |
| C-3 | `objective_high_survival_positive_contribution` | high survival が目的関数に正の寄与をする |
| C-4 | `objective_weight_sensitivity` | weight を個別に変化させたとき対応成分が比例的に影響を受ける |
| C-5 | `objective_boundary_values_no_panic` | zero や negative metrics でも panic しない |

### Sweep モードテスト

| ID | テスト名 | 内容 |
|----|---------|------|
| C-6 | `sweep_one_factor_at_a_time` | OFAT sweep が全パラメータを 1 要素ずつ変化させ、結果が `Vec<VillageCalibrationResult>` として返る |
| C-7 | `sweep_grid` | grid sweep が直積パラメータ組合せを生成し、結果に重複がない |
| C-8 | `sweep_latin_hypercube` | LHS が指定サンプル数だけ一様分布に近いサンプルを生成する |
| C-9 | `sweep_identical_mode_consistency` | 同一設定の sweep で結果が決定論的に一致する（シード固定時） |

### 統合テスト

| ID | テスト名 | 内容 |
|----|---------|------|
| C-10 | `harness_harmless_sweep_no_invariant_break` | harmless な sweep 実行が既存 invariant を壊さず完走する |
| C-11 | `calibration_report_format` | CalibrationReport が JSON として valid で全必須フィールドを持つ |
| C-12 | `harness_empty_parameter_set` | 空パラメータセットで graceful にスキップする |

### モック/外部依存

- 既存の `FakeExecutor` + `FakeLlmClient` を使用（M-1 実装済み）
- `deterministic_replay` を seed 固定 PRNG で実行可能（M1.75-8 実装済み）
- LHS は自前実装（`rand::Rng` の一様分布で十分）

## 計装方法・観測対象

### 計装方法

- 全テストは固定シード PRNG（`StdRng::seed_from_u64(12345)`）を使用
- `println!` + `--nocapture` で以下の構造化データを出力:
  - sweep 実行ごとのパラメータ設定 → 各成分値 → \(J(\theta)\) の CSV 行
  - mode ごとの経過時間・評価回数
- `CalibrationReport` は JSON として `target/calibration/` に保存（`make test` のアーティファクトとして管理）

### 観測対象

| 統計量 | 用途 | 閾値 |
|--------|------|------|
| \(J(\theta)\) 分布 | 目的関数地形の粗視化把握 | 分析用 |
| \(J(\theta)\) の plateau 有無 | 安定パラメータ領域の同定 | 分析用 |
| 感度ベクトル \(\nabla J\) 近似 | 支配パラメータの分離 (ANOVA 的概念) | 分析用 |
| sweep 実行時間 | ハーネス性能のベースライン | < 30s (1回の全モード実行) |

### 較正計画

本チケットは較正のための**ハーネス**を実装し、sweep を実行可能にする。本格的な較正（Phase 0-4 の反復）は M1.75-12（実験レポート生成）完了後、または M1.76 の較正フェーズで実施する。

## Boy Scout Rule — 翻訳可能性計画

- `VillageCalibrationHarness` は「較正ハーネス」として一貫した責務（パラメータ設定 → 実行 → 結果収集）を持ち、内部でシミュレーション実行をラップする
- 3 種の sweep モードはそれぞれ独立した関数とし、`run_sweep_ofat` / `run_sweep_grid` / `run_sweep_lhs` の命名で「何を sweep するか」を関数名で語らせる
- 目的関数 \(J(\theta)\) は `compute_village_objective` 関数として抽出し、成分重みは引数で受け取る純粋関数とする
- 新規 constants は constants.rs に追記し、分類（Calibration Candidate）を明記する
- `unwrap` は `CalibrationReport` のファイル I/O でのみ限定的に許容し、それ以外は `Result` 伝播を使用する

## Acceptance Criteria

- [ ] 実装要件を満たしている（spec の各セクションが Darvium-Tickets-v2.3.md の M1.75-11 仕様と一致）
- [ ] 翻訳可能性の検証が通っている（関数が 1 責務を満たす）
- [ ] 既存テストが通過している

### 検証項目

- [ ] C-1: 同一パラメータ束で目的関数値が決定論的に一致する
- [ ] C-2: 極端なパラメータで目的関数が低下する
- [ ] C-3: high survival が正の寄与をする
- [ ] C-4: weight 感度が比例する
- [ ] C-5: 境界値で panic しない
- [ ] C-6: OFAT sweep が正しく動作する
- [ ] C-7: grid sweep が正しく動作する
- [ ] C-8: LHS sweep が正しく動作する
- [ ] C-9: sweep の決定論的再現性が保証される
- [ ] C-10: sweep 実行が invariant を壊さない
- [ ] C-11: CalibrationReport が適切な形式で出力される
- [ ] C-12: 空パラメータセットで graceful に動作する

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

- 計画: context/0084-m175-11-village-calibration-loop-harness-j-village/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0084-m175-11-village-calibration-loop-harness-j-village/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0084-m175-11-village-calibration-loop-harness-j-village/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0084-m175-11-village-calibration-loop-harness-j-village/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
