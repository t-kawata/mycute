---
ticket_id: 54
title: M0.5-3: パッチ適用における未解決変数（VarScopeViolation）の確率的検出テスト
slug: m05-3-varscopeviolation
status: reviewed
created_at: 2026-05-23
updated_at: 2026-05-23
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0054-m05-3-varscopeviolation/observation-20260523-164745.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0054-m05-3-varscopeviolation/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0054-m05-3-varscopeviolation/review.md
---
# M0.5-3: パッチ適用における未解決変数（VarScopeViolation）の確率的検出テスト

## Summary

パッチ適用後の仮グラフに対し、入力変数スコープの前向き整合性を走査するアルゴリズムを実装し、未解決変数の確率的検出によりバリデータスコア c_v の減算規則を統計的に検証する。

## Background

M0.5-2（確率的パッチ操作インジェクション）では OTS-C1（サイクル検出完全性）と OTS-C2（ノイズ注入安全性）を検証した。M0.5-3 はその次のバリデーション軸として「変数スコープ（V-03）違反」に焦点を当てる。RFC §14.3 のバリデータスコア c_v 減算規則（未解決変数1件につき -0.15、上限3件）と、§14.4 の変数スコープ検証アルゴリズムの確率的挙動を統計的に検証する。

既存の `validate_var_scope`（src/patch.rs:285）は最初の違反で即座に `Err` を返すため、複数違反の計数には対応していない。本チケットでは、(1) 違反数を計数するバリデータスコア計算関数を実装し、(2) 確率的パッチ注入による耐久テストで減算規則の正確性と相転移挙動を観測する。

## Scope

- `compute_validator_score(var_violations: usize) -> f32` 関数の実装（RFC §14.3 c_v 計算: 1.0 - 0.15 * min(E_v, 3)）
- 既存 `validate_var_scope` の変更はしない（1件目エラー即時報告の挙動は維持）
- パッチの変数宣言をランダムに破壊（存在しない変数からの DataFlow 接続など）するテストハーネス
- OTS-V1: バリデータスコア単調性テスト（確定的 n=100）
- OTS-V2: 複合信頼度関数の偏微分感度測定（統計的 n=10,000）
- OTS-V3: 重み動的切り替えによる決定勾配の幾何学的不連続性観測（n=1,000）

## Non-scope

- `validate_var_scope` のエラー報告方式の変更（複数エラー集約など）
- 本物のグラフストアとの結合
- LLM クライアントの呼び出し

## Investigation

### 既存実装の確認

**1. `validate_var_scope`（src/patch.rs:285-309）**

全 DataFlow エッジについて from_var が送信元ノードの output_var と一致するかを検証する。違反検出時に即座に `Err(PatchError::VarScopeViolation(...))` を返す（早期リターン）。複数違反を同時に報告する機構は持たない。

```rust
pub fn validate_var_scope(graph: &WorkflowGraph) -> Result<(), PatchError> {
    for edge_idx in graph.edge_indices() {
        let (from_idx, _to_idx) = graph.edge_endpoints(edge_idx)?;
        if let Some(EdgeMeta::DataFlow { ref from_var, .. }) = graph.edge_weight(edge_idx) {
            if let Some(from_weight) = graph.node_weight(from_idx) {
                let output_var = match from_weight {
                    WorkflowNode::AgentStep { output_var, .. } => output_var,
                    WorkflowNode::SubWorkflow { output_var, .. } => output_var,
                    _ => continue,
                };
                if from_var != output_var {
                    return Err(PatchError::VarScopeViolation(/*...*/));
                }
            }
        }
    }
    Ok(())
}
```

**2. `PatchConfidence::compute`（src/patch.rs:58-72）**

`value = cs_adj^ws * cv^wv * ch^wh` の計算式。c_s < 0.50 時に重みが (ws=0.20, wv=0.50) に動的切り替え。

**3. 既存定数（src/constants.rs:85-87）**

`VALIDATOR_VAR_SCOPE_PENALTY = 0.15` が既に定義済みだが、c_v(E_v) 計算関数は未実装。

**4. 参照観察レポート**

- tickets/context/0053-m05-2/observation-20260523-154521.md — M0.5-2 OTS-C1/C2 結果: サイクル検出率 10000/10000 (p_miss=0.0)、パニック率 0/1000、DAG違反 0/1000
- tickets/context/0052-m05-1-fake-llm/observation-20260523-145926.md — M0.5-1 スクリプト化不正フォーマット: p_m sweep 線形性、シャノンエントロピー一致性確認済み

### RFC 該当セクション

- §14.3 バリデータスコア c_v 計算（Darvium-RFC-0001-Unified-v2.3-final.md:2264-2267）:
  - 未解決変数1件ごとに -0.15（上限3件）
  - DataFlow 辺の一貫性違反: -0.15
  - [0.0, 1.0] にクランプ
- §14.4 変数スコープ検証: `validate_var_scope` のアルゴリズム定義
- §12.3 PatchConfidence: 3次元スコア計算 + 重み動的切り替え

## Test Plan

### テスト構成

`tests/m0_5.rs` に以下の観測テストを追加する（M0.5-2 の OTS-C1/C2 と同じテストファイルに追記）。

### 関数実装（src/patch.rs に追加）

```rust
/// バリデータスコア c_v を計算する (RFC §14.3)。
///
/// c_v(E_v) = clamp(1.0 - 0.15 * min(E_v, 3), 0.0, 1.0)
/// E_v: 検出された未解決変数の件数
pub fn compute_validator_score(var_violations: usize) -> f32 {
    let penalty = constants::VALIDATOR_VAR_SCOPE_PENALTY as f32;
    let score = 1.0 - penalty * (var_violations.min(3) as f32);
    score.max(0.0).min(1.0)
}
```

### OTS-V1: バリデータスコア単調性（確定的 n=100）

- E_v を 0..=10 で変化させ、`compute_validator_score` の出力が RFC §14.3 と一致することを確認
- E_v=0 → 1.0, E_v=1 → 0.85, E_v=2 → 0.70, E_v=3 → 0.55, E_v≥3 → 0.55 をアサート
- E_v 増加に対してスコアが非増加（単調非増加性）を確認

### OTS-V2: 複合信頼度偏微分感度（統計的 n=10,000）

- c_s を `{0.30, 0.60, 0.80}` の3水準で固定
- c_h を 0.50 に固定
- E_v を 0..=3 で精密にインクリメント（各水準 n=1,000）
- 各組み合わせで PatchConfidence::compute(c_s, c_v(E_v), c_h) を計算
- 観測対象: ∂PatchConfidence/∂E_v の分散 σ²
- 期待値: E_v ≤ 3 領域で σ²(∂PatchConfidence/∂E_v) = 0（完全な線形性）

### OTS-V3: 重み切り替え不連続性観測（n=1,000）

- c_s を 0.45 から 0.55 まで 0.01 刻みで sweep（各点 n=100）
- E_v を 1 に固定
- c_h を 0.50 に固定
- 観測対象: c_s = 0.50 通過前後での value の幾何学的ジャンプ
- 期待出力: 決定勾配ベクトル場（ws, wv）の相転移可視化チャート
  - c_s < 0.50: (ws=0.20, wv=0.50) → 勾配は wv 方向が支配的
  - c_s ≥ 0.50: (ws=0.30, wv=0.40) → 勾配は ws 方向がより強く寄与
  - c_s = 0.50 の前後 2 点で value に確定的なジャンプ差が観測されること

### ランダムパッチ注入による変数スコープ破壊（n=1,000）

- `build_random_dag`（M0.5-2 既存）を利用してランダムグラフを構築
- グラフ中のノードからランダムに選択した DataFlow 辺の from_var を、存在しない変数名に書き換えたパッチを生成
- パッチを `apply_patch_atomic` に投入し、`Err(PatchError::VarScopeViolation(_))` が返ることを確認
- 違反内容のエラーメッセージに破壊前後の変数名が含まれていることを確認

## 計装方法・観測対象

### 計装方法

- テストコード: `tests/m0_5.rs` に追記（既存 M0.5-2 テストの下に追加）
- 固定シード: `StdRng::seed_from_u64(12345)` を使用
- 計測プローブ: `println!` + `--nocapture` で CSV 形式の構造化データを出力

**OTS-V2 CSV 出力フォーマット:**
```
c_s,c_v,E_v,value,ws,wv
0.30,1.000,0,<value>,<ws>,<wv>
0.30,0.850,1,<value>,<ws>,<wv>
...
```

**OTS-V3 CSV 出力フォーマット:**
```
c_s,c_v,value,ws,wv
0.45,0.700,<value>,0.20,0.50
0.46,0.700,<value>,0.20,0.50
...
0.50,0.700,<value>,0.30,0.40  # 切り替え点
0.51,0.700,<value>,0.30,0.40
...
```

### 観測対象

| 観測ID | 統計量 | n | 期待値 |
|--------|--------|---|--------|
| OTS-V1 | スコア値の絶対誤差 | 100 (確定的) | 全点で RFC 値と一致 |
| OTS-V2 | ∂P/∂E_v の分散 σ² | 10,000 | σ² = 0（完全線形） |
| OTS-V3 | c_s=0.50 前後の value 不連続量 | 1,000 | 決定論的ジャンプ観測 |

### 較正計画

本チケットではバリデータスコア c_v の減算規則（-0.15/violation）を検証するため、定数の調整は行わない。`VALIDATOR_VAR_SCOPE_PENALTY` は不変条件（Safety Invariant）として扱う。

## Boy Scout Rule — 翻訳可能性計画

- 新規追加する `compute_validator_score` は関数名で処理内容を完全に説明する（動詞句, 一責務）
- テスト関数名は `ots_v1_validator_score_monotonicity`, `ots_v2_confidence_partial_derivative`, `ots_v3_weight_switch_discontinuity` とし、観測対象を明示
- 既存のテスト関数に倣い、CSV ヘッダ付き構造化出力で翻訳可能性を担保

## Acceptance Criteria

- [ ] `compute_validator_score` が RFC §14.3 の減算規則と一致すること
- [ ] OTS-V1: 全 11 ケース（E_v=0..10）でスコアが期待値と完全一致
- [ ] OTS-V2: ∂PatchConfidence/∂E_v の分散 σ² = 0（n=10,000, c_s 水準別）
- [ ] OTS-V3: c_s=0.50 前後で決定論的ジャンプが観測され、勾配の不連続性が確認できる
- [ ] ランダムパッチ注入（n=1,000）で全ケース PatchError::VarScopeViolation を検出
- [ ] 既存テストがすべて通過していること

## Notes

- plan_path: /plan-ticket が plan.md 作成後に frontmatter を更新する
- implementation_path: /start-ticket が implementation.md 作成後に frontmatter を更新する
- review_report_path: /review-ticket が review.md 作成後に frontmatter を更新する
- observation_report_path: /start-ticket が observation-YYYYMMDD-HHmmss.md を作成後に frontmatter の最新パスを更新する

### 成果物

- 計画: context/0054-m05-3-varscopeviolation/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0054-m05-3-varscopeviolation/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0054-m05-3-varscopeviolation/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0054-m05-3-varscopeviolation/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
