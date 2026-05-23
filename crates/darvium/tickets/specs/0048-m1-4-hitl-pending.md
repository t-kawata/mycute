---
ticket_id: 48
title: M1-4: HITL 起動時回復ループ — 全Pendingインタラクションの確実な再開保証
slug: m1-4-hitl-pending
status: reviewed
created_at: 2026-05-23
updated_at: 2026-05-23
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0048-m1-4-hitl-pending/plan.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0048-m1-4-hitl-pending/observation-20260523-130402.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0048-m1-4-hitl-pending/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0048-m1-4-hitl-pending/review.md
---
# M1-4: HITL 起動時回復ループ — 全Pendingインタラクションの確実な再開保証

## Summary

プロセス再起動後、MetadataStore 上に Pending 状態で残存する全 HITL インタラクションを
確実に回復する起動時回復ループを実装する。
M-0.5-4 で提供された `HumanChannel::reconnect()` と MetadataStore 4 メソッドを
Orchestrator レベルで統合し、単一・複数・タイムアウト済みを含むあらゆる状態からの
再開を保証する。

**中核要件: HITL インタラクションの 100% 再開保証。**
以下のギャップを全て解消する：
- 複数 Pending インタラクションの一括 `list_pending → reconnect × N → wait → resolve` ループ
- StdinoutChannel のプロセス再起動越え回復（クロスインスタンス）
- TimedOut 状態からの回復経路
- 回復中の競合状態（旧プロセスが応答受信直後にクラッシュ等）

## Background

M-0.5-4（チケット #47）は HumanChannel トレイト・FakeHumanChannel・StdinoutChannel・
MetadataStore 4 メソッドを実装したが、以下の制約が残されている：

1. **擬似サイクル検証のみ**: T10-7/T10-8 は単一 Pending インタラクションの擬似回復のみ。
   複数同時 Pending のバッチ回復は未テスト。
2. **StdinoutChannel クロスインスタンス未検証**: reconnect() は同一プロセス内でのみ
   動作保証。プロセス再起動後の新チャネルインスタンスでの回復は未テスト。
3. **TimedOut 状態の回復未定義**: 状態機械 (§12B.5) に TimedOut は定義されているが、
   TimedOut からの回復経路が設計されていない。
4. **競合状態未テスト**: 旧プロセスが応答受信直後にクラッシュする等のタイミング競合。

これらのギャップは RFC §12B.13 で明記され、本チケットで解決される。

## Scope

- **JsonMetadataStore 実装（簡易ファイル永続化）**
  - 起動時に JSON ファイルから全レコード読み込み
  - 変更操作（store / resolve）のたびにファイルへ原子書き込み（一時ファイル + rename）
  - 既存 MetadataStore トレイトを実装
  - 依存追加不要（serde_json は既存）
- Orchestrator レベルの起動時回復ループ実装
  - `MetadataStore::list_pending_human_interactions()` 全件走査
  - 各レコードに対して `channel.reconnect()` → `handle.wait(timeout)` → `resolve_human_interaction()`
  - 回復失敗時の再試行戦略（`HITL_RECONNECT_BACKOFF_SECS` による指数バックオフ）
- 複数 Pending インタラクションの一括回復テスト
  - N ≥ 10 の同時 Pending からの全件回復
  - 一部のみ成功・一部タイムアウトの混合シナリオ
- StdinoutChannel クロスインスタンス回復テスト
  - 新チャネルインスタンス（プロセス再起動後）で同一 interaction_id の reconnect
  - 外部アプリ同時クラッシュからの回復（MetadataStore にリクエスト全文が残っている前提）
- TimedOut 状態からの回復経路定義とテスト
  - TimedOut を吸収状態とせず、再通知可能な経路の設計
- 回復中競合状態のテスト
  - 旧プロセス応答受信直後クラッシュ
  - reconnect() 中の MetadataStore 更新競合

## Non-scope

- WebSocketChannel / HttpChannel 等の新規チャネル実装（M-0.5-4 から継続して後段）
- SqliteMetadataStore 実装（JsonMetadataStore で代替）
- HumanReviewQueue との統合（M1-1 の範囲）

## Test Plan

### 不変条件テスト（assert! / assert_eq!）

1. **JsonMetadataStore 基本動作**: store → ファイル書込 → 再読込で同一内容が復元されること
2. **JsonMetadataStore 原子書き込み**: 書込途中のクラッシュ後も、事前の完全な状態がファイルに残っていること
3. **単一 Pending 回復**: T10-7 相当の統合版。Orchestrator ループ経由で回復できること
4. **N≥10 一括回復**: 10 件の Pending を一括 list_pending → reconnect → resolve し全件成功
5. **混合シナリオ**: 5 件成功 + 3 件タイムアウト + 2 件到達不能の混合、残りを正しく Pending 維持
6. **StdinoutChannel クロスインスタンス**: 新 StdinoutChannel で同一 interaction_id の reconnect
7. **TimedOut 再通知**: TimedOut 状態のインタラクションを再通知可能であること
8. **競合状態**: 旧プロセス応答直後クラッシュ → 再起動後回復の一貫性
9. **初回起動時ファイル不在**: ファイルが存在しない状態で JsonMetadataStore を初期化し、空状態から正常に動作すること
10. **ファイル破損（不正 JSON）**: 破損した JSON ファイルからの読み込みが Err を返し、クラッシュや無限再試行に至らないこと
11. **異種チャネル差し替え回復**: FakeHumanChannel で保存した Pending レコードを StdinoutChannel 経由で回復できること（RFC §12B.6「再起動後に別のチャネル実装に差し替え」の具体的検証）

### 観測テスト（println! + --nocapture）

9. **OTS-1: バッチ回復成功率**: N ∈ {1, 10, 100} における回復成功率の統計分布
10. **OTS-2: 回復レイテンシ分布**: 回復ループ全体の経過時間（中央値・P90・P99）

## Acceptance Criteria

1. JsonMetadataStore がファイルへの永続化・復元を正しく行い、再起動後もデータが生存すること
2. `list_pending_human_interactions()` が返す全件が `reconnect()` 経由で回復可能であること
3. 回復ループ完了後、全インタラクションの MetadataStore ステータスが Resolved / TimedOut のいずれかであること
4. 回復失敗時（予定）の再試行が指数バックオフに従うこと
5. 単一 Pending 回復と複数 Pending 回復で成功率に差がないこと（N≥10 評価）
6. StdinoutChannel がプロセス再起動後も interaction_id ベースで回復できること
7. TimedOut 状態のインタラクションが再通知可能であること
8. 競合状態が回復一貫性を破壊しないこと
9. 初回起動時（ファイル不在）に JsonMetadataStore が空状態で正しく初期化されること
10. ファイル破損時に JsonMetadataStore が Err を返し、プロセスが起動不能に陥らないこと
11. FakeHumanChannel で永続化された Pending レコードが StdinoutChannel 経由でも回復可能であること

## 計装方法・観測対象

- **バッチ回復成功率**: list_pending 件数に対する reconnect 成功件数の比率
- **回復レイテンシ分布**: 回復ループ開始から全件解決までの経過時間の統計分布
- **再試行回数分布**: 回復失敗時の再試行回数（指数バックオフの動作確認）
- **TimedOut 変換率**: TimedOut 経由で打ち切られたインタラクションの比率
- **競合検出率**: 競合状態テストでの不整合検出率（期待値: 0）
- **ファイル不在復旧率**: 初回起動時（空ファイル状態）に JsonMetadataStore が Err を返さず初期化できることの確認率
- **ファイル破損耐性**: 破損 JSON → Err 検出までのハンドリング成功率（期待値: 100%）
- **異種チャネル回復率**: FakeHumanChannel 保存 → StdinoutChannel 回復の成功率
