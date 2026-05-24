# 実装サマリ: M1.75-6 helper weighting、bounded remote exploration、および helper 候補フィルタ

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|----------|------|------|
| src/constants.rs | 追加 | 5つの較正用定数を定義（HELPER_WEIGHT_DISTANCE_DECAY_BETA, TRUST_EXPONENT, REPUTATION_EXPONENT, EXPLORATION_EPSILON, DEFAULT_TOP_K） |
| src/childsupport.rs | 追加 | HelperWeight 構造体、HelperSelectionPolicy 構造体、compute_helper_weights（式41B-18）、mix_with_remote_exploration（式41B-19）、select_helpers（統合パイプライン）、11のテスト（8不変条件+2観測+1計装サマリ） |
| src/lib.rs | 更新 | re-export に compute_helper_weights, mix_with_remote_exploration, select_helpers, HelperSelectionPolicy, HelperWeight を追加 |

## 実装内容

### 型定義
- `HelperWeight { helper_id: WorkflowGraphId, weight: f64, is_remote: bool }` — 選定結果
- `HelperSelectionPolicy { beta, trust_exponent, reputation_exponent, epsilon, top_k }` — 選定パラメータ

### 純粋関数
- `compute_helper_weights(distances, trusts, reputations, policy) -> Vec<f64>` — 式41B-18 正規化重み計算
- `mix_with_remote_exploration(local_weights, epsilon) -> Vec<f64>` — 式41B-19 ε混合
- `select_helpers(candidates, child_pos, trusts, reputations, policy) -> Vec<HelperWeight>` — フィルタ→距離計算→重み→混合→TOP-K

### テスト
- T-1〜T-8: 不変条件テスト（近距離優先、品質優位、ハードフィルタ、ε境界、正規化、β=0、空リスト）
- T-O1: β-εグリッド掃引（5β × 6ε = 30 grid points）
- T-O2: 距離-品質トレードオフ観測
- T-E1: 計装サマリ（全型・関数の存在確認）

## Boy Scout 改善
- 翻訳可能性を維持（動詞始まり関数名、ドメイン変数名、純粋関数設計）
- 全マジックナンバーを constants.rs の名前付き定数に集約
- unwrap/expect 不使用（Result 伝播または純粋関数の戻り値）
