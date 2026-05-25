---
ticket_id: 88
title: M1.76-3: 直接互恵性スコア compute_direct_reciprocity (F-1) 純粋関数実装
slug: m176-3-compute-direct-reciprocity-f-1
status: reviewed
created_at: 2026-05-25
updated_at: 2026-05-25
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0088-m176-3-compute-direct-reciprocity-f-1/observation-20260525-184956.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0088-m176-3-compute-direct-reciprocity-f-1/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0088-m176-3-compute-direct-reciprocity-f-1/review.md
---

# M1.76-3: 直接互恵性スコア compute_direct_reciprocity (F-1) 純粋関数実装

## Summary

RFC §15.10.2 式 F-1 で定義される直接互恵性スコア `compute_direct_reciprocity` の純粋関数を実装する。本関数は `ReciprocityEvent` の系列と `ReciprocityLifecyclePolicy` を受け取り、workflow i の直接互恵性スコア `R_i^dir ∈ [0, 1]` を計算する。本チケットは RFC §41C.3 の **M0.x（pure function validation）** フェーズに対応する。

## Background

M1.76-2 で `ReciprocityLifecyclePolicy` 構造体と `ReputationProfile` 拡張フィールドが定義された。次のステップとして、実際に互恵性スコアを計算する純粋関数が必要である。本関数は Reciprocity-Aware Survival の基盤計算エンジンであり、以降のチケット（間接互恵性 F-2、BenevolenceScore 集約 F-3、ReputationProfile 再計算 F-4/F-5）から呼び出される。

### 参照観察レポート

- `tickets/context/0087-m176-2-reciprocitylifecyclepolicy-reputationprofile/observation-20260525-183806.md` — データ型基盤の完了確認。ポリシー構造体と評判プロファイルが定義された。次チケットへの示唆として「ReciprocityLifecyclePolicy の theta_dir などがパラメータとして渡される設計」が記録されている。

## Scope

1. **`src/reciprocity.rs` モジュールの新規作成** — `compute_direct_reciprocity` 純粋関数を実装
2. **lib.rs への `pub mod reciprocity;` 追加** — モジュール公開
3. **式 F-1 の実装**: `σ( Σ_{j≠i} ω_ij^dir ( α_h H_ij + α_hs HS_ij - α_r RJ_ij - α_d DMG_ij ) exp(-ρ_dir Δt_ij) )`
4. **イベント種別→重み割り当てテーブル**: 各 `ReciprocityEventKind` variant を H/HS/RJ/DMG の4成分にマッピング
5. **時間減衰 `exp(-ρ_dir Δt_ij)`**: events の `virtual_clock` とパラメータ `now` の差分に基づく
6. **logistic sigmoid 関数**: `σ(x) = 1 / (1 + exp(-x))`、値域を `[0, 1]` に押し込む
7. **5件のユニットテスト + 1件の計装テスト**（Test Plan 参照）

## Non-scope

- `compute_indirect_reciprocity` (F-2) — チケット M1.76-4 で実装
- `BenevolenceScore` 集約 (F-3) — チケット M1.76-4 で実装
- `ReputationProfile` 再計算 (F-4, F-5) — チケット M1.76-5 で実装
- `ReciprocityEvent` のインジェスション・パイプライン — チケット M1.76-11 で実装
- EventBus や MetadataStore との結合 — 本チケットでは純粋関数として隔離
- マルチスレッド安全機構 — 本関数は純粋（`&[ReciprocityEvent]` + `&ReciprocityLifecyclePolicy` → `f32`）

## Investigation

### 物理的証拠

**証拠1: 既存型定義（event.rs L308-352）**
- `ReciprocityEventKind`: 8 variant（HelpOffered, HelpAccepted, HelpRejected, HelpExecuted, HelpSucceeded, HelpAbandoned, HarmfulMismatch, ReturnedFavor）
- `ReciprocityEvent`: 9フィールド構造体（event_id, mission_id, source_graph_id, target_graph_id, event_kind, weight, created_at, virtual_clock, trace_ref）
- `weight` フィールド（f32）が ω_ij^dir に相当

**証拠2: 既存ポリシー定義（event.rs L481-514）**
- `ReciprocityLifecyclePolicy`: 16フィールド構造体
- F-1 関連フィールド: `rho_direct_decay`（ρ_dir）、`policy_version`

**証拠3: 既存定数定義（constants.rs L625-648）**
- `RECIPROCITY_ALPHA_HELP` = 1.0（α_h, F-1）
- `RECIPROCITY_ALPHA_SUCCESS` = 2.0（α_hs, F-1）
- `RECIPROCITY_ALPHA_REJECT` = 1.0（α_r, F-1）
- `RECIPROCITY_ALPHA_HARM` = 2.0（α_d, F-1）
- `RECIPROCITY_DIRECT_DECAY_RHO` = 0.01（ρ_dir, F-1）

**証拠4: RFC §15.10.2 L4263-4286 — F-1 完全数式**
```
R_i^dir = σ( Σ_{j≠i} ω_ij^dir ( α_h H_ij + α_hs HS_ij - α_r RJ_ij - α_d DMG_ij ) exp(-ρ_dir Δt_ij) )
```
Normative constraint: α_h, α_hs > 0, α_r, α_d > 0。協力行為は非減少、裏切り・害は非増加 (MUST)。

**証拠5: モジュール構造（lib.rs L20-44）**
- `reciprocity` モジュールは未存在。新規 `pub mod reciprocity;` の追加が必要。

**証拠6: M1.76-2 観察レポート**
- データ型基盤は完了。M1.76-3 でポリシーがパラメータとして渡される設計を確認。

### 重み割り当てテーブル（設計判断）

`ReciprocityEventKind` → (H, HS, RJ, DMG) のマッピング:

| EventKind | H | HS | RJ | DMG |
|-----------|---|---|----|-----|
| HelpOffered | 1 | 0 | 0 | 0 |
| HelpAccepted | 1 | 0 | 0 | 0 |
| HelpRejected | 0 | 0 | 1 | 0 |
| HelpExecuted | 1 | 0 | 0 | 0 |
| HelpSucceeded | 0 | 1 | 0 | 0 |
| HelpAbandoned | 0 | 0 | 1 | 0 |
| HarmfulMismatch | 0 | 0 | 0 | 1 |
| ReturnedFavor | 0 | 1 | 0 | 0 |

このテーブルは `fn event_kind_weights(kind: &ReciprocityEventKind) -> (f32, f32, f32, f32)` として実装し、ハードコード値は名前付き定数として抽出する。

### 関数シグネチャ

```rust
pub fn compute_direct_reciprocity(
    events: &[ReciprocityEvent],
    now: u64,
    policy: &ReciprocityLifecyclePolicy,
) -> f32
```

- `events`: 同一 source_graph_id を持つ ReciprocityEvent のスライス
- `now`: 現在の VirtualClock 値（時間減衰計算の基準）
- `policy`: 較正パラメータ（α_h, α_hs, α_r, α_d, ρ_dir を含む）
- 戻り値: `[0, 1]` に clamp された f32 スコア

## Test Plan

### テストケース一覧

| # | 名称 | 種別 | 内容 |
|---|------|------|------|
| TC-1 | 空リスト | 正常系 | 空イベントリスト `[]` → `0.5`（sigmoid(0)）を返す |
| TC-2 | HelpSucceeded 単調増加 | 正常系 | `HelpSucceeded` のみの系列でイベント数増加に伴いスコアが非減少 |
| TC-3 | HarmfulMismatch 単調減少 | 正常系 | `HarmfulMismatch` のみの系列でイベント数増加に伴いスコアが非増加 |
| TC-4 | 時間減衰 | 正常系 | 同一内容の positive イベントでも `Δt` が大きい（古い）ほどスコアが低い |
| TC-5 | 係数ゼロ検証 | 異常系 | `α_h = 0, α_hs = 0` のとき他条件一定で正のスコア変化がゼロ |
| TC-6 | 値域 [0,1] + ρ_dir sweep | 計装 | n >= 10,000 のランダム系列で全出力が `[0, 1]` に拘束、ρ_dir sweep で減衰曲線計測 |

### モック依存

- テスト内で `ReciprocityEvent` を直接構築する（EventBus への依存なし、純粋関数のため）
- `SystemTime` の代わりに `virtual_clock` で時間軸を表現（決定論的テスト可能）

## 計装方法・観測対象

### 計装方法

- **固定シード PRNG**: `StdRng::seed_from_u64(12345)` で ReciprocityEvent 系列を生成
- **ρ_dir sweep**: `[0.001, 0.005, 0.01, 0.05, 0.1]` の5点で同一イベント系列に対する減衰曲線を計測
- **出力形式**: `println!` で JSON Lines 形式の観測データを `--nocapture` 経由で標準出力

### 観測対象

| 観測対象 | サンプルサイズ | 期待される性質 |
|---------|--------------|--------------|
| 値域 [0,1] 拘束 | n >= 10,000 | 全出力が閉区間内、NaN/Inf が存在しない |
| 単調性（正） | n = 1000 | イベント追加ごとにスコアが非減少 |
| 単調性（負） | n = 1000 | イベント追加ごとにスコアが非増加 |
| 時間減衰曲線 | ρ_dir 5点 × n = 100 | ρ_dir 大 → 減衰急峻、ρ_dir 小 → 減衰緩慢 |

### 較正計画

本チケットでは較正ループは実施しない（純粋関数実装の検証フェーズのため）。定数値は `constants.rs` の既存値（`RECIPROCITY_DIRECT_DECAY_RHO = 0.01` 等）を使用し、実際の較正は M1.76-16（多目的較正目的関数 F-16）で行う。

## Boy Scout Rule — 翻訳可能性計画

### 改善対象

1. **新規 `src/reciprocity.rs`**: 関数名 `compute_direct_reciprocity` は動詞句であり可読性良好。内部の重み割り当ては `event_kind_weights` 関数に抽出し、oneline match で散文的に読めるようにする。
2. **sigmoid 関数**: インライン計算ではなく `fn logistic_sigmoid(x: f32) -> f32` として抽出し、「ロジスティックシグモイド関数」の意図を関数名で語らせる。
3. **時間減衰**: 減衰計算を `fn time_decay(delta: u64, rho: f32) -> f32` として抽出。数式 `exp(-ρΔt)` の計算であることを関数名で表現。
4. **ハードコード禁止**: 重みマッピングテーブルの値（0/1）以外でマジックナンバー（sigmoid の clamp 値など）は全て名前付き定数として定義。
5. **コメント**: 数式 F-1 の日本語訳をモジュールドキュメントとして記載。関数内部では「なぜ」の説明のみ。

### 影響範囲外の既存コード

本チケットでは新規ファイル作成のみで既存コードへの大規模な修正は行わないが、`event.rs` 内の `ReciprocityEventKind` 定義行付近で翻訳不可能なパターンがあれば発見次第修正する。

## Acceptance Criteria

- [ ] `compute_direct_reciprocity` が空リストに対して 0.5 を返す
- [ ] HelpSucceeded 系列で単調非減少を満たす
- [ ] HarmfulMismatch 系列で単調非増加を満たす
- [ ] 時間減衰が正しく機能（古いイベントほど低スコア）
- [ ] α_h = α_hs = 0 で正のスコア変化がゼロ
- [ ] n >= 10,000 のランダム系列で全出力が `[0, 1]` に拘束
- [ ] 翻訳可能性の検証が通っている（関数名、変数名、抽出粒度）
- [ ] 既存テストが通過している
- [ ] RFC §15.10.2 との無矛盾確認済み

## Notes

- `compute_direct_reciprocity` は純粋関数（副作用ゼロ）であり、同じ入力に対して常に同じ出力を返す
- 時間軸は `ReciprocityEvent.virtual_clock` を使用し、`SystemTime` は使用しない（決定論的テストのため）
- 本関数は ReputationProfile.direct_score の計算に使用される（F-4, F-5 で呼び出し）
- `ω_ij^dir`（イベントの重み）は `ReciprocityEvent.weight` フィールドをそのまま使用する

### 成果物

- 計画: `context/0088-m176-3-compute-direct-reciprocity-f-1/plan.md`（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: `context/0088-m176-3-compute-direct-reciprocity-f-1/implementation.md`（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: `context/0088-m176-3-compute-direct-reciprocity-f-1/review.md`（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: `context/0088-m176-3-compute-direct-reciprocity-f-1/observation-YYYYMMDD-HHmmss.md`（未作成、/start-ticket 観測テスト実行時に作成）
