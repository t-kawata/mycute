---
ticket_id: 94
title: M1.76-9 Benevolence-aware remote exploration (F-13)
slug: m176-9-benevolence-aware-remote-exploration-f-13
status: reviewed
created_at: 2026-05-26
updated_at: 2026-05-26
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0094-m176-9-benevolence-aware-remote-exploration-f-13/observation-20260526-090120.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0094-m176-9-benevolence-aware-remote-exploration-f-13/implementation.md
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0094-m176-9-benevolence-aware-remote-exploration-f-13/plan.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0094-m176-9-benevolence-aware-remote-exploration-f-13/review.md
---

# M1.76-9 Benevolence-aware remote exploration (F-13)

## Summary

既存の bounded remote exploration（式 41B-19, M1.75-6）に benevolence の影響を組み込み、local adults の benevolence が十分高い場合は remote exploration 率を下げ、local shortage 時にのみ上げる純粋関数 `compute_benevolence_aware_remote_exploration`（式 F-13）を実装する。既存 `select_helpers()` の exploration 率 ε を本関数で上書きする adapter として接続する。

## Background

- RFC §41B.20.3 式 F-13 で定義。v2.3-e の bounded remote exploration (41B-19) を保持しつつ、「近くに優しい大人がいるなら、まず近所で助け合う」を operational に実現する。
- 既存の `select_helpers()`（`src/childsupport.rs`）は `HelperSelectionPolicy.epsilon` を静的な定数値（`HELPER_WEIGHT_EXPLORATION_EPSILON = 0.1`）で使用している。これを child の need と local benevolence 平均に応じて動的に変化させる。
- 対応マイルストーン: RFC §41C.3 の **M0.x**。

## Scope

1. **定数追加** (`src/constants.rs`):
   - `REMOTE_EXPLORATION_NEED_COEFF` (a₁) — child need に対する線形係数
   - `REMOTE_EXPLORATION_BENEVOLENCE_COEFF` (a₂) — local benevolence 平均に対する線形係数（減算）

2. **純粋関数の実装** (`src/reciprocity.rs`):
   - `compute_benevolence_aware_remote_exploration(child_need: f32, local_benevolence_mean: f32, policy: &ReciprocityLifecyclePolicy) -> f32`
   - 式 F-13: `ε_remote(c) = clip_{[0, ε_max]}( ε_0 + a₁·need(c) - a₂·B_local_avg(c) )`
   - `clip` 下限 0.0、上限 `policy.epsilon_remote_max`

3. **`ReciprocityLifecyclePolicy` 拡張** (`src/event.rs`):
   - `epsilon_remote_need_coeff` — a₁ 定数への参照
   - `epsilon_remote_benevolence_coeff` — a₂ 定数への参照

4. **`select_helpers()` との adapter 接続**:
   - `compute_benevolence_aware_remote_exploration` の結果で exploration 率を計算する adapter 関数または、`select_helpers` へのオプショナルパラメータ追加
   - 既存の静的な ε を動的に computed ε で上書きする

5. **テスト**:
   - 不変条件テスト（境界値・単調性・下位互換性・boundedness）
   - 観測テスト（2次元パラメータ空間の応答曲面）

## Non-scope

- `HelperSelectionPolicy` の epsilon フィールド除去（既存の静的使用箇所との互換性維持）
- `select_helpers` の内部構造変更（adapter パターンで対応）
- 較正ループ（J(θ) は M1.76-16 で扱う）

## Investigation

### 関連ソースコード箇所

| ファイル | 該当箇所 | 備考 |
|----------|----------|------|
| `src/constants.rs:770-778` | `REMOTE_EXPLORATION_BASE` / `REMOTE_EXPLORATION_MAX` | 既存の ε₀, ε_max 定数（F-13 でそのまま利用） |
| `src/constants.rs:429` | `HELPER_WEIGHT_EXPLORATION_EPSILON = 0.1` | 既存の静的な探索率（M1.75-6） |
| `src/event.rs:516-519` | `ReciprocityLifecyclePolicy.epsilon_remote_base/max` | 既存のポリシーフィールド |
| `src/childsupport.rs:123-133` | `HelperSelectionPolicy` 構造体 | epsilon を f64 で保持 |
| `src/childsupport.rs:257-306` | `select_helpers()` 関数 | Step 4 で `mix_with_remote_exploration` を呼び出す（L302） |
| `src/childsupport.rs:219-235` | `mix_with_remote_exploration()` 関数 | 式 41B-19 の混合を実装 |

### 既存の定数（未作成: a₁, a₂）

`a₁`（need 係数）および `a₂`（benevolence 係数）の定数は未定義。新規追加が必要。

### 実装方針

`select_helpers()` の呼び出し元（例: `replay.rs` や calibration コード）で、先に `compute_benevolence_aware_remote_exploration` を呼び、その戻り値を `HelperSelectionPolicy` の epsilon として設定する adapter パターンを採用する。これにより既存の `select_helpers` の内部ロジックを一切変更せずに済む。

```rust
// Adapter パターン:
let adaptive_epsilon = compute_benevolence_aware_remote_exploration(
    child_need,
    local_benevolence_mean,
    &reciprocity_policy,
);
let helper_policy = HelperSelectionPolicy {
    epsilon: adaptive_epsilon as f64,
    ..base_policy
};
select_helpers(candidates, &child_pos, &trusts, &reputations, &helper_policy);
```

## Test Plan

### ユニットテスト（`src/reciprocity.rs` の `mod tests` に追加）

| ID | 種別 | 条件 | 期待結果 |
|-----|------|------|---------|
| T-1 | 境界値 | `need = 0, B_local_avg = 1.0` | ε_remote が clip 下限 (0.0) |
| T-2 | 境界値 | `need = 1.0, B_local_avg = 0` | ε_remote が clip 上限 (ε_max) |
| T-3 | 単調性 | `local_benevolence_mean` 増加に伴う変化 | ε_remote が単調非増加 |
| T-4 | 単調性 | `child_need` 増加に伴う変化 | ε_remote が単調非減少 |
| T-5 | 下位互換性 | `a₂ = 0` で `need = 0` | ε_remote == ε₀ |
| T-6 | boundedness | 全入力値域でのランダムサンプリング (n=10⁴) | 常に [0, ε_max] に bounded |
| T-7 | 空need/空benevolence | `need = 0.5, B_local_avg = 0.5` (ニュートラル) | ε_remote == ε₀（理想値） |

### 観測テスト

- **応答曲面観測**: `(need, B_local_avg)` の 2 次元パラメータ空間（各 0.0〜1.0, step 0.05）で ε_remote の応答曲面を `println!` で出力。`a₁/a₂` の比率を 3 水準（0.5, 1.0, 2.0）で sweep し、need-driven exploration と benevolence-driven restraint のトレードオフ曲線を計測。
- **既存 exploration 率との差分分布**: ε₀（0.05）を基準に、F-13 適用後の ε_remote との差分を random village 状態 n=10⁴ で観測。benevolence が remote exploration をどの程度抑制するかを定量化。
- **全テストは固定シード PRNG（`StdRng::seed_from_u64(12345)`）を使用**。

## 計装方法・観測対象

### 計装方法

- 純粋関数 `compute_benevolence_aware_remote_exploration` の入出力を `println!` で出力
- `--nocapture` 経由で標準出力に書き出す
- 2次元 grid sweep: `need ∈ [0.0, 1.0] step 0.05`, `B_local ∈ [0.0, 1.0] step 0.05` → 21×21 = 441 点で応答曲面計測
- 差分分布: ランダムな `(need, B_local_avg)` を n=10,000 サンプリングし、F-13 適用前後の ε の差分分布をヒストグラムで観測

### 観測対象

- `(need, B_local_avg)` → `ε_remote` の応答曲面（2次元表）
- `a₁/a₂` ratio を [0.5, 1.0, 2.0] で sweep したときの曲面形状変化
- 差分（ε_remote - ε₀）の平均・標準偏差・P5/P95 分位数
- 上限 `ε_max` に張り付く割合（saturation rate）
- 下限 0.0 に張り付く割合（starvation rate）

### 較正計画

- 調整する定数: `REMOTE_EXPLORATION_NEED_COEFF` (a₁), `REMOTE_EXPLORATION_BENEVOLENCE_COEFF` (a₂)
- 目的関数: M1.76-16 で定義（本チケットでは純粋関数実装と観測テストまで）
- 停止条件: N/A（較正は後続チケット）

## Boy Scout Rule — 翻訳可能性計画

- **新規関数名**: `compute_benevolence_aware_remote_exploration` — 「慈悲を考慮した遠隔探索を計算する」という動詞句として読める
- **変数名**: `child_need` / `local_benevolence_mean` / `adaptive_epsilon` — ドメイン概念を直接表現
- **一関数一責務**: 本関数は F-13 の計算のみに専念。`select_helpers` との接続は adapter 関数または呼び出し元で行う
- **ハードコード値禁止**: ε₀, ε_max, a₁, a₂ はすべて `constants.rs` の名前付き定数 + policy 経由で注入
- **既存コード改善**: `select_helpers` のシグネチャは変更せず、呼び出し元で adapter を適用する。既存テストには影響ゼロ

## Acceptance Criteria

- [ ] F-13 の純粋関数 `compute_benevolence_aware_remote_exploration` が実装されている
- [ ] 対応する定数 a₁, a₂ が `constants.rs` に追加されている
- [ ] `ReciprocityLifecyclePolicy` に a₁, a₂ のフィールドが追加され、Default 実装が更新されている
- [ ] 既存 `select_helpers` を変更せずに adapter 経由で接続可能である
- [ ] 以下の不変条件テストがすべて PASS:
  - T-1: `need=0, B_local_avg=1.0` → 最小値 (0.0)
  - T-2: `need=1.0, B_local_avg=0` → 最大値 (ε_max)
  - T-3: B_local_avg 増加 → ε_remote 単調非増加
  - T-4: child_need 増加 → ε_remote 単調非減少
  - T-5: a₂=0 かつ need=0 → ε₀ と一致（下位互換性）
  - T-6: 全入力範囲で [0, ε_max] に bounded
- [ ] 観測テストが応答曲面を出力する
- [ ] `cargo test` が既存テスト含めて全件 PASS
- [ ] 翻訳可能性の検証が通っている

## Notes

- plan_path: context/0094-m176-9-benevolence-aware-remote-exploration-f-13/plan.md（未作成、/plan-ticket 承認後に作成）
- implementation_path: context/0094-m176-9-benevolence-aware-remote-exploration-f-13/implementation.md（未作成、/start-ticket 実装完了後に作成）
- review_report_path: context/0094-m176-9-benevolence-aware-remote-exploration-f-13/review.md（未作成、/review-ticket 全チェック通過後に作成）
- observation_report_path: context/0094-m176-9-benevolence-aware-remote-exploration-f-13/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成）

### 成果物

- 計画: context/0094-m176-9-benevolence-aware-remote-exploration-f-13/plan.md（未作成）
- 実装サマリ: context/0094-m176-9-benevolence-aware-remote-exploration-f-13/implementation.md（未作成）
- レビュー報告書: context/0094-m176-9-benevolence-aware-remote-exploration-f-13/review.md（未作成）
- 観察レポート: context/0094-m176-9-benevolence-aware-remote-exploration-f-13/observation-YYYYMMDD-HHmmss.md（未作成）
