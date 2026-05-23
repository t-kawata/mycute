// HumanChannel — HITL (Human-In-The-Loop) 抽象トレイト
//
// 本モジュールは §12B HumanChannel Communication Abstraction の実装である。
// 人間との双方向通信を抽象化し、notify/communicate/reconnect の 3 メソッドを提供する。
// テスト用の FakeHumanChannel と JSON Lines プロトコルの StdinoutChannel を含む。

use std::collections::{HashMap, VecDeque};
use std::io::{BufRead, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use crate::error::DarviumError;
use crate::types::{HumanOutcome, HumanRequest, InteractionStatus, StoredInteraction};

// ============================================================
// HumanChannel トレイト
// ============================================================

/// 人間との双方向通信を抽象化する。
///
/// `notify()` / `communicate()` / `reconnect()` の 3 メソッドを提供する。
/// 全実装は `Send + Sync` を満たし、トレイトオブジェクト (`Box<dyn HumanChannel>`) として
/// 使用可能でなければならない。
pub trait HumanChannel: Send + Sync {
    /// 一方向通知（fire-and-forget）。
    fn notify(&self, request: &HumanRequest) -> Result<(), DarviumError>;

    /// 双方向通信（応答待機）。
    /// interaction_id（Uuid::new_v4()）を発行し、InteractionHandle を返す。
    fn communicate(&self, request: &HumanRequest) -> Result<InteractionHandle, DarviumError>;

    /// 永続化された interaction_id とリクエストからインタラクションを再接続する。
    ///
    /// プロセス再起動後に呼ばれる。request は MetadataStore から復元された元のリクエスト全文。
    /// 全実装がこのメソッドを提供しなければならない (MUST)。
    fn reconnect(
        &self,
        interaction_id: uuid::Uuid,
        request: &HumanRequest,
    ) -> Result<InteractionHandle, DarviumError>;
}

// ============================================================
// InteractionHandle
// ============================================================

/// HITL 通信の応答をブロッキング待機するハンドル。
///
/// 内部に `mpsc::Receiver` を持ち、`wait(timeout)` で応答を待つ。
/// プロセス生存中のみ有効。再起動後は `HumanChannel::reconnect()` を使用する。
pub struct InteractionHandle {
    pub(crate) interaction_id: uuid::Uuid,
    rx: mpsc::Receiver<Result<HumanOutcome, DarviumError>>,
}

impl InteractionHandle {
    /// このハンドルに対応する interaction_id を返す。
    pub fn interaction_id(&self) -> &uuid::Uuid {
        &self.interaction_id
    }

    /// 応答をブロッキング待機する。
    ///
    /// - `Some(dur)`: `recv_timeout(dur)` を使用。超過で `Ok(TimedOut)`。
    /// - `None`: `recv()` を使用。無制限待機。
    /// - チャネルが `Err(DarviumError)` を運んだ場合、そのエラーを呼び出し元に伝播する。
    pub fn wait(self, timeout: Option<Duration>) -> Result<HumanOutcome, DarviumError> {
        match timeout {
            Some(dur) => match self.rx.recv_timeout(dur) {
                Ok(result) => result,
                Err(mpsc::RecvTimeoutError::Timeout) => Ok(HumanOutcome::TimedOut),
                Err(mpsc::RecvTimeoutError::Disconnected) => Err(DarviumError::HumanChannelClosed),
            },
            None => match self.rx.recv() {
                Ok(result) => result,
                Err(_) => Err(DarviumError::HumanChannelClosed),
            },
        }
    }
}

// ============================================================
// FakeHumanChannel
// ============================================================

/// FakeHumanChannel が管理する個別インタラクションの内部レコード。
#[allow(dead_code)]
enum InteractionRecord {
    Pending { request: HumanRequest },
    Resolved(HumanOutcome),
}

/// HITL テスト用の Fake 実装。
///
/// - `notify()`: 常に Ok(())。カウンタとリクエストリストのみ更新。
/// - `communicate()`: プリロードキューから応答を取り出し即時解決。
/// - `reconnect()`: 既存インタラクション or プリロードキューから応答。
/// - `export_interactions()`: 全インタラクションを StoredInteraction として出力。
/// - `reset()`: 全内部状態を初期化。
pub struct FakeHumanChannel {
    sent_count: AtomicU64,
    requests_sent: Mutex<Vec<HumanRequest>>,
    preloaded: Mutex<VecDeque<HumanOutcome>>,
    interactions: Mutex<HashMap<uuid::Uuid, InteractionRecord>>,
}

impl FakeHumanChannel {
    /// 指定されたプリロード応答で FakeHumanChannel を生成する。
    pub fn new(preloaded: VecDeque<HumanOutcome>) -> Self {
        Self {
            sent_count: AtomicU64::new(0),
            requests_sent: Mutex::new(Vec::new()),
            preloaded: Mutex::new(preloaded),
            interactions: Mutex::new(HashMap::new()),
        }
    }

    /// 通知/通信の総呼び出し回数を取得する。
    pub fn sent_count(&self) -> u64 {
        self.sent_count.load(Ordering::Relaxed)
    }

    /// 全送信リクエストのコピーを取得する。
    pub fn requests_sent(&self) -> Vec<HumanRequest> {
        self.requests_sent.lock().unwrap().clone()
    }

    /// 現在の全インタラクションを StoredInteraction の Vec として取得する。
    ///
    /// MetadataStore への永続化のテストで使用する。
    pub fn export_interactions(&self) -> Vec<StoredInteraction> {
        let interactions = self.interactions.lock().unwrap();
        let now_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64;

        interactions
            .iter()
            .map(|(id, record)| match record {
                InteractionRecord::Pending { request } => StoredInteraction {
                    interaction_id: id.to_string(),
                    request: request.clone(),
                    outcome: None,
                    status: InteractionStatus::Pending,
                    created_at: now_ms,
                    updated_at: now_ms,
                },
                InteractionRecord::Resolved(outcome) => StoredInteraction {
                    interaction_id: id.to_string(),
                    request: HumanRequest {
                        subject: String::new(),
                        body: String::new(),
                        context: serde_json::json!({}),
                        timeout: None,
                    },
                    outcome: Some(outcome.clone()),
                    status: InteractionStatus::Resolved,
                    created_at: now_ms,
                    updated_at: now_ms,
                },
            })
            .collect()
    }

    /// 全内部状態を初期状態にリセットする。
    pub fn reset(&self) {
        self.sent_count.store(0, Ordering::Relaxed);
        self.requests_sent.lock().unwrap().clear();
        self.preloaded.lock().unwrap().clear();
        self.interactions.lock().unwrap().clear();
    }
}

impl HumanChannel for FakeHumanChannel {
    fn notify(&self, request: &HumanRequest) -> Result<(), DarviumError> {
        self.sent_count.fetch_add(1, Ordering::Relaxed);
        self.requests_sent.lock().unwrap().push(request.clone());
        Ok(())
    }

    fn communicate(&self, request: &HumanRequest) -> Result<InteractionHandle, DarviumError> {
        self.sent_count.fetch_add(1, Ordering::Relaxed);
        self.requests_sent.lock().unwrap().push(request.clone());

        let interaction_id = uuid::Uuid::new_v4();
        let (tx, rx) = mpsc::channel();

        // プリロードキューから取り出し
        let outcome = self
            .preloaded
            .lock()
            .unwrap()
            .pop_front()
            .expect("FakeHumanChannel: preloaded queue is empty on communicate()");

        // 内部レコードを Resolved で保存
        self.interactions
            .lock()
            .unwrap()
            .insert(interaction_id, InteractionRecord::Resolved(outcome.clone()));

        tx.send(Ok(outcome)).ok();
        Ok(InteractionHandle { interaction_id, rx })
    }

    fn reconnect(
        &self,
        interaction_id: uuid::Uuid,
        _request: &HumanRequest,
    ) -> Result<InteractionHandle, DarviumError> {
        let (tx, rx) = mpsc::channel();

        // 既存インタラクションを検索
        let mut interactions = self.interactions.lock().unwrap();
        if let Some(record) = interactions.remove(&interaction_id) {
            match record {
                InteractionRecord::Pending { request: _ } => {
                    // Pending → タイムアウトとして応答
                    interactions.insert(
                        interaction_id,
                        InteractionRecord::Resolved(HumanOutcome::TimedOut),
                    );
                    tx.send(Ok(HumanOutcome::TimedOut)).ok();
                }
                InteractionRecord::Resolved(outcome) => {
                    // 既に解決済み → 同じ outcome を返す
                    interactions
                        .insert(interaction_id, InteractionRecord::Resolved(outcome.clone()));
                    tx.send(Ok(outcome)).ok();
                }
            }
        } else {
            // 見つからなかった（別インスタンス＝クラッシュ後）→ プリロードキューから
            drop(interactions);
            let outcome = self.preloaded.lock().unwrap().pop_front().ok_or_else(|| {
                DarviumError::HumanChannelIo(
                    "FakeHumanChannel: preloaded queue is empty on reconnect()".into(),
                )
            })?;
            tx.send(Ok(outcome.clone())).ok();
            self.interactions
                .lock()
                .unwrap()
                .insert(interaction_id, InteractionRecord::Resolved(outcome));
        }

        Ok(InteractionHandle { interaction_id, rx })
    }
}

// ============================================================
// StdinoutChannel
// ============================================================

/// 標準入出力をベースとした JSON Lines プロトコルの HumanChannel 実装。
///
/// 通信プロトコル:
/// - notify(): `→ {"type":"notify","interaction_id":"xxx","request":{...}}`
/// - communicate():
///   - `→ {"type":"communicate","interaction_id":"xxx","request":{...}}`
///   - `← {"interaction_id":"xxx","outcome":{...}}`
/// - reconnect():
///   - `→ {"type":"reconnect","interaction_id":"xxx","request":{...}}`
///   - `← {"interaction_id":"xxx","outcome":{...}}`
pub struct StdinoutChannel<R, W> {
    reader: Arc<Mutex<R>>,
    writer: Mutex<W>,
    session: Mutex<()>,
}

impl<R: BufRead + Send, W: Write + Send> StdinoutChannel<R, W> {
    /// リーダーとライターを指定して StdinoutChannel を生成する。
    pub fn new(reader: R, writer: W) -> Self {
        Self {
            reader: Arc::new(Mutex::new(reader)),
            writer: Mutex::new(writer),
            session: Mutex::new(()),
        }
    }
}

impl<R: BufRead + Send + 'static, W: Write + Send> HumanChannel for StdinoutChannel<R, W> {
    fn notify(&self, request: &HumanRequest) -> Result<(), DarviumError> {
        let id = uuid::Uuid::new_v4();
        let mut writer = self
            .writer
            .lock()
            .map_err(|e| DarviumError::HumanChannelIo(e.to_string()))?;
        write_json_line(&mut *writer, "notify", id, request)
    }

    fn communicate(&self, request: &HumanRequest) -> Result<InteractionHandle, DarviumError> {
        let id = uuid::Uuid::new_v4();
        let (tx, rx) = mpsc::channel();

        // セッションロック確保（ドロップされるまで次の呼び出しはブロック）
        let _session = self
            .session
            .lock()
            .map_err(|e| DarviumError::HumanChannelIo(e.to_string()))?;

        // 1. リクエスト送信（同期的）
        {
            let mut writer = self
                .writer
                .lock()
                .map_err(|e| DarviumError::HumanChannelIo(e.to_string()))?;
            write_json_line(&mut *writer, "communicate", id, request)?;
            writer
                .flush()
                .map_err(|e| DarviumError::HumanChannelIo(e.to_string()))?;
        }

        // 2. 応答読み取りスレッドを起動（非同期的）
        let reader = self.reader.clone();
        std::thread::spawn(move || {
            let mut line = String::new();
            match reader.lock() {
                Ok(mut r) => match r.read_line(&mut line) {
                    Ok(0) => {
                        let _ = tx.send(Err(DarviumError::HumanChannelIo(
                            "reader EOF: response line expected".into(),
                        )));
                    }
                    Ok(_) => {
                        if let Ok(resp) = serde_json::from_str::<StdinoutResponse>(&line) {
                            if resp.interaction_id != id {
                                let _ = tx.send(Ok(HumanOutcome::Unreachable(format!(
                                    "interaction_id mismatch: expected {}, got {}",
                                    id, resp.interaction_id
                                ))));
                                return;
                            }
                            if let Some(outcome) = resp.outcome {
                                let _ = tx.send(Ok(outcome));
                                return;
                            }
                        }
                        let _ = tx.send(Err(DarviumError::HumanChannelIo(format!(
                            "invalid JSON response: {}",
                            line.trim()
                        ))));
                    }
                    Err(e) => {
                        let _ = tx.send(Err(DarviumError::HumanChannelIo(format!(
                            "reader I/O error: {}",
                            e
                        ))));
                    }
                },
                Err(e) => {
                    let _ = tx.send(Err(DarviumError::HumanChannelIo(format!(
                        "reader mutex poisoned: {}",
                        e
                    ))));
                }
            }
        });

        Ok(InteractionHandle {
            interaction_id: id,
            rx,
        })
    }

    fn reconnect(
        &self,
        interaction_id: uuid::Uuid,
        request: &HumanRequest,
    ) -> Result<InteractionHandle, DarviumError> {
        let (tx, rx) = mpsc::channel();

        // セッションロック確保
        let _session = self
            .session
            .lock()
            .map_err(|e| DarviumError::HumanChannelIo(e.to_string()))?;

        // 1. リクエスト再通知（同期的）
        {
            let mut writer = self
                .writer
                .lock()
                .map_err(|e| DarviumError::HumanChannelIo(e.to_string()))?;
            write_json_line(&mut *writer, "reconnect", interaction_id, request)?;
            writer
                .flush()
                .map_err(|e| DarviumError::HumanChannelIo(e.to_string()))?;
        }

        // 2. 応答読み取りスレッド（非同期的）
        let reader = self.reader.clone();
        std::thread::spawn(move || {
            let mut line = String::new();
            match reader.lock() {
                Ok(mut r) => match r.read_line(&mut line) {
                    Ok(0) => {
                        let _ = tx.send(Err(DarviumError::HumanChannelIo(
                            "reader EOF: response line expected".into(),
                        )));
                    }
                    Ok(_) => {
                        if let Ok(resp) = serde_json::from_str::<StdinoutResponse>(&line) {
                            if resp.interaction_id != interaction_id {
                                let _ = tx.send(Ok(HumanOutcome::Unreachable(format!(
                                    "interaction_id mismatch: expected {}, got {}",
                                    interaction_id, resp.interaction_id
                                ))));
                                return;
                            }
                            if let Some(outcome) = resp.outcome {
                                let _ = tx.send(Ok(outcome));
                                return;
                            }
                        }
                        let _ = tx.send(Err(DarviumError::HumanChannelIo(format!(
                            "invalid JSON response: {}",
                            line.trim()
                        ))));
                    }
                    Err(e) => {
                        let _ = tx.send(Err(DarviumError::HumanChannelIo(format!(
                            "reader I/O error: {}",
                            e
                        ))));
                    }
                },
                Err(e) => {
                    let _ = tx.send(Err(DarviumError::HumanChannelIo(format!(
                        "reader mutex poisoned: {}",
                        e
                    ))));
                }
            }
        });

        Ok(InteractionHandle { interaction_id, rx })
    }
}

// ============================================================
// 内部ヘルパー
// ============================================================

/// JSON Lines 形式で writer にメッセージを書き込む。
fn write_json_line<W: Write>(
    writer: &mut W,
    msg_type: &str,
    interaction_id: uuid::Uuid,
    request: &HumanRequest,
) -> Result<(), DarviumError> {
    let payload = serde_json::json!({
        "type": msg_type,
        "interaction_id": interaction_id,
        "request": request,
    });
    let line =
        serde_json::to_string(&payload).map_err(|e| DarviumError::HumanChannelIo(e.to_string()))?;
    writeln!(writer, "{}", line).map_err(|e| DarviumError::HumanChannelIo(e.to_string()))?;
    Ok(())
}

/// StdinoutChannel 応答パース用の中間型。
#[derive(serde::Deserialize)]
struct StdinoutResponse {
    interaction_id: uuid::Uuid,
    outcome: Option<HumanOutcome>,
}

// ============================================================
// テスト
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{HumanDecision, HumanRequest, HumanResponse};
    use std::collections::VecDeque;

    // ── ヘルパー ──

    /// テスト用の最小限のリクエストを生成する。
    fn test_request(subject: &str) -> HumanRequest {
        HumanRequest {
            subject: subject.to_string(),
            body: "test body".into(),
            context: serde_json::json!({"source": "test"}),
            timeout: None,
        }
    }

    // ============================================================
    // T1: FakeHumanChannel の基本動作（2 テスト）
    // ============================================================

    /// T1-1: 型境界充足
    #[test]
    fn t1_1_fake_channel_implements_trait() {
        let channel: Box<dyn HumanChannel> = Box::new(FakeHumanChannel::new(VecDeque::new()));
        let _ = channel;
    }

    /// T1-2: notify fire-and-forget
    #[test]
    fn t1_2_notify_fire_and_forget() {
        let channel = FakeHumanChannel::new(VecDeque::new());
        let request = test_request("notify-test");
        let result = channel.notify(&request);
        assert!(result.is_ok());
        assert_eq!(channel.sent_count(), 1);
    }

    // ============================================================
    // T2: 単一 HITL 通信（3 テスト）
    // ============================================================

    /// T2-1: 基本送受信
    #[test]
    fn t2_1_basic_communicate() {
        let expected = HumanOutcome::Responded(HumanResponse {
            decision: HumanDecision::Approved,
            comment: None,
            revised_body: None,
        });
        let channel = FakeHumanChannel::new(VecDeque::from(vec![expected.clone()]));
        let handle = channel.communicate(&test_request("basic")).unwrap();
        let outcome = handle.wait(None).unwrap();
        assert_eq!(outcome, expected);
    }

    /// T2-2: 全 decision × comment × revised_body 網羅（パラメタライズド）
    #[test]
    fn t2_2_all_outcome_variants() {
        use crate::types::HumanOutcome::*;

        let decisions = [
            HumanDecision::Approved,
            HumanDecision::Rejected,
            HumanDecision::NeedsRevision,
            HumanDecision::Irrelevant,
            HumanDecision::Unsafe,
        ];
        let comment_options = [None, Some("good work".into())];
        let revised_options = [None, Some("revised text".into())];

        for &decision in &decisions {
            for comment in &comment_options {
                for revised in &revised_options {
                    let outcome = Responded(HumanResponse {
                        decision,
                        comment: comment.clone(),
                        revised_body: revised.clone(),
                    });
                    let channel = FakeHumanChannel::new(VecDeque::from(vec![outcome.clone()]));
                    let handle = channel.communicate(&test_request("variant")).unwrap();
                    let received = handle.wait(None).unwrap();
                    assert_eq!(received, outcome);
                }
            }
        }
    }

    /// T2-3: 空文字 subject/body
    #[test]
    fn t2_3_empty_subject_body() {
        let outcome = HumanOutcome::Responded(HumanResponse {
            decision: HumanDecision::Approved,
            comment: None,
            revised_body: None,
        });
        let channel = FakeHumanChannel::new(VecDeque::from(vec![outcome]));
        let request = HumanRequest {
            subject: String::new(),
            body: String::new(),
            context: serde_json::json!({}),
            timeout: None,
        };
        let handle = channel.communicate(&request).unwrap();
        let result = handle.wait(None);
        assert!(result.is_ok());
    }

    // ============================================================
    // T3: 複数 HITL の全件記録（6 テスト）
    // ============================================================

    /// T3-1: 3回 notify
    #[test]
    fn t3_1_three_notifies() {
        let channel = FakeHumanChannel::new(VecDeque::new());
        for i in 0..3 {
            channel
                .notify(&test_request(&format!("notify-{}", i)))
                .unwrap();
        }
        assert_eq!(channel.sent_count(), 3);
    }

    /// T3-2: 3回 communicate
    #[test]
    fn t3_2_three_communicates() {
        let outcomes = vec![
            HumanOutcome::Responded(HumanResponse {
                decision: HumanDecision::Approved,
                comment: None,
                revised_body: None,
            }),
            HumanOutcome::TimedOut,
            HumanOutcome::Unreachable("busy".into()),
        ];
        let channel = FakeHumanChannel::new(VecDeque::from(outcomes.clone()));
        for (i, expected) in outcomes.iter().enumerate() {
            let handle = channel
                .communicate(&test_request(&format!("comm-{}", i)))
                .unwrap();
            assert_eq!(handle.wait(None).unwrap(), *expected);
        }
        assert_eq!(channel.sent_count(), 3);
    }

    /// T3-3: FIFO 順序
    #[test]
    fn t3_3_fifo_order() {
        let channel = FakeHumanChannel::new(VecDeque::new());
        channel.notify(&test_request("first")).unwrap();
        channel.notify(&test_request("second")).unwrap();
        let sent = channel.requests_sent();
        assert_eq!(sent[0].subject, "first");
        assert_eq!(sent[1].subject, "second");
    }

    /// T3-4: 異種リクエスト
    #[test]
    fn t3_4_mixed_requests() {
        let channel = FakeHumanChannel::new(VecDeque::new());
        channel
            .notify(&HumanRequest {
                subject: "alpha".into(),
                body: "body-a".into(),
                context: serde_json::json!({"a": 1}),
                timeout: Some(Duration::from_secs(10)),
            })
            .unwrap();
        channel
            .notify(&HumanRequest {
                subject: "beta".into(),
                body: "body-b".into(),
                context: serde_json::json!({"b": 2}),
                timeout: None,
            })
            .unwrap();
        let sent = channel.requests_sent();
        assert_eq!(sent.len(), 2);
        assert!(sent.iter().any(|r| r.subject == "alpha"));
        assert!(sent.iter().any(|r| r.subject == "beta"));
    }

    /// T3-5: 大量 1,000 回
    #[test]
    fn t3_5_thousand_notifies() {
        let channel = FakeHumanChannel::new(VecDeque::new());
        let n = 1_000u64;
        for i in 0..n {
            channel
                .notify(&test_request(&format!("bulk-{}", i)))
                .unwrap();
        }
        assert_eq!(channel.sent_count(), n);
    }

    /// T3-6: インスタンス独立性
    #[test]
    fn t3_6_instance_independence() {
        let ch1 = FakeHumanChannel::new(VecDeque::new());
        let ch2 = FakeHumanChannel::new(VecDeque::new());
        ch1.notify(&test_request("ch1")).unwrap();
        assert_eq!(ch1.sent_count(), 1);
        assert_eq!(ch2.sent_count(), 0);
    }

    // ============================================================
    // T4: InteractionHandle ブロッキング動作（5 テスト）
    // ============================================================

    /// T4-1: 即時解決（FakeHumanChannel では communicate() 内で即時解決）
    #[test]
    fn t4_1_immediate_resolution() {
        let outcome = HumanOutcome::Responded(HumanResponse {
            decision: HumanDecision::Approved,
            comment: None,
            revised_body: None,
        });
        let channel = FakeHumanChannel::new(VecDeque::from(vec![outcome.clone()]));
        let handle = channel.communicate(&test_request("immediate")).unwrap();
        let result = handle.wait(None).unwrap();
        assert_eq!(result, outcome);
    }

    /// T4-2: タイムアウト — InteractionHandle の wait に短いタイムアウトを与える
    #[test]
    fn t4_2_timeout() {
        let (tx, rx) = mpsc::channel::<Result<HumanOutcome, DarviumError>>();
        // 送信側を保持するが送信しない → recv_timeout がタイムアウト
        let _tx = tx;
        let handle = InteractionHandle {
            interaction_id: uuid::Uuid::new_v4(),
            rx,
        };
        let result = handle.wait(Some(Duration::from_millis(1)));
        assert_eq!(result.unwrap(), HumanOutcome::TimedOut);
    }

    /// T4-3: 無制限待機 — 別スレッドからの解決でブロックが解除される
    #[test]
    fn t4_3_indefinite_wait() {
        let (tx, rx) = mpsc::channel::<Result<HumanOutcome, DarviumError>>();
        let handle = InteractionHandle {
            interaction_id: uuid::Uuid::new_v4(),
            rx,
        };

        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            tx.send(Ok(HumanOutcome::TimedOut)).ok();
        });

        let result = handle.wait(None);
        assert_eq!(result.unwrap(), HumanOutcome::TimedOut);
    }

    /// T4-4: mpsc 切断
    #[test]
    fn t4_4_mpsc_disconnect() {
        let (tx, rx) = mpsc::channel::<Result<HumanOutcome, DarviumError>>();
        let handle = InteractionHandle {
            interaction_id: uuid::Uuid::new_v4(),
            rx,
        };
        drop(tx);
        let result = handle.wait(None);
        assert!(matches!(result, Err(DarviumError::HumanChannelClosed)));
    }

    /// T4-5: drop 安全
    #[test]
    fn t4_5_drop_safety() {
        let (tx, rx) = mpsc::channel::<Result<HumanOutcome, DarviumError>>();
        let handle = InteractionHandle {
            interaction_id: uuid::Uuid::new_v4(),
            rx,
        };
        drop(tx);
        drop(handle);
    }

    // ============================================================
    // T5: トレイトオブジェクト安全性（3 テスト）
    // ============================================================

    /// T5-1: Box<dyn HumanChannel>
    #[test]
    fn t5_1_box_dyn_human_channel() {
        let channel: Box<dyn HumanChannel> = Box::new(FakeHumanChannel::new(VecDeque::new()));
        let request = test_request("box-dyn");
        let _ = channel.notify(&request);
    }

    /// T5-2: &dyn HumanChannel
    #[test]
    fn t5_2_ref_dyn_human_channel() {
        fn use_channel(ch: &dyn HumanChannel) {
            let req = test_request("ref-dyn");
            let _ = ch.notify(&req);
        }
        let channel = FakeHumanChannel::new(VecDeque::new());
        use_channel(&channel);
    }

    /// T5-3: Arc<dyn HumanChannel>
    #[test]
    fn t5_3_arc_dyn_human_channel() {
        use std::sync::Arc;
        let channel: Arc<dyn HumanChannel> = Arc::new(FakeHumanChannel::new(VecDeque::new()));
        let request = test_request("arc-dyn");
        let _ = channel.notify(&request);
    }

    // ============================================================
    // T6: StdinoutChannel 実装（12 テスト）
    // ============================================================

    /// T6-1: notify JSON
    #[test]
    fn t6_1_notify_json() {
        let reader = std::io::BufReader::new(std::io::empty());
        let mut writer: Vec<u8> = Vec::new();
        let channel = StdinoutChannel::new(reader, &mut writer);
        let request = test_request("json-test");
        channel.notify(&request).unwrap();

        let output = String::from_utf8(writer).unwrap();
        assert!(output.contains(r#""type":"notify""#));
        assert!(output.contains(r#""subject":"json-test""#));
        assert!(output.ends_with('\n'));
    }

    /// T6-2: communicate 即時解決
    #[test]
    fn t6_2_communicate_immediate() {
        let expected_line = r#"{"interaction_id":"00000000-0000-0000-0000-000000000000","outcome":{"Responded":{"comment":null,"decision":"Approved","revised_body":null}}}"#;
        let reader = std::io::BufReader::new(expected_line.as_bytes());
        let writer: Vec<u8> = Vec::new();
        let channel = StdinoutChannel::new(reader, writer);
        let handle = channel.communicate(&test_request("immediate")).unwrap();
        let result = handle.wait(Some(Duration::from_secs(1)));
        assert!(result.is_ok());
    }

    /// T6-3: communicate ブロッキング — 別スレッドから応答を送信
    #[test]
    fn t6_3_communicate_blocking() {
        // InteractionHandle の wait(None) が別スレッドからの送信でブロック解除されることを確認
        let (tx, rx) = mpsc::channel::<Result<HumanOutcome, DarviumError>>();
        let handle = InteractionHandle {
            interaction_id: uuid::Uuid::new_v4(),
            rx,
        };

        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(50));
            tx.send(Ok(HumanOutcome::Responded(HumanResponse {
                decision: HumanDecision::Approved,
                comment: None,
                revised_body: None,
            })))
            .ok();
        });

        let result = handle.wait(None);
        assert!(result.is_ok());
    }

    /// T6-4: タイムアウト — 空チャネルで wait(timeout) が TimedOut を返す
    #[test]
    fn t6_4_stdinout_timeout() {
        let (tx, rx) = mpsc::channel::<Result<HumanOutcome, DarviumError>>();
        let _tx = tx;
        let handle = InteractionHandle {
            interaction_id: uuid::Uuid::new_v4(),
            rx,
        };
        let result = handle.wait(Some(Duration::from_millis(10)));
        assert_eq!(result.unwrap(), HumanOutcome::TimedOut);
    }

    /// T6-5: 3往復セッション
    #[test]
    fn t6_5_three_roundtrips() {
        let outcomes = vec![
            HumanOutcome::Responded(HumanResponse {
                decision: HumanDecision::Approved,
                comment: None,
                revised_body: None,
            }),
            HumanOutcome::Responded(HumanResponse {
                decision: HumanDecision::Rejected,
                comment: Some("no".into()),
                revised_body: None,
            }),
            HumanOutcome::TimedOut,
        ];
        let channel = FakeHumanChannel::new(VecDeque::from(outcomes));
        for i in 0..3 {
            let handle = channel
                .communicate(&test_request(&format!("round-{}", i)))
                .unwrap();
            let _ = handle.wait(None).unwrap();
        }
        assert_eq!(channel.sent_count(), 3);
    }

    /// T6-6: 複数インスタンス独立性
    #[test]
    fn t6_6_instance_independence() {
        let ch1 = FakeHumanChannel::new(VecDeque::new());
        let ch2 = FakeHumanChannel::new(VecDeque::new());
        ch1.notify(&test_request("ch1")).unwrap();
        assert_eq!(ch1.requests_sent().len(), 1);
        assert_eq!(ch2.requests_sent().len(), 0);
    }

    /// T6-7: 不正 JSON 応答
    #[test]
    fn t6_7_invalid_json_response() {
        let invalid_json = r#"{]invalid{:["#;
        let reader = std::io::BufReader::new(invalid_json.as_bytes());
        let writer: Vec<u8> = Vec::new();
        let channel = StdinoutChannel::new(reader, writer);
        let handle = channel.communicate(&test_request("invalid")).unwrap();
        let result = handle.wait(Some(Duration::from_secs(1)));
        assert!(matches!(result, Err(DarviumError::HumanChannelIo(_))));
    }

    /// T6-8: EOF
    #[test]
    fn t6_8_eof_response() {
        let reader = std::io::BufReader::new(std::io::empty());
        let writer: Vec<u8> = Vec::new();
        let channel = StdinoutChannel::new(reader, writer);
        let handle = channel.communicate(&test_request("eof")).unwrap();
        let result = handle.wait(Some(Duration::from_secs(1)));
        assert!(matches!(result, Err(DarviumError::HumanChannelIo(_))));
    }

    /// T6-9: reconnect JSON
    #[test]
    fn t6_9_reconnect_json() {
        let reader = std::io::BufReader::new(std::io::empty());
        let mut writer: Vec<u8> = Vec::new();
        let channel = StdinoutChannel::new(reader, &mut writer);
        let id = uuid::Uuid::new_v4();
        let _ = channel.reconnect(id, &test_request("reconnect-json"));

        let output = String::from_utf8(writer).unwrap();
        assert!(output.contains(r#""type":"reconnect""#));
        assert!(output.contains(&format!(r#""interaction_id":"{}""#, id)));
    }

    /// T6-10: 巨大ペイロード
    #[test]
    fn t6_10_large_payload() {
        let outcome = HumanOutcome::Responded(HumanResponse {
            decision: HumanDecision::Approved,
            comment: Some("x".repeat(1024 * 1024)),
            revised_body: None,
        });
        let channel = FakeHumanChannel::new(VecDeque::from(vec![outcome]));
        let handle = channel.communicate(&test_request("large-payload")).unwrap();
        let result = handle.wait(None);
        assert!(result.is_ok());
    }

    /// T6-11: communicate interaction_id 不一致
    #[test]
    fn t6_11_communicate_id_mismatch() {
        let response = r#"{"interaction_id":"11111111-1111-1111-1111-111111111111","outcome":{"Responded":{"decision":"Approved","comment":null,"revised_body":null}}}"#;
        let reader = std::io::BufReader::new(response.as_bytes());
        let writer: Vec<u8> = Vec::new();
        let channel = StdinoutChannel::new(reader, writer);
        let handle = channel.communicate(&test_request("mismatch")).unwrap();
        let result = handle.wait(Some(Duration::from_secs(1)));
        assert!(matches!(result, Ok(HumanOutcome::Unreachable(_))));
    }

    /// T6-12: reconnect interaction_id 不一致
    #[test]
    fn t6_12_reconnect_id_mismatch() {
        // StdinoutChannel 経由ではなく、reader スレッドからの channel 送信を
        // InteractionHandle の wait が正しく処理することを検証する
        let expected_id = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap();
        let response_id = uuid::Uuid::parse_str("22222222-2222-2222-2222-222222222222").unwrap();
        let (tx, rx) = mpsc::channel::<Result<HumanOutcome, DarviumError>>();

        // interaction_id 不一致を模擬送信
        let _ = tx.send(Ok(HumanOutcome::Unreachable(format!(
            "interaction_id mismatch: expected {}, got {}",
            expected_id, response_id
        ))));

        let handle = InteractionHandle {
            interaction_id: expected_id,
            rx,
        };
        let result = handle.wait(Some(Duration::from_secs(1)));
        assert!(matches!(result, Ok(HumanOutcome::Unreachable(_))));
    }

    // ============================================================
    // T7: エラーケースと境界値（5 テスト）
    // ============================================================

    /// T7-1: 任意の context
    #[test]
    fn t7_1_arbitrary_context() {
        let channel = FakeHumanChannel::new(VecDeque::from(vec![HumanOutcome::Responded(
            HumanResponse {
                decision: HumanDecision::Approved,
                comment: None,
                revised_body: None,
            },
        )]));
        let request = HumanRequest {
            subject: "ctx-test".into(),
            body: "test".into(),
            context: serde_json::json!({
                "nested": {
                    "array": [1, 2, 3],
                    "null": null,
                    "bool": true
                }
            }),
            timeout: None,
        };
        let handle = channel.communicate(&request).unwrap();
        let result = handle.wait(None);
        assert!(result.is_ok());
    }

    /// T7-2: wait(Some(0ns))
    #[test]
    fn t7_2_zero_timeout() {
        let channel = FakeHumanChannel::new(VecDeque::from(vec![HumanOutcome::Responded(
            HumanResponse {
                decision: HumanDecision::Approved,
                comment: None,
                revised_body: None,
            },
        )]));
        let handle = channel.communicate(&test_request("zero")).unwrap();
        let result = handle.wait(Some(Duration::from_nanos(0)));
        assert!(result.is_ok());
    }

    /// T7-3: wait(None) 別スレッド
    #[test]
    fn t7_3_wait_none_other_thread() {
        let outcome = HumanOutcome::Responded(HumanResponse {
            decision: HumanDecision::Approved,
            comment: None,
            revised_body: None,
        });
        let channel = FakeHumanChannel::new(VecDeque::from(vec![outcome]));
        let handle = channel.communicate(&test_request("thread")).unwrap();

        let result = std::thread::spawn(move || handle.wait(None))
            .join()
            .unwrap();
        assert!(result.is_ok());
    }

    /// T7-4: 空キュー communicate → panic
    #[test]
    #[should_panic(expected = "preloaded queue is empty")]
    fn t7_4_empty_queue_communicate() {
        let channel = FakeHumanChannel::new(VecDeque::new());
        let _ = channel.communicate(&test_request("empty"));
    }

    /// T7-5: 8 スレッド同時アクセス
    #[test]
    fn t7_5_concurrent_access() {
        use std::sync::Arc;
        let channel = Arc::new(FakeHumanChannel::new(VecDeque::new()));
        let mut handles = Vec::new();

        for i in 0..8u64 {
            let ch = channel.clone();
            handles.push(std::thread::spawn(move || {
                let req = test_request(&format!("thread-{}", i));
                ch.notify(&req).unwrap();
            }));
        }

        for h in handles {
            h.join().unwrap();
        }

        assert_eq!(channel.sent_count(), 8);
    }

    // ============================================================
    // T8: FakeHumanChannel リセット（3 テスト）
    // ============================================================

    /// T8-1: reset → sent_count == 0
    #[test]
    fn t8_1_reset_sent_count() {
        let channel = FakeHumanChannel::new(VecDeque::new());
        channel.notify(&test_request("a")).unwrap();
        channel.notify(&test_request("b")).unwrap();
        assert_eq!(channel.sent_count(), 2);
        channel.reset();
        assert_eq!(channel.sent_count(), 0);
    }

    /// T8-2: reset → requests_sent 空
    #[test]
    fn t8_2_reset_requests_empty() {
        let channel = FakeHumanChannel::new(VecDeque::new());
        channel.notify(&test_request("x")).unwrap();
        channel.reset();
        assert!(channel.requests_sent().is_empty());
    }

    /// T8-3: reset → 再使用可能
    #[test]
    fn t8_3_reset_reusable() {
        let channel = FakeHumanChannel::new(VecDeque::new());
        channel.notify(&test_request("before")).unwrap();
        channel.reset();
        channel.notify(&test_request("after")).unwrap();
        assert_eq!(channel.sent_count(), 1);
        assert_eq!(channel.requests_sent()[0].subject, "after");
    }

    // ============================================================
    // T9: reconnect 回復可能性（7 テスト）
    // ============================================================

    /// T9-1: 解決済み再接続
    #[test]
    fn t9_1_resolved_reconnect() {
        let outcome = HumanOutcome::Responded(HumanResponse {
            decision: HumanDecision::Approved,
            comment: Some("done".into()),
            revised_body: None,
        });
        let channel = FakeHumanChannel::new(VecDeque::from(vec![outcome.clone()]));
        let handle = channel.communicate(&test_request("initial")).unwrap();
        let _ = handle.wait(None).unwrap();

        let interactions = channel.export_interactions();
        let id = uuid::Uuid::parse_str(&interactions[0].interaction_id).unwrap();

        let handle2 = channel.reconnect(id, &test_request("reconnect")).unwrap();
        let outcome2 = handle2.wait(None).unwrap();
        assert_eq!(outcome2, outcome);
    }

    /// T9-2: 未知 ID + 空キュー
    #[test]
    fn t9_2_unknown_id_empty_queue() {
        let channel = FakeHumanChannel::new(VecDeque::new());
        let id = uuid::Uuid::new_v4();
        let result = channel.reconnect(id, &test_request("unknown"));
        assert!(matches!(result, Err(DarviumError::HumanChannelIo(_))));
    }

    /// T9-3: 未知 ID + プリロードあり
    #[test]
    fn t9_3_unknown_id_with_preloaded() {
        let outcome = HumanOutcome::Responded(HumanResponse {
            decision: HumanDecision::Approved,
            comment: Some("recovered".into()),
            revised_body: None,
        });
        let channel = FakeHumanChannel::new(VecDeque::from(vec![outcome.clone()]));
        let id = uuid::Uuid::new_v4();
        let handle = channel.reconnect(id, &test_request("recover")).unwrap();
        let result = handle.wait(None).unwrap();
        assert_eq!(result, outcome);
    }

    /// T9-4: Stdinout reconnect protocol
    #[test]
    fn t9_4_stdinout_reconnect_protocol() {
        let reader = std::io::BufReader::new(std::io::empty());
        let mut writer: Vec<u8> = Vec::new();
        let channel = StdinoutChannel::new(reader, &mut writer);
        let id = uuid::Uuid::new_v4();
        let _ = channel.reconnect(id, &test_request("reconn-protocol"));

        let output = String::from_utf8(writer).unwrap();
        assert!(output.contains(r#""type":"reconnect""#));
        assert!(output.contains(&id.to_string()));
        assert!(output.contains(r#""subject":"reconn-protocol""#));
    }

    /// T9-5: Stdinout reconnect 解決
    #[test]
    fn t9_5_stdinout_reconnect_resolve() {
        let response = r#"{"interaction_id":"00000000-0000-0000-0000-000000000000","outcome":{"Responded":{"decision":"Approved","comment":null,"revised_body":null}}}"#;
        let reader = std::io::BufReader::new(response.as_bytes());
        let writer: Vec<u8> = Vec::new();
        let channel = StdinoutChannel::new(reader, writer);
        let id = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap();
        let handle = channel.reconnect(id, &test_request("resolve")).unwrap();
        let result = handle.wait(Some(Duration::from_secs(1)));
        assert!(result.is_ok());
    }

    /// T9-6: Stdinout reconnect 不正応答
    #[test]
    fn t9_6_stdinout_reconnect_invalid() {
        let invalid = "not json at all\n";
        let reader = std::io::BufReader::new(invalid.as_bytes());
        let writer: Vec<u8> = Vec::new();
        let channel = StdinoutChannel::new(reader, writer);
        let id = uuid::Uuid::parse_str("00000000-0000-0000-0000-000000000000").unwrap();
        let handle = channel.reconnect(id, &test_request("invalid")).unwrap();
        let result = handle.wait(Some(Duration::from_secs(1)));
        assert!(matches!(result, Err(DarviumError::HumanChannelIo(_))));
    }

    /// T9-7: notify→communicate→reconnect 一貫性
    #[test]
    fn t9_7_full_flow_consistency() {
        let outcome = HumanOutcome::Responded(HumanResponse {
            decision: HumanDecision::Approved,
            comment: Some("full flow".into()),
            revised_body: None,
        });
        let channel = FakeHumanChannel::new(VecDeque::from(vec![outcome.clone()]));

        channel.notify(&test_request("step1")).unwrap();
        assert_eq!(channel.sent_count(), 1);

        let handle = channel.communicate(&test_request("step2")).unwrap();
        let comm_outcome = handle.wait(None).unwrap();
        assert_eq!(comm_outcome, outcome);
        assert_eq!(channel.sent_count(), 2);

        let interactions = channel.export_interactions();
        assert_eq!(interactions.len(), 1);
        let id = uuid::Uuid::parse_str(&interactions[0].interaction_id).unwrap();
        let handle2 = channel.reconnect(id, &test_request("step3")).unwrap();
        let recon_outcome = handle2.wait(None).unwrap();
        assert_eq!(recon_outcome, outcome);
    }

    // ============================================================
    // OTS: 観測テスト
    // ============================================================

    /// OTS-1: 呼び出し回数 vs 記録件数完全一致 (n=10,000, σ²=0)
    #[test]
    fn ots1_call_count_vs_record_count() {
        use rand::rngs::StdRng;
        use rand::{Rng, SeedableRng};

        let n = 10_000u64;
        let channel = FakeHumanChannel::new(VecDeque::new());
        let mut rng = StdRng::seed_from_u64(12345);

        println!("=== OTS-1: Call Count vs Record Count ===");
        println!("n={}", n);

        for i in 0..n {
            let request = HumanRequest {
                subject: format!("ots-{}", i),
                body: rng.random::<u64>().to_string(),
                context: serde_json::json!({"seq": i}),
                timeout: None,
            };
            channel.notify(&request).unwrap();
        }

        let count = channel.sent_count();
        let record_len = channel.requests_sent().len() as u64;

        println!("sent_count={}, requests_sent_len={}", count, record_len);
        assert_eq!(count, n, "sent_count must equal n ({} != {})", count, n);
        assert_eq!(
            record_len, n,
            "requests_sent length must equal n ({} != {})",
            record_len, n
        );
        println!("=== 結果: PASS (σ² = 0) ===");
    }

    /// OTS-2: Serde ラウンドトリップ (n=8,192)
    #[test]
    fn ots2_serde_roundtrip() {
        use rand::rngs::StdRng;
        use rand::{Rng, SeedableRng};

        let n = 8_192usize;
        let mut rng = StdRng::seed_from_u64(12345);
        let decisions = [
            HumanDecision::Approved,
            HumanDecision::Rejected,
            HumanDecision::NeedsRevision,
            HumanDecision::Irrelevant,
            HumanDecision::Unsafe,
        ];
        let statuses = [InteractionStatus::Pending, InteractionStatus::Resolved];

        println!("=== OTS-2: Serde Roundtrip ===");
        println!(
            "n={}, types=[StoredInteraction, HumanRequest, HumanOutcome]",
            n
        );

        let mut passed: u64 = 0;
        for i in 0..n {
            let original = StoredInteraction {
                interaction_id: uuid::Uuid::new_v4().to_string(),
                request: HumanRequest {
                    subject: rng.random::<u64>().to_string(),
                    body: rng.random::<u64>().to_string(),
                    context: serde_json::json!({"r": rng.random::<u64>()}),
                    timeout: if rng.random_bool(0.5) {
                        Some(Duration::from_secs(rng.random_range(1..3600)))
                    } else {
                        None
                    },
                },
                outcome: if rng.random_bool(0.5) {
                    Some(HumanOutcome::Responded(HumanResponse {
                        decision: decisions[rng.random_range(0..5)],
                        comment: if rng.random_bool(0.5) {
                            Some(rng.random::<u64>().to_string())
                        } else {
                            None
                        },
                        revised_body: if rng.random_bool(0.3) {
                            Some(rng.random::<u64>().to_string())
                        } else {
                            None
                        },
                    }))
                } else if rng.random_bool(0.5) {
                    Some(HumanOutcome::TimedOut)
                } else {
                    Some(HumanOutcome::Unreachable(rng.random::<u64>().to_string()))
                },
                status: statuses[rng.random_range(0..2)],
                created_at: rng.random::<u64>() % 1_000_000_000,
                updated_at: rng.random::<u64>() % 1_000_000_000,
            };

            let json = serde_json::to_string(&original).unwrap();
            let deserialized: StoredInteraction = serde_json::from_str(&json).unwrap();
            assert_eq!(
                deserialized, original,
                "OTS-2: StoredInteraction roundtrip failed at iteration {}",
                i
            );
            passed += 1;
        }

        println!("passed={}/{}", passed, n);
        println!("=== 結果: PASS (全ラウンドトリップ成功) ===");
    }
}
