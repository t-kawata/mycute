---
ticket_id: 71
title: M1.5-R11: Event Architecture 較正候補定数 + プロパティベース不変条件ファジング
slug: m15-r11-event-architecture
status: reviewed
created_at: 2026-05-24
updated_at: 2026-05-24
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0071-m15-r11-event-architecture/plan.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0071-m15-r11-event-architecture/observation-20260524-134843.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0071-m15-r11-event-architecture/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0071-m15-r11-event-architecture/review.md
---
# M1.5-R11: Event Architecture 較正候補定数 + プロパティベース不変条件ファジング

## Summary

Event Architecture の較正候補定数を `constants.rs` に追加し、`proptest` によるプロパティベース不変条件ファジングを実装する。既存の FakeEventBus に対して4つの核不変条件（publish→replay 完全性、TwoWay 状態遷移有限停止、clock 単調増加性、quarantine 除外）を proptest 戦略で検証し、パラメータ空間における violation 分布を観測可能にする。failing seed を replay fixture に昇格する機構も含む。

## Background

M1.5-R4〜R10 で Event Architecture の全基盤型・トレイト・実装が完成したが、以下の2点が未対応である：

1. **constants.rs に Event Architecture 定数が未定義**: EVENT_BUS_CHANNEL_CAPACITY 等のバッファサイズ・タイムアウト・リトライポリシーがどこにも定義されていない。これらは Calibration Candidates として管理される必要がある。
2. **プロパティベーステスト戦略が未実装**: 既存のテストは `StdRng` を使用したループベースのランダムテスト（n=1000）であり、`proptest` による戦略ベースのテストではない。proptest の収束的縮約（shrinking）によるエッジケース探索が行われていない。

既存のテスト（n=1000 の一括 publish/replay、64スレッド並行アクセス等）は全て PASS しており、現状の標準パラメータ範囲における不変条件違反はゼロであることが観測済み。本チケットでは proptest 戦略を導入することで、より広範なパラメータ空間とエッジケースでの不変条件検証を自動化する。

## Scope

- `constants.rs` への Event Architecture 較正候補定数7件の追加
- `event.rs` への proptest 戦略群の実装（`darvium_event_strategy()`, `event_kind_strategy()`, `interaction_mode_strategy()`）
- 以下の5不変条件に対する proptest ベース invariant suite の実装:
  1. publish 後のイベントが replay で必ず取得可能（消失率 0%）
  2. TwoWay の状態遷移（open → resolve / abort）が finite ステップで完了
  3. clock の単調増加性（publish/open/resolve/reconnect 後の増加、replay 後不変）
  4. quarantine 後のイベントが検索から除外される
  5. projection の独立性（cross-contamination 0）
- failing seed export → replay fixture 昇格機構（`#[should_panic(expected = "failing_seed:")]` パターン）

## Non-scope

- FakeEventBus の内部実装変更（既存の Vec + Arc<Mutex<>> は変更しない）
- DarviumEventBus トレイトのシグネチャ変更
- 既存テスト（TC-1〜TC-11, R10 TC-1〜TC-9）の修正

## Investigation

### コード調査結果

#### constants.rs の現状（定数なし）
- `constants.rs` 全253行において、以下のイベント関連定数は未定義:
  - `EVENT_BUS_CHANNEL_CAPACITY` — 未定義
  - `EVENT_BUS_DEFAULT_TIMEOUT_MS` — 未定義
  - `EVENT_BUS_MAX_RETRY_COUNT` — 未定義
  - `INTERACTION_CLEANUP_INTERVAL_TICKS` — 未定義
  - `EVENT_REPLAY_BATCH_SIZE` — 未定義
  - `PROJECTION_INITIAL_CAPACITY` — 未定義
  - `QUARANTINE_MAX_EVENTS` — 未定義
- 既存の全定数は分類付きで定義されている（Safety Invariant / Environment Policy Knob / Calibration Candidate）

#### proptest 依存関係
- `Cargo.toml` 22行目: `proptest = "1"` が既に追加済み
- `src/` 以下では proptest は一切使用されていない（grep 結果ゼロ）

#### event.rs のテスト構造
- `event.rs` は 3477 行、うち `mod tests` が約2200行
- 既存のランダムテスト補助関数:
  - `generate_random_event_kind(rng: &mut StdRng) -> DarviumEventKind` — 13 variant を等確率で生成
  - `random_event_source(rng: &mut StdRng) -> EventSource` — 5 variant を等確率で生成
  - `create_random_test_event(rng: &mut StdRng) -> DarviumEvent` — 全フィールドランダム
- 並行テストの定数: `BULK_PUBLISH_COUNT = 1000`, `CONCURRENT_THREADS = 64`, `ROUNDTRIP_SAMPLE_SIZE = 1000`
- 既存テストは全て固定シード `StdRng::seed_from_u64(12345)` を使用し PASS 確認済み

#### FakeEventBus の内部実装
- events: `Arc<Mutex<Vec<DarviumEvent>>>` — 追記専用ストア
- clock: `Arc<Mutex<u64>>` — 初期値 0, publish/open/resolve/reconnect で +1
- interactions: `Arc<Mutex<HashMap<String, InteractionRecord>>>` — TwoWay 追跡
- publish: clock 割当て +1, events.push
- open: clock 割当て +1, events.push + interactions.insert
- resolve: clock +1, status→Resolved
- reconnect: clock +1, status→AwaitingExternal
- replay: clock 不変（MUST NOT #3）
- quarantine: events.retain + interactions.remove

### 参照観察レポート

- tickets/context/0070-m15-r10-searchtracetrainingrunlogtrainingorchestrator-eventprojection/observation-20260524-133216.md — DomainProjection 4種の一括配送テスト全9 PASS、フィルタリング精度100%、クロスプロジェクション汚染ゼロを確認
- tickets/context/0069-m15-r9-eventprojection-projectioncatalog/observation-20260524-131633.md — Projection フレームワークの基本テスト全8 PASS

## Test Plan

### constants.rs 追加定数テスト

| TC | 内容 | 種別 |
|----|------|------|
| C-1 | EVENT_BUS_CHANNEL_CAPACITY が 1024 であること | Safety Invariant |
| C-2 | EVENT_BUS_DEFAULT_TIMEOUT_MS が 5000 であること | Calibration Candidate |
| C-3 | EVENT_BUS_MAX_RETRY_COUNT が 3 であること | Calibration Candidate |
| C-4 | INTERACTION_CLEANUP_INTERVAL_TICKS が 100 であること | Calibration Candidate |
| C-5 | EVENT_REPLAY_BATCH_SIZE が 256 であること | Calibration Candidate |
| C-6 | PROJECTION_INITIAL_CAPACITY が 64 であること | Environment Policy Knob |
| C-7 | QUARANTINE_MAX_EVENTS が 10000 であること | Safety Invariant |

### proptest 戦略テスト

| TC | 戦略 | 検証内容 | サンプル数 |
|----|------|----------|-----------|
| P-1 | event_kind_strategy() | 全13 variant の生成が可能 | 1000 |
| P-2 | interaction_mode_strategy() | OneWay/TwoWay 生成 | 100 |
| P-3 | darvium_event_strategy() | 全フィールド値の生成とシリアライズ | 1000 |
| P-4 | ランダムイベント列 publish→replay | 消失率 0%（全イベントが replay 可能） | 10000 |
| P-5 | TwoWay 状態遷移 | open→resolve 有限ステップ完了 | 1000 |
| P-6 | clock 単調増加性 | publish/open/resolve/reconnect 後の増加 | 1000 |
| P-7 | replay 不変性 | replay 前後で clock 不変 | 1000 |
| P-8 | quarantine 除外性 | quarantine 後イベントが search 除外 | 500 |
| P-9 | projection 独立性 | クロスプロジェクション汚染 0 | 1000 |

### 極端値テスト

| TC | 内容 |
|----|------|
| E-1 | EVENT_BUS_CHANNEL_CAPACITY = 1 でもパニックしない |
| E-2 | EVENT_BUS_DEFAULT_TIMEOUT_MS = 0 でもパニックしない |
| E-3 | EVENT_BUS_MAX_RETRY_COUNT = 0 でもパニックしない |

### 環境
- 全ての proptest は `proptest::prelude::*` を使用
- 固定シード PRNG 不要（proptest が自動管理）
- failing seed は `#[should_panic(expected = "failing_seed:")]` で fixture 化可能

## 計装方法・観測対象

### 計装方法

- `println!` + `--nocapture` で proptest 実行結果を構造化テキスト（JSON）として出力
- proptest の `ProptestConfig` でサンプル数を制御（`cases = 10000` 等）
- シード指定: `ProptestConfig { seed: Some(proptest::test_runner::FileFailurePersistence::default()), .. }` で seed 保存

### 観測対象

- **不変条件 violation 率**: fuzz ケース全体に対する invariant violation 率（期待値: 0%）
- **パラメータ空間の violation clustering**: 特定の定数値の組み合わせで violation が偏る領域の有無
- **shrinking 性能**: proptest が violation 発見時に最小ケースへ縮約するまでのステップ数
- **失敗 seed の昇格数**: replay fixture に昇格した seed 数。発見されたエッジケースの蓄積を監視

### 較正計画

- 調整する定数: EVENT_BUS_DEFAULT_TIMEOUT_MS, EVENT_BUS_MAX_RETRY_COUNT, INTERACTION_CLEANUP_INTERVAL_TICKS 等
- 目的関数 J(θ): 不変条件 violation 率 + 性能劣化ペナルティの合成評価
- 停止条件: n >= 10000 の fuzz で violation 0、かつ shrinking が 10 ステップ以内で完了

## Boy Scout Rule — 翻訳可能性計画

- `constants.rs` の Event Architecture 定数追加: 既存のコメントスタイル（分類・デフォルト値・感度分析範囲）に統一。コメントは日本語で「なぜ」を説明し、コードは英単語の定数名で語らせる
- proptest 戦略関数名: `event_kind_strategy` / `darvium_event_strategy` / `interaction_mode_strategy` — 動詞を含めず名詞句で命名
- 既存の `generate_random_event_kind()` 関数は現状維持（既存テストからの依存あり）。新規の proptest 戦略は別関数として追加
- テスト補助関数群の責務は変更しない（ループベーステストと proptest テストは独立して共存）

## Acceptance Criteria

- [ ] constants.rs に7件の Event Architecture 定数が追加され、正しい値で定義されている
- [ ] proptest 戦略群（event_kind_strategy, darvium_event_strategy, interaction_mode_strategy）が実装され動作する
- [ ] 5つの不変条件に対する proptest invariant suite が PASS する
- [ ] 極端値テストでパニックしないことを確認
- [ ] 既存テスト（R4〜R10 全テスト）が依然として PASS する
- [ ] failing seed の replay fixture 昇格機構が動作する
- [ ] proptest fuzz 実行時の計装出力（violation 率等）が観測可能

## Notes

- plan_path: context/0071-m15-r11-event-architecture/plan.md（未作成、/plan-ticket 承認後に作成）
- implementation_path: context/0071-m15-r11-event-architecture/implementation.md（未作成、/start-ticket 実装完了後に作成）
- review_report_path: context/0071-m15-r11-event-architecture/review.md（未作成、/review-ticket 全チェック通過後に作成）
- observation_report_path: context/0071-m15-r11-event-architecture/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）

### 成果物

- 計画: context/0071-m15-r11-event-architecture/plan.md（未作成、/plan-ticket 承認後に作成）
- 実装サマリ: context/0071-m15-r11-event-architecture/implementation.md（未作成、/start-ticket 実装完了後に作成）
- レビュー報告書: context/0071-m15-r11-event-architecture/review.md（未作成、/review-ticket 全チェック通過後に作成）
- 観察レポート: context/0071-m15-r11-event-architecture/observation-YYYYMMDD-HHmmss.md（未作成、/start-ticket 観測テスト実行時に作成。繰り返し実行ごとに新規ファイル）
