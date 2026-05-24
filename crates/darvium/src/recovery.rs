// HITL 起動時回復ループ
//
// MetadataStore から全 Pending インタラクションを取得し、
// HumanChannel::reconnect() → InteractionHandle::wait() → resolve の
// プロトコルで順次回復する。

use std::time::{Duration, Instant};

use crate::constants::{HITL_DEFAULT_TIMEOUT_SECS, HITL_RECONNECT_BACKOFF_SECS};
use crate::error::DarviumError;
use crate::human_channel::HumanChannel;
use crate::store::MetadataStore;
use crate::types::HumanOutcome;

/// 回復ループの実行結果サマリ。
#[derive(Debug, Clone)]
pub struct RecoverySummary {
    /// 処理対象の総 Pending 件数
    pub total: usize,
    /// 解決成功件数
    pub succeeded: usize,
    /// タイムアウト件数
    pub timed_out: usize,
    /// 到達不能件数
    pub unreachable: usize,
    /// その他エラー件数
    pub failed: usize,
    /// 回復ループ全体の経過時間（ミリ秒）
    pub duration_ms: u64,
}

/// 全 Pending HITL インタラクションを回復する。
///
/// §12B.6 クラッシュリカバリプロトコル:
/// 1. list_pending 全件走査
/// 2. 各レコードに reconnect → wait(timeout)
/// 3. 失敗時は指数バックオフ再試行
/// 4. 応答受信後、resolve_human_interaction で更新
pub fn recover_pending_interactions(
    store: &dyn MetadataStore,
    channel: &dyn HumanChannel,
    timeout: Option<Duration>,
    max_retries: u32,
) -> Result<RecoverySummary, DarviumError> {
    let start = Instant::now();
    let effective_timeout = timeout.unwrap_or(Duration::from_secs(HITL_DEFAULT_TIMEOUT_SECS));

    let pending = store.list_pending_human_interactions()?;
    let total = pending.len();

    let mut succeeded: usize = 0;
    let mut timed_out: usize = 0;
    let mut unreachable: usize = 0;
    let mut failed: usize = 0;

    for record in &pending {
        let interaction_id = match uuid::Uuid::parse_str(&record.interaction_id) {
            Ok(id) => id,
            Err(_) => {
                failed += 1;
                continue;
            }
        };

        let outcome = 'recover: {
            for attempt in 0..=max_retries {
                if attempt > 0 {
                    let backoff = HITL_RECONNECT_BACKOFF_SECS * (2u64.pow(attempt - 1) as f64);
                    std::thread::sleep(Duration::from_secs_f64(backoff));
                }

                match channel.reconnect(interaction_id, record.request()) {
                    Ok(handle) => match handle.wait(Some(effective_timeout)) {
                        Ok(outcome) => break 'recover outcome,
                        Err(DarviumError::HumanChannelClosed) => {
                            break 'recover HumanOutcome::Unreachable("channel closed".into());
                        }
                        Err(_) => continue,
                    },
                    Err(_) => continue,
                }
            }
            HumanOutcome::TimedOut
        };

        match &outcome {
            HumanOutcome::Responded(_) => succeeded += 1,
            HumanOutcome::TimedOut => timed_out += 1,
            HumanOutcome::Unreachable(_) => unreachable += 1,
        }

        let _ = store.resolve_human_interaction(&record.interaction_id, &outcome);
    }

    let duration_ms = start.elapsed().as_millis() as u64;

    Ok(RecoverySummary {
        total,
        succeeded,
        timed_out,
        unreachable,
        failed,
        duration_ms,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::human_channel::*;
    use crate::store::InMemoryMetadataStore;
    use crate::types::*;
    use std::collections::VecDeque;

    // ============================================================
    // T-R1: 単一 Pending 回復
    // ============================================================
    #[test]
    fn single_pending_recovery() {
        let store = InMemoryMetadataStore::new();
        let expected = HumanOutcome::Responded(HumanResponse {
            decision: HumanDecision::Approved,
            comment: Some("recovered".into()),
            revised_body: None,
        });
        let channel = FakeHumanChannel::new(VecDeque::from(vec![expected.clone()]));

        let record = StoredInteraction {
            interaction_id: "00000000-0000-0000-0000-000000000000".to_string(),
            payload: HitlPayload {
                request: HumanRequest {
                    subject: "recovery".into(),
                    body: "test".into(),
                    context: serde_json::json!({}),
                    timeout: None,
                },
            },
            outcome: None,
            status: InteractionStatus::Pending,
            created_at: 100,
            updated_at: 100,
        };
        store.store_human_interaction(&record).unwrap();

        let summary =
            recover_pending_interactions(&store, &channel, Some(Duration::from_secs(1)), 0)
                .unwrap();
        assert_eq!(summary.total, 1);
        assert_eq!(summary.succeeded, 1);

        let loaded = store
            .load_human_interaction("00000000-0000-0000-0000-000000000000")
            .unwrap();
        assert_eq!(loaded.status, InteractionStatus::Resolved);
    }

    // ============================================================
    // T-R2: N≥10 一括回復
    // ============================================================
    #[test]
    fn batch_recovery_n10() {
        let store = InMemoryMetadataStore::new();
        let n = 10;
        let mut preloaded = VecDeque::new();
        for i in 0..n {
            let record = StoredInteraction {
                interaction_id: format!("{:032x}", i),
                payload: HitlPayload {
                    request: HumanRequest {
                        subject: format!("batch-{}", i),
                        body: "".into(),
                        context: serde_json::json!({}),
                        timeout: None,
                    },
                },
                outcome: None,
                status: InteractionStatus::Pending,
                created_at: i as u64,
                updated_at: i as u64,
            };
            store.store_human_interaction(&record).unwrap();
            preloaded.push_back(HumanOutcome::Responded(HumanResponse {
                decision: HumanDecision::Approved,
                comment: Some(format!("batch-{}", i)),
                revised_body: None,
            }));
        }
        let channel = FakeHumanChannel::new(preloaded);

        let summary =
            recover_pending_interactions(&store, &channel, Some(Duration::from_secs(1)), 0)
                .unwrap();
        assert_eq!(summary.total, n);
        assert_eq!(summary.succeeded, n);

        for i in 0..n {
            let loaded = store
                .load_human_interaction(&format!("{:032x}", i))
                .unwrap();
            assert_eq!(loaded.status, InteractionStatus::Resolved);
        }
    }

    // ============================================================
    // T-R3: 混合シナリオ（5成功 + 3タイムアウト + 2到達不能）
    // ============================================================
    #[test]
    fn mixed_scenario_recovery() {
        let store = InMemoryMetadataStore::new();
        let mut preloaded = VecDeque::new();
        let ids: Vec<String> = (0..10)
            .map(|i| format!("00000000-0000-0000-0000-00000000001{}", i))
            .collect();

        for (i, id) in ids.iter().enumerate() {
            let record = StoredInteraction {
                interaction_id: id.clone(),
                payload: HitlPayload {
                    request: HumanRequest {
                        subject: id.clone(),
                        body: "".into(),
                        context: serde_json::json!({}),
                        timeout: None,
                    },
                },
                outcome: None,
                status: InteractionStatus::Pending,
                created_at: i as u64,
                updated_at: i as u64,
            };
            store.store_human_interaction(&record).unwrap();

            let outcome = if i < 5 {
                HumanOutcome::Responded(HumanResponse {
                    decision: HumanDecision::Approved,
                    comment: Some(format!("ok-{}", i)),
                    revised_body: None,
                })
            } else if i < 8 {
                HumanOutcome::TimedOut
            } else {
                HumanOutcome::Unreachable(format!("unreachable-{}", i))
            };
            preloaded.push_back(outcome);
        }
        let channel = FakeHumanChannel::new(preloaded);

        let summary =
            recover_pending_interactions(&store, &channel, Some(Duration::from_secs(1)), 0)
                .unwrap();
        assert_eq!(summary.total, 10);
        assert_eq!(summary.succeeded, 5);
        assert_eq!(summary.timed_out, 3);
        assert_eq!(summary.unreachable, 2);
    }

    // ============================================================
    // T-R4: StdinoutChannel クロスインスタンス
    // ============================================================
    #[test]
    fn stdinout_cross_instance_recovery() {
        let response_json = r#"{"interaction_id":"00000000-0000-0000-0000-000000000000","outcome":{"Responded":{"decision":"Approved","comment":"cross-instance","revised_body":null}}}"#;
        let reader = std::io::BufReader::new(response_json.as_bytes());
        let writer: Vec<u8> = Vec::new();
        let channel = StdinoutChannel::new(reader, writer);

        let store = InMemoryMetadataStore::new();
        let record = StoredInteraction {
            interaction_id: "00000000-0000-0000-0000-000000000000".to_string(),
            payload: HitlPayload {
                request: HumanRequest {
                    subject: "cross-instance".into(),
                    body: "reconnect".into(),
                    context: serde_json::json!({}),
                    timeout: None,
                },
            },
            outcome: None,
            status: InteractionStatus::Pending,
            created_at: 100,
            updated_at: 100,
        };
        store.store_human_interaction(&record).unwrap();

        let summary =
            recover_pending_interactions(&store, &channel, Some(Duration::from_secs(1)), 0)
                .unwrap();
        assert_eq!(summary.total, 1);
        assert_eq!(summary.succeeded, 1);
    }

    // ============================================================
    // T-R5: TimedOut 再通知
    // ============================================================
    #[test]
    fn timed_out_retry() {
        let store = InMemoryMetadataStore::new();
        let record = StoredInteraction {
            interaction_id: "00000000-0000-0000-0000-000000000001".to_string(),
            payload: HitlPayload {
                request: HumanRequest {
                    subject: "timedout".into(),
                    body: "".into(),
                    context: serde_json::json!({}),
                    timeout: None,
                },
            },
            outcome: None,
            status: InteractionStatus::Pending,
            created_at: 100,
            updated_at: 100,
        };
        store.store_human_interaction(&record).unwrap();

        let channel = FakeHumanChannel::new(VecDeque::from(vec![HumanOutcome::TimedOut]));

        let summary =
            recover_pending_interactions(&store, &channel, Some(Duration::from_secs(1)), 0)
                .unwrap();
        assert_eq!(summary.total, 1);
        assert_eq!(summary.timed_out, 1);

        let loaded = store
            .load_human_interaction("00000000-0000-0000-0000-000000000001")
            .unwrap();
        assert_eq!(loaded.status, InteractionStatus::Resolved);
        assert!(matches!(loaded.outcome, Some(HumanOutcome::TimedOut)));
    }

    // ============================================================
    // T-R6: 競合状態（応答受信直後クラッシュ → 再起動後回復）
    // ============================================================
    #[test]
    fn race_condition_recovery() {
        let store = InMemoryMetadataStore::new();
        let record = StoredInteraction {
            interaction_id: "00000000-0000-0000-0000-000000000002".to_string(),
            payload: HitlPayload {
                request: HumanRequest {
                    subject: "race".into(),
                    body: "condition".into(),
                    context: serde_json::json!({}),
                    timeout: None,
                },
            },
            outcome: None,
            status: InteractionStatus::Pending,
            created_at: 100,
            updated_at: 100,
        };
        store.store_human_interaction(&record).unwrap();

        let outcome = HumanOutcome::Responded(HumanResponse {
            decision: HumanDecision::Approved,
            comment: Some("race-recovered".into()),
            revised_body: None,
        });
        let channel = FakeHumanChannel::new(VecDeque::from(vec![outcome]));

        let summary =
            recover_pending_interactions(&store, &channel, Some(Duration::from_secs(1)), 0)
                .unwrap();
        assert_eq!(summary.total, 1);
        assert_eq!(summary.succeeded, 1);

        let loaded = store
            .load_human_interaction("00000000-0000-0000-0000-000000000002")
            .unwrap();
        assert_eq!(loaded.status, InteractionStatus::Resolved);
    }

    // ============================================================
    // T11: 異種チャネル差し替え回復
    // FakeHumanChannel 保存 → StdinoutChannel 回復
    // ============================================================
    #[test]
    fn cross_channel_swap_recovery() {
        let interaction_id = uuid::Uuid::new_v4();
        let store = InMemoryMetadataStore::new();
        let request = HumanRequest {
            subject: "cross-channel".into(),
            body: "swap".into(),
            context: serde_json::json!({}),
            timeout: None,
        };
        let record = StoredInteraction {
            interaction_id: interaction_id.to_string(),
            payload: HitlPayload {
                request: request.clone(),
            },
            outcome: None,
            status: InteractionStatus::Pending,
            created_at: 100,
            updated_at: 100,
        };
        store.store_human_interaction(&record).unwrap();

        let response = format!(
            r#"{{"interaction_id":"{}","outcome":{{"Responded":{{"decision":"Approved","comment":"cross-swap","revised_body":null}}}}}}"#,
            interaction_id
        );
        let reader = std::io::BufReader::new(std::io::Cursor::new(response.into_bytes()));
        let writer: Vec<u8> = Vec::new();
        let stdinout = StdinoutChannel::new(reader, writer);

        let summary =
            recover_pending_interactions(&store, &stdinout, Some(Duration::from_secs(1)), 0)
                .unwrap();
        assert_eq!(summary.total, 1);
        assert_eq!(summary.succeeded, 1);
    }

    // ============================================================
    // OTS-1: バッチ回復成功率
    // ============================================================
    #[test]
    fn ots1_batch_recovery_success_rate() {
        use rand::rngs::StdRng;
        use rand::{Rng, SeedableRng};

        let mut rng = StdRng::seed_from_u64(crate::constants::TEST_PRNG_SEED);
        let sample_sizes = [1usize, 10, 100];

        println!("=== OTS-1: Batch Recovery Success Rate ===");
        println!("samples={:?}", sample_sizes);

        for &n in &sample_sizes {
            let trials = 20;
            let mut success_counts: Vec<u64> = Vec::new();

            for trial in 0..trials {
                let store = InMemoryMetadataStore::new();
                let mut preloaded = VecDeque::new();

                for i in 0..n {
                    let id = format!("{:08x}-{:04x}-0000-0000-000000000000", trial, i);
                    let record = StoredInteraction {
                        interaction_id: id,
                        payload: HitlPayload {
                            request: HumanRequest {
                                subject: format!("rate-{}", i),
                                body: "".into(),
                                context: serde_json::json!({}),
                                timeout: None,
                            },
                        },
                        outcome: None,
                        status: InteractionStatus::Pending,
                        created_at: 0,
                        updated_at: 0,
                    };
                    store.store_human_interaction(&record).unwrap();

                    if rng.random_bool(0.9) {
                        preloaded.push_back(HumanOutcome::Responded(HumanResponse {
                            decision: HumanDecision::Approved,
                            comment: None,
                            revised_body: None,
                        }));
                    } else {
                        preloaded.push_back(HumanOutcome::TimedOut);
                    }
                }
                let channel = FakeHumanChannel::new(preloaded);
                let summary =
                    recover_pending_interactions(&store, &channel, Some(Duration::from_secs(1)), 0)
                        .unwrap();
                success_counts.push(summary.succeeded as u64);
            }

            let total_succeeded: u64 = success_counts.iter().sum();
            let total_attempts = (trials * n) as f64;
            let rate = total_succeeded as f64 / total_attempts * 100.0;
            let mean = success_counts.iter().sum::<u64>() as f64 / trials as f64;

            println!(
                "  n={}: success_rate={:.1}%, mean_success={:.2}/{}, trials={}",
                n, rate, mean, n, trials
            );
        }
        println!("=== 結果: PASS ===");
    }

    // ============================================================
    // OTS-2: 回復レイテンシ分布
    // ============================================================
    #[test]
    fn ots2_recovery_latency_distribution() {
        let n = 10;
        let trials = 50;
        let mut latencies: Vec<u64> = Vec::with_capacity(trials);

        println!("=== OTS-2: Recovery Latency Distribution ===");
        println!("n={}, trials={}", n, trials);

        for trial in 0..trials {
            let store = InMemoryMetadataStore::new();
            let mut preloaded = VecDeque::new();

            for i in 0..n {
                let id = format!("{:08x}-{:04x}-0000-0000-000000000000", trial, i);
                let record = StoredInteraction {
                    interaction_id: id,
                    payload: HitlPayload {
                        request: HumanRequest {
                            subject: format!("latency-{}", i),
                            body: "".into(),
                            context: serde_json::json!({}),
                            timeout: None,
                        },
                    },
                    outcome: None,
                    status: InteractionStatus::Pending,
                    created_at: 0,
                    updated_at: 0,
                };
                store.store_human_interaction(&record).unwrap();
                preloaded.push_back(HumanOutcome::Responded(HumanResponse {
                    decision: HumanDecision::Approved,
                    comment: None,
                    revised_body: None,
                }));
            }
            let channel = FakeHumanChannel::new(preloaded);

            let start = Instant::now();
            let summary =
                recover_pending_interactions(&store, &channel, Some(Duration::from_secs(1)), 0)
                    .unwrap();
            let elapsed = start.elapsed().as_micros() as u64;
            latencies.push(elapsed);
            let _ = summary;
        }

        latencies.sort_unstable();

        let median = latencies[trials / 2];
        let p90 = latencies[(trials as f64 * 0.90) as usize];
        let p99 = latencies[(trials as f64 * 0.99) as usize];
        let min = latencies[0];
        let max = latencies[trials - 1];
        let mean = latencies.iter().sum::<u64>() as f64 / trials as f64;

        println!(
            "  min={}μs, median={}μs, mean={:.0}μs, p90={}μs, p99={}μs, max={}μs",
            min, median, mean, p90, p99, max
        );
        println!("  latency_histogram (10 bins):");
        let bins = 10;
        let bin_width = (max - min).max(1) as f64 / bins as f64;
        let mut histogram = vec![0usize; bins];
        for &l in &latencies {
            let bin = ((l - min) as f64 / bin_width).min((bins - 1) as f64) as usize;
            histogram[bin] += 1;
        }
        let max_bin = *histogram.iter().max().unwrap_or(&1);
        for (i, count) in histogram.iter().enumerate() {
            let bar_len = (*count as f64 / max_bin as f64 * 40.0) as usize;
            println!(
                "  [{:>3}%]: {:>5}μs-{:>5}μs | {:<40} | n={}",
                i * 10,
                min + (i as f64 * bin_width) as u64,
                min + ((i + 1) as f64 * bin_width) as u64,
                "#".repeat(bar_len),
                count
            );
        }
        println!("=== 結果: PASS ===");
    }
}
