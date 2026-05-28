# 実装計画: 評判再計算パイプラインのプロダクション実装とシミュレーション完全性確保

## 要件の再確認

RFC §15.10 の評判再計算パイプライン（F-1〜F-5）がプロダクションコードおよび本格シミュレーションで呼ばれていない問題を修正する。新規数式・新規データ構造は不要。オーケストレーションの追加のみ。

**RFC 既存実装状態検証**: 全16フィールドの ReputationProfile、全22定数、ReciprocityLifecyclePolicy（30フィールド）、全計算関数すべて RFC と完全一致。乖離ゼロ。

## 変更ファイル一覧

| # | ファイル | 種別 | 内容 |
|---|---------|------|------|
| 1 | src/simulation.rs | 修正 | run_kw_real_simulation / run_evaluation_simulation / run_evaluation_simulation_with_channel に recompute_trust_reputation 呼び出し追加 |
| 2 | src/lib.rs | 修正 | Darvium に recompute_reputations() / tick() メソッド追加 |
| 3 | src/store/coordinator.rs | 修正 | SubWorkflow 生成時に inherit_reputation を呼ぶ |
| 4 | src/help.rs | 修正 | HELP 成功時に experience_count increment + inherit_reputation |
| 5 | src/village.rs | 修正 | 村クラスタリング後の中心性計算追加 |
| 6 | src/simulation.rs | 追加 | T1 観測テスト（mod tests 内） |

## 実装手順

Step 1: シミュレーションループ修正（最小影響）
Step 2: facade に tick() 追加
Step 3: experience_count インクリメント
Step 4: inherit_reputation プロダクション呼び出し
Step 5: village_centrality 算出
Step 6: テスト実装＋全テスト確認

## 物理的レビュー方法

1. run-quality-checks.js で品質チェック
2. 翻訳可能性 grep
3. cargo test 全通過確認
4. T1 観測出力で評判値変化を目視確認
