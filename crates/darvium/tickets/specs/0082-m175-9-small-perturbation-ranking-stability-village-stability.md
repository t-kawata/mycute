---
ticket_id: 82
title: M1.75-9: small perturbation 実験スイート（ranking stability 相当の village stability 検証）
slug: m175-9-small-perturbation-ranking-stability-village-stability
status: reviewed
created_at: 2026-05-25
updated_at: 2026-05-25
spec_source: Darvium-Tickets-v2.3.md L1066-L1079
rfc_ref: RFC §41B small perturbation stability
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0082-m175-9-small-perturbation-ranking-stability-village-stability/observation-20260525-151745.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0082-m175-9-small-perturbation-ranking-stability-village-stability/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0082-m175-9-small-perturbation-ranking-stability-village-stability/review.md
---

# M1.75-9: small perturbation 実験スイート（ranking stability 相当の village stability 検証）

## Summary

微小な embedding ノイズ、trust 変動、single-edge patch、利用履歴 1 件追加などの小摂動に対し、
village 構造と helper 分布が catastrophic oscillation を起こさないことを実験的に検証する。
perturbation generator 群を実装し、baseline / perturbed シナリオ比較器によって
`StabilityRegressionSummary` を出力する観測パイプラインを構築する。

## Background

Darvium v2.3 の ranking stability 検証（M2.75-1 等）では、GED 境界近傍における
ランキングの微小摂動耐性をテストしている。これと同様の概念を village 領域に拡張する。

village 構造（child → adult マッピング）と helper 分布は、以下の摂動要因によって影響を受ける可能性がある：

1. **Embedding ノイズ**: 空間位置の微小変動によって village 構成半径が変化し、helper 選定結果が変動する
2. **Trust 変動**: Adult の信頼値の微小な上下が helper weight に影響し、選定順位が変動する
3. **Single-edge patch**: ワークフローグラフの局所的な変更によって成熟度判定が変化する
4. **Usage increment**: 利用履歴の追加によって経験値や信頼値が変化する
5. **Helper quarantine**: helper 1 体の一時隔離によって child 全体の支援が一時的に滞る

これらの摂動が **catastrophic oscillation**（village_churn の急増、helper 分布の急変、child 支援の全面崩壊）を
引き起こさないことを検証する。これにより village 動態の頑健性を保証する。

### 参照観察レポート

- tickets/context/0081-m175-8-deterministic-replay-village-help/observation-20260525-150011.md — replay エンジン（M1.75-8）の確立。seed 変更時の metrics 安定性確認済み（churn P50 < 0.5, JSD P50 < 0.3, 生存率 > 0.5）
- tickets/context/0078-m175-7-village-stability-dynamicity/observation-20260524-164848.md — village metrics 定義

## Scope

### 実装内容

1. **perturbation generator 群**（`src/replay.rs` 内に追加、または新規モジュール `src/perturbation.rs`）:
   - `EmbeddingNoise`: 空間位置の微小ノイズ注入（ガウシアン、σ 可変）
   - `TrustDelta`: 特定ワークフローの信頼値の ±Δ 変更
   - `SingleEdgePatch`: ワークフローグラフの局所変更（成熟度に影響）
   - `UsageIncrement`: 利用履歴 1 件相当の経験値追加
   - `TemporaryHelperQuarantine`: helper 1 体の一時隔離

2. **baseline / perturbed 比較器**:
   - 同一 seed で baseline trace と perturbed trace を比較
   - ReplayTrace の部分比較（diff_fields の拡張）

3. **`StabilityRegressionSummary` 出力型**:
   - 摂動種別ごとの churn/helper_jsd/survival_rate 変動量
   - 臨界摂動強度 σ_c の推定値

4. **観測テスト**（`tests/` または `src/replay.rs` 内）:
   - 微小ノイズ注入下で `village_churn` と `helper_jsd` が上限閾値を超えにくいことの検証
   - helper 1 体の一時隔離で child-support 全体が全面崩壊しないことの検証
   - patch による局所変化が global village structure の無制限振動へ波及しないことの検証
   - 摂動強度 σ 掃引による `village_churn_p95(σ)` と `helper_jsd_p95(σ)` の応答曲線観測

### 新規定数（`src/constants.rs`）

- `PERTURB_EMBEDDING_NOISE_SIGMA_DEFAULT: f64 = 0.02` — Embedding ノイズ注入のデフォルト標準偏差 (Calibration Candidate)
- `PERTURB_TRUST_DELTA_DEFAULT: f64 = 0.05` — 信頼値摂動のデフォルト Δ (Calibration Candidate)
- `PERTURB_USAGE_INCREMENT_DEFAULT: u64 = 1` — 利用履歴増分デフォルト値 (Environment Policy Knob)
- `PERTURB_CHURN_MAX_P95_INCREASE: f64 = 0.10` — 許容 churn P95 増加量上限 (Safety Invariant)
- `PERTURB_JSD_MAX_P95_INCREASE: f64 = 0.10` — 許容 JSD P95 増加量上限 (Safety Invariant)

## Non-scope

- 本格的な較正ループ（`J_village(θ)` の実装と較正ハーネスは M1.75-11 に委譲）
- property-based fuzzing と failing seed の replay fixture 昇格（M1.75-10 に委譲）
- 実環境 embedding プロバイダとの結合（M1.5-1 等、別チケット）
- Reciprocity-Aware Survival 領域の摂動（M1.76-14 SHALL perturbation に委譲）

## Investigation

### 物理的証拠

**証拠 1: 既存 replay エンジンの基盤確立（`src/replay.rs`）**

M1.75-8 で実装された `run_replay_scenario` 関数は `src/replay.rs:262` にあり、
`VillageReplayScenario` → `ReplayTrace` の変換を行う。以下のコンポーネントが既存：

- **位置更新**: ランダムウォーク (`rng.random_range(-delta_sigma..delta_sigma)`, replay.rs:295-298)
- **成熟度判定**: `classify_maturity()` による Child/Adult 分類 (replay.rs:310-318)
- **Village 構成**: `build_local_village_topk()` / `filter_adult_candidates()` (replay.rs:345-385)
- **Helper 選定**: `select_helpers()` (replay.rs:366-385)
- **HELP プロトコル**: Proposal→Offered→Accepted→Executing→Succeeded (replay.rs:397-461)
- **Child 成長**: 1 tick あたり +1 experience (replay.rs:508-518)

**証拠 2: trace 比較インフラ（`src/replay.rs:537-713`）**

`trace_diff_fields()` と `trace_eq()` は既存で、2 つの ReplayTrace の全フィールドを
浮動小数点許容誤差付きで比較する。M1.75-9 ではこれを拡張し、perturbation 後の
trace が baseline と「どの程度異なるか」の定量比較に使用する。

**証拠 3: メトリクス計算インフラ（`src/replay.rs:716-862`）**

`trace_summary_metrics()` は `SummaryMetrics` を計算する。既存の統計量：
- `village_churn_p50/p95`: Jaccard 非類似度ベース
- `helper_jsd_p50/p95`: Jensen-Shannon Divergence
- `child_survival_rate`, `child_maturation_rate`, `helper_count_mean`

これらは perturbation 後の metrics 変化量 Δchurn, ΔJSD の計算にそのまま使用可能。

**証拠 4: 既存の類似 perturbation 実装（`src/types.rs:4968`）**

`ConfidenceVector::perturb()` メソッドが既存：
```rust
pub fn perturb(&self, delta: f64, rng: &mut impl rand::Rng) -> Self
```
このパターン（固定 seed PRNG + delta パラメータ）を模倣して village-level perturbation
generator を実装する。

**証拠 5: 既存の定数定義パターン（`src/constants.rs`）**

較正候補定数は `Calibration Candidate` タグ付きで constants.rs に一元管理されている。
新規定数もこのパターンに従う。

**証拠 6: ビルド確認**

`cargo check` 成功。全テスト通過。M1.75-8 のテスト 11 件（T-1〜T-9, T-O1, T-O2）が全 PASS。

### 実装判断

- **モジュール配置**: `src/replay.rs` の末尾に perturbation generator 群を追加
  （replay エンジンと密結合であり、新規モジュール分割のコストに見合わない段階）。
  ただし、将来の M1.75-10/11 での肥大化を見越し、内部モジュール `mod perturbation` として
  独立したセクションに配置する。
- **型定義**: `StabilityRegressionSummary` は `replay.rs` の公開型として定義し、
  `lib.rs` で re-export する。これにより MYCUTE からの観測結果取得を可能にする。
- **テスト**: 観測テストは `src/replay.rs` 内の `mod tests` に追加（既存の T-O1/T-O2 パターンに合流）。

## Test Plan

### ユニットテスト計画

以下のテストは `src/replay.rs` の `mod tests` 内に実装する：

| ID | 名称 | 内容 | 種別 |
|----|------|------|------|
| P-1 | `test_p1_embedding_noise_baseline_diff` | EmbeddingNoise 注入後、位置が baseline と異なることの確認 | 正常系 |
| P-2 | `test_p2_embedding_noise_churn_bounded` | EmbeddingNoise 下での churn P95 増加が上限閾値以内であること | 不変条件 |
| P-3 | `test_p3_trust_delta_churn_bounded` | TrustDelta 下での churn P95 増加が上限閾値以内であること | 不変条件 |
| P-4 | `test_p4_single_edge_patch_no_catastrophic` | SingleEdgePatch (1 変数変更) で village が全面崩壊しないこと | 不変条件 |
| P-5 | `test_p5_usage_increment_stable` | UsageIncrement (+1) で metrics が許容範囲内であること | 不変条件 |
| P-6 | `test_p6_helper_quarantine_no_collapse` | helper 1 体の一時隔離で後続 tick の child survival が維持されること | 不変条件 |
| P-7 | `test_p7_zero_noise_identical_trace` | ノイズ強度 0 では baseline と完全一致すること | 境界値 |
| P-8 | `test_p8_extreme_noise_detects_change` | 極端なノイズ強度 (σ=10.0) では churn 増加が検出されること | 異常系 |

### 観測テスト計画

| ID | 名称 | 内容 |
|----|------|------|
| O-P1 | `test_op1_sigma_sweep` | 摂動強度 σ を 0.0→1.0 まで sweep し、`village_churn_p95(σ)` と `helper_jsd_p95(σ)` の応答曲線を CSV 出力 |
| O-P2 | `test_op2_quarantine_duration_sweep` | helper quarantine 期間を 1〜10 tick まで sweep し、生存率・回復時間を記録 |

### モック/スタブ

- すべての perturbation generator は PRNG ベースで動作（`StdRng::seed_from_u64` + `SeedableRng`）
- シナリオは `basic_scenario()` を baseline として使用（M1.75-8 の既存シナリオ）
- 外部依存は一切不要（純粋関数 + 既存 replay エンジン）

## 計装方法・観測対象

### 計装方法

- **実装場所**: `src/replay.rs` 内に公開関数群として実装
- **テストコード**: `src/replay.rs` の `mod tests` 内
- **計測プローブ**: `println!` + `--nocapture` による CSV 形式出力（T-O1/T-O2 と同一形式）
- **PRNG**: `StdRng::seed_from_u64(12345)` 固定（全テストで再現性保証）
- **perturbation 関数シグネチャ**:
  ```rust
  pub fn apply_embedding_noise(
      scenario: &VillageReplayScenario,
      noise_sigma: f64,
      rng: &mut StdRng,
  ) -> VillageReplayScenario;
  
  pub fn apply_trust_delta(
      scenario: &VillageReplayScenario,
      workflow_id: &str,
      delta: f64,
  ) -> VillageReplayScenario;
  
  pub fn compare_perturbed_metrics(
      baseline: &ReplayTrace,
      perturbed: &ReplayTrace,
  ) -> StabilityRegressionSummary;
  ```

### 観測対象

- **churn P95 の増加量**: Δchurn_p95 = churn_p95(perturbed) - churn_p95(baseline)。上限: 0.10
- **helper JSD P95 の増加量**: ΔJSD_p95 = jsd_p95(perturbed) - jsd_p95(baseline)。上限: 0.10
- **child survival rate の減少量**: Δsurvival = survival(baseline) - survival(perturbed)。上限: 0.20
- **摂動強度 σ とメトリクス応答の相関**: 線形/非線形の別、相転移点 σ_c の有無
- **臨界摂動強度 σ_c**: churn P95 が VILLAGE_STABILITY_MAX_CHURN_P95 (0.30) を超える σ の最小値

### 較正計画

本チケットの較正は M1.75-11 に委譲する。本チケットでは以下を準備する：
- 各 perturbation generator のデフォルトパラメータ値を constants.rs に定義
- O-P1/O-P2 の観測結果を CSV として出力し、後続の較正ハーネスが消費可能な形式で保存

## Boy Scout Rule — 翻訳可能性計画

1. **perturbation generator 群の関数名**: `apply_embedding_noise`, `apply_trust_delta` 等、
   動詞句（「〜を適用する」）で統一し、replay.rs 内の散文可読性を維持する。
2. **StabilityRegressionSummary のフィールド**: `delta_churn_p95`, `delta_jsd_p95` 等、
   Δ 接頭辞で「変化量」であることを明示する。
3. **責務分離**: perturbation generator（摂動の定義）と比較器（摂動の評価）を別関数に分離し、
   一関数一責務を守る。
4. **ハードコード排除**: ノイズ強度・信頼 Δ 等の数値はすべて constants.rs の名前付き定数から
   取得する。テスト内でのマジックナンバー直接指定を禁止する。
5. **エラー握りつぶし禁止**: 摂動適用時に不正なパラメータ（空ワークフローリスト、存在しない workflow_id 等）は
   panic または Result::Err で報告する。

## Acceptance Criteria

- [ ] 5 種の perturbation generator（EmbeddingNoise / TrustDelta / SingleEdgePatch / UsageIncrement / TemporaryHelperQuarantine）が実装されている
- [ ] baseline と perturbed の比較器（`compare_perturbed_metrics`）が実装され、`StabilityRegressionSummary` を返す
- [ ] 以下の不変条件テストが全 PASS すること:
  - P-1〜P-8（ユニットテスト 8 件）
  - O-P1（σ sweep 観測テスト）
  - O-P2（quarantine duration sweep 観測テスト）
- [ ] 既存テスト（M1.75-8 の T-1〜T-9, T-O1, T-O2）が全 PASS していること
- [ ] 新規定数が `constants.rs` に適切な分類タグ（Calibration Candidate / Safety Invariant / Environment Policy Knob）付きで定義されていること
- [ ] 翻訳可能性の検証が通っている（関数名は動詞句、変数名はドメイン概念、ハードコード値は名前付き定数）
- [ ] `cargo clippy -- -D warnings` が通過すること

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

- 計画: context/0082-m175-9-small-perturbation-ranking-stability-village-stability/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0082-m175-9-small-perturbation-ranking-stability-village-stability/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0082-m175-9-small-perturbation-ranking-stability-village-stability/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0082-m175-9-small-perturbation-ranking-stability-village-stability/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
