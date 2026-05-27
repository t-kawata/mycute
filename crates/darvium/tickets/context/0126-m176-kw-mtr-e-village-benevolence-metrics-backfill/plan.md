# M1.76-KW-MTR-E: Village Churn & Benevolence Ratio Backfill — 実装計画

## 要件

- `collect_final_metrics` 内の `village_churn_rate: 0.0` と `benevolent_vs_non_benevolent_coverage_ratio: 1.0` を実測値で置き換え
- 全 23 フィールドが実測値化される（MTR 系列完了）

## RFC §15.9.3 既存実装状態検証

| フィールド | RFC の定義 | 現行コードの型 | 状態 |
|---|---|---|---|
| village_churn_rate | churn rate [0, 1] | KindWorldMetricsInput.village_churn_rate: f64 | ✅ フィールド存在、値は 0.0 プレースホルダ |
| benevolent_vs_non_benevolent_coverage_ratio | 慈悲的(上位20%)/非慈悲的(下位20%)能力カバー率比 | KindWorldMetricsInput.benevolent_vs_non_benevolent_coverage_ratio: f64 | ✅ フィールド存在、値は 1.0 プレースホルダ |

評価: 両フィールドとも型・構造に乖離なし。実測値化のみが必要。

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|---|---|---|
| src/simulation.rs | 変更 | SimulationContext に 2 フィールド追加 + 初期化 |
| src/simulation.rs | 変更 | phase2_village_clustering に累積カウンタ更新 |
| src/kind_world.rs | 追加 | compute_village_churn_rate 関数 |
| src/kind_world.rs | 追加 | compute_benevolent_vs_non_benevolent_coverage_from_trust 関数 |
| src/kind_world.rs | 変更 | collect_final_metrics の 2 行置き換え |
| src/kind_world.rs | 追加 | テスト E1-E7 |

## 計装・観測の実装計画

- collect_final_metrics 出力に village dynamics 2 指標を含める
- 観測テスト E6 で --nocapture 出力を確認、両指標が非デフォルト値であることを検証
- テスト E7: 既存テスト全 PASS で回帰なし確認

## Boy Scout 改善

- compute_benevolent_vs_non_benevolent_coverage_from_trust にプロキシ値の限界をドキュメントコメントとして明記
- 旧パス関数とは別名（_from_trust サフィックス）で混同防止

## 実装手順

1. SimulationContext にフィールド追加 + new() で 0 初期化
2. phase2_village_clustering の子割り当て部にカウンタ更新挿入
3. compute_village_churn_rate 関数
4. compute_benevolent_vs_non_benevolent_coverage_from_trust 関数
5. collect_final_metrics 更新
6. テスト E1-E7 追加
7. cargo build + cargo test + cargo clippy

## 物理的レビュー方法

- run-quality-checks.js で変更ファイルチェック
- E1-E7 全 PASS
- --nocapture 出力で両指標が非デフォルト値であることを確認

## リスク

- TrustProfile 合成スコアを慈悲スコアのプロキシとして使用する近似の限界（ドキュメントコメントに明記）
- phase2_village_clustering への挿入コードが元のロジックを変更しないよう注意
