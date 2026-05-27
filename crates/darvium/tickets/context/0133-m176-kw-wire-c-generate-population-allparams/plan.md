# M1.76-KW-WIRE-C: 生成時定数のパラメーター化 — 実装計画

## 要件の再確認

`generate_population()` の 3 ハードコード値（子供 trust 上限 0.3、成人 trust 下限 0.3、benevolent 閾値 0.5）を constants.rs 定数化し、AllParams G4 として較正可能にする。`ReciprocitySimulatorConfig` 拡張 + `to_sim_config_g1g2g4` で較正ループに統合。

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|---------|------|------|
| `src/constants.rs` | 編集 | SIMULATION_CHILD_TRUST_MAX, SIMULATION_ADULT_TRUST_MIN, SIMULATION_BENEVOLENT_THRESHOLD を KW4 ブロック直後に追加 |
| `src/simulation.rs` | 編集 | ReciprocitySimulatorConfig に 3 フィールド追加。generate_population() の 467行・492行を定数参照に変更。observe_tick() の 4 箇所の 0.5 を定数参照に変更。Default impl 更新。 |
| `src/kind_world.rs` | 編集 | G3_COUNT=8, G4_COUNT=3, G4_CHILD_TRUST_MAX(25), G4_ADULT_TRUST_MIN(26), G4_BENEVOLENT_THRESHOLD(27) を追加。default_g1() に G4 デフォルト追加。to_sim_config_g1g2g4() 追加。 |

## RFC 既存実装状態検証

**該当 RFC セクション**: §41C（シミュレーション実行マイルストーン追記） — 実装レベルの構造体定義なし。§41B.20.9（Kind World 較正メトリクス）— J_kw 目的関数の定義のみで、シミュレーション内部構造の規定なし。

**評価**: RFC はシミュレーション内部実装（ReciprocitySimulatorConfig, generate_population, observe_tick）に対して構造的制約を課していない。したがって構造比較は不要。本チケットの変更は全て「シミュレーションの全初期条件を較正対象として露出する」という §41C の基本方針に合致する。

## 計装・観測の実装計画

1. **不変条件テスト**（C1-C5, simulation.rs `mod tests`）
   - `test_wirec_c1_child_trust_max_zero`: SIMULATION_CHILD_TRUST_MAX=0.0 生成で子供 trust=0.0 を assert
   - `test_wirec_c2_adult_trust_min_one`: SIMULATION_ADULT_TRUST_MIN=1.0 生成で成人 trust=1.0 を assert
   - `test_wirec_c3_threshold_one_all_non_benevolent`: BENEVOLENT_THRESHOLD=1.0 で全 tick で total_benevolent=0 を assert
   - `test_wirec_c4_threshold_zero_all_benevolent`: BENEVOLENT_THRESHOLD=0.0 で全 tick で total_non_benevolent=0 を assert
   - `test_wirec_c5_deterministic`: 同一 seed 2 回の generate_population 結果がビット一致を assert
   - `test_wirec_c6_existing_tests_pass`: cargo test で確認

2. **観測テスト**（--nocapture 出力）
   - 生成 population の trust 分布を子供/成人別に 5 数要約を println 出力
   - 固定シード StdRng::seed_from_u64(12345)

3. **較正計画**: 本チケット単独では較正しない。WIRE 全チケット完了後に統合較正。

## Boy Scout 改善

- observe_tick() の benevolent 分類処理: 4 箇所の 0.5 を定数に統一
- generate_population() の 0.3/0.5: 成人 trust 範囲幅 0.5 が構造的固定値である理由をコメント明記

## 実装手順

1. constants.rs: 3 定数を追加
2. simulation.rs: ReciprocitySimulatorConfig 拡張 + Default 更新
3. simulation.rs: generate_population() 修正
4. simulation.rs: observe_tick() 修正
5. kind_world.rs: G3_COUNT, G4 定数 + default 追加
6. kind_world.rs: to_sim_config_g1g2g4() 追加
7. simulation.rs tests: C1-C5 追加
8. cargo test 全 PASS 確認

## 物理的レビュー方法

```bash
_R=$(cat DARVIUM_PLUGIN_ROOT.md)
node "$_R/scripts/tickets/review/run-quality-checks.js" src/constants.rs src/simulation.rs src/kind_world.rs | node "$_R/scripts/tickets/review/generate-report.js"
```

翻訳可能性チェック: 新たなマジックナンバー混入の確認。構造整合性チェック（validate-structure.js）。

## リスク

- G3_COUNT 前方定義: WIRE-A 未実装のため G3_COUNT=8 を先に定義。G4 は G1+G2+G3 から派生するため自動追従。
- observe_tick の constants 直接参照: config 経由より影響範囲が小さいが、AllParams 経路にはならない。将来検討課題。
