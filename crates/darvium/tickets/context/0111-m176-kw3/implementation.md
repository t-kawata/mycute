# 変更したファイル一覧と実装内容の概要

## 変更ファイル

| ファイル | 種別 | 内容 |
|---------|------|------|
| src/kind_world.rs | 修正 | KW3 村間相互作用・知識拡散トラッキング実装 |

## 実装内容

### 追加した構造体
- `VillageInteractionMetrics` — 8フィールド（tick, village_count, cross_village_interaction_rate, village_formation_strength, knowledge_diffusion_rate, village_flow_balance, mean_village_size, village_size_variance）

### 追加した関数
1. `assign_village_ids` — DBSCAN類似の空間クラスタリング（BFS expansion）。VILLAGE_DISTANCE_THRESHOLD=0.2, VILLAGE_MIN_SIZE=3 を使用。返り値 Vec<Option<usize>>。
2. `compute_cross_village_interaction_rate` — スタブ関数（population ID→村ラベルの解決ができないため VillageInteractionObserver 内で計算）
3. `compute_village_formation_strength` — Silhouette類似スコア。各村の重心との平均距離を最大距離 √2 で正規化。
4. `compute_knowledge_diffusion_rate` — 村間の平均 experience 標準偏差の変化率。(σ_prev - σ_current) / σ_prev。
5. `compute_village_flow_balance` — 村 ID 変更比率（churn 率）。

### 更新した関数
- `compute_village_health_score` — flow_balance_health を線形近似（1.0 - churn_rate）から二値判定（churn ∈ [0.05, 0.30] → 1.0, 範囲外 → 0.0）に変更。

### 追加した構造体
- `VillageInteractionObserver` — previous_assignments を内部状態として保持。observe() で全村指標を計算し VillageInteractionMetrics を返す。print_csv() で OBS-KW3 プレフィックス付き CSV 出力。

### 追加したテスト（16 TC）
- TC1-TC4: assign_village_ids の動作検証（密集/孤立/同一位置/空）
- TC5-TC8: 4指標の純粋関数テスト
- TC9-TC10: 空入力/全None の graceful ハンドリング
- TC11: 後方互換性確認
- TC12-TC14: churn 適正範囲の境界値テスト
- TC15: VillageInteractionObserver 統合テスト（2 tick, CSV出力）
- TC16: 観測テスト（20 tick, StdRng, NaN/Inf 検証, CSV出力）

## 設計上の決定
- SimWorkflowState に村 ID 永続フィールドを追加しない（RFC §41B.3 遵守）
- 村割り当て履歴は VillageInteractionObserver の previous_assignments のみが保持
- 定数は既存の constants.rs の VILLAGE_DISTANCE_THRESHOLD, VILLAGE_MIN_SIZE, KW_VILLAGE_CHURN_LOWER/UPPER を使用
