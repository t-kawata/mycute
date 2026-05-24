# M1.75-5: child-support TrainingMission specialization 実装計画

## 要件の再確認

1. `TrainingMissionKind` 列挙型（Production / ChildSupport）の追加
2. `ChildSupportMissionPayload` 構造体の定義
3. `spawn_child_support_mission()` 関数の実装
4. Production plane ガード
5. EventBus publish + TrainingRunLog 拡張
6. 全テスト（不変条件13 + 結合2 + 観測2）

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|---------|------|------|
| src/constants.rs | 編集 | MAX_HELPERS_PER_MISSION, CHILD_SUPPORT_MISSION_TIMEOUT_SECS 追加 |
| src/types.rs | 編集 | TrainingMissionKind enum + is_allowed_on_production_plane() 追加 |
| src/childsupport.rs | 新規 | SafetyScope, ChildSupportMissionPayload, spawn, plane guard, 全テスト |
| src/lib.rs | 編集 | mod childsupport + re-export 追加 |

## 実装手順

1. constants.rs に定数追加
2. types.rs に TrainingMissionKind 追加
3. event.rs の DarviumEvent 構造体を確認して childsupport.rs 実装
4. lib.rs にモジュール登録
5. コンパイル確認 → テスト実行 → 修正
6. clippy 確認

## 物理的レビュー方法

- run-quality-checks.js で全変更ファイルをチェック
- cargo test で全テスト PASS 確認
- cargo clippy -- -D warnings 通過確認

## リスク

- DarviumEvent の構造体フィールドが event.rs で定義された実態と一致しないリスク → 事前に確認
- LocalVillage が Serialize/Deserialize 未実装 → serde derive を使用しない
