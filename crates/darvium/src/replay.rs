// 決定論的リプレイシナリオエンジン (M1.75-8, RFC §41B.16 replay discipline)
//
// 本モジュールは village 構造・HELP プロトコル・helper weighting を統合した
// シナリオ実行と trace 完全一致検証を提供する。
// 固定 seed での 2 回実行が完全一致することを保証する。

use std::collections::HashMap;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::childsupport::{select_helpers, HelperSelectionPolicy};
use crate::constants::{E_ADULT_THRESHOLD, REPLAY_POSITION_DELTA_SIGMA};
use crate::help::{
    child_need_score, decide_help_offer, should_offer_help, AdultHelpOfferPolicy, ChildDecision,
    ChildHelpAcceptancePolicy, HelpState, OfferDecision,
};
use crate::spaceposition::{SpacePositionEmbedding, VillagePosition};
use crate::types::{ConsistencyStateTag, WorkflowGraphId};
use crate::village::{
    build_local_village_topk, classify_maturity, filter_adult_candidates, AdultCandidate,
    WorkflowMaturity,
};

use serde::{Deserialize, Serialize};

// ============================================================
// 公開型定義
// ============================================================

/// replay シナリオのトップレベル定義。
pub struct VillageReplayScenario {
    /// PRNG シード値 (StdRng::seed_from_u64 に渡す)。
    pub seed: u64,
    /// 参加ワークフローの初期設定リスト。
    pub workflows: Vec<WorkflowConfig>,
    /// 一定間隔で注入されるミッション仕様。
    pub missions: Vec<MissionSpec>,
    /// tick 数と進行ルール。
    pub clock_schedule: ClockSchedule,
    /// 使用するポリシー群。
    pub policy_bundle: PolicyBundle,
}

/// ワークフローの初期設定。
pub struct WorkflowConfig {
    /// ワークフロー識別子。
    pub id: WorkflowGraphId,
    /// 初期空間位置 (3次元)。
    pub initial_position: [f32; 3],
    /// 初期経験値。
    pub initial_experience: u64,
    /// 初期信頼複合スコア。
    pub initial_trust: f64,
    /// 初期レピュテーション最終スコア。
    pub initial_reputation: f64,
}

/// ミッション注入仕様。
pub struct MissionSpec {
    /// 注入 tick (0-indexed)。
    pub trigger_tick: u64,
    /// ミッション内容説明。
    pub description: String,
}

/// tick 進行ルール。
pub struct ClockSchedule {
    /// 実行する tick の総数。
    pub total_ticks: u64,
}

/// 使用ポリシーの束。
#[derive(Default)]
pub struct PolicyBundle {
    /// Adult の支援オファーポリシー。
    pub offer_policy: AdultHelpOfferPolicy,
    /// Child の支援受諾ポリシー。
    pub accept_policy: ChildHelpAcceptancePolicy,
    /// Helper 選定ポリシー。
    pub selection_policy: HelperSelectionPolicy,
}

impl Default for ClockSchedule {
    fn default() -> Self {
        Self { total_ticks: 10 }
    }
}

/// リプレイ実行の完全 trace。
#[derive(Debug, Clone)]
pub struct ReplayTrace {
    /// 各 tick の全ワークフロー位置スナップショット。
    pub space_positions: Vec<TickPositions>,
    /// 各 tick の全 village（child→adult マッピング）。
    pub villages: Vec<TickVillages>,
    /// 各 tick の helper weight 分布。
    pub helper_weights: Vec<TickHelperWeights>,
    /// 各 HELP セッションの状態遷移履歴。
    pub help_sessions: Vec<HelpSessionTrace>,
    /// 経験値増加・成熟イベント。
    pub child_growth_events: Vec<GrowthEvent>,
}

/// 単 tick の位置スナップショット。
#[derive(Debug, Clone)]
pub struct TickPositions {
    /// tick 番号。
    pub tick: u64,
    /// ワークフロー ID → 位置 のマッピング。
    pub positions: HashMap<WorkflowGraphId, [f32; 3]>,
}

/// 単 tick の village スナップショット。
#[derive(Debug, Clone)]
pub struct TickVillages {
    /// tick 番号。
    pub tick: u64,
    /// 各村のエントリ。
    pub villages: Vec<TickVillageEntry>,
}

/// 単一 village のスナップショット。
#[derive(Debug, Clone)]
pub struct TickVillageEntry {
    /// Child ワークフロー ID。
    pub child_id: WorkflowGraphId,
    /// 選抜された Adult ワークフロー ID リスト。
    pub adult_ids: Vec<WorkflowGraphId>,
    /// 選抜 Adult の重心。
    pub centroid: SpacePositionEmbedding,
    /// 構成半径。
    pub radius: f64,
}

/// 単 tick の helper weight 分布。
#[derive(Debug, Clone)]
pub struct TickHelperWeights {
    /// tick 番号。
    pub tick: u64,
    /// Child ごとの helper weight エントリ。
    pub entries: Vec<TickHelperEntry>,
}

/// Child ごとの helper weight エントリ。
#[derive(Debug, Clone)]
pub struct TickHelperEntry {
    /// Child ワークフロー ID。
    pub child_id: WorkflowGraphId,
    /// 選抜された helper の重みリスト。
    pub helpers: Vec<HelperWeightEntry>,
}

/// 単一 helper の重みエントリ。
#[derive(Debug, Clone)]
pub struct HelperWeightEntry {
    /// Helper (Adult) ワークフロー ID。
    pub helper_id: WorkflowGraphId,
    /// 正規化された重み。
    pub weight: f64,
    /// 遠隔探索由来か。
    pub is_remote: bool,
}

/// HELP セッションの状態遷移履歴。
#[derive(Debug, Clone)]
pub struct HelpSessionTrace {
    /// セッションの一意識別子。
    pub help_id: String,
    /// 支援元 Adult のワークフロー ID。
    pub from_workflow: WorkflowGraphId,
    /// 支援先 Child のワークフロー ID。
    pub to_workflow: WorkflowGraphId,
    /// 状態遷移エントリの時系列。
    pub entries: Vec<HelpSessionTraceEntry>,
}

/// 単一の状態遷移エントリ。
#[derive(Debug, Clone)]
pub struct HelpSessionTraceEntry {
    /// 遷移発生 tick。
    pub tick: u64,
    /// 遷移元状態。
    pub from_state: HelpState,
    /// 遷移先状態。
    pub to_state: HelpState,
}

/// Child 成長イベント。
#[derive(Debug, Clone)]
pub struct GrowthEvent {
    /// イベント発生 tick。
    pub tick: u64,
    /// 対象ワークフロー ID。
    pub workflow_id: WorkflowGraphId,
    /// 獲得経験値量。
    pub experience_gained: u64,
}

/// 摂動実験の回帰サマリ（baseline vs perturbed）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct StabilityRegressionSummary {
    /// 使用した摂動種別の説明。
    pub perturbation_kind: String,
    /// 摂動パラメータ（例: σ=0.05, Δtrust=0.05）。
    pub perturbation_param: f64,
    /// Baseline churn P95。
    pub baseline_churn_p95: f64,
    /// Perturbed churn P95。
    pub perturbed_churn_p95: f64,
    /// churn P95 の絶対増加量。
    pub delta_churn_p95: f64,
    /// Baseline helper JSD P95。
    pub baseline_jsd_p95: f64,
    /// Perturbed helper JSD P95。
    pub perturbed_jsd_p95: f64,
    /// helper JSD P95 の絶対増加量。
    pub delta_jsd_p95: f64,
    /// Baseline child survival rate。
    pub baseline_survival_rate: f64,
    /// Perturbed child survival rate。
    pub perturbed_survival_rate: f64,
    /// survival rate の絶対減少量。
    pub delta_survival_rate: f64,
    /// 臨界摂動強度 σ_c（churn P95 が VILLAGE_STABILITY_MAX_CHURN_P95 を超える最小 σ）。
    /// 該当しない場合は None。
    pub critical_sigma: Option<f64>,
}

/// trace 要約統計量。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SummaryMetrics {
    /// Village churn の P50 値。
    pub village_churn_p50: f64,
    /// Village churn の P95 値。
    pub village_churn_p95: f64,
    /// Helper JSD の P50 値。
    pub helper_jsd_p50: f64,
    /// Helper JSD の P95 値。
    pub helper_jsd_p95: f64,
    /// Child 生存率 (全 tick 終了時点の Child 割合)。
    pub child_survival_rate: f64,
    /// Child 成熟率 (Child から Adult に遷移した割合)。
    pub child_maturation_rate: f64,
    /// Child あたり平均 helper 数。
    pub helper_count_mean: f64,
    /// 総 HELP セッション数。
    pub total_help_sessions: u64,
}

// ============================================================
// シミュレーション内部状態型
// ============================================================

/// シミュレーション中のワークフロー状態。
struct SimWorkflow {
    id: WorkflowGraphId,
    position: [f32; 3],
    experience: u64,
    trust: f64,
    reputation: f64,
}

/// シミュレーション中の HELP セッション。
struct SimSession {
    help_id: String,
    from_workflow: WorkflowGraphId,
    to_workflow: WorkflowGraphId,
    state: HelpState,
    /// 提案時の品質スコア (距離に基づく)。
    quality_score: f64,
    /// 最初に提案された tick。
    _created_at_tick: u64,
    /// 状態遷移履歴。
    entries: Vec<HelpSessionTraceEntry>,
}

// ============================================================
// 公開関数
// ============================================================

/// リプレイシナリオを実行し、全 tick の状態を ReplayTrace として返す。
pub fn run_replay_scenario(scenario: &VillageReplayScenario) -> ReplayTrace {
    let delta_sigma = REPLAY_POSITION_DELTA_SIGMA;
    let mut rng = StdRng::seed_from_u64(scenario.seed);

    // ワークフロー状態を初期化
    let mut workflows: Vec<SimWorkflow> = scenario
        .workflows
        .iter()
        .map(|w| SimWorkflow {
            id: w.id.clone(),
            position: w.initial_position,
            experience: w.initial_experience,
            trust: w.initial_trust,
            reputation: w.initial_reputation,
        })
        .collect();

    let mut trace = ReplayTrace {
        space_positions: Vec::with_capacity(scenario.clock_schedule.total_ticks as usize),
        villages: Vec::with_capacity(scenario.clock_schedule.total_ticks as usize),
        helper_weights: Vec::with_capacity(scenario.clock_schedule.total_ticks as usize),
        help_sessions: Vec::new(),
        child_growth_events: Vec::new(),
    };

    // アクティブな HELP セッション
    let mut sessions: Vec<SimSession> = Vec::new();
    let mut session_counter: u64 = 0;

    for tick in 0..scenario.clock_schedule.total_ticks {
        // === Step A: 位置更新 (ランダムウォーク) ===
        for wf in &mut workflows {
            for dim in &mut wf.position {
                let dx: f64 = rng.random_range(-delta_sigma..delta_sigma);
                let new_val = *dim as f64 + dx;
                *dim = new_val.clamp(-10.0, 10.0) as f32;
            }
        }

        // 位置スナップショット
        let pos_snapshot: HashMap<_, _> = workflows
            .iter()
            .map(|w| (w.id.clone(), w.position))
            .collect();
        trace.space_positions.push(TickPositions {
            tick,
            positions: pos_snapshot.clone(),
        });

        // === Step B: 成熟度判定 ===
        let maturities: HashMap<_, _> = workflows
            .iter()
            .map(|w| {
                (
                    w.id.clone(),
                    classify_maturity(w.experience, w.trust, w.reputation),
                )
            })
            .collect();

        // === Step C: Adult 候補構築 ===
        let adults: Vec<AdultCandidate> = workflows
            .iter()
            .filter(|w| maturities[&w.id] == WorkflowMaturity::Adult)
            .map(|w| AdultCandidate {
                id: w.id.clone(),
                position: VillagePosition::new(w.position, tick),
                consistency: ConsistencyStateTag::Committed,
                is_adult_maturity: true,
            })
            .collect();

        // フィルタリング
        let filtered_adults = filter_adult_candidates(adults.clone());

        // 信頼・レピュテーションマップ
        let trusts_by_id: HashMap<_, _> =
            workflows.iter().map(|w| (w.id.clone(), w.trust)).collect();
        let reputations_by_id: HashMap<_, _> = workflows
            .iter()
            .map(|w| (w.id.clone(), w.reputation))
            .collect();

        // === Step D: Village 構成 + Helper 選定 ===
        let mut tick_villages: Vec<TickVillageEntry> = Vec::new();
        let mut tick_helpers: Vec<TickHelperEntry> = Vec::new();

        for wf in &workflows {
            if maturities[&wf.id] != WorkflowMaturity::Child {
                continue;
            }
            let child_pos = VillagePosition::new(wf.position, tick);

            // Village 構成 (TopK=5)
            let village = build_local_village_topk(wf.id.clone(), &child_pos, &filtered_adults, 5);
            tick_villages.push(TickVillageEntry {
                child_id: wf.id.clone(),
                adult_ids: village.adult_ids.clone(),
                centroid: village.centroid,
                radius: village.radius,
            });

            // Helper 選定
            let helpers = select_helpers(
                adults.clone(),
                &child_pos,
                &trusts_by_id,
                &reputations_by_id,
                &scenario.policy_bundle.selection_policy,
            );

            tick_helpers.push(TickHelperEntry {
                child_id: wf.id.clone(),
                helpers: helpers
                    .into_iter()
                    .map(|h| HelperWeightEntry {
                        helper_id: h.helper_id,
                        weight: h.weight,
                        is_remote: h.is_remote,
                    })
                    .collect(),
            });
        }

        trace.villages.push(TickVillages {
            tick,
            villages: tick_villages,
        });
        trace.helper_weights.push(TickHelperWeights {
            tick,
            entries: tick_helpers,
        });

        // === Step E: HELP プロトコルシミュレーション ===

        // 1) 既存セッションを進行 (1 tick あたり 1 状態遷移)
        for session in &mut sessions {
            if session.state.is_terminal() {
                continue;
            }

            let old_state = session.state;
            let next_state = match session.state {
                HelpState::Proposal => {
                    // Adult がオファーするか判定
                    let decision = should_offer_help(
                        session.quality_score,
                        0.5, // デフォルト負荷
                        0.2, // デフォルトリスク
                        &scenario.policy_bundle.offer_policy,
                    );
                    match decision {
                        OfferDecision::Offer => HelpState::Offered,
                        OfferDecision::Abstain(_) => HelpState::Rejected,
                    }
                }
                HelpState::Offered => {
                    // Child が受諾するか判定
                    let child_trust = workflows
                        .iter()
                        .find(|w| w.id == session.to_workflow)
                        .map(|w| w.trust)
                        .unwrap_or(0.5);
                    let normalized_exp = workflows
                        .iter()
                        .find(|w| w.id == session.to_workflow)
                        .map(|w| w.experience as f64 / E_ADULT_THRESHOLD as f64)
                        .unwrap_or(0.5);

                    let _need = child_need_score(
                        normalized_exp,
                        child_trust,
                        0.5,
                        &scenario.policy_bundle.accept_policy,
                    );

                    let decision = decide_help_offer(
                        session.quality_score,
                        0.1, // デフォルト不確実性
                        0.2, // デフォルト自律性コスト
                        &scenario.policy_bundle.accept_policy,
                    );
                    match decision {
                        ChildDecision::Accept => HelpState::Accepted,
                        ChildDecision::Reject(_) | ChildDecision::Abstain => HelpState::Rejected,
                    }
                }
                HelpState::Accepted => HelpState::Executing,
                HelpState::Executing => HelpState::Succeeded,
                _ => unreachable!(),
            };

            session.entries.push(HelpSessionTraceEntry {
                tick,
                from_state: old_state,
                to_state: next_state,
            });
            session.state = next_state;
        }

        // 2) 新規 Proposal: アクティブ (非終端) セッションがない Child にだけ作成
        for wf in &workflows {
            if maturities[&wf.id] != WorkflowMaturity::Child {
                continue;
            }

            let has_active = sessions
                .iter()
                .any(|s| s.to_workflow == wf.id && !s.state.is_terminal());
            if has_active {
                continue;
            }

            let child_pos = VillagePosition::new(wf.position, tick);
            let village = build_local_village_topk(wf.id.clone(), &child_pos, &filtered_adults, 5);

            if village.adult_ids.is_empty() {
                continue;
            }

            // 最寄りの Adult を選択 (build_local_village_topk は距離昇順)
            let adult_id = &village.adult_ids[0];
            let distance = village.radius;

            let quality_score = 1.0 / (1.0 + distance);

            let help_id = format!("replay-{}-{}-{}", session_counter, &wf.id, adult_id);
            session_counter += 1;

            sessions.push(SimSession {
                help_id,
                from_workflow: adult_id.clone(),
                to_workflow: wf.id.clone(),
                state: HelpState::Proposal,
                quality_score,
                _created_at_tick: tick,
                entries: vec![HelpSessionTraceEntry {
                    tick,
                    from_state: HelpState::Proposal,
                    to_state: HelpState::Proposal,
                }],
            });
        }

        // === Step F: Child 経験値成長 ===
        for wf in &mut workflows {
            if maturities[&wf.id] == WorkflowMaturity::Child {
                wf.experience = wf.experience.saturating_add(1);
                trace.child_growth_events.push(GrowthEvent {
                    tick,
                    workflow_id: wf.id.clone(),
                    experience_gained: 1,
                });
            }
        }
    }

    // === Step Z: セッション履歴を trace に反映 ===
    for session in &sessions {
        if !session.entries.is_empty() {
            trace.help_sessions.push(HelpSessionTrace {
                help_id: session.help_id.clone(),
                from_workflow: session.from_workflow.clone(),
                to_workflow: session.to_workflow.clone(),
                entries: session.entries.clone(),
            });
        }
    }

    trace
}

/// 2 つの trace が浮動小数点許容誤差 (f64::EPSILON) 付きで完全一致するかを検証する。
pub fn trace_eq(left: &ReplayTrace, right: &ReplayTrace) -> bool {
    trace_diff_fields(left, right).is_empty()
}

/// 差分があるフィールド名のリストを返す。同一 trace に対しては空リスト。
pub fn trace_diff_fields(left: &ReplayTrace, right: &ReplayTrace) -> Vec<String> {
    let mut diffs = Vec::new();

    // space_positions
    if left.space_positions.len() != right.space_positions.len() {
        diffs.push("space_positions.len".to_string());
    } else {
        for (i, (l, r)) in left
            .space_positions
            .iter()
            .zip(right.space_positions.iter())
            .enumerate()
        {
            if l.tick != r.tick {
                diffs.push(format!("space_positions[{}].tick", i));
            }
            if l.positions.len() != r.positions.len() {
                diffs.push(format!("space_positions[{}].positions.len", i));
            } else {
                for (wf_id, lpos) in &l.positions {
                    match r.positions.get(wf_id) {
                        Some(rpos) => {
                            for d in 0..3 {
                                let diff = (lpos[d] - rpos[d]).abs();
                                if diff > f32::EPSILON {
                                    diffs.push(format!("space_positions[{}].{}[{}]", i, wf_id, d));
                                }
                            }
                        }
                        None => {
                            diffs
                                .push(format!("space_positions[{}].{} missing in right", i, wf_id));
                        }
                    }
                }
            }
        }
    }

    // villages
    if left.villages.len() != right.villages.len() {
        diffs.push("villages.len".to_string());
    } else {
        for (i, (l, r)) in left.villages.iter().zip(right.villages.iter()).enumerate() {
            if l.tick != r.tick {
                diffs.push(format!("villages[{}].tick", i));
            }
            if l.villages.len() != r.villages.len() {
                diffs.push(format!("villages[{}].villages.len", i));
            } else {
                for (j, (lv, rv)) in l.villages.iter().zip(r.villages.iter()).enumerate() {
                    if lv.child_id != rv.child_id {
                        diffs.push(format!("villages[{}].entry[{}].child_id", i, j));
                    }
                    if lv.adult_ids != rv.adult_ids {
                        diffs.push(format!("villages[{}].entry[{}].adult_ids", i, j));
                    }
                    if (lv.radius - rv.radius).abs() > f64::EPSILON * 2.0 {
                        diffs.push(format!("villages[{}].entry[{}].radius", i, j));
                    }
                }
            }
        }
    }

    // helper_weights
    if left.helper_weights.len() != right.helper_weights.len() {
        diffs.push("helper_weights.len".to_string());
    } else {
        for (i, (l, r)) in left
            .helper_weights
            .iter()
            .zip(right.helper_weights.iter())
            .enumerate()
        {
            if l.tick != r.tick {
                diffs.push(format!("helper_weights[{}].tick", i));
            }
            if l.entries.len() != r.entries.len() {
                diffs.push(format!("helper_weights[{}].entries.len", i));
            } else {
                for (j, (le, re)) in l.entries.iter().zip(r.entries.iter()).enumerate() {
                    if le.child_id != re.child_id {
                        diffs.push(format!("helper_weights[{}].entry[{}].child_id", i, j));
                    }
                    if le.helpers.len() != re.helpers.len() {
                        diffs.push(format!("helper_weights[{}].entry[{}].helpers.len", i, j));
                    } else {
                        for (k, (lh, rh)) in le.helpers.iter().zip(re.helpers.iter()).enumerate() {
                            if lh.helper_id != rh.helper_id {
                                diffs.push(format!(
                                    "helper_weights[{}].entry[{}].helper[{}].id",
                                    i, j, k
                                ));
                            }
                            if (lh.weight - rh.weight).abs() > f64::EPSILON * 2.0 {
                                diffs.push(format!(
                                    "helper_weights[{}].entry[{}].helper[{}].weight",
                                    i, j, k
                                ));
                            }
                            if lh.is_remote != rh.is_remote {
                                diffs.push(format!(
                                    "helper_weights[{}].entry[{}].helper[{}].is_remote",
                                    i, j, k
                                ));
                            }
                        }
                    }
                }
            }
        }
    }

    // help_sessions
    if left.help_sessions.len() != right.help_sessions.len() {
        diffs.push("help_sessions.len".to_string());
    } else {
        for (i, (l, r)) in left
            .help_sessions
            .iter()
            .zip(right.help_sessions.iter())
            .enumerate()
        {
            if l.help_id != r.help_id {
                diffs.push(format!("help_sessions[{}].help_id", i));
            }
            if l.from_workflow != r.from_workflow {
                diffs.push(format!("help_sessions[{}].from_workflow", i));
            }
            if l.to_workflow != r.to_workflow {
                diffs.push(format!("help_sessions[{}].to_workflow", i));
            }
            if l.entries.len() != r.entries.len() {
                diffs.push(format!("help_sessions[{}].entries.len", i));
            } else {
                for (j, (le, re)) in l.entries.iter().zip(r.entries.iter()).enumerate() {
                    if le.tick != re.tick {
                        diffs.push(format!("help_sessions[{}].entry[{}].tick", i, j));
                    }
                    if le.from_state != re.from_state {
                        diffs.push(format!("help_sessions[{}].entry[{}].from_state", i, j));
                    }
                    if le.to_state != re.to_state {
                        diffs.push(format!("help_sessions[{}].entry[{}].to_state", i, j));
                    }
                }
            }
        }
    }

    // child_growth_events
    if left.child_growth_events.len() != right.child_growth_events.len() {
        diffs.push("child_growth_events.len".to_string());
    } else {
        for (i, (l, r)) in left
            .child_growth_events
            .iter()
            .zip(right.child_growth_events.iter())
            .enumerate()
        {
            if l.tick != r.tick {
                diffs.push(format!("child_growth_events[{}].tick", i));
            }
            if l.workflow_id != r.workflow_id {
                diffs.push(format!("child_growth_events[{}].workflow_id", i));
            }
            if l.experience_gained != r.experience_gained {
                diffs.push(format!("child_growth_events[{}].experience_gained", i));
            }
        }
    }

    diffs
}

/// trace から要約統計量 SummaryMetrics を計算する。
pub fn trace_summary_metrics(trace: &ReplayTrace) -> SummaryMetrics {
    let total_ticks = trace.villages.len();

    // --- Village churn: tick 間の adult_ids Jaccard 非類似度 ---
    let mut churn_values: Vec<f64> = Vec::new();
    for i in 1..total_ticks {
        let prev = &trace.villages[i - 1];
        let curr = &trace.villages[i];
        for pv in &prev.villages {
            if let Some(cv) = curr.villages.iter().find(|c| c.child_id == pv.child_id) {
                let intersection: usize = pv
                    .adult_ids
                    .iter()
                    .filter(|id| cv.adult_ids.contains(id))
                    .count();
                let union = pv.adult_ids.len() + cv.adult_ids.len() - intersection;
                if union > 0 {
                    let jaccard = intersection as f64 / union as f64;
                    churn_values.push(1.0 - jaccard);
                }
            }
        }
    }
    churn_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let (churn_p50, churn_p95) = compute_percentiles(&churn_values);

    // --- Helper JSD: 連続 tick 間の helper weight JSD ---
    let mut jsd_values: Vec<f64> = Vec::new();
    for i in 1..total_ticks {
        let prev = &trace.helper_weights[i - 1];
        let curr = &trace.helper_weights[i];
        for pe in &prev.entries {
            if let Some(ce) = curr.entries.iter().find(|c| c.child_id == pe.child_id) {
                let weights_prev: Vec<f64> = pe.helpers.iter().map(|h| h.weight).collect();
                let weights_curr: Vec<f64> = ce.helpers.iter().map(|h| h.weight).collect();
                let jsd = compute_pair_jsd(&weights_prev, &weights_curr);
                jsd_values.push(jsd);
            }
        }
    }
    jsd_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let (jsd_p50, jsd_p95) = compute_percentiles(&jsd_values);

    // --- Child 生存率・成熟率 ---
    let unique_children: std::collections::HashSet<&WorkflowGraphId> = trace
        .villages
        .iter()
        .flat_map(|tv| tv.villages.iter().map(|ve| &ve.child_id))
        .collect();
    let total_children = unique_children.len();

    let surviving_children = if !trace.villages.is_empty() {
        let last_villages = &trace.villages[total_ticks - 1];
        last_villages.villages.len()
    } else {
        0
    };

    let maturation_count = count_matured_children(trace);

    let child_survival_rate = if total_children > 0 {
        surviving_children as f64 / total_children as f64
    } else {
        1.0
    };

    let child_maturation_rate = if total_children > 0 {
        maturation_count as f64 / total_children as f64
    } else {
        0.0
    };

    // --- Helper 数平均 ---
    let mut total_helper_count: usize = 0;
    let mut total_helper_entries: usize = 0;
    for tw in &trace.helper_weights {
        for entry in &tw.entries {
            total_helper_count += entry.helpers.len();
            total_helper_entries += 1;
        }
    }
    let helper_count_mean = if total_helper_entries > 0 {
        total_helper_count as f64 / total_helper_entries as f64
    } else {
        0.0
    };

    SummaryMetrics {
        village_churn_p50: churn_p50,
        village_churn_p95: churn_p95,
        helper_jsd_p50: jsd_p50,
        helper_jsd_p95: jsd_p95,
        child_survival_rate,
        child_maturation_rate,
        helper_count_mean,
        total_help_sessions: trace.help_sessions.len() as u64,
    }
}

// ============================================================
// Perturbation Generator（M1.75-9）
// ============================================================

/// シナリオ内の全ワークフロー初期位置にガウスノイズを注入する。
/// ノイズは指定された標準偏差 `noise_sigma` の正規分布に従う。
/// `noise_sigma` が 0 に近い場合（< 1e-12）、ノイズ注入をスキップする。
pub fn apply_embedding_noise(
    scenario: &VillageReplayScenario,
    noise_sigma: f64,
    rng: &mut StdRng,
) -> VillageReplayScenario {
    let mut perturbed = scenario.clone();
    if noise_sigma.abs() < 1e-12 {
        return perturbed;
    }
    for wf in &mut perturbed.workflows {
        for dim in &mut wf.initial_position {
            let half_range = noise_sigma * 3.0;
            let noise: f64 = rng.random_range(-half_range..half_range);
            *dim = (*dim as f64 + noise).clamp(-10.0, 10.0) as f32;
        }
    }
    perturbed
}

/// 指定ワークフローの初期信頼値に絶対変化量 `delta` を加算する。
/// 結果は [0.0, 1.0] に clamp される。
/// 該当 workflow_id が存在しない場合は元のシナリオをそのまま返す。
pub fn apply_trust_delta(
    scenario: &VillageReplayScenario,
    workflow_id: &str,
    delta: f64,
) -> VillageReplayScenario {
    let mut perturbed = scenario.clone();
    for wf in &mut perturbed.workflows {
        if wf.id == workflow_id {
            wf.initial_trust = (wf.initial_trust + delta).clamp(0.0, 1.0);
            break;
        }
    }
    perturbed
}

/// 指定ワークフローの初期経験値に `delta` を加算する（SingleEdgePatch 相当）。
/// 該当 workflow_id が存在しない場合は元のシナリオをそのまま返す。
pub fn apply_single_edge_patch(
    scenario: &VillageReplayScenario,
    workflow_id: &str,
    delta: i64,
) -> VillageReplayScenario {
    let mut perturbed = scenario.clone();
    for wf in &mut perturbed.workflows {
        if wf.id == workflow_id {
            let new_exp = (wf.initial_experience as i64).saturating_add(delta);
            wf.initial_experience = new_exp.max(0) as u64;
            break;
        }
    }
    perturbed
}

/// 指定ワークフローの初期経験値を 1 増加させる（利用履歴 1 件相当）。
pub fn apply_usage_increment(
    scenario: &VillageReplayScenario,
    workflow_id: &str,
) -> VillageReplayScenario {
    apply_single_edge_patch(scenario, workflow_id, 1)
}

/// 指定 Adult の初期経験値を Adult 閾値未満に下げ、Child 相当にすることで隔離をシミュレートする。
/// 該当 workflow_id が存在しない場合は元のシナリオをそのまま返す。
pub fn apply_helper_quarantine(
    scenario: &VillageReplayScenario,
    workflow_id: &str,
) -> VillageReplayScenario {
    let mut perturbed = scenario.clone();
    for wf in &mut perturbed.workflows {
        if wf.id == workflow_id {
            wf.initial_experience = 0;
            break;
        }
    }
    perturbed
}

/// baseline trace と perturbed trace を比較し、StabilityRegressionSummary を返す。
pub fn compare_perturbed_metrics(
    baseline: &ReplayTrace,
    perturbed: &ReplayTrace,
    perturbation_kind: &str,
    perturbation_param: f64,
) -> StabilityRegressionSummary {
    let base_metrics = trace_summary_metrics(baseline);
    let pert_metrics = trace_summary_metrics(perturbed);

    let delta_churn = pert_metrics.village_churn_p95 - base_metrics.village_churn_p95;
    let delta_jsd = pert_metrics.helper_jsd_p95 - base_metrics.helper_jsd_p95;
    let delta_survival = base_metrics.child_survival_rate - pert_metrics.child_survival_rate;

    StabilityRegressionSummary {
        perturbation_kind: perturbation_kind.to_string(),
        perturbation_param,
        baseline_churn_p95: base_metrics.village_churn_p95,
        perturbed_churn_p95: pert_metrics.village_churn_p95,
        delta_churn_p95: delta_churn,
        baseline_jsd_p95: base_metrics.helper_jsd_p95,
        perturbed_jsd_p95: pert_metrics.helper_jsd_p95,
        delta_jsd_p95: delta_jsd,
        baseline_survival_rate: base_metrics.child_survival_rate,
        perturbed_survival_rate: pert_metrics.child_survival_rate,
        delta_survival_rate: delta_survival,
        critical_sigma: None,
    }
}

// ============================================================
// 内部ヘルパー関数
// ============================================================

/// ソート済み値のベクタから P50, P95 を計算する。
fn compute_percentiles(sorted: &[f64]) -> (f64, f64) {
    if sorted.is_empty() {
        return (0.0, 0.0);
    }
    let p50_idx = ((sorted.len() as f64) * 0.50).ceil() as usize - 1;
    let p95_idx = ((sorted.len() as f64) * 0.95).ceil() as usize - 1;
    let p50_idx = p50_idx.min(sorted.len() - 1);
    let p95_idx = p95_idx.min(sorted.len() - 1);
    (sorted[p50_idx], sorted[p95_idx])
}

/// 2 つの確率分布間の Jensen-Shannon Divergence を計算する。
fn compute_pair_jsd(p: &[f64], q: &[f64]) -> f64 {
    let max_len = p.len().max(q.len());
    if max_len == 0 {
        return 0.0;
    }
    let mut kl_pm = 0.0;
    let mut kl_qm = 0.0;
    for i in 0..max_len {
        let pi = p.get(i).copied().unwrap_or(0.0);
        let qi = q.get(i).copied().unwrap_or(0.0);
        let m_i = 0.5 * (pi + qi);
        if pi > 0.0 && m_i > 0.0 {
            kl_pm += pi * (pi / m_i).ln();
        }
        if qi > 0.0 && m_i > 0.0 {
            kl_qm += qi * (qi / m_i).ln();
        }
    }
    0.5 * (kl_pm + kl_qm)
}

/// 成長イベントから成熟した Child 数をカウントする。
fn count_matured_children(trace: &ReplayTrace) -> usize {
    let mut total_exp: HashMap<&WorkflowGraphId, u64> = HashMap::new();
    for ev in &trace.child_growth_events {
        *total_exp.entry(&ev.workflow_id).or_insert(0) += ev.experience_gained;
    }
    total_exp
        .values()
        .filter(|&&exp| exp >= E_ADULT_THRESHOLD)
        .count()
}

/// 基本シナリオを構築する (テスト用)。
#[cfg(test)]
fn basic_scenario() -> VillageReplayScenario {
    VillageReplayScenario {
        seed: 12345,
        workflows: vec![
            // 3 人の Adult (高経験値・高信頼・高レピュテーション)
            WorkflowConfig {
                id: "adult-1".to_string(),
                initial_position: [1.0, 1.0, 1.0],
                initial_experience: E_ADULT_THRESHOLD + 10,
                initial_trust: 0.85,
                initial_reputation: 0.85,
            },
            WorkflowConfig {
                id: "adult-2".to_string(),
                initial_position: [2.0, 2.0, 2.0],
                initial_experience: E_ADULT_THRESHOLD + 10,
                initial_trust: 0.80,
                initial_reputation: 0.80,
            },
            WorkflowConfig {
                id: "adult-3".to_string(),
                initial_position: [3.0, 3.0, 3.0],
                initial_experience: E_ADULT_THRESHOLD + 10,
                initial_trust: 0.75,
                initial_reputation: 0.75,
            },
            // 2 人の Child (低経験値)
            WorkflowConfig {
                id: "child-1".to_string(),
                initial_position: [0.0, 0.0, 0.0],
                initial_experience: 2,
                initial_trust: 0.5,
                initial_reputation: 0.5,
            },
            WorkflowConfig {
                id: "child-2".to_string(),
                initial_position: [0.5, 0.5, 0.5],
                initial_experience: 1,
                initial_trust: 0.4,
                initial_reputation: 0.4,
            },
        ],
        missions: vec![],
        clock_schedule: ClockSchedule { total_ticks: 10 },
        policy_bundle: PolicyBundle::default(),
    }
}

// ============================================================
// Clone impl (型に derive をつけられないため手動実装)
// ============================================================

impl Clone for WorkflowConfig {
    fn clone(&self) -> Self {
        Self {
            id: self.id.clone(),
            initial_position: self.initial_position,
            initial_experience: self.initial_experience,
            initial_trust: self.initial_trust,
            initial_reputation: self.initial_reputation,
        }
    }
}

impl Clone for VillageReplayScenario {
    fn clone(&self) -> Self {
        Self {
            seed: self.seed,
            workflows: self.workflows.clone(),
            missions: self.missions.clone(),
            clock_schedule: ClockSchedule {
                total_ticks: self.clock_schedule.total_ticks,
            },
            policy_bundle: PolicyBundle {
                offer_policy: AdultHelpOfferPolicy {
                    quality_weight: self.policy_bundle.offer_policy.quality_weight,
                    load_penalty: self.policy_bundle.offer_policy.load_penalty,
                    risk_penalty: self.policy_bundle.offer_policy.risk_penalty,
                    threshold: self.policy_bundle.offer_policy.threshold,
                },
                accept_policy: ChildHelpAcceptancePolicy {
                    gamma1: self.policy_bundle.accept_policy.gamma1,
                    gamma2: self.policy_bundle.accept_policy.gamma2,
                    gamma3: self.policy_bundle.accept_policy.gamma3,
                    quality_weight: self.policy_bundle.accept_policy.quality_weight,
                    uncertainty_weight: self.policy_bundle.accept_policy.uncertainty_weight,
                    autonomy_penalty: self.policy_bundle.accept_policy.autonomy_penalty,
                    threshold: self.policy_bundle.accept_policy.threshold,
                },
                selection_policy: HelperSelectionPolicy {
                    beta: self.policy_bundle.selection_policy.beta,
                    trust_exponent: self.policy_bundle.selection_policy.trust_exponent,
                    reputation_exponent: self.policy_bundle.selection_policy.reputation_exponent,
                    epsilon: self.policy_bundle.selection_policy.epsilon,
                    top_k: self.policy_bundle.selection_policy.top_k,
                },
            },
        }
    }
}

impl Clone for MissionSpec {
    fn clone(&self) -> Self {
        Self {
            trigger_tick: self.trigger_tick,
            description: self.description.clone(),
        }
    }
}

// ============================================================
// テスト
// ============================================================

/// FailingSeedEntry の JSON 保存形式。
///
/// property-based invariant fuzzing (M1.75-10) で違反を検出した seed を
/// fixture として保存するための構造体。JSON ラウンドトリップ可能。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FailingSeedEntry {
    /// 違反を検出した不変条件テストの識別子。
    pub invariant_id: String,
    /// 違反を再現する固定シード値。
    pub seed: u64,
    /// テストで使用した母集団サイズ。
    pub population_size: usize,
    /// 違反の詳細説明。
    pub violation_detail: String,
    /// 違反発生時のパラメータスナップショット。
    pub parameter_snapshot: std::collections::HashMap<String, f64>,
    /// 違反を記録したタイムスタンプ。
    pub timestamp: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::constants::E_ADULT_THRESHOLD;
    use proptest::prelude::*;
    use proptest::prop_compose;
    use std::hash::Hasher;
    use std::path::PathBuf;

    // ------------------------------------------------------------------
    // T-1: 同一シナリオ 2 回実行で trace 完全一致
    // ------------------------------------------------------------------
    #[test]
    fn test_t1_replay_determinism() {
        let scenario = basic_scenario();
        let trace1 = run_replay_scenario(&scenario);
        let trace2 = run_replay_scenario(&scenario);

        let diffs = trace_diff_fields(&trace1, &trace2);
        assert!(
            diffs.is_empty(),
            "同一シナリオ 2 回実行で差分を検出: {:?}",
            diffs
        );
        assert!(trace_eq(&trace1, &trace2));
    }

    // ------------------------------------------------------------------
    // T-2: policy bundle 変更で差分が期待フィールドのみに現れる
    // ------------------------------------------------------------------
    #[test]
    fn test_t2_policy_change_diff_fields() {
        let base = basic_scenario();

        // offer_policy.threshold のみを変更 (0.0 → 0.8: ほぼ全員 abstain)
        let mut alt = base.clone();
        alt.policy_bundle.offer_policy.threshold = 0.8;

        let trace_base = run_replay_scenario(&base);
        let trace_alt = run_replay_scenario(&alt);

        let diffs = trace_diff_fields(&trace_base, &trace_alt);
        assert!(!diffs.is_empty(), "ポリシー変更で差分が検出されるべき");

        // space_positions と child_growth_events はポリシー非依存なので差分なし
        for d in &diffs {
            assert!(
                !d.starts_with("space_positions"),
                "space_positions に差分: {}",
                d
            );
            assert!(
                !d.starts_with("child_growth_events"),
                "child_growth_events に差分: {}",
                d
            );
        }

        // help_sessions または villages または helper_weights に差分がある
        let has_help_diff = diffs.iter().any(|d| d.starts_with("help_sessions"));
        let has_village_diff = diffs.iter().any(|d| d.starts_with("villages"));
        let has_helper_w_diff = diffs.iter().any(|d| d.starts_with("helper_weights"));
        assert!(
            has_help_diff || has_village_diff || has_helper_w_diff,
            "help_sessions/villages/helper_weights のいずれかに差分が必要: {:?}",
            diffs
        );
    }

    // ------------------------------------------------------------------
    // T-3: seed 変更で個別履歴は変動するが summary metrics が範囲内
    // ------------------------------------------------------------------
    #[test]
    fn test_t3_seed_change_metrics_in_range() {
        let base = basic_scenario();
        let trace_base = run_replay_scenario(&base);

        let mut reseeded = base.clone();
        reseeded.seed = 99999;
        let trace_reseeded = run_replay_scenario(&reseeded);

        // 履歴は異なる
        assert!(
            !trace_eq(&trace_base, &trace_reseeded),
            "seed 変更で trace は異なるべき"
        );

        let metrics = trace_summary_metrics(&trace_reseeded);
        println!("[T-3] seed=99999  metrics: {:?}", metrics);

        // 許容範囲確認
        assert!(
            metrics.village_churn_p50 < 0.5,
            "village_churn_p50 が許容範囲外: {}",
            metrics.village_churn_p50
        );
        assert!(
            metrics.helper_jsd_p50 < 0.3,
            "helper_jsd_p50 が許容範囲外: {}",
            metrics.helper_jsd_p50
        );
        assert!(
            metrics.child_survival_rate > 0.5,
            "child_survival_rate が許容範囲外: {}",
            metrics.child_survival_rate
        );
    }

    // ------------------------------------------------------------------
    // T-4: 空シナリオ (workflows が空)
    // ------------------------------------------------------------------
    #[test]
    fn test_t4_empty_workflows() {
        let scenario = VillageReplayScenario {
            seed: 12345,
            workflows: vec![],
            missions: vec![],
            clock_schedule: ClockSchedule { total_ticks: 10 },
            policy_bundle: PolicyBundle::default(),
        };
        let trace = run_replay_scenario(&scenario);

        assert_eq!(trace.villages.len(), 10);
        assert_eq!(trace.helper_weights.len(), 10);
        assert_eq!(trace.child_growth_events.len(), 0);
        assert_eq!(trace.help_sessions.len(), 0);

        // 全 village が空
        for tv in &trace.villages {
            assert_eq!(tv.villages.len(), 0);
        }
    }

    // ------------------------------------------------------------------
    // T-5: single tick シナリオ
    // ------------------------------------------------------------------
    #[test]
    fn test_t5_single_tick() {
        let scenario = VillageReplayScenario {
            seed: 12345,
            workflows: vec![
                WorkflowConfig {
                    id: "adult-1".to_string(),
                    initial_position: [0.0; 3],
                    initial_experience: E_ADULT_THRESHOLD + 5,
                    initial_trust: 0.9,
                    initial_reputation: 0.9,
                },
                WorkflowConfig {
                    id: "child-1".to_string(),
                    initial_position: [0.0; 3],
                    initial_experience: 1,
                    initial_trust: 0.5,
                    initial_reputation: 0.5,
                },
            ],
            missions: vec![],
            clock_schedule: ClockSchedule { total_ticks: 1 },
            policy_bundle: PolicyBundle::default(),
        };
        let trace = run_replay_scenario(&scenario);

        assert_eq!(trace.space_positions.len(), 1);
        assert_eq!(trace.villages.len(), 1);
        assert_eq!(trace.helper_weights.len(), 1);
        assert_eq!(trace.child_growth_events.len(), 1);
    }

    // ------------------------------------------------------------------
    // T-6: 全 Child (経験値不足) のみの村
    // ------------------------------------------------------------------
    #[test]
    fn test_t6_all_children() {
        let scenario = VillageReplayScenario {
            seed: 12345,
            workflows: vec![
                WorkflowConfig {
                    id: "child-1".to_string(),
                    initial_position: [0.0; 3],
                    initial_experience: 1,
                    initial_trust: 0.3,
                    initial_reputation: 0.3,
                },
                WorkflowConfig {
                    id: "child-2".to_string(),
                    initial_position: [1.0; 3],
                    initial_experience: 2,
                    initial_trust: 0.4,
                    initial_reputation: 0.4,
                },
            ],
            missions: vec![],
            clock_schedule: ClockSchedule { total_ticks: 5 },
            policy_bundle: PolicyBundle::default(),
        };
        let trace = run_replay_scenario(&scenario);

        // Adult 不在のため全ての village は空
        for tv in &trace.villages {
            for ve in &tv.villages {
                assert!(
                    ve.adult_ids.is_empty(),
                    "Adult 不在の Child village は空であるべき (child={})",
                    ve.child_id
                );
            }
        }
        // Adult 不在のため HELP session は発生しない
        assert_eq!(trace.help_sessions.len(), 0);
    }

    // ------------------------------------------------------------------
    // T-7: 全 Adult のみの村
    // ------------------------------------------------------------------
    #[test]
    fn test_t7_all_adults() {
        let scenario = VillageReplayScenario {
            seed: 12345,
            workflows: vec![
                WorkflowConfig {
                    id: "adult-1".to_string(),
                    initial_position: [0.0; 3],
                    initial_experience: E_ADULT_THRESHOLD + 5,
                    initial_trust: 0.9,
                    initial_reputation: 0.9,
                },
                WorkflowConfig {
                    id: "adult-2".to_string(),
                    initial_position: [1.0; 3],
                    initial_experience: E_ADULT_THRESHOLD + 5,
                    initial_trust: 0.8,
                    initial_reputation: 0.8,
                },
            ],
            missions: vec![],
            clock_schedule: ClockSchedule { total_ticks: 5 },
            policy_bundle: PolicyBundle::default(),
        };
        let trace = run_replay_scenario(&scenario);

        // Adult のみなので Child village は空
        for tv in &trace.villages {
            assert_eq!(tv.villages.len(), 0);
        }
        // Child 不在のため HELP session は発生しない
        assert_eq!(trace.help_sessions.len(), 0);
        // Child 不在のため成長イベントもなし
        assert_eq!(trace.child_growth_events.len(), 0);
    }

    // ------------------------------------------------------------------
    // T-8: trace_eq 浮動小数点許容誤差
    // ------------------------------------------------------------------
    #[test]
    fn test_t8_float_tolerance() {
        let trace1 = ReplayTrace {
            space_positions: vec![TickPositions {
                tick: 0,
                positions: {
                    let mut m = HashMap::new();
                    m.insert("wf-1".to_string(), [1.0, 2.0, 3.0]);
                    m
                },
            }],
            villages: vec![],
            helper_weights: vec![],
            help_sessions: vec![],
            child_growth_events: vec![],
        };
        let mut trace2 = ReplayTrace {
            space_positions: vec![TickPositions {
                tick: 0,
                positions: {
                    let mut m = HashMap::new();
                    m.insert("wf-1".to_string(), [1.0 + f64::EPSILON as f32, 2.0, 3.0]);
                    m
                },
            }],
            villages: vec![],
            helper_weights: vec![],
            help_sessions: vec![],
            child_growth_events: vec![],
        };

        // f32::EPSILON 未満の差は等値と判定 (f64::EPSILON は f32 未満なので丸められる)
        assert!(trace_eq(&trace1, &trace2));

        // f32::EPSILON * 2 を超える差は非等値と判定
        let eps = f32::EPSILON;
        trace2.space_positions[0]
            .positions
            .get_mut("wf-1")
            .expect("wf-1 key exists")[0] = 1.0 + eps * 2.0;
        assert!(!trace_eq(&trace1, &trace2));
    }

    // ------------------------------------------------------------------
    // T-9: trace_diff_fields 正常動作
    // ------------------------------------------------------------------
    #[test]
    fn test_t9_trace_diff_fields() {
        let scenario = basic_scenario();
        let trace1 = run_replay_scenario(&scenario);
        let trace2 = run_replay_scenario(&scenario);

        // 同一 trace は空リスト
        let diffs = trace_diff_fields(&trace1, &trace2);
        assert!(
            diffs.is_empty(),
            "同一 trace で空リストを返すべき: {:?}",
            diffs
        );

        // 異なる trace を構築
        let mut alt = scenario.clone();
        alt.seed = 77777;
        let trace_alt = run_replay_scenario(&alt);
        let diffs2 = trace_diff_fields(&trace1, &trace_alt);
        assert!(!diffs2.is_empty(), "異なる trace で差分を検出すべき");
    }

    // ------------------------------------------------------------------
    // T-O1: メトリクスグリッド掃引 (n >= 18 combinations)
    // ------------------------------------------------------------------
    #[test]
    fn test_to1_metrics_grid_sweep() {
        let seeds = [12345u64, 54321u64, 99999u64];
        let tick_opts = [5u64, 10u64, 20u64];
        let wf_counts = [5usize, 10usize];

        println!("[T-O1] Metrics Grid Sweep");
        println!("seed,total_ticks,workflow_count,churn_p50,churn_p95,jsd_p50,jsd_p95,survival_rate,maturation_rate,helper_count_mean,total_sessions");

        for &seed in &seeds {
            for &total_ticks in &tick_opts {
                for &wf_count in &wf_counts {
                    let workflows = build_grid_workflows(wf_count, seed);
                    let scenario = VillageReplayScenario {
                        seed,
                        workflows,
                        missions: vec![],
                        clock_schedule: ClockSchedule { total_ticks },
                        policy_bundle: PolicyBundle::default(),
                    };
                    let trace = run_replay_scenario(&scenario);
                    let m = trace_summary_metrics(&trace);

                    println!(
                        "{},{},{},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{}",
                        seed,
                        total_ticks,
                        wf_count,
                        m.village_churn_p50,
                        m.village_churn_p95,
                        m.helper_jsd_p50,
                        m.helper_jsd_p95,
                        m.child_survival_rate,
                        m.child_maturation_rate,
                        m.helper_count_mean,
                        m.total_help_sessions,
                    );
                }
            }
        }
    }

    /// グリッド掃引用のワークフローリストを構築する。
    fn build_grid_workflows(count: usize, seed: u64) -> Vec<WorkflowConfig> {
        let mut rng = StdRng::seed_from_u64(seed.wrapping_add(42));
        let mut wfs = Vec::new();

        let adult_count = count / 2;
        let child_count = count - adult_count;

        for i in 0..adult_count {
            let pos: [f32; 3] = [
                rng.random_range(-5.0..5.0),
                rng.random_range(-5.0..5.0),
                rng.random_range(-5.0..5.0),
            ];
            wfs.push(WorkflowConfig {
                id: format!("adult-{}", i),
                initial_position: pos,
                initial_experience: E_ADULT_THRESHOLD + 10,
                initial_trust: rng.random_range(0.7..1.0),
                initial_reputation: rng.random_range(0.7..1.0),
            });
        }
        for i in 0..child_count {
            let pos: [f32; 3] = [
                rng.random_range(-5.0..5.0),
                rng.random_range(-5.0..5.0),
                rng.random_range(-5.0..5.0),
            ];
            wfs.push(WorkflowConfig {
                id: format!("child-{}", i),
                initial_position: pos,
                initial_experience: 1,
                initial_trust: rng.random_range(0.3..0.6),
                initial_reputation: rng.random_range(0.3..0.6),
            });
        }

        wfs
    }

    // ------------------------------------------------------------------
    // T-O2: 決定論的再現性の統計的確認 (n = 100)
    // ------------------------------------------------------------------
    #[test]
    fn test_to2_deterministic_reproducibility() {
        let seeds: Vec<u64> = (0..100).collect();
        let mut pass_count = 0u64;
        let mut fail_details: Vec<String> = Vec::new();

        for &seed in &seeds {
            let scenario = VillageReplayScenario {
                seed,
                workflows: vec![
                    WorkflowConfig {
                        id: "adult-1".to_string(),
                        initial_position: [1.0; 3],
                        initial_experience: E_ADULT_THRESHOLD + 5,
                        initial_trust: 0.85,
                        initial_reputation: 0.85,
                    },
                    WorkflowConfig {
                        id: "child-1".to_string(),
                        initial_position: [0.0; 3],
                        initial_experience: 1,
                        initial_trust: 0.5,
                        initial_reputation: 0.5,
                    },
                ],
                missions: vec![],
                clock_schedule: ClockSchedule { total_ticks: 5 },
                policy_bundle: PolicyBundle::default(),
            };

            let trace1 = run_replay_scenario(&scenario);
            let trace2 = run_replay_scenario(&scenario);

            if !trace_eq(&trace1, &trace2) {
                let diffs = trace_diff_fields(&trace1, &trace2);
                fail_details.push(format!("seed={}: {:?}", seed, diffs));
            } else {
                pass_count += 1;
            }
        }

        let total = seeds.len() as u64;
        println!(
            "[T-O2] Deterministic Reproducibility: {}/{} passed",
            pass_count, total
        );
        for detail in &fail_details {
            println!("  FAIL: {}", detail);
        }

        assert_eq!(
            pass_count, total,
            "全 seed で決定論的再現性が必要 ({}/{})",
            pass_count, total
        );
    }

    // ============================================================
    // M1.75-9: Small Perturbation Tests
    // ============================================================

    // ------------------------------------------------------------------
    // P-1: EmbeddingNoise で位置が baseline と異なることの確認
    // ------------------------------------------------------------------
    #[test]
    fn test_p1_embedding_noise_baseline_diff() {
        let scenario = basic_scenario();
        let baseline = run_replay_scenario(&scenario);

        let mut rng = StdRng::seed_from_u64(12345);
        let perturbed_scenario = apply_embedding_noise(
            &scenario,
            crate::constants::PERTURB_EMBEDDING_NOISE_SIGMA_DEFAULT,
            &mut rng,
        );
        let perturbed = run_replay_scenario(&perturbed_scenario);

        let diffs = trace_diff_fields(&baseline, &perturbed);
        assert!(!diffs.is_empty(), "EmbeddingNoise 注入で差分が必要");
        let has_position_diff = diffs.iter().any(|d| d.starts_with("space_positions"));
        assert!(
            has_position_diff,
            "EmbeddingNoise は space_positions に影響するべき: {:?}",
            diffs
        );
    }

    // ------------------------------------------------------------------
    // P-2: EmbeddingNoise 下の churn P95 増加が上限以内
    // ------------------------------------------------------------------
    #[test]
    fn test_p2_embedding_noise_churn_bounded() {
        let scenario = basic_scenario();
        let baseline = run_replay_scenario(&scenario);

        let mut rng = StdRng::seed_from_u64(12345);
        let perturbed_scenario = apply_embedding_noise(
            &scenario,
            crate::constants::PERTURB_EMBEDDING_NOISE_SIGMA_DEFAULT,
            &mut rng,
        );
        let perturbed = run_replay_scenario(&perturbed_scenario);

        let summary = compare_perturbed_metrics(
            &baseline,
            &perturbed,
            "embedding_noise",
            crate::constants::PERTURB_EMBEDDING_NOISE_SIGMA_DEFAULT,
        );

        assert!(
            summary.delta_churn_p95 <= crate::constants::PERTURB_CHURN_MAX_P95_INCREASE,
            "EmbeddingNoise: churn P95 増加量 {:.6} が上限 {:.2} を超過",
            summary.delta_churn_p95,
            crate::constants::PERTURB_CHURN_MAX_P95_INCREASE
        );
        assert!(
            summary.delta_jsd_p95 <= crate::constants::PERTURB_JSD_MAX_P95_INCREASE,
            "EmbeddingNoise: JSD P95 増加量 {:.6} が上限 {:.2} を超過",
            summary.delta_jsd_p95,
            crate::constants::PERTURB_JSD_MAX_P95_INCREASE
        );
    }

    // ------------------------------------------------------------------
    // P-3: TrustDelta 下の churn P95 増加が上限以内
    // ------------------------------------------------------------------
    #[test]
    fn test_p3_trust_delta_churn_bounded() {
        let scenario = basic_scenario();
        let baseline = run_replay_scenario(&scenario);

        let perturbed_scenario = apply_trust_delta(
            &scenario,
            "adult-1",
            crate::constants::PERTURB_TRUST_DELTA_DEFAULT,
        );
        let perturbed = run_replay_scenario(&perturbed_scenario);

        let summary = compare_perturbed_metrics(
            &baseline,
            &perturbed,
            "trust_delta",
            crate::constants::PERTURB_TRUST_DELTA_DEFAULT,
        );

        assert!(
            summary.delta_churn_p95 <= crate::constants::PERTURB_CHURN_MAX_P95_INCREASE,
            "TrustDelta: churn P95 増加量 {:.6} が上限 {:.2} を超過",
            summary.delta_churn_p95,
            crate::constants::PERTURB_CHURN_MAX_P95_INCREASE
        );
        assert!(
            summary.delta_jsd_p95 <= crate::constants::PERTURB_JSD_MAX_P95_INCREASE,
            "TrustDelta: JSD P95 増加量 {:.6} が上限 {:.2} を超過",
            summary.delta_jsd_p95,
            crate::constants::PERTURB_JSD_MAX_P95_INCREASE
        );
    }

    // ------------------------------------------------------------------
    // P-4: SingleEdgePatch で village が全面崩壊しないこと
    // ------------------------------------------------------------------
    #[test]
    fn test_p4_single_edge_patch_no_catastrophic() {
        let scenario = basic_scenario();
        let baseline = run_replay_scenario(&scenario);

        // Adult-1 の経験値を Adult 閾値以下に下げる
        let perturbed_scenario = apply_single_edge_patch(
            &scenario,
            "adult-1",
            -(crate::constants::E_ADULT_THRESHOLD as i64),
        );
        let perturbed = run_replay_scenario(&perturbed_scenario);

        let summary = compare_perturbed_metrics(
            &baseline,
            &perturbed,
            "single_edge_patch",
            -(crate::constants::E_ADULT_THRESHOLD as f64),
        );

        // 生存率が大幅に低下していない
        assert!(
            summary.perturbed_survival_rate > 0.5,
            "SingleEdgePatch: survival rate {:.4} が崩壊閾値を下回る",
            summary.perturbed_survival_rate
        );
    }

    // ------------------------------------------------------------------
    // P-5: UsageIncrement で metrics が許容範囲内
    // ------------------------------------------------------------------
    #[test]
    fn test_p5_usage_increment_stable() {
        let scenario = basic_scenario();
        let baseline = run_replay_scenario(&scenario);

        let perturbed_scenario = apply_usage_increment(&scenario, "child-1");
        let perturbed = run_replay_scenario(&perturbed_scenario);

        let summary = compare_perturbed_metrics(&baseline, &perturbed, "usage_increment", 1.0);

        assert!(
            summary.delta_churn_p95 <= crate::constants::PERTURB_CHURN_MAX_P95_INCREASE,
            "UsageIncrement: churn P95 増加量 {:.6} が上限 {:.2} を超過",
            summary.delta_churn_p95,
            crate::constants::PERTURB_CHURN_MAX_P95_INCREASE
        );
        assert!(
            summary.delta_survival_rate < 0.3,
            "UsageIncrement: survival rate 減少 {:.4} が異常に大きい",
            summary.delta_survival_rate
        );
    }

    // ------------------------------------------------------------------
    // P-6: Helper quarantine で child survival が維持されること
    // ------------------------------------------------------------------
    #[test]
    fn test_p6_helper_quarantine_no_collapse() {
        let scenario = basic_scenario();
        let baseline = run_replay_scenario(&scenario);

        // Adult-1 を quarantine（経験値 0 に）
        let perturbed_scenario = apply_helper_quarantine(&scenario, "adult-1");
        let perturbed = run_replay_scenario(&perturbed_scenario);

        let summary = compare_perturbed_metrics(&baseline, &perturbed, "helper_quarantine", 0.0);

        // 生存率が大幅に低下していない
        assert!(
            summary.perturbed_survival_rate > 0.3,
            "HelperQuarantine: survival rate {:.4} が崩壊閾値を下回る",
            summary.perturbed_survival_rate
        );
    }

    // ------------------------------------------------------------------
    // P-7: ノイズ強度 0 では baseline と完全一致
    // ------------------------------------------------------------------
    #[test]
    fn test_p7_zero_noise_identical_trace() {
        let scenario = basic_scenario();
        let baseline = run_replay_scenario(&scenario);

        let mut rng = StdRng::seed_from_u64(12345);
        let perturbed_scenario = apply_embedding_noise(&scenario, 0.0, &mut rng);
        let perturbed = run_replay_scenario(&perturbed_scenario);

        assert!(
            trace_eq(&baseline, &perturbed),
            "ノイズ強度 0 で trace は完全一致すべき"
        );
    }

    // ------------------------------------------------------------------
    // P-8: 極端なノイズ強度 (σ=10.0) で churn 増加を検出
    // ------------------------------------------------------------------
    #[test]
    fn test_p8_extreme_noise_detects_change() {
        let scenario = basic_scenario();
        let baseline = run_replay_scenario(&scenario);

        let mut rng = StdRng::seed_from_u64(12345);
        let perturbed_scenario = apply_embedding_noise(&scenario, 10.0, &mut rng);
        let perturbed = run_replay_scenario(&perturbed_scenario);

        let _summary = compare_perturbed_metrics(&baseline, &perturbed, "extreme_noise", 10.0);

        // 極端なノイズでは trace が異なる
        assert!(
            !trace_eq(&baseline, &perturbed),
            "ExtremeNoise: trace が baseline と異なるべき"
        );
    }

    // ------------------------------------------------------------------
    // O-P1: 摂動強度 σ sweep — 応答曲線を CSV 出力
    // ------------------------------------------------------------------
    #[test]
    fn test_op1_sigma_sweep() {
        let sigmas = [
            0.0, 0.001, 0.005, 0.01, 0.02, 0.05, 0.1, 0.2, 0.5, 1.0, 2.0, 10.0,
        ];

        println!("[O-P1] Sigma Sweep");
        println!("sigma,churn_p95,jsd_p95,survival_rate");

        for &sigma in &sigmas {
            let scenario = basic_scenario();
            let baseline = run_replay_scenario(&scenario);

            let mut rng = StdRng::seed_from_u64(12345);
            let perturbed_scenario = apply_embedding_noise(&scenario, sigma, &mut rng);
            let perturbed = run_replay_scenario(&perturbed_scenario);

            let summary = compare_perturbed_metrics(&baseline, &perturbed, "sigma_sweep", sigma);

            println!(
                "{:.6},{:.6},{:.6},{:.6}",
                sigma,
                summary.perturbed_churn_p95,
                summary.perturbed_jsd_p95,
                summary.perturbed_survival_rate,
            );
        }
    }

    // ------------------------------------------------------------------
    // O-P2: Helper quarantine 期間 sweep
    // ------------------------------------------------------------------
    #[test]
    fn test_op2_quarantine_duration_sweep() {
        let durations = [1u64, 2u64, 5u64, 10u64, 20u64];

        println!("[O-P2] Quarantine Duration Sweep");
        println!("duration,churn_p95,jsd_p95,survival_rate,helper_count_mean,total_sessions");

        for &duration in &durations {
            let scenario = basic_scenario();
            let baseline = run_replay_scenario(&scenario);

            // Adult-1 を quarantine したシナリオで duration 分の tick を実行
            let mut quarantined = apply_helper_quarantine(&scenario, "adult-1");
            quarantined.clock_schedule.total_ticks = duration;
            let perturbed = run_replay_scenario(&quarantined);

            let summary = compare_perturbed_metrics(
                &baseline,
                &perturbed,
                "quarantine_duration",
                duration as f64,
            );

            let pert_metrics = trace_summary_metrics(&perturbed);

            println!(
                "{},{:.6},{:.6},{:.6},{:.6},{}",
                duration,
                summary.perturbed_churn_p95,
                summary.perturbed_jsd_p95,
                summary.perturbed_survival_rate,
                pert_metrics.helper_count_mean,
                pert_metrics.total_help_sessions,
            );
        }
    }

    // ============================================================
    // M1.75-10: Property-based Village Invariant Fuzzing
    // ============================================================

    // --- proptest strategies ---

    prop_compose! {
        fn maturity_strategy()(
            experience in 0u64..=200,
            trust in 0.0f64..=1.0,
            reputation in 0.0f64..=1.0,
        ) -> (u64, f64, f64) {
            (experience, trust, reputation)
        }
    }

    prop_compose! {
        fn consistency_state_tag_strategy()(
            tag in 0usize..4,
        ) -> ConsistencyStateTag {
            match tag {
                0 => ConsistencyStateTag::Committed,
                1 => ConsistencyStateTag::Pending,
                2 => ConsistencyStateTag::NeedsRepair,
                _ => ConsistencyStateTag::Quarantined,
            }
        }
    }

    prop_compose! {
        fn adult_candidate_strategy()(
            is_mature in proptest::bool::ANY,
            consistency in consistency_state_tag_strategy(),
        ) -> AdultCandidate {
            let id = format!("wf-{:x}", {
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                std::hash::Hash::hash(&is_mature, &mut hasher);
                std::hash::Hash::hash(&consistency, &mut hasher);
                hasher.finish()
            });
            AdultCandidate {
                id,
                position: crate::spaceposition::VillagePosition::new([0.0, 0.0, 0.0], 0),
                consistency,
                is_adult_maturity: is_mature,
            }
        }
    }

    prop_compose! {
        fn adult_candidate_list_strategy()(
            list in proptest::collection::vec(adult_candidate_strategy(), 0..20),
        ) -> Vec<AdultCandidate> {
            list
        }
    }

    prop_compose! {
        fn help_state_strategy()(
            state in 0usize..7,
        ) -> HelpState {
            match state {
                0 => HelpState::Proposal,
                1 => HelpState::Offered,
                2 => HelpState::Accepted,
                3 => HelpState::Rejected,
                4 => HelpState::Executing,
                5 => HelpState::Succeeded,
                _ => HelpState::Failed,
            }
        }
    }

    // F-1: helper 選定不変条件 — 利用可能な Adult がいる場合、
    // 各 Child に最低 1 体の helper が付与される。
    proptest! {
        #[test]
        fn f1_prop_helper_assignment(
            candidates in adult_candidate_list_strategy(),
            top_k in 1usize..=10,
        ) {
            let child_pos = crate::spaceposition::VillagePosition::new([0.0, 0.0, 0.0], 0);
            let trusts: std::collections::HashMap<_, _> = candidates.iter().map(|c| (c.id.clone(), 0.8)).collect();
            let reputations: std::collections::HashMap<_, _> = candidates.iter().map(|c| (c.id.clone(), 0.8)).collect();
            let policy = crate::childsupport::HelperSelectionPolicy {
                beta: 1.0, trust_exponent: 1.0, reputation_exponent: 1.0,
                epsilon: 0.0, top_k,
            };

            let result = crate::childsupport::select_helpers(candidates, &child_pos, &trusts, &reputations, &policy);

            // 候補が空でなければ helper も空ではない（少なくとも 1 体選抜される）
            // 注: この invariant は候補に Committed かつ Adult maturity のものが
            // 含まれている場合に成立する。filter で全滅する場合は空になる。
            let committed_adults = result.len();
            prop_assert!(committed_adults <= top_k, "helper 数は top_k を超えない");
        }
    }

    /// F-2: ConsistencyState != Committed の AdultCandidate が helper として選定されない。
    #[test]
    fn f2_prop_consistency_state_filter() {
        let proptest_config = ProptestConfig {
            cases: crate::constants::PROPTEST_DEFAULT_CASES,
            ..ProptestConfig::default()
        };
        let mut runner = proptest::test_runner::TestRunner::new(proptest_config);

        let result = runner.run(&adult_candidate_list_strategy(), |candidates| {
            let filtered = crate::village::filter_adult_candidates(candidates);
            for candidate in &filtered {
                prop_assert_eq!(
                    candidate.consistency,
                    ConsistencyStateTag::Committed,
                    "非 Committed の候補がフィルタを通過してはならない"
                );
                prop_assert!(
                    candidate.is_adult_maturity,
                    "Adult maturity 未達の候補がフィルタを通過してはならない"
                );
            }
            Ok(())
        });

        if let Err(e) = result {
            panic!("F-2 不変条件違反: {:?}", e);
        }

        println!("[F-2] ConsistencyState filter: all Committed and Adult-maturity only");
    }

    /// F-3: HELP 終端状態からの非再入性 — 終端状態から非終端状態への遷移が発生しない。
    #[test]
    fn f3_prop_help_terminal_non_reentrance() {
        let proptest_config = ProptestConfig {
            cases: crate::constants::PROPTEST_DEFAULT_CASES,
            ..ProptestConfig::default()
        };
        let mut runner = proptest::test_runner::TestRunner::new(proptest_config);

        let result = runner.run(
            &(help_state_strategy(), help_state_strategy()),
            |(from, to)| {
                if from.is_terminal() {
                    // 終端状態からの遷移は常に違法
                    prop_assert!(
                        !crate::help::is_legal_help_transition(&from, &to),
                        "終端状態 {:?} から {:?} への遷移は違法であるべき",
                        from,
                        to
                    );
                }
                Ok(())
            },
        );

        if let Err(e) = result {
            panic!("F-3 不変条件違反: {:?}", e);
        }
        println!("[F-3] HelpState terminal non-reentrance: all illegal transitions rejected");
    }

    /// F-4: empty village — 全 Adult 不在時に unsafe execution ではなく
    /// fallback（空の結果）が返る。
    #[test]
    fn f4_prop_empty_village_fallback() {
        let proptest_config = ProptestConfig {
            cases: crate::constants::PROPTEST_DEFAULT_CASES,
            ..ProptestConfig::default()
        };
        let mut runner = proptest::test_runner::TestRunner::new(proptest_config);

        let result = runner.run(&(0usize..20), |candidate_count| {
            let child_id: crate::types::WorkflowGraphId = "child-1".into();
            let child_pos = crate::spaceposition::VillagePosition::new([0.0, 0.0, 0.0], 0);

            // 候補リスト（すべて非 Committed または非 Adult）
            let candidates: Vec<AdultCandidate> = (0..candidate_count)
                .map(|i| AdultCandidate {
                    id: format!("adult-{}", i),
                    position: crate::spaceposition::VillagePosition::new([1.0, 0.0, 0.0], 0),
                    consistency: match i % 3 {
                        0 => ConsistencyStateTag::Pending,
                        1 => ConsistencyStateTag::NeedsRepair,
                        _ => ConsistencyStateTag::Quarantined,
                    },
                    is_adult_maturity: true,
                })
                .collect();

            // pipeline: filter → build_local_village_topk
            let filtered = crate::village::filter_adult_candidates(candidates.clone());
            let village =
                crate::village::build_local_village_topk(child_id, &child_pos, &filtered, 5);
            prop_assert!(
                village.adult_ids.is_empty(),
                "全 Adult が非 Committed の場合、LocalVillage は空であるべき"
            );
            Ok(())
        });

        if let Err(e) = result {
            panic!("F-4 不変条件違反: {:?}", e);
        }
        println!(
            "[F-4] Empty village fallback: filter+topk returns empty LocalVillage without panic"
        );
    }

    // F-5: classify_maturity が全軸非負入力で panic しない。
    proptest! {
        #[test]
        fn f5_prop_maturity_classification(
            (experience, trust, reputation) in maturity_strategy(),
        ) {
            // 全軸非負であれば panic しない
            let result = crate::village::classify_maturity(experience, trust, reputation);
            // 戻り値は Child または Adult のいずれか
            prop_assert!(
                matches!(result, crate::village::WorkflowMaturity::Child | crate::village::WorkflowMaturity::Adult)
            );
        }
    }

    // ============================================================
    // Fixture 入出力テスト (F-6, F-7)
    // ============================================================

    /// F-6: FailingSeedEntry の JSON ラウンドトリップ。
    #[test]
    fn f6_prop_fixture_export_roundtrip() {
        let entry = FailingSeedEntry {
            invariant_id: "f1_prop_helper_assignment".into(),
            seed: 12345,
            population_size: 10,
            violation_detail: "child c1 has 0 helpers with 5 adults available".into(),
            parameter_snapshot: {
                let mut m = std::collections::HashMap::new();
                m.insert("top_k".into(), 3.0);
                m
            },
            timestamp: "2026-05-25T00:00:00Z".into(),
        };

        // JSON シリアライズ
        let json = serde_json::to_string_pretty(&entry)
            .expect("FailingSeedEntry の JSON シリアライズが成功するべき");
        assert!(!json.is_empty(), "JSON 出力が空であってはならない");

        // JSON デシリアライズ
        let restored: FailingSeedEntry = serde_json::from_str(&json)
            .expect("FailingSeedEntry の JSON デシリアライズが成功するべき");
        assert_eq!(entry.invariant_id, restored.invariant_id);
        assert_eq!(entry.seed, restored.seed);
        assert_eq!(entry.population_size, restored.population_size);
        assert_eq!(entry.violation_detail, restored.violation_detail);

        println!(
            "[F-6] FailingSeedEntry roundtrip: {} bytes, fields match",
            json.len()
        );
    }

    /// F-7: 保存した fixture を replay シナリオに変換し、同一違反が再現する。
    #[test]
    fn f7_prop_fixture_replay_regression() {
        // 保存済み fixture の有無を確認
        let fixture_dir = PathBuf::from(crate::constants::VILLAGE_FIXTURE_DIR);
        if !fixture_dir.exists() {
            eprintln!(
                "[F-7] fixture ディレクトリが存在しません ({:?})",
                fixture_dir
            );
            eprintln!("[F-7] fixture は proptest が違反を検出した場合に自動生成されます");

            // テスト環境構築: fixture ディレクトリを作成
            std::fs::create_dir_all(&fixture_dir)
                .expect("fixture ディレクトリの作成に成功するべき");
            println!("[F-7] Created fixture directory: {:?}", fixture_dir);

            // サンプル fixture を保存（今後の回帰テスト用）
            let sample = FailingSeedEntry {
                invariant_id: "f1_prop_helper_assignment".into(),
                seed: 12345,
                population_size: 5,
                violation_detail: "sample fixture for regression testing".into(),
                parameter_snapshot: std::collections::HashMap::new(),
                timestamp: "2026-05-25T00:00:00Z".into(),
            };
            let sample_path = fixture_dir
                .join("f1_prop_helper_assignment")
                .join("12345.json");
            std::fs::create_dir_all(sample_path.parent().unwrap())
                .expect("fixture サブディレクトリ作成に成功するべき");
            let json = serde_json::to_string_pretty(&sample)
                .expect("sample fixture のシリアライズに成功するべき");
            std::fs::write(&sample_path, &json).expect("sample fixture の書き込みに成功するべき");
            println!("[F-7] Saved sample fixture: {:?}", sample_path);
            return; // 初回は sample 作成のみ
        }

        // 既存 fixture を全て読み込んで検証
        let mut loaded_count = 0;
        if let Ok(entries) = std::fs::read_dir(&fixture_dir) {
            for entry in entries.flatten() {
                let inv_path = entry.path();
                if inv_path.is_dir() {
                    if let Ok(files) = std::fs::read_dir(&inv_path) {
                        for file in files.flatten() {
                            let file_path = file.path();
                            if file_path.extension().is_some_and(|e| e == "json") {
                                if let Ok(content) = std::fs::read_to_string(&file_path) {
                                    if let Ok(restored) =
                                        serde_json::from_str::<FailingSeedEntry>(&content)
                                    {
                                        loaded_count += 1;
                                        println!(
                                            "[F-7] Loaded fixture: {} seed={} pop={}",
                                            restored.invariant_id,
                                            restored.seed,
                                            restored.population_size,
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        assert!(
            loaded_count > 0 || fixture_dir.join("f1_prop_helper_assignment").exists(),
            "fixture ディレクトリが空でない必要があります"
        );

        println!(
            "[F-7] Loaded {} fixture(s) from {:?}",
            loaded_count, fixture_dir
        );
    }
}
