---
ticket_id: 140
title: 評判ベースGCのプロダクション実装とMYCUTE結合設計
slug: gcmycute
status: reviewed
created_at: 2026-05-28
updated_at: 2026-05-28
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0140-gcmycute/plan.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0140-gcmycute/observation-20260528-173503.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0140-gcmycute/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0140-gcmycute/review.md
---

# 評判ベースGCのプロダクション実装とMYCUTE結合設計

## Summary

シミュレーション内でのみ動作している GC/Lifecycle パイプライン（`compute_lifecycle_score` + `compute_gc_hazard` + `transition_gc_state`）をプロダクションでも実行可能にする。同時に、Darvium crate と MYCUTE アプリケーションの結合設計を確立する。

## Background

シミュレーションでは `phase4_gc_survival`（simulation.rs:2660）が定期的に GC を実行し、`LifecycleScore` → `compute_gc_hazard` → `transition_gc_state` → `compute_survival_probability` のパイプラインが完全に動作している。一方、プロダクションでは：

1. **LifecycleScore が計算されない**: `compute_lifecycle_score`（lifecycle.rs:40）はプロダクションから呼ばれていない
2. **gc_state が死んでいる**: `MemoizedGraph.gc_state`（trust.rs:59）は `GcEvent::Active` で初期化されるが、`transition_gc_state` が呼ばれないため、どのグラフも初期状態から遷移しない
3. **生存率判定がない**: `compute_survival_probability`（reciprocity.rs:347）で計算された確率に基づく GC 削除が行われない
4. **Darvium が MYCUTE に統合されていない**: `darvium::Darvium` は MYCUTE のどのコードからも参照されていない

本チケットでは、Darvium 内部でプロダクション GC を駆動する facade メソッドを追加し、MYCUTE との結合設計を確立する。MYCUTE の実際のコード変更は最小限にとどめ、結合インターフェースの定義を主目的とする。

## Scope

### A. Darvium facade に GC 駆動メソッドを追加
- `Darvium::run_lifecycle_gc()` メソッドを追加
- 入力: `WorkflowRegistry`, `ReciprocityLifecyclePolicy`, 現在 tick
- 内部処理:
  1. 全グラフの `LifecycleScore` を計算
  2. 全グラフの GC hazard を計算
  3. 各グラフの `gc_state` を `transition_gc_state` で遷移
  4. `HardDeleteCandidate` 以上の gc_state を持つグラフを削除候補として返す
- 出力: `Vec<WorkflowGraphId>`（削除候補リスト）

### B. LifecycleScore 計算のプロダクション対応
- `freshness`: `last_update_tick` と現在 tick からの経過時間に基づく blended freshness
- `usage`: `experience_count` の正規化（`compute_experience_normalization`）
- `trust`: `MemoizedGraph.trust` の3成分平均値
- `reputation`: `MemoizedGraph.reputation.final_score`
- `success`: 当面スタブ値（0.5、シミュレーションの `PHASE4_LIFECYCLE_SUCCESS_STUB` を流用）

### C. 削除候補グラフの処理
- `DualStoreCoordinator` に `delete_graph(graph_id)` メソッドを追加
- GraphStore からの物理削除 + 論理削除可否の判断

### D. DarviumConfig の拡張
- `gc_interval: u64` — GC tick 間隔（デフォルト: 1000）
- `gc_enabled: bool` — GC 有効/無効（デフォルト: false。安全のため opt-in）
- `lifecycle_success_stub: f64` — success 成分スタブ値（デフォルト: 0.5）

### E. MYCUTE 結合設計の提示
- MYCUTE 側の想定利用コードを設計書として提示（実際の変更は行わない）
- 必要な Darvium 公開 API の拡張項目を洗い出し

### F. テスト
- `run_lifecycle_gc` のユニットテスト（正常系、空レジストリ、無効設定）
- 削除候補の GraphStore 整合性テスト

## Non-scope

- `ConcreteEventBus` の実装（別チケット）
- Lifecycle/GC イベントの発行（別チケット）
- プロダクションの HELP プロトコル実行経路（別チケット#139）
- MYCUTE アプリケーションコードの実際の変更
- GC しきい値の較正
- success スタブ値の較正
- 分散環境での GC

## Investigation

### [E1] compute_lifecycle_score はプロダクション未使用 → #140 で run_lifecycle_gc 経由で使用開始

lifecycle.rs:40:
```rust
pub fn compute_lifecycle_score(
    freshness: f64, success: f64, trust: f64, usage: f64, reputation: f64,
) -> LifecycleScore {
    let geometric_mean = (freshness * success * trust * usage * reputation).powf(1.0 / 5.0);
    LifecycleScore { freshness, success, trust, usage, reputation, geometric_mean }
}
```
lib.rs:90 で `pub use` されている。#140 実装により `Darvium::run_lifecycle_gc()` から呼ばれるようになった。

### [E2] gc_state は MemoizedGraph に存在するが遷移されない → #140 で遷移開始

trust.rs:59: `gc_state: GcEvent` — 常に `GcEvent::Active` で初期化される。#140 実装により `run_lifecycle_gc()` 内で `transition_gc_state` 経由で遷移されるようになった。

### [E3] 全 GC 関連関数はプロダクション未使用 → #140 で使用開始

| 関数 | 定義 | プロダクション呼び出し |
|------|------|----------------------|
| `compute_gc_hazard` | reciprocity.rs:304 | run_lifecycle_gc |
| `compute_survival_probability` | reciprocity.rs:347 | なし（別チケット） |
| `transition_gc_state` | event.rs:313 | run_lifecycle_gc |
| `recompute_all_gc_hazards` | reciprocity.rs:772 | なし（別チケット） |

### [E4] Darvium は MYCUTE に統合されていない

`~/shyme/mycute` 内で `darvium::Darvium` を使用しているコードは皆無。MYCUTE の `Cargo.toml` に darvium crate の依存関係が存在しない。

### [E5] シミュレーション GC（phase4_gc_survival）の完全ロジック

simulation.rs:2758-2804:
1. LifecycleScore 計算
2. `compute_gc_hazard(lifecycle, benevolence, child_prot, policy)`
3. `transition_gc_state(current_state, hazard)`
4. `compute_survival_probability(hazard, 1)` と乱数比較 → alive 判定

このロジックをプロダクションに移植した（生存確率判定は別チケット）。

### [E6] DarviumConfig は最小構成のみ → #140 で拡張

lib.rs:314: `pub struct DarviumConfig { gc_interval, gc_enabled, lifecycle_success_stub }` — GC 設定項目追加済み。
デフォルト: gc_interval=1000, gc_enabled=false (opt-in), lifecycle_success_stub=0.5。

### 参照観察レポート

- `tickets/context/0137-untitled/observation-20260528-154420.md` — チケット#137。プロダクション GC は未着手。

## Test Plan

### T1: run_lifecycle_gc 正常系
- 複数グラフ登録の `WorkflowRegistry` で GC 実行
- 削除候補が空でない（低 LifecycleScore のグラフが含まれる場合）
- 高 LifecycleScore のグラフが削除候補にならない

### T2: 空レジストリ
- 空レジストリで GC → 空リスト

### T3: GC 無効設定
- `gc_enabled: false` → 空リスト（一切処理しない）

### T4: gc_state 遷移正当性
- `transition_gc_state` が各状態を正しく遷移
- 禁止遷移（`Protected`→`Tombstoned` 直接）が発生しない

### T5: 削除候補の実際の削除
- `delete_graph` で GraphStore から削除確認
- 削除後のロードがエラーを返す確認

### T6: 既存テスト回帰なし
- `cargo test` 全テスト通過

## 計装方法・観測対象

### 計装方法
- T1-T5: 通常の `#[test]` + `println!` + `--nocapture`
- T6: `cargo test`

### 観測対象
- T1: 削除候補数と LifecycleScore 分布
- T6: PASS/FAIL カウント

### 較正計画
- 本チケットでは較正は行わない

## Boy Scout Rule — 翻訳可能性計画

- `DarviumConfig` にフィールド追加時、名前が散文として読めることを確認
- `run_lifecycle_gc` 内の責務分割を確認（シミュレーションの `phase4_gc_survival` との重複を避ける）
- `delete_graph` のエラーハンドリングの適切性を確認

## Acceptance Criteria

- [ ] `Darvium::run_lifecycle_gc()` が実装され、LifecycleScore → GC hazard → state transition → 削除候補出力のパイプラインが動作する
- [ ] `DarviumConfig` に `gc_interval`, `gc_enabled`, `lifecycle_success_stub` が追加されている
- [ ] `DualStoreCoordinator` に `delete_graph()` が追加されている
- [ ] MYCUTE 結合設計書が提示されている
- [ ] 全テスト通過

## 成果物

- 計画: context/0140-gcmycute/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0140-gcmycute/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0140-gcmycute/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0140-gcmycute/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
