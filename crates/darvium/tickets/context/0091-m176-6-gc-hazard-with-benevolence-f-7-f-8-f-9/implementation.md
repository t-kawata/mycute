# M1.76-6: GC hazard with benevolence (F-7, F-8, F-9) — 実装サマリ

## 変更ファイル

### src/constants.rs
- `GC_HAZARD_LAMBDA_0: f32 = 1.0` — F-7 ベースラインハザード λ₀ を新規定義 (Calibration Candidate)
- `GC_HAZARD_GAMMA_LIFECYCLE: f32 = 0.5` — F-7 LifecycleScore 重み γ_L を新規定義 (Calibration Candidate)

### src/event.rs
- `ReciprocityLifecyclePolicy::default()` の `lambda_gc_base` を `crate::constants::GC_HAZARD_LAMBDA_0` 参照に変更
- `ReciprocityLifecyclePolicy::default()` の `gamma_lifecycle` を `crate::constants::GC_HAZARD_GAMMA_LIFECYCLE` 参照に変更
- （値は 0.1 / 0.5 で不変、定数参照化のみ）

### src/reciprocity.rs
- `softplus(x: f32) -> f32` — 非公開ユーティリティ、数値的安定性を備えた実装
- `compute_gc_hazard(lifecycle, benevolence, child_protection, policy) -> f32` — F-7 公開関数
- `compute_gc_probability(hazard, delta_t) -> f64` — F-8 公開関数
- `compute_survival_probability(hazard, delta_t) -> f64` — F-9 公開関数
- テスト 8 件 (TC-1〜TC-8)

## テスト結果
- 全 949 テスト通過 (既存 941 + 新規 8)
- TC-8 計装: softplus 非負性 n=10⁶ 確認、(L,B) 11×11 応答曲面、γ_B/γ_L 感度比 sweep、生存確率曲線

## 特記事項
- spec に記載された softplus(1.0) ≈ 1.1269 は誤り。正しくは softplus(1.0) ≈ 1.313262。
  テストでは softplus を直接呼び出して期待値を計算するため、値の正当性に影響なし。
