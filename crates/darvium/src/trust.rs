// Darvium 信頼管理
//
// 本モジュールはワークフローグラフの信頼プロファイル操作を提供する。
// RFC §8.2 (管理者 fast-track), §10.3 (HumanTrustLogistic) に基づく。

use std::time::SystemTime;

use crate::constants;
use crate::types::{HumanTrustLogistic, TrustAuditEvent, TrustAuditLog, TrustProfile};

/// 試験用 MemoizedGraph 縮約実装。
///
/// M1-2 の検証に必要な最小限のフィールドのみ保持する。
/// 完全実装 (WorkflowRepository 統合・GraphVersion CAS 等) は M2/M3 以降。
#[derive(Debug, Clone)]
pub struct MemoizedGraph {
    /// グラフ識別子
    pub id: String,
    /// 信頼プロファイル
    pub trust: TrustProfile,
    /// キャッシュ無効化フラグ (invalidate_applicability_cache 呼び出し追跡用)
    pub cache_invalidated: bool,
}

impl MemoizedGraph {
    /// 指定された human_score 初期値で MemoizedGraph を生成する。
    pub fn new(id: String, human_score: f64) -> Self {
        Self {
            id,
            trust: TrustProfile {
                operational: 0.0,
                semantic: 0.0,
                temporal: 0.0,
                human: HumanTrustLogistic {
                    score: human_score,
                    ..HumanTrustLogistic::default()
                },
            },
            cache_invalidated: false,
        }
    }

    /// Applicability キャッシュを無効化する。
    ///
    /// 現在は追跡フラグを立てるのみ。
    /// 完全実装では依存キャッシュ多様体の全エントリを走査・クリアする。
    pub fn invalidate_applicability_cache(&mut self) {
        self.cache_invalidated = true;
    }
}

/// 管理者 fast-track を適用する。
///
/// `HumanTrustLogistic.score` を `TRUST_ADMIN_FAST_TRACK (0.80)` に強制固定し、
/// キャッシュ無効化フラグを立て、`TrustAuditLog` 配列に監査レコードを追加する。
///
/// RFC §8.2:
/// - 管理者 fast-track を適用した場合、その操作を TrustAuditLog に記録しなければならない (SHOULD)
pub fn apply_admin_fast_track(
    graph: &mut MemoizedGraph,
    actor_id: String,
    audit_log: &mut Vec<TrustAuditLog>,
    reason: Option<String>,
) {
    let old_value = graph.trust.human.score;
    graph.trust.human.score = constants::TRUST_ADMIN_FAST_TRACK;
    graph.invalidate_applicability_cache();
    audit_log.push(TrustAuditLog {
        graph_id: graph.id.clone(),
        event_type: TrustAuditEvent::AdminFastTrack,
        actor_id,
        old_value,
        new_value: constants::TRUST_ADMIN_FAST_TRACK,
        timestamp: SystemTime::now(),
        reason,
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    // ================================================================
    // 不変条件テスト T1〜T7
    // ================================================================

    /// T1: 呼び出し後、graph.trust.human.score が TRUST_ADMIN_FAST_TRACK (0.80) になる。
    #[test]
    fn t1_score_forced_to_admin_fast_track() {
        let mut graph = MemoizedGraph::new("g-001".into(), 0.50);
        let mut audit_log = Vec::new();

        apply_admin_fast_track(&mut graph, "admin-001".into(), &mut audit_log, None);

        assert_eq!(
            graph.trust.human.score,
            constants::TRUST_ADMIN_FAST_TRACK,
            "score must be forced to TRUST_ADMIN_FAST_TRACK (0.80)"
        );
    }

    /// T2: 監査ログ末尾の event_type が AdminFastTrack である。
    #[test]
    fn t2_audit_log_event_type_is_admin_fast_track() {
        let mut graph = MemoizedGraph::new("g-001".into(), 0.50);
        let mut audit_log = Vec::new();

        apply_admin_fast_track(&mut graph, "admin-001".into(), &mut audit_log, None);

        let last = audit_log.last().expect("audit log must not be empty");
        assert_eq!(
            last.event_type,
            TrustAuditEvent::AdminFastTrack,
            "last audit log event_type must be AdminFastTrack"
        );
    }

    /// T3: 監査ログの old_value が呼び出し前の score と一致する。
    #[test]
    fn t3_old_value_matches_previous_score() {
        let initial_score = 0.50;
        let mut graph = MemoizedGraph::new("g-001".into(), initial_score);
        let mut audit_log = Vec::new();

        apply_admin_fast_track(&mut graph, "admin-001".into(), &mut audit_log, None);

        let last = audit_log.last().expect("audit log must not be empty");
        assert_eq!(
            last.old_value, initial_score,
            "old_value must match the score before fast-track"
        );
    }

    /// T4: 監査ログの new_value が TRUST_ADMIN_FAST_TRACK と一致する。
    #[test]
    fn t4_new_value_is_admin_fast_track() {
        let mut graph = MemoizedGraph::new("g-001".into(), 0.50);
        let mut audit_log = Vec::new();

        apply_admin_fast_track(&mut graph, "admin-001".into(), &mut audit_log, None);

        let last = audit_log.last().expect("audit log must not be empty");
        assert_eq!(
            last.new_value,
            constants::TRUST_ADMIN_FAST_TRACK,
            "new_value must be TRUST_ADMIN_FAST_TRACK (0.80)"
        );
    }

    /// T5: 呼び出し後、cache_invalidated が true である。
    #[test]
    fn t5_cache_invalidated_flag_is_set() {
        let mut graph = MemoizedGraph::new("g-001".into(), 0.50);
        let mut audit_log = Vec::new();

        assert!(!graph.cache_invalidated, "cache must start as valid");
        apply_admin_fast_track(&mut graph, "admin-001".into(), &mut audit_log, None);
        assert!(
            graph.cache_invalidated,
            "cache must be invalidated after fast-track"
        );
    }

    /// T6: actor_id と reason が監査ログに正確に反映される。
    #[test]
    fn t6_actor_id_and_reason_are_recorded() {
        let mut graph = MemoizedGraph::new("g-001".into(), 0.50);
        let mut audit_log = Vec::new();
        let actor = "admin-002".to_string();
        let reason = Some("B2B contract approval".to_string());

        apply_admin_fast_track(&mut graph, actor.clone(), &mut audit_log, reason.clone());

        let last = audit_log.last().expect("audit log must not be empty");
        assert_eq!(last.actor_id, actor, "actor_id must be recorded");
        assert_eq!(last.reason, reason, "reason must be recorded");
    }

    /// T7: 異なる初期値 (0.50 と 0.30) の両方で old_value が正しく記録される。
    #[test]
    fn t7_old_value_correct_for_different_initial_scores() {
        let test_cases = [("g-50", 0.50_f64), ("g-30", 0.30_f64)];

        for (graph_id, initial_score) in &test_cases {
            let mut graph = MemoizedGraph::new(graph_id.to_string(), *initial_score);
            let mut audit_log = Vec::new();

            apply_admin_fast_track(&mut graph, "admin-001".into(), &mut audit_log, None);

            let last = audit_log.last().expect("audit log must not be empty");
            assert_eq!(
                last.old_value, *initial_score,
                "graph {}: old_value must match initial score {}",
                graph_id, initial_score
            );
        }
    }

    // ================================================================
    // 観測テスト OTS
    // ================================================================

    /// OTS-1: キャッシュ無効化の完全性。
    ///
    /// 1,000 回の apply_admin_fast_track 呼び出し後、常に cache_invalidated == true であること、
    /// および呼び出しレイテンシを計測する。
    #[test]
    fn ots1_cache_invalidation_completeness() {
        let n = 1_000;
        let mut audit_log = Vec::new();
        let mut results: Vec<(usize, u128)> = Vec::with_capacity(n);

        for i in 0..n {
            let mut graph = MemoizedGraph::new(format!("g-{:04}", i), 0.50);
            let start = std::time::Instant::now();
            apply_admin_fast_track(&mut graph, "admin-bulk".into(), &mut audit_log, None);
            let dt = start.elapsed().as_nanos();
            results.push((i, dt));
            assert!(
                graph.cache_invalidated,
                "iteration {}: cache must be invalidated",
                i
            );
        }

        let total_ns: u128 = results.iter().map(|(_, dt)| dt).sum();
        let avg_ns = total_ns / n as u128;
        let min_ns = results.iter().map(|(_, dt)| dt).min().unwrap();
        let max_ns = results.iter().map(|(_, dt)| dt).max().unwrap();

        println!(
            "OTS-1,n={},avg_ns={},min_ns={},max_ns={}",
            n, avg_ns, min_ns, max_ns
        );
        assert!(
            audit_log.len() == n,
            "must have exactly {} audit log entries, got {}",
            n,
            audit_log.len()
        );
        println!("OTS-1 PASS: all {} iterations completed, cache_invalidated=true", n);
    }

    /// OTS-2: TrustAuditLog 記録完全性。
    ///
    /// 10,000 回の連続発動で、記録された全レコードの event_type が
    /// AdminFastTrack であることを確認する。
    #[test]
    fn ots2_audit_log_record_completeness() {
        let n = 10_000;
        let mut graph = MemoizedGraph::new("g-perf".into(), 0.50);
        let mut audit_log = Vec::with_capacity(n);

        let start = std::time::Instant::now();
        for i in 0..n {
            apply_admin_fast_track(
                &mut graph,
                format!("admin-{:04}", i),
                &mut audit_log,
                None,
            );
        }
        let total_dt = start.elapsed();

        let records_mismatch: usize = audit_log
            .iter()
            .map(|log| {
                if log.event_type != TrustAuditEvent::AdminFastTrack {
                    1
                } else {
                    0
                }
            })
            .sum();

        let records_new_value_mismatch: usize = audit_log
            .iter()
            .map(|log| {
                if (log.new_value - constants::TRUST_ADMIN_FAST_TRACK).abs() > 1e-10 {
                    1
                } else {
                    0
                }
            })
            .sum();

        println!(
            "OTS-2,n={},records_added={},records_mismatch={},new_value_mismatch={},total_dt_ns={}",
            n,
            audit_log.len(),
            records_mismatch,
            records_new_value_mismatch,
            total_dt.as_nanos()
        );

        assert_eq!(
            audit_log.len(),
            n,
            "must have exactly {} records",
            n
        );
        assert_eq!(
            records_mismatch, 0,
            "all records must have event_type == AdminFastTrack"
        );
        assert_eq!(
            records_new_value_mismatch, 0,
            "all records must have new_value == TRUST_ADMIN_FAST_TRACK"
        );
        println!("OTS-2 PASS: records_added={}, all event_type=AdminFastTrack, all new_value=0.80", n);
    }
}
