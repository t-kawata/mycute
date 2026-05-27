# MTR-D: Capability & Knowledge Metrics Backfill — 実装計画

## 要件

spec 通り、3 関数追加 + `collect_final_metrics` 更新 + D1–D8 テスト。

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|---------|------|------|
| `src/kind_world.rs` | 修正 | 3 関数追加、`collect_final_metrics` 更新、D1–D8 テスト追加 |

**新規 SimulationContext フィールドは不要** — `ctx.positions` と `ctx.reciprocity_pair_counts` で全て導出可能。

## 関数名の決定

Rust は関数オーバーロードをサポートしないため、旧パス関数との衝突を避ける：

- `compute_capability_coverage` — 同名が存在しないためそのまま（old: `compute_capability_coverage_shannon`）
- `compute_reuse_ratio_from_pair_counts` — old: `compute_reuse_ratio` と衝突回避
- `compute_knowledge_diffusion_from_pair_counts` — old: `compute_knowledge_diffusion_rate` と衝突回避

## 実装内容

1. `compute_capability_coverage(positions)`: Shannon 多様性指数 on 10×10 grid
2. `compute_reuse_ratio_from_pair_counts(pair_counts)`: 頻度 >= 2 の比率
3. `compute_knowledge_diffusion_from_pair_counts(pair_counts)`: ユニークペア / 全インタラクション
4. `collect_final_metrics`: 3 行更新 (lines 2364, 2366, 2374)
5. D1–D8 テスト

## 計装・観測

- D1–D6: 通常の `#[test]` ユニットテスト（assert_eq! / assert!）
- D7: 観測テスト — simulate_kw_reality ヘルパーでシミュレーション実行、3 指標を println!
- D8: cargo test 全 PASS 確認

## 較正ループ

自由パラメータなし — 純粋プロキシ値バックフィルのため定数変更不要。

## 実装手順

1. compute_capability_coverage 関数追加
2. compute_reuse_ratio_from_pair_counts 関数追加
3. compute_knowledge_diffusion_from_pair_counts 関数追加
4. collect_final_metrics 更新
5. D1–D7 テスト追加
6. cargo build → cargo test → --nocapture 観測
7. 観察レポート保存 → done 遷移
