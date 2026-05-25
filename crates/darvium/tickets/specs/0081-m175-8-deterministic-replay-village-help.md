---
ticket_id: 81
title: M1.75-8: deterministic replay シナリオによる village-help 再現性テスト
slug: m175-8-deterministic-replay-village-help
status: reviewed
created_at: 2026-05-25
updated_at: 2026-05-25
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0081-m175-8-deterministic-replay-village-help/plan.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0081-m175-8-deterministic-replay-village-help/observation-20260525-150011.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0081-m175-8-deterministic-replay-village-help/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0081-m175-8-deterministic-replay-village-help/review.md
---

# M1.75-8: deterministic replay シナリオによる village-help 再現性テスト

## Summary

既存の village 構造（M1.75-2/7）と HELP プロトコル（M1.75-3/4）および helper weighting（M1.75-6）を統合した決定論的リプレイシナリオを実装し、固定 seed・固定 population・固定 mission stream・固定 VirtualClock 進行のもとで bit-level 再現性を検証する。リプレイ機構は将来の golden trace 回帰テストの基盤となる。

## Background

RFC §41B replay discipline に従い、village 構造（位置・村構成メンバーシップ）と HELP outcome（提案・オファー・実行・成功・失敗）は、全外部入力が固定された条件下で完全再現可能でなければならない（MUST）。これは以下の理由による：

1. **科学的再現性**: シミュレーション結果が実行ごとに変動する場合、「改善」が真の改良か偶然の変動かを区別できない
2. **バグ再現**: 複雑な村動的現象が発生したときに、同一条件で再実行できないと根本原因分析が不可能
3. **回帰テスト**: golden trace を fixture として保存することで、コード変更が既存の village 動態を意図せず変更していないことを自動検証できる

現在、village モジュールの各純粋関数（`classify_maturity`, `build_local_village_topk`, メトリクス計算）は個別にテストされているが、これらを**統合したシナリオ実行**と**trace の完全一致検証**は未実装である。

## Scope

### 新規実装: `src/replay.rs`

**データ型:**

1. `VillageReplayScenario`
   - `seed: u64` — StdRng のシード値
   - `workflows: Vec<WorkflowConfig>` — 参加ワークフローの初期設定（ID, 初期位置, 初期経験値, 初期信頼, 初期レピュテーション）
   - `missions: Vec<MissionSpec>` — 一定間隔で注入されるミッション仕様
   - `clock_schedule: ClockSchedule` — tick 数と各 tick の進行ルール
   - `policy_bundle: PolicyBundle` — 使用するポリシー群（AdultHelpOfferPolicy, ChildHelpAcceptancePolicy, HelperSelectionPolicy）

2. `WorkflowConfig`
   - `id: WorkflowGraphId`
   - `initial_position: [f32; 3]`
   - `initial_experience: u64`
   - `initial_trust: f64`
   - `initial_reputation: f64`

3. `MissionSpec`
   - `trigger_tick: u64`
   - `description: String`（村更新をトリガーする仕様。実際のミッション内容はシナリオとして簡略化）

4. `ClockSchedule`
   - `total_ticks: u64`
   - 各 tick で何が起きるか（例: 毎 tick 位置更新 or 定刻でのみ更新）

5. `PolicyBundle`
   - `offer_policy: AdultHelpOfferPolicy`
   - `accept_policy: ChildHelpAcceptancePolicy`
   - `selection_policy: HelperSelectionPolicy`

6. `ReplayTrace`
   - `space_positions: Vec<TickPositions>` — 各 tick の全ワークフロー位置スナップショット
   - `villages: Vec<TickVillages>` — 各 tick の全 village（child→adult マッピング）
   - `helper_weights: Vec<TickHelperWeights>` — 各 tick の helper weight 分布
   - `help_sessions: Vec<HelpSessionTrace>` — 各 HELP セッションの状態遷移履歴
   - `child_growth_events: Vec<GrowthEvent>` — 経験値増加・成熟イベント

7. `TickPositions` / `TickVillages` / `TickHelperWeights` / `HelpSessionTrace` / `GrowthEvent`
   — 各 tick のスナップショット用内部型

**関数:**

1. `run_replay_scenario(scenario: &VillageReplayScenario) -> ReplayTrace`
   - 全 tick を順次実行し、各 tick の村状態を記録して ReplayTrace を返す
   - 内部では FakeEventBus を使用し、決定論的 PRNG（StdRng::from_seed）で動作
   - 各 tick の処理:
     a. ワークフロー位置更新（dummy: 微小ランダムウォークを PRNG で生成）
     b. `classify_maturity` で成熟度再判定
     c. `filter_adult_candidates` + `build_local_village_topk` で村再構成（child ごと）
     d. `select_helpers` で helper weighting
     e. HELP プロトコルシミュレーション（既存の HelpSession を使用）
     f. 経験値増加・成熟イベントの記録

2. `trace_eq(left: &ReplayTrace, right: &ReplayTrace) -> bool`
   - 2 つの trace が完全一致するかを比較
   - 浮動小数点比較は許容誤差 `f64::EPSILON` で行う

3. `trace_diff_fields(left: &ReplayTrace, right: &ReplayTrace) -> Vec<String>`
   - 差分があるフィールド名のリストを返す（検証テスト T2 用）

4. `trace_summary_metrics(trace: &ReplayTrace) -> SummaryMetrics`
   - village_churn_p50/p95, helper_jsd_p50/p95, survival_rate, maturation_rate 等の要約統計量
   - 検証テスト T3 の summary metrics 範囲確認に使用

### 新規モジュール

- `src/replay.rs` — replay シナリオエンジン + trace データ型 + 比較器（単一ファイルに集約）

**公開 API への追加:**
- `Darvium` 構造体に `run_replay_scenario` メソッドを追加する
- `ReplayTrace`, `VillageReplayScenario` を公開型としてエクスポート

## Non-scope

- 実際の GED 計算（CheapGED / FullGED）の統合（後続チケット）
- 実際の EmbeddingProvider 結合（後続チケット）
- Training Orchestrator の実結合（M1.75-5 で完了済み）
- Reciprocity integration（M1.76 で実装予定）
- リプレイ結果の永続ストアへの保存（fixture 管理は M1.75-10 で扱う）
- 複数シナリオのバッチ実行・パラメータスイープ（M1.75-9/11 で扱う）

## Investigation

### 現在の実装状況（コードベース調査 2026-05-25）

**既存モジュール:**

1. **`src/village.rs`** — M1.75-2（#72）, M1.75-7（#78）で実装済み
   - `WorkflowMaturity { Child, Adult }` — 成熟度二値分類
   - `AdultCandidate` — フィルタリング中間表現
   - `LocalVillage { child_id, adult_ids, centroid, radius }` — 村構造
   - `classify_maturity` — 経験値・信頼・レピュテーションの3軸判定
   - `filter_adult_candidates` — ConsistencyState + maturity フィルタ
   - `build_local_village_topk` — TopK 近傍選抜（式 41B-6）
   - `build_local_village_radius` — 半径内選抜（式 41B-7）
   - `VillageMetrics` / `VillageMetricsWindow` / `VillageMetricsSnapshot` — メトリクスパイプライン
   - `compute_position_drift`, `compute_village_jaccard`, `compute_village_churn`, `compute_helper_jsd`, `compute_child_survival_rate`, `compute_child_maturation_time`
   - テスト: T-1〜T-20, T-E1（maturity, filter, topk, radius, centroid）, M1.75-7 T-1〜T-13, T-O1〜T-O3（メトリクス）

2. **`src/help.rs`** — M1.75-3（#74）, M1.75-4（#75）で実装済み
   - `HelpState { Proposal, Offered, Accepted, Rejected, Executing, Succeeded, Failed }` — 7状態
   - `HelpSession { help_id, from_workflow, to_workflow, current_state }` — セッション管理
   - `HelpProposal`, `HelpOffer`, `HelpDecision`, `HelpExecution`, `HelpSuccess`, `HelpFailure` — 構造体
   - `AdultHelpOfferPolicy` / `ChildHelpAcceptancePolicy` — ポリシー構造体
   - `is_legal_help_transition`, `transition_to_event`, `emit_help_event`
   - `should_offer_help`（式 41B-10）, `child_need_score`（式 41B-12）, `decide_help_offer`（式 41B-13）
   - テスト: T-1〜T-10, T-O1〜T-O3, M1.75-4 T-1〜T-8, T-O1〜T-O3
   - テスト総数: 約 30 テスト（全 PASS 確認済み）

3. **`src/childsupport.rs`** — M1.75-5（#76）, M1.75-6（#77）で実装済み
   - `ChildSupportMissionPayload` — mission 特殊化情報
   - `HelperWeight { helper_id, weight, is_remote }` — 重み付き helper
   - `HelperSelectionPolicy { beta, trust_exponent, reputation_exponent, epsilon, top_k }` — 選定ポリシー
   - `compute_helper_weights`（式 41B-18）
   - `mix_with_remote_exploration`（式 41B-19）
   - `select_helpers` — フィルタ→重み計算→混合→TOP-K 選抜
   - `spawn_child_support_mission` — mission 発行
   - テスト: T-1〜T-10, T-E1, T-E2, T-O1, T-O2 + M1.75-6 T-1〜T-8, T-O1, T-O2, T-E1（全 PASS 確認済み）

4. **`src/event.rs`** — M1.5-R4〜R11（#69〜#71）で実装済み
   - `FakeEventBus` — メモリ内イベントストア、publish/replay 対応
   - `EventFilter` — フィルタリング（kind_filter, since_vt, until_vt）

5. **`src/spaceposition.rs`** — M1.75-1（#72）で実装済み
   - `SpacePositionEmbedding` / `VillagePosition` / `l2_distance`

6. **`src/types.rs`** — 基底型定義
   - `WorkflowGraphId`（String）, `ConsistencyStateTag`, `TrainingMissionKind`

**不足しているもの（本チケットで実装が必要）:**

1. **`VillageReplayScenario`** — シナリオ定義型が存在しない
2. **`ReplayTrace`** — trace 格納型が存在しない
3. **`run_replay_scenario`** — シナリオ実行関数が存在しない
4. **trace 比較器** — `trace_eq` / `trace_diff_fields` が存在しない
5. **summary metrics** — trace から要約統計量を計算する関数が存在しない

### 参照観察レポート

- `tickets/context/0078-m175-7-village-stability-dynamicity/observation-20260524-164848.md` — 村安定性メトリクスの較正結果
- `tickets/context/0077-m175-6-helper-weightingbounded-remote-exploration-helper/observation-20260524-154834.md` — helper weighting のβ-εグリッド掃引結果
- `tickets/context/0080-m-05-7-r-retrieve-top-level-candidates-workflowcache-repositorypair-v23-j/observation-20260525-144228.md` — 4層検索パイプライン計装の実装パターン（参考）

## Test Plan

### ユニットテスト（同一ファイル内 `#[cfg(test)] mod tests`）

**T-1: 同一シナリオ 2 回実行で trace 完全一致**
- `basic_scenario` を作成し、2 回 `run_replay_scenario` を実行
- `trace_eq(trace1, trace2)` が `true` であることを `assert!`
- 確認項目: `space_positions`, `villages`, `helper_weights`, `help_sessions`, `child_growth_events` の全フィールド

**T-2: policy bundle 変更で差分が期待フィールドのみに現れる**
- `basic_scenario` をベースに `policy_bundle.offer_policy.threshold` のみを変更したシナリオを作成
- 両 trace の `help_sessions` フィールドに差分があることを確認
- `space_positions` と `child_growth_events` に差分がないことを確認（ポリシー変更前の決定論的部分）
- `trace_diff_fields` が期待するフィールド名のみを含むことを確認

**T-3: seed 変更で個別履歴は変動するが summary metrics が範囲内**
- `basic_scenario` の seed を変更したシナリオを実行
- `trace_eq(original, reseeded)` が `false` であることを確認（履歴が異なる）
- `trace_summary_metrics(reseeded)` の値が許容範囲内であることを確認
  - `village_churn_p50` < 0.5
  - `helper_jsd_p50` < 0.3
  - `child_survival_rate` > 0.5

**T-4: 空シナリオ（workflows が空）でのエッジケース**
- 空の workflows を持つシナリオを実行
- `ReplayTrace` が空のベクタを持つことを確認
- panic が発生しないことを確認

**T-5: single tick シナリオ**
- `clock_schedule.total_ticks = 1` のシナリオを実行
- trace に 1 件の tick データのみ含まれることを確認

**T-6: 全 Child（経験値不足）のみの村**
- 全ワークフローが Child であるシナリオを実行
- すべての village が空であることを確認（Adult 不在のため）
- `help_sessions` が空であることを確認

**T-7: 全 Adult のみの村**
- 全ワークフローが Adult であるシナリオを実行
- village が構成されることを確認
- HELP 提案が発生することを確認（policy 次第）

**T-8: trace_eq 浮動小数点許容誤差**
- 浮動小数点値を極小量（`f64::EPSILON`）だけ変動させた trace を作成し比較
- `trace_eq` が許容誤差内の変動を等値と判定することを確認

**T-9: trace_diff_fields 正常動作**
- 異なる 2 つの trace に対して `trace_diff_fields` を実行
- 差分があるフィールドが検出されることを確認
- 同一 trace に対して空リストが返ることを確認

### 観測テスト

**T-O1: メトリクスグリッド掃引（n >= 1,000）**
- seed 3 種 × total_ticks 3 種（5, 10, 20）× ワークフロー数 2 種（5, 10）の組み合わせ（18 条件）でシナリオを実行
- 各条件の summary metrics（village_churn_p50/p95, helper_jsd_p50/p95, survival_rate, helper_count_mean）を出力
- `println!` で構造化 CSV として表示し、計装データを記録

**T-O2: 決定論的再現性の統計的確認（n = 100）**
- 100 通りの seed で各シナリオを 2 回実行
- 全ケースで `trace_eq(trace1, trace2) == true` であることを確認
- 成功率 100% を観測

## 計装方法・観測対象

### 計装方法

- テストコード内で `println!` + `--nocapture` による構造化出力
- 固定シード `StdRng::seed_from_u64(12345)` を使用した決定論的 PRNG
- `FakeEventBus` によるイベント記録（既存の publish/replay 機構を流用）
- シナリオ実行ごとに `ReplayTrace` を JSON 文字列として `println!` 出力可能にする（`#[derive(Serialize)]` で対応）

### 観測対象

- **trace 完全一致率**: 同一 seed での 2 回実行間の一致率（T-O2 で 100% を期待）
- **summary metrics 分布**: seed 変動時の churn/JSD/survival rate の範囲
- **政策変更影響**: policy bundle 変更による diff field の特定（「どのフィールドが変わるか」が予測可能であることの確認）
- **村構成の多様性**: 異なる seed で minimum 1 件以上の HELP session が発生すること

### 較正計画

本チケットでは新規定数の導入は最小限とする。ただし replay シナリオ内で使用する疑似位置更新のランダムウォーク標準偏差を `constants.rs` に新規追加する可能性がある：

```rust
/// replay シナリオ内位置更新のランダムウォーク標準偏差 (Calibration Candidate)
/// Default: 0.1, 感度分析推奨範囲: 0.01-1.0
pub const REPLAY_POSITION_DELTA_SIGMA: f64 = 0.1;
```

目的関数 J(θ) は本チケットでは導入せず、M1.75-11 の較正ハーネスに委ねる。

## Boy Scout Rule — 翻訳可能性計画

### 新規コード `src/replay.rs`

- **関数は動詞句**: `run_replay_scenario`, `trace_eq`, `trace_diff_fields`, `trace_summary_metrics` — すべて動詞で開始
- **変数名はドメイン概念**: `scenario`, `trace`, `tick_positions`, `village_snapshot`, `help_session_trace` — `x`, `data`, `tmp` 不使用
- **一関数一責務**: `run_replay_scenario` はシナリオ実行のみ、比較器は別関数、metrics 計算は別関数
- **ハードコード値は定数化**: ランダムウォーク標準偏差は `REPLAY_POSITION_DELTA_SIGMA` で定数化
- **エラー握りつぶし禁止**: `unwrap()` 不使用、`Result` 伝播またはパニックメッセージ付き `expect()`

### 既存コードの改善

本チケットの範囲内では既存コードへの大規模な変更は行わない。ただし、`run_replay_scenario` が依存する既存関数のインターフェースが不自然な場合は、必要最小限の調整を行う。

## Acceptance Criteria

- [ ] `VillageReplayScenario` 型が定義され、seed / workflows / missions / clock_schedule / policy_bundle を持つ
- [ ] `ReplayTrace` 型が定義され、space_positions / villages / helper_weights / help_sessions / child_growth_events を持つ
- [ ] `run_replay_scenario(scenario) -> ReplayTrace` が実装され、全 tick の状態を記録する
- [ ] `trace_eq(left, right) -> bool` が実装され、浮動小数点許容誤差付きで完全一致比較できる
- [ ] `trace_diff_fields(left, right) -> Vec<String>` が実装され、差分フィールドを報告する
- [ ] `trace_summary_metrics(trace) -> SummaryMetrics` が実装される
- [ ] T-1〜T-9 の全ユニットテストが PASS
- [ ] T-O1, T-O2 の観測テストが PASS（構造化出力を含む）
- [ ] `cargo test` が既存テストも含めて全 PASS
- [ ] RFC §41B replay discipline との無矛盾確認完了

## Notes

- plan_path: /plan-ticket が plan.md を作成後に frontmatter に更新する
- implementation_path: /start-ticket が implementation.md を作成後に frontmatter に更新する
- review_report_path: /review-ticket が review.md を作成後に frontmatter に更新する
- observation_report_path: /start-ticket が observation-YYYYMMDD-HHmmss.md を作成後に frontmatter に最新パスを更新する

### 成果物

- 計画: context/0081-m175-8-deterministic-replay-village-help/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0081-m175-8-deterministic-replay-village-help/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0081-m175-8-deterministic-replay-village-help/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0081-m175-8-deterministic-replay-village-help/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
