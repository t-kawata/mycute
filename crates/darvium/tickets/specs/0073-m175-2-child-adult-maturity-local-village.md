---
ticket_id: 73
title: M1.75-2: Child / Adult maturity 判定器および Local Village 構成ロジックの実装
slug: m175-2-child-adult-maturity-local-village
status: reviewed
created_at: 2026-05-24
updated_at: 2026-05-24
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0073-m175-2-child-adult-maturity-local-village/plan.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0073-m175-2-child-adult-maturity-local-village/observation-20260524-141956.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0073-m175-2-child-adult-maturity-local-village/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0073-m175-2-child-adult-maturity-local-village/review.md
---

# M1.75-2: Child / Adult maturity 判定器および Local Village 構成ロジックの実装

## Summary

v2.3-e Child Support Villages の中核として、ワークフローの成熟度（Child / Adult）を判定する純粋関数と、Child の近傍 Adult 集合として Local Village を構成するロジックを実装する。RFC §41B.3 で定義される経験値・信頼・レピュテーションに基づく二値成熟判定、および TopK / radius 両方式の近傍選択をカバーする。

## Background

RFC §41B.3 では、以下の2つの区別が定義されている：

1. **Child**: `experiencecount(G) < MINSURVIVALEXPERIENCE` を満たすワークフロー。経験値猶予期間（Experience Grace Period）規律と整合し、GC 禁止を強化してはならない。
2. **Adult**: `E(G) ≥ E_adult ∧ T(G) ≥ T_adult ∧ R(G) ≥ R_adult` をすべて満たすワークフロー。経験値・信頼複合スコア・レピュテーション最終スコアの3軸で判定される。

Child の **Local Village** は静的なクラスではなく、Adult ワークフローの導出近傍（derived neighborhood）である。規範的なデフォルトは TopK 近傍（式 41B-6）、代替として半径形式（式 41B-7）をサポートする。

本チケットは M1.75 マイルストーンの基盤の2つ目であり、後続チケット（M1.75-3 HELP プロトコル、M1.75-4 Adult offer policy / Child consent policy 等）に成熟判定器および村構成ロジックを提供する。

すでに完了している前チケット M1.75-1 により `SpacePositionEmbedding` / `VillagePosition` / `l2_distance` が利用可能である。本チケットはこれらを直接用いて近傍選択を行う。

## Scope

### 実装するもの

1. **`WorkflowMaturity` 列挙型**: `{Child, Adult}` の2値。RFC §41B.3 の二値分類を表現する。
2. **`classify_maturity(experience_count: u64, trust_composite: f64, reputation_finalscore: f64) -> WorkflowMaturity`**: 純粋関数。経験値・信頼・レピュテーションの3軸で成熟度を判定する。各閾値は `constants.rs` の較正候補定数を参照する。
3. **`LocalVillage` 構造体**: `{ child_id: WorkflowGraphId, adult_ids: Vec<WorkflowGraphId>, centroid: SpacePositionEmbedding, radius: f64 }`。Child の近傍 Adult 集合を表現する。
4. **`build_local_village_topk(child_pos: &VillagePosition, adults: &[AdultCandidate], k: usize) -> LocalVillage`**: TopK 方式（式 41B-6）。child 位置からの L2 距離昇順で最大 k 件の Adult を選抜する。
5. **`build_local_village_radius(child_pos: &VillagePosition, adults: &[AdultCandidate], d_max: f64) -> LocalVillage`**: Radius 方式（式 41B-7）。child 位置からの L2 距離が d_max 以下の Adult をすべて選抜する。
6. **`AdultCandidate` 構造体**: 村構成の中間表現。`{ id: WorkflowGraphId, position: VillagePosition, consistency: ConsistencyStateTag, is_adult_maturity: bool }`。フィルタリング対象フィールドを含む。
7. **`filter_adult_candidates(candidates: Vec<AdultCandidate>) -> Vec<AdultCandidate>`**: `ConsistencyState != Committed` および adult maturity 未達の workflow を候補から除外するフィルタ。LifecycleState 型が未実装のため、本チケットでは `is_adult_maturity: bool` で代替する。
8. **`MIN_SURVIVAL_EXPERIENCE`**, **`E_ADULT_THRESHOLD`**, **`T_ADULT_THRESHOLD`**, **`R_ADULT_THRESHOLD`** 定数を `constants.rs` に追加する。

### 実装ファイル

RFC §41B.17 の推奨実装分割に従い、本チケットのコードは新規ファイル `src/village.rs` に配置する。これにより `src/spaceposition.rs`（M1.75-1）と責務分離される。

`constants.rs` に成熟度判定用の較正候補定数を追加する。

### 依存関係

- **M1.75-1 成果物**: `VillagePosition` / `SpacePositionEmbedding` / `l2_distance` / `update_space_position`（`src/spaceposition.rs`）
- **既存型**: `WorkflowGraphId`（`src/types.rs`）、`ConsistencyStateTag`（`src/types.rs`）

### 機能的結合

本チケットの成果物は以下の後続チケットから直接利用される：

- M1.75-3: HELP 状態機械 → Adult 選抜と Child 判定の組み合わせ
- M1.75-4: Adult offer policy / Child consent policy → maturity 判定結果
- M1.75-6: Helper weighting / remote exploration → village 構成結果を helper 候補として利用

## Non-scope

- `ReputationProfile` 構造体の定義（M1.76-1 で実装予定）。`classify_maturity` は `reputation_finalscore: f64` をスカラー値として受け取る。
- `LifecycleState` 列挙型の定義。現時点では `ConsistencyStateTag` で代替する。
- HELP プロトコル状態機械（M1.75-3）。
- Adult offer policy / Child consent policy（M1.75-4）。
- Village stability / dynamicity メトリクス（M1.75-7）。

## Investigation

### 参照観察レポート

- `tickets/context/0072-m175-1-spacepositionembedding-villageposition/observation-20260524-140442.md` — M1.75-1 の全19テスト PASS。位置更新ダイナミクス（指数平滑化）の正しさ、f32 精度内の収束、EventBus 結合を確認。次チケットへの示唆として「f32 収束判定 ε は 5e-6 以上を推奨」「proptest 戦略では integer 百分率→f64 変換でシリアライズ精度問題を回避」を記録。本チケットの `l2_distance` 依存に直接影響する。

### 既存コード調査結果

**constants.rs**: 村関連の成熟度定数は未定義。全4定数を新規追加する必要がある。

**types.rs**:
- `TrustProfile`（L4763）は `composite()` メソッドを持ち `[0.0, 1.0]` 範囲の複合スコアを返す。
- `ConsistencyState`（L503）は `Committed` / `Pending` / `NeedsRepair` / `Quarantined` の4状態を持つ。
- `ConsistencyStateTag`（L577）はフィルタリング用の簡易タグとして利用可能。
- `WorkflowGraphId` は `pub type WorkflowGraphId = u64` で定義済み。
- `ReputationProfile` / `LifecycleState` は未実装。

**spaceposition.rs** (M1.75-1 成果物):
- `SpacePositionEmbedding(Option<[f32; 3]>)` — newtype ラッパー、`from()` / `unknown()` / `inner()` アクセサ完備。
- `VillagePosition` — `position: [f32; 3]`, `last_updated_vt: u64`。
- `l2_distance(a: &VillagePosition, b: &VillagePosition) -> f64` — RFC 式 41B-5 のユークリッド距離。spaceposition.rs の mod tests で同一性・非負性・三角不等式を検証済み。

**lib.rs**: `pub use types::{WorkflowGraphId, ...}` 等で公開 API を構成。本チケットの追加型・関数は `lib.rs` に公開 API として追加する。

### 物理的証拠

- 成熟度関連定数が constants.rs に存在しないこと → 新規追加対象として確定
- ReputationProfile.finalscore / LifecycleState が未実装 → classify_maturity はスカラー値を受け取る設計で回避
- ConsistencyStateTag がフィルタリング用に使用可能 → AdultCandidate のフィルタ条件として直接利用
- l2_distance が spaceposition.rs で実装済み → 近傍選択の距離計算に再利用
- M1.75-1 観察レポートで f32 収束判定 ε=5e-6 推奨 → build_local_village の近傍判定に活用

### 事実誤認訂正（updated at plan time: 2026-05-24）

| 項目 | spec 記述 | 実際のコード | 影響 |
|------|----------|------------|------|
| WorkflowGraphId の型 | `pub type WorkflowGraphId = u64` | `pub type WorkflowGraphId = String`（types.rs:19） | LocalVillage の adult_ids は Vec\<String\> となる。設計上の変更不要 |
| l2_distance シグネチャ | `l2_distance(a: &VillagePosition, b: &VillagePosition) -> f64` | `l2_distance(a: &[f32; 3], b: &[f32; 3]) -> f64`（spaceposition.rs:146） | VillagePosition の position フィールド（`[f32; 3]`）を引数に渡す。`l2_distance(&child_pos.position, &adult_pos.position)` で呼び出し可能。|

## Test Plan

### テスト構成

`src/village.rs` 内の `mod tests` に以下を実装する。全テストは固定シード `StdRng::seed_from_u64(12345)` での再現可能な人工データを使用する。

### T-1: classify_maturity — Child 判定（経験値不足）

- experience_count = MIN_SURVIVAL_EXPERIENCE - 1（境界値-1）で `WorkflowMaturity::Child` を返すこと
- trust / reputation が adult 閾値を超えていても Child と判定されること（経験値不足が支配的）

### T-2: classify_maturity — Adult 判定（全軸充足）

- experience_count ≥ E_ADULT_THRESHOLD、trust_composite ≥ T_ADULT_THRESHOLD、reputation_finalscore ≥ R_ADULT_THRESHOLD（境界値）で `WorkflowMaturity::Adult` を返すこと

### T-3: classify_maturity — 信頼不足で Child

- experience_count, reputation が閾値以上だが trust_composite が閾値未満の場合、Child を返すこと

### T-4: classify_maturity — レピュテーション不足で Child

- experience_count, trust が閾値以上だが reputation_finalscore が閾値未満の場合、Child を返すこと

### T-5: classify_maturity — 全軸ギリギリ不足（境界値 - ε）

- 全3軸が閾値から `f64::EPSILON` だけ低い場合、Child を返すこと

### T-6: classify_maturity — 全軸閾値超過（境界値 + ε）

- 全3軸が閾値から `f64::EPSILON` だけ高い場合、Adult を返すこと

### T-7: classify_maturity — 極値入力

- experience_count = 0（未経験）で Child を返すこと
- 全軸最大値（u64::MAX, 1.0, 1.0）で Adult を返すこと

### T-8: filter_adult_candidates — ConsistencyState 除外

- `ConsistencyStateTag::Pending`、`NeedsRepair`、`Quarantined` の候補がすべて除外されること
- `ConsistencyStateTag::Committed` の候補は保持されること

### T-9: filter_adult_candidates — Adult maturity 未達除外

- `is_adult_maturity: false` の候補が除外されること
- `is_adult_maturity: true` の候補は保持されること

### T-10: filter_adult_candidates — 複合フィルタ

- 複数の除外条件に合致する候補が重複除去されること
- 全条件を満たす候補のみが保持されること
- 空リスト入力に対して空リストを返すこと

### T-11: build_local_village_topk — 基本的な選抜

- 人工配置した adult 群（5件）に対して、child 近傍から距離昇順で k=3 件が選抜されること
- 選抜された adult_id の順序が距離昇順であること

### T-12: build_local_village_topk — k が adult 総数より大きい場合

- 全 adult が選抜されること（切り詰めなし）

### T-13: build_local_village_topk — k = 0

- 空の village（adult_ids = []）が返ること

### T-14: build_local_village_topk — 同距離のタイ処理

- 同一距離に複数の adult がいる場合、任意の順序で最大 k 件が選抜されること（順序不定許容）

### T-15: build_local_village_radius — 半径内選抜

- d_max 内の adult のみが選抜されること
- d_max 外の adult は除外されること

### T-16: build_local_village_radius — 半径内に adult 不在

- 空の village が返ること

### T-17: build_local_village_radius — 全 adult が半径内

- 全 adult が選抜されること

### T-18: centroid 計算 — 単一 adult

- 単一 adult の位置が centroid と一致すること

### T-19: centroid 計算 — 複数 adult

- 複数 adult の重心（算術平均）が正しく計算されること

### T-20: centroid 計算 — 空 village

- 空 village の centroid は `SpacePositionEmbedding::unknown()` であること

### T-E1: 計装サマリ出力

- 全 T-1〜T-20 テストを実行し、PASS/FAIL サマリを標準出力に構造化テキストで出力すること

## 計装方法・観測対象

### 計装方法

- `src/village.rs` の `mod tests` 内に全ユニットテストを実装する。
- 全テストは `println!` で構造化テキスト（テスト名・期待値・実測値）を `--nocapture` 経由で標準出力に書き出す。
- 人工データ生成は固定シード `StdRng::seed_from_u64(12345)` を使用し完全再現を保証する。
- Adult 位置の人工配置は `VillagePosition::new()` を使用し、Child 位置との距離が既知の値になるよう設計する。

### 観測対象

- T-1〜T-20 全テストの PASS/FAIL カウント
- classify_maturity の境界値 ±1 ステップでの判定切り替わり確認
- build_local_village の距離順位・選抜数・空 village 発生条件
- フィルタ前後の候補数の減少率

### 較正計画

本チケットでは単独の目的関数 J(θ) を定義しない（M1.75-11 の J_village(θ) の構成要素となる）。以下の定数を初期値で設定し、後続チケットで多目的最適化される：

| 定数 | 初期値 | 分類 | 根拠 |
|------|--------|------|------|
| `MIN_SURVIVAL_EXPERIENCE` | 5 | Calibration Candidate | RFC §41B.3 推奨デフォルト |
| `E_ADULT_THRESHOLD` | 20 | Calibration Candidate | v1.7 ライフサイクル保護との整合 |
| `T_ADULT_THRESHOLD` | 0.70 | Calibration Candidate | TrustProfile.composite の中間値 |
| `R_ADULT_THRESHOLD` | 0.70 | Calibration Candidate | ReputationProfile.finalscore の中間値 |

## Boy Scout Rule — 翻訳可能性計画

本チケットで新規作成する `src/village.rs` において、以下の翻訳可能性基準を最初から満たしたコードを書く：

1. **関数名は動詞句**: `classify_maturity`（成熟度を分類する）、`filter_adult_candidates`（成人候補をフィルタリングする）、`build_local_village_topk`（ローカルビレッジを TopK で構築する）
2. **構造体名は名詞**: `WorkflowMaturity`（ワークフロー成熟度）、`LocalVillage`（ローカルビレッジ）、`AdultCandidate`（成人候補）
3. **一関数一責務**: `classify_maturity` は判定のみ、`filter_adult_candidates` はフィルタのみ、`build_local_village_*` は村構成のみ。責務の混在を禁止する。
4. **ハードコード値は名前付き定数**: 閾値は `constants.rs` に定義し、テストからも定数名で参照する。マジックナンバーの埋め込み禁止。
5. **エラー握りつぶし禁止**: `unwrap()` 不使用。`expect("理由")` でパニック理由を明示するか、`Result` 伝播を行う。
6. **既存コード改善**: 触る既存ファイル（`src/constants.rs` への定数追加）においても、既存の定数命名規則とコメントスタイル（分類コメント／デフォルト値／感度分析推奨範囲）に合わせて記述する。

## Acceptance Criteria

- [ ] `WorkflowMaturity::{Child, Adult}` 列挙型が定義されている
- [ ] `classify_maturity` が経験値・信頼・レピュテーションの3軸で正しく判定する（T-1〜T-7 通過）
- [ ] `LocalVillage` 構造体が定義されている
- [ ] `filter_adult_candidates` が不整合状態・隔離・未成熟の候補を正しく除外する（T-8〜T-10 通過）
- [ ] `build_local_village_topk` が TopK 近傍選抜を正しく行う（T-11〜T-14 通過）
- [ ] `build_local_village_radius` が半径内選抜を正しく行う（T-15〜T-17 通過）
- [ ] centroid 計算が正しい（T-18〜T-20 通過）
- [ ] 全20テストが PASS すること（T-E1 サマリ出力含む）
- [ ] 翻訳可能性の検証が通っている
- [ ] 既存テストが全 PASS している
- [ ] RFC §41B.3 との矛盾がないこと

## Notes

注: このコメントは人間向けの説明である。AI は以下の手順に従うこと。

- plan_path: /plan-ticket が plan.md を作成後に frontmatter に更新する
- implementation_path: /start-ticket が implementation.md を作成後に frontmatter に更新する
- review_report_path: /review-ticket が review.md を作成後に frontmatter に更新する
- observation_report_path: /start-ticket が observation-YYYYMMDD-HHmmss.md を作成後に frontmatter に最新パスを更新する

各コマンドのワークフロー手順が frontmatter の更新である。

### 成果物

- 計画: context/0073-m175-2-child-adult-maturity-local-village/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0073-m175-2-child-adult-maturity-local-village/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0073-m175-2-child-adult-maturity-local-village/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0073-m175-2-child-adult-maturity-local-village/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
