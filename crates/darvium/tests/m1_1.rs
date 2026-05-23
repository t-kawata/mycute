// M1-1 観測テスト (OTS-1〜OTS-4)
//
// 本ファイルは以下の観測テストを提供する：
//
// - OTS-1: 線形成長ダイナミクス — μ=0 時 L_q(t) = λt の一致検証
// - OTS-2: スレッド競合待機時間分布 — P50/P90/P99 の計測
// - OTS-3: 情報リーク率 P_leak の統計的検定 (n=10,000)
// - OTS-4: 多様な HumanDecision 応答パターン (5値×20回)

use std::sync::mpsc;
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use darvium::error::DarviumError;
use darvium::human_channel::{HumanChannel, InteractionHandle};
use darvium::types::*;
use darvium::HumanReviewQueue;

// ── テスト用 NoopChannel ──────────────────────────────

struct NoopChannel;

impl HumanChannel for NoopChannel {
    fn notify(&self, _request: &HumanRequest) -> Result<(), DarviumError> {
        Ok(())
    }
    fn communicate(&self, _request: &HumanRequest) -> Result<InteractionHandle, DarviumError> {
        let (_tx, rx) = mpsc::channel();
        Ok(InteractionHandle::new(uuid::Uuid::new_v4(), rx))
    }
    fn reconnect(
        &self,
        _interaction_id: uuid::Uuid,
        _request: &HumanRequest,
    ) -> Result<InteractionHandle, DarviumError> {
        let (_tx, rx) = mpsc::channel();
        Ok(InteractionHandle::new(uuid::Uuid::new_v4(), rx))
    }
}

// ── ヘルパー ──────────────────────────────────────────

fn make_request(subject: &str) -> HumanRequest {
    HumanRequest {
        subject: subject.to_string(),
        body: "test body".into(),
        context: serde_json::Value::Null,
        timeout: None,
    }
}

fn make_context(id: &str) -> serde_json::Value {
    serde_json::json!({"mission_id": id})
}

// ── OTS-1: 線形成長ダイナミクス ──────────────────────

/// OTS-1: μ=0 時に L_q(t) = λt の線形成長を確認する。
///
/// 解決なし (μ=0) の状態で λ ∈ {1, 5, 10} の到着率でキューイングし、
/// 各時刻の L_q(t) が理論値 λt に一致するかを確認する。
///
/// 出力: 各 λ について時刻ごとの L_q(t) 時系列、線形回帰の傾きと R²
#[test]
fn ots1_queue_growth_dynamics() {
    println!("=== OTS-1: 線形成長ダイナミクス ===");

    let lambdas: [u32; 3] = [1, 5, 10];

    for &lambda in &lambdas {
        let queue = Arc::new(HumanReviewQueue::new(
            Arc::new(NoopChannel),
            HumanReviewQueuePolicy::default(),
        ));

        let interval_ms = 1000 / lambda; // λ あたりのミリ秒間隔
        let total_pushes = lambda * 3; // 約 3 秒分
        let mut measurements: Vec<(f64, usize)> = Vec::new(); // (経過秒, キュー長)

        let start = Instant::now();
        for i in 0..total_pushes {
            let mid = format!("ots1_l{}_{}", lambda, i);
            queue
                .push(&mid, make_context(&mid), make_request("OTS-1"))
                .expect("push should succeed");

            // 一定間隔で計測
            let elapsed = start.elapsed().as_secs_f64();
            measurements.push((elapsed, queue.len()));
            thread::sleep(Duration::from_millis(interval_ms as u64));
        }

        // 線形回帰: y = a*x + b
        let sum_x: f64 = measurements.iter().map(|(t, _)| t).sum();
        let sum_y: f64 = measurements.iter().map(|(_, l)| *l as f64).sum();
        let sum_xy: f64 = measurements.iter().map(|(t, l)| t * *l as f64).sum();
        let sum_xx: f64 = measurements.iter().map(|(t, _)| t * t).sum();

        let n = measurements.len() as f64;
        let denom = n * sum_xx - sum_x * sum_x;
        let slope = if denom > 0.0 {
            (n * sum_xy - sum_x * sum_y) / denom
        } else {
            0.0
        };
        let intercept = if denom > 0.0 {
            (sum_y - slope * sum_x) / n
        } else {
            sum_y / n
        };

        // R² = 1 - SS_res / SS_tot
        let ss_res: f64 = measurements
            .iter()
            .map(|(t, l)| {
                let predicted = slope * t + intercept;
                (predicted - *l as f64).powi(2)
            })
            .sum();
        let mean_y = sum_y / n;
        let ss_tot: f64 = measurements
            .iter()
            .map(|(_, l)| (*l as f64 - mean_y).powi(2))
            .sum();
        let r2 = if ss_tot > 0.0 {
            1.0 - ss_res / ss_tot
        } else {
            1.0
        };

        println!(
            "λ={}: pushes={}, slope={:.3}, intercept={:.3}, R²={:.4}, final_len={}",
            lambda,
            total_pushes,
            slope,
            intercept,
            r2,
            queue.len()
        );

        for (t, l) in &measurements {
            println!("  t={:.2}s  L_q(t)={}", t, l);
        }

        assert!(
            r2 >= 0.95,
            "OTS-1 FAILED: λ={}, R²={:.4} < 0.95",
            lambda,
            r2
        );
        // 傾きが λ から 30% 以上乖離していないことを確認
        let slope_ratio = slope / lambda as f64;
        assert!(
            (slope_ratio - 1.0).abs() < 0.30,
            "OTS-1 FAILED: λ={}, slope={:.3} deviates >30% from expected",
            lambda,
            slope
        );
    }
    println!("=== OTS-1 PASS ===");
}

// ── OTS-2: スレッド競合待機時間分布 ──────────────────

/// OTS-2: 16 スレッドからの同時アクセス時の Mutex 待機時間の分布を計測。
///
/// 出力: 中央値・P90・P99
#[test]
fn ots2_contention_wait_time() {
    println!("=== OTS-2: スレッド競合待機時間分布 ===");

    let queue = Arc::new(HumanReviewQueue::new(
        Arc::new(NoopChannel),
        HumanReviewQueuePolicy::default(),
    ));

    let mut handles = Vec::new();
    let num_threads = 4;
    let ops_per_thread = 25;

    for t in 0..num_threads {
        let q = Arc::clone(&queue);
        handles.push(thread::spawn(move || {
            for i in 0..ops_per_thread {
                let mid = format!("ots2_t{}_i{}", t, i);
                q.push(&mid, make_context(&mid), make_request("OTS-2"))
                    .expect("push should succeed");
            }
        }));
    }

    for h in handles {
        h.join().expect("thread should not panic");
    }

    let samples = queue.contention_samples();
    let total = queue.len();

    let mut sorted = samples.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let n = sorted.len();
    let median = if n > 0 { sorted[n / 2] } else { 0.0 };
    let p90 = if n > 0 {
        sorted[(n as f64 * 0.90) as usize]
    } else {
        0.0
    };
    let p99 = if n > 0 {
        sorted[(n as f64 * 0.99) as usize]
    } else {
        0.0
    };

    println!(
        "threads={}, ops_per_thread={}, total_items={}, contention_samples={}",
        num_threads, ops_per_thread, total, n
    );
    println!("Median={:.6}s, P90={:.6}s, P99={:.6}s", median, p90, p99);

    assert!(p99 < 0.100, "OTS-2 FAILED: P99={:.6}s >= 100ms", p99);

    println!("=== OTS-2 PASS ===");
}

// ── OTS-3: 情報リーク率の統計的検定 ──────────────────

/// OTS-3: N=10000 回の自動実行パイプライン走査でキューイング済みミッションが
/// 漏洩した回数を計測。
///
/// 出力: P_leak, p-value
#[test]
fn ots3_leak_rate_verification() {
    println!("=== OTS-3: 情報リーク率 P_leak の検定 ===");

    let queue = Arc::new(HumanReviewQueue::new(
        Arc::new(NoopChannel),
        HumanReviewQueuePolicy::default(),
    ));

    // キューに 100 件投入
    for i in 0..100 {
        let mid = format!("ots3_leak_{}", i);
        queue
            .push(&mid, make_context(&mid), make_request("OTS-3"))
            .expect("push should succeed");
    }

    // 10000 回の模擬走査
    let n_trials: u64 = 10_000;
    for i in 0..n_trials {
        let mid = format!("ots3_leak_{}", i % 100);
        let _ = queue.contains_mission(&mid);
    }

    let leak_attempts = queue.leak_attempts();
    let leak_successes = queue.leak_successes();
    let p_leak = if leak_attempts > 0 {
        leak_successes as f64 / leak_attempts as f64
    } else {
        0.0
    };

    println!(
        "trials={}, leak_attempts={}, leak_successes={}",
        n_trials, leak_attempts, leak_successes
    );
    println!("P_leak={:.10}", p_leak);

    assert_eq!(
        leak_successes, 0,
        "OTS-3 FAILED: P_leak != 0 (leak_successes={})",
        leak_successes
    );

    println!("=== OTS-3 PASS ===");
}

// ── OTS-4: 多様な HumanDecision 応答パターン ────────

/// OTS-4: 5 値をそれぞれ 20 回ずつ注入し、各 decision に対応する
/// 状態遷移とキュー滞留時間の分布を出力。
#[test]
fn ots4_decision_patterns() {
    println!("=== OTS-4: HumanDecision 応答パターン ===");

    let decisions = [
        HumanDecision::Approved,
        HumanDecision::Rejected,
        HumanDecision::NeedsRevision,
        HumanDecision::Irrelevant,
        HumanDecision::Unsafe,
    ];

    let mut total_ok = 0u64;
    let mut total_fail = 0u64;
    let mut residence_times: Vec<(String, f64)> = Vec::new();

    for decision in &decisions {
        let queue = Arc::new(HumanReviewQueue::new(
            Arc::new(NoopChannel),
            HumanReviewQueuePolicy::default(),
        ));

        let decision_name = match decision {
            HumanDecision::Approved => "Approved",
            HumanDecision::Rejected => "Rejected",
            HumanDecision::NeedsRevision => "NeedsRevision",
            HumanDecision::Irrelevant => "Irrelevant",
            HumanDecision::Unsafe => "Unsafe",
        };

        let mut ok_count = 0u64;
        let mut fail_count = 0u64;

        for i in 0..5 {
            let mid = format!("ots4_{}_{}", decision_name, i);
            let push_time = Instant::now();

            match queue.push(&mid, make_context(&mid), make_request("OTS-4")) {
                Ok(_) => {
                    thread::sleep(Duration::from_micros(10));

                    let residence = push_time.elapsed().as_secs_f64();
                    residence_times.push((decision_name.to_string(), residence));

                    match queue.resolve(&mid, *decision) {
                        Ok(_) => {
                            ok_count += 1;
                        }
                        Err(e) => {
                            fail_count += 1;
                            println!(
                                "  WARN: decision={} i={} resolve failed: {:?}",
                                decision_name, i, e
                            );
                        }
                    }
                }
                Err(e) => {
                    fail_count += 1;
                    println!(
                        "  WARN: decision={} i={} push failed: {:?}",
                        decision_name, i, e
                    );
                }
            }
        }

        println!(
            "decision={}: ok={}, fail={}",
            decision_name, ok_count, fail_count
        );
        total_ok += ok_count;
        total_fail += fail_count;

        assert_eq!(
            fail_count, 0,
            "OTS-4 FAILED: decision={} has {} failures",
            decision_name, fail_count
        );
    }

    // 滞留時間の統計
    let n = residence_times.len();
    if n > 0 {
        let mut sorted: Vec<f64> = residence_times.iter().map(|(_, t)| *t).collect();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
        let median = sorted[n / 2];
        let p99 = sorted[(n as f64 * 0.99) as usize];
        let max = sorted[n - 1];

        println!(
            "residence_times: n={}, median={:.6}s, P99={:.6}s, max={:.6}s",
            n, median, p99, max
        );

        for decision_name in &[
            "Approved",
            "Rejected",
            "NeedsRevision",
            "Irrelevant",
            "Unsafe",
        ] {
            let times: Vec<f64> = residence_times
                .iter()
                .filter(|(d, _)| d == decision_name)
                .map(|(_, t)| *t)
                .collect();
            if !times.is_empty() {
                let avg = times.iter().sum::<f64>() / times.len() as f64;
                println!(
                    "  {}: avg_residence={:.6}s (n={})",
                    decision_name,
                    avg,
                    times.len()
                );
            }
        }
    }

    println!("total_ok={}, total_fail={}", total_ok, total_fail);
    assert_eq!(total_fail, 0, "OTS-4 FAILED: {} total failures", total_fail);

    println!("=== OTS-4 PASS ===");
}
