# 実装サマリ: 首長性スコア導入 (#147)

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|---|---|---|
| `src/constants.rs` | 追加 | `CHIEFDOM_DEPTH_SCALE = 3.0` 定数 |
| `src/event.rs` | 追加 | `ReputationProfile.chiefdom_score: f32` フィールド + cold_start 初期化 |
| `src/graph_query.rs` | 追加 | 4関数 + ヘルパー + T1-T15 テスト |
| `src/simulation.rs` | 追加 | Phase 3.7/3.8、elect_village_chiefs、payload 拡張 |
| `src/reciprocity.rs` | 修正 | ReputationProfile 初期化子に chiefdom_score 追加 |
| `src/trust.rs` | 修正 | 同上 |
| `src/lib.rs` | 修正 | 同上 |
| `web/cube/observation/index.html` | 追加 | 首長性中央値・首長数表示行 |
| `web/cube/observation/script.js` | 追加 | chiefdom_score 受信・集計・首長黒色描画 |

## 実装の詳細

### 関数一覧（graph_query.rs）

- `compute_abstraction_ratio` — 抽象化割合を計算し (ratio-1)/ratio で [0,1] 正規化
- `calculate_max_nest_depth` — 最大 SubWorkflow ネスト深度（visited set で循環保護）
- `compute_abstraction_depth` — 深度を depth/(depth+SCALE) で [0,1] 正規化
- `compute_sophistication_score` — 洗練スコア (ratio + depth) / 2
- `compute_chiefdom_score` — 首長性スコア 0.5 * final_score + 0.5 * sophistication

### Phase 実行順序

Phase 3.5 (評判再計算) → Phase 3.7 (首長性スコア) → Phase 3.8 (首長選出) → Phase 3.6 (自己抽象化)

### テスト結果

1441 tests passed, 0 failed, 0 regressions.
