---
ticket_id: 89
title: M1.76-4: 間接互恵性スコア compute_indirect_reciprocity (F-2) + BenevolenceScore 集約 (F-3)
slug: m176-4-compute-indirect-reciprocity-f-2-benevolence-f-3
status: reviewed
created_at: 2026-05-26
updated_at: 2026-05-26
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0089-m176-4-compute-indirect-reciprocity-f-2-benevolence-f-3/plan.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0089-m176-4-compute-indirect-reciprocity-f-2-benevolence-f-3/observation-20260526-075128.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0089-m176-4-compute-indirect-reciprocity-f-2-benevolence-f-3/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0089-m176-4-compute-indirect-reciprocity-f-2-benevolence-f-3/review.md
---

# M1.76-4: 間接互恵性スコア compute_indirect_reciprocity (F-2) + BenevolenceScore 集約 (F-3)

## Summary

RFC §15.10.2 式 F-2 および F-3 で定義される間接互恵性スコア `compute_indirect_reciprocity` と BenevolenceScore 集約 `compute_benevolence_score` の純粋関数を実装する。間接互恵性スコアは「社会全体から見た善良さ」を表し、直接互恵性と分離して保持される (MUST)。BenevolenceScore は評判・直接互恵性・間接互恵性の合成量として定義される。本チケットは RFC §41C.3 の **M0.x（pure function validation）** フェーズに対応する。

## Background

M1.76-3 で `compute_direct_reciprocity` (F-1) が実装され、直接互恵性スコア計算の基盤が整った。次のステップとして、HELP network 上の global benevolence（間接互恵性）を計算する F-2、および直接互恵性・間接互恵性・評判を統合する BenevolenceScore 集約 (F-3) が必要である。

間接互恵性は「社会全体から見たワークフローの善良さ」であり、直接互恵性（二者間の相互関係）とは分離して保持される。F-3 で合成される BenevolenceScore は、以降のチケット（GC hazard F-7/F-8/F-9、Helper quality F-11、Child protection F-10）の入力となる。

### 参照観察レポート

- `tickets/context/0088-m176-3-compute-direct-reciprocity-f-1/observation-20260525-184956.md` — 直接互恵性スコア実装完了確認。sigmoid、重み割り当て、時間減衰の基盤関数が実装された。
- M1.76-3 の `logistic_sigmoid` 関数は F-2 でも共用可能。同じ `src/reciprocity.rs` モジュールに追加する。

## Scope

1. **`compute_indirect_reciprocity` 関数の実装** — 式 F-2 の純粋関数
2. **`compute_benevolence_score` 関数の実装** — 式 F-3 の純粋関数
3. **F-2 β 係数定数の追加** — `src/constants.rs` に β_1〜β_5 を新規定義
4. **F-3 w_rep 定数の追加** — `REPUTATION_WEIGHT_REPUTATION` を新規定義
5. **6件のテスト**（Test Plan 参照）

## Non-scope

- `recompute_reputation` (F-4, F-5) — チケット M1.76-5 で実装
- GC hazard with benevolence (F-7, F-8, F-9) — チケット M1.76-6 で実装
- Child protection (F-10) — チケット M1.76-7 で実装
- Helper quality score (F-11) — チケット M1.76-8 で実装
- `ReciprocityEvent` のインジェスション・パイプライン — チケット M1.76-11 で実装
- EventBus や MetadataStore との結合 — 本チケットでは純粋関数として隔離
- ReputationProfile の直接更新 — 本関数は純粋（値を返すのみ）

## Investigation

### 物理的証拠

**証拠1: 既存 reciprociry.rs モジュール（L1-345）**
- `compute_direct_reciprocity`、`logistic_sigmoid`、`time_decay`、`event_kind_weights` が実装済み
- F-2 では時間減衰は不要（式 F-2 に exp(-ρΔt) 項がない）。ただし `logistic_sigmoid` は共用可能。
- テストパターン（空リスト、単調性、値域、sweep）が確立されている。

**証拠2: 既存定数（constants.rs L625-683）**
- F-1 関連: `RECIPROCITY_ALPHA_HELP` (1.0), `RECIPROCITY_ALPHA_SUCCESS` (2.0), `RECIPROCITY_ALPHA_REJECT` (1.0), `RECIPROCITY_ALPHA_HARM` (2.0), `RECIPROCITY_DIRECT_DECAY_RHO` (0.01)
- F-3 関連: `REPUTATION_WEIGHT_DIRECT` (0.35), `REPUTATION_WEIGHT_INDIRECT` (0.35)
- **未定義定数**: β_1〜β_5、`REPUTATION_WEIGHT_REPUTATION`（w_rep）。本チケットで新規追加する。

**証拠3: RFC §15.10.2 L4288-4321 — F-2 / F-3 完全数式**
```
F-2: R_i^ind = σ( β_1·C_i^help + β_2·A_i^village + β_3·U_i^accepted + β_4·Q_i^success - β_5·B_i^harm )
F-3: B_i = w_dir · R_i^dir + w_ind · R_i^ind + w_rep · Rep_i
```

**証拠4: VillageMetrics 構造体（village.rs L208-227）**
- 村の統計メトリクス（position_drifts, village_jaccards, churns, helper_jsds, helper_counts, child_survival_count 等）
- F-2 の `A_i^village`（村参加度）は VillageMetrics から導出することを前提とする。
- `accepted_offer_rate` と `help_success_rate` は `ReputationProfile` に既存フィールドとして存在（event.rs L431-432）。

**証拠5: F-2 各項の導出方針**
- `C_i^help`（中心性）: `ReputationProfile.village_centrality`（event.rs L434）をそのまま使用
- `A_i^village`（村参加度）: VillageMetrics.child_survival_count / total_child_count の比率などを指標化
- `U_i^accepted`（受諾率）: `ReputationProfile.accepted_offer_rate` を使用
- `Q_i^success`（成功貢献率）: `ReputationProfile.help_success_rate` を使用
- `B_i^harm`（負評価）: `ReputationProfile.harm_event_count` を正規化して使用（または他負イベントの統合指標）
- 全ての入力は `[0, 1]` に正規化されていることを前提とする。

**証拠6: ReciprocityLifecyclePolicy（event.rs L481-534）**
- F-2 の β 係数はポリシーに含まれていない。本チケットでは一旦 `constants.rs` の定数を使用する簡素な設計とする（ポリシー拡張は将来のチケット）。

### 関数シグネチャ

```rust
/// 間接互恵性スコア R_i^ind を計算する (F-2)。
///
/// 式 F-2:
///   R_i^ind = σ( β_1·C_i^help + β_2·A_i^village + β_3·U_i^accepted + β_4·Q_i^success - β_5·B_i^harm )
///
/// # 引数
/// - `centrality`: C_i^help — helper network 上の中心性 [0, 1]
/// - `village_participation`: A_i^village — local village 参加度 [0, 1]
/// - `accepted_rate`: U_i^accepted — offer 受諾率 [0, 1]
/// - `success_rate`: Q_i^success — 支援成功率 [0, 1]
/// - `harm_score`: B_i^harm — 負評価スコア [0, 1]
///
/// # 戻り値
/// - [0, 1] の範囲に正規化された f32 スコア
/// - 全入力 0 のとき 0.5（sigmoid(0)）を返す
pub fn compute_indirect_reciprocity(
    centrality: f32,
    village_participation: f32,
    accepted_rate: f32,
    success_rate: f32,
    harm_score: f32,
) -> f32

/// BenevolenceScore B_i を計算する (F-3)。
///
/// 式 F-3:
///   B_i = w_dir · R_i^dir + w_ind · R_i^ind + w_rep · Rep_i
///
/// 係数は非負、かつ w_dir + w_ind + w_rep = 1 （推奨）。
///
/// # 引数
/// - `direct_score`: R_i^dir — 直接互恵性スコア [0, 1]
/// - `indirect_score`: R_i^ind — 間接互恵性スコア [0, 1]
/// - `reputation`: Rep_i — 評判スコア (final_score) [0, 1]
///
/// # 戻り値
/// - [0, 1] の範囲にクランプされた f32 スコア
pub fn compute_benevolence_score(
    direct_score: f32,
    indirect_score: f32,
    reputation: f32,
) -> f32
```

**設計判断**: F-2 は `VillageMetrics` 構造体全体を引数に取らず、必要な値を個別の f32 引数で受け取る純粋関数とする。これにより:
1. テスト容易性（任意の値を注入可能）
2. VillageMetrics への結合度低下
3. 将来的な別指標への差し替え容易性
を確保する。

**F-3 のポリシー引数**: 現時点では `ReciprocityLifecyclePolicy` を引数に取らず、定数直接使用とする（`constants::REPUTATION_WEIGHT_DIRECT` 等）。ポリシー経由のパラメータ化は M1.76-16（較正ハーネス）以降で検討する。

## Test Plan

### テストケース一覧

| # | 名称 | 種別 | 内容 |
|---|------|------|------|
| TC-1 | 全成分ゼロ | 正常系 | 全入力 0 → `R_i^ind = 0.5`（sigmoid(0)）を返す |
| TC-2 | 中心性単調増加 | 正常系 | `C_i^help` を `[0, 1]` で sweep したとき `R_i^ind` が単調増加 |
| TC-3 | 負評価単調減少 | 正常系 | `B_i^harm` 増加に伴い `R_i^ind` が単調減少 |
| TC-4 | BenevolenceScore bounded | 正常系 | `B_i` が `[0, 1]` に bounded される（n >= 10,000） |
| TC-5 | w_dir = 1 の退化 | 正常系 | `w_dir = 1, w_ind = 0, w_rep = 0` のとき `B_i = R_i^dir` |
| TC-6 | w_rep = 1 の退化 | 正常系 | `w_dir = 0, w_ind = 0, w_rep = 1` のとき `B_i = Rep_i` |

### モック依存

- テスト内で直接 f32 値を構築（イベントストアや VillageMetrics への依存なし、純粋関数のため）
- `ReputationProfile` の構築は補助的（BenevolenceScore の引数として f32 を直接渡す）

## 計装方法・観測対象

### 計装方法

- **固定シード PRNG**: `StdRng::seed_from_u64(12345)` でランダム入力を生成
- **β 係数 sweep**: β_1〜β_5 の各係数を個別に `[0.5, 1.0, 2.0, 4.0]` で sweep し、感度曲線 `∂R_i^ind / ∂β_k` を中心差分で推定
- **応答曲面**: `C_i^help`（中心性）と `B_i^harm`（負評価）の 2 次元パラメータ空間（[0,1]×[0,1]、11×11 グリッド）上で `R_i^ind` の応答曲面を観測
- **出力形式**: `println!` で CSV 形式の観測データを `--nocapture` 経由で標準出力

### 観測対象

| 観測対象 | サンプルサイズ | 期待される性質 |
|---------|--------------|--------------|
| 値域 [0,1] 拘束 | n >= 10,000 | 全出力が閉区間内、NaN/Inf が存在しない |
| 中心性 sweep 単調性 | n = 100 | C_i^help 増加で R_i^ind が非減少 |
| 負評価 sweep 単調性 | n = 100 | B_i^harm 増加で R_i^ind が非増加 |
| 応答曲面グリッド | 11×11 grid | 凸性、中心性と負評価の直交性 |
| β 感度曲線 | 4値 × 5変数 = 20点 | ∂R_i^ind/∂β_k の符号と大きさ |

### 較正計画

本チケットでは較正ループは実施しない（純粋関数実装の検証フェーズ）。β 係数の初期値は以下の推奨値を設定し、実際の較正は M1.76-16（多目的較正目的関数 F-16）で行う：

- `β_1 = 1.0`（中心性、中程度の寄与）
- `β_2 = 1.0`（村参加度、中程度の寄与）
- `β_3 = 1.0`（受諾率、中程度の寄与）
- `β_4 = 2.0`（成功貢献率、高めの寄与）
- `β_5 = 2.0`（負評価、高めのペナルティ）
- `w_rep = 0.30`（w_dir + w_ind + w_rep = 1 を満たす値）

## 追加定数定義

以下の定数を `src/constants.rs` に追加する：

```rust
/// 間接互恵性 β_1 — 中心性 C_i^help の係数 (F-2) (Calibration Candidate)
/// F-2: β_1 * C_i^help。β_1 > 0 (MUST)。
/// Default: 1.0, 感度分析推奨範囲: 0.5-4.0
pub const INDIRECT_BETA_CENTRALITY: f32 = 1.0;

/// 間接互恵性 β_2 — 村参加度 A_i^village の係数 (F-2) (Calibration Candidate)
/// Default: 1.0, 感度分析推奨範囲: 0.5-4.0
pub const INDIRECT_BETA_VILLAGE_PARTICIPATION: f32 = 1.0;

/// 間接互恵性 β_3 — 受諾率 U_i^accepted の係数 (F-2) (Calibration Candidate)
/// Default: 1.0, 感度分析推奨範囲: 0.5-4.0
pub const INDIRECT_BETA_ACCEPTED_RATE: f32 = 1.0;

/// 間接互恵性 β_4 — 成功貢献率 Q_i^success の係数 (F-2) (Calibration Candidate)
/// Default: 2.0, 感度分析推奨範囲: 1.0-4.0
pub const INDIRECT_BETA_SUCCESS_RATE: f32 = 2.0;

/// 間接互恵性 β_5 — 負評価 B_i^harm の係数 (F-2) (Calibration Candidate)
/// F-2: -β_5 * B_i^harm。β_5 > 0 (MUST)。
/// Default: 2.0, 感度分析推奨範囲: 1.0-4.0
pub const INDIRECT_BETA_HARM_SCORE: f32 = 2.0;

/// BenevolenceScore 集約重み w_rep (F-3) (Calibration Candidate)
/// F-3: B_i = w_dir * R_dir + w_ind * R_ind + w_rep * Rep
/// Default: 0.30 (w_dir=0.35 + w_ind=0.35 + w_rep=0.30 = 1.0),
/// 感度分析推奨範囲: 0.15-0.45
pub const REPUTATION_WEIGHT_REPUTATION: f32 = 0.30;
```

## Boy Scout Rule — 翻訳可能性計画

### 改善対象

1. **既存 `reciprocity.rs`**: F-2 追加関数の命名は `compute_indirect_reciprocity`（動詞句）とし、内部で `logistic_sigmoid` を共用。5 つの β 項の線形結合はインラインの散文的な記述とする。
2. **引数名**: `centrality`, `village_participation`, `accepted_rate`, `success_rate`, `harm_score` は各項の意味を明確に表現。
3. **BenevolenceScore**: 関数名 `compute_benevolence_score` で「慈悲スコアを計算する」という意図を明確に。内部での重み付き線形和は `w_dir * direct_score + w_ind * indirect_score + w_rep * reputation` と散文的に書く。
4. **ハードコード禁止**: β 値、w_rep 値は全て `constants.rs` の名前付き定数として定義。
5. **コメント**: 数式 F-2 / F-3 をモジュールドキュメントに追記。関数内部では「なぜ」の説明のみ。

### 影響範囲外の既存コード

`constants.rs` への定数追加はおこなうが、既存定数（REPUTATION_WEIGHT_DIRECT 等）の値は変更しない。既存 `village.rs` の `VillageMetrics` は直接使用しない（個別 f32 引数で受け取るため）。

## Acceptance Criteria

- [ ] `compute_indirect_reciprocity` が全入力 0 に対して 0.5 を返す
- [ ] 中心性 `C_i^help` sweep で単調増加
- [ ] 負評価 `B_i^harm` sweep で単調減少
- [ ] `compute_benevolence_score` が n >= 10,000 で常に `[0, 1]` に bounded
- [ ] `w_dir = 1, w_ind = 0, w_rep = 0` で `B_i = R_i^dir`
- [ ] `w_dir = 0, w_ind = 0, w_rep = 1` で `B_i = Rep_i`
- [ ] 応答曲面グリッド（中心性 × 負評価 11×11）が観測可能
- [ ] β 係数 sweep による感度曲線が観測可能
- [ ] 既存テスト（M1.76-3 の F-1 テスト）が通過している
- [ ] RFC §15.10.2 との無矛盾確認済み

## Notes

- `compute_indirect_reciprocity` と `compute_benevolence_score` は共に純粋関数（副作用ゼロ）
- F-2 には時間減衰項がない（RFC §15.10.2 L4292-4299 に exp(-ρΔt) なし）。社会的な「印象」の持続は直接互恵性でモデル化され、間接互恵性は累積的な社会的評価として即時反映される設計。
- `logistic_sigmoid` 関数（既存）の可視性を `pub(crate)` に変更し、F-2 からも共用することを検討（F-1 では private fn として定義）。
- F-3 の `w_dir + w_ind + w_rep = 1` は推奨であって強制ではない。テストでは和が 1 でないケースも検証する。
- `Rep_i` は `ReputationProfile.final_score` を参照するが、本関数は純粋関数として f32 を直接受け取る。

### 成果物

- 計画: `context/0089-.../plan.md`（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: `context/0089-.../implementation.md`（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: `context/0089-.../review.md`（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: `context/0089-.../observation-YYYYMMDD-HHmmss.md`（未作成、/start-ticket 観測テスト実行時に作成）
