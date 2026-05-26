# 変更したファイル一覧と実装内容の概要

## src/constants.rs
- `EVENTBUS_METRICS_WINDOW_SIZE: usize = 100` 追加（M1.76-19 定数ブロック内）

## src/event.rs

### EventBusMetrics 構造体（9 フィールド + 3 補助メソッド）
- `total_published`, `total_clock_advances`, `two_way_opened`, `two_way_resolved`, `two_way_aborted`, `two_way_timeout`, `quarantine_count`, `replay_count`, `subscribe_count`
- 補助メソッド: `two_way_resolution_rate()`, `quarantine_ratio()`, `event_throughput_per_clock_tick()` — 全てゼロ除算安全

### EventBusMetricsObserver
- `observe(bus: &FakeEventBus) -> EventBusMetrics` — FakeEventBus の metrics をスナップショット
- `print_csv(series: &[EventBusMetrics], prefix: &str)` — CSV 時系列出力（--nocapture 用）

### FakeEventBus の変更
- `metrics: Arc<Mutex<EventBusMetrics>>` フィールド追加（new() で初期化）
- 全 7 メソッドに metrics hook 追加（各操作ごとに該当カウンタ + clock advance を更新）
- `reset()` で metrics もリセット
- new() の clock 初期値リテラル 0 に RFC §A.x EVENTBUS_CLOCK_INITIAL 参照コメント追加

### テスト（8 件）
- T1: publish 100 回 → total_published == 100 ✅
- T2: open 50 + resolve 50 → カウンタ一致 ✅
- T3: quarantine → カウンタ増加 ✅
- T4: replay 5 + subscribe 5 → カウンタ一致 ✅
- T5: 計装有無で publish 結果不変（透過性） ✅
- O1: n=1000 ランダム操作 + CSV 出力 + カウンタ一致性 ✅
- O2: n=500 全解決後 resolution_rate/quarantine_ratio 検証 ✅
- O3: 初期状態 metrics 全 0 ✅

