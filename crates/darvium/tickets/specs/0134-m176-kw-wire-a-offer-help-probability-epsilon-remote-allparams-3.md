---
ticket_id: 134
title: M1.76-KW-WIRE-A: offer_help_probability の epsilon_remote 経由化 — 固定確率パラメータの AllParams 結合（実装順序: 3 番目）
slug: m176-kw-wire-a-offer-help-probability-epsilon-remote-allparams-3
status: reviewed
created_at: 2026-05-28
updated_at: 2026-05-28
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0134-m176-kw-wire-a-offer-help-probability-epsilon-remote-allparams-3/plan.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0134-m176-kw-wire-a-offer-help-probability-epsilon-remote-allparams-3/observation-20260528-080417.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0134-m176-kw-wire-a-offer-help-probability-epsilon-remote-allparams-3/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0134-m176-kw-wire-a-offer-help-probability-epsilon-remote-allparams-3/review.md
---
# M1.76-KW-WIRE-A: offer_help_probability の epsilon_remote 経由化 — 固定確率パラメータの AllParams 結合（実装順序: 3 番目）

## Summary

`offer_help_probability()`（simulation.rs:550）と `advance_help_sessions()`（simulation.rs:592）にハードコードされた 8 つの値（offer base 0.3, offer bv_coeff 0.4, accept base 0.5, accept bv_coeff 0.3, success base 0.6, success bv_coeff 0.25, harmful base 0.15, harmful bv_coeff 0.1）を constants.rs の定数化 + AllParams G3 グループとしてパラメーター化する。同時に `offer_help_probability()` のロジックを `compute_epsilon_remote()`（reciprocity.rs:532-534）経由に変更し、epsilon_remote の各成分（base, need_coeff, benevolence_coeff）が HELP 発動確率に反映されるようにする。これにより HELP プロトコルの挙動が较正ループから制御可能になる。

## Background

RFC §4A.5 および §41B.20.1 F-11 の helper quality score では、遠隔探索確率が `epsilon_remote_base + epsilon_remote_need_coeff * child_need - epsilon_remote_benevolence_coeff * local_benevolence_mean` で定式化される。しかし `offer_help_probability()` はこの理論を完全に無視し、常に `0.3 + helper_benevolence * 0.4` の固定式を返す。AllParams の REMOTE_EXPLORATION_BASE/MAX/NEED_COEFF/BENEVOLENCE_COEFF の 4 定数（constants.rs:802-818）がいくら変動しても HELP プロトコルの挙動にまったく影響しない状態である。

また `advance_help_sessions()` の accept/success/harmful 確率も固定値で、constants.rs にも AllParams にも経路を持たない。RFC §41C はシミュレーションの全初期条件を較正対象として露出することを要求しているが、HELP プロトコルの挙動を決めるこれらの確率は較正不能である。

### WIRE 系列における位置づけ

WIRE 系列は 5 チケットで構成される（実装順）：
1. WIRE-B (#132 - reviewed): help_session ID フォーマット統一
2. WIRE-C (#133 - reviewed): generate_population 定数化
3. **WIRE-A (本チケット)**: offer_help_probability の epsilon_remote 経由化 + 8 定数化
4. WIRE-D: REMOTE_EXPLORE_* 定数のシミュレーション実装
5. WIRE-E: 残余ハードコード値の全数パラメーター化

WIRE-A が 3 番目なのは、HELP プロトコル本体の挙動を変更するため、WIRE-B/C の「挙動を変えない定数化」を先に行う安全設計による。

## Scope

1. **constants.rs に 8 定数を追加**: OFFER_HELP_BASE, OFFER_HELP_BV_COEFF, ADVANCE_HELP_ACCEPT_BASE, ADVANCE_HELP_ACCEPT_BV_COEFF, ADVANCE_HELP_SUCCESS_BASE, ADVANCE_HELP_SUCCESS_BV_COEFF, ADVANCE_HELP_HARMFUL_BASE, ADVANCE_HELP_HARMFUL_BV_COEFF
2. **AllParams に G3 グループ（8 パラメーター）を追加**: G3_OFFER_HELP_BASE (17) 〜 G3_ADVANCE_HELP_HARMFUL_BV_COEFF (24)。デフォルト値は constants.rs の値
3. **`offer_help_probability()` のシグネチャ変更 + epsilon_remote 経由化**: policy と village_mean_benevolence を引数に追加
4. **`advance_help_sessions()` の accept/success/harmful 確率を定数参照に変更**
5. **AllParams::to_sim_config_g1g2() に G3 値の反映を追加**
6. **`default_g1g2g4()` に G3 デフォルト値の実装**
7. **テスト A1-A9 の追加**

## Non-scope

- REMOTE_EXPLORE_* 4 定数のシミュレーション実装（村内/村外の区別）→ WIRE-D で対処
- `compute_epsilon_remote()` 自体の修正 → 既に reciprocity.rs に正しい実装あり
- ReciprocityLifecyclePolicy の epsilon_remote フィールドの変更 → 既に正しく定義済み
- 残余ハードコード値の全数パラメーター化 → WIRE-E で対処
- AllParams G4 関連の変更（WIRE-C で完了済み）

## Investigation

### 発見 1: offer_help_probability の固定式（simulation.rs:548-553）

```rust
fn offer_help_probability(helper_benevolence: f32) -> f64 {
    0.3 + helper_benevolence as f64 * 0.4
}
```

引数は `helper_benevolence: f32` のみ。`policy` や `village_mean_benevolence` を受け取らない。`0.3` と `0.4` がマジックナンバー。`compute_epsilon_remote()` が全く呼ばれていない。

呼び出し箇所（simulation.rs:569）でも policy や village 情報が渡されていない。

### 発見 2: advance_help_sessions の 6 つのマジックナンバー（simulation.rs:592-617）

```rust
let accept_prob = 0.5 + session.helper_benevolence as f64 * 0.3;  // 601
let success_prob = 0.6 + session.helper_benevolence as f64 * 0.25; // 616
let harmful_prob = 0.15 - session.helper_benevolence as f64 * 0.1; // 617
```

3 つの計算式に 6 つのマジックナンバー（0.5, 0.3, 0.6, 0.25, 0.15, 0.1）。全て constants.rs にも AllParams にも経路を持たない。

### 発見 3: compute_epsilon_remote は既に正しく実装されている（reciprocity.rs:527-535）

```rust
pub fn compute_benevolence_aware_remote_exploration(
    child_need: f32, local_benevolence_mean: f32, policy: &ReciprocityLifecyclePolicy,
) -> f32 {
    let raw = policy.epsilon_remote_base + policy.epsilon_remote_need_coeff * child_need
        - policy.epsilon_remote_benevolence_coeff * local_benevolence_mean;
    raw.clamp(0.0, policy.epsilon_remote_max)
}
```

正しい数式を実装済みだが `offer_help_probability()` から呼ばれていない。

### 発見 4: ReciprocityLifecyclePolicy の epsilon_remote 4 フィールド（event.rs:596-603）

```rust
pub epsilon_remote_base: f32,       // = REMOTE_EXPLORATION_BASE (0.05)
pub epsilon_remote_max: f32,        // = REMOTE_EXPLORATION_MAX (0.40)
pub epsilon_remote_need_coeff: f32, // = REMOTE_EXPLORATION_NEED_COEFF (1.0)
pub epsilon_remote_benevolence_coeff: f32, // = REMOTE_EXPLORATION_BENEVOLENCE_COEFF (1.0)
```

Default 実装（event.rs:655-660）で constants.rs の値を参照して初期化済み。ただし simulation コードで全く使われていない。

### 発見 5: G3_COUNT は既に定義済み（kind_world.rs:356-357）

```rust
pub const G3_COUNT: usize = 8;
```

WIRE-C で前方参照として G3_COUNT=8 が定義済み。G3 インデックスは 17-24。G4 は 25-27 で正しく配置済み。

### 発見 6: default_g1g2g4 の G3 は現在 0.0 (inactive)（kind_world.rs:525-529）

```rust
let g3_defaults = vec![0.0; G3_COUNT];  // 本チケットで constants.rs 値に置き換え
```

### 発見 7: to_sim_config_g1g2 は G3 をまだ反映していない（kind_world.rs:514-519）

G3 値を policy に反映するコードがない。本チケットで G3 値 → policy の経路を追加する。

### 参照観察レポート
- tickets/context/0133-m176-kw-wire-c-generate-population-allparams/observation-20260528-073548.md — 「WIRE-A では G3 グループ（8パラメーター）の実装が必要。G3_COUNT=8 は既に定義済み」

## Test Plan

### A1: epsilon=0 時に OFFER_HELP_BASE を返す
epsilon_remote_base=0, need_coeff=0, benevolence_coeff=0 の policy で offer_help_probability を呼び出し、結果が OFFER_HELP_BASE と一致することを検証。

### A2: epsilon 増加に伴い offer_help_probability が単調増加
policy の epsilon_remote_base を sweep（0.0 → 0.5）し、単調増加を検証。

### A3/A4/A5: advance_help_sessions の確率が各定数変更に応答
ADVANCE_HELP_ACCEPT_BASE, ADVANCE_HELP_SUCCESS_BASE, ADVANCE_HELP_HARMFUL_BASE の各定数を変更し、対応する分岐確率が変化することを検証。

### A6: AllParams G3 値変更で HELP プロトコルの挙動が変化
`default_g1g2g4()` で構築した AllParams の G3 値を変更し、to_sim_config_g1g2 経由で run_simulation の結果に変化が現れることを検証。

### A7: epsilon_remote_max=0 で offer 確率が OFFER_HELP_BASE に clamp
clamp 動作確認。

### A8: 既存テスト全 PASS
修正後も既存のテストスイートが全て通過すること。

## 計装方法・観測対象

### 計装方法
- simulation.rs の `mod tests` に A1-A7 のユニットテストを追加
- A1-A7 は固定値の assert テスト（観測不要、PASS/FAIL のみ）
- A8 は `cargo test` で全テスト通過を確認
- 観測テストとして、epsilon_remote 導入前後の offer_help_probability の分布比較を println! + --nocapture で出力

### 観測対象
- epsilon_remote 導入前後の offer 確率の分布比較（ヒストグラム）
- accept/success/harmful 確率の定数変更に対する感度
- AllParams G3 値と s_topology/j_help の相関
- サンプルサイズ: n=10,000 の random sampling で分布比較

### 較正計画
本チケットは定数化（パラメーター経路の確立）＋ epsilon_remote 経路の導入が目的。較正は後続の WIRE 全チケット（D, E）完了後に行う。

## Boy Scout Rule — 翻訳可能性計画

1. **offer_help_probability のマジックナンバー**: 名前付き定数に置き換え
2. **advance_help_sessions の 6 マジックナンバー**: 名前付き定数に置き換え
3. **offer_help_probability のシグネチャ**: helper_benevolence のみ → policy + village_mean_benevolence を追加

## Acceptance Criteria

- [ ] A1: epsilon=0 時に OFFER_HELP_BASE を返す
- [ ] A2: epsilon 増加に伴い offer_help_probability が単調増加
- [ ] A3/A4/A5: advance_help_sessions の確率が各定数変更に応答
- [ ] A6: AllParams G3 値変更で HELP プロトコルの挙動が変化
- [ ] A7: epsilon_remote_max=0 で offer 確率が OFFER_HELP_BASE に clamp
- [ ] A8: 既存テスト全 PASS
- [ ] 翻訳可能性の検証が通っている

## Notes

<!--
注: このコメントは人間向けの説明である。AI は以下の手順に従うこと。

- plan_path: /plan-ticket が plan.md を作成後に frontmatter に更新する
- implementation_path: /start-ticket が implementation.md を作成後に frontmatter に更新する
- review_report_path: /review-ticket が review.md を作成後に frontmatter に更新する
- observation_report_path: /start-ticket が observation-YYYYMMDD-HHmmss.md を作成後に frontmatter に最新パスを更新する

各コマンドのワークフロー手順が frontmatter 更新の正しい手順である。
-->

### 成果物

- 計画: context/0134-m176-kw-wire-a-offer-help-probability-epsilon-remote-allparams-3/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0134-m176-kw-wire-a-offer-help-probability-epsilon-remote-allparams-3/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0134-m176-kw-wire-a-offer-help-probability-epsilon-remote-allparams-3/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0134-m176-kw-wire-a-offer-help-probability-epsilon-remote-allparams-3/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
