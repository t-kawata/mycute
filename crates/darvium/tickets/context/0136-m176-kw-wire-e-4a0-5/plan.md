## WIRE-E 実装計画

### 要件の再確認

WIRE-A/C/D で未対処の残余ハードコード値を simulation.rs の Phase3/4/5/6 から抽出し、constants.rs の名前付き定数化 + AllParams G6 グループ追加 + G1 active=false 設定 + 4A.0 カタログ更新。

### 変更ファイル一覧

| ファイル | 種別 | 内容 |
|---------|------|------|
| `src/constants.rs` | 定数追加 | Phase3/4/5/6 用制御パラメーター定数 10 個 + フォールバック定数追加 |
| `src/kind_world.rs` | 構造体拡張 | G6_COUNT=10 + G6_* インデックス定数追加。`default_g1g2g4()` 更新。`to_sim_config_g1g2g4()` 更新。G1 active=false 設定 |
| `src/simulation.rs` | 値置換 | Phase3/4/5/6 のハードコード数値リテラルを定数参照に置き換え |
| `Darvium-RFC-0001-Unified-v2.3-final.md` | カタログ更新 | 4A.0 の (H) エントリを更新 |

### G6 パラメーター一覧

| G6 idx | 定数名 | 値 | 分類 |
|--------|--------|-----|------|
| 0 | PHASE3_HELP_LOAD_LEVEL | 0.3 | 制御パラメーター |
| 1 | PHASE3_HELP_RISK_LEVEL | 0.2 | 制御パラメーター |
| 2 | PHASE3_HELP_UNCERTAINTY | 0.3 | 制御パラメーター |
| 3 | PHASE3_HELP_AUTONOMY_COST | 0.2 | 制御パラメーター |
| 4 | PHASE3_SUCCESS_BV_COEFF | 0.5 | 制御パラメーター |
| 5 | PHASE3_SUCCESS_BASE | 0.3 | 制御パラメーター |
| 6 | PHASE4_FRESHNESS_HUMAN_WEIGHT | 0.5 | 制御パラメーター |
| 7 | PHASE4_LIFECYCLE_SUCCESS_STUB | 0.5 | 制御パラメーター |
| 8 | PHASE4_CHILD_PROT_VALUE | 0.5 | 制御パラメーター |
| 9 | PHASE5_REPUTATION_INHERIT_DECAY | 0.7 | 制御パラメーター |

### 計装・観測の実装計画

- テスト: simulation.rs 内に E1-E8 追加
- 観測: `println!` + `--nocapture`
- 較正: PHASE4_LIFECYCLE_SUCCESS_STUB の感度確認

### 実装手順

1. constants.rs に定数追加
2. kind_world.rs に G6 グループ追加 + G1 active=false
3. simulation.rs の値を定数置換
4. cargo build + cargo test 確認
5. RFC カタログ更新
6. 観測テスト + 品質チェック + done 遷移
