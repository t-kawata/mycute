# 実装サマリー: チケット M-2-1.8 — Clock / VirtualClock 抽象トレイトの定義

## 変更ファイル

| ファイル | 種別 | 内容 |
|---|---|---|
| src/clock/mod.rs | 新規作成 | Clock トレイト + VirtualClock / SystemClock / FrozenClock + 全テスト (T1-T16) |
| src/lib.rs | 編集 | pub mod clock; 追加 (1行) |
| src/constants.rs | 編集 | CLOCK_DEFAULT_START_MS 追加 (1定数) |

## 実装内容

### Clock トレイト
- `fn now_ms(&self) -> u64` — UTC ミリ秒を返す (Send + Sync 境界, オブジェクト安全)
- `fn advance(&mut self, delta_ms: u64)` — VirtualClock のみ有効, 他は no-op

### VirtualClock
- 内部 u64 カウンタ, advance() で飽和加算, with_start() で任意開始時刻
- 完全決定論的 — テストでの deterministic replay を保証

### SystemClock
- SystemTime::now() → UNIX_EPOCH のラップ, advance() は no-op

### FrozenClock
- コンストラクタ指定の固定値を返す, advance() は no-op

## テスト結果
- 全 83 テスト通過 (既存 67 + 新規 16)
- cargo clippy -- -D warnings 通過
- 観測テスト出力: SystemClock 実時間検証, VirtualClock 経過時間観測
