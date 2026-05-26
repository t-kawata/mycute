---
ticket_id: 98
title: M1.76-13: 決定論的リプレイテスト（MUST replay test）
slug: m176-13-must-replay-test
status: reviewed
created_at: 2026-05-26
updated_at: 2026-05-26
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0098-m176-13-must-replay-test/implementation.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0098-m176-13-must-replay-test/observation-20260526-110021.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0098-m176-13-must-replay-test/review.md
---
# M1.76-13: 決定論的リプレイテスト（MUST replay test）

## Summary

同一 event stream（DarviumEvent 列）、同一 policy version、同一 EventBus clock のもとで、ReputationProfile と GC hazard の再計算結果が常にビットレベルで一致することを検証する決定論的リプレイ機構を実装する。

## Background

M1.76-6（GC hazard with benevolence）から M1.76-12（単調性テスト）までの Reciprocity モジュールの全計算（F-1〜F-15）はすべて純粋関数として実装されており、外部状態（乱数・IO・時刻）に依存しない。しかし、これらを **合成したパイプライン（イベントインジェスション → 再計算）** が全体として決定論的であることは自明ではない。

理由：
- `compute_replay_comparison` は既に実装されているが、**リプレイシナリオの定義と実行エンジン**が存在しない
- 複数グラフの profile/hazard を跨ぐ順序依存性の有無が未検証
- policy version 変更や clock 進行による差分が期待するフィールドにのみ限定されることの確認が必要
- golden trace による回帰検出機構がない

本チケットは RFC §41B.20.8 Testing discipline「Replay test (MUST)」、v2.3-g §12C DarviumEventBus replay に対応する。RFC §41C.3 の **M1.x** に位置づけられる。

## Scope

1. **`ReciprocityReplayScenario` 構造体の定義**: 再現可能なリプレイシナリオ
2. **`run_reciprocity_replay` 関数の実装**: シナリオを受け取り trace を生成する純粋関数
3. **`ReciprocityReplayTrace` 構造体の定義**: リプレイ結果（profiles, hazards, snapshots, trace_hash）
4. **`ReplayTraceComparator::assert_bitwise_eq` の実装**: 2 つの trace のビットレベル一致検証
5. **Golden trace 保存機構**: trace_hash による回帰検出
6. **観測計装**: スナップショット間の差分ノルムの時間発展記録、n=100 回の独立実行による最大差分量=0 の検定

## Non-scope

- EventBus そのものの実装変更（既存のイベント型 `DarviumEvent` / `DarviumEventKind` をそのまま利用）
- VirtualClock そのものの実装変更（既存の clock 進め方をそのまま利用）
- M1.76-11 で既に実装済みの `ReciprocityReplaySnapshot` / `ReciprocityDiffReport` / `compute_replay_comparison` の再設計
- M1.76-12 で実装済みの単調性テストスイートとの重複
- プロダクションコードへのリプレイ機構の組み込み（テスト専用ユーティリティとして実装）

## Investigation

### 参照観察レポート

- `tickets/context/0097-m176-12-must-monotonicity-tests/observation-20260526-104115.md` — M1.76-12 単調性テスト完了。「全 MUST 単調性条件が現行パラメータで成立しているため、M1.76-13（決定論的リプレイテスト）の前提条件は満たされている」と結論。

### 既存コード調査結果

**既に存在する型・関数（M1.76-11 で実装済み）:**

- `src/reciprocity.rs:793` — `ReciprocityReplaySnapshot`:
  ```rust
  pub struct ReciprocityReplaySnapshot {
      pub profiles: HashMap<WorkflowGraphId, ReputationProfile>,
      pub hazards: HashMap<WorkflowGraphId, f32>,
      pub policy_version: String,
      pub clock: u64,
  }
  ```

- `src/reciprocity.rs:808` — `ReciprocityDiffReport`:
  profiles/hazards の差分を (before, after) タプルで記録。

- `src/reciprocity.rs:837` — `compute_replay_comparison`:
  2 つの `ReciprocityReplaySnapshot` を比較し `ReciprocityDiffReport` を返す。

**未実装（本チケットで作成するもの）:**

- `ReciprocityReplayScenario` — event_stream, policy, clock_schedule, initial_profiles を保持
- `ReciprocityReplayTrace` — 実行結果のプロファイル群、ハザード群、スナップショット列、trace_hash
- `run_reciprocity_replay` — シナリオを逐次実行し trace を生成
- `ReplayTraceComparator` — `assert_bitwise_eq` で 2 つの trace の完全一致を検証
- Golden trace 保存機構 — trace_hash を生成し回帰テスト種として保存

**テストパターン（既存の test_direct_score_survival_monotonicity 等の観測テストパターンを踏襲）:**

- `StdRng::seed_from_u64(12345)` 固定シード
- `println!` + `--nocapture` で観測出力

### 依存関係の確認

- `ReputationProfile`（event.rs:403）: final_score, direct_score, indirect_score 等の全フィールド
- `DarviumEvent`（event.rs:754）: event_id, kind, payload 等
- `ReciprocityEventKind`（event.rs:735）: 互恵性イベント種別
- `ReciprocityLifecyclePolicy`（event.rs 内）: ポリシーパラメータ
- `recompute_reputation`（reciprocity.rs:225）: F-4/F-5
- `compute_gc_hazard`（reciprocity.rs:290）: F-7/F-8

## Test Plan

### テスト 1: 完全同一シナリオのビットレベル一致

同一の `ReciprocityReplayScenario` を 2 回実行し、`ReplayTraceComparator::assert_bitwise_eq` で全フィールドの一致を確認。n=10 回の独立実行を繰り返す（固定シードのため 10 回とも一致する必要がある）。

- 入力: 3〜5 件の WorkflowGraphId を含む初期プロファイル群、10〜20 件の Reciprocity イベント列、デフォルトポリシー、一定間隔の clock_schedule
- 期待結果: 全 10 回の実行で trace が完全一致。assert_bitwise_eq が panic しない。
- 観測出力: 各実行の trace_hash を表示、全 hash が同一であることの確認。

### テスト 2: policy version 変更による限定差分

policy version のみ変更したシナリオ（同一イベント列、同一 clock_schedule）で、trace の `policy_version` フィールドのみ異なり、profiles/hazards の値は一致することを確認。

- 入力: テスト 1 と同一のイベント列、policy.version = "v1" と "v2"
- 期待結果: `before.policy_version != after.policy_version`。profile_diffs と hazard_diffs は空（ただし policy の数値パラメータが同一のため再計算結果は一致）。
- 注意: 実際に RECIPROCITY_LIFECYCLE_DECAY 等の定数値が変わっている policy を渡した場合、profile/hazard 値に差分が生じうる。その場合は profile_diffs/hazard_diffs が空でないことを確認し、changed_graph_ids が空でないことを検証する。

### テスト 3: VirtualClock 進行スケジュール変更による限定差分

clock_schedule のみ変更した場合、**時刻依存項（時間減衰 `exp(-ρ Δt)`）のみ**に差分が現れることを確認。profiles/hazards の最終値は同一タイミングで計測すれば一致する。

- 入力: テスト 1 と同一のイベント列と初期プロファイル。clock_schedule = [100, 200, 300] と [100, 150, 300]
- 期待結果: 中間スナップショットの差分は時刻依存のため許容。最終スナップショットの profiles/hazards は一致。

### テスト 4: イベント順序維持の再現性

イベント列の順序を維持したまま再実行した場合、完全に一致することを確認（テスト 1 の変形）。

- 期待結果: テスト 1 と同一

### テスト 5: n=100 独立実行による最大差分量 0 の検定

固定シードを用い n=100 回の独立リプレイにおける最大差分量を計測。全 trace_hash が同一であることを検証する観測テスト。

- 計装: 各回の trace_hash を表示。全 hash の一致を確認。
- 期待結果: 全 100 回の trace_hash が同一。

### テスト 6: Golden trace 保存と回帰検出

初回実行の trace_hash を golden hash として保存。2 回目実行で同一 hash を返すことを確認。コード変更後に hash が変化した場合（回帰）、警告を出力する。

## 計装方法・観測対象

### 計装方法

- テストコードは `src/reciprocity.rs` の `mod tests` 内に実装（既存の M1.76-3〜M1.76-12 テストと同様）
- 固定シード PRNG は `StdRng::seed_from_u64(12345)` を使用
- `println!` + `--nocapture` で以下の観測データを標準出力に書き出す:
  - 各実行の trace_hash
  - スナップショット間の差分ノルム `||trace_A(t) - trace_B(t)||`
  - n=100 回の全 hash 一致判定結果

### 観測対象

- **trace_hash**: `ReciprocityReplayTrace` に付与される文字列ハッシュ（全フィールドを serde_json で直列化し SHA-256 ハッシュ）
- **最大差分量**: n=100 回の独立実行における最大 trace_hash 不一致数。すべて 0 であることを確認。
- **スナップショット差分ノルム**: 各 clock 刻みにおける profile/hazard の L2 ノルム差分の時間発展。

### 較正計画

本チケットは較正を伴わない（constants.rs の変更は行わない）。ただし、以下を観測する：
- n=100 回のリプレイがすべて同一結果を返すことによる決定論的再現性の立証

## Boy Scout Rule — 翻訳可能性計画

本チケットで触るコードは以下の方針で実装する：

1. **関数名は動詞句**: `run_reciprocity_replay`, `assert_bitwise_eq`, `compute_trace_hash` のように処理内容を動詞で始める。
2. **一関数一責務**: リプレイ実行、差分比較、hash 計算は個別の関数に分離。
3. **ハードコード値の定数化**: テスト内のマジックナンバー（イベント数 20、seed 値等）は名前付き定数として定義。
4. **エラー握りつぶし禁止**: `assert_bitwise_eq` は差分がある場合に panic する明確なエラーメッセージを出力（`assert_eq!` で十分）。

既存の `compute_replay_comparison` 関数（reciprocity.rs:837）の読みやすさはそのまま維持する。新規追加コードは既存の命名パターンに従う。

## Acceptance Criteria

- [ ] `ReciprocityReplayScenario` / `ReciprocityReplayTrace` / `ReplayTraceComparator` が実装され、テストがパスする
- [ ] 完全同一シナリオの 2 回実行でビットレベル一致（テスト 1）
- [ ] policy version 変更で限定差分（テスト 2）
- [ ] clock_schedule 変更で時刻依存項のみ差分（テスト 3）
- [ ] イベント順序維持の再実行で完全一致（テスト 4）
- [ ] n=100 回の独立実行で最大差分量 0（テスト 5）
- [ ] Golden trace 保存機構（テスト 6）
- [ ] 既存テスト（M1.76-1〜12）がすべて通過すること
- [ ] RFC 該当セクションとの無矛盾確認
- [ ] 翻訳可能性計画に沿ったコード記述

## Notes

- plan_path: {{plan 作成後にセット}}
- implementation_path: {{実装後にセット}}
- review_report_path: {{レビュー後にセット}}
- observation_report_path: {{観測レポート作成後にセット}}

### 成果物

- 計画: context/0098-m176-13-must-replay-test/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0098-m176-13-must-replay-test/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0098-m176-13-must-replay-test/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0098-m176-13-must-replay-test/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
