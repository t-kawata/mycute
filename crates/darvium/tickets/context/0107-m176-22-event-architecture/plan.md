# 実装計画: M1.76-22 Event Architecture 運用メトリクス観測パイプライン統合

## RFC 既存実装状態検証

### RFC §12C.5 DarviumEventBus Trait

| 項目 | RFC 仕様 | 現行コード (event.rs L901) | 状態 |
|---|---|---|---|
| トレイト名 | DarviumEventBus | DarviumEventBus | ✅ 一致 |
| Super-trait | Send + Sync | VirtualClock + Send + Sync | ⚠️ 余剰 (VirtualClock は §12C.6 要件) |
| publish | async fn(&self, DarviumEventKind, Value) -> Result<EventId> | fn(&self, DarviumEvent) -> Result<EventId> | ⚠️ 型不一致 (sync 化 + DarviumEvent 全体) |
| subscribe | async fn(&self, &[DarviumEventKind]) -> Result<Subscription> | fn(&self, EventFilter) -> Box<dyn EventSubscription> | ⚠️ 型不一致 (EventFilter 拡張) |
| quarantine_failed_events | async fn(&self) -> Result<Vec<DarviumEvent>> | fn(&self, &InteractionId, &str) -> Result<()> | ⚠️ 型不一致 (引数追加) |

**評価**: DarviumEventBus はテスト容易性のため意図的に簡略化されており、RFC との乖離は既知。本チケットでは新規フィールド追加のみを行い、トレイトシグニチャは変更しない。

### RFC §12C.10 FakeEventBus

| フィールド | RFC | 現行コード | 状態 |
|---|---|---|---|
| events | Arc<Mutex<Vec<DarviumEvent>>> | 同一 | ✅ 一致 |
| clock | Arc<Mutex<u64>> | 同一 | ✅ 一致 |
| interactions | Arc<Mutex<HashMap<String, InteractionStatus>>> | `Arc<Mutex<HashMap<String, InteractionRecord<JsonInteractionPayload>>>>` | ⚠️ 型不一致 |
| metrics | (未定義) | (欠落) | ➕ 本チケットで追加 |

### RFC A.x v2.3-g 追加定数 (7件)

全文未実装。本チケットで必要なものはないが reference として記録:
EVENTBUS_CLOCK_INITIAL(0), EVENTBUS_MAX_RECONNECT_RETRIES(3), EVENTBUS_SUBSCRIPTION_MAX_KINDS(32), EVENTBUS_REPLAY_BATCH_SIZE(100), EVENTBUS_CHANNEL_RECONNECT_BASE_DELAY_MS(1000), EVENTBUS_CHANNEL_RECONNECT_MAX_DELAY_MS(30000), EVENTBUS_PROJECTION_ERROR_BACKOFF_MS(5000) — 全て未定義。

## 要件の再確認

v2.3-g §12C Event Architecture の運用メトリクスを計測・観測可能にする。M1.76-18 の `ReciprocityMetricsObserver` / `ExtendedOperationalMetrics` パターンを模倣し、`EventBusMetrics` + `EventBusMetricsObserver` を実装する。

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|---|---|---|
| `src/event.rs` | 変更 | EventBusMetrics 構造体、FakeEventBus metrics フィールド + hook、EventBusMetricsObserver、テスト T1-T5 + O1-O3 |
| `src/constants.rs` | 変更 | EVENTBUS_METRICS_WINDOW_SIZE 追加 |

## 計装・観測の実装計画

### 実装コード

全コードを `src/event.rs` に追加。

1. **EventBusMetrics 構造体** (9 フィールド + 補助メソッド)
   - `total_published`, `total_clock_advances`, `two_way_opened`, `two_way_resolved`, `two_way_aborted`, `two_way_timeout`, `quarantine_count`, `replay_count`, `subscribe_count`
   - `two_way_resolution_rate()`, `quarantine_ratio()`, `event_throughput_per_clock_tick()`

2. **FakeEventBus への metrics フィールド追加**
   - 全メソッド先頭でカウンタ更新
   - reset() で metrics もリセット

3. **EventBusMetricsObserver**
   - `observe(bus: &FakeEventBus) -> EventBusMetrics`
   - `print_csv(series: &[EventBusMetrics], prefix: &str)`

4. **テストコード**
   - T1: publish 100 回 → total_published == 100
   - T2: open 50 + resolve 50 でカウンタ一致
   - T3: quarantine 後カウンタ増加
   - T4: replay 5 + subscribe 5 でカウンタ一致
   - T5: 計装有無で publish 結果不変（透過性）
   - O1: n=1000 ランダム操作 + CSV 出力 + カウンタ一致性
   - O2: n=500 全 open 解決後 resolution_rate == 1.0
   - O3: 初期状態 metrics 全 0

### 観測出力取得

```bash
cargo test --package darvium --lib event::tests::o1_random_operations_n1000 -- --nocapture
cargo test --package darvium --lib event::tests::o2_two_way_full_resolve_rate -- --nocapture
```

### 較正対象定数

| 定数 | 既定値 | 分類 |
|---|---|---|
| EVENTBUS_METRICS_WINDOW_SIZE | 100 | Calibration Candidate |

## Boy Scout 改善

- FakeEventBus::new() の clock 初期値リテラル 0 に RFC §A.x EVENTBUS_CLOCK_INITIAL への参照コメントを追加
- 新規 `expect()` メッセージは既存の日本語形式に統一

## 実装手順

1. src/constants.rs に EVENTBUS_METRICS_WINDOW_SIZE 追加
2. src/event.rs に EventBusMetrics 構造体追加
3. FakeEventBus に metrics フィールド追加 + 各メソッド hook
4. EventBusMetricsObserver 実装
5. テスト T1-T5 追加
6. 観測テスト O1-O3 追加
7. cargo test 全通過確認
8. cargo clippy 通過確認

## 物理的レビュー方法

1. `run-quality-checks.js` で spec 受入基準充足チェック
2. 翻訳可能性 grep: 名詞始まり関数、ゼロ除算安全性確認
3. `cargo test` 全通過
4. `cargo clippy -- -D warnings` 通過

## リスク

- Low: カウンタ不一致 → T1-T4 で全網羅
- Low: 透過性違反 → T5 で検証
- None: 既存テスト影響なし
