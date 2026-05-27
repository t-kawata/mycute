# 実装計画: M1.76-KW-FIX-C — HELP 任意ペア化 + Proposal 生成機構

## RFC 既存実装状態検証

### RFC §41B-9 HelpProposal 条件式
| 観点 | RFC §41B-9 | 現行コード | 状態 |
|------|-----------|-----------|------|
| 提案条件 | Child(c) ∧ Adult(h) | adult→child 限定 | ❌ Layer 2（RFC の設計矛盾） |
| 実装制限 | adult helper, child helpee | simulation.rs:1779-1796 | ❌ Layer 1（実装バグ） |
| θ_proposal 閾値 | Q ≥ θ_proposal | mission_rate 閾値のみ | ⚠️ 簡略化 |

**評価**: FIX-C は Layer 1（実装バグ）を修正し RFC から意図的に逸脱する（Layer 2 ワークアラウンド）。

### RFC §41B.20.1 F-11 Helper Quality Score
| 観点 | RFC | 現行コード | 状態 |
|------|-----|-----------|------|
| 式 | Q = w_s·S + w_t·T(h) + w_r·Rep(h) + w_b·B(h) + w_n·N(c) - w_d·d | quality = benevolence_score | ⚠️ 簡略化 |
| バイアス方向 | helper 側バイアス | helper 側バイアス | ❌ Layer 3（本チケットで提案生成側に helpee バイアスを移譲） |

### RFC §15.9.2 s_topology / j_reciprocity
| 観点 | RFC | 現行コード | 状態 |
|------|-----|-----------|------|
| s_topology 式 | 6 成分算術平均 | kind_world.rs:437 | ✅ 完全一致 |
| j_reciprocity | mean_reciprocity_score | kind_world.rs:411 | ✅ 完全一致 |
| mean_reciprocity | 双方向ペア割合 | kind_world.rs:2254-2273 | ✅ 完全一致（入力が常に 0） |

## 要件の再確認
phase3_help_protocol のハードフィルタ（helper=成人、helpee=子供）を撤廃し、全 alive ノードから任意ペアを選択。子供 helpee バイアス機構を追加。

## 変更ファイル一覧
| ファイル | 種別 | 内容 |
|---------|------|------|
| src/constants.rs | 追加 | CHILD_HELPEE_BIAS_FACTOR: f64 = 2.0 |
| src/simulation.rs | 変更 | phase3_help_protocol: ハードフィルタ撤廃 + 任意ペア選択 + 子供 helpee バイアス |
| src/simulation.rs | 追加 | FIX-C1〜C8 テスト |

## 計装・観測の実装計画
- 不変条件: C1(adult→adult), C2(child→child), C3(child→adult), C4(adult→child), C5(双方向), C6(reciprocity>0), C8(既存PASS)
- 観測: C7(4方向HELP割合 + 子供helpeeバイアス確認)
- 固定シード: StdRng::seed_from_u64(12345)

## Boy Scout 改善
1. phase3_help_protocol の proposal 生成ブロックを generate_help_proposals() として独立関数抽出
2. 生存ノードフィルタリングを collect_alive_node_ids() ヘルパー関数に抽出
3. バイアス係数を CHILD_HELPEE_BIAS_FACTOR として定数化

## 実装手順
1. constants.rs: CHILD_HELPEE_BIAS_FACTOR 追加
2. simulation.rs: phase3_help_protocol 修正（任意ペア + バイアス）
3. cargo test で既存テスト PASS 確認
4. simulation.rs: FIX-C1〜C8 テスト追加
5. cargo test + cargo test fixc_observe -- --nocapture
6. cargo clippy
7. 品質チェック（run-quality-checks.js）
8. 観察レポート保存 → done 遷移

## 物理的レビュー方法
_R=$(cat DARVIUM_PLUGIN_ROOT.md)
node "$_R/scripts/tickets/review/run-quality-checks.js" src/constants.rs src/simulation.rs | node "$_R/scripts/tickets/review/generate-report.js"

## リスク
- 任意ペア化で子供保護が弱まる → 子供 helpee バイアスで補償
- 既存テスト期待値変動 → C8 で確認
- j_reciprocity 改善後も他の因子がボトルネック → 観測で確認
