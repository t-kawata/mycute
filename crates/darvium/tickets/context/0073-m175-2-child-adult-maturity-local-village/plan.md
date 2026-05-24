# 計画: チケット #73 M1.75-2 Child/Adult maturity 判定器および Local Village 構成ロジック

## 要件
- WorkflowMaturity enum (Child/Adult)
- classify_maturity 純粋関数 (経験値・信頼・レピュテーション3軸)
- LocalVillage / AdultCandidate 構造体
- filter_adult_candidates (ConsistencyState + maturity フィルタ)
- build_local_village_topk / build_local_village_radius
- centroid 計算
- 4定数を constants.rs に追加

## 変更ファイル
| ファイル | 種別 | 内容 |
| src/constants.rs | 修正 | 4定数追加 |
| src/village.rs | 新規 | 全型定義 + 関数 + 21テスト |
| src/lib.rs | 修正 | mod + pub use |

## 実装手順
1. constants.rs に4定数追加
2. src/village.rs 作成（型→関数→テスト）
3. lib.rs にモジュール追加
4. cargo test 全PASS確認
5. clippy 通過確認
6. 観察レポート生成

## 物理的レビュー方法
- run-quality-checks.js on changed files
- cargo clippy -- -D warnings
- 翻訳可能性grep（名詞始まり関数、1文字変数、ハードコード値）
- RFC §41B.3 交叉参照

## リスク
- なし（純粋関数のみ、外部依存なし）
