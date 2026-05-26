---
ticket_id: 112
title: M1.76-KW4: Kind World 較正ループ実行
slug: m176-kw4-kind-world
status: done
created_at: 2026-05-26
updated_at: 2026-05-26
experiment_cycle: 0
experiment_count: 0
experiment_log: tickets/context/0112-m176-kw4-kind-world/experiments.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0112-m176-kw4-kind-world/observation-20260526-171302.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0112-m176-kw4-kind-world/implementation.md
---

# M1.76-KW4: Kind World 較正ループ実行

## Summary

Kind World 較正ループは二重構造を持つ：

**内側のループ（自動最適化）**: 1 回の `cargo test` の中で、Nelder-Mead 直接探索法により約 100〜160 回のシミュレーションを実行し、$J_{kw}$ を最大化する 7 パラメータセットに収束する。これは完全に自動で進行し、AI の介入は不要。

**外側のループ（実験者主導）**: AI（Claude）が内側ループの結果を解釈し、探索範囲や定数を調整して次の `cargo test` を実行する。8 回ごとに平易な日本語で中間報告を生成し、ユーザーの指示を仰ぐ。最大 24 サイクル（3 サイクル × 8 回）で打ち切り。

$J_{kw} > 0.8$ かつ全 8 条件成立をもって Kind World 達成と判定し、最終結果を Human review queue に配送する。

## Background

- **RFC §15.10.9**: 較正フェーズ Phase 3-4。Kind World の成立条件を探索する。
- **RFC §41C.3**: M4.x — 較正ループの位置づけ。
- **既存実装**: KW1 (ticket 109, `done`) — `compute_kind_world_objective`、`MagnificentSevenParams`、`KindWorldMetricsInput`、`KindWorldAssessment`。KW2 (ticket 110, `reviewed`) — `EcosystemGrowthObserver`。KW3 (ticket 111, `reviewed`) — `VillageInteractionObserver`。
- **なぜ Nelder-Mead か**: 7 次元の連続最適化問題において、導関数不要、実装が軽量（約 80-100行）、追加クレート不要。各反復が前回の結果に依存する逐次処理であり、並列化不能だが、内側ループ内ではこれで十分である。

### 参照観察レポート

- `tickets/context/0111-m176-kw3/observation-20260526-162259.md` — M1.76-KW3 村間相互作用計装の観測結果。
- `tickets/context/0110-m176-kw2/observation-20260526-160526.md` — M1.76-KW2 エコシステム成長メトリクス観測結果。
- `tickets/context/0109-m176-kw1-kind-world-j-kw/observation-20260526-154455.md` — M1.76-KW1 J_kw 目的関数実装状況。

## Investigation

### ソースコード調査結果

- `src/kind_world.rs`: `MagnificentSevenParams` (L90-L105)、`KindWorldMetricsInput` (L20-L39)、`KindWorldAssessment` (L68-L78)、`compute_kind_world_objective` (L165-L261)、`EcosystemGrowthObserver` (L574-L625)、`VillageInteractionObserver` (L982-L1131) — すべて実装済み。Nelder-Mead 最適化器、`kw4_optimize` テスト関数、`ExperimentRecord` は未実装。
- `src/calibration.rs`: `run_simulation` (simulation.rs)、`ReciprocitySimulatorConfig`、`apply_params_to_sim_config` (L1093-L1095) が利用可能。
- `src/constants.rs`: KW1-KW3 定数 (L971-L1035) が既存。KW4 では探索範囲定数（各パラメータの min/max）を追加する。

### 内側ループ設計（Nelder-Mead 法）

Nelder-Mead 法（シンプレックス法）は n 次元空間で n+1 個の頂点を持つシンプレックスを変形しながら最適点を探索する直接探索法。

**アルゴリズム概要（7 次元の場合、8 頂点での動作）:**

```
1. 初期シンプレックス: 中心(default値) + 7方向に微小変位した点 = 8点
2. 各頂点で evaluate(params) → J_kw を計算（1回のシミュレーション実行）
3. 悪い順に並べ替え: 最悪・次悪・最良
4. 最悪点を重心に対して「反射」→ 改善されれば「拡大」→ そこそこなら「収縮」→ 全部ダメなら「縮小」
5. 収束判定（頂点間の J_kw 分散 < ε）まで 2-4 を繰り返す
```

**評価関数 `evaluate` の中身:**

```rust
fn evaluate(params: &MagnificentSevenParams) -> f64 {
    let config = params.to_sim_config();       // MagnificentSevenParams → ReciprocitySimulatorConfig
    let result = run_simulation(&config);       // 200 tick のシミュレーション
    // 全 tick の metrics を収集し、最終状態で J_kw を計算
    let metrics = collect_final_metrics(&result);
    compute_kind_world_objective(&metrics).j_kw
}
```

## Scope

### 実装スコープ（コード）

1. **Nelder-Mead 最適化器**（kind_world.rs 内の新規モジュールまたは構造体）:
   - `NelderMeadOptimizer` 構造体
   - `fn new(params: &MagnificentSevenParams, ranges: &[(f64, f64); 7]) -> Self` — 7 パラメータ各々の探索範囲 (min, max) を指定
   - `fn run(&mut self, max_iterations: usize) -> OptimizationReport` — 収束までループ
   - 内部で `evaluate(params) -> f64` を繰り返し呼び出し
   - 最大反復回数を超えたら強制終了
   - 各 iteration の J_kw 履歴を保持

2. **`OptimizationReport` 構造体**（kind_world.rs, public）:
   - `best_params: MagnificentSevenParams` — 最良パラメータ
   - `best_j_kw: f64` — 最良 J_kw
   - `assessment: KindWorldAssessment` — 最良パラメータでの判定結果
   - `iterations: u32` — 実行反復数
   - `history: Vec<(MagnificentSevenParams, f64)>` — 全反復の履歴
   - `converged: bool` — 収束したかどうか
   - `experiment_id: String` — 実験 ID

3. **`kw4_optimize` テスト関数**（kind_world.rs, `mod tests`）:
   - `#[test]` 属性
   - 内側で `NelderMeadOptimizer::run()` を呼び出し
   - 各 iteration の J_kw とパラメータを標準出力に CSV 形式で逐次出力
   - 最終的な `OptimizationReport` を JSON 形式で出力
   - 収束判定結果と Kind World 成立判定を出力
   - `#[ignore]` は付けない（通常実行に含める）
   - 実行時間が問題になる場合は `#[cfg_attr(not(feature = "slow_tests"), ignore)]` で制御

4. **探索範囲定数**（constants.rs）:
   - `KW4_GAMMA_BENEVOLENCE_RANGE: (f64, f64) = (0.0, 0.8)`
   - `KW4_LAMBDA_GC_BASE_RANGE: (f64, f64) = (0.1, 2.0)`
   - `KW4_DIRECT_RECIPROCITY_WEIGHT_RANGE: (f64, f64) = (0.1, 0.8)`
   - `KW4_INDIRECT_RECIPROCITY_WEIGHT_RANGE: (f64, f64) = (0.1, 0.8)`
   - `KW4_SOFTMAX_TEMPERATURE_RANGE: (f64, f64) = (0.1, 5.0)`
   - `KW4_GC_INTERVAL_RANGE: (f64, f64) = (1.0, 10.0)`
   - `KW4_CHILD_RATIO_RANGE: (f64, f64) = (0.1, 0.5)`
   - `KW4_NELDER_MEAD_MAX_ITERATIONS: usize = 200`
   - `KW4_NELDER_MEAD_CONVERGENCE_EPSILON: f64 = 1e-6`
   - `KW4_NELDER_MEAD_INITIAL_PERTURBATION: f64 = 0.05`

5. **`ExperimentRecord` 構造体**（kind_world.rs, public）:
   - `experiment_id: String`, `experiment_cycle: u32`
   - `report: OptimizationReport`
   - `timestamp: String`（ISO 8601）

6. **`experiments.md` ログファイル**（context ディレクトリ）:
   - AI が各外側ループの結果を追記する Markdown ファイル
   - frontmatter で実験カウントを管理

### 二重ループの動作手順

**内側ループ（自動）:**

```
NelderMeadOptimizer::run()
  ├─ 初期 8 点を生成
  ├─ for iteration in 0..MAX_ITER {
  │    ├─ 次候補のパラメータを提案
  │    ├─ run_simulation(params)
  │    ├─ 全 observer で metrics 収集
  │    ├─ compute_kind_world_objective() → J_kw
  │    ├─ println!("ITER {},params={:.4?},J_kw={:.6}", ...)
  │    └─ Nelder-Mead 更新（反射/拡大/収縮/縮小）
  │      └─ 収束判定 → break
  }
  └─ 最良パラメータ + J_kw + 履歴を出力
```

**外側ループ（AI 主導）:**

```
=== 外側サイクル開始 (cycle=0, count=0) ===

[Step 1] 前回の OptimizationReport を解釈
  └─ 最良パラメータは収束したか？
  └─ J_kw 成分のうち弱いものは何か？
  └─ 探索範囲を狭める/広げる/ずらすべきか？

[Step 2] constants.rs の探索範囲定数または初期値を編集

[Step 3] cargo test kw4_optimize -- --nocapture
  └─ 内側ループが約 100-160 回のシミュレーションを自動実行
  └─ 各 iteration の J_kw が逐次出力される
  └─ 収束後、最終結果を出力

[Step 4] 結果を observation.md に記録

[Step 5] experiment_count++

[Step 6] 8回に達した → 中間報告 → ユーザー指示待ち
[Step 6'] 24回に達した → 最終報告 → Human review queue → 終了
[Step 6''] 収束かつ J_kw > 0.8 + 全フラグ成立 → Kind World 達成！→ 終了
```

### 中間報告のフォーマット

中間報告は以下の構成で、**平易な日本語**で書く：

```markdown
## 中間報告 (外側サイクル N / 3)

### ここまでの実験結果

- 全 8 回の cargo test で、それぞれ内側で約 100 回の最適化を行いました
- 最も社会加速度が高かったのは N 回目（スコア X.XX）の実験で、[簡潔な説明]
- 最も低かったのは N 回目（スコア X.XX）で、[簡潔な説明]

### Nelder-Mead の収束状況

- 8 回中 N 回が正常収束、N 回が最大反復に達して打ち切り
- 収束までに要した平均シミュレーション回数: 約 XXX 回
- 収束しなかった場合は探索範囲が不適切だった可能性があります

### エコシステムの状態

社会加速度を構成する 6 つの要素のうち、どの分野が足を引っ張っているかを分析しました：
- ✅ [分野]: 良好です。[理由]
- ❌ [分野]: 改善が必要です。[理由]

### 探索範囲の広さと収束の関係

[どの探索範囲が広すぎたか/狭すぎたか、Nelder-Mead の履歴からの分析]

### つまり社会加速度にとってどういう状態か

[社会加速度の数字をわかりやすく言い換えたもの、何が最大の障害かを説明]

### どうしてみようと思うか

以上の分析から、次のサイクルでは以下を試します：
1. [仮説とその理由]
2. [追加の仮説]

→ この方針で進めてよろしいでしょうか？
```

## Non-scope

- `run_simulation` 関数自体の修正は行わない
- `run_phase4` の human review queue 実装の修正は行わない（配送先として利用するのみ）
- `compute_kind_world_objective` の計算ロジックは修正しない
- 複数の cargo test の並列実行は行わない
- ベイズ最適化や勾配法等、Nelder-Mead 以外の最適化アルゴリズムは実装しない

## Test Plan

### ユニットテスト（kind_world.rs, `mod tests`）

| ID | テスト | アサーション |
|----|--------|-------------|
| TC1 | Nelder-Mead 初期シンプレックス生成 | 8 頂点がすべて探索範囲内かつ異なる値を持つ |
| TC2 | Nelder-Mead 1次元での収束 | y = x² の最小値が x=0 付近に収束（1次元なので 2 頂点） |
| TC3 | Nelder-Mead 反射・拡大・収縮・縮小の各操作 | 各操作後の頂点が探索範囲内に収まる |
| TC4 | `evaluate` 関数 | 同一パラメータで同一 J_kw（決定論的） |
| TC5 | `OptimizationReport` JSON シリアライズ | 全フィールドが正しく JSON 出力可能 |
| TC6 | `kw4_optimize` 正常実行と出力形式 | panic せず完了、履歴 CSV + 最終 JSON が出力される |
| TC7 | 異なる探索範囲で異なる結果 | 範囲を狭めると異なる最良値に収束 |
| TC8 | 既存 Phase 0-2 後方互換 | 本チケット追加後も既存テスト全 PASS |

**TC2 の詳細（Nelder-Mead 検証用）:**

```rust
#[test]
fn tc2_nelder_mead_1d_convergence() {
    // f(x) = (x - 3)² の最小化: 理論解 x=3
    let mut optimizer = NelderMeadOptimizer::new_1d(0.0, (0.0, 5.0));
    let report = optimizer.run(100);
    assert!((report.best_params_as_1d() - 3.0).abs() < 0.1,
        "1次元 Nelder-Mead が x=3 に収束: got {}", report.best_params_as_1d());
}
```

### 観測テスト

| ID | テスト | 観測対象 |
|----|--------|---------|
| OBS1 | `kw4_optimize` 履歴観測 | 各 iteration の J_kw 時系列（収束曲線） |
| OBS2 | 探索範囲変更による収束点の変化 | 範囲を変えると最適値がどう変わるか |

## 計装方法・観測対象

`kw4_optimize` テストは以下の出力を行う：

```
=== kw4_optimize [experiment_id=kw4-001] ===

--- Nelder-Mead iteration history ---
iter,J_kw,gamma_benevolence,lambda_gc_base,...
0,0.4213,0.1500,1.0000,...
1,0.4387,0.1623,0.9876,...
2,0.4521,0.1710,0.9543,...
...
127,0.7246,0.2310,0.8120,...

--- Final Report (JSON) ---
{
  "experiment_id": "kw4-001",
  "best_j_kw": 0.7246,
  "iterations": 128,
  "converged": true,
  "best_params": { ... },
  "assessment": { "is_kind_world": false, "j_kw": 0.7246, "flags": [...] },
  "j_components": { "j_pop": 0.12, "j_cov": 0.74, ... }
}

--- Kind World Check ---
is_kind_world: false (7/8 flags) — missing: cost_efficiency
```

**観測対象**: 内側ループの収束曲線（J_kw が iteration とともにどう改善するか）、収束点のパラメータ値、未成立の条件フラグ。

## Acceptance Criteria

1. **内側ループ**: Nelder-Mead が 1 回の `cargo test` 内で約 100〜160 回のシミュレーションを実行し、収束すること。
2. **検証用テスト**: 1 次元の単純な凸関数 (y = (x-3)²) で Nelder-Mead が理論解に収束することを確認すること。
3. **結果出力**: 各 iteration の履歴 CSV + 最終 JSON レポートが標準出力に書き出されること。
4. **外側ループ記録**: 各 cargo test の結果が `experiments.md` に記録されること。
5. **中間報告**: 8 回ごとに平易な日本語で中間報告が生成され、ユーザーの指示を仰ぐこと。
6. **Kind World 達成**: J_kw > 0.8 かつ全 8 条件成立をもって Kind World 達成と判定すること。
7. **打ち切り**: 最大 24 サイクルで外側ループを終了すること。
8. **Human review**: 最終結果が Human review queue に配送されること。
9. **後方互換性**: 既存テストが本チケット追加後も全 PASS すること。

## Boy Scout Rule — 翻訳可能性計画

**スコープ内**: Nelder-Mead の各操作（反射・拡大・収縮・縮小）は独立した関数に分割し、最適化の流れが `run()` メソッドを読むだけで理解できるようにする。各パラメータの探索範囲は constants.rs に定数定義。unwrap 禁止。

**スコープ外（触れた範囲のみ）**: `compute_cross_village_interaction_rate` (kind_world.rs:719-746) のスタブ関数 — `#[allow(dead_code)]` 追加を検討。

## Dependencies

- **必須**: M1.76-KW1 (ticket 109, `done`)
- **必須**: M1.76-KW2 (ticket 110, `reviewed`)
- **必須**: M1.76-KW3 (ticket 111, `reviewed`)
- **必須**: M1.76-17 (ticket 102, `reviewed`)
- **必須**: M1.76-19 (ticket 104, `reviewed`)
