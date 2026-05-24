---
ticket_id: 78
title: M1.75-7: village stability / dynamicity メトリクス定義および観測パイプラインの実装
slug: m175-7-village-stability-dynamicity
status: approved
created_at: 2026-05-24
updated_at: 2026-05-24
---

# M1.75-7: village stability / dynamicity メトリクス定義および観測パイプラインの実装

## Summary

child-support village の安定性と動特性を定量的に測定するメトリクスを定義し、観測パイプラインを実装する。位置ドリフト (式 41B-21)、近傍集合 Jaccard 重複 (式 41B-22)、village churn (式 41B-23)、helper 分布 Jensen-Shannon divergence、child survival rate、child maturation time の純粋関数を実装し、EventProjection フレームワーク上の `VillageObservationLog` として materialize する。

## Background

M1.75-1 〜 M1.75-6 で village 構成・HELP プロトコル・helper weighting の実装が完了した。これらの機能が健全に動作していることを確認するには、**観測可能なメトリクス**が必要である。

RFC §41B.14 は以下の要件を定義する：
- 位置ドリフト指標 (式 41B-21): `compute_position_drift`
- 短期 village 重複指標 (式 41B-22): `compute_village_jaccard`
- village churn 指標 (式 41B-23): `compute_village_churn`
- helper 重み分布ドリフト: `compute_helper_jsd`
- トラスト成長勾配 (式 41B-24): 将来の M1.76 で拡張予定
- 長期 village 重複指標 (式 41B-25): 将来の M1.76 で拡張予定

RFC §41B.15 は以下の運用指標を推奨する：
- `space_position_drift_p50/p95`, `village_churn_p50/p95`, `helper_jsd_p50/p95`
- `helper_count_mean`, `child_survival_rate`, `child_maturation_time_p50/p95`
- `long_horizon_village_jaccard_p50/p95` (将来拡張)
- `false_new_rate`, `compose_fallback_frequency`, `review_queue_depth`, `review_latency` (既存指標との比較用)

これらの指標は RFC §41B.15 の較正候補（`VILLAGE_STABILITY_MAX_CHURN_P95`, `VILLAGE_DYNAMICITY_MIN_LONG_HORIZON_CHANGE` 等）の初期値設定の根拠となる。

## 参照観察レポート

- tickets/context/0077-m175-6-helper-weightingbounded-remote-exploration-helper/observation-20260524-154834.md — β-ε グリッド掃引により helper 選定の分布特性を観測。helper weighting のエントロピーと探索影響を定量化済み。本チケットの helper_jsd はこの知見を入力として利用する。
- tickets/context/0076-m175-5-child-support-trainingmission-specialization-training-orchestrator/observation-20260524-152336.md — spawn 成功率 ~61% を観測。village サイズ別の発行率 sweep により MAX_HELPERS_PER_MISSION 境界効果を確認。child survival rate のベースラインとして利用可能。

## Scope

### 型定義

- `VillageMetrics`: 単一 tick の全メトリクス値を格納する構造体
  - `position_drifts: Vec<f64>` — 全 child の位置ドリフト (式 41B-21)
  - `village_jaccards: Vec<f64>` — 全 child の短期 Jaccard 重複 (式 41B-22)
  - `village_churns: Vec<f64>` — 全 child の churn 値 (式 41B-23)
  - `helper_jsds: Vec<f64>` — 全 child の helper weight JSD
  - `helper_counts: Vec<usize>` — 全 child の helper 数
  - `child_survival_count: usize` — 生存 child 数
  - `child_maturation_count: usize` — 成熟 child 数
  - `total_child_count: usize` — 総 child 数
  - `tick: u64` — 観測 tick

- `VillageMetricsWindow`: 時系列ウィンドウ集計
  - `metrics: VecDeque<VillageMetrics>` — 直近 N tick 分の生メトリクス
  - `window_size: usize` — ウィンドウサイズ（デフォルト 100 tick）

- `VillageMetricsSnapshot`: 分位点・平均を含む集約スナップショット
  - `position_drift_p50: f64`, `position_drift_p95: f64`
  - `village_churn_p50: f64`, `village_churn_p95: f64`
  - `helper_jsd_p50: f64`, `helper_jsd_p95: f64`
  - `helper_count_mean: f64`
  - `child_survival_rate: f64`
  - `child_maturation_time_p50: f64`, `child_maturation_time_p95: f64`

### 純粋関数

すべての関数は `src/village.rs` に実装する：

- `compute_position_drift(prev_pos: &[f32; 3], curr_pos: &[f32; 3]) -> f64` — 式 41B-21: L2 distance on space position delta
- `compute_village_jaccard(prev_adults: &[WorkflowGraphId], curr_adults: &[WorkflowGraphId]) -> f64` — 式 41B-22: |intersection| / |union|
- `compute_village_churn(jaccard: f64) -> f64` — 式 41B-23: 1 - J(c,t)
- `compute_helper_jsd(prev_weights: &[f64], curr_weights: &[f64]) -> f64` — Jensen-Shannon divergence on helper weight distributions (正規化済み分布を前提)
- `compute_child_survival_rate(surviving: usize, total: usize) -> f64` — surviving / total
- `compute_child_maturation_time(birth_tick: u64, maturation_tick: u64) -> f64` — maturation_tick - birth_tick (tick 単位)

### SimulationRunner 拡張

- `SimulationRunner` に metrics hook を追加し、tick ごとに `VillageMetrics` を記録
- 以下の hook ポイントを定義:
  - `before_metrics_collect`: 集計前処理（デフォルトは no-op）
  - `collect_metrics`: 現 tick のメトリクス収集（必須）
  - `after_metrics_collect`: 集計後処理（デフォルトは no-op）
- `VillageMetricsWindow` によるスライディングウィンドウ集計

### EventProjection 統合

- `VillageObservationLog` を新規 `DomainProjection` として `ProjectionCatalog` に登録
- `DarviumEventKind::Village(VillageEvent::TickCompleted)` を購読
- `initialize_domain_projections()` に `village_observation_log` を追加（5 つ目の projection）
- `SearchTrace` / `TrainingRunLog` / `VillageObservationLog` 間のキー整合:
  - `SearchTrace.mission_id` ↔ `TrainingRunLog.mission_id` ↔ `VillageObservationLog.tick`

## Non-scope

- 長期 village 重複指標 (式 41B-25): M1.75-9 または M1.76 で実装
- トラスト成長勾配 (式 41B-24): M1.76-10 で child growth increment と同時実装
- 較正ループ (J_village 目的関数): M1.75-11 で実装
- 摂動テストスイート: M1.75-9 で実装
- プロパティベースファジング: M1.75-10 で実装
- マルチシード実 SimulationRunner: 既存のパターンに従い、M1.75-8 の replay でカバー

## Investigation

### ソースコード調査結果

1. **既存の型定義** (`src/types.rs:5292-5315`):
   - `VillageObservation` 構造体が既存。`mission_locality`, `helper_locality`, `knowledge_locality`, `delta` の 4 フィールドを持つ。本チケットの `VillageMetrics` はこれと異なり tick 単位の集計メトリクスを格納する。

2. **village モジュール** (`src/village.rs`):
   - `LocalVillage`, `WorkflowMaturity`, `AdultCandidate` が定義済み。本チケットのメトリクス関数はこのモジュールに追記する。

3. **EventProjection フレームワーク** (`src/event.rs:845-1278`):
   - `EventProjection` トレイト、`ProjectionCatalog` トレイト、`FakeProjection`, `DomainProjection`, `FakeProjectionCatalog` が実装済み。
   - `initialize_domain_projections()` が 4 つの projection (`search_trace`, `training_run_log`, `reciprocity_event`, `search_run_log`) を一括登録。ここに `village_observation_log` を追加する。
   - 各 `DomainProjection` は `ProjectionEventFilter` で購読種別をフィルタリング。

4. **DarviumEventKind の既存 Village 関連 variant** (`src/event.rs`):
   - `DarviumEventKind::Reciprocity(ReciprocityEvent::HelpOffered)` など HELP 関連 variant は既存
   - 新規 `DarviumEventKind::Village(VillageEvent::TickCompleted)` および `VillageEvent` 列挙型の定義が必要

5. **childsupport モジュールのテストパターン** (`src/childsupport.rs:460-670`):
   - 不変条件テスト (T-1〜T-10) → 観測テスト (T-O1〜T-O2) → 計装サマリ (T-E1〜T-E2) の三段階構成
   - `mod tests` 内の `#[test]` 関数。`--nocapture` で println! 出力。
   - 固定シード `StdRng::seed_from_u64(12345)` の使用。

6. **RFC §41B.14 の数式**:
   - 式 41B-21: Δ_x(G,t) = ‖x_{t+1}(G) — x_t(G)‖₂ (位置ドリフト)
   - 式 41B-22: J(c,t) = |N_t(c) ∩ N_{t+1}(c)| / |N_t(c) ∪ N_{t+1}(c)| (Jaccard 重複)
   - 式 41B-23: V(c,t) = 1 - J(c,t) (churn)
   - Helper weight JSD: 推奨される分布ドリフト指標（具体的な数式は RFC にないため、標準的な JSD を実装）

### 既存コードの利用可能なヘルパー

- `l2_distance` (`src/spaceposition.rs`): 位置ドリフト計算に再利用可能
- `VillagePosition = [f32; 3]`: 既存の位置型
- `WorkflowGraphId = String`: 既存の ID 型
- `EventProjection` パターン: `DomainProjection` を流用し `ProjectionEventFilter` で VillageEvent のみを購読

### チケット間の依存関係

- **入力依存**: M1.75-6 の helper weighting 実装が完了済み。helper weight 分布が本チケットの helper_jsd の入力となる。
- **出力依存**: M1.75-8 (replay), M1.75-9 (perturbation), M1.75-11 (calibration) が本チケットのメトリクス関数を利用する。
- **Event Architecture 依存**: M1.5-R 系列の EventProjection フレームワークが完了済み。

## Test Plan

### 不変条件テスト

| ID | 分類 | 内容 | 検証方法 |
|----|------|------|----------|
| T-1 | 境界値 | 同一近傍集合のとき Jaccard = 1, churn = 0 | `compute_village_jaccard(&a, &a) == 1.0`, `compute_village_churn(1.0) == 0.0` |
| T-2 | 境界値 | 完全 disjoint な近傍集合のとき Jaccard = 0, churn = 1 | `compute_village_jaccard(&a, &b)` where a∩b=∅ → 0.0, `compute_village_churn(0.0) == 1.0` |
| T-3 | 境界値 | 空集合のとき Jaccard = 0 | `compute_village_jaccard(&[], &[]) == 0.0` |
| T-4 | 境界値 | 片方のみ空のとき Jaccard = 0 | `compute_village_jaccard(&["a"], &[]) == 0.0` |
| T-5 | 正常系 | 同一 helper weight 分布に対して JSD = 0 | `compute_helper_jsd(&[0.5,0.5], &[0.5,0.5])` が epsilon 以内で 0 |
| T-6 | 正常系 | 完全分離した分布の JSD = ln 2 | `compute_helper_jsd(&[1.0,0.0], &[0.0,1.0])` が epsilon 以内で ln 2 |
| T-7 | 境界値 | child survival rate: 全生存 = 1.0 | `compute_child_survival_rate(5, 5) == 1.0` |
| T-8 | 境界値 | child survival rate: 全滅 = 0.0 | `compute_child_survival_rate(0, 5) == 0.0` |
| T-9 | 境界値 | child survival rate: total=0 の場合 panic しない | `compute_child_survival_rate(0, 0) == 0.0` (or 1.0) |
| T-10 | 正常系 | maturation time が正の値を返す | `compute_child_maturation_time(10, 25) == 15.0` |
| T-11 | 境界値 | birth_tick == maturation_tick で 0 | `compute_child_maturation_time(10, 10) == 0.0` |
| T-12 | 境界値 | 位置ドリフト: 同一位置で 0 | `compute_position_drift(&[0,0,0], &[0,0,0]) == 0.0` |
| T-13 | 正常系 | 位置ドリフトが期待値と一致 | `compute_position_drift(&[0,0,0], &[1,0,0])` ~= 1.0 |

### EventProjection 結合テスト

| ID | 内容 | 検証方法 |
|----|------|----------|
| T-E1 | VillageObservationLog projection が VillageEvent::TickCompleted を materialize する | 購読した village event が DomainProjection に記録される |
| T-E2 | VillageObservationLog が他種別イベントを無視する (分離完全性) | 他 domain の event を publish しても村観測 projection が増加しない |
| T-E3 | initialize_domain_projections() に village_observation_log が含まれる | registered_names() に "village_observation_log" が存在する |
| T-E4 | VillageMetricsSnapshot のキーが SearchTrace / TrainingRunLog と整合する | tick ベースのキーが直列化/復元で一致する |

### 観測テスト

| ID | 内容 | 計装方法 |
|----|------|----------|
| T-O1 | メトリクス関数のグリッド掃引 | 位置差・近傍差・重み差を sweep し、各メトリクスの応答曲線を観測 |
| T-O2 | VillageMetricsWindow の集約動作 | ランダム tick 系列を生成し、p50/p95 が期待範囲に収まることを確認 |
| T-O3 | 既存 operational metrics との比較 | Village 導入前後の false_new_rate / compose_fallback_frequency を同一 seed 条件で比較 |

## 計装方法・観測対象

### 計装方法

- 全メトリクス関数は純粋関数として実装し、`mod tests` 内で固定シード `StdRng::seed_from_u64(12345)` を使用
- 観測テストは `println!` + `--nocapture` で CSV 形式の構造化テキストを標準出力に書き出す
- VillageMetricsSnapshot の p50/p95 計算にはソート後の位置統計を使用

### 観測対象

- `position_drift_p50/p95`: 位置ドリフトの分布。通常値 < 1.0 (空間位置の正規化後)
- `village_churn_p50/p95`: churn 分布。0.0 = 完全安定, 1.0 = 完全 churn。健全な値は 0.0〜0.3。
- `helper_jsd_p50/p95`: helper weight 分布ドリフト。0.0 = 変化なし。健全な村では時間経過とともに漸減。
- `helper_count_mean`: 村あたりの平均 helper 数。期待値は TOP_K 付近。
- `child_survival_rate`: 生存 child 比率。RFC 期待値 ≥ 0.8。
- `child_maturation_time_p50/p95`: 成熟までに要する tick 数。小さいほど育成効率が良い。

### サンプルサイズ要件

- 分布同定: n >= 10,000 tick (VillageMetricsWindow の window_size を 10,000 に設定)
- ドリフト検出: n >= 1,000 tick (実用的な感度)
- VillageMetricsWindow のデフォルト window_size: 100 tick

### 較正計画

本チケットでは較正ループは実施せず、定数定義とメトリクス計装のみを行う。以下を `constants.rs` に追加する：

- `VILLAGE_METRICS_WINDOW_SIZE: usize = 100` — VillageMetricsWindow のデフォルトサイズ
- `VILLAGE_EVENT_PROJECTION_NAME: &str = "village_observation_log"` — Projection 登録名
- 既存定数 `SPACE_POSITION_UPDATE_ALPHA`, `HELPER_WEIGHT_DEFAULT_TOP_K` 等をメトリクス観測の文脈で参照

較正ループは M1.75-11 で `J_village(θ)` 目的関数と共に実装する。

## Boy Scout Rule — 翻訳可能性計画

1. **関数名の統一**: 全メトリクス関数は `compute_` プレフィックスで統一し、「〜を計算する」という動詞句として読めるようにする
2. **型名の明確化**: `VillageMetrics` / `VillageMetricsWindow` / `VillageMetricsSnapshot` は名詞として統一感を持たせる。`VillageObservation` (既存) とは役割を明確に区別する
3. **一関数一責務**: 各メトリクス関数は一つの統計量のみを計算する。複合メトリクスの集約は `VillageMetricsWindow` 側で行う
4. **エラー握りつぶし禁止**: ゼロ除算や空リストなどエッジケースでは `f64::NAN` や `0.0` を明確なポリシーで返す
5. **ハードコード値の定数化**: tick 差の閾値、window_size 等は `constants.rs` に追記する

## Acceptance Criteria

- [ ] VillageMetrics / VillageMetricsWindow / VillageMetricsSnapshot が定義されている
- [ ] compute_position_drift (式 41B-21) が実装されている
- [ ] compute_village_jaccard (式 41B-22) が実装されている
- [ ] compute_village_churn (式 41B-23) が実装されている
- [ ] compute_helper_jsd が実装されている
- [ ] compute_child_survival_rate が実装されている
- [ ] compute_child_maturation_time が実装されている
- [ ] 全メトリクス関数の不変条件テスト (T-1〜T-13) が通過している
- [ ] VillageObservationLog が EventProjection として登録されている
- [ ] initialize_domain_projections() に village_observation_log が含まれている
- [ ] VillageEvent::TickCompleted が定義され、VillageObservationLog が購読する
- [ ] VillageMetricsWindow のスナップショット集約が正しく動作する
- [ ] EventProjection 結合テスト (T-E1〜T-E4) が通過している
- [ ] 観測テスト (T-O1〜T-O3) が通過し、出力が確認できる
- [ ] 翻訳可能性の検証が通っている
- [ ] 既存テストが通過している

## Notes

### 成果物

- 計画: context/0078-m175-7-village-stability-dynamicity/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0078-m175-7-village-stability-dynamicity/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0078-m175-7-village-stability-dynamicity/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0078-m175-7-village-stability-dynamicity/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）

### 事前交叉参照

- RFC §41B.14 (式 41B-21〜41B-23): 位置ドリフト・Jaccard・churn の定義
- RFC §41B.15 (operational metrics): 観測すべき指標と較正候補の一覧
- Darvium-Tickets-v2.3.md (M1.75-7): 本チケットのテスト仕様
- Darvium-v2.3-final-table-and-struct-definition-spec.md: VillageObservation の型定義
