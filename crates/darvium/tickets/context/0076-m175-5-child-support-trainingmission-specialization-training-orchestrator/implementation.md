# 変更したファイル一覧と実装内容の概要

## src/constants.rs
- `MAX_HELPERS_PER_MISSION: u32 = 10` 追加 — 1 mission あたりの最大 helper 数
- `CHILD_SUPPORT_MISSION_TIMEOUT_SECS: u64 = 3600` 追加 — mission タイムアウト

## src/types.rs
- `pub enum TrainingMissionKind { Production, ChildSupport }` 追加
- `impl TrainingMissionKind` に `is_allowed_on_production_plane()` 追加
- 既存の stub `pub struct TrainingMission;` を削除

## src/childsupport.rs (新規)
- `SafetyScope` enum: ReadOnly / LimitedMutation / FullSandbox
- `ChildSupportMissionPayload` struct: child_id, helper_ids, village_snapshot, objective, safety_scope, timeout_secs — new() で不変条件検証
- `spawn_child_support_mission()`: village から mission 生成 → EventBus publish
- `is_allowed_on_plane()`: production plane ガード
- テスト 13 件: T-1〜T-10 (不変条件), T-E1〜T-E2 (EventBus結合), T-O1〜T-O2 (観測)

## src/lib.rs
- `pub mod childsupport;` 追加
- `pub use childsupport::{...}` で全公開型を re-export
- `TrainingMissionKind` を types re-export に追加

## ビルド結果
- cargo test: 802 tests, 0 failed
- cargo clippy -- -D warnings: 通過
