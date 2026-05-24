---
ticket_id: 69
title: M1.5-R9: EventProjection フレームワーク + ProjectionCatalog 実装
slug: m15-r9-eventprojection-projectioncatalog
status: reviewed
created_at: 2026-05-24
updated_at: 2026-05-24
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0069-m15-r9-eventprojection-projectioncatalog/plan.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0069-m15-r9-eventprojection-projectioncatalog/implementation.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0069-m15-r9-eventprojection-projectioncatalog/observation-20260524-131633.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0069-m15-r9-eventprojection-projectioncatalog/review.md
---

# M1.5-R9: EventProjection フレームワーク + ProjectionCatalog 実装

## Summary

DarviumEvent のストリームからドメイン固有の投影ビューを materialize する
`EventProjection` トレイトおよびその管理コンテナ `ProjectionCatalog` を実装する。
Projection はイベントソーシングの読み取りモデルとして機能し、基盤の EventBus
に一切影響を与えてはならない (MUST NOT)。

## Background

v2.3-g Event Architecture において、DarviumEventBus が全イベントの commit 基盤
として整備された。しかし、イベントを**読み取り専用のドメイン別ビュー**として
解釈する仕組みが未実装である。具体的には:

- SearchTrace は SearchWorkflow の各ステップを時系列で追跡する
- ReciprocityProjection は互恵性イベントから信頼状態を導出する
- FusionTrace / LifecycleLog も同様にドメイン別の投影を必要とする

これらの投影は DarviumEventBus のイベント列から独立に materialize 可能であり、
EventBus 自体に変更を加えることなく追加的に構築できる (MUST)。

RFC §12E では `ProjectionEngine` として規定されているが、本チケットでは
チケット仕様 (Darvium-Tickets-v2.3.md) に従い `ProjectionCatalog` として実装する
（機能的に等価であり、名前の違いである）。

## Scope

- `EventProjection` トレイト: `project()`, `snapshot()`, `clear()` (sync版)
- `ProjectionCatalog`: `register()`, `get()`, `project_all()`
- `ProjectionEventFilter`: どの `DarviumEventKind` をどの projection に配送するかのフィルタ定義
- `FakeProjectionCatalog`: メモリ内実装
- エラー型: `DarviumError::Projection` variant の追加

## Non-scope

- SearchTrace / ReciprocityProjection / FusionTrace / LifecycleLog 等の
  ドメイン投影の実装はチケット M1.5-R10 に委ねる
- async 版の `EventProjection` は本チケットでは対象外（既存コードベースが
  sync パターンを採用しているため、M1.76-21 の外部イベント購読基盤と併せて検討）
- スナップショットの永続化戦略 (OQ-15) は本チケットの対象外

## Investigation

### 現状確認 (2026-05-24)

**既存実装の全容:**

- `src/event.rs`: DarviumEvent / DarviumEventKind 全13 variant 定義済み。
  DarviumEventBus トレイト + FakeEventBus 実装済み (21 tests + ~30 tests)。
  行数: ~2300行 (単一ファイル)。
- `src/error.rs`: EventBus error variants は定義済み。Projection 関連 variant なし。
  行数: 138行。
- `src/event_channel.rs`: EventChannel トレイト + StdinoutEventChannel (21 tests)。
  sync パターンを採用。
- `src/lib.rs`: 全 event モジュールの re-export 済み。

**RFC §12E との差異:**
| 項目 | RFC | チケット仕様 | 採用方針 |
|------|-----|------------|---------|
| コンテナ名 | `ProjectionEngine` | `ProjectionCatalog` | チケット仕様に従う |
| フィルタ方式 | `interested_kinds()` trait メソッド | `ProjectionEventFilter` 外部定義 | 両方採用（trait 内判定 + 外部フィルタの併用） |
| `clear()` | なし | あり | チケット仕様に従う |
| async | `#[async_trait]` | 暗黙的に sync | sync 採用（既存 event モジュールと統一） |

**設計判断:**
- `EventProjection` トレイトは RFC の `interested_kinds()` を保持し、
  `ProjectionEventFilter` と二重にフィルタ可能にする
- `project()` は `Result<(), DarviumError>` を返し、エラー分離は呼び出し側
  (`ProjectionCatalog::project_all`) で実現する
- 既存の `DarviumError` に `Projection(String)` variant を追加する
- FakeProjectionCatalog は `Arc<Mutex<HashMap<String, Arc<dyn EventProjection>>>>` で実装

**参照観察レポート:**
- `tickets/context/0068-m15-r8-eventchannel-stdinouteventchannel-canonical-json-lines/observation-20260524-130605.md`
  — R8 完了。EventChannel トレイト + StdinoutEventChannel 全 21 tests PASS。
  消失率 0%。次への示唆として WebSocketEventChannel への拡張性を記録。
- `tickets/context/0065-m15-r5-darviumeventbus-fakeeventbus/observation-20260524-115247.md`
  — R5 完了。DarviumEventBus + FakeEventBus 動作確認済み。
  n=1000 一括 publish 完全性、n=64 並行 clock 一意性を確認。

## Test Plan

### TC-1: EventProjection トレイト境界のコンパイル時検証
- `FakeProjection` が `EventProjection` を実装可能であることを型システムで確認
- `Send + Sync` 境界を充足すること

### TC-2: 単一 projection の project() + snapshot() ラウンドトリップ
- `project()` でイベントを投入後、`snapshot()` で状態を取得
- イベント数に応じて snapshot が正しく更新されること

### TC-3: 複数 projection への同時配送 (project_all)
- 2つの独立した projection を catalog に登録
- 1イベントを `project_all()` で配送
- 両 projection が同一イベントを受け取っていること

### TC-4: ProjectionEventFilter フィルタリング
- SearchEvent のみを受け取る projection と TrainingEvent のみを受け取る projection
- SearchEvent 配送時、Search projection のみが更新されること
- TrainingEvent 配送時、Training projection のみが更新されること

### TC-5: clear() 後スナップショット
- 複数イベント投入後に `clear()` を呼び出し
- `snapshot()` が空の状態を返すこと (e.g., `serde_json::Value::Null` または空配列)

### TC-6: クロスプロジェクション汚染ゼロ (cross-projection contamination)
- projection A に 5 イベント、projection B に 2 イベントを配送
- A の snapshot に 5 件のデータが含まれ、B の snapshot に 2 件のデータが含まれること
- A と B の状態が互いに影響しないこと

### TC-7: FakeProjectionCatalog の get() / register()
- `register()` で登録した projection が `get()` で取得できること
- 未登録の name に対する `get()` が None を返すこと
- 同一名の重複登録が上書きされること（またはエラー）

### TC-8: 計装 — n = 1000 イベント一括配送後、各 projection の独立完全性
- 3つの projection を catalog に登録
- 各 projection が異なる kind フィルタを持つ
- 1000 イベントをランダム生成し `project_all()` で一括配送
- 各 projection の snapshot に正しい kind のイベントのみが含まれること
- フィルタリング精度 (配送イベントの kind 一致率 100%) を検証

## 計装方法・観測対象

### 計装方法
- 全テストは同一ファイル内の `mod tests` に実装
- 固定シード PRNG (`StdRng::seed_from_u64(12345)`) を使用
- `println!` + `--nocapture` で観測データを標準出力に書き出す
- n = 1000 の計装テストで配送完全性・フィルタリング精度を観測

### 観測対象
| 観測量 | サンプルサイズ | 検証方法 |
|--------|---------------|----------|
| project + snapshot ラウンドトリップ完全性 | 全テスト | assert_eq! |
| フィルタリング精度 (kind 一致率) | n = 1000 | 統計的観測 (100%) |
| クロスプロジェクション独立完全性 | 全テスト | assert 確認 |
| clear 後状態リセット | 全テスト | assert 確認 |

## Boy Scout Rule — 翻訳可能性計画

### 対象ファイル
- `src/event.rs`: 本チケットの実装を末尾の mod tests に追加。既存の ~2300行
  は編集せず、additive な追加のみ。
- `src/error.rs`: `DarviumError::Projection` variant を EventBus セクション
  直後に追加。

### 改善項目
- チケット仕様で規定された関数名は全て英語の動詞句 (`project`, `snapshot`,
  `clear`, `register`, `get`, `project_all`)
- 変数名はドメイン概念 (`projection`, `catalog`, `filter`, `event`) で統一
- ハードコードせず、テストサンプルサイズは `const BULK_EVENT_COUNT: usize = 1000`
  として定数化

## Acceptance Criteria

- [ ] EventProjection トレイト (project / snapshot / clear) が定義されている
- [ ] ProjectionCatalog (register / get / project_all) が実装されている
- [ ] ProjectionEventFilter が実装されている
- [ ] FakeProjectionCatalog が実装されている
- [ ] TC-1〜TC-8 が全て PASS すること
- [ ] 既存テストに影響を与えないこと

## Notes

- plan_path: context/0069-m15-r9-eventprojection-projectioncatalog/plan.md (未作成、/plan-ticket 承認後に作成)
- implementation_path: context/0069-m15-r9-eventprojection-projectioncatalog/implementation.md (未作成、/start-ticket 実装完了後に作成)
- review_report_path: context/0069-m15-r9-eventprojection-projectioncatalog/review.md (未作成、/review-ticket 全チェック通過後に作成)
- observation_report_path: context/0069-m15-r9-eventprojection-projectioncatalog/observation-YYYYMMDD-HHmmss.md (未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル)
