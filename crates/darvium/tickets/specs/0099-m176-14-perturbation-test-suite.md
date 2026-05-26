---
ticket_id: 99
title: M1.76-14: 摂動テストスイート（SHOULD perturbation）
slug: m176-14-perturbation-test-suite
status: reviewed
created_at: 2026-05-26
updated_at: 2026-05-26
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0099-m176-14-perturbation-test-suite/implementation.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0099-m176-14-perturbation-test-suite/observation-20260526-120000.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0099-m176-14-perturbation-test-suite/review.md
---

# M1.76-14: 摂動テストスイート（SHOULD perturbation）

## Summary

Reciprocity モジュールの計算結果（ranking, village composition, hazard, survival probability）が微小な摂動に対して安定であることを検証する摂動テストスイートを実装する。1 件の help success 追加で village 全体が崩壊的に並び替わらないこと、1 helper の微小な trust change で helper set が全入れ替えしないことを確認する。

## Background

M1.76-12（単調性テスト）では極端な入力に対する単調性を MUST 条件として検証した。M1.76-13（決定論的リプレイテスト）では完全同一入力に対するビットレベル再現性を MUST 条件として検証した。

これらを補完するのが SHOULD レベルの**摂動テスト**である。現実のエコシステムではイベントの到着順序・タイミング・成功/失敗の判定に微小な揺らぎが常に存在する。このような摂動に対して Reciprocity 計算結果が「安定」（定性的に同じ ranking / village 構成を維持）していることを保証する必要がある。

**なぜ SHOULD か**: 摂動に対する安定性は「絶対に満たすべき不変条件」ではなく「実用上望ましい性質」である。過度に安定（摂動を完全に無視）すると生態系の適応性が失われ、過度に敏感（微小摂動で ranking が激変）するとシステム全体の予測可能性が損なわれる。適切なバランスを取ることが較正の目標となる。

本チケットは RFC §41B.20.8 Testing discipline「Perturbation test (SHOULD)」、RFC §41C.3 の **M2.x** に対応する。

## Scope

1. **`ReciprocityPerturbationGenerator` トレイトの定義**: `apply(snapshot) -> PerturbedSnapshot`
2. **5 種の摂動種の列挙型定義**:
   - `HelpSuccessAddition(usize)` — N 件の help success を追加
   - `TrustDelta(f64)` — 0.01 程度の信頼値微増減
   - `LocalityDistanceDelta(f64)` — 位置距離の微小変更
   - `AcceptedOfferToOneRejected` — 1 件の accepted offer を rejected に置換
   - `SingleHelperReputationDelta(f64)` — 1 helper の reputation 微調整
3. **`PerturbedSnapshot` 構造体**: 摂動後の snapshot（profiles, hazards, イベント列）を保持
4. **`ReciprocityPerturbationSuite`**: 全摂動種を baseline と perturbed のペアで実行し結果を比較
5. **`StabilityRegressionSummary` 構造体**: 以下のメトリクスを集約
   - `flip_rate: f64` — ranking の flip rate（順位変動率）
   - `churn_delta: f64` — helper set の入替率
   - `hazard_drift: f64` — GC hazard のドリフト量
   - `survival_drift: f64` — survival probability のドリフト量
   - `oscillation_detected: bool` — 無限ループ的振動の有無
   - `village_churn_delta: f64` — village 構成の変動率
6. **`OscillationDetector`**: 摂動前後の ranking 順位変動を追跡し、複数時点間で順位が往復する（A > B > A > B のような）振動パターンを検出
7. **観測計装**: 摂動強度 σ を sweep し `flip_rate(σ)` の応答曲線を観測。臨界値 σ_c の同定

## Non-scope

- Reciprocity モジュールの計算ロジックそのものの変更（F-1〜F-15 は既存実装をそのまま利用）
- M1.76-11 の replay snapshot / diff report の再設計
- M1.76-12 の単調性テストとの重複（単調性は MUST、摂動安定性は SHOULD として独立）
- M1.76-13 のリプレイ機構との重複（リプレイはビットレベル一致、摂動は微小変化後の安定性）
- プロダクションコードへの摂動テスト機構の組み込み（テスト専用ユーティリティとして実装）
- 摂動の自動生成・ファジング（M1.76-15 で別途扱う）

## Investigation

### 既存コード調査結果

**既存の関連実装（参考）:**

- `src/reciprocity.rs:783` — `ReciprocityReplaySnapshot`: profiles, hazards, policy_version, clock を持つ replay snapshot
- `src/reciprocity.rs:827` — `compute_replay_comparison`: 2 つの snapshot を比較し `DiffReport` を返す
- `src/reciprocity.rs:903` — `ReciprocityReplayScenario`: リプレイシナリオ（event_stream, policy, clock_schedule, initial_profiles）
- `src/reciprocity.rs:989` — `run_reciprocity_replay`: シナリオを逐次実行し trace を生成
- `src/reciprocity.rs:1028` — `ReplayTraceComparator::assert_bitwise_eq`: 2 trace のビットレベル一致検証
- `src/reciprocity.rs:1322` 以降 — `mod tests` テストセクション

**新規作成するもの（本チケット）:**

- `ReciprocityPerturbationGenerator` トレイト — 摂動生成の抽象化
- `PerturbationKind` 列挙型 — 5 種の摂動種
- `PerturbedSnapshot` 構造体 — 摂動適用後の snapshot
- `ReciprocityPerturbationSuite` — 全摂動種をループ実行
- `StabilityRegressionSummary` — 摂動結果の集約メトリクス
- `OscillationDetector` — 順位振動検出器
- 上記のテストコード群

### 依存関係の確認

- `ReputationProfile`（event.rs） — profile の全フィールド
- `DarviumEvent` / `ReciprocityEvent`（event.rs） — イベント型
- `ReciprocityLifecyclePolicy`（event.rs） — ポリシーパラメータ
- `recompute_reputation`（reciprocity.rs） — F-4/F-5
- `compute_gc_hazard`（reciprocity.rs） — F-7/F-8
- `compute_direct_reciprocity`（reciprocity.rs） — F-1
- `compute_indirect_reciprocity`（reciprocity.rs） — F-2
- `ReciprocityReplaySnapshot` / `compute_replay_comparison` — 差分比較に利用可能

## Test Plan

### テスト 1: Help success 1 件追加の ranking stability

baseline の snapshot に help success 1 件を追加し、perturbed snapshot との間で helper ranking の flip rate を計測。上限閾値（例: 0.20）を超えないことを確認。

- 入力: 5〜10 件の WorkflowGraphId を含む初期プロファイル群、デフォルトポリシー
- 摂動: `PerturbationKind::HelpSuccessAddition(1)`
- 期待結果: `flip_rate <= 0.20`
- 観測出力: baseline ranking と perturbed ranking の一覧、flip rate

### テスト 2: Trust 微増減の village churn stability

trust 値を 0.01 微増減したときの village churn delta が許容範囲内であることを確認。

- 摂動: `PerturbationKind::TrustDelta(0.01)` および `TrustDelta(-0.01)`
- 期待結果: `village_churn_delta <= 0.15`（例）
- 観測出力: churn delta の正負両方向の値

### テスト 3: Accepted offer → rejected 置換の survival stability

accepted offer 1 件を rejected に置換したときの survival probability drift が許容範囲内であることを確認。

- 摂動: `PerturbationKind::AcceptedOfferToOneRejected`
- 期待結果: `survival_drift <= 0.10`（例）
- 観測出力: survival probability の before/after 比較

### テスト 4: 1 helper reputation 微調整の helper set 全入替チェック

1 helper の reputation を微調整したときに helper set の全入れ替えが発生しないことを確認。

- 摂動: `PerturbationKind::SingleHelperReputationDelta(0.01)` および `SingleHelperReputationDelta(-0.01)`
- 期待結果: `churn_delta < 1.0`（全入替で 1.0 となる）
- 観測出力: helper set の構成変化（共通要素数 / 全要素数）

### テスト 5: 全摂動種で oscillation 検出されないこと

各摂動種について `OscillationDetector` が振動を検出しないことを確認。

- 期待結果: 全摂動種で `oscillation_detected == false`
- 観測出力: 摂動種ごとの oscillation_detected フラグ

### テスト 6: 摂動強度 σ sweep による応答曲線観測

摂動強度 σ を sweep し、`flip_rate(σ)` の応答曲線をプロットする。

- σ の sweep range: [0.001, 0.005, 0.01, 0.02, 0.05, 0.1]
- 期待結果: flip_rate が σ に対して単調非減少であること（線形性までは要求しない）
- 観測出力: (σ, flip_rate) のペア列

### テスト 7: n=100 回の独立実行による統計的安定性

各摂動種について n=100 回の独立実行を行い、flip_rate / churn_delta / hazard_drift / survival_drift の平均・標準偏差を観測。

- 計装: 各回の summary を収録し、平均 ± 2σ の範囲内で閾値を超過しないこと
- 期待結果: 全 n=100 回で flip_rate <= 0.20 等の閾値を遵守

## 計装方法・観測対象

### 計装方法

- テストコードは `src/reciprocity.rs` の `mod tests` 内に実装（既存テストと同様）
- 固定シード PRNG は `StdRng::seed_from_u64(12345)` を使用
- `println!` + `--nocapture` で以下の観測データを標準出力に書き出す:
  - 各摂動種の baseline vs perturbed ranking
  - flip_rate, churn_delta, hazard_drift, survival_drift, oscillation_detected
  - σ sweep の (σ, flip_rate) 応答曲線
  - n=100 回の統計量（平均・標準偏差）

### 観測対象

- **flip_rate**: ranking 順位の変動割合。0.0 = 完全一致、1.0 = 全順位逆転
- **churn_delta**: helper set の入れ替わり割合（Jaccard 距離相当）
- **hazard_drift**: GC hazard 値の平均絶対変動
- **survival_drift**: survival probability の平均絶対変動
- **oscillation_detected**: 順位の往復振動が検出されたか（bool）
- **village_churn_delta**: village 構成メンバーの変動割合
- **flip_rate(σ) 応答曲線**: 摂動強度に対する flip rate の応答関数

### 較正計画

本チケットは直接の較正（constants.rs の変更）を伴わない。ただし、以下を観測する:
- 閾値（flip_rate 0.20 等）の適切性評価 — 現行パラメータで SHOULD 条件が満たされるか
- 摂動強度 σ_c（臨界値）の同定 — どの程度の摂動で flip_rate が急増するか
- 観測結果は M1.76-16（多目的較正目的関数 F-16）への入力として利用される

## Boy Scout Rule — 翻訳可能性計画

本チケットで新規追加するコードは以下の方針で実装する:

1. **トレイト名・関数名は動詞句/名詞として明確に**: `ReciprocityPerturbationGenerator::apply`, `OscillationDetector::detect`, `ReciprocityPerturbationSuite::run_all`
2. **一関数一責務**: 摂動の種類ごとに個別の関数として実装し、suite がそれらを統一的に呼び出す
3. **摂動種の列挙型はバリアントごとに意味をコメント**: 各バリアントが何を変更するか一言添える
4. **ハードコード値の定数化**: テスト内のマジックナンバー（flip_rate 閾値、n=100 等）は名前付き定数として定義
5. **既存の命名パターンに従う**: `compute_replay_comparison` 等の既存関数のスタイルを踏襲

既存コード（特に `ReciprocityReplaySnapshot` や `compute_replay_comparison`）のインターフェースは変更しない。摂動テストはこれらを消費する形で実装する。

## Acceptance Criteria

- [ ] `ReciprocityPerturbationGenerator` トレイト + 5 種の `PerturbationKind` + `PerturbedSnapshot` が実装される
- [ ] `ReciprocityPerturbationSuite` + `StabilityRegressionSummary` + `OscillationDetector` が実装される
- [ ] テスト 1: Help success 1 件追加で flip rate が閾値以下（SHOULD）
- [ ] テスト 2: Trust 微増減で village churn delta が許容範囲内（SHOULD）
- [ ] テスト 3: Accepted→Rejected 置換で survival drift が許容範囲内（SHOULD）
- [ ] テスト 4: 1 helper reputation 微調整で helper set 全入替なし（SHOULD）
- [ ] テスト 5: 全摂動種で oscillation 検出なし（SHOULD）
- [ ] テスト 6: 摂動強度 σ sweep による応答曲線観測
- [ ] テスト 7: n=100 回の独立実行による統計的安定性確認
- [ ] 既存テスト（M1.76-1〜13）がすべて通過すること
- [ ] RFC 該当セクションとの無矛盾確認

## Notes

- plan_path: {{plan 作成後にセット}}
- implementation_path: {{実装後にセット}}
- review_report_path: {{レビュー後にセット}}
- observation_report_path: {{観測レポート作成後にセット}}

### 成果物

- 計画: context/0099-m176-14-perturbation-test-suite/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0099-m176-14-perturbation-test-suite/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0099-m176-14-perturbation-test-suite/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0099-m176-14-perturbation-test-suite/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成）
