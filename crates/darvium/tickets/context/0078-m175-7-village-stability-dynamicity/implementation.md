# 実装サマリ: M1.75-7 Village Stability / Dynamicity Metrics

## 変更したファイル

| ファイル | 種別 | 内容 |
|---------|------|------|
| src/constants.rs | 追加 | VILLAGE_METRICS_WINDOW_SIZE, VILLAGE_EVENT_PROJECTION_NAME, VILLAGE_STABILITY_MAX_CHURN_P95, VILLAGE_DYNAMICITY_MIN_LONG_HORIZON_CHANGE の4定数 |
| src/village.rs | 追加 | VillageMetrics (9 fields), VillageMetricsWindow (VecDeque リングバッファ), VillageMetricsSnapshot (p50/p95 集約) の3構造体 |
| src/village.rs | 追加 | compute_position_drift, compute_village_jaccard, compute_village_churn, compute_helper_jsd (Jensen-Shannon Divergence), compute_child_survival_rate, compute_child_maturation_time の6関数 |
| src/village.rs | 追加 | 13不変条件テスト (T-1〜T-13) + 3観測テスト (T-O1〜T-O3) |
| src/event.rs | 追加 | VillageEvent enum (TickCompleted), DarviumEventKind::Village variant, DomainProjection::village_observation_log() constructor |
| src/event.rs | 更新 | initialize_domain_projections() に第5 projection 追加, TC-1 を14 variant に更新 |
| src/event.rs | 追加 | 4 EventProjection テスト (T-E1〜T-E4) + 計装サマリ (T-O4) |
| src/lib.rs | 更新 | VillageEvent, VillageMetrics, VillageMetricsWindow, VillageMetricsSnapshot, compute_* 関数の re-export |

## RFC との無矛盾性

- RFC §41B.14: position drift (式41B-21), Jaccard overlap (式41B-22), village churn (式41B-23) を完全実装
- RFC §41B.15: VILLAGE_STABILITY_MAX_CHURN_P95, VILLAGE_DYNAMICITY_MIN_LONG_HORIZON_CHANGE を定数定義。長期指標（trust growth slope 式41B-24, long-horizon Jaccard 式41B-25）は将来拡張として識別
- EventProjection フレームワーク: v2.3-g の additive variant ルールに従い既存 variant を変更せず追加

## 観測ベース検証

- 不変条件テスト: 全13件 PASS
- 観測テスト: グリッド掃引 (T-O1), Window集約 (T-O2), イベント分離 (T-E1〜T-E4) 全件 PASS
- 全817テスト PASS
