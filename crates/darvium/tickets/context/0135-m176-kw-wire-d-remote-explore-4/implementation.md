# 実装サマリ: WIRE-D — REMOTE_EXPLORE_* 定数のシミュレーション実装

## 変更したファイル一覧

| ファイル | 種別 | 内容 |
|---------|------|------|
| `src/constants.rs` | 追加 | `LOCAL_HELP_BOOST: f64 = 1.0` |
| `src/kind_world.rs` | 追加 | G5 グループ（G5_COUNT=1, G5_LOCAL_HELP_BOOST=28） |
| `src/simulation.rs` | 修正 | Config + 3 補助関数 + offer_help_sessions + テスト D1-D7 |

## 実装内容の概要

### constants.rs
- `LOCAL_HELP_BOOST: f64 = 1.0` 追加 — 村内 offer 確率のブースト係数

### kind_world.rs
- G5_COUNT=1, G5_LOCAL_HELP_BOOST=28 追加
- `default_g1g2g4()` に G5 セクション追加（active=true）
- `to_sim_config_g1g2g4()` に G5→sim_config 伝播追加

### simulation.rs
- `ReciprocitySimulatorConfig` に `local_help_boost: f64` 追加
- `compute_village_mean_benevolence()`: 指定村の平均慈悲スコア計算
- `compute_child_need_in_village()`: 指定村内の is_child 割合計算
- `offer_help_sessions()`: `village_assignments` 引数追加
  - 村内: `(base_prob + epsilon_remote) * local_help_boost`
  - 村外: `epsilon_remote`（探索的）
  - `village_assignments=None`: 全ノード村内扱い（後方互換）
- `run_simulation()` 呼び出し元: None を渡す修正
- テスト D1-D6 追加（固定 seed assert + n>=1000 Monte Carlo）
- A6 テスト修正: epsilon_remote 成分も 0 に設定

## テスト結果
- 全 1328 テスト PASS（0 failed, 17 ignored）
- D7 確認: 既存テスト全 PASS
