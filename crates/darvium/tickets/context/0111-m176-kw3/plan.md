# 計画: M1.76-KW3: 村間相互作用・知識拡散トラッキング

## RFC 既存実装状態検証

### RFC §15.9.4 — 村間相互作用指標
RFC §15.9.4 は関数の意味論と数式を定義。以下の関数が実装対象:

| 関数 | RFC 定義 | 現行コード | 状態 |
|------|----------|-----------|------|
| assign_village_ids | DBSCAN 類似の空間クラスタリング | 未実装 | ❌ 未実装 |
| compute_cross_village_interaction_rate | 村間セッション割合 | 未実装 | ❌ 未実装 |
| compute_village_formation_strength | silhouette 類似スコア | 未実装 | ❌ 未実装 |
| compute_knowledge_diffusion_rate | experience 分散の時間変化率 | 未実装 | ❌ 未実装 |
| compute_village_flow_balance | churn 率 | 未実装 | ❌ 未実装 |
| compute_village_health_score | 4 指標合成、churn 適正範囲判定 | 1.0 - churn_rate の線形近似 | ❌ ロジック不一致 |

### RFC §41B.3 — Child, adult, local village
SimWorkflowState.position: ✅ 一致 (f32; 3)
村 ID 永続フィールド: 禁止（§41B.3）→ 存在しない ✅

## 変更ファイル一覧
- src/kind_world.rs: VillageInteractionMetrics 構造体、6 関数、VillageInteractionObserver、14 TC
- src/constants.rs: 変更なし（既存定数を使用）
- src/lib.rs: 変更なし

## 実装手順
1. VillageInteractionMetrics 構造体定義
2. compute_village_health_score 更新（適正範囲判定）
3. assign_village_ids 実装（簡易 DBSCAN）
4. compute_cross_village_interaction_rate 実装
5. compute_village_formation_strength 実装
6. compute_knowledge_diffusion_rate 実装
7. compute_village_flow_balance 実装
8. VillageInteractionObserver 実装
9. 14 TC 実装

## 物理的レビュー方法
1. cargo check
2. cargo test -- --nocapture 2>&1 | grep -E "(PASS|FAIL|OBS-KW3:|test result)"
3. 翻訳可能性 grep: 関数名が動詞句、unwrap なし、ハードコード定数なし
4. run-quality-checks.js
5. 既存テスト回帰確認

## リスク
- DBSCAN O(n²) だが n <= 100 で許容範囲
- village_assignments と population の長不一致 → debug_assert_eq! でガード
- ゼロ割 → max(denominator, 1e-10) でガード
- compute_village_health_score 修正による KW1 テストへの影響確認
