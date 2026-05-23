# 実装サマリー: M1.5-3 起動時修復スキャン（Repair Worker）

## 変更したファイル

| ファイル | 変更内容 |
|---------|---------|
| src/store/coordinator.rs | `RepairScanSummary` 構造体追加、`startup_repair_scan()` メソッド実装、T51-T65 不変条件テスト + OTS-1〜OTS-3 観測テスト追加 |
| src/constants.rs | `REPAIR_SCAN_MAX_RETRY=3`（Safety Invariant）、`REPAIR_SCAN_BATCH_SIZE=100`（Calibration Candidate）を追加 |
| src/types.rs | `RankedCandidate` に `#[derive(Default)]` を追加（テストコードで必要なため） |
| src/store/mod.rs | `RepairScanSummary` を `pub use` に追加 |
| src/lib.rs | `RepairScanSummary` を crate の公開 API として再エクスポート |

## 実装内容

### RepairScanSummary
- `total_scanned`, `found_inconsistent`, `repaired`, `quarantined`, `already_clean`, `duration_ms` の 6 フィールドを持つ構造体

### startup_repair_scan()
- 全 consistency_state をスナップショット走査
- Committed → スキップ
- Pending → NeedsRepair に遷移後、apply_repair() で修復試行
- NeedsRepair → apply_repair() で修復試行
- apply_repair() 失敗 → Quarantined に遷移
- 修復成功 → Committed に遷移

### テスト網羅
- T51-T55: 正常系（全Committed/全Pending/全NeedsRepair/混合/空）
- T56-T58: FailingStore エラー注入（100%修復失敗/100%部分失敗）
- T59-T60: 冪等性・大規模
- T61-T65: 検索除外ゲート（スキャン前後・Quarantined維持・filter不変性）
- OTS-1: 10,000アンサンブル修復成功率 100%
- OTS-2: 減衰曲線（30% FailingStore でも初回スキャンで即収束）
- OTS-3: 吸収分布（repaired=4,886 / quarantined=114, terminal_inconsistent=0）
