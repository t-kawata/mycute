# 計画: M1.76-9 Benevolence-aware remote exploration (F-13)

## 要件
RFC §41B.20.3 式 F-13 の純粋関数 `compute_benevolence_aware_remote_exploration` を実装し、既存 `select_helpers()` の ε を動的に上書きする adapter と接続する。

## 変更ファイル一覧
| ファイル | 種別 | 内容 |
|----------|------|------|
| src/constants.rs | 修正 | a₁, a₂ 定数追加 |
| src/event.rs | 修正 | ReciprocityLifecyclePolicy に a₁, a₂ フィールド追加 |
| src/reciprocity.rs | 修正 | F-13 関数実装 + テスト追加 |

## 実装手順
1. constants.rs に定数追加
2. event.rs にフィールド追加 + Default 実装修正 + JSON roundtrip 修正
3. reciprocity.rs に関数 + テスト実装
4. cargo test 全件確認

## 物理的レビュー方法
- cargo test 全件 PASS
- quality checks (run-quality-checks.js)
- RFC §41B.20.3 との数式一致確認

## リスク
- 低: 純粋関数追加のみで既存コードに影響なし
- adapter パターンで select_helpers のシグネチャ不変
