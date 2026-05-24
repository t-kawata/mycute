---
ticket_id: 74
title: HELP プロトコル (HelpProposal/HelpOffer/HelpDecision/HelpExecution/HelpSuccess) 状態機械の実装
slug: help-helpproposalhelpofferhelpdecisionhelpexecutionhelpsuccess
status: reviewed
created_at: 2026-05-24
updated_at: 2026-05-24
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0074-help-helpproposalhelpofferhelpdecisionhelpexecutionhelpsuccess/plan.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0074-help-helpproposalhelpofferhelpdecisionhelpexecutionhelpsuccess/observation-20260524-143958.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0074-help-helpproposalhelpofferhelpdecisionhelpexecutionhelpsuccess/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0074-help-helpproposalhelpofferhelpdecisionhelpexecutionhelpsuccess/review.md
---

# HELP プロトコル (HelpProposal/HelpOffer/HelpDecision/HelpExecution/HelpSuccess) 状態機械の実装

## Summary

M1.75-3 は Child Support Villages / HELP Consensus（v2.3-e）マイルストーンの第3チケットとして、RFC §41B.4-41B.9 で定義された HELP 5段階プロトコルの純粋状態機械を実装する。HelpProposal → HelpOffer → HelpDecision（Accept/Reject）→ HelpExecution → HelpSuccess の遷移を形式化し、各遷移を EventBus 経由で DarviumEventKind::Reciprocity イベントとして publish する。終端状態（Rejected / Succeeded / Failed）からの再遷移を厳格に禁止する。

## Background

M1.75-1（SpacePositionEmbedding）と M1.75-2（Child/Adult maturity / Local Village）は完了し、村形成の基盤が整った。HELP プロトコルは、Adult が Child に対して行う支援の統治プロトコルである。以下の理由から純粋状態機械として独立した実装が必要である：

1. HELP は単なる構造体定義ではなく、**状態遷移の正当性保証**が要求される（違法遷移禁止）
2. 全 HELP 状態遷移は EventBus へ publish され、監査・リプレイ可能でなければならない
3. 後続チケット（M1.75-4: adult offer policy / child consent policy, M1.75-5: TrainingMission 統合）は本状態機械を前提とする

### 参照観察レポート

- tickets/context/0073-m175-2-child-adult-maturity-local-village/observation-20260524-141956.md — 村構成ロジック全テスト PASS、既存テストとの競合なし
- tickets/context/0072-m175-1-spacepositionembedding-villageposition/observation-20260524-140442.md — 位置更新ダイナミクス全テスト PASS

## Scope

以下の実装を含む：

1. **HelpState 列挙型**: `{Proposal, Offered, Accepted, Rejected, Executing, Succeeded, Failed}` の7状態
2. **構造体定義**: `HelpProposal`, `HelpOffer`, `HelpDecision`, `HelpExecution`, `HelpSuccess`, `HelpFailure`
3. **理由コード列挙型**: `HelpRejectionReason`, `HelpFailureReason`
4. **遷移判定純粋関数**: `is_legal_help_transition(current: &HelpState, next: &HelpState) -> bool`
5. **HelpSession**: `transition_to(next)` メソッド付きの状態機械コンテナ（ガード実装）
6. **EventBus publish**: `emit_help_event(session, transition)` — 各遷移を `DarviumEventKind::Reciprocity` イベントとして publish
7. **新規モジュール `src/help.rs`**: `pub mod help` を `lib.rs` に追加
8. **EventBus publish される DarviumEvent の payload**: `help_id`, `from_workflow`, `to_workflow`, `transition_type`, `timestamp_vt` を含む

## Non-scope

- Adult 側の offer policy（M1.75-4 で実装）
- Child 側の consent policy（M1.75-4 で実装）
- TrainingMission との統合（M1.75-5 で実装）
- SQLite 永続化（HELP session はメモリ状態機械として実装し、永続化は上位レイヤーに委譲）
- Helper weighting / ranking（M1.75-6 で実装）

## Investigation

### 証拠1: 既存 ReciprocityEvent は HELP variant を定義済みだが状態機械は未実装

`src/event.rs:304-322` に `ReciprocityEvent` 列挙型が定義されており、HELP 関連の variant として以下が存在する：

```rust
pub enum ReciprocityEvent {
    HelpOffered,
    HelpAccepted,
    HelpRejected,
    HelpExecuted,
    HelpSucceeded,
    HelpAbandoned,
    HarmfulMismatch,
    ReturnedFavor,
}
```

これらは EventBus 経由で publish 可能なイベント種別であるが、HELP 状態機械（HelpState, HelpSession, 遷移行列）は一切存在しない。

### 証拠2: EventBus 基盤は既に整備済み

`event.rs` に以下が実装済み：

- `DarviumEventKind::Reciprocity(ReciprocityEvent)` — `event.rs:362` 以降の extensible taxonomy
- `DarviumEvent` 構造体 — `event_id`, `kind`, `interaction_mode`, `payload`, `causality`
- `DarviumEventBus` トレイト — `publish`, `subscribe`, `replay` メソッド
- `FakeEventBus` 実装 — テスト用インメモリ実装
- `DomainProjection::reciprocity_event()` — 既に HELP イベントを購読する Projection 定義

### 証拠3: 状態機械実装のための新規モジュールが必要

- `src/help.rs` — 未作成
- `src/lib.rs` の module 一覧に `pub mod help;` なし
- `src/constants.rs` に HELP 関連定数なし

### 証拠4: RFC §41B.4-41B.9 に5段階 HELP プロトコルの定義あり

RFC 6521-6779 に以下が存在：
- `HelpOffer` 構造体: `helpofferid`, `missionid`, `childgraphid`, `adultgraphid`, 他
- `HelpOfferState` 列挙型: `Pending`, `Accepted`, `Rejected`, `Expired`
- `HelpMode` 列挙型: `ReuseAsSubWorkflow`, `ComposeWithChild`, `PatchChild`, `DemonstrationOnly`
- 5段階プロトコル: `HelpProposal → HelpOffer → HelpDecision → HelpExecution → HelpSuccess`
- 各段階の数式: (41B-8)〜(41B-16)

### 証拠5: 既存の類似実装パターン

`src/village.rs` が同一マイルストーンのパターンを示す：

- 単一ファイルに型定義、純粋関数、ユニットテスト（mod tests）を集約
- `lib.rs` への `pub mod` 追加 + `pub use` による再公開
- テストは `mod tests` 内にチケット内で一意な prefix（T-1, T-2, ...）で記述

## Test Plan

### 単体テスト（src/help.rs mod tests 内）

#### T-1: 全合法遷移の遷移行列総当たりテスト
- 7×7 = 49 通りの遷移すべてが `is_legal_help_transition` で正しく判定されること

#### T-2: 正常系列完走テスト
- `Proposal → Offered → Accepted → Executing → Succeeded` が `HelpSession::transition_to` でエラーなく実行できること

#### T-3: Rejected 終端テスト
- `Proposal → Offered → Rejected` の遷移後、Rejected からの再遷移がすべて `Err` を返すこと

#### T-4: Failed 終端テスト
- `Executing → Failed` 後の再実行や Succeeded への飛び遷移が不可能であること

#### T-5: 違法遷移 rejection テスト
- 直接 Succeeded への遷移（Proposal→Succeeded）、Accepted→Succeeded のスキップなど、全違法遷移パターンが `transition_to` で拒否されること

#### T-6: EventBus publish テスト
- 各正当遷移の実行後に、対応する `DarviumEventKind::Reciprocity` イベントが EventBus へ publish されていること
- 遷移種別とイベント種別の一致検証

#### T-7: EventBus replay 完全性テスト
- 全遷移系列を publish 後、EventBus の `replay()` で全イベントが取得可能であること
- イベント件数が遷移回数と一致すること

#### T-8: 構造体フィールド整合性テスト
- `HelpProposal`, `HelpOffer`, `HelpDecision`, `HelpExecution`, `HelpSuccess`, `HelpFailure` が期待されるフィールドを持つこと
- serde Serialize/Deserialize 対応（JSON ラウンドトリップ）

#### T-9: HelpRejectionReason / HelpFailureReason の全 variant enum テスト

#### T-10: 空 EventBus（None）時の publish 耐性テスト
- EventBus が None の場合でも `transition_to` が正常に動作すること

### 観測テスト

#### T-O1: ランダム遷移系列の違法遷移流入フラックス観測
- 固定シード PRNG（`StdRng::seed_from_u64(12345)`）
- n >= 10,000 のランダム遷移系列で違法遷移フラックスが厳密に 0 であることを観測

#### T-O2: 吸収状態までの平均到達長・終端分布観測
- n >= 5,000 のランダム遷移系列
- 吸収状態（Rejected, Succeeded, Failed）までの平均到達長と終端分布

#### T-O3: EventBus 上の HELP イベント一貫性検証
- n = 1,000 遷移の系列で遷移系列と EventBus イベント系列の完全対応を検証

## 計装方法・観測対象

### 計装方法

- テストコード: `src/help.rs` 内の `mod tests`
- 観測出力: `println!` + `--nocapture` 経由で標準出力に構造化テキスト
- PRNG 固定シード: `StdRng::seed_from_u64(12345)` を使用し完全再現を保証

### 観測対象

| 統計量 | サンプルサイズ | 期待値 |
|--------|---------------|--------|
| 違法遷移フラックス | n >= 10,000 | 厳密に 0 |
| 平均到達長（成功系列） | n >= 5,000 | 4 (5段階中4遷移) |
| EventBus 一貫性 | n = 1,000 | 完全一致（不変条件） |

### 較正計画

本チケットでは較正対象の定数は存在しない（純粋状態機械のため）。

## Boy Scout Rule — 翻訳可能性計画

- **新規 `help.rs` は翻訳可能性を最初から徹底**:
  - 関数名は動詞句（`is_legal_help_transition`, `emit_help_event`）
  - 構造体名はドメイン概念名詞（`HelpSession`, `HelpProposal`）
  - 一関数一責務を厳守（遷移判定と publish は別関数）
  - ハードコード値は定数化（将来の較正に備える）
  - エラー握りつぶし禁止: `transition_to` は `Result` を返し、違法遷移は `Err` で報告
- **既存コード改善**: 本チケットでは `event.rs` の既存定義に触れない（状態機械との分離を維持）

## Acceptance Criteria

- [ ] `HelpState` 7状態 + 遷移行列の正当性証明（総当たりテスト PASS）
- [ ] 正常系列（Proposal→Succeeded）完走テスト PASS
- [ ] 終端状態（Rejected/Failed）からの再遷移禁止テスト PASS
- [ ] EventBus publish 連携テスト PASS
- [ ] EventBus replay 完全性テスト PASS
- [ ] serde ラウンドトリップ対応
- [ ] ランダム遷移系列 n >= 10,000 で違法遷移フラックス = 0 観測
- [ ] n = 1,000 遷移の EventBus 一貫性検証 PASS
- [ ] cargo test 全PASS（既存 + 新規）
- [ ] cargo clippy 全通過

## Notes

### 成果物

- 計画: context/0074-help-helpproposalhelpofferhelpdecisionhelpexecutionhelpsuccess/plan.md（未作成）
- 実装サマリ: context/0074-help-helpproposalhelpofferhelpdecisionhelpexecutionhelpsuccess/implementation.md（未作成）
- レビュー報告書: context/0074-help-helpproposalhelpofferhelpdecisionhelpexecutionhelpsuccess/review.md（未作成）
- 観察レポート: context/0074-help-helpproposalhelpofferhelpdecisionhelpexecutionhelpsuccess/observation-YYYYMMDD-HHmmss.md（未作成）
