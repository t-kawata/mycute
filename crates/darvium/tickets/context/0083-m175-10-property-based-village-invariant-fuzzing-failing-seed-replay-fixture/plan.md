# 計画: M1.75-10 property-based village invariant fuzzing

## 要件の再確認

village 不変条件（helper 選定、ConsistencyState フィルタ、HELP 終端状態の非再入性、empty village fallback）を proptest でファジングし、違反を発見した seed を replay fixture に自動昇格する。

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|---|---|---|
| src/constants.rs | 変更 | PROPTEST_DEFAULT_CASES, VILLAGE_FIXTURE_DIR |
| src/replay.rs | 変更 | proptest strategy, invariant assertion F-1〜F-7 |
| tests/fixtures/ | 新規 | failing seed fixture 保存用ディレクトリ |

## 計装・観測の実装計画

- proptest strategy: prop_compose! で WorkflowConfig 他を生成
- invariant tests: 7 テスト (F-1〜F-7), 各 10,000 cases
- 観測出力: println! + --nocapture
- Fixtures: tests/fixtures/village_invariant_failures/

## 実装手順

1. constants.rs に定数追加
2. replay.rs に proptest 戦略 + invariant tests 実装
3. cargo test 全通過確認
4. 観測レポート生成

## 物理的レビュー方法

run-quality-checks.js + 翻訳可能性 grep

## リスク

proptest 実行時間、fixture I/O パス
