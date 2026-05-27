# 実装サマリ: M1.76-KW-WIRE-A (#134)

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|---------|------|------|
| src/constants.rs | 追加 | WIRE-C ブロック直後に HELP 確率 8 定数を追加（OFFER_HELP_BASE〜ADVANCE_HELP_HARMFUL_BV_COEFF） |
| src/simulation.rs | 修正 | ReciprocitySimulatorConfig に 8 フィールド追加 + Default impl + offer_help_probability 書き換え（epsilon_remote 経由化）+ advance_help_sessions 定数化 + compute_mean_benevolence 追加 + A1-A7 テスト + 7箇所の struct リテラル修正 |
| src/kind_world.rs | 修正 | G3 インデックス定数 8 個追加（G3_OFFER_HELP_BASE=17〜G3_ADVANCE_HELP_HARMFUL_BV_COEFF=24）+ default_g1g2g4() の G3 を constants.rs 値に変更 + to_sim_config_g1g2g4() に G3 伝播追加 + 既存 clippy 警告修正（needless_range_loop） |

## 実装内容

1. **constants.rs**: 8 定数（OFFER_HELP_BASE=0.3, OFFER_HELP_BV_COEFF=0.4, ADVANCE_HELP_ACCEPT_BASE=0.5, ADVANCE_HELP_ACCEPT_BV_COEFF=0.3, ADVANCE_HELP_SUCCESS_BASE=0.6, ADVANCE_HELP_SUCCESS_BV_COEFF=0.25, ADVANCE_HELP_HARMFUL_BASE=0.15, ADVANCE_HELP_HARMFUL_BV_COEFF=0.1）
2. **kind_world.rs**: G3 インデックス定数（17-24）。default_g1g2g4() の G3 を 0.0 から constants.rs 値に変更 + active=true。to_sim_config_g1g2g4() に G3→sim_config 伝播
3. **simulation.rs**: ReciprocitySimulatorConfig に 8 フィールド追加。offer_help_probability() を epsilon_remote 経由に書き換え（compute_benevolence_aware_remote_exploration 呼び出し追加）。advance_help_sessions() の 6 マジックナンバーを config フィールド参照に置き換え。7箇所の struct リテラルに新フィールド追加。A1-A7 テスト追加（epsilon=0 clamp, 単調増加, accept/success/harmful 制御, G3 伝搬, max clamp）
4. **Boy Scout**: kind_world.rs の needless_range_loop を iterator に修正

## テスト結果
- A1-A7: 全 PASS
- 既存テスト 1313: 全 PASS（0 failed, 8 ignored）
- clippy: -D warnings クリーン
- 品質チェック: 303 件 existing issues（全件既存コード由来、新規 0）
