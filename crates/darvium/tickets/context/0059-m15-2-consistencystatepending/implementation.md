# 変更したファイル一覧と実装内容の概要

## 変更ファイル

### src/store/coordinator.rs (新規作成)
DualStoreCoordinator — 異種ストア論理一貫性コミットプロトコルの調整役。
- `DualStoreCoordinator` struct: GraphStore + MetadataStore をラップし、RFC §18.2 の commit_dual_store_update プロトコルを実装
- `commit_dual_store_update()`: Pending → BlobPrepared → Committed の5段階コミットプロトコル、失敗時は NeedsRepair 遷移
- `is_eligible_for_retrieval()`: 検索対象適格判定 (Committed のみ許可)
- `filter_retrieval_eligible()`: 候補一括フィルタリング (Hard Retrieval Exclusion Gate)
- `apply_repair()`: NeedsRepair → 再書き込み → Committed/Quarantined の修復プロトコル
- `FailingGraphStore`: GraphStore ラッパー、確率的エラー注入 (固定シード PRNG)
- `FailingMetadataStore`: MetadataStore ラッパー、成功予算ベースのエラー注入
- T1-T48 不変条件テスト: 型検証 (T1-T10)、コミット正常系/異常系 (T11-T20)、検索除外ゲート (T21-T30)、エラー注入 (T31-T40)、並行アクセス (T41-T48)
- OTS-1〜OTS-4 観測テスト: P_taint 曲線、Δτ_unclean 強度、修復収束、吸収分布

### src/store/mod.rs (修正)
- `mod coordinator;` 追加
- `pub use coordinator::DualStoreCoordinator;` 追加

### src/types.rs (修正) — 前回セッション完了
- `ConsistencyState` enum: 4 バリアント (Committed/Pending/NeedsRepair/Quarantined)
- `CommitPhase` enum: 4 位相 (MetaPrepared/BlobPrepared/MetaCommitted/BlobCommitted)
- `RepairLog` struct: 5 フィールド
- `RepairAction` enum: 4 アクション
- `ConsistencyStateTag` enum: 4 タグ (状態識別用)
- `ConsistencyState::is_eligible_for_retrieval()`, `to_tag()` メソッド

### src/constants.rs (修正) — 前回セッション完了
- `DUAL_STORE_MAX_RETRY: u32 = 3` (Safety Invariant)
- `DUAL_STORE_ERROR_INJECTION_SEED: u64 = 67890` (Calibration Candidate)

### src/lib.rs (修正)
- `pub use store::DualStoreCoordinator;` 追加
- `pub use types::{CommitPhase, ConsistencyState, RepairAction, RepairLog};` 追加

## 検証結果
- cargo check: clean (0 warnings, 0 errors)
- cargo test: 589 passed, 0 failed
- cargo clippy -- -D warnings: clean
- cargo fmt --check: clean
- run-quality-checks: passed (all findings are pre-existing in other files or intentional OTS println!)
