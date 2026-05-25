# 実装計画: M1.76-4 間接互恵性スコア (F-2) + BenevolenceScore 集約 (F-3)

## 要件の再確認
2つの純粋関数を src/reciprocity.rs に追加:
- F-2: compute_indirect_reciprocity(centrality, village_participation, accepted_rate, success_rate, harm_score) -> f32
- F-3: compute_benevolence_score(direct_score, indirect_score, reputation) -> f32
- 6つの定数を src/constants.rs に追加
- 6テストケース + 計装テスト
- logistic_sigmoid を pub(crate) に変更し F-2 と共用

## RFC 既存実装状態検証
- F-2/F-3 は RFC §15.10.2 の数式のみ。新規型定義なし
- β_1〜β_5、w_rep は未定義 → 新規追加
- 既存コードとの非互換性なし

## 変更ファイル一覧
| ファイル | 種別 | 内容 |
|---------|------|------|
| src/constants.rs | 修正 | 6定数追加 (β_1〜β_5, w_rep) |
| src/reciprocity.rs | 修正 | 2関数追加 + logistic_sigmoid pub(crate)化 + モジュールドキュメント更新 |

## 実装手順
1. 定数追加: constants.rs に INDIRECT_BETA_* 6定数
2. sigmoid 共用化: logistic_sigmoid を pub(crate) fn に
3. F-2 実装: compute_indirect_reciprocity
4. F-3 実装: compute_benevolence_score
5. モジュールドキュメント更新: F-2/F-3 数式追記
6. テスト実装: TC-1〜TC-6 + TC-7(計装)
7. 検証: cargo test + cargo clippy

## 計装・観測の実装計画
- テストファイル: src/reciprocity.rs 内 mod tests
- TC-7(計装): test_indirect_response_surface — 応答曲面(11×11) + β sweep(4値×5変数)
- 観測出力: println! CSV形式 → cargo test -- --nocapture
- サンプルサイズ: TC-4 n=10,000、TC-2/3 n=50、TC-7 141点

## 物理的レビュー方法
- run-quality-checks.js で reciprociry.rs + constants.rs
- 翻訳可能性 grep: 名詞始まり関数、ハードコード数値、デバッグ出力
- cargo test -- --nocapture で観測出力確認

## リスク評価
- 低: 純粋関数のみ。既存コードへの影響は logistic_sigmoid 可視性変更のみ
