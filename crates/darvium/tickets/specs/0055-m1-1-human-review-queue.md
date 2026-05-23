---
ticket_id: 55
title: 'M1-1: NeedsHumanReview 発生時の隔離レビューキューイングロジックの検証'
slug: m1-1-human-review-queue
status: reviewed
created_at: 2026-05-23
updated_at: 2026-05-23
plan_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0055-m1-1-human-review-queue/plan.md
observation_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0055-m1-1-human-review-queue/observation-20260523-181239.md
implementation_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0055-m1-1-human-review-queue/implementation.md
review_report_path: /Users/kawata/shyme/mycute/crates/darvium/tickets/context/0055-m1-1-human-review-queue/review.md
---

# M1-1: `NeedsHumanReview` 発生時の隔離レビューキューイングロジックの検証

## Summary

SearchWorkflow から `SearchOutcome::NeedsHumanReview` が発行された際に、
該当ミッションとコンテキストを専用のメモリ内キュー `HumanReviewQueue` へ隔離し、
人間の明示的な応答（`HumanDecision::Approved` / `Rejected`）が `HumanChannel` 経由で
到着するまで通常の自動実行ラインに絶対に復帰させない隔離機構を実装・検証する。

**中核要件: レビュー待ちミッションの自動実行ラインへの情報漏洩確率 $P_{leak} = 0$。**
自動実行スレッドからのアクセスに対して無限大ポテンシャル障壁を維持する。

## Background

以下の既存実装の上に構築される：

1. **SearchOutcome::NeedsHumanReview** (§13.3): 既存の enum variant。`guard.rs` の
   `guard_new_proposal_or_review()` が GenerateNew の不安全ケースでこの outcome を発行。
2. **HumanChannel トレイト** (§12B, M-0.5-4): `communicate()` / `notify()` / `reconnect()` を提供。
   `InteractionHandle::wait()` でブロッキング待機。
3. **JsonMetadataStore** (M1-4): Pending インタラクションのファイル永続化と起動時回復ループを提供。
   M1-4 の Non-scope で「HumanReviewQueue との統合（M1-1 の範囲）」と明記。

**既存のギャップ:**
- `NeedsHumanReview` は発行されるが、それを受けてミッションをキューイングし、隔離し、
  人間の応答を待つ統合ロジックが未実装。
- キューイング後の自動実行ラインへのリーク防止が未検証。
- スレッド安全性（複数探索スレッドからの同時アクセス）が未検証。
- キューイングされたミッションの状態遷移（Pending → Resolved/TimedOut）が未定義。

## Scope

- **`HumanReviewQueue` 構造体の実装**
  - スレッドセーフなメモリ内キュー（`Mutex<VecDeque<QueuedReview>>` または同等）
  - `QueuedReview`: ミッションID、コンテキスト、`HumanRequest`、到着時刻、状態を持つ
  - `push()`: キューへ追加し Pending 状態に固定
  - `pop_next()` / `peek_next()`: 優先度考慮デキュー（`HumanReviewQueuePolicy.priority_aware_dequeue`）
  - `len()` / `is_empty()` / `pending_count()`
  - `resolve()`: HumanDecision 到着時にキューから除去し状態解決
  - `timeout_expired()`: review_timeout_secs 超過検出

- **自動実行ライン隔離機構**
  - キュー上の Pending ミッションが自動実行の検索候補として出現しない保証
  - `HumanReviewQueue` は自動実行パイプラインの検索対象から本キュー上の assets を除外する
  - ミッションがキューイングされている間は `EvaluateCandidatesStep` / `RefineSearchPolicyStep` から
    絶対にアクセスされないことを保証する論理的分離壁

- **HumanChannel 統合**
  - push() 時に `HumanChannel::communicate()` を呼び出し、`HumanRequest` を生成
  - 応答（`HumanOutcome::Responded` / `TimedOut` / `Unreachable`）の受信処理
  - `HumanDecision` の各値に対応した状態遷移ハンドリング

- **観測計装**
  - キュー滞留長の時間発展測定（$\mu = 0$ 時の線形成長 $L_q(t) = \lambda t$）
  - スレッド競合時のセマフォ待機時間分布
  - 情報リーク率の統計的検定 $\hat{P}_{leak}$

## Non-scope

- WebSocketChannel / HttpChannel / Slack 等の新規チャネル実装
- SqliteMetadataStore（JsonMetadataStore で代替済み）
- 起動時回復ループ全体（M1-4 の範囲）
- AdminFastTrack（M1-2 の範囲）
- Debounce ロジック（M1-3 の範囲）
- Training Orchestrator レベルのミッション編集・マージ（§13A 規範要件2、上位レイヤー責務）

## 実装対象の型定義

以下の型を `src/types.rs` または専用モジュールに追加する：

```rust
/// レビュー待ちミッションのキューエントリ (RFC §16A.1)。
pub struct QueuedReview {
    pub mission_id: String,
    pub context: serde_json::Value,
    pub request: HumanRequest,
    pub arrived_at: std::time::Instant,
    pub status: QueuedReviewStatus,
}

pub enum QueuedReviewStatus {
    Pending,
    Resolved(HumanDecision),
    TimedOut,
}

/// 隔離レビューキュー (RFC §16A.1)。
///
/// スレッドセーフなメモリ内キュー。
/// SearchOutcome::NeedsHumanReview を受け取り、HumanChannel 経由の
/// 人間応答があるまでミッションを自動実行ラインから隔離する。
pub struct HumanReviewQueue {
    queue: Mutex<VecDeque<QueuedReview>>,
    channel: Arc<dyn HumanChannel>,
    policy: HumanReviewQueuePolicy,
    /// 観測用: 到着イベントの時系列 (秒単位のタイムスタンプ)
    arrival_timeline: Mutex<Vec<f64>>,
    /// 観測用: 解決イベントの時系列
    resolution_timeline: Mutex<Vec<(f64, HumanDecision)>>,
    /// 観測用: セマフォ待機時間のサンプル (秒)
    contention_samples: Mutex<Vec<f64>>,
    /// 観測用: リーク試行検出カウンタ
    leak_attempts: AtomicU64,
    /// 観測用: リーク成功検出カウンタ (期待値: 常に 0)
    leak_successes: AtomicU64,
}
```

## 不変条件 (MUST)

1. **隔離不変条件**: Pending 状態の QueuedReview は、対応する HumanDecision が到着するまで
   あらゆる自動実行パス（検索候補評価・パッチ生成・ワークフロー実行）から参照されてはならない。
2. **状態不変条件**: 各 QueuedReview は一度だけ resolve() される。二重解決は禁止。
3. **キュー不変条件**: push() される QueuedReview.mission_id はキュー内で一意でなければならない。
4. **時間不変条件**: review_timeout_secs 超過後は resolve() または escalate() のいずれかが
   呼ばれなければならない。

## Test Plan

### 不変条件テスト (assert! / assert_eq!)

1. **T1: 基本キューイング・解決サイクル**: push → 状態が Pending → resolve(Approved) →
   キューから除去され、応答が返る。
2. **T2: 複数キューイング**: 10 件連続 push → len() == 10 → 順次 resolve → 最終的に空。
3. **T3: 隔離障壁**: キューイング後、自動実行パイプラインの検索関数が当該ミッションを
   絶対に返さないことを mock 経由でアサート。
4. **T4: 二重解決禁止**: 同一 QueuedReview を 2 回 resolve → panic または Err。
5. **T5: mission_id 一意性**: 同一 mission_id の重複 push → Err。
6. **T6: 優先度デキュー**: 高優先度ミッションが low priority より先に pop_next() される
   （`priority_aware_dequeue = true` 時）。
7. **T7: タイムアウト検出**: review_timeout_secs 超過後、timeout_expired() が true を返す。
8. **T8: スレッドセーフ同時 push**: 10 スレッドから同時に 100 件ずつ push → 最終件数 1000。
9. **T9: スレッドセーフ同時 push/pop**: 5 スレッド push + 5 スレッド pop → デッドロック・
   データ競合・panic なし。
10. **T10: キュー空時の pop**: 空キューで pop_next() → None。

### 観測テスト (println! + --nocapture)

1. **OTS-1: 線形成長ダイナミクス**: $\mu = 0$（解決なし）状態で $\lambda$ を変化させたときの
   $L_q(t)$ の時間発展を計測し、理論値 $L_q(t) = \lambda t$ との一致を確認。
   - $\lambda \in \{1, 5, 10\}$ (到着/秒)、観測時間 $t \in [0, 10]$ 秒
   - 出力: 各 $\lambda$ について時刻ごとの $L_q(t)$ 時系列、線形回帰の傾きと $R^2$

2. **OTS-2: スレッド競合待機時間分布**: 16 スレッドからの同時アクセス時の
   Mutex 待機時間の分布を計測。
   - 中央値・P90・P99 を出力
   - 期待値: 待機時間は有限かつ bounded

3. **OTS-3: 情報リーク率の統計的検定**: $N = 10000$ 回の自動実行パイプライン走査で
   キューイング済みミッションが漏洩した回数を計測。
   - 出力: $\hat{P}_{leak} = 0$ の片側検定 $p$-値

4. **OTS-4: 多様な HumanDecision 応答パターン**: Approved / Rejected / NeedsRevision /
   Irrelevant / Unsafe の 5 値をそれぞれ 20 回ずつ注入し、各 decision に対応する
   状態遷移とキュー滞留時間の分布を出力。

## Acceptance Criteria

1. 全不変条件テスト (T1–T10) が PASS
2. OTS-1: 各 $\lambda$ で線形回帰の $R^2 \ge 0.95$
3. OTS-2: P99 待機時間が 100ms 未満（模擬負荷時）
4. OTS-3: $\hat{P}_{leak} = 0$ かつ $p \ge 0.999$（10000 回試行で 1 件も漏洩なし）
5. OTS-4: 各 HumanDecision が期待通りの状態に解決され、キュー滞留時間が有限であること
6. HumanChannel 統合: push → communicate() → wait() → resolve() の一連がエラーなく完了
7. キュー状態が M1-4 の JsonMetadataStore と統合可能であること（Interface Segregation）

## 計装方法・観測対象

| 観測量 | 方法 | 期待値 |
|--------|------|--------|
| $L_q(t)$ 時間発展 | arrival_timeline / resolution_timeline からの離散サンプリング | $R^2 \ge 0.95$ の線形 |
| スレッド競合待機時間 | contention_samples への elapsed 記録 | P99 < 100ms |
| 情報リーク率 $\hat{P}_{leak}$ | leak_attempts / leak_successes | 厳密に 0 |
| 状態遷移正解率 | 各 decision 注入後の解決状態アサート | 100% |
| スループット | push/pop の 1 秒間あたり処理件数 | ベースライン計測 |
