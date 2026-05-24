# 実装サマリー: M1.75-1

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|---|---|---|
| src/spaceposition.rs | NEW | 型定義 (SpacePositionEmbedding, VillagePosition, PositionUpdatePolicy) + 純粋関数 (update_space_position, should_update_position, l2_distance, is_position_equal) + EventBus publish (publish_position_update) + ユニットテスト 19件 |
| src/constants.rs | 追加 | SPACE_POSITION_UPDATE_ALPHA=0.30, SPACE_POSITION_UPDATE_MIN_INTERVAL=5, SPACE_POSITION_L2_EPSILON=1e-6 |
| src/event.rs | 追加 | SystemEvent::SpacePositionUpdated(SpacePositionUpdatedPayload) variant + proptest戦略拡張 |
| src/types.rs | 追加 | VillageObservation 構造体定義 + new() コンストラクタ |
| src/lib.rs | 追加 | pub mod spaceposition + 全公開型の re-export |

## 実装した型

- SpacePositionEmbedding: Option<[f32; 3]> の newtype (From<[f32;3]>, From<Option<[f32;3]>>, unknown(), inner())
- VillagePosition: { position: [f32;3], last_updated_vt: u64 }
- VillageObservation: { mission_locality, helper_locality, knowledge_locality, delta } (RFC §41B.2)
- PositionUpdatePolicy: Always / MinInterval(u64) / Manual
- SpacePositionUpdatedPayload: { prev, current, observation, alpha }

## 実装した関数

- update_space_position(prev, obs, alpha) -> [f32;3] (式 41B-1)
- should_update_position(last_vt, now_vt, policy) -> bool
- l2_distance(a, b) -> f64
- is_position_equal(a, b) -> bool
- publish_position_update(bus, prev, current, observation, alpha) -> Result<(), DarviumError>

## テスト結果

- ユニットテスト 19件: 全 PASS (T-1〜T-8 + 追加テスト)
- 既存テスト 726件: 全 PASS (回帰なし)
- clippy -- -D warnings: PASS (新規警告なし)
