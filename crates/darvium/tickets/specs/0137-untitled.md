---
ticket_id: 137
title: 評判再計算パイプラインのプロダクション実装とシミュレーション完全性確保
slug: untitled
status: reviewed
created_at: 2026-05-28
updated_at: 2026-05-28
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0137-untitled/plan.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0137-untitled/implementation.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0137-untitled/observation-20260528-154420.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0137-untitled/review.md
---

# 評判再計算パイプラインのプロダクション実装とシミュレーション完全性確保

## Summary

ReputationProfile の再計算パイプライン（F-1〜F-5）が、プロダクションコードから一切呼ばれていない問題を修正する。同時に、`run_kw_real_simulation` および `run_evaluation_simulation` のシミュレーションループでも評判再計算が欠落している問題を修正し、シミュレーション結果の信頼性を確保する。あわせて、`experience_count` のインクリメント、`inherit_reputation` の正しい呼び出し、`village_centrality` の算出という評判計算に必要な前提条件の整備も行う。

## Background

### 問題

RFC §15.10 に「評判値は定期的に再計算される」と明記されているにも関わらず、プロダクションコードには評判再計算を実行するパスが存在しない。具体的には以下の3層の問題がある：

**層1: 本格シミュレーションにも再計算がない**
`run_kw_real_simulation`（simulation.rs:1430）と `run_evaluation_simulation`（simulation.rs:1619）の6フェーズループには評判再計算フェーズが存在しない。ただ `recompute_trust_reputation`（simulation.rs:894）は簡易パス `run_simulation`（simulation.rs:1326）でのみ呼ばれている。つまり、較正ループ（J_kw 最適化）に使われている2つの本格シミュレーションは、評判値を常に初期値（`cold_start()`）+ 継承時の値のまま固定して計算しており、`s_fairness` や `s_topology` 因子の信頼性を損なっている。

**層2: プロダクションコードに評判再計算のパスがない**
`recompute_all_profiles()`（reciprocity.rs:719）はプロダクションの `pub fn` だが、テストコードと `run_reciprocity_replay` からのみ呼ばれている。`run_reciprocity_replay` 自体も `run_perturbation_suite`（テスト専用）からのみ使われるため、実質的にプロダクションで動作する経路はない。

**層3: 評判計算の前提条件も未整備**
- `experience_count` は `MemoizedGraph`（trust.rs:63）にフィールド定義があるだけで、プロダクションコードでインクリメントするコードが一切ない
- `inherit_reputation`（trust.rs:259）は公開関数として定義・テストされているが、プロダクションから呼ばれていない
- `village_centrality` は算出コードがなく、常に 0.0 のまま
- `ReciprocityEventStore::ingest()`（reciprocity.rs:651）で HELP イベントを蓄積する機構はあるが、そのイベントを消費して評判再計算するパイプラインが繋がっていない

### なぜ放置されたか

M1.76 シリーズでは、シミュレーションと較正ループの基盤整備（パラメーター化・WIRE実装・観測テスト・Nelder-Mead 最適化）に集中したため、評判再計算のオーケストレーションは後回しにされた。数式の実装と単体テストは M1.76-5 で完了し、`recompute_all_profiles` の決定論性や単調性も M1.76-11〜13 で検証済みである。

## Scope

以下のすべてを実装する：

### A. シミュレーションループの修正（2箇所）
- `run_kw_real_simulation` に `recompute_trust_reputation` 呼び出しを追加（Phase 4 直前または Phase 3 直後）
- `run_evaluation_simulation` に同様の呼び出しを追加
- `run_evaluation_simulation_with_channel` も同様

### B. Darvium facade に評判再計算メソッドを追加
- `Darvium` 構造体（lib.rs）に `recompute_reputations()` メソッドを追加
- 内部で `recompute_all_profiles` を呼び出すインターフェースを設計
- 実行モデル（A: 明示的 tick / B: イベント駆動 / C: 内部タイマー）は plan で決定

### C. experience_count のインクリメント（プロダクション）
- HELP 成功時に experience_count を increment するコードを適切な場所に追加
- ワークフロー実行・再利用成功時にも experience_count を increment するコードを追加

### D. inherit_reputation のプロダクション呼び出し
- SubWorkflow 生成時（子誕生時）に `inherit_reputation` を呼ぶコードを適切な場所に追加
- HELP 成功時（能力拡散相当）にも `inherit_reputation` を呼ぶコードを追加

### E. village_centrality の算出
- 村クラスタリング後の中心性計算を実装
- `compute_indirect_reciprocity` の入力として正しく伝播させる

## Non-scope

- 重み定数（`REPUTATION_WEIGHT_*`, `theta_*`）の較正は行わない（別チケット）
- `ReputationProfile` の DB 永続化は行わない（別チケット）
- シミュレーションとプロダクションで共通化された抽象実行モデルの設計（MYCUTE 側との結合設計が必要なため、本チケットでは最小限の facade 追加に留める）
- GC のプロダクション実装（本チケットは評判再計算まで。GC それ自体は別チケット）

## Investigation

### 参照観察レポート

- `tickets/context/0136-m176-kw-wire-e-4a0-5/observation-20260528-084843.md` — M1.76-KW-WIRE-E 残余ハードコード値の全数パラメーター化。E6 で inherit_reputation 減衰効果の動作確認済み、E4 で LifecycleScore 伝播確認済み。

### 物理的証拠

#### [E1] `recompute_trust_reputation` の唯一の呼び出し元

- **定義**: `src/simulation.rs:894`
- **唯一の呼び出し**: `src/simulation.rs:1365` — 簡易パス `run_simulation` 内のみ
- **欠落**: `run_kw_real_simulation`（simulation.rs:1430）と `run_evaluation_simulation`（simulation.rs:1619）のループでは呼ばれていない

#### [E2] `run_kw_real_simulation` のループ構造（simulation.rs:1464）

6フェーズあるが、評判再計算フェーズは存在しない：
| フェーズ | 行 | 関数 | 評判との関係 |
|---------|------|------|------------|
| Phase 1 | 1468 | phase1_population_growth | 子誕生時 inherit_reputation あり（OK） |
| Phase 2 | 1475 | phase2_village_clustering | village_assignment 設定のみ。centrality 未計算 |
| Phase 3 | 1478 | phase3_help_protocol | HELP プロトコル実行。Benevolence を offer 判定に使うが、再計算されていない初期値 |
| Phase 4 | 1489 | phase4_gc_survival | GC hazard 計算で reputation.final_score と benevolence_score を読むが、初期値のまま |
| Phase 5 | 1500 | phase5_capability_diffusion | inherit_reputation を呼ぶ（OK）。experience_count を increment（OK） |
| Phase 6 | 1527 | phase6_measure_jkw | J_kw 測定。s_fairness は reputation に依存 |

#### [E3] `run_evaluation_simulation` も同構造（simulation.rs:1658）

同じ6フェーズループ。評判再計算なし。

#### [E4] `experience_count` のプロダクション非更新

- `trust.rs:63`: `MemoizedGraph.experience_count` 定義
- `trust.rs:94, 128`: コンストラクタで 0 初期化
- `store/coordinator.rs:269`: `load_memoized_graph` でも 0 初期化
- プロダクションコード（非 test、非 simulation）で `experience_count` をインクリメントする箇所は **1行も存在しない**
- 唯一のインクリメントは `simulation.rs:2563-2564`（phase5_capability_diffusion 内）

#### [E5] `inherit_reputation` のプロダクション非呼び出し

- `trust.rs:259`: 定義（`pub fn`）
- `lib.rs:87`: 再エクスポートのみ
- プロダクションコードからの呼び出しは **皆無**
- シミュレーションからの呼び出し: `simulation.rs:2141`（phase1_population_growth）、`simulation.rs:2557`（phase5_capability_diffusion）

#### [E6] `recompute_all_profiles` の実質テスト専用

- `reciprocity.rs:719`: 定義（`pub fn`）
- 呼び出し元の内訳:
  - `reciprocity.rs:1018` → `run_reciprocity_replay` 内 → さらに `run_perturbation_suite`（reciprocity.rs:1591）からのみ → `run_perturbation_suite` 自体もテスト専用（reciprocity.rs:4793）
  - 残りは全て `#[cfg(test)]` 内
- **プロダクションのいかなる実行パスからも到達不能**

#### [E7] `compute_direct_reciprocity` / `compute_indirect_reciprocity` のプロダクション呼び出し

- `compute_direct_reciprocity`: プロダクションからの呼び出しは **なし**（`recompute_all_profiles` 経由は E6 の通り到達不能）
- `compute_indirect_reciprocity`: `calibration.rs:1236-1237` で Phase 0 のレンジ健全性チェックとして2回呼ばれるのみ。これは本来の目的（グラフの間接互恵性計算）での呼び出しではない

#### [E8] `emit_help_event` のプロダクション非呼び出し

- `help.rs:572`: 定義
- `help.rs:294`: `transition_to` 内で条件付き呼び出し
- `event_bus` に `Some(...)` を渡すコードは **全て `#[cfg(test)]` 内**
- プロダクションからは `None` が渡され、イベントが発行されない

#### [E9] `load_memoized_graph` が reputation を初期化で上書き

- `store/coordinator.rs:270`: `reputation: ReputationProfile::cold_start()`
- DB に保存された評判値を復元する機構がない（そもそも評判値を永続化する機構自体がない）

## Test Plan

### T1: シミュレーション評判再計算の挿入確認
- `run_kw_real_simulation` が `recompute_trust_reputation` を呼ぶことを確認するテスト
- 同一 config で呼び出し前後の reputation 値が変化することを確認（定性的）
- `run_evaluation_simulation` も同様

### T2: experience_count インクリメントテスト（プロダクション）
- HELP 成功イベント後に experience_count が 1 増加することを確認
- 複数回成功で累積的に増加することを確認
- `u64::MAX` 飽和時のオーバーフロー安全を確認

### T3: inherit_reputation 呼び出しテスト（プロダクション）
- SubWorkflow 生成後に `inherited_score` が 0.0 以外になることを確認
- 減衰係数 0.0 → inherited_score = 0.0（継承なし）
- 減衰係数 1.0 → inherited_score == parent.final_score（完全継承）
- 減衰係数 0.7 → inherited_score == parent.final_score * 0.7

### T4: village_centrality 算出テスト
- 村クラスタリング後の中心性が [0, 1] 範囲であることを確認
- 同一村内全員が同一中心性を持たないことを確認（グラフ構造の反映）
- 孤立ノードの中心性が 0 に近いことを確認

### T5: facade recompute_reputations 基本動作
- 空のストア → 空の結果（MUST）
- 同一入力 → 同一出力（決定論性 MUST）
- シリアライズ・デシリアライズのラウンドトリップ

### T6: 既存テスト回帰なし
- `cargo test` 全テスト通過
- 既存の観測テスト出力が従来と同等であること

## 計装方法・観測対象

### 計装方法
- T1: 通常の `#[test]` + `println!` + `--nocapture` で観測。固定シード `StdRng::seed_from_u64(12345)` を使用
- T2-T5: 通常の unit test（`#[cfg(test)]` 内で完結）
- T6: `cargo test` で全テスト実行

### 観測対象
- T1: シミュレーションループ内での reputation.final_score の推移（tick 毎の変化）
- T6: 全テストの PASS/FAIL カウント

### 較正計画
- 本チケットでは較正は行わない。既存定数値の変更はなし

## Boy Scout Rule — 翻訳可能性計画

- `recompute_trust_reputation` 内の変数名を確認し、散文として読めることを検証する
- 特筆すべきハードコード値は発見次第 `constants.rs` への抽出を検討
- 既存コメントの陳腐化がないことを確認する

## Acceptance Criteria

- [ ] `run_kw_real_simulation` が `recompute_trust_reputation` を呼び、評判値が tick 経過とともに変化する
- [ ] `run_evaluation_simulation` も同様
- [ ] `Darvium::recompute_reputations()` が実装され、正しく動作する
- [ ] HELP 成功時に `experience_count` がインクリメントされる
- [ ] SubWorkflow 生成時に `inherit_reputation` が呼ばれる
- [ ] 村クラスタリング後に `village_centrality` が算出される
- [ ] 既存全テストが通過する
- [ ] 翻訳可能性の検証が通っている

## Notes

本チケットの実装は Darvium crate 内で完結する。MYCUTE との結合設計（タイマーループからの `recompute_reputations()` 呼び出し等）は含まない。

### 成果物

- 計画: context/0137-untitled/plan.md（未作成）
- 実装サマリ: context/0137-untitled/implementation.md（未作成）
- レビュー報告書: context/0137-untitled/review.md（未作成）
- 観察レポート: context/0137-untitled/observation-YYYYMMDD-HHmmss.md（未作成）
