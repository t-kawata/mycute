# 変更したファイル一覧と実装内容の概要

## src/constants.rs
- 5 つの Calibration Candidate 定数を追加:
  - `HELP_QUALITY_SUITABILITY_WEIGHT` (w_s) = 1.0
  - `HELP_QUALITY_TRUST_WEIGHT` (w_t) = 1.0
  - `HELP_QUALITY_REPUTATION_WEIGHT` (w_r) = 1.0
  - `HELP_QUALITY_CHILD_NEED_WEIGHT` (w_n) = 1.0
  - `HELP_QUALITY_DISTANCE_PENALTY` (w_d) = 1.0

## src/event.rs
- `ReciprocityLifecyclePolicy` に 6 フィールド追加: `helper_quality_w_s`〜`helper_quality_w_d`
- `Default` impl に各定数参照による初期化を追加
- `QualityScoreBreakdown` 構造体を追加（7 f32 フィールド: mission_suitability, trust, reputation, benevolence, child_need, distance_penalty, total）
- `SoftmaxWeight` 構造体を追加（helper_id: WorkflowGraphId, probability: f64, rank: usize, score_breakdown: QualityScoreBreakdown）
- 較正定数ダンプテストを追記（5 定数のダンプ + NaN 否定アサーション）
- JSON ラウンドトリップテストに 6 フィールドの初期化を追加

## src/reciprocity.rs
- `compute_helper_quality_score` (F-11): 6 成分線形結合 Q = w_s·S + w_t·T + w_r·Rep + w_b·B + w_n·N - w_d·d
- `softmax_helper_selection` (F-12): log-sum-exp trick による数値的安定な softmax
- テスト 10 件 (TC-1〜TC-10):
  - TC-1: w_b=0 後方互換
  - TC-2: benevolence 単調非減少 (n=101 sweep)
  - TC-3: 全入力ゼロ → Q=0
  - TC-4: ランダム入力 n=10,000 で NaN/Inf 不在
  - TC-5: softmax 確率和 = 1.0
  - TC-6: τ=100 → argmax > 0.999
  - TC-7: τ=0.001 → 一様分布
  - TC-8: 空リスト → 空 Vec
  - TC-9: 数値安定性 (n=10^5, max_dev=2.2e-14)
  - TC-10: τ エントロピー応答曲線 (7 水準 × 3 分布)
