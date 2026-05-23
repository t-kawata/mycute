# 計画: M1.5-3 起動時修復スキャン（Repair Worker）

## 要件
- `DualStoreCoordinator::startup_repair_scan()` — 全 asset 走査、Pending/NeedsRepair 検出、修復 or 隔離
- `RepairScanSummary` 戻り値構造体
- 修復中も is_eligible_for_retrieval() による hard exclusion 維持
- 定数: REPAIR_SCAN_MAX_RETRY=3, REPAIR_SCAN_BATCH_SIZE=100

## 変更ファイル
| ファイル | 種別 | 内容 |
|---|---|---|
| src/store/coordinator.rs | 実装 | RepairScanSummary + startup_repair_scan() + T1-T15 + OTS-1〜OTS-3 |
| src/constants.rs | 定数追加 | REPAIR_SCAN_MAX_RETRY, REPAIR_SCAN_BATCH_SIZE |

## 計装・観測
- 不変条件テスト T1-T15: assert! / assert_eq! による決定論的検証
- 観測テスト OTS-1〜OTS-3: println! + --nocapture + StdRng 固定シード
- N=10,000 損傷アンサンブル、ln||E(t)|| ~ -Γt 指数減衰観測

## 実装手順
1. constants.rs に REPAIR_SCAN_MAX_RETRY と REPAIR_SCAN_BATCH_SIZE 追加
2. coordinator.rs に RepairScanSummary 構造体追加
3. coordinator.rs に startup_repair_scan() メソッド実装
4. coordinator.rs に T1-T15 + OTS-1〜OTS-3 テスト追加
5. cargo test + cargo clippy 検証
6. 観察レポート保存

## 検証
- cargo test -- --nocapture (全テスト PASS)
- cargo clippy -- -D warnings (新規警告ゼロ)
- 観測テスト出力の構造化確認 (println! 出力)

## リスク
- apply_repair の状態不整合 → set_consistency_state で明示的遷移
- N=10,000 パフォーマンス → 単発実行で CI 影響軽微
