# 計画: M1.75-6 helper weighting / bounded remote exploration / helper 候補フィルタ

## 要件
RFC §41B.12 に基づく helper 重み関数（式 41B-18）と bounded remote exploration（式 41B-19）、helper 候補ハードフィルタの実装。

## 変更ファイル一覧
| ファイル | 種別 | 内容 |
|---------|------|------|
| src/constants.rs | 追加 | 5件の較正候補定数 |
| src/childsupport.rs | 追加 | 3型定義 + 3純粋関数 + 10テスト |
| src/lib.rs | 追加 | 公開APIの re-export |

## 実装手順
1. constants.rs に 5 つの定数を追加
2. childsupport.rs に型定義（HelperWeight, HelperSelectionPolicy, RemoteExplorationPolicy）を追加
3. compute_helper_weights()（式 41B-18）を実装
4. mix_with_remote_exploration()（式 41B-19）を実装
5. select_helpers()（フィルタ→重み→混合→TOP-K）を実装
6. spawn_child_support_mission を select_helpers 呼び出しに更新
7. T-1〜T-8 不変条件テスト + T-O1〜T-O2 観測テスト
8. lib.rs re-export 更新
9. cargo test で全テスト通過確認

## レビュー方法
1. run-quality-checks.js を全変更ファイルに実行
2. 翻訳可能性 grep（関数名が動詞句、汎用変数名なし、マジックナンバーなし）
3. cargo test PASS 確認
4. RFC §41B.12 との無矛盾確認

## リスク
- 浮動小数点誤差による重み総和の微少ずれ（T-6 で ε=1e-10 許容）
- β 大でのアンダーフロー（saturated 加算 + 一様フォールバック）
- 既存テストへの影響なし（MAX_HELPERS_PER_MISSION 不変）
