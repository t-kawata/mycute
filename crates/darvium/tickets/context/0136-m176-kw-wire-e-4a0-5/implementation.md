# 実装サマリ: M1.76-KW-WIRE-E: 残余ハードコード値の全数パラメーター化

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|---------|------|------|
| `src/constants.rs` | 新規定数 | 25 定数追加: PHASE3_* (7), PHASE4_* (6), PHASE5_* (1), PHASE6_* (11) |
| `src/kind_world.rs` | 修正 | G6 グループ (10 インデックス定数), G1 3 フラグ active=false, default_g1g2g4/to_sim_config_g1g2g4 更新 |
| `src/simulation.rs` | 修正 | Phase3/4/5/6 の全ハードコード値を定数置換 + E1-E9 観測テスト追加 |
| `Darvium-RFC-0001-Unified-v2.3-final.md` | 文書 | 4A.0 カタログ (H)→(U) 7件変更, #92 記述修正, サマリ更新 |

## 変更の分類

### G6 制御パラメーター (10, AllParams インデックス 29-38)
全て [0.0, 1.0] 範囲で較正可能。KW-REAL パスで constants.rs 直接参照のため Config 未統合。

| 定数 | 値 | 影響 |
|------|-----|------|
| PHASE3_HELP_LOAD_LEVEL | 0.3 | Phase3 should_offer_help 負荷水準 |
| PHASE3_HELP_RISK_LEVEL | 0.2 | Phase3 should_offer_help リスク水準 |
| PHASE3_HELP_UNCERTAINTY | 0.3 | Phase3 decide_help_offer 不確実性 |
| PHASE3_HELP_AUTONOMY_COST | 0.2 | Phase3 decide_help_offer 自律コスト |
| PHASE3_SUCCESS_BV_COEFF | 0.5 | Phase3 成功確率の慈悲係数 |
| PHASE3_SUCCESS_BASE | 0.3 | Phase3 成功確率のベース値 |
| PHASE4_FRESHNESS_HUMAN_WEIGHT | 0.5 | Phase4 fresh 成分の人時重み |
| PHASE4_LIFECYCLE_SUCCESS_STUB | 0.5 | Phase4 成功成分（P6 未完成スタブ） |
| PHASE4_CHILD_PROT_VALUE | 0.5 | Phase4 子供保護値 |
| PHASE5_REPUTATION_INHERIT_DECAY | 0.7 | Phase5 評判継承減衰係数 |

### Rust 定数のみ (15, AllParams 非対応)
- フォールバック定数 (4): PHASE3_HELPER_BENEVOLENCE_FALLBACK, PHASE3_QUALITY_FALLBACK, PHASE4_CHILD_PROT_ADULT, PHASE4_TRUST_FALLBACK, PHASE4_REPUTATION_FALLBACK
- Phase6 スタブ定数 (11): PHASE6_CAPABILITY_COVERAGE_STUB 等、全 0.5（デバッグ出力用）

### G1 active=false (3)
- G1_SEARCH_TICK_FRACTION, G1_EVALUATE_FRACTION, G1_REMOTE_EXPLORE_HUMAN_WEIGHT

### 4A.0 カタログ更新
- (H)→(U): #14, #15, #36, #62, #63, #64, #92
- #92 記述修正: 「永久スタブ」→「空セッションフォールバック。実関数は L2 距離計算」
- 残存 (H): #16 (s_growth, scope外), #79 (s_density, scope外), #80 (s_density, scope外)
