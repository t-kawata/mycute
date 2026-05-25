---
ticket_id: 83
title: M1.75-10: property-based village invariant fuzzing と failing seed の replay fixture 昇格
slug: m175-10-property-based-village-invariant-fuzzing-failing-seed-replay-fixture
status: reviewed
created_at: 2026-05-25
updated_at: 2026-05-25
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0083-m175-10-property-based-village-invariant-fuzzing-failing-seed-replay-fixture/plan.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0083-m175-10-property-based-village-invariant-fuzzing-failing-seed-replay-fixture/implementation.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0083-m175-10-property-based-village-invariant-fuzzing-failing-seed-replay-fixture/observation-20260525-155110.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0083-m175-10-property-based-village-invariant-fuzzing-failing-seed-replay-fixture/review.md
---
# M1.75-10: property-based village invariant fuzzing と failing seed の replay fixture 昇格

## Summary

village 不変条件（helper 選定、ConsistencyState フィルタ、HELP 終端状態の非再入性、empty village fallback）を proptest でファジングし、違反を発見した seed を replay fixture に自動昇格する基盤を実装する。

## Background

village 不変条件は RFC §41B で規定されている。既存の M1.75-7 〜 M1.75-9 では決定論的リプレイおよび小摂動実験により village の安定性を確認したが、以下の領域は未検証である：

1. **パラメータ空間の網羅的探索**: `proptest` によるランダム生成で数千〜数万の初期条件に対する invariant 検証
2. **failing seed の replay fixture 昇格**: CI で発見した違反を恒久的な regression テストとして登録する機構
3. **HELP 終端状態の非再入性**: ファジング下での状態機械の安全性保証
4. **empty village fallback**: 全ての Adult が不在の場合に unsafe execution ではなく fallback policy が発火すること

既存の proptest インフラ（src/event.rs の M1.5-R11 proptest 戦略）は event ドメインに特化しており、village ドメイン用の generator 戦略は未整備である。

## Scope

1. **proptest workflow population generator**: ランダムな WorkflowConfig の集合（サイズ 1〜20）を生成する proptest strategy
2. **parameter generator**: `k`（top-K）、`alpha`（不変性重み）、`beta`（新規性重み）、`epsilon`（一様性補正）、maturity thresholds（MIN_SURVIVAL_EXPERIENCE, T_ADULT_THRESHOLD, R_ADULT_THRESHOLD）の乱数生成戦略
3. **invariant assertion suite**: 以下の 4 種の不変条件をアサートする proptest テスト
   - 利用可能な adult が存在する限り child に helper が最低 1 体以上付与される
   - `ConsistencyState != Committed` の helper 混入が 100% 拒否される
   - HELP 終端状態（HelpSuccess, HelpDeclined, etc.）の非再入性
   - empty village で unsafe execution ではなく fallback policy が発火する
4. **failing seed exporter**: 違反を検出した seed を JSON ファイルに保存する機構
5. **replay fixture writer**: 保存した seed を replay シナリオとして fixture ディレクトリに書き出す

## Non-scope

- M1.75-11（較正ハーネス）の実装 — 本チケットは不変条件発見に集中し、パラメータ最適化には踏み込まない
- M1.75-12（実験レポート生成と系列管理）の実装 — fixture の保存形式のみ定義し、レポート生成は別チケットで扱う
- M1.76 の reciprocity/benevolence 不変条件 — 本チケットは M1.75 時点の village invariant に限定
- 実 LLM 結合 — すべて FakeExecutor + FakeLlmClient 前提

## Investigation

### 参照観察レポート

- tickets/context/0082-m175-9-small-perturbation-ranking-stability-village-stability/observation-20260525-151745.md — 小摂動実験の結果、embedding noise generator が fuzzing の入力変動基盤として流用可能であること、quarantine duration sweep パターンがランダム化して再利用可能であることを確認
- tickets/context/0081-m175-8-deterministic-replay-village-help/observation-20260525-150011.md — 決定論的リプレイの完全再現性（n=100 で 100%）、trace JSON 出力パターンが fixture 管理の基盤となることを確認

### 物理的証拠

1. **proptest 依存関係**: `Cargo.toml` に `proptest = "1"` が既に追加済み（22行目）。M1.5-R11（event.rs）で proptest 使用実績あり。

2. **既存の proptest パターン**: `src/event.rs:1308` で `use proptest::prelude::*` および `use proptest::prop_compose` が使用されている。テストは `proptest!` マクロで記述され、`mod tests` 内に配置されている。このパターンを踏襲する。

3. **既存の replay 型定義**:
   - `VillageReplayScenario`（replay.rs:32）: seed, workflows, missions, clock_schedule, policy_bundle を持つ
   - `WorkflowConfig`（replay.rs:46）: id, initial_position, initial_experience, initial_trust, initial_reputation
   - `PolicyBundle`（replay.rs:74）: offer_policy, accept_policy, selection_policy
   - これらを proptest でランダム生成可能

4. **既存の不変条件テスト**:
   - `village.rs` T-8（503行目）: `filter_adult_candidates` の ConsistencyState フィルタ — Committed のみ保持、Pending/NeedsRepair/Quarantined を排除
   - `replay.rs` P-1〜P-8（1591〜1800行目）: 摂動下の各種不変条件
   - これらを proptest で網羅的にランダム検証できる

5. **village 不変条件の実体**:
   - `src/village.rs:filter_adult_candidates` で ConsistencyState フィルタ（要確認: 非 Committed は 100% 拒否）
   - `src/help.rs:decide_help_offer` / `should_offer_help` で HELP 状態遷移
   - `src/village.rs:build_local_village_topk` / `src/childsupport.rs:select_helpers` で helper 選定

6. **fixture 保存機構**: 既存の replay.rs に fixture 保存機能は未実装。新規に FailingSeedFixture 型とファイル I/O が必要。

7. **constants.rs の該当定数**:
   - `MIN_SURVIVAL_EXPERIENCE`, `T_ADULT_THRESHOLD`, `R_ADULT_THRESHOLD` — maturity 閾値
   - `E_ADULT_THRESHOLD` — 経験値 Adult 閾値
   - `VILLAGE_CHURN_P95_WARNING_THRESHOLD`（445行目） — 安定性警告
   - fuzzing 用の新規定数（fuzz iteration count、fixture 保存パス等）は必要に応じて追加

## Test Plan

### テスト構成

既存の M1.75-8/M1.75-9 パターンに従い `src/replay.rs` の `mod tests` に追加する。

### 不変条件テスト（proptest）

| ID | テスト名 | 内容 | 戦略 |
|----|---------|------|------|
| F-1 | `prop_helper_assignment` | 利用可能な Adult が存在する限り、各 Child に最低 1 体の helper が付与される | ランダムな WorkflowConfig 集合 + policy → build_local_village_topk → select_helpers → 各 Child の helper 数 ≥ 1 |
| F-2 | `prop_consistency_state_filter` | ConsistencyState != Committed の AdultCandidate が helper として選定されない | filter_adult_candidates の網羅的ファジング |
| F-3 | `prop_help_terminal_non_reentrance` | HELP 終端状態から非終端状態への遷移が発生しない | HelpState 状態機械のランダム遷移系列 |
| F-4 | `prop_empty_village_fallback` | 全 Adult 不在時に unsafe execution ではなく fallback policy が発火する | adult_count=0 の条件で build_local_village_topk |
| F-5 | `prop_maturity_classification` | classify_maturity が全軸非負で panic しない | ランダムな experience/trust/reputation の組み合わせ |

### Parameter Generator 戦略

| パラメータ | 生成範囲 | 分布 |
|-----------|---------|------|
| top_k (k) | 1..=10 | 一様整数 |
| alpha | 0.0..1.0 | 一様浮動小数点 |
| beta | 0.0..1.0 | 一様浮動小数点（alpha+beta ≤ 1） |
| epsilon | 0.0..0.5 | 一様浮動小数点 |
| MIN_SURVIVAL_EXPERIENCE | 0..=100 | 一様整数 |
| T_ADULT_THRESHOLD | 0.0..1.0 | 一様浮動小数点 |
| R_ADULT_THRESHOLD | 0.0..1.0 | 一様浮動小数点 |
| workflow_count | 0..=20 | 一様整数（0 は empty village テスト用） |

### モック/外部依存

- FakeExecutor + FakeLlmClient を使用（M-1 で実装済み）
- 新たな外部依存は不要

### fixture 出力テスト

| ID | テスト名 | 内容 |
|----|---------|------|
| F-6 | `prop_fixture_export_roundtrip` | 違反 seed → JSON 保存 → 再読み込み → seed 一致 |
| F-7 | `prop_fixture_replay_regression` | 保存した fixture を replay シナリオに変換し、同一違反が再現する |

## 計装方法・観測対象

### 計装方法

- 全テストは `StdRng::seed_from_u64(12345)` ベースの固定シード PRNG を使用（既存 replay.rs のパターンに準拠）
- proptest の `ProptestConfig { cases: 10000 }` をデフォルトとし、10,000 ケースのファジングを実行
- `println!` + `--nocapture` で以下の構造化データを出力
  - テストケースごとの invariant violation 有無
  - violation 時のパラメータスナップショット（簡易 JSON）
  - seed 値と population size
- 違反検出時は `FailingSeedEntry { seed, population, invariant_id, parameter_snapshot }` を JSON ファイルに保存
- fixture は `tests/fixtures/village_invariant_failures/` に出力

### 観測対象

| 統計量 | 用途 | 閾値 |
|--------|------|------|
| violation_rate | 全ケース中の invariant 違反率 | < 0.001（許容違反率） |
| min_failing_population_size | 違反が観測された最小 population | 記録・分析用 |
| parameter_clustering | 違反が集中するパラメータ領域の有無 | 分析用（定性評価） |
| seed_unique_count | 違反検出 seed のユニーク数 | 記録・回帰テスト編入用 |

### 較正計画

本チケットでは較正ループは実施しない（M1.75-11 に委譲）。proptest の iteration 数（10,000）は既存の event.rs のパターンを踏襲する。

## Boy Scout Rule — 翻訳可能性計画

- `filter_adult_candidates`、`build_local_village_topk`、`select_helpers` はいずれも関数名が「何をするか」を明確に語っており、翻訳可能性は高い。proptest 戦略も同水準の命名を維持する。
- 既存の replay.rs 固定シード定数 `REPLAY_POSITION_DELTA_SIGMA` のような名前付き定数パターンに従い、fuzzing パラメータも `constants.rs` に集約する。ハードコードされた iteration 数や閾値は許容しない。
- proptest 戦略の責務は「生成」に限定し、検証ロジック（invariant assertion）と分離する。1 戦略 1 責務の原則を守る。
- エラーの握りつぶし（`unwrap` / `expect`）は fixture ファイル I/O でのみ限定的に許容し、それ以外は `Result` 伝播を使用する。

## Acceptance Criteria

- [x] 実装要件を満たしている（spec の各セクションが Darvium-Tickets-v2.3.md の M1.75-10 仕様と一致）
- [ ] 翻訳可能性の検証が通っている（proptest 戦略が 1 戦略 1 責務を満たす）
- [ ] 既存テストが通過している

### 検証項目

- [ ] F-1: ランダム population 全域で helper 選定 invariant が破れない
- [ ] F-2: `ConsistencyState != Committed` helper 混入が 100% 検出・拒否される
- [ ] F-3: HELP 終端状態の非再入性が fuzz 下でも維持される
- [ ] F-4: empty village ケースで unsafe execution ではなく fallback policy が発火する
- [ ] F-5: classify_maturity が全軸非負で panic しない
- [ ] F-6: fixture export roundtrip が正しく動作する
- [ ] F-7: 保存した fixture から replay シナリオを生成し違反が再現する

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

- 計画: context/0083-m175-10-property-based-village-invariant-fuzzing-failing-seed-replay-fixture/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0083-m175-10-property-based-village-invariant-fuzzing-failing-seed-replay-fixture/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0083-m175-10-property-based-village-invariant-fuzzing-failing-seed-replay-fixture/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0083-m175-10-property-based-village-invariant-fuzzing-failing-seed-replay-fixture/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
