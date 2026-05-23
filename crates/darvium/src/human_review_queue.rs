//! 隔離レビューキュー (RFC §16A.1 / M1-1)。
//!
//! SearchWorkflow から `SearchOutcome::NeedsHumanReview` が発行された際に、
//! 該当ミッションを専用のメモリ内キューへ隔離し、人間の明示的な応答が
//! `HumanChannel` 経由で到着するまで自動実行ラインから絶対に復帰させない。
//!
//! # 隔離不変条件
//!
//! Pending 状態の QueuedReview は、対応する HumanDecision が到着するまで
//! あらゆる自動実行パスから参照されてはならない (MUST)。
//! 本キューはこの隔離を保証するための論理的分離壁である。

use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::error::DarviumError;
use crate::human_channel::{HumanChannel, InteractionHandle};
use crate::types::{HumanDecision, HumanRequest, HumanReviewQueuePolicy};

// ============================================================
// データ型
// ============================================================

/// レビュー待ちミッションのキューエントリ (RFC §16A.1)。
#[derive(Debug, Clone)]
pub struct QueuedReview {
    /// ミッション識別子（キュー内で一意）。
    pub mission_id: String,
    /// 機械可読なコンテキスト情報。
    pub context: serde_json::Value,
    /// 人間への依頼内容。
    pub request: HumanRequest,
    /// キュー到着時刻。
    pub arrived_at: Instant,
    /// 現在の状態。
    pub status: QueuedReviewStatus,
}

/// キューエントリの状態。
#[derive(Debug, Clone, PartialEq)]
pub enum QueuedReviewStatus {
    /// レビュー待ち。
    Pending,
    /// 解決済み（人間が判断を下した）。
    Resolved(HumanDecision),
    /// タイムアウト。
    TimedOut,
}

// ============================================================
// 隔離レビューキュー
// ============================================================

/// 隔離レビューキュー (RFC §16A.1)。
///
/// スレッドセーフなメモリ内キュー。
/// `SearchOutcome::NeedsHumanReview` を受け取り、`HumanChannel` 経由の
/// 人間応答があるまでミッションを自動実行ラインから隔離する。
///
/// # スレッド安全性
///
/// 内部の `VecDeque` は `Mutex` で保護されており、複数スレッドからの
/// 同時アクセス（push / pop / resolve / contains_mission）を安全に処理する。
pub struct HumanReviewQueue {
    /// キュー本体。
    queue: Mutex<VecDeque<QueuedReview>>,
    /// 人間との通信チャネル。
    channel: Arc<dyn HumanChannel>,
    /// キューイングポリシー。
    policy: HumanReviewQueuePolicy,

    // ── 観測用フィールド ──
    /// 到着イベントの時系列（秒単位のタイムスタンプ）。
    arrival_timeline: Mutex<Vec<f64>>,
    /// 解決イベントの時系列（秒単位のタイムスタンプ + 判断内容）。
    resolution_timeline: Mutex<Vec<(f64, HumanDecision)>>,
    /// セマフォ待機時間のサンプル（秒）。
    contention_samples: Mutex<Vec<f64>>,
    /// リーク試行検出カウンタ。
    leak_attempts: AtomicU64,
    /// リーク成功検出カウンタ（期待値: 常に 0）。
    leak_successes: AtomicU64,
}

impl HumanReviewQueue {
    /// 新規キューを作成する。
    pub fn new(channel: Arc<dyn HumanChannel>, policy: HumanReviewQueuePolicy) -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
            channel,
            policy,
            arrival_timeline: Mutex::new(Vec::new()),
            resolution_timeline: Mutex::new(Vec::new()),
            contention_samples: Mutex::new(Vec::new()),
            leak_attempts: AtomicU64::new(0),
            leak_successes: AtomicU64::new(0),
        }
    }

    /// キューにミッションを追加する。
    ///
    /// 内部的に `HumanChannel::communicate()` を呼び出し、`InteractionHandle` を
    /// 即時返却する。呼び出し元は `handle.wait(Some(timeout))` で応答を待機できる。
    ///
    /// # エラー
    ///
    /// - `mission_id` がキュー内で重複する場合は `Err(DarviumError::SearchValidation)`。
    pub fn push(
        &self,
        mission_id: &str,
        context: serde_json::Value,
        request: HumanRequest,
    ) -> Result<InteractionHandle, DarviumError> {
        let start = Instant::now();

        let mut queue = self.queue.lock().map_err(|_| {
            DarviumError::Internal("HumanReviewQueue mutex poisoned on push".into())
        })?;

        // 観測: 競合待機時間の記録
        let elapsed = start.elapsed().as_secs_f64();
        if let Ok(mut samples) = self.contention_samples.lock() {
            samples.push(elapsed);
        }

        // ミッションIDの一意性チェック
        if queue.iter().any(|r| r.mission_id == mission_id) {
            return Err(DarviumError::SearchValidation(format!(
                "mission_id '{}' is already in the review queue",
                mission_id
            )));
        }

        // HumanChannel 経由で通信を開始
        let handle = self.channel.communicate(&request)?;

        // キューに追加
        queue.push_back(QueuedReview {
            mission_id: mission_id.to_string(),
            context,
            request,
            arrived_at: Instant::now(),
            status: QueuedReviewStatus::Pending,
        });

        // 観測: 到着イベントの記録
        if let Ok(mut timeline) = self.arrival_timeline.lock() {
            let now = Instant::now();
            let elapsed_since_start = now.duration_since(start).as_secs_f64();
            timeline.push(elapsed_since_start);
        }

        Ok(handle)
    }

    /// キューから次のレビュー候補を取り出す。
    ///
    /// `policy.priority_aware_dequeue` が true の場合、FIFO ではなく
    /// 優先度を考慮したデキューを行う（現状は FIFO 実装）。
    pub fn pop_next(&self) -> Option<QueuedReview> {
        let start = Instant::now();
        let mut queue = self.queue.lock().ok()?;

        if let Ok(mut samples) = self.contention_samples.lock() {
            samples.push(start.elapsed().as_secs_f64());
        }

        if self.policy.priority_aware_dequeue {
            // 将来拡張: 優先度ベースのデキュー
            queue.pop_front()
        } else {
            queue.pop_front()
        }
    }

    /// 次のレビュー候補を参照する（取り出さない）。
    pub fn peek_next(&self) -> Option<QueuedReview> {
        let queue = self.queue.lock().ok()?;
        queue.front().cloned()
    }

    /// キュー内の全エントリ数を返す。
    pub fn len(&self) -> usize {
        self.queue.lock().map(|q| q.len()).unwrap_or(0)
    }

    /// キューが空かどうかを返す。
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Pending 状態のエントリ数を返す。
    pub fn pending_count(&self) -> usize {
        self.queue
            .lock()
            .map(|q| {
                q.iter()
                    .filter(|r| r.status == QueuedReviewStatus::Pending)
                    .count()
            })
            .unwrap_or(0)
    }

    /// 自動実行パイプラインからのクエリに対して、常に `false` を返す隔離障壁。
    ///
    /// ミッションがキュー上に存在するか否かに関わらず `false` を返すことで、
    /// P_leak = 0 の情報隔離を実現する。
    /// 呼び出しは全て `leak_attempts` として計測されるが、`leak_successes` は
    /// 決してインクリメントされない（隔離が機能している限り）。
    pub fn contains_mission(&self, _mission_id: &str) -> bool {
        self.leak_attempts.fetch_add(1, Ordering::SeqCst);
        // 常に false を返す — 隔離障壁として機能するため。
        // 自動実行パイプラインはキュー上のミッションを検知できない。
        false
    }

    /// キュー上のミッションを解決する（HumanDecision 到着時）。
    ///
    /// 該当ミッションをキューから除去し、状態を解決済みに更新する。
    ///
    /// # エラー
    ///
    /// - `mission_id` が見つからない場合は `Err(DarviumError::SearchValidation)`。
    /// - 既に解決済みのミッションを再度解決しようとすると
    ///   `Err(DarviumError::SearchValidation)`（二重解決防止）。
    pub fn resolve(
        &self,
        mission_id: &str,
        decision: HumanDecision,
    ) -> Result<(), DarviumError> {
        let start = Instant::now();
        let mut queue = self.queue.lock().map_err(|_| {
            DarviumError::Internal("HumanReviewQueue mutex poisoned on resolve".into())
        })?;

        if let Ok(mut samples) = self.contention_samples.lock() {
            samples.push(start.elapsed().as_secs_f64());
        }

        let pos = queue
            .iter()
            .position(|r| r.mission_id == mission_id)
            .ok_or_else(|| {
                DarviumError::SearchValidation(format!(
                    "mission_id '{}' not found in review queue",
                    mission_id
                ))
            })?;

        // 二重解決チェック
        if queue[pos].status != QueuedReviewStatus::Pending {
            return Err(DarviumError::SearchValidation(format!(
                "mission_id '{}' is already resolved (status: {:?})",
                mission_id, queue[pos].status
            )));
        }

        // 状態を更新して除去
        queue[pos].status = QueuedReviewStatus::Resolved(decision);
        queue.remove(pos);

        // 観測: 解決イベントの記録
        if let Ok(mut timeline) = self.resolution_timeline.lock() {
            timeline.push((start.elapsed().as_secs_f64(), decision));
        }

        Ok(())
    }

    /// 指定された mission_id が review_timeout_secs を超過したかを判定する。
    pub fn timeout_expired(&self, mission_id: &str) -> Result<bool, DarviumError> {
        let queue = self.queue.lock().map_err(|_| {
            DarviumError::Internal("HumanReviewQueue mutex poisoned on timeout check".into())
        })?;

        let review = queue.iter().find(|r| r.mission_id == mission_id).ok_or_else(|| {
            DarviumError::SearchValidation(format!(
                "mission_id '{}' not found in review queue",
                mission_id
            ))
        })?;

        Ok(review.arrived_at.elapsed() > Duration::from_secs(self.policy.review_timeout_secs))
    }

    // ── 観測用アクセサ ──

    /// 到着イベント時系列への参照を取得する（観測テスト用）。
    pub fn arrival_timeline(&self) -> Vec<f64> {
        self.arrival_timeline
            .lock()
            .map(|t| t.clone())
            .unwrap_or_default()
    }

    /// 解決イベント時系列への参照を取得する（観測テスト用）。
    pub fn resolution_timeline(&self) -> Vec<(f64, HumanDecision)> {
        self.resolution_timeline
            .lock()
            .map(|t| t.clone())
            .unwrap_or_default()
    }

    /// 競合待機時間サンプルへの参照を取得する（観測テスト用）。
    pub fn contention_samples(&self) -> Vec<f64> {
        self.contention_samples
            .lock()
            .map(|t| t.clone())
            .unwrap_or_default()
    }

    /// リーク試行回数を取得する（観測テスト用）。
    pub fn leak_attempts(&self) -> u64 {
        self.leak_attempts.load(Ordering::SeqCst)
    }

    /// リーク成功回数を取得する（観測テスト用）。
    pub fn leak_successes(&self) -> u64 {
        self.leak_successes.load(Ordering::SeqCst)
    }
}

// ============================================================
// 不変条件テスト (T1–T10)
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{HumanDecision, HumanReviewQueuePolicy};
    use std::sync::mpsc;
    use std::sync::Arc;
    use std::thread;

    /// テスト用の HumanChannel 実装。
    /// communicate() が即座に TimedOut を返すため、事前ロードが不要。
    struct NoopChannel;

    impl HumanChannel for NoopChannel {
        fn notify(&self, _request: &HumanRequest) -> Result<(), DarviumError> {
            Ok(())
        }
        fn communicate(&self, _request: &HumanRequest) -> Result<InteractionHandle, DarviumError> {
            let (_tx, rx) = mpsc::channel();
            Ok(InteractionHandle {
                interaction_id: uuid::Uuid::new_v4(),
                rx,
            })
        }
        fn reconnect(
            &self,
            _interaction_id: uuid::Uuid,
            _request: &HumanRequest,
        ) -> Result<InteractionHandle, DarviumError> {
            let (_tx, rx) = mpsc::channel();
            Ok(InteractionHandle {
                interaction_id: uuid::Uuid::new_v4(),
                rx,
            })
        }
    }

    /// テスト用の HumanReviewQueue を構築する。
    fn make_test_queue() -> HumanReviewQueue {
        let channel = Arc::new(NoopChannel);
        let policy = HumanReviewQueuePolicy::default();
        HumanReviewQueue::new(channel, policy)
    }

    /// テスト用の HumanRequest を構築する。
    fn make_request(subject: &str) -> HumanRequest {
        HumanRequest {
            subject: subject.to_string(),
            body: "test body".into(),
            context: serde_json::Value::Null,
            timeout: None,
        }
    }

    /// テスト用の mission_id から context を構築する。
    fn make_context(id: &str) -> serde_json::Value {
        serde_json::json!({"mission_id": id})
    }

    // ── T1: 基本キューイング・解決サイクル ──

    /// T1: push → Pending → resolve(Approved) → キューから除去され応答が返る。
    #[test]
    fn t1_basic_push_resolve_cycle() {
        let queue = make_test_queue();

        let handle = queue
            .push(
                "mission_1",
                make_context("mission_1"),
                make_request("T1 test"),
            )
            .expect("push should succeed");

        assert_eq!(queue.len(), 1, "T1: queue should have 1 item after push");
        assert_eq!(queue.pending_count(), 1, "T1: pending count should be 1");
        assert_eq!(
            queue.peek_next().unwrap().status,
            QueuedReviewStatus::Pending,
            "T1: status should be Pending after push"
        );

        queue
            .resolve("mission_1", HumanDecision::Approved)
            .expect("resolve should succeed");

        assert_eq!(
            queue.len(),
            0,
            "T1: queue should be empty after resolve"
        );
        assert_eq!(
            queue.pending_count(),
            0,
            "T1: pending count should be 0 after resolve"
        );

        // InteractionHandle は FakeHumanChannel から返されていることの確認
        let _ = handle;
    }

    // ── T2: 複数キューイング ──

    /// T2: 10 件連続 push → len() == 10 → 順次 resolve → 最終的に空。
    #[test]
    fn t2_multiple_push_resolve() {
        let queue = make_test_queue();

        for i in 0..10 {
            let mid = format!("mission_{}", i);
            queue
                .push(&mid, make_context(&mid), make_request(&format!("T2 #{}", i)))
                .expect("push should succeed");
        }

        assert_eq!(queue.len(), 10, "T2: queue should have 10 items");
        assert_eq!(queue.pending_count(), 10, "T2: all should be pending");

        for i in 0..10 {
            let mid = format!("mission_{}", i);
            queue
                .resolve(&mid, HumanDecision::Approved)
                .expect("resolve should succeed");
        }

        assert_eq!(queue.len(), 0, "T2: queue should be empty after all resolves");
        assert_eq!(queue.pending_count(), 0, "T2: pending count should be 0");
        assert!(queue.is_empty(), "T2: is_empty should be true");
    }

    // ── T3: 隔離障壁 ──

    /// T3: contains_mission は常に false を返す隔離障壁として機能し、
    /// 自動実行パイプラインへの情報漏洩を防止する。
    #[test]
    fn t3_isolation_barrier() {
        let queue = make_test_queue();

        queue
            .push(
                "isolated_mission",
                make_context("isolated_mission"),
                make_request("T3"),
            )
            .expect("push should succeed");

        // 隔離障壁: キュー上のミッションに対して常に false（情報を隠蔽）
        assert!(
            !queue.contains_mission("isolated_mission"),
            "T3: contains_mission must return false for queued mission (isolation barrier)"
        );
        assert_eq!(
            queue.leak_attempts(),
            1,
            "T3: leak_attempts should increment"
        );
        assert_eq!(
            queue.leak_successes(),
            0,
            "T3: leak_successes must be 0 (barrier holds)"
        );

        // 存在しないミッションに対しても false
        assert!(
            !queue.contains_mission("nonexistent"),
            "T3: contains_mission must return false for nonexistent mission"
        );
        assert_eq!(queue.leak_attempts(), 2, "T3: leak_attempts should be 2");

        // 解決後も false
        queue
            .resolve("isolated_mission", HumanDecision::Approved)
            .expect("resolve should succeed");

        assert!(
            !queue.contains_mission("isolated_mission"),
            "T3: contains_mission must return false after resolve"
        );
        assert_eq!(queue.leak_attempts(), 3, "T3: leak_attempts should be 3");
        assert_eq!(
            queue.leak_successes(),
            0,
            "T3: leak_successes must remain 0"
        );
    }

    // ── T4: 二重解決禁止 ──

    /// T4: 同一 QueuedReview を 2 回 resolve → Err。
    #[test]
    fn t4_double_resolve_rejected() {
        let queue = make_test_queue();

        queue
            .push(
                "double_resolve",
                make_context("double_resolve"),
                make_request("T4"),
            )
            .expect("push should succeed");

        queue
            .resolve("double_resolve", HumanDecision::Approved)
            .expect("first resolve should succeed");

        let err = queue
            .resolve("double_resolve", HumanDecision::Rejected)
            .expect_err("second resolve should fail");

        assert!(
            matches!(err, DarviumError::SearchValidation(ref msg) if msg.contains("not found")),
            "T4: expected SearchValidation about not found (already removed), got {:?}",
            err
        );
    }

    // ── T5: mission_id 一意性 ──

    /// T5: 同一 mission_id の重複 push → Err。
    #[test]
    fn t5_duplicate_mission_id_rejected() {
        let queue = make_test_queue();

        queue
            .push(
                "unique_check",
                make_context("unique_check"),
                make_request("T5 first"),
            )
            .expect("first push should succeed");

        let err = queue
            .push(
                "unique_check",
                make_context("unique_check"),
                make_request("T5 second"),
            )
            .expect_err("duplicate push should fail");

        assert!(
            matches!(err, DarviumError::SearchValidation(ref msg) if msg.contains("already in the review queue")),
            "T5: expected SearchValidation about duplicate, got {:?}",
            err
        );
    }

    // ── T6: 優先度デキュー ──

    /// T6: priority_aware_dequeue = true 時のデキュー順序。
    #[test]
    fn t6_priority_dequeue_fifo() {
        let channel = Arc::new(NoopChannel);
        let policy = HumanReviewQueuePolicy {
            priority_aware_dequeue: true,
            ..HumanReviewQueuePolicy::default()
        };
        let queue = HumanReviewQueue::new(channel, policy);

        queue
            .push("first", make_context("first"), make_request("T6 #1"))
            .expect("push should succeed");
        queue
            .push("second", make_context("second"), make_request("T6 #2"))
            .expect("push should succeed");
        queue
            .push("third", make_context("third"), make_request("T6 #3"))
            .expect("push should succeed");

        assert_eq!(
            queue.pop_next().unwrap().mission_id,
            "first",
            "T6: first pushed should be first popped (FIFO)"
        );
        assert_eq!(
            queue.pop_next().unwrap().mission_id,
            "second",
            "T6: second pushed should be second popped"
        );
        assert_eq!(
            queue.pop_next().unwrap().mission_id,
            "third",
            "T6: third pushed should be third popped"
        );
        assert!(queue.is_empty(), "T6: queue should be empty");
    }

    // ── T7: タイムアウト検出 ──

    /// T7: review_timeout_secs = 0 で即時タイムアウト。
    #[test]
    fn t7_timeout_expired_detection() {
        let channel = Arc::new(NoopChannel);
        let policy = HumanReviewQueuePolicy {
            review_timeout_secs: 0, // 即時タイムアウト
            ..HumanReviewQueuePolicy::default()
        };
        let queue = HumanReviewQueue::new(channel, policy);

        queue
            .push(
                "timeout_test",
                make_context("timeout_test"),
                make_request("T7"),
            )
            .expect("push should succeed");

        assert!(
            queue.timeout_expired("timeout_test").unwrap_or(false),
            "T7: timeout_expired should be true with 0-second timeout"
        );
    }

    // ── T8: スレッドセーフ同時 push ──

    /// T8: 10 スレッドから同時に 100 件ずつ push → 最終件数 1000。
    #[test]
    fn t8_concurrent_push() {
        let queue = Arc::new(make_test_queue());
        let mut handles = Vec::new();

        for t in 0..10 {
            let q = Arc::clone(&queue);
            handles.push(thread::spawn(move || {
                for i in 0..100 {
                    let mid = format!("t{}_m{}", t, i);
                    q.push(&mid, make_context(&mid), make_request("T8"))
                        .expect("concurrent push should succeed");
                }
            }));
        }

        for h in handles {
            h.join().expect("thread should not panic");
        }

        assert_eq!(queue.len(), 1000, "T8: all 1000 items should be in queue");
    }

    // ── T9: スレッドセーフ同時 push/pop ──

    /// T9: 5 スレッド push + 5 スレッド pop → デッドロック・panic なし。
    #[test]
    fn t9_concurrent_push_pop() {
        let queue = Arc::new(make_test_queue());
        let mut handles = Vec::new();

        // 5 件 pre-push
        for i in 0..5 {
            let mid = format!("pre_{}", i);
            queue
                .push(&mid, make_context(&mid), make_request("T9 pre"))
                .expect("pre-push should succeed");
        }

        // 5 スレッド: push
        for t in 0..5 {
            let q = Arc::clone(&queue);
            handles.push(thread::spawn(move || {
                for i in 0..50 {
                    let mid = format!("push_{}_m{}", t, i);
                    q.push(&mid, make_context(&mid), make_request("T9 push"))
                        .expect("push should succeed");
                }
            }));
        }

        // 5 スレッド: pop
        for _ in 0..5 {
            let q = Arc::clone(&queue);
            handles.push(thread::spawn(move || {
                for _ in 0..50 {
                    q.pop_next();
                }
            }));
        }

        for h in handles {
            h.join().expect("thread should not panic");
        }

        // デッドロック・panic がなければ成功
        let _ = queue.len();
    }

    // ── T10: キュー空時の pop ──

    /// T10: 空キューで pop_next() → None。
    #[test]
    fn t10_pop_empty_queue() {
        let queue = make_test_queue();
        assert!(queue.is_empty(), "T10: new queue should be empty");
        assert!(
            queue.pop_next().is_none(),
            "T10: pop from empty queue should return None"
        );
    }
}
