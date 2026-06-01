# 空間移動力学 — 首長レジストリと引力・斥力による個体移動

## 変更したファイル一覧

| ファイル | 種別 | 内容 |
|---|---|---|
| `src/constants.rs` | 追加 | MOVEMENT_DISTANCE (0.02), MIN_APPROACH_DISTANCE (0.05) — Calibration Candidate |
| `src/chief_registry.rs` | **新規** | ChiefRegistry 構造体（シングルトン管理）、ChiefEntry、sync_from_chiefs、get_paramount、get_nearest、get_second_nearest、T1-T9 テスト |
| `src/lib.rs` | 追加 | `pub mod chief_registry;` |
| `src/simulation.rs` | 追加 | SimulationContext に chief_registry (Arc RwLock), chief_movement_targets (HashMap) 追加。6関数（compute_attraction_vector, compute_repulsion_vector, compute_chief_movement_vector, compute_non_chief_movement_vector, random_nearby_direction, phase3_chief_movement）。Phase 3.9 呼び出しを Phase 3.8 直後に追加。T2-T8 + O1-O3 テスト。 |

## 実装の概要

### ChiefRegistry
- `sync_from_chiefs(village_chiefs, population)`: Phase 3.8 の村首長マップからレジストリ再構築
- `get_paramount()`: chiefdom_score 最大の首長（主首長）を返す
- `get_nearest(pos)` / `get_second_nearest(pos)`: L2距離に基づく最近接首長検索
- 死亡個体は自動除外

### 移動力学
- **主首長**: 移動しない（不動点）
- **副首長（主首長以外の首長）**: 主首長への引力 + 他副首長からの斥力の合力方向に MOVEMENT_DISTANCE 移動
- **非首長**: 最寄り首長への引力。距離が MIN_APPROACH_DISTANCE 未満になると2番目首長に永久固定。単一首長時は近傍ランダム点へフォールバック
- **設計上の判断**: 主首長は斥力対象から除外（引力で既に考慮済み、2首長系での力の完全相殺防止）

### Phase 3.9
Phase 3.8（首長選出）直後、Phase 3.6（自己抽象化）の前に配置。
