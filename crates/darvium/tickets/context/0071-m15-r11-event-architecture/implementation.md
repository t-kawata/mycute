# 変更したファイル一覧と実装内容の概要

## src/constants.rs

Event Architecture 定数11件を `// === Event Architecture (RFC §12C) ===` セクションとして追加:

| 定数名 | 値 | 分類 |
|--------|-----|------|
| EVENTBUS_CHANNEL_CAPACITY | 1024 | Safety Invariant |
| EVENTBUS_DEFAULT_TIMEOUT_MS | 5000 | Calibration Candidate |
| EVENTBUS_MAX_RECONNECT_RETRIES | 3 | Calibration Candidate |
| EVENTBUS_SUBSCRIPTION_MAX_KINDS | 32 | Calibration Candidate |
| EVENTBUS_REPLAY_BATCH_SIZE | 100 | Calibration Candidate |
| EVENTBUS_CHANNEL_RECONNECT_BASE_DELAY_MS | 1000 | Calibration Candidate |
| EVENTBUS_CHANNEL_RECONNECT_MAX_DELAY_MS | 30000 | Calibration Candidate |
| EVENTBUS_PROJECTION_ERROR_BACKOFF_MS | 5000 | Calibration Candidate |
| INTERACTION_CLEANUP_INTERVAL_TICKS | 100 | Calibration Candidate |
| PROJECTION_INITIAL_CAPACITY | 64 | Environment Policy Knob |
| QUARANTINE_MAX_EVENTS | 10000 | Safety Invariant |

RFC 優先ルールにより、チケット spec の定数名/値ではなく RFC §12C の定義を正本とした。
チケット spec の EVENT_BUS_ プレフィックス → RFC の EVENTBUS_ プレフィックスに統一。
EVENT_REPLAY_BATCH_SIZE (spec: 256) → EVENTBUS_REPLAY_BATCH_SIZE (RFC: 100) に修正。

## src/event.rs

### 追加したテスト (mod tests 内)

| テスト | 内容 | 種別 |
|-------|------|------|
| C-1〜C-7 | 定数値検証 (assert_eq!) | 不変条件テスト |
| P-1 | event_kind_strategy 全13 variant 生成 | proptest 戦略 (n=256) |
| P-2 | interaction_mode_strategy OneWay/TwoWay | proptest 戦略 (n=256) |
| P-3 | darvium_event_strategy 全フィールド生成 | proptest 戦略 (n=256) |
| P-4 | publish→replay 完全性（消失率 0%） | proptest 不変条件 (n=256) |
| P-5 | TwoWay 状態遷移有限ステップ完了 | proptest 不変条件 (n=256) |
| P-6 | clock 単調増加性 | proptest 不変条件 (n=256) |
| P-7 | replay clock 不変性 (MUST NOT #3) | proptest 不変条件 (n=256) |
| P-8 | quarantine 除外性 | proptest 不変条件 (n=256) |
| P-9 | projection 独立完全性 | proptest 不変条件 (n=256) |
| E-1〜E-3 | 極端値テスト | 境界値テスト |
| R11 | 計装サマリ出力 | 観測テスト |

### 追加した proptest 戦略関数

- event_kind_strategy() → 全13 variant を生成 (System/Search/WorkflowExecution/Training/Knowledge/Conversational/Lifecycle/Gc/Repair/Reciprocity/Fusion/Hitl/Extension)
- interaction_mode_strategy() → OneWay/TwoWay 生成
- darvium_event_strategy() → 全フィールド (event_id/kind/interaction_mode/payload/causality/metadata/transport_meta/visibility/retention/privacy) 生成
- サブ戦略: system_event_strategy, search_event_strategy, training_event_strategy, reciprocity_event_strategy

### 主な技術的決定

1. proptest の `#![proptest_config = ...]` 内属性構文は `#[cfg(test)] mod tests` 内でコンパイルエラーとなるため使用せず、デフォルト cases=256 の標準 `proptest! { #[test] fn ... }` 形式を採用
2. `super::CONSTANT` 参照は `crate::constants::CONSTANT` に変更（constants.rs は別モジュール）
3. P-8 の quarantine テストでは event_id 重複を避けるため通常イベントに `norm-` プレフィックスを付与
