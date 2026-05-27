# 計画: M1.76-KW4-CAL: Kind World 較正継続（外側ループ 2-24）

## 要件の再確認

1. evaluate_single を ReciprocitySimulator → SimulationContext（KW-REAL 6 フェーズ）に移行
2. collect_final_metrics_from_result → collect_final_metrics（SimulationContext 版）に切り替え
3. TC9（決定論的検証）+ TC10（6 指標非ゼロ検証）を追加
4. SimulationContext 移行後、外側ループ（最大 23 サイクル）を実行
5. experiments.md に各サイクル結果を追記

## RFC 既存実装状態検証

### RFC §15.9.2 KindWorldAssessment
| フィールド | RFC の型 | 現行コードの型 | 状態 |
|---|---|---|---|
| is_kind_world | bool | bool | ✅ 一致 |
| j_kw | f64 | f64 | ✅ 一致 |
| s_growth/s_density/s_topology/s_search/s_fairness | f64 | f64 | ✅ 一致 |
| 20 下位成分 (j_pop_growth..j_reasoning_steps_inv) | f64 | f64 | ✅ 一致 |
| legacy_flags | [bool; 8] | [bool; 8] | ✅ 一致 |

### RFC §15.10.9 Phase 3（評価経路）
| 観点 | RFC 要求 | 現行 | 状態 |
|---|---|---|---|
| シミュレーション方式 | 合成生態系 / SimulationContext | ReciprocitySimulator | ❌ 不一致 |
| J_kw 評価 | 全 20 指標から 5 因子乗算結合 | 14/20 指標（6 指標 0.0 fallback） | ❌ 不完全 |
| 決定論的再現性 | 必須 | ✅ 固定シード | ✅ 一致 |

**評価サマリ**: KindWorldAssessment 構造体は RFC §15.9.2 と完全一致。evaluate_single の評価経路のみ RFC §15.10.9 Phase 3 と乖離。本チケットで修正する。

## 変更ファイル一覧

| ファイル | 種別 | 内容 |
|---|---|---|
| `src/kind_world.rs` | 修正 | evaluate_single を SimulationContext 版に書き換え |
| `src/kind_world.rs` | 修正 | collect_final_metrics の #[allow(dead_code)] 除去 |
| `src/kind_world.rs` | 追加 | TC9（決定論的検証）+ TC10（6 指標非ゼロ検証） |
| `src/kind_world.rs` | 修正 | tc6_kw4_optimize_run 出力に 5 因子内訳を追加 |
| `tickets/context/0120-m176-kw4-cal-kind-world-2-24/experiments.md` | 作成 | 実験ログ |
| `src/constants.rs` | 修正（可能性） | SimulationContext 移行後の J_kw 絶対値変化に応じた探索範囲調整 |

## 計装・観測の実装計画

### 実装するテストコード
- **TC9** (`src/kind_world.rs`): evaluate_single を 2 回連続で同一パラメータ+同一シードで呼び出し、J_kw が完全一致することを確認
- **TC10** (`src/kind_world.rs`): evaluate_single を実行後、内部的に collect_final_metrics を呼び出し 6 指標が非ゼロであることを確認
- **tc6_kw4_optimize_run**: 既存テスト、SimulationContext 移行後も同じく動作。出力に 5 因子内訳を追加

### --nocapture での観測出力
```bash
cargo test tc6_kw4_optimize_run -- --nocapture
```

### 観測すべき統計量
- 各 iteration の J_kw 値、5 因子値（s_growth〜s_fairness）、パラメータ 7 種
- 収束判定結果（converged, iterations）
- 最良パラメータの 5 因子内訳と最小値ゲート成立状況
- 20 下位成分全値

### 較正対象と目的関数
- **較正対象**: MagnificentSevenParams（7 次元）
- **探索範囲**: KW4_GAMMA_BENEVOLENCE_RANGE 等 7 定数
- **目的関数 J(θ)**: compute_kind_world_objective の j_kw（5 因子乗算結合）
- **内側ループ停止条件**: シンプレックス頂点間 J_kw 分散 < 1e-6 または 200 iteration
- **外側ループ停止条件**: 24 サイクルまたは J_kw > 0.8 ∧ min(s_i) > 0.6

### 較正ループ停止条件
- 外側 24 サイクル到達 → 最終報告
- J_kw > 0.8 かつ全 5 因子 > 0.6 → Kind World 達成（早期終了）

## Boy Scout 改善（スコープ外の翻訳可能性修正）

- `collect_final_metrics_from_result`（kind_world.rs:2068）: 13 指標がハードコード 0.0 のまま。本チケットでは SimulationContext 版に移行するため直接修正しないが、関数コメントに「この関数は ReciprocitySimulator 用の後方互換インターフェース。新規コードは collect_final_metrics（SimulationContext 版）を使用すべき」と注記を追加
- `collect_final_metrics`（SimulationContext 版, kind_world.rs:2147）: `#[allow(dead_code)]` 除去
- `to_sim_config`（kind_world.rs:1849）: 引数 hardcode の 50 人口を定数参照に変更しない（本チケットスコープ外だが、将来の改善点として experiments.md に記録）

## 実装手順

### Step 1: evaluate_single の SimulationContext 移行

```rust
// 現在（ReciprocitySimulator）:
fn evaluate_single(params: &MagnificentSevenParams, seed: u64) -> f64 {
    let config = params.to_sim_config(50, seed);
    let result = crate::simulation::run_simulation(&config);
    let metrics = collect_final_metrics_from_result(&result, config.population_size);
    compute_kind_world_objective(&metrics).j_kw
}

// 移行後（SimulationContext）:
fn evaluate_single(params: &MagnificentSevenParams, seed: u64) -> f64 {
    let mut rng = StdRng::seed_from_u64(seed);
    // SimulationContext を構築して 6 フェーズシミュレーションを実行
    // collect_final_metrics で全 20 指標を収集
    // compute_kind_world_objective で J_kw 計算
}
```

SimulationContext の構築には、memoized_graph（初期グラフ）、config（MagnificentSevenParams から変換）、rng（固定シード）が必要。KW-REAL-P4（ticket 115）の 6 フェーズループを使用する。

### Step 2: collect_final_metrics の #[allow(dead_code)] 除去

collect_final_metrics（SimulationContext 版）は evaluate_single から呼ばれるようになるため、dead_code 属性を削除する。

### Step 3: TC9・TC10 追加

TC9: 同一パラメータ + 同一シードで同一 J_kw が得られることを確認
TC10: collect_final_metrics 経由の 6 指標が 0.0 でないことを確認

### Step 4: cargo build/clippy/test 通過確認

```bash
cargo build
cargo clippy -- -D warnings
cargo test
```

### Step 5: 観測テスト初回実行

```bash
cargo test tc6_kw4_optimize_run -- --nocapture
```

SimulationContext 移行後、J_kw がどう変化したかを確認。出力に 5 因子内訳（s_growth〜s_fairness）が含まれていることを確認。

### Step 6: experiments.md 作成 + 初回エントリ記録

### Step 7: 外側ループ継続

## 物理的レビュー方法

```bash
# 静的品質チェック
_R=$(cat DARVIUM_PLUGIN_ROOT.md)
node "$_R/scripts/tickets/review/run-quality-checks.js" src/kind_world.rs src/constants.rs | node "$_R/scripts/tickets/review/generate-report.js"

# 翻訳可能性 grep
grep -rn 'fn [a-z].*_' src/kind_world.rs | grep -v 'test'  # 名詞始まり関数チェック（テスト除く）
grep -rn 'let [a-z]_[a-z]' src/kind_world.rs | grep -v 'test\|fn\|for\|impl'  # 1文字/汎用変数チェック

# テスト
cargo test
cargo clippy -- -D warnings
```

## リスク

1. **SimulationContext 移行に伴う J_kw の急変**: 移行後、J_kw の絶対値が大きく変わる可能性がある。探索範囲の再調整が必要。
2. **SimulationContext 構築コスト**: ReciprocitySimulator より重い可能性がある。内側ループの iteration あたり実行時間が増加する場合は、KW4_SIMULATION_TICKS の調整が必要。
3. **決定論的再現性の維持**: SimulationContext 内で非決定論的な要素が混入していないか、TC9 で厳密に確認する。
4. **既存テストとの競合**: TC1-TC8 は ReciprocitySimulator 用に設計されている。evaluate_single 移行後も TC4（決定論的検証）と TC6（最適化実行）が正しく動作することを確認。
