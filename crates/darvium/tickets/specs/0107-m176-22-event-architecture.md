---
ticket_id: 107
title: M1.76-22: Event Architecture 運用メトリクス観測パイプライン統合
slug: m176-22-event-architecture
status: reviewed
created_at: 2026-05-26
updated_at: 2026-05-26
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0107-m176-22-event-architecture/plan.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0107-m176-22-event-architecture/implementation.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0107-m176-22-event-architecture/observation-20260526-144721.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0107-m176-22-event-architecture/review.md
---

# M1.76-22: Event Architecture 運用メトリクス観測パイプライン統合

## Summary

M1.76-18 の運用メトリクス観測パイプラインと同様のアプローチで、v2.3-g §12C Event Architecture の運用メトリクス（EventBus スループット、TwoWay 解決率、quarantine 率等）を計測・観測可能にする。`EventBusMetrics` 構造体で 9 指標を収集し、`EventBusMetricsObserver` で既存観測パイプラインと統合する。

## Background

- M1.76-18 で `ExtendedOperationalMetrics` + `ReciprocityMetricsObserver` による運用メトリクス観測パイプラインが構築済み
- M1.76-21 で `EventSubscriber` + `SubscriberManager` による外部イベント購読基盤が完了
- しかし `FakeEventBus` には未だメトリクス収集機構が存在せず、EventBus 操作（publish/open/resolve/abort/quarantine 等）の回数・解決率が観測不可能
- v2.3-g §12C では EventBus の運用メトリクス（スループット、クロック単調増加性、TwoWay 解決率等）の監視が MUST 要件として規定されている

## Investigation

### 参照観察レポート

- `tickets/context/0103-m176-18-additional-operational-metrics/observation-20260526-123144.md` — `ReciprocityMetricsObserver` / `ExtendedOperationalMetrics` 実装パターン（observer hook + CSV 時系列出力器 + 観測テスト 10 件）
- `tickets/context/0106-m176-21-eventsubscriber-websocketeventchannel/observation-20260526-143420.md` — EventSubscriber/SubscriberManager 完了実績（配送完全性 100%）。FakeExternalEventClient の固定シード PRNG 再現性確認済み

### 既存コード調査結果

- `src/event.rs` L941-1199: `FakeEventBus` 実装 — 計 9 メソッドを持つが、メトリクスカウンタは未実装
- `src/simulation.rs` L804-870: `ReciprocityMetricsObserver` — `observe()` + `print_csv()` の既存パターン。本チケットはこれを模倣
- `src/constants.rs`: EventBus 関連の定数は未定義（本チケットで初導入）

### FakeEventBus のメソッド一覧（計装 hook 設置対象）

| メソッド | 増加するメトリクスカウンタ |
|---|---|
| `publish()` | `total_published` |
| `open()` | `two_way_opened` |
| `resolve()` | `two_way_resolved` |
| `subscribe()` | `subscribe_count` |
| `replay()` | `replay_count` |
| `quarantine_failed_events()` | `quarantine_count` |
| 内部 clock advance | `total_clock_advances` |

## Scope

1. `EventBusMetrics` 構造体の定義（9 フィールド + 補助監視指標導出メソッド）
2. `FakeEventBus` へのメトリクス収集 hook 追加（各メソッド呼び出し時にカウンタ更新）
3. `EventBusMetricsObserver` 実装（observer hook + CSV 時系列出力器）
4. EventBus 関連較正定数の `constants.rs` 追加
5. 観測テスト 3 件 n=1000

## Non-scope

- `ConcreteEventBus`（MetadataStore 結合版）への計装
- TwoWay abort / timeout の新しい状態機械
- EventBus の論理動作変更
- DarviumEvent のペイロード変更

## 計装方法・観測対象

### 計装方法

- `FakeEventBus` 内に `metrics: Arc<Mutex<EventBusMetrics>>` フィールド追加
- 各メソッド先頭で該当カウンタを更新
- 固定シード PRNG（`StdRng::seed_from_u64(12345)`）でランダム操作系列を生成

### 観測対象

- 9 メトリクスカウンタの単調増加性
- `two_way_resolution_rate = two_way_resolved / two_way_opened`（全解決後 1.0）
- `quarantine_ratio = quarantine_count / two_way_opened`
- `event_throughput_per_clock_tick = total_published / total_clock_advances`
- メトリクス観測有無による透過性

## Test Plan

### ユニットテスト

| ID | 名称 | 内容 |
|---|---|---|
| T1 | `metrics_publish_count` | publish 100 回後に `total_published == 100` |
| T2 | `metrics_open_resolve_count` | open 50 回 + resolve 50 回後にカウンタ一致 |
| T3 | `metrics_quarantine_count` | quarantine 後にカウンタ増加 |
| T4 | `metrics_replay_subscribe_count` | replay 5 回 + subscribe 5 回後にカウンタ一致 |
| T5 | `metrics_transparency` | メトリクス観測あり/なしで動作不変 |

### 観測テスト

| ID | 名称 | n | 内容 |
|---|---|---|---|
| O1 | `random_operations_n1000` | 1000 | ランダム操作系列でカウンタ一致性 + CSV 出力 |
| O2 | `two_way_full_resolve_rate` | 500 | 全 open 解決後 resolution_rate == 1.0 |
| O3 | `empty_bus_all_zero` | — | 初期状態 metrics 全 0 確認 |

## Acceptance Criteria

- [ ] `EventBusMetrics` 構造体が 9 フィールドを持つ
- [ ] `FakeEventBus` の全メソッド呼び出しで該当カウンタが正確に増加
- [ ] `EventBusMetricsObserver` が実装されている
- [ ] 全テスト（T1-T5 + O1-O3）が通過
- [ ] メトリクス観測が EventBus の論理動作に影響しない（T5）
- [ ] 既存テストが全て通過

## Notes

### Boy Scout Rule — 翻訳可能性計画

- 各メソッドの責務を保ち、メトリクス更新は1行追加に留める
- 補助監視指標メソッドはゼロ除算を安全に処理する
- フィールド名はドメイン概念を直接表現する

### 成果物

- 計画: `context/0107-m176-22-event-architecture/plan.md`
- 実装サマリ: `context/0107-m176-22-event-architecture/implementation.md`
- レビュー報告書: `context/0107-m176-22-event-architecture/review.md`
- 観察レポート: `context/0107-m176-22-event-architecture/observation-YYYYMMDD-HHmmss.md`
