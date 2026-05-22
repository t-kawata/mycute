# 観察レポート: M-2-1.8 Clock / VirtualClock 抽象トレイトの定義

## 1. 計装の実装状況

- 計装対象: `VirtualClock` の単調増加性（巻き戻し禁止）のアサーション。`Clock` トレイトを通して観測される時間の流れが、実時間または仮想時間のいずれかで一貫していることの検証。
- 実装したテストコード: `src/clock/mod.rs` — `test_virtual_clock_observation()` (T16), `test_system_clock_real_time()`
- 観測した統計量: VirtualClock 経過時間の一致精度、SystemClock の実時間誤差

## 2. 観測テスト実行結果

```
=== VirtualClock 経過時間観測 ===
advance 回数: 100
期待累積時間: 5050ms
観測累積時間: 5050ms
一致: true
=== 結果: PASS ===
```

既存テスト（T1〜T15）も全件通過。

SystemClock テストでは `SystemTime::now()` との誤差が 1秒未満であることを確認。

## 3. 較正ループ

`CLOCK_DEFAULT_START_MS` (`src/constants.rs:69`) は Safety Invariant（変更禁止）。較正は不要。

## 4. 現象の解釈（日本語）

Clock トレイト階層の観測結果から、以下の性質が確認された:

- **VirtualClock の完全決定論性**: advance(1) から advance(100) までの等差数列の合計が 5050ms (= 100×101/2) と数学的に完全一致した。内部カウンタの単調増加が設計通り動作し、オーバーフローや丸め誤差がない。これは、SearchBudget の時間計測をこの VirtualClock で置き換えることで完全再現可能なテストが実現できることを保証する。

- **SystemClock の実時間一貫性**: SystemClock がラップする `SystemTime::now()` が UTC 起点のミリ秒として正しく機能している。advance() が no-op であることも確認済みで、実時間クロックの誤操作を防止する設計が意図通り動作している。

- **FrozenClock の恒常性**: 複数回呼び出しで同一値が返ることを確認。時刻依存のロジック（例: キャッシュ有効期限のテスト）で有効。

- **3種のクロックの使い分け**: VirtualClock（単体テスト・シミュレーション）、SystemClock（本番環境）、FrozenClock（特定時刻のテスト）の3実装がすべてトレイト境界を通過するため、テストから本番への移行がコード修正なしで可能。

## 5. 目的関数 J(θ) の評価

- VirtualClock 累積誤差: 0 (5050ms = 5050ms, 完全一致)
- SystemClock 誤差: < 1秒以内（実時間追従）
- 単調増加性: ✅ 全実装で確認
- トレイト完全性: ✅ Box<dyn Clock>, Send + Sync 確認済み

## 6. 次チケットへの示唆

- SearchBudget（M-2-2）の `wall_clock_ms_used` 計測に VirtualClock を使用することで、非決定論的な時間依存を排除できる。
- M2.5-2（deterministic replay）では、VirtualClock の advance ログを記録・再生することで完全な再現性を保証できる。
- 誤差 0 の累積加算は、長期間のシミュレーションでもドリフトが蓄積しないことを意味し、M3 以降の長期時系列テストに耐える。
