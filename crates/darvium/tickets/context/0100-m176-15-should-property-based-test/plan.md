# 実装計画: M1.76-15 プロパティベース不変条件ファジング

## 要件
proptest を用いた6種の不変条件検証（T1-T6）+ 3種の極端ケーステスト（E1-E3）+ failing seed export

## 変更ファイル一覧
| ファイル | 種別 | 内容 |
|---|---|---|
| src/reciprocity.rs | 変更 | mod tests 内に proptest 戦略6種 + テスト6種 + 極端ケース3種 + 観測サマリ |

## 計装・観測の実装計画
- src/reciprocity.rs mod tests 内に実装
- proptest: prop_compose! で6種戦略定義、proptest! で6種テスト
- ProptestConfig { cases: PROPTEST_DEFAULT_CASES } (10,000 cases)
- 観測出力: println! + --nocapture
- Welch t-test: grace_period_child_protection の保護効果検証
- 較正: 本チケットでは constants.rs 変更なし

## 実装手順
1. mod tests に use proptest 追加
2. 6種 proptest 戦略定義（prop_compose!）
3. T1-T6 proptest テスト実装
4. E1-E3 極端ケーステスト実装
5. Failing seed export 機構実装
6. 観測サマリ関数実装
7. cargo test 全テスト PASS 確認
8. cargo test -- --nocapture 観測出力確認

## 物理的レビュー方法
1. run-quality-checks.js 静的チェック
2. cargo test 全テスト PASS
3. cargo test -- --nocapture 観測出力確認
4. 翻訳可能性 grep

## リスク
- proptest 60,000 cases でテスト時間増加 → PROPTEST_DEFAULT_CASES 調整可能
- NaN/Inf 検出 → softplus/clamp 実装が正しければ violation 0 見込み
- failing seed fixture ディレクトリ存在確認
