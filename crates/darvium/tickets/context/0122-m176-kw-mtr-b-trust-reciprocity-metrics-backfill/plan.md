# 実装計画: M1.76-KW-MTR-B — Trust & Reciprocity Metrics Backfill

## RFC §15.9.2 既存実装状態検証

### RFC §10 TrustProfile
| フィールド | RFC の型 | 現行コードの型 | 状態 |
|---|---|---|---|
| operational | f32 | f64 | ⚠️ 型不一致（安全な拡張） |
| semantic | f32 | f64 | ⚠️ 型不一致（安全な拡張） |
| temporal | DualTemporalTrust | f64 | ⚠️ 型不一致（簡略実装） |
| human | HumanTrustLogistic | HumanTrustLogistic | ✅ 一致 |

**評価サマリ**: 型不一致はいずれもシミュレーション実装の簡略化によるもので、本チケットのスコープ外。

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|---------|------|------|
| simulation.rs | 修正 | SimulationContext: total_inheritance_fidelity, inheritance_event_count, reciprocity_pair_counts 追加 |
| simulation.rs | 修正 | new() 初期化、phase1 fidelity 記録、phase3 ペアカウント更新 |
| kind_world.rs | 追加 | compute_mean_benevolence, compute_mean_reciprocity, compute_trust_inheritance_fidelity |
| kind_world.rs | 修正 | collect_final_metrics 3 指標置き換え |

## 実装手順

1. SimulationContext に 3 フィールド追加 + new() 初期化
2. phase1_population_growth: inherit_trust 後に fidelity 記録
3. phase3_help_protocol: HELP セッション作成時にペアカウント更新
4. kind_world.rs: 3 関数追加
5. collect_final_metrics: 3 指標置き換え
6. Boy Scout: phase5 の 0.7 → TRUST_INHERIT_DECAY
7. テスト B1-B7

## Boy Scout 改善

- phase5_capability_diffusion: inherit_trust(ht, ct, 0.7) のハードコード 0.7 → TRUST_INHERIT_DECAY

## レビュー方法

run-quality-checks.js + 翻訳可能性 grep

## リスク

- mean_benevolence_aggregate は TrustProfile の 3 次元平均の proxy（RFC 定義と完全一致しない）
- デフォルト設定で出生が発生しない場合、pair_counts も空になり reciprocity=0.0 となる可能性
