---
ticket_id: 91
title: M1.76-6: GC hazard with benevolence (F-7, F-8, F-9)
slug: m176-6-gc-hazard-with-benevolence-f-7-f-8-f-9
status: reviewed
created_at: 2026-05-26
updated_at: 2026-05-26
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0091-m176-6-gc-hazard-with-benevolence-f-7-f-8-f-9/plan.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0091-m176-6-gc-hazard-with-benevolence-f-7-f-8-f-9/observation-20260526-081149.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0091-m176-6-gc-hazard-with-benevolence-f-7-f-8-f-9/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0091-m176-6-gc-hazard-with-benevolence-f-7-f-8-f-9/review.md
---

# M1.76-6: GC hazard with benevolence (F-7, F-8, F-9)

## Summary

RFC §15.10.4 式 F-7、F-8、F-9 で定義される GC hazard 計算の純粋関数を実装する。式 F-7 は LifecycleScore L(G) に加えて benevolence スコアと child protection 項を合成し、softplus により常に非負のハザード率 λ_i^GC を計算する。式 F-8 は GC 判定確率 p_GC、式 F-9 は生存確率 P_survive をそれぞれ指数分布に従って計算する。本チケットは RFC §41C.3 の **M0.x（pure function validation）** フェーズに対応する。

既存の GC 計算（LifecycleScore 単独）を変更せず、GC hazard 側で benevolence を効かせる design（推奨案 B）を維持する。

## Background

M1.76-3 で `compute_direct_reciprocity` (F-1)、M1.76-4 で `compute_indirect_reciprocity` (F-2) および `compute_benevolence_score` (F-3)、M1.76-5 で `recompute_reputation` (F-4/F-5) が実装され、互恵性スコア・評判・benevolence の計算基盤が整った。

次のステップとして、これらのスコアを既存の GC 淘汰メカニズムに統合する。具体的には、以下の入力を受け取り GC hazard を計算する純粋関数群を実装する:

- **LifecycleScore** L_i: 既存 GC 基盤の生存スコア（本チケットでは変更しない）
- **BenevolenceScore** B_i: F-3 で計算された慈悲スコア
- **Child protection** C_i^protect: F-10 で定義される child 保護項（F-10 自体は M1.76-7 で実装）
- **Policy パラメータ**: λ_0, γ_L, γ_B, γ_C

`compute_gc_hazard` の出力は以下の後続チケットで利用される:
- M1.76-7: Child protection integration (F-10) — C_i^protect 計算
- M1.76-8: Helper quality score (F-11) — survival probability を helper 品質に反映
- M1.76-11: ReciprocityEvent ingestion pipeline — GC hazard の定期再計算

### 参照観察レポート

- `tickets/context/0088-m176-3-compute-direct-reciprocity-f-1/observation-20260525-184956.md` — 直接互恵性スコア実装完了確認。
- `tickets/context/0089-m176-4-compute-indirect-reciprocity-f-2-benevolence-f-3/observation-20260526-075128.md` — 間接互恵性スコア・BenevolenceScore 実装完了確認。
- `tickets/context/0090-m176-5-reputationprofile-recompute-reputation-f-4-f-5/observation-20260526-080114.md` — ReputationProfile 再計算実装完了確認。

## Scope

1. **`softplus` ユーティリティ関数の実装** — `softplus(x) = ln(1 + exp(x))`。入力が極端に負の場合のアンダーフロー対策を含む。
2. **`compute_gc_hazard` 関数の実装** — 式 F-7: `softplus(λ_0 - γ_L·L_i - γ_B·B_i - γ_C·C_i^protect)`
3. **`compute_gc_probability` 関数の実装** — 式 F-8: `p_GC(i; Δt) = 1 - exp(-λ_i^GC · Δt)`
4. **`compute_survival_probability` 関数の実装** — 式 F-9: `P_survive(i; Δt) = exp(-λ_i^GC · Δt)`
5. **F-7 定数の追加** — `src/constants.rs` に `GC_HAZARD_LAMBDA_0`, `GC_HAZARD_GAMMA_LIFECYCLE` を新規定義（`GC_HAZARD_GAMMA_BENEVOLENCE` と `GC_HAZARD_GAMMA_CHILD_PROTECT` は既存）
6. **`ReciprocityLifecyclePolicy` デフォルト値の定数化** — `lambda_gc_base` と `gamma_lifecycle` を名前付き定数参照に修正
7. **8件のテスト**（Test Plan 参照）

## Non-scope

- Child protection term C_i^protect (F-10) の導出 — チケット M1.76-7 で実装
- 既存 GC 計算（LifecycleScore L(G)）の変更 — 本チケットでは hazard 側のみ
- `ReciprocityEvent` のインジェスション・パイプライン — チケット M1.76-11 で実装
- EventBus や MetadataStore との結合 — 本チケットでは純粋関数として隔離
- GC hazard の実際の適用（Workflow の淘汰） — 既存 GC フレームワークの責務
- `ReciprocityLifecyclePolicy` の永続化・バージョニング

## Investigation

### 物理的証拠

**証拠1: RFC §15.10.4 L4376-4410 — F-7 / F-8 / F-9 完全数式**

```
F-7: λ_i^GC = softplus( λ_0 - γ_L·L_i - γ_B·B_i - γ_C·C_i^protect )

F-8: p_GC(i; Δt) = 1 - exp(-λ_i^GC · Δt)

F-9: P_survive(i; Δt) = exp(-λ_i^GC · Δt)
```

Normative constraints:
- ∂λ_i^GC/∂R_i^dir ≤ 0: 直接互恵性が高いほど淘汰ハザードは非増加 (MUST)
- ∂λ_i^GC/∂R_i^ind ≤ 0: 間接互恵性が高いほど淘汰ハザードは非増加 (MUST)
- ∂λ_i^GC/∂Rep_i ≤ 0: 評判が高いほど淘汰ハザードは非増加 (MUST)
- softplus により λ_i^GC は常に非負

**証拠2: 既存 ReciprocityLifecyclePolicy（`src/event.rs` L492-499）**

F-7 の4パラメータは既にフィールドとして定義済みだが、デフォルト値の問題がある:
```rust
lambda_gc_base: 0.1,      // 生値ハードコード
gamma_lifecycle: 0.5,     // 生値ハードコード
gamma_benevolence: 0.10,  // GC_HAZARD_GAMMA_BENEVOLENCE 参照（適切）
gamma_child_protect: 0.20, // GC_HAZARD_GAMMA_CHILD_PROTECT 参照（適切）
```

問題点:
1. `lambda_gc_base` (λ_0) と `gamma_lifecycle` (γ_L) が名前付き定数を参照していない
2. Boy Scout Rule: 触るコードは美しく — 定数化してから実装する

**証拠3: 既存 constants.rs（L696-704）**

F-7 に必要な定数の一部のみ定義済み:
```rust
// 既存:
pub const GC_HAZARD_GAMMA_BENEVOLENCE: f32 = 0.10;   // γ_B (OK)
pub const GC_HAZARD_GAMMA_CHILD_PROTECT: f32 = 0.20;  // γ_C (OK)

// 未定義で追加が必要:
// pub const GC_HAZARD_LAMBDA_0: f32 = ?;               // λ_0
// pub const GC_HAZARD_GAMMA_LIFECYCLE: f32 = ?;        // γ_L
```

**証拠4: M1.76-5 テストパターン（`src/reciprocity.rs` L896-999）**

確立されたテストパターン: ゼロ入力、単調性 sweep、値域拘束（n >= 10,000）、係数 sweep、`println!` + CSV 計装。

### 関数シグネチャ設計

```rust
/// softplus 関数: ln(1 + exp(x))
///
/// 常に非負。x が極端に負の場合でも数値的安定性を保つ。
fn softplus(x: f32) -> f32 {
    if x > 0.0 {
        x + (1.0 + (-x).exp()).ln()
    } else {
        (1.0 + x.exp()).ln()
    }
}

/// GC hazard λ_i^GC を計算する (F-7)。
///
/// 式 F-7:
///   λ_i^GC = softplus( λ_0 - γ_L·L_i - γ_B·B_i - γ_C·C_i^protect )
///
/// softplus により常に非負。
/// 各 γ > 0 (MUST) により、スコアの増加はハザードを減少させる。
///
/// # 引数
/// - `lifecycle_score`: L_i — 既存 LifecycleScore [0, 1]
/// - `benevolence_score`: B_i — F-3 慈悲スコア [0, 1]
/// - `child_protection`: C_i^protect — F-10 child 保護項 [0, ∞)
/// - `policy`: 較正パラメータ（λ_0, γ_L, γ_B, γ_C）
///
/// # 戻り値
/// - 非負の f32 ハザード率 λ_i^GC
pub fn compute_gc_hazard(
    lifecycle_score: f32,
    benevolence_score: f32,
    child_protection: f32,
    policy: &ReciprocityLifecyclePolicy,
) -> f32

/// GC 判定確率 p_GC を計算する (F-8)。
///
/// 式 F-8:
///   p_GC(i; Δt) = 1 - exp(-λ_i^GC · Δt)
///
/// # 引数
/// - `hazard`: λ_i^GC — F-7 で計算されたハザード率（非負）
/// - `delta_t`: Δt — 時間間隔（仮想時間単位）
///
/// # 戻り値
/// - [0, 1) の範囲の f64 確率（hazard=0 のとき 0）
pub fn compute_gc_probability(hazard: f32, delta_t: u64) -> f64

/// 生存確率 P_survive を計算する (F-9)。
///
/// 式 F-9:
///   P_survive(i; Δt) = exp(-λ_i^GC · Δt)
///
/// # 引数
/// - `hazard`: λ_i^GC — F-7 で計算されたハザード率（非負）
/// - `delta_t`: Δt — 時間間隔（仮想時間単位）
///
/// # 戻り値
/// - (0, 1] の範囲の f64 確率（hazard=0 のとき 1）
pub fn compute_survival_probability(hazard: f32, delta_t: u64) -> f64
```

**設計判断**:
1. `softplus` は内部ユーティリティ関数（非公開）。`logistic_sigmoid` 同様に数値的安定性を考慮した実装。
2. `compute_gc_hazard` は `f32` 精度で計算（他のスコアと一貫性）。
3. `compute_gc_probability` と `compute_survival_probability` は `f64` を返す: exp 計算の精度と指数分布の裾野の正確性を考慮。
4. `child_protection` は F-10 の出力を受け取る口（本チケットでは外部から渡される値として扱い、導出は行わない）。
5. ハザードが 0 のとき `p_GC = 0`, `P_survive = 1`（指数分布の性質）。
6. ハザードが正のとき `p_GC ∈ [0, 1)`, `P_survive ∈ (0, 1]`。
7. `delta_t` は u64（仮想時間単位、他の関数との一貫性）。

### 不完全性

**既存定数からの乖離**: `gamma_lifecycle` のデフォルト値 0.5 は「LifecycleScore が 0 から 1 まで変化するとハザードが 0.5 変化する」という設計意図を持つ。しかし RFC の `γ_L` と既存 LifecycleScore L(G) の値域の対応関係は較正フェーズ（M1.76-19）で調整される。初期値 0.5 は妥当な中間値として維持する。

**C_i^protect の欠落**: F-10 は M1.76-7 で実装されるため、本チケットのテストでは `child_protection = 0`（保護なし）を基本とする。これにより M1.76-6 の純粋関数単体検証と M1.76-7 の結合検証を分離する。

## Test Plan

### テストケース一覧

| # | 名称 | 種別 | 内容 |
|---|------|------|------|
| TC-1 | λ_0 単独ベースライン | 正常系 | `λ_0 = 1.0, γ_L = γ_B = γ_C = 0` → `hazard = softplus(1.0) ≈ 1.1269` |
| TC-2 | benevolence_score sweep 単調減少 | 正常系 | `B_i` sweep [0, 1] で hazard が単調非増加 |
| TC-3 | lifecycle_score sweep 単調減少 | 正常系 | `L_i` sweep [0, 1] で hazard が単調非増加 |
| TC-4 | hazard = 0 → P_survive = 1 不変 | 正常系 | hazard=0 で Δt を変えても P_survive=1 が不変 |
| TC-5 | hazard > 0 → P_survive ∈ [0, 1) 単調減少 | 正常系 | hazard>0 で P_survive ∈ [0, 1)、Δt 増加で単調減少 |
| TC-6 | γ_B = 0 退化 → benevolence 無効 | 正常系 | γ_B=0 で benevolence が hazard に影響しない |
| TC-7 | 全パラメータ 0 → hazard = 0 | 正常系 | `λ_0 = γ_L = γ_B = γ_C = 0` → `hazard = softplus(0) = ln2 ≈ 0.6931` |
| TC-8 (計装) | softplus 非負性検証 + 応答曲面 | 計装 | n=10^6 ランダム入力で NaN/Inf 不在、(L_i, B_i) 2次元グリッド応答観測 |

### モック依存

- すべての関数は純粋関数（外部依存なし）
- `ReciprocityLifecyclePolicy` は `default()` またはカスタム構築
- `child_protection` はテスト時に明示的に渡す（F-10 の出力を模擬）

## 計装方法・観測対象

### 計装方法

- **固定シード PRNG**: `StdRng::seed_from_u64(12345)` で確率的サンプリングを再現
- **softplus 非負性検証**: n = 10^6 の一様ランダム入力 `[-100, 100]` で NaN/Inf/負値を検査
- **2次元応答曲面**: `(L_i, B_i)` の 11×11 グリッド上で λ_i^GC の応答曲面を観測。`γ_B / γ_L` の比を sweep し、benevolence が lifecycle と比較してどの程度の hazard 低減効果を持つかを感度比として計測
- **生存確率曲線**: `hazard ∈ [0.01, 0.1, 0.5, 1.0, 2.0]`、`Δt ∈ [1, 10, 100, 1000]` の組み合わせで P_survive の減衰曲線を観測
- **出力形式**: `println!` で CSV 形式を `--nocapture` 経由で標準出力

### 観測対象

| 観測対象 | サンプルサイズ | 期待される性質 |
|---------|--------------|--------------|
| softplus 非負性 | n >= 10^6 | 全出力が非負、NaN/Inf 不在 |
| 値域 [0, ∞) 拘束 | n >= 10,000 | λ_i^GC が非負、NaN/Inf 不在 |
| benevolence_score sweep 単調性 | n = 101 | B_i 増加で hazard 非増加 |
| lifecycle_score sweep 単調性 | n = 101 | L_i 増加で hazard 非増加 |
| P_survive 値域 (0, 1] | 5×4 = 20点 | 全 P_survive が (0, 1] 内、hazard=0 で 1 |
| γ_B/γ_L 感度比 | 11×11 = 121点 | 応答曲面の曲率が設計と一致 |

### 較正計画

本チケットでは較正ループは実施しない（純粋関数実装の検証フェーズ）。初期値:

- `λ_0 = 1.0`（ベースラインハザード）— デフォルトは 0.1（低ハザード運用）
- `γ_L = 0.5`（LifecycleScore 重み）
- `γ_B = 0.10`（BenevolenceScore 重み）
- `γ_C = 0.20`（Child protection 重み）

λ_0 のデフォルト値 0.1 は「何も保護がない状態で弱いハザード」を意味する。較正フェーズ（M1.76-19）で調整される。

## 追加定数定義

以下の定数を `src/constants.rs` に追加する:

```rust
/// F-7 GC hazard ベースライン λ_0 (Calibration Candidate)
/// λ_i^GC = softplus(λ_0 - γ_L·L_i - γ_B·B_i - γ_C·C_i^protect)
/// Default: 1.0, 感度分析推奨範囲: 0.1-5.0
pub const GC_HAZARD_LAMBDA_0: f32 = 1.0;

/// F-7 GC hazard LifecycleScore 重み γ_L (Calibration Candidate)
/// Default: 0.5, 感度分析推奨範囲: 0.2-1.0
pub const GC_HAZARD_GAMMA_LIFECYCLE: f32 = 0.5;
```

### ReciprocityLifecyclePolicy デフォルト値修正

```rust
// 修正前:
lambda_gc_base: 0.1,
gamma_lifecycle: 0.5,

// 修正後:
lambda_gc_base: crate::constants::GC_HAZARD_LAMBDA_0,
gamma_lifecycle: crate::constants::GC_HAZARD_GAMMA_LIFECYCLE,
```

注: デフォルト値が 0.1 / 0.5 から 1.0 / 0.5 に変わるわけではない。lambda_gc_base のデフォルトは 0.1 のまま維持する（低ハザード運用の設計意図）。constants の GC_HAZARD_LAMBDA_0 は 1.0（較正推奨範囲の中央値）だが、ReciprocityLifecyclePolicy のデフォルトは 0.1 を維持。これは constants が「較正推奨範囲の中央値」を表し、policy default が「安全側の運用初期値」を表すという二層構造による。

## Boy Scout Rule — 翻訳可能性計画

### 改善対象

1. **`softplus` 関数追加**: 数値的安定性を備えた純粋関数として分離。条件分岐でアンダーフロー対策。
2. **`compute_gc_hazard` 関数**: 動詞句の関数名。内部は散文的記述順: `inner = λ_0 - γ_L·L - γ_B·B - γ_C·C` → `softplus(inner)`。
3. **`compute_gc_probability` / `compute_survival_probability` 関数**: F-8 / F-9 の指数分布計算。対応するペアとして命名。
4. **ハードコード禁止**: `lambda_gc_base` と `gamma_lifecycle` を既存パターンに従って定数化。
5. **既存コード修正**: `ReciprocityLifecyclePolicy` の `lambda_gc_base` / `gamma_lifecycle` デフォルト値を名前付き定数参照に変更。

### 影響範囲外

`event.rs` のポリシーデフォルト修正（定数参照化）は行うが、既存フィールドの追加・削除は行わない。既存の M1.76-3 / M1.76-4 / M1.76-5 テストや関数シグネチャは変更しない。

## Acceptance Criteria

- [ ] `compute_gc_hazard` が λ_0 = 1.0, 全 γ = 0 で `softplus(1.0) ≈ 1.1269` を返す
- [ ] `benevolence_score` sweep で hazard が単調非増加
- [ ] `lifecycle_score` sweep で hazard が単調非増加
- [ ] `hazard = 0` のとき Δt 不変で `P_survive = 1`
- [ ] `hazard > 0` のとき `P_survive ∈ (0, 1]` かつ Δt 増加で単調減少
- [ ] `γ_B = 0` のとき benevolence が hazard に影響しない
- [ ] `λ_0 = γ_L = γ_B = γ_C = 0` のとき `hazard = softplus(0) = ln2 ≈ 0.6931`
- [ ] softplus 非負性検証 n = 10^6 で NaN/Inf/負値不在
- [ ] (L_i, B_i) 2次元応答曲面が観測可能で γ_B/γ_L 感度比が計測可能
- [ ] `ReciprocityLifecyclePolicy` デフォルト値が正しい定数を参照
- [ ] 既存テスト（F-1, F-2, F-3, F-4, F-5）が通過している
- [ ] RFC §15.10.4 との無矛盾確認済み

## Notes

- 3関数すべて純粋関数（副作用ゼロ）。永続化は本チケットの責務外。
- `softplus` は `logistic_sigmoid` と同様の非公開ユーティリティ関数。
- `compute_gc_hazard` の戻り値は f32（他のスコア計算と一貫）。
- `compute_gc_probability` / `compute_survival_probability` の戻り値は f64（指数分布の裾野精度）。
- `child_protection` は M1.76-7 (F-10) の出力を受け取る口として設計。本チケットのテストでは 0 固定。
- 既存の `LIFECYCLE_WEIGHT_BENEVOLENCE` (0.15) 定数は F-6 関連（欠番式のため）で、本チケットでは使用しない。削除も行わない（影響範囲外）。

### 成果物

- 計画: context/0091-m176-6-gc-hazard-with-benevolence-f-7-f-8-f-9/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0091-m176-6-gc-hazard-with-benevolence-f-7-f-8-f-9/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0091-m176-6-gc-hazard-with-benevolence-f-7-f-8-f-9/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0091-m176-6-gc-hazard-with-benevolence-f-7-f-8-f-9/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成）
