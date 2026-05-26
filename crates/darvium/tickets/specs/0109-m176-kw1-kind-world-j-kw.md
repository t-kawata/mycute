---
ticket_id: 109
title: M1.76-KW1: Kind World 成立条件定数 + J_kw 目的関数実装
slug: m176-kw1-kind-world-j-kw
status: done
created_at: 2026-05-26
updated_at: 2026-05-26
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0109-m176-kw1-kind-world-j-kw/plan.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0109-m176-kw1-kind-world-j-kw/implementation.md
review_report_path: ""
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0109-m176-kw1-kind-world-j-kw/observation-20260526-154455.md
---

# M1.76-KW1: Kind World 成立条件定数 + J_kw 目的関数実装

## Summary

Kind World の成立を定義する 8 つの成立条件定数と、エコシステム繁栄度を定量化する目的関数 J_kw を実装する。これにより「慈悲的集団が優位な世界」が実現しているかを自動判定する基盤が整う。

## Background

- **RFC:** §15.9 SocialAcceleration、§41B.20.7 ExtendedOperationalMetrics、§41C.3 目的関数設計
- Kind World の成立は「ワークフロー人口の継続的増加」「実務遂行能力カバー率の拡大」「再利用効率の向上」「単位コストの単調減少」「村の健全な形成と知識交換」「慈悲的集団の非慈悲的集団に対する優位」の6条件をすべて同時に満たすことで定義される
- これらは F-16 の機構健全性とは独立した、エコシステム繁栄指標として設計する (MUST)
- **現在の状態:** 既存の `constants.rs` には M1.76 系の定数（GC hazard、child protection、reputation 等）は実装済みだが、Kind World 関連の定数・構造体・関数は一切未実装。`MagnificentSevenParams` のパラメータ名一覧は `constants.rs:939` に定数配列として存在するが、構造体定義は未作成。

## Scope

### 実装範囲

1. **`constants.rs`** に以下を Safety Invariant として追加:
   - KW 条件ターゲット閾値 (8個): `KW_MIN_POPULATION_GROWTH_RATE`, `KW_MIN_CAPABILITY_COVERAGE_SHANNON`, `KW_MIN_REUSE_RATIO`, `KW_MAX_COST_EFFICIENCY_DECAY`, `KW_MIN_VILLAGE_FORMATION_SCORE`, `KW_VILLAGE_CHURN_LOWER`, `KW_VILLAGE_CHURN_UPPER`, `KW_CROSS_VILLAGE_INTERACTION_MIN`
   - Village 距離/サイズ閾値 (2個): `VILLAGE_DISTANCE_THRESHOLD` (Calibration Candidate), `VILLAGE_MIN_SIZE` (Safety Invariant)
   - J_kw 重み係数 (6個): `KW_ALPHA_POP`〜`KW_ALPHA_PENALTY` (Calibration Candidate)

2. **`MagnificentSevenParams` 構造体** (新規ファイルまたは calibration.rs): 較正ループで sweep する 7 パラメータ:
   - `gamma_benevolence`, `lambda_gc_base`, `direct_reciprocity_weight`, `indirect_reciprocity_weight`, `softmax_temperature`, `gc_interval`, `child_ratio`

3. **`KindWorldAssessment` 構造体**: `is_kind_world: bool`, `flags: [bool; 8]`, `j_kw: f64`
   - 6 概念条件 → 8 測定閾値に分解

4. **`compute_kind_world_objective()` 純粋関数**: J_kw(θ) = Σ α_i·J_i (6 成分の重み付き和)

### 非スコープ

- シミュレーターとの統合 (KW2〜KW4 で実施)
- エコシステム成長メトリクスの計装 (KW2)
- 村間相互作用・知識拡散トラッキング (KW3)
- 較正ループ実行 (KW4)

## Investigation

### ソースコード調査結果

- `src/constants.rs` に M1.76-2〜M1.76-22 の全定数が実装済み。958行。Kind World 関連の定数は未実装。
- `MagnificentSevenParams` 構造体は未定義。定数配列 `SWEEP_MAGNIFICENT_PARAM_NAMES` (`constants.rs:939-947`) のみ存在。
- `KindWorldAssessment` / `compute_kind_world_objective` は未実装。
- `src/calibration.rs` に既存の較正モジュールが存在。`MagnificentSevenParams` はここに追加が適切。
- 既存の目的関数パターン: `src/constants.rs` に `F16_LAMBDA_AUC` 等の重み定数が既存。J_village の定数も `OBJECTIVE_WEIGHT_CHURN` として既存。これらと同様の命名パターンを踏襲する。
- `src/reciprocity.rs` に既存の benevolence 関連関数が存在し、F-1〜F-15 は実装済み。KW1 はこれらの出力をエコシステムレベルで評価する上位目的関数。

### 参照観察レポート

- `tickets/context/0108-m176-23-event-architecture/observation-20260526-150906.md` — M1.76-23 全ドメイン横断 Event Architecture 一貫性検証完了。全テスト PASS。M1.76 系列の最終チケットとして Kind World 実装に進む準備が整った。

## Test Plan

### テスト対象

- `KindWorldAssessment` 構造体の生成とフラグ設定
- `compute_kind_world_objective()` 純粋関数

### テストケース

| # | ケース | 内容 |
|---|--------|------|
| 1 | 全条件成立 | 8 フラグ全て閾値超過 → `is_kind_world == true` |
| 2 | 全条件不成立 | 8 フラグ全て閾値未満 → `is_kind_world == false` |
| 3 | J_kw 範囲 | 任意入力で J_kw ∈ [0, 1]、NaN/Inf が一切出現しない |
| 4 | J_pop 単調性 | population_growth_rate 増加に伴い J_pop が非減少 |
| 5 | 重み総和 | Σ α_i == 1.0 (静的アサート) |
| 6 | 空入力 | 全 metrics = 0 → panic せず J_kw = 0 |
| 7 | J_penalty 慈悲劣位 | 慈悲的 < 非慈悲的 → J_penalty > 0 |
| 8 | J_penalty 慈悲同等 | 慈悲的 == 非慈悲的 → J_penalty = 0 |
| 9 | 境界値 ±0.001 | 各指標の閾値 ±0.001 で成立/不成立が切り替わる |
| 10 | JSON ラウンドトリップ | KindWorldAssessment の serde 通し |

### 観測テスト

- n=10,000 のランダム入力で J_kw が [0, 1] かつ NaN/Inf フリーであることを統計的に確認
- 固定シード PRNG (`StdRng::seed_from_u64(12345)`) を使用、完全再現性を保証

## 計装方法・観測対象

### 計装方法

- テストコードは `src/` 内の `mod tests` に実装（既存パターンに従う）
- 観測テスト: n=10,000 のランダム入力を生成し J_kw ∈ [0,1] を確認。`println!` + `--nocapture` で JSON 出力
- 固定シード PRNG (`StdRng::seed_from_u64(12345)`) 使用
- JSON ラウンドトリップテストも含める

### 観測対象

- J_kw の統計量（平均・最小・最大・NaN 出現率）
- 各サブ目的関数の単調性検定
- 全条件成立/不成立の二値分類精度

### 較正計画

- 本チケットでは較正ループは実行しない（KW4 で実施）
- 較正に必要なパラメータ構造体 (`MagnificentSevenParams`) と目的関数のみを実装
- 定数の分類に従い、KW 条件閾値は Safety Invariant、J_kw 重みは Calibration Candidate として定義

## Boy Scout Rule — 翻訳可能性計画

- 関数名は動詞句 (`compute_kind_world_objective`) とし、何を計算するかが関数名から一意に判別できるようにする
- 構造体名はドメイン名詞 (`KindWorldAssessment`, `MagnificentSevenParams`) とする
- 各サブ目的関数は独立した純粋関数として抽出し、一関数一責務を徹底する
- ハードコード値は全て `constants.rs` の名前付き定数とし、マジックナンバーを排除する
- NaN/Inf のチェックは `f64::is_finite()` を使用し、握りつぶさない
- コメントは「なぜその閾値か」「どの RFC 由来か」を説明する。自明な計算過程の説明は行わない

## Acceptance Criteria

- [ ] 実装要件を満たしている
- [ ] 翻訳可能性の検証が通っている
- [ ] 既存テストが通過している
- [ ] 全10テストケースが PASS すること
- [ ] n=10,000 ランダム入力で NaN/Inf が 0 件であること
- [ ] Σ α_i == 1.0 の静的アサートがコンパイル時に成立すること

## Notes

### 依存関係

- 実装は `src/constants.rs` + `src/calibration.rs` (または新規 `src/kind_world.rs`) に追加
- 既存の M1.76-2〜M1.76-22 の実装に対する依存はなし（純粋関数として独立）
- 後続チケット KW2（エコシステム成長メトリクス計装）へのインターフェースとして `KindWorldMetricsInput` 相当の型を用意するかは設計判断

### 成果物

- 計画: context/0109-m176-kw1-kind-world-j-kw/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0109-m176-kw1-kind-world-j-kw/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0109-m176-kw1-kind-world-j-kw/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0109-m176-kw1-kind-world-j-kw/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
