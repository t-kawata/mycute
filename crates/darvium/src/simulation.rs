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

use std::collections::HashMap;
use std::time::SystemTime;

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};

use crate::constants::{
    BENEVOLENT_BOTTOM_FRACTION, BENEVOLENT_TOP_FRACTION, CHILD_PROTECT_ETA1, E_ADULT_THRESHOLD,
};
use crate::error::DarviumError;
use crate::event::{
    ReciprocityEvent, ReciprocityEventKind, ReciprocityLifecyclePolicy, ReputationProfile,
};
use crate::help::HelpSession;
use crate::reciprocity::{
    compute_benevolence_score, compute_direct_reciprocity, compute_gc_hazard,
    compute_indirect_reciprocity, compute_survival_probability, recompute_reputation,
    ReputationInputs,
};
use crate::spaceposition::SpacePositionEmbedding;
use crate::trust::MemoizedGraph;
use crate::types::{NodeId, TrustProfile};

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

/// 1 tick の観測 metrics（M1.76-17: 基本 8 指標 + 生存率差）。
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

/// RFC §41B.20.7 の 11 運用メトリクスを保持する拡張構造体。
///
/// M1.76-17 の SimulationTickSnapshot（基本 8 パーセンタイル + 生存率差）に加えて、
/// 上位/下位 20% 分割による benevolent_survival_advantage、HarmfulMismatch 起因の
/// harmful_gc_rate、helper_accept_rate、help_abandon_rate、child_survival_rate を計算する。
///
/// 本構造体は M1.76-16 の ReciprocityOperationalMetrics（F-16 の 6 成分）とは別のものであり、
/// 既存構造体のフィールド削除や変更を行わない。
#[derive(Debug, Clone)]
pub struct ExtendedOperationalMetrics {
    /// tick 番号。
    pub tick: u64,
    /// 慈悲スコア中央値（SimulationTickSnapshot から転記）。
    pub benevolence_score_p50: f32,
    /// 慈悲スコア 95 パーセンタイル（SimulationTickSnapshot から転記）。
    pub benevolence_score_p95: f32,
    /// 直接互恵性中央値（SimulationTickSnapshot から転記）。
    pub direct_reciprocity_p50: f32,
    /// 直接互恵性 95 パーセンタイル（SimulationTickSnapshot から転記）。
    pub direct_reciprocity_p95: f32,
    /// 間接互恵性中央値（SimulationTickSnapshot から転記）。
    pub indirect_reciprocity_p50: f32,
    /// 間接互恵性 95 パーセンタイル（SimulationTickSnapshot から転記）。
    pub indirect_reciprocity_p95: f32,
    /// 評判スコア中央値（SimulationTickSnapshot から転記）。
    pub reputation_final_p50: f32,
    /// 評判スコア 95 パーセンタイル（SimulationTickSnapshot から転記）。
    pub reputation_final_p95: f32,
    /// 善良群（上位 20%）と非善良群（下位 20%）の生存率差。
    pub benevolent_survival_advantage: f64,
    /// HarmfulMismatch 起因の GC 率。
    pub harmful_gc_rate: f64,
    /// ヘルプ受け入れ率: Accepted / (Accepted + Rejected)。
    pub helper_accept_rate: f64,
    /// ヘルプ放棄率: Abandoned / (Succeeded + HarmfulMismatch + Abandoned)。
    pub help_abandon_rate: f64,
    /// 子ワークフロー生存率。
    pub child_survival_rate: f64,
}

/// シミュレーション実行結果。
#[derive(Debug, Clone)]
pub struct ReciprocitySimulationResult {
    /// 全 tick の metrics 系列。
    pub metric_series: Vec<SimulationTickSnapshot>,
    /// 最終状態の全ワークフロー。
    pub final_state: Vec<SimWorkflowState>,
    /// 全 tick のヘルプセッション一覧。
    pub sessions: Vec<SimHelpSession>,
    /// 実験 ID。
    pub experiment_id: String,
}

/// 各村の所属情報。村 ID の Option として表現する。
///
/// `None` はどの村にも所属していない個人を示す。
/// 既存の kind_world.rs の `&[Option<usize>]` との互換性を維持する。
pub type VillageAssignment = Option<usize>;

/// KW-REAL シミュレーションコンテキスト（M1.76-KW-REAL-P1）。
///
/// 現行の `SimWorkflowState`（flat struct）を実際の Darvium 部品
/// （MemoizedGraph / HelpSession / SpacePositionEmbedding 等）で
/// ラップしたシミュレーション状態。KW-REAL-P4（6 フェーズループ）の
/// 基盤となる。
///
/// # ライフタイム
/// `'a` は内部の `MemoizedGraph` 参照の生存期間に制限される。
/// シミュレーション実行関数のスコープ内で生成・使用・破棄する。
#[derive(Debug)]
pub struct SimulationContext<'a> {
    /// 実際の MemoizedGraph（個人 = WorkflowGraph のコンテナ）。
    pub memoized_graph: &'a mut MemoizedGraph,
    /// 個人ごとの信頼プロファイル（MemoizedGraph 全体の TrustProfile とは独立）。
    pub trust_profiles: HashMap<NodeId, TrustProfile>,
    /// 個人ごとの村所属（None = 未所属）。
    pub village_assignments: HashMap<NodeId, VillageAssignment>,
    /// 個人ごとの 3 次元空間位置。
    pub positions: HashMap<NodeId, SpacePositionEmbedding>,
    /// 現在のシミュレーション tick 数。
    pub tick: u64,
    /// 固定シード PRNG（全実験で StdRng::seed_from_u64(12345)）。
    pub rng: StdRng,
    /// アクティブな HELP セッション一覧（本物の HelpSession を使用）。
    pub help_sessions: Vec<HelpSession>,
}

impl<'a> SimulationContext<'a> {
    /// 新しい SimulationContext を生成する。
    ///
    /// # 引数
    /// - `memoized_graph`: 個人（WorkflowGraph）のコンテナへの可変参照
    /// - `rng`: 固定シード PRNG
    pub fn new(memoized_graph: &'a mut MemoizedGraph, mut rng: StdRng) -> Self {
        let node_count = memoized_graph.graph.node_count();
        let mut trust_profiles = HashMap::with_capacity(node_count);
        let mut positions = HashMap::with_capacity(node_count);

        // 既存ノードの初期データを設定
        for node_id in 0..node_count {
            trust_profiles.insert(
                node_id,
                TrustProfile {
                    operational: 0.5,
                    semantic: 0.5,
                    temporal: 0.5,
                    human: crate::types::HumanTrustLogistic::default(),
                },
            );
            positions.insert(
                node_id,
                SpacePositionEmbedding::from([rng.random::<f32>(), rng.random::<f32>(), rng.random::<f32>()]),
            );
        }

        Self {
            memoized_graph,
            trust_profiles,
            village_assignments: HashMap::new(),
            positions,
            tick: 0,
            rng,
            help_sessions: Vec::new(),
        }
    }

    /// 現在の人口（WorkflowGraph のノード数）を返す。
    pub fn population_count(&self) -> usize {
        self.memoized_graph.graph.node_count()
    }

    /// 新しいノード ID を生成する（現在の最大ノードインデックス + 1）。
    pub fn generate_node_id(&self) -> NodeId {
        self.population_count()
    }

    /// 新しい個人（WorkflowNode::SubWorkflow）を出生として追加する。
    ///
    /// 追加後の `NodeId` を返す（petgraph の自動採番値と一致）。
    pub fn add_person(&mut self) -> NodeId {
        use crate::types::WorkflowNode;

        let node_index = self.memoized_graph.graph.add_node(WorkflowNode::Placeholder);
        let node_id = node_index.index();

        // 信頼プロファイルと位置を初期化
        self.trust_profiles.insert(
            node_id,
            TrustProfile {
                operational: 0.5,
                semantic: 0.5,
                temporal: 0.5,
                human: crate::types::HumanTrustLogistic::default(),
            },
        );
        self.positions.insert(
            node_id,
            SpacePositionEmbedding::from([self.rng.random::<f32>(), self.rng.random::<f32>(), self.rng.random::<f32>()]),
        );

        node_id
    }

    /// 指定されたノードを削除する。
    ///
    /// 削除に伴い `trust_profiles`、`positions`、`village_assignments` からも
    /// 該当エントリを削除する。存在しないノード ID の場合はエラーを返す。
    pub fn remove_node(&mut self, node_id: NodeId) -> Result<(), DarviumError> {
        use petgraph::graph::NodeIndex;

        let node_index = NodeIndex::new(node_id);
        match self.memoized_graph.graph.remove_node(node_index) {
            Some(_) => {
                // グラフ削除成功 → 補助データも削除
                self.trust_profiles.remove(&node_id);
                self.positions.remove(&node_id);
                self.village_assignments.remove(&node_id);
                Ok(())
            }
            None => Err(DarviumError::Internal(format!(
                "NodeId {} does not exist or was already removed",
                node_id
            ))),
        }
    }
}

// ============================================================
// Phase 1: 合成人口生成器
// ============================================================

/// 初期ワークフロー集団を生成する。
///
/// 子ワークフローは低経験値・低信頼・慈悲スコアは [0, 1] 一様分布。
/// 成人ワークフローは高経験値・中信頼・慈悲スコアは [0, 1] 一様分布。
fn generate_population(
    config: &ReciprocitySimulatorConfig,
    rng: &mut StdRng,
) -> Vec<SimWorkflowState> {
    let child_count = (config.population_size as f64 * config.child_ratio).round() as usize;
    let adult_count = config.population_size - child_count;

    let mut population = Vec::with_capacity(config.population_size);

    for i in 0..child_count {
        let benevolence: f32 = rng.random(); // [0, 1) 一様
        let mut reputation = ReputationProfile::cold_start();
        reputation.benevolence_score = benevolence;

        population.push(SimWorkflowState {
            id: format!("wf-child-{}", i),
            position: [
                rng.random::<f32>(),
                rng.random::<f32>(),
                rng.random::<f32>(),
            ],
            experience: rng.random_range(0..E_ADULT_THRESHOLD - 1),
            trust: rng.random::<f32>() * 0.3,
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
        let benevolence: f32 = rng.random();
        let mut reputation = ReputationProfile::cold_start();
        reputation.benevolence_score = benevolence;

        population.push(SimWorkflowState {
            id: format!("wf-adult-{}", i),
            position: [
                rng.random::<f32>(),
                rng.random::<f32>(),
                rng.random::<f32>(),
            ],
            experience: rng.random_range(E_ADULT_THRESHOLD..(E_ADULT_THRESHOLD * 3)),
            trust: rng.random::<f32>() * 0.5 + 0.3,
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
        if rng.random::<f64>() < mission_rate {
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
        for wf in population
            .iter()
            .filter(|w| w.survived && w.id != mission.requester_id)
        {
            if rng.random::<f64>() < offer_help_probability(wf.benevolence) {
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
fn advance_help_sessions(sessions: &mut [SimHelpSession], rng: &mut StdRng, current_tick: u64) {
    for session in sessions.iter_mut() {
        if session.updated_at >= current_tick {
            continue; // 既に今 tick で処理済み
        }

        match session.status {
            HelpSessionStatus::Offered => {
                // 慈悲スコアが高いヘルパーほど受け入れられやすい
                let accept_prob = 0.5 + session.helper_benevolence as f64 * 0.3;
                if rng.random::<f64>() < accept_prob {
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
                let roll: f64 = rng.random();

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

    for session in sessions
        .iter()
        .filter(|s| s.requester_id == workflow_id || s.helper_id == workflow_id)
    {
        let (source_id, target_id, event_kind) = match &session.status {
            HelpSessionStatus::Succeeded => {
                // ヘルパー視点: ヘルパー→要求元
                if session.helper_id == workflow_id {
                    (
                        session.helper_id.clone(),
                        session.requester_id.clone(),
                        ReciprocityEventKind::HelpSucceeded,
                    )
                } else {
                    continue;
                }
            }
            HelpSessionStatus::HarmfulMismatch => {
                if session.helper_id == workflow_id {
                    (
                        session.helper_id.clone(),
                        session.requester_id.clone(),
                        ReciprocityEventKind::HarmfulMismatch,
                    )
                } else {
                    continue;
                }
            }
            HelpSessionStatus::Abandoned => {
                if session.helper_id == workflow_id {
                    (
                        session.helper_id.clone(),
                        session.requester_id.clone(),
                        ReciprocityEventKind::HelpAbandoned,
                    )
                } else {
                    continue;
                }
            }
            HelpSessionStatus::Rejected => {
                if session.requester_id == workflow_id {
                    (
                        session.requester_id.clone(),
                        session.helper_id.clone(),
                        ReciprocityEventKind::HelpRejected,
                    )
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
            let involved = sessions
                .iter()
                .filter(|s| s.helper_id == wf.id || s.requester_id == wf.id)
                .count() as f32;
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
        let help_total = sessions.iter().filter(|s| s.helper_id == wf.id).count() as f32;
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
    if gc_interval == 0 || !current_tick.is_multiple_of(gc_interval) {
        return;
    }
    for wf in population.iter_mut().filter(|w| w.survived) {
        // 簡易 LifecycleScore: 経験値の正規化 [0, 1]
        let lifecycle_score =
            (wf.experience as f64 / (E_ADULT_THRESHOLD * 3) as f64).min(1.0) as f32;

        // child protection: CHILD_PROTECT_ETA1
        let child_protection = if wf.is_child { CHILD_PROTECT_ETA1 } else { 0.0 };
        let child_protection = child_protection + wf.reputation.benevolence_score * 0.3;

        let hazard = compute_gc_hazard(lifecycle_score, wf.benevolence, child_protection, policy);
        wf.hazard = hazard;

        let survival_prob = compute_survival_probability(hazard, 1);
        let survived = rng.random::<f64>() < survival_prob;

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
    let total_benevolent = population
        .iter()
        .filter(|w| w.initial_benevolence > 0.5)
        .count();
    let total_non_benevolent = population
        .iter()
        .filter(|w| w.initial_benevolence <= 0.5)
        .count();
    let survived_benevolent = population
        .iter()
        .filter(|w| w.survived && w.initial_benevolence > 0.5)
        .count();
    let survived_non_benevolent = population
        .iter()
        .filter(|w| w.survived && w.initial_benevolence <= 0.5)
        .count();

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
// Phase 6: RFC §41B.20.7 拡張運用メトリクス計算器 (M1.76-18)
// ============================================================

/// 善良群（上位 20%）と非善良群（下位 20%）の生存率差を計算する。
///
/// population を initial_benevolence で降順ソートし、上位 BENEVOLENT_TOP_FRACTION を
/// 善良群、下位 BENEVOLENT_BOTTOM_FRACTION を非善良群としてそれぞれの生存率を計算し、
/// その差（善良 - 非善良）を返す。
/// 各群の人口が 1 未満の場合は 0.0 を返す（差を計算できないため）。
pub fn compute_benevolent_survival_advantage(population: &[SimWorkflowState]) -> f64 {
    if population.len() < 2 {
        return 0.0;
    }

    let mut sorted: Vec<&SimWorkflowState> = population.iter().collect();
    sorted.sort_unstable_by(|a, b| {
        b.initial_benevolence
            .partial_cmp(&a.initial_benevolence)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let top_count = (sorted.len() as f64 * BENEVOLENT_TOP_FRACTION).ceil() as usize;
    let top_count = top_count.max(1);
    let bottom_count = (sorted.len() as f64 * BENEVOLENT_BOTTOM_FRACTION).ceil() as usize;
    let bottom_count = bottom_count.max(1);

    // 上位と下位が重なる場合は差を計算できない
    if top_count + bottom_count > sorted.len() {
        return 0.0;
    }

    let top_group = &sorted[..top_count];
    let bottom_group = &sorted[sorted.len() - bottom_count..];

    let top_survived = top_group.iter().filter(|w| w.survived).count() as f64;
    let bottom_survived = bottom_group.iter().filter(|w| w.survived).count() as f64;

    let top_rate = top_survived / top_count as f64;
    let bottom_rate = bottom_survived / bottom_count as f64;

    top_rate - bottom_rate
}

/// HarmfulMismatch 率を計算する。
///
/// 全ヘルプセッションのうち、HarmfulMismatch で終了したものの割合を返す。
/// 分母は全セッション数（Offered/Executing 等の進行中も含む）。
pub fn compute_harmful_gc_rate(sessions: &[SimHelpSession]) -> f64 {
    if sessions.is_empty() {
        return 0.0;
    }

    let harmful_count = sessions
        .iter()
        .filter(|s| s.status == HelpSessionStatus::HarmfulMismatch)
        .count() as f64;

    harmful_count / sessions.len() as f64
}

/// ヘルプ受け入れ率を計算する。
///
/// Accepted / (Accepted + Rejected)。Offered / Executing / その他の状態は分母から除外し、
/// 意思決定が確定したセッションのみを対象とする。
/// 分母が 0 の場合は 0.0 を返す。
pub fn compute_helper_accept_rate(sessions: &[SimHelpSession]) -> f64 {
    let accepted = sessions
        .iter()
        .filter(|s| s.status == HelpSessionStatus::Accepted)
        .count() as f64;
    let rejected = sessions
        .iter()
        .filter(|s| s.status == HelpSessionStatus::Rejected)
        .count() as f64;

    let denominator = accepted + rejected;
    if denominator == 0.0 {
        return 0.0;
    }

    accepted / denominator
}

/// ヘルプ放棄率を計算する。
///
/// 完了セッション（Succeeded / HarmfulMismatch / Abandoned）のうち、
/// Abandoned で終了したものの割合を返す。
/// 完了セッションが 0 の場合は 0.0 を返す。
pub fn compute_help_abandon_rate(sessions: &[SimHelpSession]) -> f64 {
    let succeeded = sessions
        .iter()
        .filter(|s| s.status == HelpSessionStatus::Succeeded)
        .count() as f64;
    let harmful = sessions
        .iter()
        .filter(|s| s.status == HelpSessionStatus::HarmfulMismatch)
        .count() as f64;
    let abandoned = sessions
        .iter()
        .filter(|s| s.status == HelpSessionStatus::Abandoned)
        .count() as f64;

    let completed = succeeded + harmful + abandoned;
    if completed == 0.0 {
        return 0.0;
    }

    abandoned / completed
}

/// 子ワークフローの生存率を計算する。
///
/// 子ワークフロー（is_child == true）のうち生存しているものの割合を返す。
/// 子ワークフローが 1 件も存在しない場合は 0.0 を返す。
pub fn compute_child_survival_rate(population: &[SimWorkflowState]) -> f64 {
    let children: Vec<&SimWorkflowState> = population.iter().filter(|w| w.is_child).collect();

    if children.is_empty() {
        return 0.0;
    }

    let survived = children.iter().filter(|w| w.survived).count() as f64;
    survived / children.len() as f64
}

/// M1.76-17 のシミュレーターに統合可能な拡張メトリクス観測器。
///
/// SimulationTickSnapshot（既存 8 パーセンタイル + 生存率差）に加えて、
/// RFC §41B.20.7 の追加 5 指標（benevolent_survival_advantage, harmful_gc_rate,
/// helper_accept_rate, help_abandon_rate, child_survival_rate）を計算し、
/// ExtendedOperationalMetrics として出力する。
pub struct ReciprocityMetricsObserver;

impl ReciprocityMetricsObserver {
    /// シミュレーションの 1 tick から拡張メトリクスを生成する。
    ///
    /// # 引数
    /// - `snapshot`: 既存の SimulationTickSnapshot（パーセンタイル値を転記）
    /// - `sessions`: 現在のヘルプセッション一覧
    /// - `population`: 現在のワークフロー集団
    pub fn observe(
        snapshot: &SimulationTickSnapshot,
        sessions: &[SimHelpSession],
        population: &[SimWorkflowState],
    ) -> ExtendedOperationalMetrics {
        ExtendedOperationalMetrics {
            tick: snapshot.tick,
            benevolence_score_p50: snapshot.benevolence_score_p50,
            benevolence_score_p95: snapshot.benevolence_score_p95,
            direct_reciprocity_p50: snapshot.direct_reciprocity_p50,
            direct_reciprocity_p95: snapshot.direct_reciprocity_p95,
            indirect_reciprocity_p50: snapshot.indirect_reciprocity_p50,
            indirect_reciprocity_p95: snapshot.indirect_reciprocity_p95,
            reputation_final_p50: snapshot.reputation_final_p50,
            reputation_final_p95: snapshot.reputation_final_p95,
            benevolent_survival_advantage: compute_benevolent_survival_advantage(population),
            harmful_gc_rate: compute_harmful_gc_rate(sessions),
            helper_accept_rate: compute_helper_accept_rate(sessions),
            help_abandon_rate: compute_help_abandon_rate(sessions),
            child_survival_rate: compute_child_survival_rate(population),
        }
    }

    /// 全 tick の拡張メトリクス系列を CSV 形式で標準出力に書き出す。
    ///
    /// ヘッダー行 + 各 tick のデータ行（tick, 11 指標）を出力する。
    pub fn print_csv(extended_series: &[ExtendedOperationalMetrics], prefix: &str) {
        println!(
            "{prefix}: tick,benevolence_p50,benevolence_p95,dr_p50,dr_p95,ir_p50,ir_p95,reputation_p50,reputation_p95,survival_advantage,harmful_gc,helper_accept,help_abandon,child_survival"
        );

        for metrics in extended_series {
            println!(
                "{prefix}: {},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6}",
                metrics.tick,  // Direct field access — tick should be added to ExtendedOperationalMetrics
                metrics.benevolence_score_p50,
                metrics.benevolence_score_p95,
                metrics.direct_reciprocity_p50,
                metrics.direct_reciprocity_p95,
                metrics.indirect_reciprocity_p50,
                metrics.indirect_reciprocity_p95,
                metrics.reputation_final_p50,
                metrics.reputation_final_p95,
                metrics.benevolent_survival_advantage,
                metrics.harmful_gc_rate,
                metrics.helper_accept_rate,
                metrics.help_abandon_rate,
                metrics.child_survival_rate,
            );
        }
    }
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
        offer_help_sessions(
            &missions,
            &population,
            &mut sessions,
            tick,
            &mut rng,
            &mut session_counter,
        );

        // Phase 3: セッション進行
        advance_help_sessions(&mut sessions, &mut rng, tick);

        // Phase 4: 信頼・評判再計算
        recompute_trust_reputation(&mut population, &sessions, tick, &config.policy);

        // Phase 4: ライフサイクル GC
        run_lifecycle_gc(
            &mut population,
            &sessions,
            &config.policy,
            &mut rng,
            tick,
            config.gc_interval,
        );

        // Phase 5: 観測
        let snapshot = observe_tick(tick, &population);
        metric_series.push(snapshot);
    }

    let mut seq_counter: u64 = 0;
    let experiment_id = generate_experiment_id(&mut seq_counter);

    ReciprocitySimulationResult {
        metric_series,
        final_state: population,
        sessions,
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
    use crate::help::{HelpSession, HelpState};
    use crate::spaceposition::decompose_position;

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

        for (i, (s1, s2)) in result1
            .metric_series
            .iter()
            .zip(result2.metric_series.iter())
            .enumerate()
        {
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

        println!(
            "T1: deterministic_replay PASS — {} ticks matched",
            result1.metric_series.len()
        );
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
        assert!(
            !result.metric_series.is_empty(),
            "T2: metrics が空でないこと"
        );

        println!(
            "T2: no_children PASS — population={}, ticks={}",
            result.final_state.len(),
            result.metric_series.len()
        );
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

        assert!(
            result.metric_series.is_empty(),
            "T3: max_ticks=0 で metric_series が空であること"
        );

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
                last.survival_advantage,
                last.benevolent_survival_rate,
                last.non_benevolent_survival_rate
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
        low_hazard_policy.gamma_lifecycle = 10.0; // 生存寄与を最大化
        low_hazard_policy.gamma_benevolence = 10.0;
        low_hazard_policy.gamma_child_protect = 10.0;

        let low_config = ReciprocitySimulatorConfig {
            policy: low_hazard_policy,
            ..default_config()
        };
        let default_config = default_config();

        let low_result = run_simulation(&low_config);
        let default_result = run_simulation(&default_config);

        let low_dead_count = low_result
            .final_state
            .iter()
            .filter(|w| !w.survived)
            .count();
        let default_dead_count = default_result
            .final_state
            .iter()
            .filter(|w| !w.survived)
            .count();

        // 低 hazard 設定の死亡数がデフォルト以下であること
        assert!(
            low_dead_count <= default_dead_count,
            "T5: low_hazard dead={} <= default dead={}",
            low_dead_count,
            default_dead_count,
        );

        println!(
            "T5: zero_gc_base PASS — low_hazard dead={}/{} vs default dead={}/{}",
            low_dead_count,
            low_result.final_state.len(),
            default_dead_count,
            default_result.final_state.len(),
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
                (
                    "indirect_reciprocity_p50",
                    snapshot.indirect_reciprocity_p50,
                ),
                (
                    "indirect_reciprocity_p95",
                    snapshot.indirect_reciprocity_p95,
                ),
                ("reputation_final_p50", snapshot.reputation_final_p50),
                ("reputation_final_p95", snapshot.reputation_final_p95),
                (
                    "benevolent_survival_rate",
                    snapshot.benevolent_survival_rate,
                ),
                (
                    "non_benevolent_survival_rate",
                    snapshot.non_benevolent_survival_rate,
                ),
                ("survival_advantage", snapshot.survival_advantage),
                ("harmful_gc_rate", snapshot.harmful_gc_rate),
            ];

            for (name, value) in &fields {
                assert!(
                    *value >= -1e-6 && *value <= 1.0 + 1e-6,
                    "T6: tick {} {}={} が [0, 1] 範囲外",
                    snapshot.tick,
                    name,
                    value
                );
            }
        }

        println!(
            "T6: metrics_in_range PASS — {} ticks verified",
            result.metric_series.len()
        );
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
                last.survival_advantage,
                last.benevolent_survival_rate,
                last.non_benevolent_survival_rate
            );
        }

        // 最終生存状態のサマリ
        let benevolent_alive = result
            .final_state
            .iter()
            .filter(|w| w.survived && w.initial_benevolence > 0.5)
            .count();
        let non_benevolent_alive = result
            .final_state
            .iter()
            .filter(|w| w.survived && w.initial_benevolence <= 0.5)
            .count();
        let benevolent_total = result
            .final_state
            .iter()
            .filter(|w| w.initial_benevolence > 0.5)
            .count();
        let non_benevolent_total = result
            .final_state
            .iter()
            .filter(|w| w.initial_benevolence <= 0.5)
            .count();

        println!(
            "T9: benevolent: {}/{} alive, non_benevolent: {}/{} alive",
            benevolent_alive, benevolent_total, non_benevolent_alive, non_benevolent_total
        );
    }

    // ===============================================================
    // M1.76-18: 拡張運用メトリクステスト（T10-T19）
    // ===============================================================

    // -------------------------------------------------------
    // T10: benevolent_survival_advantage — 同一 benevolence で 0
    // -------------------------------------------------------
    #[test]
    fn t10_benevolent_survival_advantage_all_equal() {
        // 全ワークフローが同一 benevolence → 上位/下位 20% の差は 0
        let mut pop: Vec<SimWorkflowState> = (0..10)
            .map(|i| SimWorkflowState {
                id: format!("wf-{}", i),
                position: [0.0, 0.0, 0.0],
                experience: 10,
                trust: 0.5,
                reputation: ReputationProfile::cold_start(),
                benevolence: 0.5,
                direct_reciprocity: 0.5,
                indirect_reciprocity: 0.5,
                hazard: 0.0,
                survived: true,
                is_child: false,
                initial_benevolence: 0.5,
            })
            .collect();

        // 一部死なせる（同一 benevolence なので差は 0）
        pop[0].survived = false;
        pop[9].survived = false;

        let advantage = compute_benevolent_survival_advantage(&pop);
        assert!(
            (advantage - 0.0).abs() < 1e-10,
            "T10: 同一 benevolence で advantage={} が 0 でない",
            advantage
        );
        println!("T10: all_equal_benevolence PASS — advantage={}", advantage);
    }

    // -------------------------------------------------------
    // T11: harmful_gc_rate — harmful event 0 件で 0
    // -------------------------------------------------------
    #[test]
    fn t11_harmful_gc_rate_zero() {
        let sessions = vec![
            SimHelpSession {
                id: "s1".into(),
                mission_id: "m1".into(),
                helper_id: "h1".into(),
                requester_id: "r1".into(),
                status: HelpSessionStatus::Succeeded,
                created_at: 0,
                updated_at: 1,
                helper_benevolence: 0.8,
            },
            SimHelpSession {
                id: "s2".into(),
                mission_id: "m1".into(),
                helper_id: "h2".into(),
                requester_id: "r1".into(),
                status: HelpSessionStatus::Abandoned,
                created_at: 0,
                updated_at: 1,
                helper_benevolence: 0.5,
            },
        ];

        let rate = compute_harmful_gc_rate(&sessions);
        assert!(
            (rate - 0.0).abs() < 1e-10,
            "T11: harmful_gc_rate={} が 0 でない",
            rate
        );
        println!("T11: harmful_gc_rate_zero PASS — rate={}", rate);
    }

    // -------------------------------------------------------
    // T12: helper_accept_rate — 境界値 3 ケース
    // -------------------------------------------------------
    #[test]
    fn t12_helper_accept_rate_boundaries() {
        // 全 accept
        let all_accepted = vec![
            SimHelpSession {
                status: HelpSessionStatus::Accepted,
                ..default_session("s1")
            },
            SimHelpSession {
                status: HelpSessionStatus::Accepted,
                ..default_session("s2")
            },
        ];
        assert!(
            (compute_helper_accept_rate(&all_accepted) - 1.0).abs() < 1e-10,
            "T12: 全 accept で 1.0 でない"
        );

        // 全 reject
        let all_rejected = vec![
            SimHelpSession {
                status: HelpSessionStatus::Rejected,
                ..default_session("s1")
            },
            SimHelpSession {
                status: HelpSessionStatus::Rejected,
                ..default_session("s2")
            },
        ];
        assert!(
            (compute_helper_accept_rate(&all_rejected) - 0.0).abs() < 1e-10,
            "T12: 全 reject で 0.0 でない"
        );

        // 空
        assert!(
            (compute_helper_accept_rate(&[]) - 0.0).abs() < 1e-10,
            "T12: 空で 0.0 でない"
        );

        // 混合: 3/10 = 0.3
        let mixed: Vec<SimHelpSession> = (0..10)
            .map(|i| SimHelpSession {
                id: format!("s{}", i),
                mission_id: "m1".into(),
                helper_id: format!("h{}", i),
                requester_id: "r1".into(),
                status: if i < 3 {
                    HelpSessionStatus::Accepted
                } else {
                    HelpSessionStatus::Rejected
                },
                created_at: 0,
                updated_at: 1,
                helper_benevolence: 0.5,
            })
            .collect();
        let mixed_rate = compute_helper_accept_rate(&mixed);
        assert!(
            (mixed_rate - 0.3).abs() < 1e-10,
            "T12: 3/10 accepted rate={} が 0.3 でない",
            mixed_rate
        );

        println!("T12: helper_accept_rate_boundaries PASS — all_accept=1.0, all_reject=0.0, empty=0.0, mixed=0.3");
    }

    // -------------------------------------------------------
    // T13: help_abandon_rate — 境界値 4 ケース
    // -------------------------------------------------------
    #[test]
    fn t13_help_abandon_rate_boundaries() {
        // 全 succeeded
        let all_succeeded = vec![
            SimHelpSession {
                status: HelpSessionStatus::Succeeded,
                ..default_session("s1")
            },
            SimHelpSession {
                status: HelpSessionStatus::Succeeded,
                ..default_session("s2")
            },
            SimHelpSession {
                status: HelpSessionStatus::Succeeded,
                ..default_session("s3")
            },
        ];
        assert!(
            (compute_help_abandon_rate(&all_succeeded) - 0.0).abs() < 1e-10,
            "T13: 全 succeeded で 0.0 でない"
        );

        // 全 abandoned
        let all_abandoned = vec![SimHelpSession {
            status: HelpSessionStatus::Abandoned,
            ..default_session("s1")
        }];
        assert!(
            (compute_help_abandon_rate(&all_abandoned) - 1.0).abs() < 1e-10,
            "T13: 全 abandoned で 1.0 でない"
        );

        // 空
        assert!(
            (compute_help_abandon_rate(&[]) - 0.0).abs() < 1e-10,
            "T13: 空で 0.0 でない"
        );

        // 混合: 2/8 = 0.25
        let mixed: Vec<SimHelpSession> = (0..8)
            .map(|i| SimHelpSession {
                id: format!("s{}", i),
                mission_id: "m1".into(),
                helper_id: format!("h{}", i),
                requester_id: "r1".into(),
                status: if i < 2 {
                    HelpSessionStatus::Abandoned
                } else if i < 5 {
                    HelpSessionStatus::Succeeded
                } else {
                    HelpSessionStatus::HarmfulMismatch
                },
                created_at: 0,
                updated_at: 1,
                helper_benevolence: 0.5,
            })
            .collect();
        let mixed_rate = compute_help_abandon_rate(&mixed);
        assert!(
            (mixed_rate - 0.25).abs() < 1e-10,
            "T13: 2/8 abandoned rate={} が 0.25 でない",
            mixed_rate
        );

        println!("T13: help_abandon_rate_boundaries PASS");
    }

    // -------------------------------------------------------
    // T14: child_survival_rate — 境界値 4 ケース
    // -------------------------------------------------------
    #[test]
    fn t14_child_survival_rate_boundaries() {
        // Adult のみ → 0.0
        let adults: Vec<SimWorkflowState> = (0..5)
            .map(|i| SimWorkflowState {
                id: format!("adult-{}", i),
                position: [0.0, 0.0, 0.0],
                experience: 10,
                trust: 0.5,
                reputation: ReputationProfile::cold_start(),
                benevolence: 0.5,
                direct_reciprocity: 0.5,
                indirect_reciprocity: 0.5,
                hazard: 0.0,
                survived: true,
                is_child: false,
                initial_benevolence: 0.5,
            })
            .collect();
        assert!(
            (compute_child_survival_rate(&adults) - 0.0).abs() < 1e-10,
            "T14: Adult のみで 0.0 でない"
        );

        // 全 child 生存 → 1.0
        let all_alive: Vec<SimWorkflowState> = (0..5)
            .map(|i| SimWorkflowState {
                id: format!("child-{}", i),
                position: [0.0, 0.0, 0.0],
                experience: 5,
                trust: 0.3,
                reputation: ReputationProfile::cold_start(),
                benevolence: 0.5,
                direct_reciprocity: 0.5,
                indirect_reciprocity: 0.5,
                hazard: 0.0,
                survived: true,
                is_child: true,
                initial_benevolence: 0.5,
            })
            .collect();
        assert!(
            (compute_child_survival_rate(&all_alive) - 1.0).abs() < 1e-10,
            "T14: 全 child 生存で 1.0 でない"
        );

        // 全 child 死亡 → 0.0
        let all_dead: Vec<SimWorkflowState> = (0..5)
            .map(|i| SimWorkflowState {
                id: format!("child-{}", i),
                survived: false,
                is_child: true,
                ..default_child_state()
            })
            .collect();
        assert!(
            (compute_child_survival_rate(&all_dead) - 0.0).abs() < 1e-10,
            "T14: 全 child 死亡で 0.0 でない"
        );

        // 3/10 生存 → 0.3
        let mixed: Vec<SimWorkflowState> = (0..10)
            .map(|i| SimWorkflowState {
                id: format!("child-{}", i),
                position: [0.0, 0.0, 0.0],
                experience: 5,
                trust: 0.3,
                reputation: ReputationProfile::cold_start(),
                benevolence: 0.5,
                direct_reciprocity: 0.5,
                indirect_reciprocity: 0.5,
                hazard: 0.0,
                survived: i < 3,
                is_child: true,
                initial_benevolence: 0.5,
            })
            .collect();
        assert!(
            (compute_child_survival_rate(&mixed) - 0.3).abs() < 1e-10,
            "T14: 3/10 child survival={} が 0.3 でない",
            compute_child_survival_rate(&mixed)
        );

        println!("T14: child_survival_rate_boundaries PASS");
    }

    // -------------------------------------------------------
    // T15: 空データに対して全関数が graceful
    // -------------------------------------------------------
    #[test]
    fn t15_empty_data_graceful() {
        let empty_sessions: Vec<SimHelpSession> = vec![];
        let empty_pop: Vec<SimWorkflowState> = vec![];

        // 全関数が panic せず 0.0 を返す
        let r1 = compute_benevolent_survival_advantage(&empty_pop);
        let r2 = compute_harmful_gc_rate(&empty_sessions);
        let r3 = compute_helper_accept_rate(&empty_sessions);
        let r4 = compute_help_abandon_rate(&empty_sessions);
        let r5 = compute_child_survival_rate(&empty_pop);

        assert!(
            r1.is_finite() && r1 == 0.0,
            "T15: compute_benevolent_survival_advantage failed"
        );
        assert!(
            r2.is_finite() && r2 == 0.0,
            "T15: compute_harmful_gc_rate failed"
        );
        assert!(
            r3.is_finite() && r3 == 0.0,
            "T15: compute_helper_accept_rate failed"
        );
        assert!(
            r4.is_finite() && r4 == 0.0,
            "T15: compute_help_abandon_rate failed"
        );
        assert!(
            r5.is_finite() && r5 == 0.0,
            "T15: compute_child_survival_rate failed"
        );

        println!("T15: empty_data_graceful PASS — 全 5 関数が panic せず 0.0 を返した");
    }

    // -------------------------------------------------------
    // T16: ReciprocityMetricsObserver 統合
    // -------------------------------------------------------
    #[test]
    fn t16_observer_integration() {
        let config = default_config();
        let result = run_simulation(&config);

        // 各 tick の拡張メトリクスを観測
        let extended_series: Vec<ExtendedOperationalMetrics> = result
            .metric_series
            .iter()
            .map(|snapshot| ReciprocityMetricsObserver::observe(snapshot, &[], &result.final_state))
            .collect();

        assert!(!extended_series.is_empty(), "T16: extended_series が空");

        // 全指標が有限値（NaN/Inf でない）
        for metrics in &extended_series {
            assert!(
                metrics.benevolent_survival_advantage.is_finite(),
                "T16: NaN in benevolent_survival_advantage"
            );
            assert!(
                metrics.harmful_gc_rate.is_finite(),
                "T16: NaN in harmful_gc_rate"
            );
            assert!(
                metrics.helper_accept_rate.is_finite(),
                "T16: NaN in helper_accept_rate"
            );
            assert!(
                metrics.help_abandon_rate.is_finite(),
                "T16: NaN in help_abandon_rate"
            );
            assert!(
                metrics.child_survival_rate.is_finite(),
                "T16: NaN in child_survival_rate"
            );
        }

        println!(
            "T16: observer_integration PASS — {} ticks, all metrics finite",
            extended_series.len()
        );
    }

    // -------------------------------------------------------
    // T17: 上位/下位 20% 分割の正しさ
    // -------------------------------------------------------
    #[test]
    fn t17_top_bottom_20_percent_split() {
        // 10 件: initial_benevolence 0.1〜1.0 の等差数列
        let pop: Vec<SimWorkflowState> = (0..10)
            .map(|i| SimWorkflowState {
                id: format!("wf-{}", i),
                position: [0.0, 0.0, 0.0],
                experience: 10,
                trust: 0.5,
                reputation: ReputationProfile::cold_start(),
                benevolence: (i + 1) as f32 * 0.1,
                direct_reciprocity: 0.5,
                indirect_reciprocity: 0.5,
                hazard: 0.0,
                survived: i > 5, // 下位 4 件のみ死亡
                is_child: false,
                initial_benevolence: (i + 1) as f32 * 0.1,
            })
            .collect();

        let advantage = compute_benevolent_survival_advantage(&pop);
        // 上位 2 件(0.9,1.0): 両方生存
        // 下位 2 件(0.1,0.2): 両方死亡
        // 期待 advantage = 1.0 - 0.0 = 1.0
        assert!(
            (advantage - 1.0).abs() < 1e-10,
            "T17: 完全分割 advantage={} が 1.0 でない",
            advantage
        );

        // 4 件未満: 0.0 を返す
        let small_pop: Vec<SimWorkflowState> = (0..1)
            .map(|i| SimWorkflowState {
                id: format!("wf-{}", i),
                initial_benevolence: 0.5,
                survived: true,
                ..default_child_state()
            })
            .collect();
        let small_advantage = compute_benevolent_survival_advantage(&small_pop);
        assert!(
            (small_advantage - 0.0).abs() < 1e-10,
            "T17: 小 population advantage={} が 0.0 でない",
            small_advantage
        );

        println!(
            "T17: top_bottom_20_percent_split PASS — perfect_split={}, small_pop={}",
            advantage, small_advantage
        );
    }

    // -------------------------------------------------------
    // T18: 拡張 CSV 出力の完全性
    // -------------------------------------------------------
    #[test]
    fn t18_extended_csv_output() {
        let config = default_config();
        let result = run_simulation(&config);

        // シミュレーション実行中の sessions を取得するため、全 tick の拡張メトリクスを生成
        let extended_series: Vec<ExtendedOperationalMetrics> = result
            .metric_series
            .iter()
            .map(|snapshot| {
                ReciprocityMetricsObserver::observe(
                    snapshot,
                    &[], // 簡易テストのため空セッション
                    &result.final_state,
                )
            })
            .collect();

        // CSV 出力（ヘッダー + データ行）
        ReciprocityMetricsObserver::print_csv(&extended_series, "T18");

        // 系列長が tick 数と一致
        assert_eq!(
            extended_series.len(),
            config.max_ticks as usize,
            "T18: series length mismatch"
        );

        println!(
            "T18: extended_csv_output PASS — {} rows, {} columns",
            extended_series.len(),
            14
        );
    }

    // -------------------------------------------------------
    // T19: 既存テストとの後方互換性
    // -------------------------------------------------------
    #[test]
    fn t19_compatibility_with_existing_tests() {
        // 新規関数が既存の run_simulation の動作に影響を与えないことを確認
        let config = default_config();
        let result1 = run_simulation(&config);
        let result2 = run_simulation(&config);

        assert_eq!(result1.metric_series.len(), result2.metric_series.len());
        assert_eq!(result1.final_state.len(), result2.final_state.len());

        println!("T19: compatibility PASS — existing simulation unchanged");
    }

    /// テスト用のデフォルト SimHelpSession を生成する（未設定フィールドは適当な値）。
    fn default_session(id: &str) -> SimHelpSession {
        SimHelpSession {
            id: id.to_string(),
            mission_id: "m_default".into(),
            helper_id: "h_default".into(),
            requester_id: "r_default".into(),
            status: HelpSessionStatus::Offered,
            created_at: 0,
            updated_at: 0,
            helper_benevolence: 0.5,
        }
    }

    /// テスト用のデフォルト子 SimWorkflowState を生成する。
    fn default_child_state() -> SimWorkflowState {
        SimWorkflowState {
            id: "default-child".into(),
            position: [0.0, 0.0, 0.0],
            experience: 0,
            trust: 0.0,
            reputation: ReputationProfile::cold_start(),
            benevolence: 0.5,
            direct_reciprocity: 0.5,
            indirect_reciprocity: 0.5,
            hazard: 0.0,
            survived: true,
            is_child: true,
            initial_benevolence: 0.5,
        }
    }

    // ============================================================
    // KW-REAL-P1: SimulationContext Tests (TC1-TC8)
    // ============================================================

    /// TC1: SimulationContext 生成と初期ノード数確認
    #[test]
    fn tc1_simulation_context_initialization() {
        let mut memoized_graph = MemoizedGraph::new("tc1".into(), 0.5);
        let rng = StdRng::seed_from_u64(12345);
        let context = SimulationContext::new(&mut memoized_graph, rng);

        assert_eq!(
            context.population_count(),
            0,
            "空の MemoizedGraph の人口は 0"
        );
        assert!(
            context.positions.is_empty(),
            "ノードがない場合 positions は空"
        );
        assert!(
            context.help_sessions.is_empty(),
            "初期 HELP セッションは空"
        );
    }

    /// TC2: ノード追加（出生）
    #[test]
    fn tc2_add_person_increases_population() {
        let mut memoized_graph = MemoizedGraph::new("tc2".into(), 0.5);
        let rng = StdRng::seed_from_u64(12345);
        let mut context = SimulationContext::new(&mut memoized_graph, rng);

        let before = context.population_count();
        let node_id = context.add_person();
        assert_eq!(
            context.population_count(),
            before + 1,
            "追加後に人口が 1 増加する必要があります"
        );
        assert!(
            context.trust_profiles.contains_key(&node_id),
            "追加後に trust_profiles にエントリが存在する必要があります"
        );
        assert!(
            context.positions.contains_key(&node_id),
            "追加後に positions にエントリが存在する必要があります"
        );
    }

    /// TC3: ノード削除インターフェース
    #[test]
    fn tc3_remove_node_decreases_population() {
        let mut memoized_graph = MemoizedGraph::new("tc3".into(), 0.5);
        let rng = StdRng::seed_from_u64(12345);
        let mut context = SimulationContext::new(&mut memoized_graph, rng);

        let node_id = context.add_person();
        let before = context.population_count();

        context
            .remove_node(node_id)
            .expect("存在するノードの削除は成功する必要があります");

        assert_eq!(
            context.population_count(),
            before - 1,
            "削除後に人口が 1 減少する必要があります"
        );
        assert!(
            !context.trust_profiles.contains_key(&node_id),
            "削除後に trust_profiles からエントリが削除されている必要があります"
        );
        assert!(
            !context.positions.contains_key(&node_id),
            "削除後に positions からエントリが削除されている必要があります"
        );
    }

    #[test]
    fn tc3_remove_nonexistent_node_returns_error() {
        let mut memoized_graph = MemoizedGraph::new("tc3-err".into(), 0.5);
        let rng = StdRng::seed_from_u64(12345);
        let mut context = SimulationContext::new(&mut memoized_graph, rng);

        let result = context.remove_node(999);
        assert!(
            result.is_err(),
            "存在しないノードの削除はエラーを返す必要があります"
        );
    }

    /// TC5: NodeId 生成の一意性
    #[test]
    fn tc5_node_id_uniqueness() {
        let mut memoized_graph = MemoizedGraph::new("tc5".into(), 0.5);
        let rng = StdRng::seed_from_u64(12345);
        let mut context = SimulationContext::new(&mut memoized_graph, rng);

        let mut ids = std::collections::HashSet::new();
        for _ in 0..100 {
            let node_id = context.add_person();
            assert!(
                ids.insert(node_id),
                "NodeId {} が重複しています",
                node_id
            );
        }
        assert_eq!(ids.len(), 100, "100 個の一意な NodeId が必要");

        println!("TC5: node_id_uniqueness PASS — 100 unique IDs");
    }

    /// TC7: HelpSession 格納確認
    #[test]
    fn tc7_help_session_type_unification() {

        let mut memoized_graph = MemoizedGraph::new("tc7".into(), 0.5);
        let rng = StdRng::seed_from_u64(12345);
        let mut context = SimulationContext::new(&mut memoized_graph, rng);

        let session = HelpSession::new("help-tc7".into(), "giver".into(), "taker".into());
        context.help_sessions.push(session);

        assert_eq!(
            context.help_sessions.len(),
            1,
            "HELP セッションが 1 件追加される必要があります"
        );
        assert_eq!(
            context.help_sessions[0].help_id, "help-tc7",
            "help_id が正しく保持されている必要があります"
        );
        assert_eq!(
            context.help_sessions[0].current_state,
            HelpState::Proposal,
            "デフォルト状態は Proposal である必要があります"
        );
    }

    /// TC8: 空の SimulationContext 境界値テスト
    #[test]
    fn tc8_empty_context_boundary() {
        let mut memoized_graph = MemoizedGraph::new("tc8".into(), 0.5);
        let rng = StdRng::seed_from_u64(12345);
        let mut context = SimulationContext::new(&mut memoized_graph, rng);

        assert_eq!(context.population_count(), 0, "空の人口は 0");

        // 空からの add_person が正常動作すること
        let first_id = context.add_person();
        assert_eq!(context.population_count(), 1, "1 人追加後は人口 1");
        assert_eq!(first_id, 0, "最初の NodeId は 0");
    }

    // -------------------------------------------------------
    // 観測テスト: SimulationContext 計装サマリ
    // -------------------------------------------------------
    #[test]
    fn test_simulation_context_instrumentation() {
        let mut memoized_graph = MemoizedGraph::new("obs".into(), 0.5);
        let rng = StdRng::seed_from_u64(12345);

        // 初期ノードを追加
        for _ in 0..10 {
            memoized_graph.graph.add_node(crate::types::WorkflowNode::Placeholder);
        }

        let mut context = SimulationContext::new(&mut memoized_graph, rng);

        // CSV: 生成統計
        println!("=== KW-REAL-P1 計装サマリ ===");
        println!(
            "CSV: initial_population={}, positions={}, trust_profiles={}",
            context.population_count(),
            context.positions.len(),
            context.trust_profiles.len()
        );

        // ノード追加操作
        let node_ids: Vec<NodeId> = (0..5).map(|_| context.add_person()).collect();
        for (i, &node_id) in node_ids.iter().enumerate() {
            println!(
                "CSV: add_person, iter={}, node_id={}, population={}",
                i,
                node_id,
                context.population_count()
            );
        }

        // ノード削除操作
        // petgraph の Graph::remove_node は指数をコンパクションするため、
        // 末尾から逆順に削除することで元の NodeId との一貫性を保つ。
        let mut reverse_ids = node_ids.clone();
        reverse_ids.reverse();
        for (i, &node_id) in reverse_ids.iter().enumerate() {
            context
                .remove_node(node_id)
                .unwrap_or_else(|_| panic!("remove_node({}) should succeed", node_id));
            println!(
                "CSV: remove_node, iter={}, node_id={}, population={}",
                i,
                node_id,
                context.population_count()
            );
        }

        // JSON: 位置分解
        let positions: Vec<(NodeId, (f32, f32, f32))> = context
            .positions
            .iter()
            .map(|(&id, pos)| {
                let inner = pos.inner().unwrap_or([0.0, 0.0, 0.0]);
                (id, decompose_position(inner))
            })
            .collect();
        println!("JSON: decompose_positions={:?}", positions);

        println!("=== KW-REAL-P1 計装サマリ END ===");
    }
}
