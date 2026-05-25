---
ticket_id: 92
title: M1.76-7: Child protection integration (F-10)
slug: m176-7-child-protection-integration-f-10
status: reviewed
created_at: 2026-05-26
updated_at: 2026-05-26
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0092-m176-7-child-protection-integration-f-10/plan.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0092-m176-7-child-protection-integration-f-10/observation-20260526-082844.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0092-m176-7-child-protection-integration-f-10/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0092-m176-7-child-protection-integration-f-10/review.md
---
# M1.76-7: Child protection integration (F-10)

## Summary

児童保護スコア `C_i^protect` の計算関数 `compute_child_protection` (F-10) を実装する。
本式は既存の Grace Period (`experience_count < MIN_SURVIVAL_EXPERIENCE`) を補強し、
child エンティティに対して追加の GC hazard 低減保護を提供する。

## Background

RFC §15.10.5 式 F-10 で定義される児童保護項 `C_i^protect` は、child エンティティが
GC (Garbage Collection) の対象となる hazard を低減するための保護スコアである。

式: `C_i^protect = η_1 · 1[Child(i)] + η_2 · H_i^received + η_3 · G_i^growth`

既存の Grace Period (`experience_count < MIN_SURVIVAL_EXPERIENCE`) は単に経験値不足を
理由に child を保護するが、F-10 は「支援を受けた child」や「成長している child」に対して
追加の保護を提供する。これにより、Grace Period 超過後も「育っている child」が
不必要に GC されないことを保証する。

本式は既に実装済みの `compute_gc_hazard` (F-7) の `child_protection` パラメータとして
使用される。つまりパイプラインは `compute_child_protection` → `compute_gc_hazard` となる。

### 参照観察レポート

- `tickets/context/0091-m176-6-gc-hazard-with-benevolence-f-7-f-8-f-9/observation-20260526-081149.md` — F-7/F-8/F-9 の GC hazard 実装完了。softplus 非負性確認（n=10⁶）、γ_B/γ_L 感度比 sweep、生存確率曲線の観測完了。`gamma_child_protect = 0.20` がデフォルト設定済み。

## Scope

### 実装スコープ

1. **定数追加** (`src/constants.rs`): η_1, η_2, η_3 (Calibration Candidates) の定義
   - `CHILD_PROTECT_ETA1`: 基本 child 保護定数（is_child によるベース保護）
   - `CHILD_PROTECT_ETA2`: 支援受領重み（H_i^received の係数）
   - `CHILD_PROTECT_ETA3`: 成長改善重み（G_i^growth の係数）
2. **純粋関数実装** (`src/reciprocity.rs`):
   - `compute_child_protection(is_child: bool, help_received: f32, growth_improvement: f32, policy: &ReciprocityLifecyclePolicy) -> f32`
   - `is_child` 判定は既存 `classify_maturity()` の `WorkflowMaturity::Child` を流用（この関数内では行わない）
   - `help_received`: child として有効支援を受けた量 [0, 1]（M1.75-3 の HelpExecution/HelpSuccess から派生）
   - `growth_improvement`: child が maturation に向けて改善している量 [0, 1]（M1.76-10 F-14 と接続）
   - 戻り値: 非負の f32（C_i^protect ≥ 0）
3. **既存 Grace Period との併用アサーション**:
   - Grace Period 中かつ C_i^protect > 0 でも hazard が増加しないことの検証
   - 既存 Grace Period の保護効果と F-10 の保護効果が独立に additive に効くことの確認
4. **テストコード** (`src/reciprocity.rs mod tests`): 以下「Test Plan」の全 TC を実装

### 既に存在するもの（実装不要）

以下の要素は M1.76-6 で既に実装済みであり、本チケットでは触れない：

- `compute_gc_hazard` (F-7) — 既に `child_protection: f32` パラメータを受け取る
- `ReciprocityLifecyclePolicy::gamma_child_protect` — 既存フィールド
- `GC_HAZARD_GAMMA_CHILD_PROTECT` — 既存定数 (0.20)
- Grace Period 判定ロジック — `classify_maturity()` + `MIN_SURVIVAL_EXPERIENCE`

## Non-scope

- `classify_maturity()` 自体の修正は行わない（F-10 はこの関数を呼び出す側で使用する）
- `compute_gc_hazard` の修正は行わない（既に child_protection パラメータ対応済み）
- `HelpExecution`/`HelpSuccess` からの help_received 導出ロジックは本チケットの範囲外（M1.75-3 の責務）
- F-14 (growth_improvement) の実装は本チケットの範囲外（M1.76-10 の責務）

## Investigation

### ソースコード調査結果

1. **`compute_gc_hazard` (F-7) のインターフェース確認** (`src/reciprocity.rs:279-290`):
   - 既に `child_protection: f32` パラメータを持ち、`policy.gamma_child_protect * child_protection` で hazard を低減する
   - 本チケットの出力を直接このパラメータに渡す設計

2. **`ReciprocityLifecyclePolicy` (`src/event.rs:481-517`)**:
   - `gamma_child_protect: f32` フィールドが既に存在（デフォルト値 `GC_HAZARD_GAMMA_CHILD_PROTECT = 0.20`）
   - F-10 の η₁, η₂, η₃ は policy には含まれず、定数として管理する（ガンマ値が GC hazard 側の重みであるのに対し、η 値は child protection スコア計算側の重み）

3. **`classify_maturity` (`src/village.rs:80-96`)**:
   - `experience_count < MIN_SURVIVAL_EXPERIENCE` → `WorkflowMaturity::Child`
   - `WorkflowMaturity::Child` の判定条件は確認済み。F-10 の `is_child` 引数はこの判定を外部から受け取る

4. **参照観察レポート**: M1.76-6 の観測結果から、`compute_gc_hazard` の softplus 非負性と応答曲面が確認されている。本チケットの出力が F-7 に入力された際の挙動は既存実装でカバー済み。

## Test Plan

全テストは `src/reciprocity.rs` の `mod tests` 内に実装する。

### TC-1: 非 child かつ全入力ゼロ → C_i^protect = 0

- `is_child = false, help_received = 0.0, growth_improvement = 0.0` のとき出力が 0.0 であることを確認
- 全ての保護項がゼロになるケース

### TC-2: is_child = true → 最低 η_1 の保護

- `is_child = true, help_received = 0.0, growth_improvement = 0.0` のとき出力が η_1 以上であることを確認
- `is_child = true, help_received = 1.0, growth_improvement = 1.0` のとき出力が η_1 以上であることも確認（当然）
- つまり η_1 は常に最低限の保護として機能する

### TC-3: help_received sweep で単調非減少

- `is_child = true` 固定で `help_received` を [0.0, 1.0] の範囲で sweep（ステップ数 101）
- C_i^protect が単調非減少であることを確認（MUST）

### TC-4: growth_improvement sweep で単調非減少

- `is_child = true` 固定で `growth_improvement` を [0.0, 1.0] の範囲で sweep（ステップ数 101）
- C_i^protect が単調非減少であることを確認（MUST）

### TC-5: Grace Period 独立性アサーション

- Grace Period 中（`classify_maturity` で Child 判定される状況）かつ C_i^protect > 0 のとき、
  `compute_gc_hazard` の出力が Grace Period なし + C_i^protect = 0 の場合より低い（保護されている）ことを確認
- 既存 Grace Period の child 保護効果と本式の保護効果が独立に additive に効くことを確認
  - `hazard_with_grace_and_protection <= hazard_with_grace_only` かつ
    `hazard_with_grace_and_protection <= hazard_with_protection_only`

### TC-6 (計装): C_i^protect 応答曲面 (3D) + 確率的値域検証

- `(is_child, help_received, growth_improvement)` の 3 次元空間上で C_i^protect の応答を出力
  - `is_child`: [false, true] (2)
  - `help_received`: [0.0, 0.5, 1.0] (3)
  - `growth_improvement`: [0.0, 0.5, 1.0] (3)
  - 計 2×3×3 = 18 点の応答を `println!` で構造化出力
- ランダム入力 n=10,000 で値域非負性を確認（StdRng::seed_from_u64(12345)）
- NaN/Inf が一切発生しないことを確認

### TC-7 (計装): η 係数感度分析

- η_1, η_2, η_3 をそれぞれ [0.1, 0.5, 1.0, 2.0] で sweep
- 固定入力 `(is_child=true, help_received=0.5, growth_improvement=0.5)` に対する出力変化を観測
- 各 η が期待通り線形に効くことを確認

## 計装方法・観測対象

### 計装方法

- `src/reciprocity.rs` の `mod tests` 内で実装
- 全テストで `println!` による構造化出力（`--nocapture` で観測）
- 確率的テストでは `StdRng::seed_from_u64(12345)` を使用し再現性を保証

### 観測対象

- **TC-6 (応答曲面)**: `(is_child, help_received, growth_improvement)` 3 次元空間での C_i^protect 応答を観測
- **TC-6 (値域検証)**: n=10,000 の確率的な値域 [0, ∞) 拘束確認
- **TC-7 (η 感度分析)**: 各 η が出力に与える線形影響の確認
- **計装出力形式**: `child_protect_response,is_child={},help_received={:.1},growth={:.1},value={:.6}`

### 較正計画

- 調整する定数: `CHILD_PROTECT_ETA1`, `CHILD_PROTECT_ETA2`, `CHILD_PROTECT_ETA3`
  （`src/constants.rs` に Calibration Candidates として定義）
- デフォルト値: η_1=0.50, η_2=0.30, η_3=0.20（推奨初期値、θ と同程度のスケール感）
- 目的関数 J(θ): M1.76-16 の多目的較正目的関数 F-16 と統合
  （本チケット単独では η のデフォルト値での動作確認が完了条件）
- η 係数 sweep 出力は M1.76-16 の較正ハーネスへの入力として再利用可能

## Boy Scout Rule — 翻訳可能性計画

1. **関数抽出**: F-10 は単一責務の純粋関数として実装し、`compute_child_protection` という動詞句の関数名で処理内容が一目でわかるようにする
2. **変数名**: 引数名 `is_child`, `help_received`, `growth_improvement` はドメイン概念を直接表現し、コードの逐語訳可能性を確保する
3. **定数名**: η₁=`CHILD_PROTECT_ETA1`, η₂=`CHILD_PROTECT_ETA2`, η₃=`CHILD_PROTECT_ETA3` とし、式 F-10 との対応が一目でわかるようにする
4. **一関数一責務**: `compute_child_protection` は C_i^protect の計算のみを行い、`is_child` 判定は呼び出し元に委譲する
5. **ハードコード禁止**: すべての重み値は `constants.rs` に定数として定義する

## Acceptance Criteria

- [ ] `compute_child_protection` 純粋関数が実装されている
- [ ] η_1, η_2, η_3 が constants.rs に Calibration Candidates として定義されている
- [ ] TC-1〜TC-7 の全テストが PASS すること
- [ ] 既存テストが全て通過していること
- [ ] `compute_gc_hazard` との統合が成立していること（F-7 の child_protection パラメータとの接続確認）
- [ ] 既存 Grace Period の保護効果が F-10 により弱められていないこと

## Notes

<!--
注: このコメントは人間向けの説明である。AI は以下の手順に従うこと。

- plan_path: /plan-ticket が plan.md を作成後に frontmatter に更新する
- implementation_path: /start-ticket が implementation.md を作成後に frontmatter に更新する
- review_report_path: /review-ticket が review.md を作成後に frontmatter に更新する
- observation_report_path: /start-ticket が observation-YYYYMMDD-HHmmss.md を作成後に frontmatter に最新パスを更新する

各コマンドのワークフロー手順が frontmatter の更新の正しい手順である。
-->

### 成果物

- 計画: context/0092-m176-7-child-protection-integration-f-10/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0092-m176-7-child-protection-integration-f-10/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0092-m176-7-child-protection-integration-f-10/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0092-m176-7-child-protection-integration-f-10/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
