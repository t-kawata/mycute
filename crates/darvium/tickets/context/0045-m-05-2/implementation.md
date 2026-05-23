# M-0.5-2: ランクドリフト頑健性シミュレーションテスト — 実装サマリー

## 変更ファイル

| ファイル | 種別 | 内容 |
|---|---|---|
| Cargo.toml | 依存変更 | rand を [dependencies] に移動 |
| src/constants.rs | 定数追加 | NOISE_SIMULATION_SIGMA = 0.05 |
| src/lib.rs | モジュール登録 | pub mod search; 追加 |
| src/search/mod.rs | 新規作成 | 探索モジュール宣言 |
| src/search/simulated_ranker.rs | 新規作成 | ノイズ注入・ランクドリフトシミュレーション・全テスト |

## 実装関数

- inject_gaussian_noise — Box-Muller 法によるガウスノイズ注入
- compute_blended_score — RFC §12.2 統合式 (α=0.35)
- select_top_candidate — 安定最大値選択（同値時先頭優先）
- simulate_rank_drift — ドリフトシミュレーション（コアロジック）
- compute_mean_squared_displacement — MSD 系列計算
- estimate_hurst_exponent — log-log 回帰によるハースト指数推定

## テスト結果

- T1-T7: 全通過
- OTS-1 (MSD 解析): 全ノイズ水準で H ≤ 0.5 確認
- OTS-2 (トップ1一致率): σ=0.01 で 100%、単調減少確認
- 全 289 テスト通過, clippy OK, fmt OK

## 特記事項

- rand_distr は rand 0.9 と非互換のため Box-Muller 自前実装で代替
