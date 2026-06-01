---
ticket_id: 150
title: 生存者リストのキャッシュと全人口スキャンの削減
slug: untitled-5
status: reviewed
created_at: 2026-06-01
updated_at: 2026-06-01
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0150-untitled-5/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0150-untitled-5/review.md
---
# 生存者リストのキャッシュと全人口スキャンの削減

## Summary

3つのシミュレーションループにおいて、毎 tick 7回以上呼ばれる `alive_ids()` が毎回死者を含む全人口 Vec をフルスキャンしている問題を修正する。加えて、同様のパターンである `positions_map()` `gc_states_map()` `last_update_ticks_map()` の HashMap 再構築、`phase1_population_growth` の内部全人口スキャン、および観測関数群の重複スキャンをまとめて改善する。

## Background

チケット #149 の調査過程で、`alive_ids()` が以下に挙げる全箇所で呼ばれていることが判明したが、スコープ超過のため #149 では対応しなかった。各関数が独立に全人口 Vec をイテレートして生存者フィルタリングを行っており、これらを1回の `alive_ids()` 呼び出しに集約することで毎 tick のO(total_population)スキャンを削減できる。

また `phase1_population_growth` は内部で `alive_ids()` 相当のフィルタを実行しており、Phase 1 出生直後に呼ばれる Phase 2〜5 では出生者を含めた正しい生存者リストが必要となる。

## Scope

### 必須: alive_ids() 呼び出し集約

各 tick 先頭で1回だけ `alive_ids()` を呼び、結果を Vec として全フェーズに引数で渡す。Phase 1 出生後は新生児を追加した更新版リストを後続フェーズに渡す。

影響を受ける関数とその呼び出し元は以下の通り（3つのシミュレーションループ全て）:

| 関数 | simulation.rs 行 | 役割 |
|---|---|---|
| `alive_ids()` | 458 | 生存者PersonId一覧 |
| `phase1_population_growth` | 2633 | 内部で全人口スキャンして生存成人→出生判定 |
| `phase2_village_clustering` | 2719 | 内部で `alive_ids()` 呼び出し |
| `compute_village_centrality` | 2868 | 同上 |
| `phase3_help_protocol` | 2970 | 同上（さらに成人/子に分割） |
| `phase4_gc_survival` | 3169 | 同上 |
| `run_self_refinement_for_population` | 2612 | 同上 |
| `observe_kw_real_tick` | 3595 | 同上 |
| `check_convergence` | 1945 | `gc_states_map()` `last_update_ticks_map()` も全人口→HashMap |

## Non-scope

- 空間グリッドインデックスの導入（別チケット）
- `check_convergence` 内の具体的な収束判定ロジック変更

## Investigation

### チケット #149 からの引き継ぎ事項

#149 の実装計画で「Step B: alive_ids() 呼び出し集約」として定義されたが、スコープが大きくなりすぎるため分離された。

### 物理的証拠

**証拠1**: `alive_ids()` の実装（`simulation.rs:458-464`）
返す `self.population`（死者を含む増加し続ける Vec）をフルスキャン。

**証拠2**: 3つのシミュレーションループのいずれでも、以下の関数が各 tick 独立に alive_ids() を呼ぶ:
- `phase2_village_clustering` `compute_village_centrality` `phase3_help_protocol`
- `run_self_refinement_for_population` `phase4_gc_survival` `observe_kw_real_tick`
- `check_convergence`（ObservationInterval ごと）
合計7回以上の全人口スキャンが毎 tick 発生。

**証拠3**: `phase1_population_growth`（`simulation.rs:2633-2701`）は内部で `alive_ids()` 相当のフィルタを実行。出生後の生存者リスト更新は行われていない。

**証拠4**: `check_convergence`（`simulation.rs:1945-1983`）は1回の呼び出しで `alive_ids()` + `gc_states_map()` + `last_update_ticks_map()` + `positions_map()` の4回の全人口スキャンを行う。

## Test Plan

- **T1**: `alive_ids()` の結果と、Phase 1 前に1回だけ呼び出した生存者リストが一致する
- **T2**: Phase 1 出生後、出生者が生存者リストに追加されることを確認
- **T3**: Phase 4 GC死亡後、死亡者が同 tick 内の生存者リストから除外されない（次 tick 先頭で再計算される）ことを確認

## Boy Scout Rule — 翻訳可能性計画

- `alive_ids()` の呼び出し集約により、関数シグネチャが「どの生存者リストで動作するか」を明示する
- `phase1_population_growth` から生存者フィルタリング責務を分離

## Acceptance Criteria

- [ ] 各 tick の先頭で1回だけ生存者リストを構築し、全フェーズに結果を共有する
- [ ] Phase 1 出生後に出生者が生存者リストに追加される
- [ ] 3つのシミュレーションループすべてで同じパターンを適用する
- [ ] 既存テストがすべて通過する（`cargo test`）
- [ ] `cargo check --features server` が警告ゼロ

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

- 計画: context/0150-untitled-5/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0150-untitled-5/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0150-untitled-5/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0150-untitled-5/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
