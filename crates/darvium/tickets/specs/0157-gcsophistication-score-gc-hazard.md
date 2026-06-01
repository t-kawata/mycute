---
ticket_id: 157
title: 洗練スコアのGC保護組み込み（sophistication_score → GC hazard）
slug: gcsophistication-score-gc-hazard
status: reviewed
created_at: 2026-06-01
updated_at: 2026-06-01
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0157-gcsophistication-score-gc-hazard/plan.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0157-gcsophistication-score-gc-hazard/implementation.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0157-gcsophistication-score-gc-hazard/observation-20260601-171246.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0157-gcsophistication-score-gc-hazard/review.md
---
# 洗練スコアのGC保護組み込み（sophistication_score → GC hazard）

## Summary

現在 GC hazard は LifecycleScore（5成分）、Benevolence、Child Protection のみで計算されている。本チケットでは洗練スコア（sophistication_score = 0.5×abstraction_ratio + 0.5×abstraction_depth）を第4の保護項として GC hazard に追加する。

Phase 3.7（首長性計算）で既に計算されている洗練スコアを MemoizedGraph にキャッシュし、Phase 4（GC 実行）でそのキャッシュ値を参照する方式（都度計算ではなくキャッシュ参照）で実装する。

## Background

**現状**: #156 の議論で、洗練スコアが現在 GC にまったく影響していないことが判明した。sophistication_score は首長選出（Phase 3.7→3.8）専用であり、生存確率には反映されていない。

**「洗練スコアが高いほど死ににくい」の根拠**: 
- 洗練された（高 abstraction、深いネスト）ワークフローは質の高い資産であり、保全する価値がある
- 首長性スコア全体（= 評判 + 洗練）を GC に入れると Kind World 創発・子供保護等との矛盾が生じるが、洗練スコア単独なら追加の保護項として安全に導入できる

**キャッシュ方式を選ぶ理由**:
- `compute_sophistication_score(graph, store)` は SubWorkflow を再帰探索するコストの高い処理
- Phase 3.7 で既に全生存個体に対して計算している
- 同じ tick の Phase 4 で再計算するのは無駄
- MemoizedGraph にキャッシュしておけばゼロコストで参照可能

## Scope

1. **`MemoizedGraph` に `sophistication_score: f32` フィールド追加**
2. **`compute_gc_hazard` に sophistication_score 引数追加 + F-7 式拡張**
3. **`ReciprocityLifecyclePolicy` に `gamma_sophistication` 追加**（デフォルト 0.5）
4. **`compute_and_update_gc_state` でキャッシュ値を参照**
5. **Phase 3.7 で計算結果をキャッシュに書き込み**
6. **default / 定数 / 全呼び出し箇所の更新**

## Non-scope

- compute_sophistication_score 自体の変更
- chiefdom_score 計算ロジックの変更
- LifecycleScore への sophistication 直接組み込み
- 首長性スコア全体の GC 反映（評判の二重計上回避）

## Investigation

### 物理的証拠

**証拠1: GC Hazard の現在の式（`src/reciprocity.rs:305-316`）**
→ 第4引数 `sophistication_score` を追加し、式に末尾項 `- gamma_S * sophistication_score` を加える。

**証拠2: MemoizedGraph に sophistication_score がない**
→ 新規フィールド追加が必要。

**証拠3: Phase 3.7 の実装（`simulation.rs:~2434`）**
→ `sophistication_score` を計算後破棄している。キャッシュ書き込みを追加。

**証拠4: compute_and_update_gc_state（`lifecycle.rs:90`）**
→ ここで `graph.sophistication_score` を読み取り `compute_gc_hazard` に渡す。

### 参照観察レポート
- tickets/context/0156-population-control/observation-20260601-164229.md — #156 人口圧力制御実装、GC hazard 式構造の詳細

## Test Plan

### ユニットテスト
| # | テスト名 | 内容 |
|---|---------|------|
| TC1 | sophistication=0.0 → 既存 hazard 計算と同等 |
| TC2 | sophistication=1.0 → hazard が有意に減少 |
| TC3 | sophistication 増加 → hazard 非増加（単調性） |
| TC4 | gamma_soph=0.0 → 従来動作と完全一致（非回帰） |

### 観測テスト
| # | テスト名 | 内容 |
|---|---------|------|
| OT1 | Phase 3.7 計算後、Phase 4 でキャッシュ値が使われる確認 |

## 計装方法・観測対象

- `println!` で各 hazard 計算時の `(lifecycle, benevolence, child_prot, sophistication) → hazard` を CSV 出力
- 固定シード `StdRng::seed_from_u64(12345)`
- 観測: sophistication 上位 20% vs 下位 20% の生存率差（有意に上位が高いこと）
- 較正: `gamma_sophistication: 0.5` 初期値（γ_L=1.0 より低め、γ_B=0.10 より高め）

## Boy Scout Rule — 翻訳可能性計画

- 新規フィールド名: `sophistication_score` — ドメイン概念を直接表現
- 変数名: `sophistication`, `gamma_sophistication` — 翻訳可能
- `compute_and_update_gc_state` のコメントに「9. graph.sophistication_score を第4項として hazard 計算に使用」を追記

## Acceptance Criteria

- [ ] `compute_gc_hazard` が sophistication_score を受け取り hazard 計算に反映する
- [ ] sophistication_score=0.0 で既存結果が完全一致する（非回帰）
- [ ] sophistication_score が高いほど hazard が非増加する（単調性）
- [ ] MemoizedGraph.sophistication_score が Phase 3.7 で書き込まれ Phase 4 で参照される
- [ ] 既存テストがすべて通過する

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

- 計画: context/0157-gcsophistication-score-gc-hazard/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0157-gcsophistication-score-gc-hazard/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0157-gcsophistication-score-gc-hazard/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0157-gcsophistication-score-gc-hazard/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
