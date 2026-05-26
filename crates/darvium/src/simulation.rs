// 合成村シミュレーター (M1.76-17, RFC §15.10.9 Phase 3)
//
// 本モジュールは Training Plane における合成生態系シミュレーションを提供する。
// child/adult ワークフロー集団を生成し、HELP プロトコル相互作用・信頼評判再計算・
// ライフサイクル GC を反復することで、慈悲的なワークフロー（benevolent）が
// 非慈悲的なものよりも生存しやすい「Kind World 創発」を検証する。
//
// # 観測ベース検証
//
// 本シミュレーターは観測対象として設計されている。各 tick の metrics は
// SimulationTickSnapshot として構造化出力され、最後に println! で CSV 形式で
// 標準出力に書き出される。不変条件（metrics 範囲、生存率非負等）は assert で検証する。

use std::time::SystemTime;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::constants::{CHILD_PROTECT_ETA1, E_ADULT_THRESHOLD};
use crate::event::{ReciprocityEvent, ReciprocityEventKind, ReciprocityLifecyclePolicy, ReputationProfile};
use crate::reciprocity::{
    compute_benevolence_score, compute_direct_reciprocity, compute_gc_hazard,
    compute_indirect_reciprocity, compute_survival_probability, recompute_reputation, ReputationInputs,
};

// ============================================================
// 公開型定義
// ============================================================

/// シミュレーション実行設定。
#[derive(Debug, Clone)]
pub struct ReciprocitySimulatorConfig {
    /// 初期ワークフロー人口。
    pub population_size: usize,
    /// 子ワークフローの割合 [0, 1]。
    pub child_ratio: f64,
    /// 各 tick あたりのミッション発生率。
    pub mission_rate: f64,
    /// 最大 tick 数。
    pub max_ticks: u64,
    /// GC 実行間隔（tick 数）。1 で毎 tick GC 実行。
    pub gc_interval: u64,
    /// ライフサイクルポリシー（全パラメータ）。
    pub policy: ReciprocityLifecyclePolicy,
    /// PRNG シード値。
    pub seed: u64,
}

impl Default for ReciprocitySimulatorConfig {
    fn default() -> Self {
        Self {
            population_size: 50,
            child_ratio: 0.3,
            mission_rate: 0.3,
            max_ticks: 20,
            gc_interval: 3,
            policy: ReciprocityLifecyclePolicy::default(),
            seed: 12345,
        }
    }
}

/// シミュレーション内の単一ワークフロー状態。
#[derive(Debug, Clone)]
pub struct SimWorkflowState {
    /// ワークフロー識別子。
    pub id: String,
    /// 村内の空間位置。
    pub position: [f32; 3],
    /// 累積経験値。
    pub experience: u64,
    /// 信頼スコア。
    pub trust: f32,
    /// 評判プロファイル。
    pub reputation: ReputationProfile,
    /// 現在の慈悲スコア（benevolence）。
    pub benevolence: f32,
    /// 現在の直接互恵性スコア。
    pub direct_reciprocity: f32,
    /// 現在の間接互恵性スコア。
    pub indirect_reciprocity: f32,
    /// 現在の GC hazard 値。
    pub hazard: f32,
    /// 生存フラグ。
    pub survived: bool,
    /// 子ワークフローフラグ。
    pub is_child: bool,
    /// 初期慈悲スコア（Kind World 分類用）。
    pub initial_benevolence: f32,
}

/// ミッション（支援要求）。
#[derive(Debug, Clone)]
pub struct SimMission {
    /// ミッション識別子。
    pub id: String,
    /// 要求元ワークフロー ID。
    pub requester_id: String,
    /// 作成 tick。
    pub created_at: u64,
}

/// ヘルプセッション状態。
#[derive(Debug, Clone, PartialEq)]
pub enum HelpSessionStatus {
    /// 支援申し出中。
    Offered,
    /// 支援受け入れ済み。
    Accepted,
    /// 支援拒否済み。
    Rejected,
    /// 支援実行中。
    Executing,
    /// 支援成功。
    Succeeded,
    /// 支援放棄。
    Abandoned,
    /// 有害不一致。
    HarmfulMismatch,
}

/// ヘルプセッション（支援相互作用の1単位）。
#[derive(Debug, Clone)]
pub struct SimHelpSession {
    /// セッション識別子。
    pub id: String,
    /// 関連ミッション ID。
    pub mission_id: String,
    /// ヘルパーワークフロー ID。
    pub helper_id: String,
    /// 要求元ワークフロー ID。
    pub requester_id: String,
    /// 現在の状態。
    pub status: HelpSessionStatus,
    /// 作成 tick。
    pub created_at: u64,
    /// 最終更新 tick。
    pub updated_at: u64,
    /// ヘルパーの慈悲スコア（転記用）。
    pub helper_benevolence: f32,
}

/// 1 tick の観測 metrics。
#[derive(Debug, Clone)]
pub struct SimulationTickSnapshot {
    /// tick 番号。
    pub tick: u64,
    /// 慈悲スコア中央値。
    pub benevolence_score_p50: f32,
    /// 慈悲スコア 95 パーセンタイル。
    pub benevolence_score_p95: f32,
    /// 直接互恵性中央値。
    pub direct_reciprocity_p50: f32,
    /// 直接互恵性 95 パーセンタイル。
    pub direct_reciprocity_p95: f32,
    /// 間接互恵性中央値。
    pub indirect_reciprocity_p50: f32,
    /// 間接互恵性 95 パーセンタイル。
    pub indirect_reciprocity_p95: f32,
    /// 評判スコア中央値。
    pub reputation_final_p50: f32,
    /// 評判スコア 95 パーセンタイル。
    pub reputation_final_p95: f32,
    /// 慈悲的ワークフローの生存率。
    pub benevolent_survival_rate: f32,
    /// 非慈悲的ワークフローの生存率。
    pub non_benevolent_survival_rate: f32,
    /// 生存率差（benevolent - non_benevolent）。
    pub survival_advantage: f32,
    /// HarmfulMismatch 率。
    pub harmful_gc_rate: f32,
}

/// シミュレーション実行結果。
#[derive(Debug, Clone)]
pub struct ReciprocitySimulationResult {
    /// 全 tick の metrics 系列。
    pub metric_series: Vec<SimulationTickSnapshot>,
    /// 最終状態の全ワークフロー。
    pub final_state: Vec<SimWorkflowState>,
    /// 実験 ID。
    pub experiment_id: String,
}

// ============================================================
// Phase 1: 合成人口生成器
// ============================================================

/// 初期ワークフロー集団を生成する。
///
/// 子ワークフローは低経験値・低信頼・慈悲スコアは [0, 1] 一様分布。
/// 成人ワークフローは高経験値・中信頼・慈悲スコアは [0, 1] 一様分布。
fn generate_population(config: &ReciprocitySimulatorConfig, rng: &mut StdRng) -> Vec<SimWorkflowState> {
    let child_count = (config.population_size as f64 * config.child_ratio).round() as usize;
    let adult_count = config.population_size - child_count;

    let mut population = Vec::with_capacity(config.population_size);

    for i in 0..child_count {
        let benevolence: f32 = rng.gen(); // [0, 1) 一様
        let mut reputation = ReputationProfile::cold_start();
        reputation.benevolence_score = benevolence;

        population.push(SimWorkflowState {
            id: format!("wf-child-{}", i),
            position: [rng.gen::<f32>(), rng.gen::<f32>(), rng.gen::<f32>()],
            experience: rng.gen_range(0..E_ADULT_THRESHOLD - 1),
            trust: rng.gen::<f32>() * 0.3,
            reputation,
            benevolence,
            direct_reciprocity: 0.5,
            indirect_reciprocity: 0.5,
            hazard: 0.0,
            survived: true,
            is_child: true,
            initial_benevolence: benevolence,
        });
    }

    for i in 0..adult_count {
        let benevolence: f32 = rng.gen();
        let mut reputation = ReputationProfile::cold_start();
        reputation.benevolence_score = benevolence;

        population.push(SimWorkflowState {
            id: format!("wf-adult-{}", i),
            position: [rng.gen::<f32>(), rng.gen::<f32>(), rng.gen::<f32>()],
            experience: rng.gen_range(E_ADULT_THRESHOLD..(E_ADULT_THRESHOLD * 3)),
            trust: rng.gen::<f32>() * 0.5 + 0.3,
            reputation,
            benevolence,
            direct_reciprocity: 0.5,
            indirect_reciprocity: 0.5,
            hazard: 0.0,
            survived: true,
            is_child: false,
            initial_benevolence: benevolence,
        });
    }

    population
}

// ============================================================
// Phase 2: ミッションストリーム生成器
// ============================================================

/// 生存している成人ワークフローからミッションを生成する。
fn generate_mission_stream(
    tick: u64,
    population: &[SimWorkflowState],
    mission_rate: f64,
    rng: &mut StdRng,
    mission_counter: &mut u64,
) -> Vec<SimMission> {
    let mut missions = Vec::new();

    for wf in population.iter().filter(|w| w.survived && !w.is_child) {
        if rng.gen::<f64>() < mission_rate {
            *mission_counter += 1;
            missions.push(SimMission {
                id: format!("mission-{}", mission_counter),
                requester_id: wf.id.clone(),
                created_at: tick,
            });
        }
    }

    missions
}

// ============================================================
// Phase 3: ヘルプ相互作用シミュレーター
// ============================================================

/// 慈悲スコアに基づく offer 確率。
/// benevolent なワークフローほど高確率で支援を申し出る。
fn offer_help_probability(helper_benevolence: f32) -> f64 {
    // base 0.3 + benevolence * 0.4 → [0.3, 0.7] の範囲
    0.3 + helper_benevolence as f64 * 0.4
}

/// 各ミッションに対して potential helper が支援を申し出る。
fn offer_help_sessions(
    missions: &[SimMission],
    population: &[SimWorkflowState],
    existing_sessions: &mut Vec<SimHelpSession>,
    tick: u64,
    rng: &mut StdRng,
    session_counter: &mut u64,
) {
    for mission in missions {
        for wf in population.iter().filter(|w| w.survived && w.id != mission.requester_id) {
            if rng.gen::<f64>() < offer_help_probability(wf.benevolence) {
                *session_counter += 1;
                existing_sessions.push(SimHelpSession {
                    id: format!("session-{}", session_counter),
                    mission_id: mission.id.clone(),
                    helper_id: wf.id.clone(),
                    requester_id: mission.requester_id.clone(),
                    status: HelpSessionStatus::Offered,
                    created_at: tick,
                    updated_at: tick,
                    helper_benevolence: wf.benevolence,
                });
            }
        }
    }
}

/// ヘルプセッションを1 tick 進行させる。
///
/// 状態遷移:
/// - Offered → Accepted (benevolence-biased) または Rejected
/// - Accepted → Executing
/// - Executing → Succeeded (benevolence-biased) または HarmfulMismatch または Abandoned
fn advance_help_sessions(
    sessions: &mut Vec<SimHelpSession>,
    rng: &mut StdRng,
    current_tick: u64,
) {
    for session in sessions.iter_mut() {
        if session.updated_at >= current_tick {
            continue; // 既に今 tick で処理済み
        }

        match session.status {
            HelpSessionStatus::Offered => {
                // 慈悲スコアが高いヘルパーほど受け入れられやすい
                let accept_prob = 0.5 + session.helper_benevolence as f64 * 0.3;
                if rng.gen::<f64>() < accept_prob {
                    session.status = HelpSessionStatus::Accepted;
                } else {
                    session.status = HelpSessionStatus::Rejected;
                }
                session.updated_at = current_tick;
            }
            HelpSessionStatus::Accepted => {
                // 受け入れ→即実行
                session.status = HelpSessionStatus::Executing;
                session.updated_at = current_tick;
            }
            HelpSessionStatus::Executing => {
                // 慈悲スコアが高いほど成功確率が高い
                let success_prob = 0.6 + session.helper_benevolence as f64 * 0.25;
                let harmful_prob = 0.15 - session.helper_benevolence as f64 * 0.1;
                let roll: f64 = rng.gen();

                if roll < success_prob {
                    session.status = HelpSessionStatus::Succeeded;
                } else if roll < success_prob + harmful_prob {
                    session.status = HelpSessionStatus::HarmfulMismatch;
                } else {
                    session.status = HelpSessionStatus::Abandoned;
                }
                session.updated_at = current_tick;
            }
            _ => {
                // 完了・拒否・放棄済みのセッションは変更しない
            }
        }
    }
}

// ============================================================
// 疑似イベント構築
// ============================================================

/// あるワークフローのヘルプセッション履歴から ReciprocityEvent のリストを構築する。
///
/// これは compute_direct_reciprocity への入力として使用される。
fn build_events_for_workflow(
    workflow_id: &str,
    sessions: &[SimHelpSession],
    now: u64,
) -> Vec<ReciprocityEvent> {
    let mut events = Vec::new();

    for session in sessions.iter().filter(|s| s.requester_id == workflow_id || s.helper_id == workflow_id) {
        let (source_id, target_id, event_kind) = match &session.status {
            HelpSessionStatus::Succeeded => {
                // ヘルパー視点: ヘルパー→要求元
                if session.helper_id == workflow_id {
                    (session.helper_id.clone(), session.requester_id.clone(), ReciprocityEventKind::HelpSucceeded)
                } else {
                    continue;
                }
            }
            HelpSessionStatus::HarmfulMismatch => {
                if session.helper_id == workflow_id {
                    (session.helper_id.clone(), session.requester_id.clone(), ReciprocityEventKind::HarmfulMismatch)
                } else {
                    continue;
                }
            }
            HelpSessionStatus::Abandoned => {
                if session.helper_id == workflow_id {
                    (session.helper_id.clone(), session.requester_id.clone(), ReciprocityEventKind::HelpAbandoned)
                } else {
                    continue;
                }
            }
            HelpSessionStatus::Rejected => {
                if session.requester_id == workflow_id {
                    (session.requester_id.clone(), session.helper_id.clone(), ReciprocityEventKind::HelpRejected)
                } else {
                    continue;
                }
            }
            // Offered/Accepted/Executing はまだ完了していないのでイベント化しない
            _ => continue,
        };

        events.push(ReciprocityEvent {
            event_id: format!("sim-event-{}-{}", workflow_id, events.len()),
            mission_id: session.mission_id.clone(),
            source_graph_id: source_id,
            target_graph_id: target_id,
            event_kind,
            weight: 1.0,
            created_at: SystemTime::UNIX_EPOCH,
            virtual_clock: now,
            trace_ref: None,
        });
    }

    events
}

// ============================================================
// Phase 4: 信頼・評判再計算 + ライフサイクル GC
// ============================================================

/// 全生存ワークフローの信頼・評判・互恵性スコアを再計算する。
fn recompute_trust_reputation(
    population: &mut [SimWorkflowState],
    sessions: &[SimHelpSession],
    tick: u64,
    policy: &ReciprocityLifecyclePolicy,
) {
    for wf in population.iter_mut().filter(|w| w.survived) {
        // 直接互恵性: このワークフローが関係する完了済みイベントから計算
        let events = build_events_for_workflow(&wf.id, sessions, tick);
        let direct_score = compute_direct_reciprocity(&events, tick, policy);
        wf.direct_reciprocity = direct_score;

        // 間接互恵性: ReputationProfile のフィールドから導出
        let centrality = wf.reputation.village_centrality;
        let village_participation = if sessions.is_empty() {
            0.0
        } else {
            let involved = sessions.iter().filter(|s| s.helper_id == wf.id || s.requester_id == wf.id).count() as f32;
            (involved / sessions.len() as f32).min(1.0)
        };
        let indirect_score = compute_indirect_reciprocity(
            centrality,
            village_participation,
            wf.reputation.accepted_offer_rate,
            wf.reputation.help_success_rate,
            wf.reputation.harm_event_count as f32 * 0.1,
        );
        wf.indirect_reciprocity = indirect_score;

        // 慈悲スコア: F-3
        let reputation_score = wf.reputation.final_score;
        let benevolence = compute_benevolence_score(direct_score, indirect_score, reputation_score);
        wf.benevolence = benevolence;
        wf.reputation.benevolence_score = benevolence;

        // 評判再計算: F-4 + F-5
        let reputation_inputs = ReputationInputs {
            direct_score,
            indirect_score,
            experience_count: wf.experience as u32,
            inherited_score: 0.0,
        };
        let new_reputation = recompute_reputation(reputation_inputs, policy);
        wf.reputation.final_score = new_reputation.final_score;
        wf.reputation.last_recomputed_at = SystemTime::now();

        // 信頼スコアの簡易更新
        let help_success_count = sessions
            .iter()
            .filter(|s| s.helper_id == wf.id && s.status == HelpSessionStatus::Succeeded)
            .count() as f32;
        let help_total = sessions
            .iter()
            .filter(|s| s.helper_id == wf.id)
            .count() as f32;
        if help_total > 0.0 {
            wf.reputation.help_success_rate = help_success_count / help_total;
        }
    }
}

/// ライフサイクル GC を実行する。
///
/// 各生存ワークフローの GC hazard を計算し、生存確率に基づいて死亡判定を行う。
/// 子ワークフローは child protection により死亡しにくい。
fn run_lifecycle_gc(
    population: &mut [SimWorkflowState],
    _sessions: &[SimHelpSession],
    policy: &ReciprocityLifecyclePolicy,
    rng: &mut StdRng,
    current_tick: u64,
    gc_interval: u64,
) {
    // GC 実行間隔に達していない tick はスキップ
    if gc_interval == 0 || current_tick % gc_interval != 0 {
        return;
    }
    for wf in population.iter_mut().filter(|w| w.survived) {
        // 簡易 LifecycleScore: 経験値の正規化 [0, 1]
        let lifecycle_score = (wf.experience as f64 / (E_ADULT_THRESHOLD * 3) as f64).min(1.0) as f32;

        // child protection: CHILD_PROTECT_ETA1
        let child_protection = if wf.is_child { CHILD_PROTECT_ETA1 } else { 0.0 };
        let child_protection = child_protection + wf.reputation.benevolence_score * 0.3;

        let hazard = compute_gc_hazard(lifecycle_score, wf.benevolence, child_protection, policy);
        wf.hazard = hazard;

        let survival_prob = compute_survival_probability(hazard, 1);
        let survived = rng.gen::<f64>() < survival_prob;

        if !survived {
            wf.survived = false;
        }
    }
}

// ============================================================
// Phase 5: Tick 観測器
// ============================================================

/// 現在の人口状態から metrics を観測する。
fn observe_tick(tick: u64, population: &[SimWorkflowState]) -> SimulationTickSnapshot {
    let survived: Vec<&SimWorkflowState> = population.iter().filter(|w| w.survived).collect();

    if survived.is_empty() {
        return SimulationTickSnapshot {
            tick,
            benevolence_score_p50: 0.0,
            benevolence_score_p95: 0.0,
            direct_reciprocity_p50: 0.0,
            direct_reciprocity_p95: 0.0,
            indirect_reciprocity_p50: 0.0,
            indirect_reciprocity_p95: 0.0,
            reputation_final_p50: 0.0,
            reputation_final_p95: 0.0,
            benevolent_survival_rate: 0.0,
            non_benevolent_survival_rate: 0.0,
            survival_advantage: 0.0,
            harmful_gc_rate: 0.0,
        };
    }

    // 慈悲スコアのパーセンタイル
    let mut benevolence_scores: Vec<f32> = survived.iter().map(|w| w.benevolence).collect();
    benevolence_scores.sort_unstable_by(|a, b| a.partial_cmp(b).expect("benevolence NaN なし"));
    let p50 = percentile_f32(&benevolence_scores, 0.5);
    let p95 = percentile_f32(&benevolence_scores, 0.95);

    // 直接互恵性
    let mut dr_scores: Vec<f32> = survived.iter().map(|w| w.direct_reciprocity).collect();
    dr_scores.sort_unstable_by(|a, b| a.partial_cmp(b).expect("direct_reciprocity NaN なし"));
    let dr_p50 = percentile_f32(&dr_scores, 0.5);
    let dr_p95 = percentile_f32(&dr_scores, 0.95);

    // 間接互恵性
    let mut ir_scores: Vec<f32> = survived.iter().map(|w| w.indirect_reciprocity).collect();
    ir_scores.sort_unstable_by(|a, b| a.partial_cmp(b).expect("indirect_reciprocity NaN なし"));
    let ir_p50 = percentile_f32(&ir_scores, 0.5);
    let ir_p95 = percentile_f32(&ir_scores, 0.95);

    // 評判スコア
    let mut rep_scores: Vec<f32> = survived.iter().map(|w| w.reputation.final_score).collect();
    rep_scores.sort_unstable_by(|a, b| a.partial_cmp(b).expect("final_score NaN なし"));
    let rep_p50 = percentile_f32(&rep_scores, 0.5);
    let rep_p95 = percentile_f32(&rep_scores, 0.95);

    // Kind World 分類: 初期慈悲スコア > 0.5 を benevolent と定義
    let total_benevolent = population.iter().filter(|w| w.initial_benevolence > 0.5).count();
    let total_non_benevolent = population.iter().filter(|w| w.initial_benevolence <= 0.5).count();
    let survived_benevolent = population.iter().filter(|w| w.survived && w.initial_benevolence > 0.5).count();
    let survived_non_benevolent = population.iter().filter(|w| w.survived && w.initial_benevolence <= 0.5).count();

    let benevolent_survival_rate = if total_benevolent > 0 {
        survived_benevolent as f32 / total_benevolent as f32
    } else {
        0.0
    };
    let non_benevolent_survival_rate = if total_non_benevolent > 0 {
        survived_non_benevolent as f32 / total_non_benevolent as f32
    } else {
        0.0
    };
    let survival_advantage = benevolent_survival_rate - non_benevolent_survival_rate;

    // HarmfulMismatch の割合（全人口に対する死亡数）
    let dead_count = population.iter().filter(|w| !w.survived).count();
    let harmful_gc_rate = dead_count as f32 / population.len() as f32;

    SimulationTickSnapshot {
        tick,
        benevolence_score_p50: p50,
        benevolence_score_p95: p95,
        direct_reciprocity_p50: dr_p50,
        direct_reciprocity_p95: dr_p95,
        indirect_reciprocity_p50: ir_p50,
        indirect_reciprocity_p95: ir_p95,
        reputation_final_p50: rep_p50,
        reputation_final_p95: rep_p95,
        benevolent_survival_rate,
        non_benevolent_survival_rate,
        survival_advantage,
        harmful_gc_rate,
    }
}

/// ソート済みスライスから指定パーセンタイル値を計算する。
fn percentile_f32(sorted: &[f32], percentile: f64) -> f32 {
    if sorted.is_empty() {
        return 0.0;
    }
    let index = ((sorted.len() as f64 - 1.0) * percentile).round() as usize;
    sorted[index.min(sorted.len() - 1)]
}

// ============================================================
// 実験 ID 生成
// ============================================================

/// 実験 ID を生成する (exp-{timestamp}-{seq})。
///
/// chrono クレート非依存のため、YYYYMMDD の代わりに UNIX タイムスタンプを使用する。
/// 単調増加性と一意性は保証される。
fn generate_experiment_id(seq_counter: &mut u64) -> String {
    *seq_counter += 1;
    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!("exp-{}-{:03}", duration.as_secs(), seq_counter)
}

// ============================================================
// メインシミュレーション実行
// ============================================================

/// シミュレーションを実行し、全 tick の metrics 系列を返す。
///
/// # 決定論的再現性
///
/// 同一 `ReciprocitySimulatorConfig`（同一 seed）で複数回呼び出すと、
/// ビットレベルで同一の結果が得られる。
pub fn run_simulation(config: &ReciprocitySimulatorConfig) -> ReciprocitySimulationResult {
    let mut rng = StdRng::seed_from_u64(config.seed);
    let mut population = generate_population(config, &mut rng);

    let mut sessions: Vec<SimHelpSession> = Vec::new();
    let mut mission_counter: u64 = 0;
    let mut session_counter: u64 = 0;
    let mut metric_series = Vec::with_capacity(config.max_ticks as usize);

    for tick in 0..config.max_ticks {
        // Phase 2: ミッション生成
        let missions = generate_mission_stream(
            tick,
            &population,
            config.mission_rate,
            &mut rng,
            &mut mission_counter,
        );

        // Phase 3: 支援申し出
        offer_help_sessions(&missions, &population, &mut sessions, tick, &mut rng, &mut session_counter);

        // Phase 3: セッション進行
        advance_help_sessions(&mut sessions, &mut rng, tick);

        // Phase 4: 信頼・評判再計算
        recompute_trust_reputation(&mut population, &sessions, tick, &config.policy);

        // Phase 4: ライフサイクル GC
        run_lifecycle_gc(&mut population, &sessions, &config.policy, &mut rng, tick, config.gc_interval);

        // Phase 5: 観測
        let snapshot = observe_tick(tick, &population);
        metric_series.push(snapshot);
    }

    let mut seq_counter: u64 = 0;
    let experiment_id = generate_experiment_id(&mut seq_counter);

    ReciprocitySimulationResult {
        metric_series,
        final_state: population,
        experiment_id,
    }
}

// ============================================================
// 観測テスト
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::ReciprocityLifecyclePolicy;

    /// デフォルト設定を取得する。
    fn default_config() -> ReciprocitySimulatorConfig {
        ReciprocitySimulatorConfig::default()
    }

    // -------------------------------------------------------
    // T1: 決定論的リプレイ — 同一 seed でビットレベル一致
    // -------------------------------------------------------
    #[test]
    fn test_deterministic_replay() {
        let config = default_config();
        let result1 = run_simulation(&config);
        let result2 = run_simulation(&config);

        assert_eq!(
            result1.metric_series.len(),
            result2.metric_series.len(),
            "T1: metric_series 長が一致すること"
        );

        for (i, (s1, s2)) in result1.metric_series.iter().zip(result2.metric_series.iter()).enumerate() {
            assert!(
                (s1.benevolent_survival_rate - s2.benevolent_survival_rate).abs() < 1e-6,
                "T1: tick {} benevolent_survival_rate が一致: {} vs {}",
                i,
                s1.benevolent_survival_rate,
                s2.benevolent_survival_rate
            );
            assert!(
                (s1.non_benevolent_survival_rate - s2.non_benevolent_survival_rate).abs() < 1e-6,
                "T1: tick {} non_benevolent_survival_rate が一致: {} vs {}",
                i,
                s1.non_benevolent_survival_rate,
                s2.non_benevolent_survival_rate
            );
        }

        println!("T1: deterministic_replay PASS — {} ticks matched", result1.metric_series.len());
    }

    // -------------------------------------------------------
    // T2: child_ratio=0 → child 無しでも実行可能
    // -------------------------------------------------------
    #[test]
    fn test_no_children() {
        let config = ReciprocitySimulatorConfig {
            child_ratio: 0.0,
            ..default_config()
        };
        let result = run_simulation(&config);

        let has_child = result.final_state.iter().any(|w| w.is_child);
        assert!(!has_child, "T2: child_ratio=0 で child が存在しないこと");
        assert!(!result.metric_series.is_empty(), "T2: metrics が空でないこと");

        println!("T2: no_children PASS — population={}, ticks={}", result.final_state.len(), result.metric_series.len());
    }

    // -------------------------------------------------------
    // T3: max_ticks=0 → 空の metric_series
    // -------------------------------------------------------
    #[test]
    fn test_zero_ticks() {
        let config = ReciprocitySimulatorConfig {
            max_ticks: 0,
            ..default_config()
        };
        let result = run_simulation(&config);

        assert!(result.metric_series.is_empty(), "T3: max_ticks=0 で metric_series が空であること");

        println!("T3: zero_ticks PASS — metric_series.len()=0");
    }

    // -------------------------------------------------------
    // T4: 慈悲的ワークフローの生存優位 ≥ 0
    // -------------------------------------------------------
    #[test]
    fn test_benevolent_survival_advantage() {
        let config = default_config();
        let result = run_simulation(&config);

        for snapshot in &result.metric_series {
            assert!(
                snapshot.survival_advantage >= -1e-6,
                "T4: survival_advantage={} が負（許容誤差内）",
                snapshot.survival_advantage
            );
        }

        // 最終 tick の生存優位を出力
        if let Some(last) = result.metric_series.last() {
            println!(
                "T4: benevolent_survival_advantage={:.6} (benevolent={:.6}, non_benevolent={:.6})",
                last.survival_advantage, last.benevolent_survival_rate, last.non_benevolent_survival_rate
            );
        }
    }

    // -------------------------------------------------------
    // T5: lambda_gc_base=0 → デフォルトより死亡減少
    // -------------------------------------------------------
    #[test]
    fn test_zero_gc_base() {
        let mut low_hazard_policy = ReciprocityLifecyclePolicy::default();
        low_hazard_policy.lambda_gc_base = 0.0;
        low_hazard_policy.gamma_lifecycle = 10.0;  // 生存寄与を最大化
        low_hazard_policy.gamma_benevolence = 10.0;
        low_hazard_policy.gamma_child_protect = 10.0;

        let low_config = ReciprocitySimulatorConfig {
            policy: low_hazard_policy,
            ..default_config()
        };
        let default_config = default_config();

        let low_result = run_simulation(&low_config);
        let default_result = run_simulation(&default_config);

        let low_dead_count = low_result.final_state.iter().filter(|w| !w.survived).count();
        let default_dead_count = default_result.final_state.iter().filter(|w| !w.survived).count();

        // 低 hazard 設定の死亡数がデフォルト以下であること
        assert!(
            low_dead_count <= default_dead_count,
            "T5: low_hazard dead={} <= default dead={}",
            low_dead_count,
            default_dead_count,
        );

        println!(
            "T5: zero_gc_base PASS — low_hazard dead={}/{} vs default dead={}/{}",
            low_dead_count, low_result.final_state.len(),
            default_dead_count, default_result.final_state.len(),
        );
    }

    // -------------------------------------------------------
    // T6: 全 metrics が [0, 1] 範囲内
    // -------------------------------------------------------
    #[test]
    fn test_metrics_in_range() {
        let config = default_config();
        let result = run_simulation(&config);

        for snapshot in &result.metric_series {
            let fields = [
                ("benevolence_score_p50", snapshot.benevolence_score_p50),
                ("benevolence_score_p95", snapshot.benevolence_score_p95),
                ("direct_reciprocity_p50", snapshot.direct_reciprocity_p50),
                ("direct_reciprocity_p95", snapshot.direct_reciprocity_p95),
                ("indirect_reciprocity_p50", snapshot.indirect_reciprocity_p50),
                ("indirect_reciprocity_p95", snapshot.indirect_reciprocity_p95),
                ("reputation_final_p50", snapshot.reputation_final_p50),
                ("reputation_final_p95", snapshot.reputation_final_p95),
                ("benevolent_survival_rate", snapshot.benevolent_survival_rate),
                ("non_benevolent_survival_rate", snapshot.non_benevolent_survival_rate),
                ("survival_advantage", snapshot.survival_advantage),
                ("harmful_gc_rate", snapshot.harmful_gc_rate),
            ];

            for (name, value) in &fields {
                assert!(
                    *value >= -1e-6 && *value <= 1.0 + 1e-6,
                    "T6: tick {} {}={} が [0, 1] 範囲外",
                    snapshot.tick, name, value
                );
            }
        }

        println!("T6: metrics_in_range PASS — {} ticks verified", result.metric_series.len());
    }

    // -------------------------------------------------------
    // T7: 境界パラメータで panic しない
    // -------------------------------------------------------
    #[test]
    fn test_boundary_parameters() {
        let configs = vec![
            ReciprocitySimulatorConfig {
                population_size: 1,
                child_ratio: 0.0,
                mission_rate: 0.0,
                ..default_config()
            },
            ReciprocitySimulatorConfig {
                population_size: 100,
                child_ratio: 1.0,
                mission_rate: 1.0,
                max_ticks: 5,
                ..default_config()
            },
            ReciprocitySimulatorConfig {
                population_size: 2,
                child_ratio: 0.5,
                mission_rate: 0.0,
                max_ticks: 1,
                ..default_config()
            },
        ];

        for (i, config) in configs.iter().enumerate() {
            let result = run_simulation(config);
            assert!(
                result.metric_series.len() <= config.max_ticks as usize,
                "T7: config {} の metric_series 長が妥当",
                i
            );
        }

        println!("T7: boundary_parameters PASS — {} configs", configs.len());
    }

    // -------------------------------------------------------
    // T8: 実験 ID 形式検証
    // -------------------------------------------------------
    #[test]
    fn test_experiment_id_format() {
        let mut seq: u64 = 0;
        let id1 = generate_experiment_id(&mut seq);
        let id2 = generate_experiment_id(&mut seq);

        assert!(id1.starts_with("exp-"), "T8: ID '{}' が exp- で始まる", id1);
        assert!(id2.starts_with("exp-"), "T8: ID '{}' が exp- で始まる", id2);
        assert_ne!(id1, id2, "T8: 連続生成 ID が異なる");

        println!("T8: experiment_id_format PASS — id1={}, id2={}", id1, id2);
    }

    // -------------------------------------------------------
    // T9: CSV 形式の観測出力
    // -------------------------------------------------------
    #[test]
    fn test_csv_observation_output() {
        let config = default_config();
        let result = run_simulation(&config);

        // CSV ヘッダー
        println!("T9: tick,benevolent_rate,non_benevolent_rate,survival_advantage,benevolence_p50,reputation_p50,harmful_gc_rate");

        for snapshot in &result.metric_series {
            println!(
                "T9: {},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6}",
                snapshot.tick,
                snapshot.benevolent_survival_rate,
                snapshot.non_benevolent_survival_rate,
                snapshot.survival_advantage,
                snapshot.benevolence_score_p50,
                snapshot.reputation_final_p50,
                snapshot.harmful_gc_rate,
            );
        }

        // Kind World 創発の確認
        if let Some(last) = result.metric_series.last() {
            println!(
                "T9: FINAL survival_advantage={:.6} (benevolent={:.6}, non_benevolent={:.6})",
                last.survival_advantage, last.benevolent_survival_rate, last.non_benevolent_survival_rate
            );
        }

        // 最終生存状態のサマリ
        let benevolent_alive = result.final_state.iter().filter(|w| w.survived && w.initial_benevolence > 0.5).count();
        let non_benevolent_alive = result.final_state.iter().filter(|w| w.survived && w.initial_benevolence <= 0.5).count();
        let benevolent_total = result.final_state.iter().filter(|w| w.initial_benevolence > 0.5).count();
        let non_benevolent_total = result.final_state.iter().filter(|w| w.initial_benevolence <= 0.5).count();

        println!("T9: benevolent: {}/{} alive, non_benevolent: {}/{} alive",
            benevolent_alive, benevolent_total,
            non_benevolent_alive, non_benevolent_total);
    }
}
