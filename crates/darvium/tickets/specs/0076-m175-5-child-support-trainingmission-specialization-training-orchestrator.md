---
ticket_id: 76
title: M1.75-5: child-support TrainingMission specialization および Training Orchestrator 統合
slug: m175-5-child-support-trainingmission-specialization-training-orchestrator
status: reviewed
created_at: 2026-05-24
updated_at: 2026-05-24
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0076-m175-5-child-support-trainingmission-specialization-training-orchestrator/plan.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0076-m175-5-child-support-trainingmission-specialization-training-orchestrator/implementation.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0076-m175-5-child-support-trainingmission-specialization-training-orchestrator/observation-20260524-152336.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0076-m175-5-child-support-trainingmission-specialization-training-orchestrator/review.md
---
# M1.75-5: child-support TrainingMission specialization および Training Orchestrator 統合

## Summary

child workflow が発行する child-support ミッションを Training Plane 上で正しく orchestrate するため、`TrainingMissionKind::ChildSupport` variant の追加、`ChildSupportMissionPayload` データ型の定義、`spawn_child_support_mission()` 生成関数の実装、および production path との混線を防ぐ plane ガードを実装する。

## Background

M1.75-4 までの実装で Adult HELP offer policy と child consent policy の純粋判定器が完成した。次の段階として、child に対して実際に HELP ミッションを発行・実行・完了させる orchestration 基盤が必要である。この orchestration は Training Plane 上で行われ、production path と分離されなければならない。

RFC §16A (Training Plane) および §41B.11 (Child-support TrainingMission specialization) に基づき、以下の設計上の制約が存在する：

- child-support mission は通常の TrainingMission に child-support 固有メタデータを付与した特殊化であり、独立した実行宇宙ではない
- training-production separation (P-10〜P-12) を破ってはならない
- mission 発行・進行・完了の各段階で DarviumEventKind::Training イベントを EventBus へ publish する必要がある
- TrainingRunLog (EventProjection) への記録拡張が必要

## Investigation

### 現状の実装状態

| 項目 | 状態 | 場所 |
|------|------|------|
| `TrainingMission` | stub (`pub struct TrainingMission;`) | `src/types.rs:5034` |
| `TrainingMissionKind` | 未実装 | — |
| `ChildSupportMissionPayload` | 未実装 | — |
| `TrainingOrchestrator` | 未実装 | — |
| `spawn_child_support_mission()` | 未実装 | — |
| `TrainingEvent` | 9 variants あり、ChildSupport 固有 variant なし | `src/event.rs:213-232` |
| `TrainingRunLogProjection` | 実装済み (9 variant subscribe) | `src/event.rs:1139-1156` |
| EventBus publish 基盤 | DarviumEventBus / FakeEventBus 実装済み | `src/event.rs` |
| 既存 ChildSupport モジュール | 未作成 | — |

### 詳細調査結果

1. `src/types.rs:5034` の `TrainingMission` は現在空のユニット構造体であり、フィールド・メソッド・列挙型の区別を一切持たない。
2. RFC §41B.11 は `ChildSupportPolicy` 構造体 (11フィールド: `enabled`, `maxhelpers`, `minadulttrust`, `minadultreputation`, `spatialtopk`, `spatialmaxdistance`, `offerrequired`, `childacceptrequired`, `allowremoteexplorationfraction`, `helpgrowththreshold`, `positionupdatealpha`) を定義している。
3. RFC §41B.11 は `TrainingMission` 拡張として `childtarget: Option<WorkflowGraphId>` と `childsupportpolicy: Option<ChildSupportPolicy>` を推奨している。
4. `TrainingEvent` 列挙型には現在 9 個の variant が存在する。child-support ミッションの実行段階を区別するための variant 追加が可能か検討が必要。
5. 既存の `DarviumEventKind::Training(TrainingEvent)` は child-support と通常 training をイベントレベルで区別しない。区別が必要な場合は `TrainingEvent` に `ChildSupportMissionGenerated` 等の variant を追加するか、payload 内で区別する設計が考えられる。
6. `src/lib.rs` には training モジュールは存在せず、`src/training.rs` / `src/childsupport.rs` は未作成。
7. RFC §41B.17 の推奨実装分割では `src/childsupport.rs` に child-support mission orchestration と helper weighting を配置することを推奨している。

### 参照観察レポート

- tickets/context/0075-adult-help-offer-policy-child-consent-policy/observation-20260524-145607.md — offer 発火率 ~60%、accept 率 ~99.9%
- tickets/context/0074-help-helpproposalhelpofferhelpdecisionhelpexecutionhelpsuccess/observation-20260524-143958.md — HELP 5段階プロトコル状態機械 実装完了
- tickets/context/0073-m175-2-child-adult-maturity-local-village/observation-20260524-141956.md — maturity 判定器 21テスト全PASS
- tickets/context/0072-m175-1-spacepositionembedding-villageposition/observation-20260524-140442.md — 位置埋め込み実装完了
- tickets/context/0071-m15-r11-event-architecture/observation-20260524-134843.md — EventArchitecture 較正候補定数 + プロパティベース不変条件ファジング

## Scope

### 実装スコープ

1. **`TrainingMissionKind` 列挙型の追加** (`src/types.rs`)
   - `enum TrainingMissionKind { Production, ChildSupport }`
   - child-support mission か production mission かを静的に区別する

2. **`ChildSupportMissionPayload` 構造体の定義** (`src/childsupport.rs`)
   - `child_id: EntityId` — 対象 child の識別子
   - `helper_ids: Vec<EntityId>` — 選定された helper (adult) のリスト
   - `village_snapshot: LocalVillage` — ミッション発行時点の village スナップショット
   - `objective: String` — ミッションの目的記述
   - `safety_scope: SafetyScope` — safe sandbox 範囲の指定

3. **`spawn_child_support_mission()` 関数の実装** (`src/childsupport.rs`)
   - `spawn_child_support_mission(child_id, village, policy) -> Option<TrainingMissionKind>`
   - empty village の場合は `None` を返し fallback policy へ移行
   - ミッション発行時に `DarviumEventKind::Training(TrainingEvent::MissionGenerated)` を EventBus へ publish

4. **Production plane ガード**
   - production plane では `TrainingMissionKind::ChildSupport` が直接実行されないことを検証するロジック
   - child-support mission は safe sandbox 条件下でのみ許容

5. **TrainingRunLog 拡張** (`src/event.rs`)
   - HELP execution / outcome / child growth delta の記録を TrainingRunLog に追加
   - 必要に応じて `TrainingEvent` に child-support 特化 variant を拡張

6. **モジュール構成**
   - `src/childsupport.rs` 新規作成 (child-support mission orchestration)
   - `src/lib.rs` に `pub mod childsupport;` を追加し必要な型を re-export

### Non-scope

- helper weighting / bounded remote exploration の実装 (M1.75-6)
- village stability / dynamicity メトリクス (M1.75-7)
- TrainingMission の完全な状態機械 (mission generation, human review, sandbox execution の各 phase は Training Plane 全体として別途)
- ChildSupportPolicy の全フィールドを用いた policy 判定器 (M1.75-4 の policy 判定器を流用)

## Test Plan

### 不変条件テスト (mod tests 内)

| ID | 種別 | 内容 |
|----|------|------|
| T-1 | 正常系 | `spawn_child_support_mission` が適切な child/village に対して `ChildSupport` mission を生成すること |
| T-2 | 異常系 | empty village の child に対して mission が生成されず `None` を返すこと |
| T-3 | 型検証 | `TrainingMissionKind::ChildSupport` と `TrainingMissionKind::Production` の判別がコンパイル時に保証されること |
| T-4 | 境界値 | village に helper が 1 人だけの場合に mission が生成されること |
| T-5 | 正常系 | `ChildSupportMissionPayload` の全フィールドが mission 発行時に正しく設定されること |
| T-6 | 不変条件 | production plane ガード関数が `ChildSupport` を拒否し `Production` を許可すること |
| T-7 | 正常系 | mission 発行時に helper snapshot と village snapshot が payload へ完全記録されること |
| T-8 | 正常系 | `TrainingEvent::MissionGenerated` が EventBus へ publish されること |
| T-9 | 異常系 | 空の helper_ids で mission が生成されないこと |
| T-10 | 境界値 | child maturity が Adult 未満の場合に mission が生成されること (Child のみ対象) |

### EventBus 結合テスト

| ID | 種別 | 内容 |
|----|------|------|
| T-E1 | 結合 | child-support mission 発行 → EventBus publish → FakeEventBus でイベント受信確認 |
| T-E2 | 結合 | TrainingRunLog (EventProjection) が child-support mission 生成イベントを materialize すること |

### 観測テスト

| ID | 内容 | サンプルサイズ |
|----|------|---------------|
| T-O1 | child-support mission 発行率の観測 (各種 village サイズで sweep) | n >= 1,000 |
| T-O2 | mission 発行から完了までの統計分布 (到達長・完了率) | n >= 500 |

## 計装方法・観測対象

### 計装方法

- `src/childsupport.rs` 内の `mod tests` で全テストを実装
- `println!` + `--nocapture` で観測データを標準出力へ書き出す
- 固定シード PRNG: `StdRng::seed_from_u64(12345)`

### 観測対象

- child-support mission 発行率 (child あたりの平均発行数)
- mission 実行完了率
- empty village における fallback 移行率
- 発行された mission の helper 数分布

### 較正計画

- 該当する constants.rs 定数: 新規追加の mission-related 定数 (max_helpers_per_mission 等)
- 目的関数 J(θ): mission 成功率 / 発行コスト比

## Boy Scout Rule — 翻訳可能性計画

- `src/types.rs:5034` の stub `TrainingMission` を整理し、`TrainingMissionKind` として再定義する（Boy Scout: 触るコードは美しく）
- 新規作成する `src/childsupport.rs` は翻訳可能性を最初から徹底：
  - 関数名は動詞句 (`spawn_child_support_mission`, `validate_mission_village` 等)
  - 変数名はドメイン概念 (`child_id`, `village`, `helper_pool` 等)
  - 一関数一責務を徹底
  - ハードコード値は全て `constants.rs` の名前付き定数に
  - エラーの握りつぶし禁止 (全て `Result` 伝播)

## Acceptance Criteria

- [ ] `TrainingMissionKind::ChildSupport` が定義され、child workflow のみが ChildSupport mission を生成できること
- [ ] `ChildSupportMissionPayload` が全必須フィールドを持ち、village snapshot を含むこと
- [ ] `spawn_child_support_mission()` が正常系・境界値・異常系で正しく動作すること
- [ ] production plane ガードが child-support mission を safe sandbox に制限すること
- [ ] empty village では mission が生成されず、fallback policy へ移行すること
- [ ] EventBus への publish と TrainingRunLog への記録が正しく行われること
- [ ] 翻訳可能性の検証が通っている
- [ ] 既存テストが全て通過している

## Notes

### 成果物

- 計画: context/0076-m175-5-child-support-trainingmission-specialization-training-orchestrator/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0076-m175-5-child-support-trainingmission-specialization-training-orchestrator/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0076-m175-5-child-support-trainingmission-specialization-training-orchestrator/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0076-m175-5-child-support-trainingmission-specialization-training-orchestrator/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
