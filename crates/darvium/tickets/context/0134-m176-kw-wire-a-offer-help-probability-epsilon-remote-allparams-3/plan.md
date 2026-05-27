# WIRE-A (Ticket #134) 実装計画

## 要件の再確認

1. constants.rs に 8 定数を追加: OFFER_HELP_BASE(0.3), OFFER_HELP_BV_COEFF(0.4), ADVANCE_HELP_ACCEPT_BASE(0.5), ADVANCE_HELP_ACCEPT_BV_COEFF(0.3), ADVANCE_HELP_SUCCESS_BASE(0.6), ADVANCE_HELP_SUCCESS_BV_COEFF(0.25), ADVANCE_HELP_HARMFUL_BASE(0.15), ADVANCE_HELP_HARMFUL_BV_COEFF(0.1)
2. kind_world.rs G3 インデックス定数を追加 (G3_OFFER_HELP_BASE = 17 〜 G3_ADVANCE_HELP_HARMFUL_BV_COEFF = 24)
3. simulation.rs ReciprocitySimulatorConfig に 8 フィールド追加
4. offer_help_probability() シグネチャ変更 + compute_benevolence_aware_remote_exploration 経由化
5. advance_help_sessions() を定数参照に変更
6. default_g1g2g4() の G3 を 0.0 → constants.rs 値に変更 + active=true
7. to_sim_config_g1g2g4() に G3 反映を追加
8. テスト A1-A8 追加

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|---------|------|------|
| src/constants.rs | 追加 | WIRE-C ブロック直後に HELP 確率 8 定数 |
| src/simulation.rs | 修正 | Config 8 フィールド追加 + Default impl + offer/advance 関数修正 + 呼び出し元修正 + A1-A8 テスト |
| src/kind_world.rs | 修正 | G3 インデックス定数 + default_g1g2g4 G3 更新 + to_sim_config_g1g2g4 G3 伝播 |

## 実装手順

1. constants.rs: WIRE-C ブロック直後に 8 定数追加
2. kind_world.rs: G3 インデックス定数追加 (G3_COUNT=8 直後)
3. kind_world.rs: default_g1g2g4() の G3 を constants.rs 値に変更 + active=true
4. kind_world.rs: to_sim_config_g1g2g4() に G3 伝播を追加
5. simulation.rs: ReciprocitySimulatorConfig に 8 フィールド追加 + Default impl
6. simulation.rs: offer_help_probability() 書き換え (epsilon_remote 経由化)
7. simulation.rs: offer_help_sessions() 呼び出し元変更 (policy, vmb, child_need 追加)
8. simulation.rs: advance_help_sessions() を定数参照に変更
9. simulation.rs: テスト A1-A8 追加
10. ビルド・テスト・clippy

## 計装・観測の実装計画

- A1-A5, A7: 固定値の assert テスト (PASS/FAIL のみ)
- A6: run_simulation 経由の観測テスト (固定 seed で結果再現性確認)
- A8: cargo test 全テスト通過確認
- 観測テスト: epsilon_remote 導入前後の offer_help_probability 分布比較 (println! + --nocapture, n=10,000)

## Boy Scout 改善

- offer_help_probability() シグネチャ改善 (翻訳可能性向上)
- advance_help_sessions() の 6 マジックナンバーを名前付き定数経由に置き換え

## 物理的レビュー方法

run-quality-checks.js on 3 files + translate-ability grep + cargo test

## リスク

- offer_help_probability() 挙動変更で既存テストの HELP セッション数が変動する可能性
- child_need=0.0 は WIRE-D までの暫定値
