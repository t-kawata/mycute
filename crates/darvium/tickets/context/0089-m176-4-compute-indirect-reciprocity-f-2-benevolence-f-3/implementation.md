# 実装サマリ: M1.76-4 間接互恵性スコア (F-2) + BenevolenceScore 集約 (F-3)

## 変更したファイル

### src/constants.rs
以下の6定数を追加:
- INDIRECT_BETA_CENTRALITY (1.0) — β_1 中心性係数 (F-2)
- INDIRECT_BETA_VILLAGE_PARTICIPATION (1.0) — β_2 村参加度係数 (F-2)
- INDIRECT_BETA_ACCEPTED_RATE (1.0) — β_3 受諾率係数 (F-2)
- INDIRECT_BETA_SUCCESS_RATE (2.0) — β_4 成功貢献率係数 (F-2)
- INDIRECT_BETA_HARM_SCORE (2.0) — β_5 負評価係数 (F-2)
- REPUTATION_WEIGHT_REPUTATION (0.30) — w_rep BenevolenceScore 評判重み (F-3)

### src/reciprocity.rs
- モジュールドキュメントに F-2/F-3 数式を追記
- logistic_sigmoid を pub(crate) に変更（F-2 から共用）
- compute_indirect_reciprocity(centrality, village_participation, accepted_rate, success_rate, harm_score) -> f32 — F-2 純粋関数
- compute_benevolence_score(direct_score, indirect_score, reputation) -> f32 — F-3 純粋関数
- 7テストケース追加 (TC-1〜TC-7)

## 実装詳細

### compute_indirect_reciprocity (F-2)
式: R_i^ind = σ(β_1·C_i^help + β_2·A_i^village + β_3·U_i^accepted + β_4·Q_i^success - β_5·B_i^harm)
- 5項線形結合 → logistic_sigmoid で [0,1] に正規化
- 時間減衰なし（RFC 設計通り、社会的評価は即時反映）
- 全入力 0 → 0.5 を返す（sigmoid(0)）

### compute_benevolence_score (F-3)
式: B_i = w_dir·R_i^dir + w_ind·R_i^ind + w_rep·Rep_i
- 3項重み付き線形和 + clamp(0.0, 1.0)
- 定数は constants.rs の REPUTATION_WEIGHT_DIRECT/INDIRECT/REPUTATION を使用
- w_dir + w_ind + w_rep = 1.0 の推奨正規化を満たす (0.35+0.35+0.30=1.0)

## テスト結果
全テスト PASS。既存テストへの影響なし。
