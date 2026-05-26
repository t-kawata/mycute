---
ticket_id: 108
title: M1.76-23: 全ドメイン横断 Event Architecture 一貫性検証
slug: m176-23-event-architecture
status: reviewed
created_at: 2026-05-26
updated_at: 2026-05-26
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0108-m176-23-event-architecture/plan.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0108-m176-23-event-architecture/observation-20260526-150906.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0108-m176-23-event-architecture/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0108-m176-23-event-architecture/review.md
---
# M1.76-23: 全ドメイン横断 Event Architecture 一貫性検証

## Summary

全13ドメイン（System・Search・WorkflowExecution・Training・Knowledge・Conversational・Lifecycle・GC・Repair・Reciprocity・Fusion・HITL・Village）の DarviumEvent が、EventBus 経由で統一された canonical envelope により publish・subscribe・replay・projection されることを検証する。ドメイン横断一貫性スコア 1.0 を完了条件とする。

## Background

M1.5-R 系列で整備された Event Architecture（DarviumEvent canonical envelope・DarviumEventBus・EventProjection・ProjectionCatalog）は、既に個別ドメインでの実装とテストが完了している。しかし、**全13ドメインを横断した統合的な一貫性検証は未実施**である。

既存の `test_r10_cross_domain_contamination_zero`（event.rs:3904）は Search・Training・Reciprocity の3ドメインのみを対象としており、WorkflowExecution・Knowledge・Conversational・Lifecycle・GC・Repair・Fusion・HITL・Village・System の10ドメインが未検証である。また、全13種の EventKind に対して publish → replay の完全取得性や subscribe フィルタの正確性が確認されていない。

M1.76-22 で実装された EventBus 運用メトリクス観測パイプラインは FakeEventBus の透過性を確認したが、ドメイン横断の意味的正しさ（他ドメインイベントの相互汚染がないこと）は検証範囲外であった。本チケットはこのギャップを埋める。

## Scope

1. **全ドメインイベント生成ヘルパー関数の実装**: `make_search_event`、`make_training_event` 等、13ドメイン × 各サブイベント種別の `DarviumEvent` 生成ヘルパー（現在未実装）
2. **全13ドメインの Projection 定義**: 現状5種類のみ（search_trace・training_run_log・reciprocity_event・search_run_log・village_observation_log）。不足する System・WorkflowExecution・Knowledge・Conversational・Lifecycle・GC・Repair・Fusion・HITL の9ドメイン projection を DomainProjection のコンストラクタとして追加
3. **ドメイン横断一貫性検証スイート**:
   - 全13種の event_kind が publish → replay → kind 一致
   - 全13種を混在 publish → subscribe フィルタで正しく分別
   - 各 domain projection が自身の event_kind 以外を受け取らないこと（相互汚染ゼロ）
   - 全13種が同一 EventBus で一貫したクロック進行を示すこと
   - 全13種の JSON シリアライズ/デシリアライズラウンドトリップ完全性
4. **ドメイン間イベント相互汚染検出器**: Search イベントが Training projection に漏れていないこと等を検証

## Non-scope

- ConcreteEventBus（MetadataStore 結合版）の実装 — FakeEventBus のみを対象とする
- EventBus パフォーマンス測定やレイテンシ検証
- 新たな EventKind variant の追加
- 既存 Projection のフィルタロジック修正（search_run_log の subset 設計は維持）
- Extension escape hatch の projection 対応（非ドメインイベントのため対象外）

## Investigation

### ソースコード調査結果

**DarviumEventKind 列挙型**（event.rs:717-746）:
- 13 domain variant + Extension escape hatch の計14 variant
- 各 variant は対応するサブイベント型を持つ（SystemEvent, SearchEvent, WorkflowExecutionEvent 等）

**DomainProjection コンストラクタ**（event.rs:1699-1772）:
- 現状5種類のみ実装済み:
  - `search_trace()` — Search 全5種
  - `training_run_log()` — Training 全9種
  - `reciprocity_event()` — Reciprocity 全8種
  - `search_run_log()` — Search subset（StepCompleted/Completed/Failed/Aborted）
  - `village_observation_log()` — Village(TickCompleted)
- 不足9ドメイン: System, WorkflowExecution, Knowledge, Conversational, Lifecycle, GC, Repair, Fusion, HITL

**既存クロスプロジェクション汚染テスト**（event.rs:3904-3960）:
- `test_r10_cross_domain_contamination_zero` — Search・Training・Reciprocity の3ドメインのみ検証
- 他10ドメインは未検証

**イベント生成ヘルパー**:
- `make_*_event` 関数は全く未実装
- `create_event_with_kind`（event.rs のテスト補助関数）は Kind 指定のみで payload の一貫性が低い
- `generate_random_event_kind`（event.rs:2357）は 13 domain（0..13）のサブイベントをランダム生成するが、Extension を含めると14種

**既存テスト群**（event.rs:1871-）:
- TC-4: `test_darvium_event_json_roundtrip_n1000` — ランダムイベントの JSON ラウンドトリップ（n=1000）
- 既に FakeEventBus の publish/replay/subscribe 機能は十分にテスト済み
- Projection 機能（FakeProjectionCatalog）の kind フィルタリングは実証済み（`test_r10_cross_domain_contamination_zero`）

**参照観察レポート**:
- `tickets/context/0107-m176-22-event-architecture/observation-20260526-144721.md` — EventBus 運用メトリクス観測パイプライン統合。FakeEventBus 全7メソッドの透過性確認。9カウンタ + 3補助指標の正確性を実証。本チケットで使用する FakeEventBus の信頼性基盤。
- `tickets/context/0105-m176-20/observation-20260526-141743.md` — 較正フェーズ実験レポート。較正ループの停止条件設計に示唆。

## Test Plan

### TC-1: 全13ドメイン DomainProjection コンストラクタ
- System・WorkflowExecution・Knowledge・Conversational・Lifecycle・GC・Repair・Fusion・HITL の9種を新規追加
- 正常系: 各コンストラクタが `DomainProjection` を返し、`interested_kinds()` が自身のドメインに属する kind のみを返す
- 境界値: サブイベント0件の DomainProjection（該当するサブイベントがないドメインがあった場合の挙動確認）

### TC-2: 全13ドメイン publish → replay 完全取得性
- 130件（13 domain × 10 events）を FakeEventBus に publish
- `replay(0, EventFilter::all())` で全件取得できること
- 130件の kind が publish 時と一致すること
- 再現性のため PRNG 固定シードを使用

### TC-3: subscribe フィルタ分別精度
- 13 domain のイベントを混在 publish（計130件）
- 各 domain 専用の subscribe フィルタで受信したイベントが自身の kind のみであること
- 他 domain のイベントが1件も混入していないこと（レート: 1.0 = 完全分別）

### TC-4: 全13 Projection 相互汚染ゼロ
- 全13 domain の projection を FakeProjectionCatalog に登録
- 130件を混在 publish し、`project_all` 経由で配送
- 各 projection の `received_events()` に自身以外の kind が存在しないこと
- サブセット projection（search_run_log）は自身のフィルタ定義に従う subset のみ受信

### TC-5: 全13ドメイン一貫クロック進行
- 13 domain 混在 publish 130件を同一 FakeEventBus 経由で実行
- 全イベントの `metadata.clock` が単調増加していること
- 同一 clock 値の重複がないこと（FakeEventBus の atomic increment 保証）

### TC-6: 全13ドメイン JSON ラウンドトリップ完全性（n=1300）
- 各 domain から n=100 件のランダムイベントを生成
- serde_json のシリアライズ/デシリアライズが全件成功すること
- ラウンドトリップ前後で完全一致すること

### TC-7: 観測テスト — n=1300 ランダム publish 系列
- 13種の event_kind を各 n=100 件、計1300イベントをランダム順に publish
- 以下の指標を観測:
  - replay 完全取得率（全1300件が取得可能）
  - kind フィルタ精度（各ドメイン100件のみ通過）
  - クロック単調増加性（1300件の clock が strict に単調増加）
  - projection 配送完全性（各 projection が正確に100件を受信）
- ドメイン横断一貫性スコア = 全指標の加重平均（1.0 を完了条件）

## 計装方法・観測対象

### 計装方法
- テストコードは `src/event.rs` の既存 `mod tests` 内に追記
- 固定シード `StdRng::seed_from_u64(12345)` を全観測テストで使用（既存の TC-4 等と同一シード）
- 新規ヘルパー関数 `make_system_event()`、`make_search_event()`、`make_training_event()` 等は test モジュールではなく公開 API（`pub fn`）として event.rs に実装（他のドメインからの利用を想定）
- 観測結果は `println!` + `--nocapture` で JSON 構造化出力

### 観測対象

| 指標 | 計算方法 | 完了条件 |
|------|----------|----------|
| replay 完全取得率 | replay 件数 / publish 件数 | = 1.0 |
| kind フィルタ精度 | 正しく分別された件数 / 全件数 | = 1.0 |
| クロック単調増加性 | 隣接 clock 差の最小値 | > 0 |
| projection 配送完全性 | 各 projection 受信件数 / 期待件数 | = 1.0 |
| JSON ラウンドトリップ成功率 | 成功件数 / 全件数 | = 1.0 |
| **一貫性スコア（総合）** | 全指標の加重平均 | = 1.0 |

サンプルサイズ: TC-7 で n=1300（各ドメイン n=100 × 13 domain）、ラウンドトリップは n=1300。分布同定ではなく一貫性検証が目的のため、n >= 130 で十分。

### 較正計画
本チケットは新規一貫性検証の実装が主目的であり、較正対象の定数はない。ただし、テストで不足9ドメインの DomainProjection コンストラクタを追加する際に、各ドメインのサブイベントリストの完全性を constants 定義と照合する。既存定数 `EVENTBUS_SUBSCRIPTION_MAX_KINDS = 32`（Safety Invariant）は13ドメインを十分カバーすることを確認する。

## Boy Scout Rule — 翻訳可能性計画

- **不足9ドメインの DomainProjection コンストラクタ追加時**: 既存の `search_trace()` 等と同一パターンで実装し、関数名（`with_filter`, `search_trace`, `training_run_log` 等）は既に「翻訳可能」な動詞句/名詞句であるため、新規追加も同一命名規則に従う
- **全13ドメインの `make_*_event` ヘルパー**: 関数名は `make_${domain}_event` の形式で統一（動詞句 + ドメイン名）、各ヘルパー内で `DarviumEvent { .. }` の構造体リテラルを完全指定する（可読性重視、ビルダーパターンは過剰設計と判断）
- **`create_event_with_kind` 既存関数**: テスト補助関数として利用価値が高いが、payload が `serde_json::Value::Null` 固定であり、より現実的なペイロードを持つヘルパーが上位互換となる。既存コードは維持し、新規の `make_*_event` がこれを置き換える位置づけとする
- **ハードコード値**: `generate_random_event_kind` 内の `match rng.random_range(0..13)` のマジックナンバー 13 は `DarviumEventKind` variant 数に依存するが、コンパイル時チェックが効かない。ただし本チケットでは新たな定数抽出は行わず、既存コードへの介入は最小限に留める

## Acceptance Criteria

- [ ] 全13ドメインの `make_*_event` ヘルパー関数が実装されている（公開 API）
- [ ] 不足9ドメインの DomainProjection コンストラクタが追加されている
- [ ] TC-1〜TC-7 の全テストが通過している
- [ ] ドメイン横断一貫性スコアが 1.0 であること（観測テスト TC-7）
- [ ] 既存テスト（M-2〜M1.76-22 の全テスト）が退行していない
- [ ] 翻訳可能性の検証が通っている
- [ ] RFC §12C の全13種 DarviumEventKind が一貫した canonical envelope で publish されることの確認が完了している

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

- 計画: context/0108-m176-23-event-architecture/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0108-m176-23-event-architecture/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0108-m176-23-event-architecture/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0108-m176-23-event-architecture/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
