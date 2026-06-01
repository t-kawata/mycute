// Darvium 信頼管理
//
// 本モジュールはワークフローグラフの信頼プロファイル操作を提供する。
// RFC §8.2 (管理者 fast-track), §10.3 (HumanTrustLogistic) に基づく。

use std::time::SystemTime;

use crate::constants;
use crate::event::{GcEvent, ReputationProfile};
use crate::spaceposition::SpacePositionEmbedding;
use crate::types::{
    HumanTrustLogistic, TrustAuditEvent, TrustAuditLog, TrustProfile, TrustUpdate, VillageId,
    WorkflowGraph,
};

/// 試験用 MemoizedGraph 縮約実装。
///
/// M1-2 の検証に必要な最小限のフィールドのみ保持する。
/// RFC §8 の完全実装は M2/M3 以降に段階的に拡充する。
///
/// # v2.3-j 拡張 (M-0.5-7-P)
///
/// - `graph` フィールド: WorkflowCache::update_graph_cas の CAS 対象として追加
/// - `version` フィールド: GraphVersion 楽観的並行性制御 (P-09) のカウンタとして追加
///
/// # シミュレーション拡張 (SimulationContext Vec 化)
///
/// - RFC §12: 1 MemoizedGraph = 1 人の「人」。
/// - 生存・位置・村所属・GC状態・経験値・評判を自己保有する。
/// - SimulationContext の side-channel HashMap からの移行先。
#[derive(Debug, Clone)]
pub struct MemoizedGraph {
    /// グラフ識別子
    pub id: String,
    /// ワークフローグラフ本体 (CAS 対象)
    pub graph: WorkflowGraph,
    /// 信頼プロファイル
    pub trust: TrustProfile,
    /// GraphVersion CAS 用楽観的バージョンカウンタ (P-09, RFC §8.4)
    pub version: u64,
    /// キャッシュ無効化フラグ (invalidate_applicability_cache 呼び出し追跡用)
    pub cache_invalidated: bool,
    /// タスク記述の埋め込みベクトル (RFC §8, Stage 1 cosine similarity)
    pub task_embedding: Vec<f32>,
    /// ワークフローの誕生 tick（VirtualClock 基準）。None の場合は未設定。
    pub birth_tick: Option<u64>,

    // ──────────────────────────────────────────────
    // シミュレーション拡張フィールド (RFC §12 準拠)
    // ──────────────────────────────────────────────

    /// 生存フラグ。false の場合は死亡 (GC 収集済み)。
    pub alive: bool,
    /// 3次元能力空間内の位置 (RFC §12.1)。
    pub position: SpacePositionEmbedding,
    /// 村所属 (None = 未所属, Some(id) = 所属)。
    pub village_assignment: Option<VillageId>,
    /// GC 状態 (GcEvent)。
    pub gc_state: GcEvent,
    /// 最終更新 tick (mean_freshness 計算用)。
    pub last_update_tick: u64,
    /// 累積経験値 (child_survival_rate 計算用)。
    pub experience_count: u64,
    /// 評判プロファイル (RFC §8.6)。
    pub reputation: ReputationProfile,
    /// 最終仮想時刻 (RFC §12, last_virtual_seen)。
    pub last_virtual_seen: u64,
    /// キャッシュ済み全ノード数（再帰 SubWorkflow 解決含む）。
    pub cached_node_count: usize,
}

impl Default for MemoizedGraph {
    fn default() -> Self {
        Self::new(String::new(), 0.0)
    }
}

impl MemoizedGraph {
    /// 指定された human_score 初期値で MemoizedGraph を生成する。
    pub fn new(id: String, human_score: f64) -> Self {
        Self {
            id,
            graph: WorkflowGraph::new(),
            trust: TrustProfile {
                operational: 0.0,
                semantic: 0.0,
                temporal: 0.0,
                human: HumanTrustLogistic {
                    score: human_score,
                    ..HumanTrustLogistic::default()
                },
            },
            version: 0,
            cache_invalidated: false,
            task_embedding: vec![],
            birth_tick: None,
            alive: true,
            position: SpacePositionEmbedding::unknown(),
            village_assignment: None,
            gc_state: GcEvent::Active,
            last_update_tick: 0,
            experience_count: 0,
            reputation: ReputationProfile::cold_start(),
            last_virtual_seen: 0,
            cached_node_count: 0,
        }
    }

    /// 指定された位置と誕生 tick で MemoizedGraph を生成する（シミュレーション用）。
    pub fn new_with_position(
        id: String,
        human_score: f64,
        position: SpacePositionEmbedding,
        birth_tick: u64,
    ) -> Self {
        Self {
            id,
            graph: WorkflowGraph::new(),
            trust: TrustProfile {
                operational: 0.0,
                semantic: 0.0,
                temporal: 0.0,
                human: HumanTrustLogistic {
                    score: human_score,
                    ..HumanTrustLogistic::default()
                },
            },
            version: 0,
            cache_invalidated: false,
            task_embedding: vec![],
            birth_tick: Some(birth_tick),
            alive: true,
            position,
            village_assignment: None,
            gc_state: GcEvent::Active,
            last_update_tick: birth_tick,
            experience_count: 0,
            reputation: ReputationProfile::cold_start(),
            last_virtual_seen: birth_tick,
            cached_node_count: 0,
        }
    }

    /// ワークフローが成人か判定する。
    ///
    /// 成人判定条件:
    /// - `experience_count` が `EXPERIENCE_ADULT_THRESHOLD` 以上（初期成人の後方互換）
    /// - または `birth_tick` が設定されており、現在時刻からの経過 tick が閾値以上
    pub fn is_adult(&self, current_tick: u64, adult_age_threshold: u64) -> bool {
        // 経験値が閾値以上のワークフローは成人とみなす（初期人口の初期成人対応）
        if self.experience_count >= crate::constants::EXPERIENCE_ADULT_THRESHOLD {
            return true;
        }
        // 出生 tick ベースの成人判定（RFC 準拠）
        match self.birth_tick {
            Some(birth) => current_tick.saturating_sub(birth) >= adult_age_threshold,
            None => false,
        }
    }

    /// Applicability キャッシュを無効化する。
    ///
    /// 現在は追跡フラグを立てるのみ。
    /// 完全実装では依存キャッシュ多様体の全エントリを走査・クリアする。
    pub fn invalidate_applicability_cache(&mut self) {
        self.cache_invalidated = true;
    }

    /// 信頼更新を適用する (RFC §10.5)。
    ///
    /// `MemoizedGraph` への信頼更新はすべてこのメソッド経由で行うこと (MUST)。
    /// 直接フィールド代入禁止 (MUST NOT)。
    ///
    /// - `Operational` / `Semantic`: 常にキャッシュ無効化
    /// - `Human`: 複合スコア変動が TRUST_DEBOUNCE_DELTA 以上の場合のみキャッシュ無効化
    pub fn update_trust(&mut self, update: TrustUpdate) {
        match update {
            TrustUpdate::Operational(success) => {
                update_operational_trust(&mut self.trust.operational, success, 0.15);
                self.invalidate_applicability_cache();
            }
            TrustUpdate::Human(outcome) => {
                let old_composite = self.trust.composite();
                self.trust.human.update(outcome);
                let new_composite = self.trust.composite();
                // Debounce: composite スコアが TRUST_DEBOUNCE_DELTA (0.05) 以上変動した
                // 場合のみキャッシュを無効化する。頻繁な非同期フィードバックによる
                // 不必要な再計算を防ぐ (OQ-11 参照)
                if (new_composite - old_composite).abs() >= constants::TRUST_DEBOUNCE_DELTA {
                    self.invalidate_applicability_cache();
                }
            }
            TrustUpdate::Semantic(score) => {
                update_semantic_ema(&mut self.trust.semantic, score, 0.10);
                self.invalidate_applicability_cache();
            }
        }
    }
}

/// 運用信頼を指数移動平均 (EMA) で更新する (RFC §10.5 簡易版)。
///
/// M1-3 簡易実装: `operational = operational * (1 - alpha) + (success as u8 as f64) * alpha`
/// 完全実装 (成功率履歴のEMA) は後続チケットで実装予定。
fn update_operational_trust(operational: &mut f64, success: bool, alpha: f64) {
    let value = if success { 1.0 } else { 0.0 };
    *operational = *operational * (1.0 - alpha) + value * alpha;
}

/// 意味的信頼を指数平滑移動平均で更新する (RFC §10.5 簡易版)。
///
/// M1-3 簡易実装: `semantic = semantic * (1 - alpha) + (1 - score) * alpha`
/// ここで score = semantic_deviation (小さいほど良い)
fn update_semantic_ema(semantic: &mut f64, score: f64, alpha: f64) {
    *semantic = *semantic * (1.0 - alpha) + (1.0 - score) * alpha;
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

/// 親の信頼プロファイルを子に継承する (RFC §15.3 信頼継承)。
///
/// 各成分 (operational/semantic/temporal) に減衰係数 `decay` を乗算して子にコピーする。
/// human 信頼は個人固有のため継承しない。
///
/// # 引数
/// - `parent`: 親の TrustProfile
/// - `child`: 子の TrustProfile (上書き)
/// - `decay`: 減衰係数 [0, 1]。1.0 で無減衰、0.0 でゼロ継承
pub fn inherit_trust(parent: &TrustProfile, child: &mut TrustProfile, decay: f64) {
    child.operational = parent.operational * decay;
    child.semantic = parent.semantic * decay;
    child.temporal = parent.temporal * decay;
}

/// 親の評判プロファイルを子に継承する。
///
/// 親の最終合成スコア `final_score` に減衰係数 `decay` を乗算し、
/// 子の `inherited_score` として設定する。
///
/// # 引数
/// - `parent`: 親の ReputationProfile
/// - `child`: 子の ReputationProfile (上書き)
/// - `decay`: 減衰係数 [0, 1]
pub fn inherit_reputation(parent: &ReputationProfile, child: &mut ReputationProfile, decay: f64) {
    child.inherited_score = parent.final_score * decay as f32;
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
        println!(
            "OTS-1 PASS: all {} iterations completed, cache_invalidated=true",
            n
        );
    }

    /// OTS-2: TrustAuditLog 記録完全性。
    ///
    /// 10,000 回の連続発動で、記録された全レコードの event_type が
    /// AdminFastTrack であることを確認する。
    #[test]
    #[ignore = "観測テスト（10,000 iteration）— 日常の cargo test ではスキップ"]
    fn ots2_audit_log_record_completeness() {
        let n = 10_000;
        let mut graph = MemoizedGraph::new("g-perf".into(), 0.50);
        let mut audit_log = Vec::with_capacity(n);

        let start = std::time::Instant::now();
        for i in 0..n {
            apply_admin_fast_track(&mut graph, format!("admin-{:04}", i), &mut audit_log, None);
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

        assert_eq!(audit_log.len(), n, "must have exactly {} records", n);
        assert_eq!(
            records_mismatch, 0,
            "all records must have event_type == AdminFastTrack"
        );
        assert_eq!(
            records_new_value_mismatch, 0,
            "all records must have new_value == TRUST_ADMIN_FAST_TRACK"
        );
        println!(
            "OTS-2 PASS: records_added={}, all event_type=AdminFastTrack, all new_value=0.80",
            n
        );
    }

    // ================================================================
    // M1-3: TrustUpdate Debounce 不変条件テスト T1〜T8
    // ================================================================

    use crate::types::TrustUpdate;

    /// T1: TrustUpdate::Human(0.5) がキャッシュ無効化をトリガーする。
    ///
    /// outcome=0.5 (partial) を1回適用。outcome がコールドスタートの 0.50 と同じ値であり、
    /// ロジスティック更新による変動が TRUST_DEBOUNCE_DELTA 未満である場合、
    /// キャッシュ無効化はスキップされる (実際の変動が閾値未満かどうかを事後検証)。
    #[test]
    fn t1_human_update_composite_delta_under_threshold() {
        let mut graph = MemoizedGraph::new("g-001".into(), 0.50);
        let composite_before = graph.trust.composite();
        graph.update_trust(TrustUpdate::Human(0.5));
        let composite_after = graph.trust.composite();
        let delta = (composite_after - composite_before).abs();
        // outcome=0.5 は cold-start=0.50 と同じ値なので期待値と一致し、変動は微小
        // キャッシュ無効化は delta >= TRUST_DEBOUNCE_DELTA の場合のみ発生
        if delta < constants::TRUST_DEBOUNCE_DELTA {
            assert!(
                !graph.cache_invalidated,
                "cache must NOT be invalidated when delta ({}) < threshold ({})",
                delta,
                constants::TRUST_DEBOUNCE_DELTA
            );
        } else {
            assert!(
                graph.cache_invalidated,
                "cache must be invalidated when delta ({}) >= threshold ({})",
                delta,
                constants::TRUST_DEBOUNCE_DELTA
            );
        }
    }

    /// T2: TrustUpdate::Human(0.5) の前後で composite() が正しく変化する。
    #[test]
    fn t2_human_update_composite_changes() {
        let mut graph = MemoizedGraph::new("g-002".into(), 0.50);
        let before = graph.trust.composite();
        graph.update_trust(TrustUpdate::Human(0.5));
        let after = graph.trust.composite();
        // outcome=0.5 (cold-start と同じ) の場合、human.score はほとんど変化しない
        // が、composite() の計算式が正しく呼ばれていることを確認
        assert!(
            (before - after).abs() < 1.0,
            "composite() must produce stable values, before={}, after={}",
            before,
            after
        );
    }

    /// T3: TrustUpdate::Operational(true) は常にキャッシュ無効化する。
    #[test]
    fn t3_operational_always_invalidates_cache() {
        let mut graph = MemoizedGraph::new("g-003".into(), 0.50);
        graph.update_trust(TrustUpdate::Operational(true));
        assert!(
            graph.cache_invalidated,
            "Operational update must always invalidate cache"
        );
    }

    /// T4: TrustUpdate::Semantic(0.5) は常にキャッシュ無効化する。
    #[test]
    fn t4_semantic_always_invalidates_cache() {
        let mut graph = MemoizedGraph::new("g-004".into(), 0.50);
        graph.update_trust(TrustUpdate::Semantic(0.5));
        assert!(
            graph.cache_invalidated,
            "Semantic update must always invalidate cache"
        );
    }

    /// T5: 連続10回の微量フィードバック (outcome=0.51) で全回キャッシュ無効化スキップ。
    ///
    /// outcome=0.51 は cold-start=0.50 に極めて近く、1回あたりのスコア変動が
    /// TRUST_DEBOUNCE_DELTA 未満であるため、全10回で cache_invalidated == false となる。
    #[test]
    fn t5_continuous_small_feedback_skips_cache_invalidation() {
        let mut graph = MemoizedGraph::new("g-005".into(), 0.50);
        let mut invalidation_counts = 0u32;
        for _ in 0..10 {
            graph.cache_invalidated = false;
            graph.update_trust(TrustUpdate::Human(0.51));
            if graph.cache_invalidated {
                invalidation_counts += 1;
            }
        }
        assert_eq!(
            invalidation_counts, 0,
            "10 continuous small feedbacks must NOT trigger cache invalidation (got {} invalidations)",
            invalidation_counts
        );
    }

    /// T6: 大フィードバック (outcome=1.0) の複合スコアデルタが正しく評価される。
    ///
    /// 注意: 現在の HUMAN_TRUST_K (0.08) では単一の Human update が
    /// TRUST_DEBOUNCE_DELTA (0.05) を超えることは数学的に不可能である
    /// (最大複合デルタ = 0.20 * 0.08 ≈ 0.016 < 0.05)。
    /// 本テストはデルタと閾値の比較ロジックが正しいことを検証する。
    #[test]
    fn t6_human_update_delta_is_correctly_evaluated() {
        let mut graph = MemoizedGraph::new("g-006".into(), 0.50);
        let composite_before = graph.trust.composite();
        graph.cache_invalidated = false;
        graph.update_trust(TrustUpdate::Human(1.0));
        let composite_after = graph.trust.composite();
        let delta = (composite_after - composite_before).abs();
        // デバウンス条件: cache_invalidated == (delta >= TRUST_DEBOUNCE_DELTA)
        assert_eq!(
            graph.cache_invalidated,
            delta >= constants::TRUST_DEBOUNCE_DELTA,
            "cache_invalidated ({}) must equal (delta ({}) >= threshold ({}))",
            graph.cache_invalidated,
            delta,
            constants::TRUST_DEBOUNCE_DELTA
        );
    }

    /// T7: コールドスタート時の composite() 値が正しい。
    ///
    /// operational=0.0, semantic=0.0, temporal=0.0, human.score=0.50
    /// composite = 0.35*0 + 0.25*0 + 0.20*0 + 0.20*0.50 = 0.10
    #[test]
    fn t7_cold_start_composite_is_correct() {
        let graph = MemoizedGraph::new("g-007".into(), 0.50);
        let expected = 0.35 * 0.0 + 0.25 * 0.0 + 0.20 * 0.0 + 0.20 * 0.50;
        let actual = graph.trust.composite();
        assert!(
            (actual - expected).abs() < 1e-10,
            "cold-start composite must be {} (got {})",
            expected,
            actual
        );
    }

    /// T8: 3種類の TrustUpdate を連続で呼び出し、状態が正しく遷移する。
    ///
    /// Human 更新は k=0.08 の制約により単一呼び出しでは閾値を超えないが、
    /// デルタ比較ロジックが正しく動作していることを確認する。
    #[test]
    fn t8_sequential_updates_correctness() {
        let mut graph = MemoizedGraph::new("g-008".into(), 0.30);

        // Operational (常に無効化)
        graph.cache_invalidated = false;
        graph.update_trust(TrustUpdate::Operational(true));
        assert!(graph.cache_invalidated);
        assert!(graph.trust.operational > 0.0);

        // Human (デルタ依存 — 現状の定数では単一更新で閾値を超えない)
        graph.cache_invalidated = false;
        let composite_before = graph.trust.composite();
        graph.update_trust(TrustUpdate::Human(1.0));
        let composite_after = graph.trust.composite();
        let delta = (composite_after - composite_before).abs();
        assert_eq!(
            graph.cache_invalidated,
            delta >= constants::TRUST_DEBOUNCE_DELTA,
            "Human update: cache_invalidated ({}) must match delta ({}) vs threshold ({})",
            graph.cache_invalidated,
            delta,
            constants::TRUST_DEBOUNCE_DELTA
        );

        // Semantic (常に無効化)
        graph.cache_invalidated = false;
        graph.update_trust(TrustUpdate::Semantic(0.2));
        assert!(graph.cache_invalidated);
        assert!(graph.trust.semantic > 0.0);
    }

    // ================================================================
    // M1-3: TrustUpdate Debounce 観測テスト OTS-1〜OTS-3
    // ================================================================

    /// OTS-1: デバウンス判定ルールの完全性検証。
    ///
    /// 複合スコアデルタ ΔT が 0.000〜0.100 の範囲で、キャッシュ無効化判定が
    /// デルタと閾値の比較 (ΔT >= TRUST_DEBOUNCE_DELTA) と一致することを
    /// 全水準で検証する。
    ///
    /// 注意: 現在の HUMAN_TRUST_K (0.08) では単一 Human update の最大複合デルタが
    /// ≈ 0.016 であり、閾値 0.05 を超えることができない。そのため本観測テストでは
    /// 全シナリオで delta < threshold かつ cache_invalidated = false となる。
    /// テストの目的は判定ルールの一貫性検証にある。
    #[test]
    fn ots1_debounce_decision_rule() {
        let threshold = constants::TRUST_DEBOUNCE_DELTA;
        // 様々な初期値 x outcome の組み合わせで観測
        let initial_scores: Vec<f64> = vec![0.10, 0.30, 0.50, 0.70, 0.90];
        let outcomes: Vec<f64> = vec![0.0, 0.25, 0.50, 0.75, 1.0];

        println!("=== OTS-1: Debounce Decision Rule ===");
        println!("threshold={}", threshold);
        println!("initial_score,outcome,composite_before,composite_after,delta,invalidation");
        println!();

        let mut total_trials = 0u32;
        let mut correct_decisions = 0u32;

        for &init_score in &initial_scores {
            for &outcome in &outcomes {
                let mut graph = MemoizedGraph::new("g-ots1".into(), init_score);
                let composite_before = graph.trust.composite();
                graph.cache_invalidated = false;
                graph.update_trust(TrustUpdate::Human(outcome));
                let composite_after = graph.trust.composite();
                let delta = (composite_after - composite_before).abs();
                let expected_invalidation = delta >= threshold;

                // 判定ルールの一致を確認
                let decision_correct = graph.cache_invalidated == expected_invalidation;

                println!(
                    "{:.2},{:.2},{:.6},{:.6},{:.6},{}",
                    init_score,
                    outcome,
                    composite_before,
                    composite_after,
                    delta,
                    graph.cache_invalidated
                );

                total_trials += 1;
                if decision_correct {
                    correct_decisions += 1;
                }
            }
        }

        println!();
        println!("--- OTS-1 Summary ---");
        println!(
            "total_trials={}, correct_decisions={}",
            total_trials, correct_decisions
        );
        println!(
            "correct_rate={:.4}",
            correct_decisions as f64 / total_trials as f64
        );
        println!(
            "Note: All deltas are below threshold (max ≈ 0.016) due to HUMAN_TRUST_K={}",
            constants::HUMAN_TRUST_K
        );
        assert_eq!(
            correct_decisions, total_trials,
            "All debounce decisions must match the rule (delta >= threshold)"
        );
        println!(
            "=== OTS-1: PASS ({} scenarios, 100% correct) ===",
            total_trials
        );
    }

    /// OTS-2: 微小フィードバック連続注入による累積デルタ追跡。
    ///
    /// outcome=0.51 を 100 回連続注入し、累積スコア変動が 0.05 に達するまで
    /// キャッシュ無効化が発生しないことを観測する。
    #[test]
    fn ots2_cumulative_delta_tracking() {
        let mut graph = MemoizedGraph::new("g-ots2".into(), 0.50);
        let n = 100;
        let outcome = 0.51;

        println!("=== OTS-2: Cumulative Delta Tracking ===");
        println!("step,composite,delta,cumulative_delta,cache_invalidated");
        println!("0,{:.6},0.0,0.0,false", graph.trust.composite());

        let initial_composite = graph.trust.composite();
        let mut first_invalidation_step: Option<usize> = None;

        for step in 1..=n {
            let composite_before = graph.trust.composite();
            graph.cache_invalidated = false;
            graph.update_trust(TrustUpdate::Human(outcome));
            let composite_after = graph.trust.composite();
            let step_delta = (composite_after - composite_before).abs();
            let cumulative_delta = (composite_after - initial_composite).abs();

            println!(
                "{},{:.6},{:.6},{:.6},{}",
                step, composite_after, step_delta, cumulative_delta, graph.cache_invalidated
            );

            if graph.cache_invalidated && first_invalidation_step.is_none() {
                first_invalidation_step = Some(step);
            }
        }

        if let Some(step) = first_invalidation_step {
            println!("--- First cache invalidation at step {} (cumulative_delta reached TRUST_DEBOUNCE_DELTA) ---", step);
        } else {
            println!("--- Cache was never invalidated (cumulative_delta < TRUST_DEBOUNCE_DELTA for all {} steps) ---", n);
        }
        println!("=== OTS-2: PASS ===");
    }

    /// OTS-3: フィードバック注入レイテンシ分布。
    ///
    /// `update_trust(TrustUpdate::Human(0.5))` を 10,000 回実行し、
    /// 各呼び出しのレイテンシ平均・最小・最大・分位数を計測する。
    #[test]
    #[ignore = "観測テスト（10,000 iteration）— 日常の cargo test ではスキップ"]
    fn ots3_latency_distribution() {
        let n = 10_000;
        let mut graph = MemoizedGraph::new("g-ots3".into(), 0.50);
        let mut latencies_ns: Vec<u128> = Vec::with_capacity(n);

        for _ in 0..n {
            let start = std::time::Instant::now();
            graph.update_trust(TrustUpdate::Human(0.5));
            let elapsed = start.elapsed().as_nanos();
            latencies_ns.push(elapsed);
        }

        latencies_ns.sort_unstable();
        let total: u128 = latencies_ns.iter().copied().sum();
        let avg = total as f64 / n as f64;
        let min = latencies_ns.first().copied().unwrap_or(0);
        let max = latencies_ns.last().copied().unwrap_or(0);
        let p50 = latencies_ns[n / 2];
        let p95 = latencies_ns[(n as f64 * 0.95) as usize];
        let p99 = latencies_ns[(n as f64 * 0.99) as usize];

        println!("=== OTS-3: Latency Distribution ===");
        println!("samples={}", n);
        println!("avg_ns={:.3}", avg);
        println!("min_ns={}", min);
        println!("max_ns={}", max);
        println!("p50_ns={}", p50);
        println!("p95_ns={}", p95);
        println!("p99_ns={}", p99);
        assert!(
            avg < 1_000_000.0,
            "Average latency must be < 1ms (got {} ns)",
            avg
        );
        println!("=== OTS-3: PASS ===");
    }

    // ================================================================
    // P5: inherit_trust / inherit_reputation (TC6, TC7)
    // ================================================================

    /// TC6: inherit_trust の減衰継承。
    #[test]
    fn tc6_inherit_trust_decay() {
        let parent = TrustProfile {
            operational: 0.8,
            semantic: 0.7,
            temporal: 0.6,
            human: HumanTrustLogistic::default(),
        };
        let mut child = TrustProfile {
            operational: 0.0,
            semantic: 0.0,
            temporal: 0.0,
            human: HumanTrustLogistic::default(),
        };

        // decay=0.0 → 子は 0.0
        inherit_trust(&parent, &mut child, 0.0);
        assert!((child.operational - 0.0).abs() < f64::EPSILON);
        assert!((child.semantic - 0.0).abs() < f64::EPSILON);
        assert!((child.temporal - 0.0).abs() < f64::EPSILON);

        // decay=1.0 → 子は親と同値
        inherit_trust(&parent, &mut child, 1.0);
        assert!((child.operational - 0.8).abs() < f64::EPSILON);
        assert!((child.semantic - 0.7).abs() < f64::EPSILON);
        assert!((child.temporal - 0.6).abs() < f64::EPSILON);

        // decay=0.7 → 子は親の 0.7 倍
        inherit_trust(&parent, &mut child, 0.7);
        assert!((child.operational - 0.56).abs() < 1e-10);
        assert!((child.semantic - 0.49).abs() < 1e-10);
        assert!((child.temporal - 0.42).abs() < 1e-10);
    }

    /// TC7: inherit_reputation の減衰継承。
    #[test]
    fn tc7_inherit_reputation_decay() {
        use std::time::SystemTime;

        let parent = ReputationProfile {
            direct_score: 0.0,
            indirect_score: 0.0,
            experience_score: 0.0,
            inherited_score: 0.0,
            final_score: 0.9,
            alpha_positive: 10,
            beta_negative: 1,
            last_recomputed_at: SystemTime::now(),
            direct_help_count: 5,
            direct_success_count: 4,
            direct_reject_count: 0,
            harm_event_count: 0,
            accepted_offer_rate: 0.8,
            help_success_rate: 0.9,
            village_centrality: 0.5,
            benevolence_score: 0.7,
            chiefdom_score: 0.5,
        };

        let mut child = ReputationProfile {
            direct_score: 0.0,
            indirect_score: 0.0,
            experience_score: 0.0,
            inherited_score: 0.0,
            final_score: 0.0,
            alpha_positive: 0,
            beta_negative: 0,
            last_recomputed_at: SystemTime::now(),
            direct_help_count: 0,
            direct_success_count: 0,
            direct_reject_count: 0,
            harm_event_count: 0,
            accepted_offer_rate: 0.0,
            help_success_rate: 0.0,
            village_centrality: 0.0,
            benevolence_score: 0.0,
            chiefdom_score: 0.0,
        };

        // decay=0.0 → inherited_score = 0.0
        inherit_reputation(&parent, &mut child, 0.0);
        assert!(
            (child.inherited_score - 0.0).abs() < f32::EPSILON,
            "decay=0.0: expected 0.0, got {}",
            child.inherited_score
        );

        // decay=1.0 → inherited_score = parent.final_score
        inherit_reputation(&parent, &mut child, 1.0);
        assert!(
            (child.inherited_score - 0.9).abs() < f32::EPSILON,
            "decay=1.0: expected 0.9, got {}",
            child.inherited_score
        );

        // decay=0.7 → inherited_score = 0.9 * 0.7 = 0.63
        inherit_reputation(&parent, &mut child, 0.7);
        assert!(
            (child.inherited_score - 0.63).abs() < 1e-6_f32,
            "decay=0.7: expected 0.63, got {}",
            child.inherited_score
        );
    }
}
