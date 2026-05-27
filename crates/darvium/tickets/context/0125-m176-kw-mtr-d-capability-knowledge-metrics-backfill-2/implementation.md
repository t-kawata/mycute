# MTR-D: Capability & Knowledge Metrics Backfill — 実装サマリ

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|---------|------|------|
| `src/kind_world.rs` | 修正 | 3 関数追加、collect_final_metrics 更新、D1-D8 テスト追加 |

## 新規追加した関数

### `compute_capability_coverage`
- positions の位置分布を 10×10 グリッドに量子化し Shannon 多様性指数 H を計算
- H_max = log(100) で正規化
- 旧パス `compute_capability_coverage_shannon` と同名衝突なし

### `compute_reuse_ratio_from_pair_counts`
- reciprocity_pair_counts から頻度 >= 2 のペア比率を計算
- 旧パス `compute_reuse_ratio` との名称衝突回避のため接尾辞 `_from_pair_counts`

### `compute_knowledge_diffusion_from_pair_counts`
- ユニークペア数 / 全インタラクション数で knowledge diffusion のプロキシ値
- 旧パス `compute_knowledge_diffusion_rate` との名称衝突回避のため接尾辞 `_from_pair_counts`

## collect_final_metrics 更新箇所

- `capability_coverage: 0.0` → `compute_capability_coverage(&ctx.positions)`
- `reuse_ratio: 0.0` → `compute_reuse_ratio_from_pair_counts(&ctx.reciprocity_pair_counts)`
- `knowledge_diffusion_rate: 0.0` → `compute_knowledge_diffusion_from_pair_counts(&ctx.reciprocity_pair_counts)`

## テスト結果

- D1-D8 全 PASS
- 既存テスト 1267 件全 PASS（回帰なし）
- D7 観測テスト値: capability_coverage=0.802, reuse_ratio=0.056, knowledge_diffusion_rate=0.947

## Acceptance Criteria 達成状況

1. ✅ capability_coverage が 0.0 から 0.802 に改善
2. ✅ reuse_ratio が 0.0 から 0.056 に改善
3. ✅ knowledge_diffusion_rate が 0.0 から 0.947 に改善
4. ✅ SimulationContext への新規フィールド追加なし
5. ✅ 既存テスト全 PASS
6. ✅ s_density が 0.5 となり 0.30 を超過
