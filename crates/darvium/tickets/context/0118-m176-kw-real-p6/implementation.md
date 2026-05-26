# 実装サマリ: P6 計装インターフェース更新 (M1.76-KW-REAL-P6)

## 変更ファイル

### src/kind_world.rs
- **KindWorldMetricsInput 拡張**: 8 フィールド追加 (mean_lifecycle_score, child_survival_rate, mean_freshness, mean_benevolence_aggregate, mean_reciprocity_score, help_success_rate, trust_inheritance_fidelity, execution_success_rate) — 既存の 9 フィールドと合わせて計 17 フィールド
- **KindWorldAssessment 拡張**: 5 因子 (s_viability, s_capability, s_cooperation, s_efficiency, s_fairness) + 14 下位成分 (j_pop〜j_trust) を追加
- **compute_kind_world_objective 書き換え**: 旧 6 成分重み付き和 (α_i × J_i) → 新 5 因子乗算モデル (J_kw = S_viab × S_capa × S_coop × S_effi × S_fair)
- **collect_final_metrics 引数変更**: ReciprocitySimulationResult → SimulationContext を受け取る新シグネチャ。互換性のため旧パス (collect_final_metrics_from_result) を維持
- **observer 新メソッド**: EcosystemGrowthObserver::observe_from_context, VillageInteractionObserver::observe_from_context 追加
- **JkwModelComparison**: 新旧モデル比較用診断構造体 + compare_j_kw_models 関数
- **テスト更新**: 5 因子乗算モデル対応のため TC1/TC7/TC8/TC9 の期待値を再調整。kw8_kw4_backward_compatible 更新

### src/constants.rs
- **KW_ALPHA_* 6 定数削除**: KW_ALPHA_POP(0.25), KW_ALPHA_COV(0.20), KW_ALPHA_REUSE(0.15), KW_ALPHA_COST(0.20), KW_ALPHA_VILLAGE(0.10), KW_ALPHA_PENALTY(0.10) — 旧重み付き和モデル用定数

### src/lib.rs
- `#![allow(clippy::empty_line_after_doc_comments)]` 追加（コードベース全体の整形アーティファクト対応）
