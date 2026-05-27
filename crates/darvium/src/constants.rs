// Darvium 定数定義
//
// 本ファイルは較正対象の全定数を一元管理する。
// 定数の分類:
// - Safety Invariants: RFC 改訂なしでは変更禁止
// - Environment Policy Knobs: デプロイ環境ごとの調整が可能
// - Calibration Candidates: 実験的チューニング対象

/// 信頼継承減衰係数 (Calibration Candidate)
/// Default: 0.70, 感度分析推奨範囲: 0.50-0.90
pub const TRUST_INHERIT_DECAY: f64 = 0.70;

/// 時間減衰 λ_use (Temporal Lambda for Usage)
pub const TEMPORAL_LAMBDA_USE: f64 = 0.0001;

/// 時間減衰 λ_verify (Temporal Lambda for Verification)
/// 設計不変条件: MUST be less than TEMPORAL_LAMBDA_USE
pub const TEMPORAL_LAMBDA_VERIFY: f64 = 0.00005;

/// 時間ブレンド α (Temporal Alpha Blend)
pub const TEMPORAL_ALPHA_BLEND: f64 = 0.35;

/// 人間信頼 K (Human Trust K)
pub const HUMAN_TRUST_K: f64 = 0.08;

/// 自己信頼ディスカウント
pub const SELF_CONF_DISCOUNT: f64 = 0.85;

// === Evaluation Thresholds ===

/// 評価閾値 (Calibration Candidate)
///
/// EvaluateCandidatesStep において、候補スコアがこの値以上の場合に
/// ReuseExisting、未満の場合に PatchExisting を選択する。
/// Default: 0.50, 感度分析推奨範囲: 0.30-0.70
pub const EVALUATION_THRESHOLD: f64 = 0.50;

/// GED ブレンドマージン
pub const GED_BLEND_MARGIN: usize = 5;

/// 最大グラフノード数 (Safety Invariant)
pub const MAX_GRAPH_NODES: usize = 10_000;

/// 最大コンパイルステップ数 (Safety Invariant)
pub const MAX_COMPILED_STEPS: usize = 100_000;

/// 最大パッチ操作数 (Safety Invariant)
pub const MAX_PATCH_OPS: usize = 1_000;

// === Patch Confidence (RFC §12.3) ===

/// パッチ信頼度閾値 (Safety Invariant)
/// RFC §14.2 指定値: 0.75
/// この値未満の confidence は PatchError::LowConfidence として拒否
pub const PATCH_CONFIDENCE_THRESHOLD: f32 = 0.75;

/// パッチ履歴 cold-start prior (Safety Invariant)
/// RFC §12.3 指定値: 0.50
/// 履歴がない場合のデフォルト履歴スコア
pub const PATCH_CONFIDENCE_PRIOR: f32 = 0.50;

/// 自己信頼度重み切り替え閾値 (Safety Invariant)
/// RFC §12.3 指定値: 0.50
/// cₛ < 0.50 で validator 優先 (ws=0.20, wv=0.50) に動的切り替え
pub const PATCH_SELF_CONF_SWITCH_THRESHOLD: f32 = 0.50;

/// 自己信頼度重み ws (通常時) (Safety Invariant)
/// RFC §12.3 指定値: 0.30
pub const PATCH_CONFIDENCE_WS: f32 = 0.30;

/// バリデータ重み wv (通常時) (Safety Invariant)
/// RFC §12.3 指定値: 0.40
pub const PATCH_CONFIDENCE_WV: f32 = 0.40;

/// 自己信頼度重み ws (低自信時) (Calibration Candidate)
/// RFC §12.3 指定値: 0.20
/// 動的切り替え: cₛ < 0.50 時に適用
pub const PATCH_CONFIDENCE_WS_LOW: f32 = 0.20;

/// バリデータ重み wv (低自信時) (Calibration Candidate)
/// RFC §12.3 指定値: 0.50
/// 動的切り替え: cₛ < 0.50 時に適用
pub const PATCH_CONFIDENCE_WV_HIGH: f32 = 0.50;

/// バリデータ変数スコープ違反ペナルティ (Safety Invariant)
/// RFC §14.2 減算規則: 未解決変数1件につき -0.15 (上限3件=計-0.45)
pub const VALIDATOR_VAR_SCOPE_PENALTY: f64 = 0.15;

/// 検索予算 最大プロンプトトークン (Environment Policy Knob)
pub const MAX_PROMPT_TOKENS: u64 = 16_384;

/// SearchBudget デフォルト: 最大イテレーション回数 (Environment Policy Knob)
pub const DEFAULT_MAX_ITERATIONS: u32 = 100;

/// SearchBudget デフォルト: 最大検索呼び出し回数 (Environment Policy Knob)
pub const DEFAULT_MAX_RETRIEVAL_CALLS: u32 = 50;

/// SearchBudget デフォルト: 最大実時間 (ms) (Environment Policy Knob)
pub const DEFAULT_MAX_WALL_CLOCK_MS: u64 = 30_000;

/// RecursionGuard デフォルト: 最大再帰深度 (Safety Invariant)
pub const DEFAULT_RECURSION_MAX_DEPTH: u32 = 8;

/// デフォルトシード値 (テスト用 PRNG)
/// 全ての確率的テストはこのシードを使用し、再現性を保証する
pub const TEST_PRNG_SEED: u64 = 12345;

/// FakeLlmClient のデフォルト不正フォーマット確率
/// 0.0 = 常に正常出力（乱数モード無効）
pub const FAKE_LLM_DEFAULT_MALFORMED_PROB: f64 = 0.0;

/// ScriptedFakeLlmClient のデフォルト不正フォーマット確率
/// 0.0 = 常に正常出力（スクリプトモード無効）
pub const SCRIPTED_FAKE_LLM_DEFAULT_MALFORMED_PROB: f64 = 0.0;

/// FakeEmbeddingProvider のデフォルト埋め込み次元数 (Calibration Candidate)
/// Default: 384, 感度分析推奨範囲: 64-1536
pub const FAKE_EMBEDDING_DEFAULT_DIMENSION: usize = 384;

/// HNSW Mock のデフォルト埋め込み次元数 (Safety Invariant)
/// 1536 は OpenAI text-embedding-3-small 等の実フォーマット形状に基づく。
/// RFC §12.2 Stage 2a/2b の ANN 次元数として固定。
pub const HNSW_MOCK_DEFAULT_DIMENSION: usize = 1536;

/// VirtualClock のデフォルト開始時刻 (ms) (Safety Invariant)
/// 0 = UNIX epoch (1970-01-01T00:00:00Z)
pub const CLOCK_DEFAULT_START_MS: u64 = 0;

/// ノイズ注入シミュレーションのガウスノイズ標準偏差 (Calibration Candidate)
/// Default: 0.05, 感度分析推奨範囲: 0.01-0.20
pub const NOISE_SIMULATION_SIGMA: f64 = 0.05;

// === Applicability Gate ===

/// デフォルトの埋め込みモデルバージョン (Environment Policy Knob)
///
/// AG-06/AG-07 のハードゲート判定で使用されるデフォルト値。
/// 環境（テスト/ステージング/本番）に応じて変更可能。
pub const AG_HARD_GATE_DEFAULT_MODEL_VERSION: &str = "v2.0-final";

/// デフォルトのテンプレートバージョン (Environment Policy Knob)
///
/// AG-07 の structural channel テンプレートバージョン判定で使用されるデフォルト値。
pub const AG_HARD_GATE_DEFAULT_TEMPLATE_VERSION: &str = "v2.0-final";

/// 管理者 fast-track の強制信頼値 (Safety Invariant)
/// RFC §8.2 指定値: 0.80
pub const TRUST_ADMIN_FAST_TRACK: f64 = 0.80;

/// 人間信頼ロジスティックスケール (Safety Invariant)
/// RFC §10.3 指定値: 0.30
pub const HUMAN_TRUST_SCALE: f64 = 0.30;

/// 人間信頼コールドスタート値 (Safety Invariant)
/// RFC §10.3 指定値: 0.50
pub const HUMAN_TRUST_COLD_START: f64 = 0.50;

/// TrustUpdate::Human デバウンス閾値 (Calibration Candidate)
///
/// RFC §10.5 指定値: 0.05
/// 複合信頼スコアの変動がこの値未満の場合、キャッシュ無効化をスキップする。
/// 非同期フィードバックの頻繁な注入による不必要な再計算を防止する (OQ-11 参照)。
/// 感度分析推奨範囲: 0.02-0.10
pub const TRUST_DEBOUNCE_DELTA: f64 = 0.05;

/// 発振検出 最大発振カウント (Calibration Candidate)
/// Default: 3, 感度分析推奨範囲: 1-10
/// Refine↔Retrieve の交互遷移がこの回数に達すると is_oscillating() = true
pub const OSCILLATION_MAX_COUNT: u32 = 3;

// === HITL (Human-In-The-Loop) ===

/// HITL communicate のデフォルトタイムアウト秒数 (Environment Policy Knob)
/// Default: 3600 (1 hour)
pub const HITL_DEFAULT_TIMEOUT_SECS: u64 = 3600;

/// HITL reconnect 失敗時の再試行間隔 (Calibration Candidate)
/// Default: 5.0, 調整ガイド: 小さくすると再試行頻度増加、大きくすると回復遅延
pub const HITL_RECONNECT_BACKOFF_SECS: f64 = 5.0;

// === Confidence / Mock Proposer ===

/// c_s（Semantic Validity）の統合重み (Calibration Candidate)
/// Default: 0.40, 感度分析推奨範囲: 0.20-0.60
pub const CONFIDENCE_C_S_WEIGHT: f64 = 0.40;

/// c_v（Variable Consistency）の統合重み (Calibration Candidate)
/// Default: 0.35, 感度分析推奨範囲: 0.15-0.55
pub const CONFIDENCE_C_V_WEIGHT: f64 = 0.35;

/// c_h（Heuristic Alignment）の統合重み (Calibration Candidate)
/// Default: 0.25, 感度分析推奨範囲: 0.05-0.45
pub const CONFIDENCE_C_H_WEIGHT: f64 = 0.25;

/// Refine へ分岐する統合 confidence 上限 (Calibration Candidate)
/// Default: 0.50, 感度分析推奨範囲: 0.30-0.70
pub const CONFIDENCE_REFINE_THRESHOLD: f64 = 0.50;

/// Finalize へ分岐する統合 confidence 下限 (Calibration Candidate)
/// Default: 0.70, 感度分析推奨範囲: 0.50-0.90
pub const CONFIDENCE_FINALIZE_THRESHOLD: f64 = 0.70;

/// Mock 提案器の confidence 最小値 (Calibration Candidate)
/// Default: 0.30, 感度分析推奨範囲: 0.10-0.50
pub const MOCK_PROPOSER_CONFIDENCE_MIN: f64 = 0.30;

/// Mock 提案器の confidence 最大値 (Calibration Candidate)
/// Default: 0.95, 感度分析推奨範囲: 0.70-1.00
pub const MOCK_PROPOSER_CONFIDENCE_MAX: f64 = 0.95;

// === Human Review Queue Constants ===

/// 人間レビューデフォルトタイムアウト（秒）(Environment Policy Knobs)
/// RFC 推奨値: 3600。この値を超えて未処理の mission は再通知または escalation。
pub const HUMAN_REVIEW_TIMEOUT_SECS: u64 = 3600;

/// エスカレーションタイムアウト（秒）(Environment Policy Knobs)
/// RFC 推奨値: 14400。TIMEOUT 後も未解決の場合により上位の reviewer へ通知。
pub const HUMAN_REVIEW_ESCALATION_SECS: u64 = 14400;

/// 同一種類の滞留 mission に対する一括承認/却下の最大件数 (Environment Policy Knobs)
/// RFC 推奨値: 20。
pub const HUMAN_REVIEW_MAX_BATCH_SIZE: u32 = 20;

/// ツイン軌道初期摂動 δC(0) (Safety Invariant)
pub const LYAPUNOV_DELTA_C0: f64 = 1e-6;

/// 信頼度ベクトル次元 (Safety Invariant)
pub const CONFIDENCE_VECTOR_DIM: usize = 3;

// === Dual-Store Consistency (M1.5-2) ===

/// デュアルストア コミット最大再試行回数 (Safety Invariant)
/// 論理コミット失敗時の再試行上限。M1.5-3 の修復スキャンにも波及する。
pub const DUAL_STORE_MAX_RETRY: u32 = 3;

/// デュアルストア エラー注入テスト用シード (Calibration Candidate)
/// エラー注入テストの再現性確保に使用する固定シード。
pub const DUAL_STORE_ERROR_INJECTION_SEED: u64 = 67890;

// === Startup Repair Scan (M1.5-3) ===

/// 修復スキャン1資産あたりの最大再試行回数 (Safety Invariant)
/// 起動時修復スキャンで各不整合資産に対して apply_repair を
/// 呼び出す最大回数。実環境の I/O エラー率に応じて調整可能だが、
/// v2.3 では Safety Invariant として固定。
pub const REPAIR_SCAN_MAX_RETRY: u32 = 3;

/// 修復スキャンのバッチサイズ (Calibration Candidate)
/// 1回のスキャン走査で処理する最大資産数。
/// 大規模アンサンブルでのメモリ使用量と時間のトレードオフ調整用。
pub const REPAIR_SCAN_BATCH_SIZE: usize = 100;

// === Event Architecture (RFC §12C) ===

/// EventBus チャネルバッファ容量 (Safety Invariant)
/// RFC §12C のチャネル容量。同期 FakeEventBus では Vec の初期容量の参考値。
pub const EVENTBUS_CHANNEL_CAPACITY: usize = 1024;

/// EventBus publish/open のデフォルトタイムアウト (ms) (Calibration Candidate)
/// RFC §12C 推奨範囲: 1000-30000
pub const EVENTBUS_DEFAULT_TIMEOUT_MS: u64 = 5000;

/// EventBus max reconnect retries (Calibration Candidate)
/// RFC §12C 既定値: 3, 範囲: 1-10
pub const EVENTBUS_MAX_RECONNECT_RETRIES: u32 = 3;

/// EventBus 購読フィルタに指定可能な最大 kind 数 (Calibration Candidate)
/// RFC §12C 既定値: 32, 範囲: 1-128
pub const EVENTBUS_SUBSCRIPTION_MAX_KINDS: u32 = 32;

/// SubscriberManager の最大購読者数 (Safety Invariant)
/// この値を超える購読登録は拒否される。
pub const MAX_SUBSCRIBERS: usize = 100;

/// FakeWebSocketEventChannel の内部バッファ容量 (Environment Policy Knob)
pub const FAKE_WS_CHANNEL_BUFFER_SIZE: usize = 1024;

/// EventBus replay のバッチサイズ (Calibration Candidate)
/// RFC §12C 既定値: 100, 範囲: 10-1000
pub const EVENTBUS_REPLAY_BATCH_SIZE: u32 = 100;

/// EventBus チャネル再接続の初期バックオフ遅延 (ms) (Calibration Candidate)
/// RFC §12C 既定値: 1000, 範囲: 100-10000
pub const EVENTBUS_CHANNEL_RECONNECT_BASE_DELAY_MS: u64 = 1000;

/// EventBus チャネル再接続の最大バックオフ遅延 (ms) (Calibration Candidate)
/// RFC §12C 既定値: 30000, 範囲: 5000-120000
pub const EVENTBUS_CHANNEL_RECONNECT_MAX_DELAY_MS: u64 = 30000;

/// EventBus projection エラー再試行間隔 (ms) (Calibration Candidate)
/// RFC §12C 既定値: 5000, 範囲: 1000-60000
pub const EVENTBUS_PROJECTION_ERROR_BACKOFF_MS: u64 = 5000;

/// 未解決インタラクションの定期クリーンアップ間隔 (ticks) (Calibration Candidate)
/// VirtualClock tick 単位。100 tick ごとに stale インタラクションを監査する。
pub const INTERACTION_CLEANUP_INTERVAL_TICKS: u64 = 100;

/// ProjectionCatalog の初期登録可能数 (Environment Policy Knob)
/// 起動時に登録される projection 数の事前割当に使用。
pub const PROJECTION_INITIAL_CAPACITY: usize = 64;

/// Quarantine 可能な最大イベント数 (Safety Invariant)
/// この値を超える quarantine は拒否される。
pub const QUARANTINE_MAX_EVENTS: usize = 10000;

// === Space Position / Village (M1.75-1, RFC §41B.2) ===

/// 空間位置更新の指数平滑化率 α (Calibration Candidate)
/// x_{t+1} = (1-α)x_t + α·p_t （式 41B-1）。
/// RFC §41B.2 推奨範囲: 0.05-0.50。0.30 は中間値。
pub const SPACE_POSITION_UPDATE_ALPHA: f64 = 0.30;

/// 空間位置更新の最小間隔 (VirtualClock ticks) (Calibration Candidate)
/// 同一位置の過剰更新を防ぐための最小間隔。
/// RFC §41B 推奨範囲: 1-50。5 は中間的更新頻度。
pub const SPACE_POSITION_UPDATE_MIN_INTERVAL: u64 = 5;

/// L2 距離のゼロ判定イプシロン (Safety Invariant)
/// 浮動小数点誤差を考慮した同一位置判定の閾値。
pub const SPACE_POSITION_L2_EPSILON: f64 = 1e-6;

// === Maturity Thresholds (M1.75-2, RFC §41B.3) ===

/// Child 判定の最小生存経験値 (Calibration Candidate)
/// experiencecount(G) < MIN_SURVIVAL_EXPERIENCE → Child（式 41B-3）。
/// RFC §41B.3 推奨デフォルト: 5。
pub const MIN_SURVIVAL_EXPERIENCE: u64 = 5;

/// Adult 判定の経験値閾値 E_adult (Calibration Candidate)
/// E(G) >= E_ADULT_THRESHOLD が Adult 条件の一つ（式 41B-4）。
/// v1.7 ライフサイクル保護との整合性を考慮。
pub const E_ADULT_THRESHOLD: u64 = 20;

/// 経験値正規化のスケーリング係数 (Calibration Candidate)
/// compute_experience_normalization の減衰速度を制御する。
/// デフォルト: E_ADULT_THRESHOLD / 2 = 10.0。経験値が 10 で約 63% 飽和。
pub const EXPERIENCE_NORMALIZATION_SCALE: f64 = 10.0;

/// Adult 判定の信頼複合スコア閾値 T_adult (Calibration Candidate)
/// T(G) >= T_ADULT_THRESHOLD が Adult 条件の一つ（式 41B-4）。
pub const T_ADULT_THRESHOLD: f64 = 0.70;

/// Adult 判定のレピュテーション最終スコア閾値 R_adult (Calibration Candidate)
/// R(G) >= R_ADULT_THRESHOLD が Adult 条件の一つ（式 41B-4）。
pub const R_ADULT_THRESHOLD: f64 = 0.70;

// === HELP Offer Policy (M1.75-4, RFC §41B.6) ===

/// Offer policy 品質重み a₁ (Calibration Candidate)
/// 式 41B-10: Q(h,c,M) の係数 a₁。大きいほど品質を重視。
/// RFC §41B.6 推奨: 1.0、感度分析推奨範囲: 0.5-2.0
pub const HELP_OFFER_QUALITY_WEIGHT: f64 = 1.0;

/// Offer policy 負荷ペナルティ a₂ (Calibration Candidate)
/// 式 41B-10: L_load(h) の係数 a₂。大きいほど負荷ペナルティを強く課す。
/// RFC §41B.6 推奨: 0.5、感度分析推奨範囲: 0.0-1.0
pub const HELP_OFFER_LOAD_PENALTY: f64 = 0.5;

/// Offer policy リスクペナルティ a₃ (Calibration Candidate)
/// 式 41B-10: P_risk(M) の係数 a₃。大きいほどリスクを重視。
/// RFC §41B.6 推奨: 0.3、感度分析推奨範囲: 0.0-1.0
pub const HELP_OFFER_RISK_PENALTY: f64 = 0.3;

/// Offer policy 閾値 θ_offer (Calibration Candidate)
/// 式 41B-10: O(h,c,M)=1 の判定閾値。
/// RFC §41B.6 推奨: 0.0、感度分析推奨範囲: -0.5-0.5
pub const HELP_OFFER_THRESHOLD: f64 = 0.0;

// === HELP Acceptance Policy (M1.75-4, RFC §41B.7) ===

/// Child need 経験値重み γ₁ (Calibration Candidate)
/// 式 41B-12: (1-Ẽ(c)) の係数 γ₁。経験値不足をニーズとして評価する重み。
/// RFC §41B.7 推奨: 0.4、感度分析推奨範囲: 0.2-0.6
pub const HELP_ACCEPT_NEED_GAMMA1: f64 = 0.4;

/// Child need 信頼重み γ₂ (Calibration Candidate)
/// 式 41B-12: (1-T(c)) の係数 γ₂。信頼不足をニーズとして評価する重み。
/// RFC §41B.7 推奨: 0.3、感度分析推奨範囲: 0.1-0.5
pub const HELP_ACCEPT_NEED_GAMMA2: f64 = 0.3;

/// Child need ライフサイクル重み γ₃ (Calibration Candidate)
/// 式 41B-12: (1-L(c)) の係数 γ₃。ライフサイクルリスクをニーズとして評価する重み。
/// RFC §41B.7 推奨: 0.3、感度分析推奨範囲: 0.1-0.5
pub const HELP_ACCEPT_NEED_GAMMA3: f64 = 0.3;

/// Acceptance policy 品質重み b₁ (Calibration Candidate)
/// 式 41B-13: Q(h,c,M) の係数 b₁。大きいほど offer quality を重視。
/// RFC §41B.7 推奨: 1.0、感度分析推奨範囲: 0.5-2.0
pub const HELP_ACCEPT_QUALITY_WEIGHT: f64 = 1.0;

/// Acceptance policy 不確実性重み b₂ (Calibration Candidate)
/// 式 41B-13: U(c,M) の係数 b₂。大きいほど child の不確実性を重視。
/// RFC §41B.7 推奨: 0.5、感度分析推奨範囲: 0.0-1.0
pub const HELP_ACCEPT_UNCERTAINTY_WEIGHT: f64 = 0.5;

/// Acceptance policy 自律性ペナルティ b₃ (Calibration Candidate)
/// 式 41B-13: A(c,h) の係数 b₃。大きいほど自律性喪失リスクを重視。
/// RFC §41B.7 推奨: 0.3、感度分析推奨範囲: 0.0-1.0
pub const HELP_ACCEPT_AUTONOMY_PENALTY: f64 = 0.3;

/// Acceptance policy 閾値 θ_accept (Calibration Candidate)
/// 式 41B-13: Accept(c,h,M)=1 の判定閾値。
/// RFC §41B.7 推奨: 0.0、感度分析推奨範囲: -0.5-0.5
pub const HELP_ACCEPT_THRESHOLD: f64 = 0.0;

// === Child Support Mission (M1.75-5, RFC §41B.11) ===

/// 1つの child-support mission に参加可能な最大 helper 数 (Calibration Candidate)
/// RFC §41B.11 推奨デフォルト: 10。感度分析推奨範囲: 1-50。
pub const MAX_HELPERS_PER_MISSION: u32 = 10;

/// child-support mission のデフォルトタイムアウト秒数 (Environment Policy Knob)
/// この時間を超えて完了しない mission はタイムアウトとして扱われる。
pub const CHILD_SUPPORT_MISSION_TIMEOUT_SECS: u64 = 3600;

// === Helper Weighting (M1.75-6, RFC §41B.12) ===

/// 距離減衰係数 β (Calibration Candidate)
/// 式 41B-18: exp(-β·d_t(h,c)) の距離減衰強度。
/// 値が大きいほど遠距離 helper の重みが急激に減少する。
/// Default: 1.0, 感度分析推奨範囲: 0.1-5.0
pub const HELPER_WEIGHT_DISTANCE_DECAY_BETA: f64 = 1.0;

/// 信頼指数 μ (Calibration Candidate)
/// 式 41B-18: T(h)^μ の信頼重み指数。
/// 値が大きいほど信頼の差が重みに強く反映される。
/// Default: 1.0, 感度分析推奨範囲: 0.5-2.0
pub const HELPER_WEIGHT_TRUST_EXPONENT: f64 = 1.0;

/// レピュテーション指数 ν (Calibration Candidate)
/// 式 41B-18: R(h)^ν のレピュテーション重み指数。
/// 値が大きいほどレピュテーションの差が重みに強く反映される。
/// Default: 1.0, 感度分析推奨範囲: 0.5-2.0
pub const HELPER_WEIGHT_REPUTATION_EXPONENT: f64 = 1.0;

/// 探索率 ε (Calibration Candidate)
/// 式 41B-19: 局所重みと遠隔探索の混合率。
/// ε = 0 で純局所、ε = 1 で純遠隔探索。
/// Default: 0.1, 感度分析推奨範囲: 0.0-1.0
pub const HELPER_WEIGHT_EXPLORATION_EPSILON: f64 = 0.1;

/// デフォルトの TOP-K 選抜数 (Calibration Candidate)
/// select_helpers が返す helper の最大数。
/// MAX_HELPERS_PER_MISSION と一致させること。
/// Default: 10, 感度分析推奨範囲: 1-50
pub const HELPER_WEIGHT_DEFAULT_TOP_K: usize = 10;

// === Village Metrics (M1.75-7, RFC §41B.14, §41B.15) ===

/// VillageMetricsWindow のデフォルトウィンドウサイズ (Environment Policy Knob)
/// 直近 N tick 分の生メトリクスを保持する。
/// Default: 100, 分布同定時は 10,000 まで拡大可能。
pub const VILLAGE_METRICS_WINDOW_SIZE: usize = 100;

/// VillageObservationLog Projection の登録名 (Environment Policy Knob)
pub const VILLAGE_EVENT_PROJECTION_NAME: &str = "village_observation_log";

/// Village churn の最大許容 P95 値 (Calibration Candidate)
/// V(c,t) の P95 がこの値を超える場合、village が不安定であることを示唆する。
/// RFC §41B.15 推奨デフォルト: 0.30, 感度分析推奨範囲: 0.10-0.50
pub const VILLAGE_STABILITY_MAX_CHURN_P95: f64 = 0.30;

/// Village dynamicity の最小長期変化 (Calibration Candidate)
/// 長期 Jaccard J_τ がこの値を下回る場合、village が過度に静的であることを示唆する。
/// RFC §41B.15 推奨デフォルト: 0.10, 感度分析推奨範囲: 0.05-0.30
pub const VILLAGE_DYNAMICITY_MIN_LONG_HORIZON_CHANGE: f64 = 0.10;

// ============================================================================
// M1.75-8: Deterministic Replay 定数
// ============================================================================

/// replay シナリオ内位置更新のランダムウォーク標準偏差 (Calibration Candidate)
/// Default: 0.1, 感度分析推奨範囲: 0.01-1.0
pub const REPLAY_POSITION_DELTA_SIGMA: f64 = 0.1;

// ============================================================================
// M1.75-9: Small Perturbation 定数
// ============================================================================

/// Embedding ノイズ注入のデフォルト標準偏差 (Calibration Candidate)
/// apply_embedding_noise で使用するガウスノイズの標準偏差。
/// Default: 0.02, 感度分析推奨範囲: 0.001-1.0
pub const PERTURB_EMBEDDING_NOISE_SIGMA_DEFAULT: f64 = 0.02;

/// 信頼値摂動のデフォルト Δ (Calibration Candidate)
/// apply_trust_delta で使用する絶対変化量。
/// Default: 0.05, 感度分析推奨範囲: 0.01-0.20
pub const PERTURB_TRUST_DELTA_DEFAULT: f64 = 0.05;

/// 利用履歴増分デフォルト値 (Environment Policy Knob)
/// apply_usage_increment で使用する経験値増加量。
/// Default: 1
pub const PERTURB_USAGE_INCREMENT_DEFAULT: u64 = 1;

/// 許容 churn P95 増加量上限 (Safety Invariant)
/// 摂動後の churn_p95 - baseline churn_p95 がこの値を超えた場合、
/// 摂動に対して脆弱であると判定する。
pub const PERTURB_CHURN_MAX_P95_INCREASE: f64 = 0.10;

/// 許容 JSD P95 増加量上限 (Safety Invariant)
/// 摂動後の jsd_p95 - baseline jsd_p95 がこの値を超えた場合、
/// helper 分布が不安定であると判定する。
pub const PERTURB_JSD_MAX_P95_INCREASE: f64 = 0.10;

// ============================================================================
// M-0.5-7-R: Pipeline Search 定数
// ============================================================================

// --- Pipeline stage 閾値 ---

/// Semantic TopK のデフォルト k (Stage 1)
/// RFC §12.3D 推奨: 20
pub const SEMANTIC_TOPK_K_SEM: usize = 20;

/// Metadata Filter 通過後のデフォルト k (Stage 2)
/// RFC §12.3D 推奨: 50
pub const METADATA_FILTER_K_META: usize = 50;

/// Cheap GED Filter 通過後のデフォルト k (Stage 3)
/// RFC §12.3D 推奨: 20
pub const CHEAP_GED_FILTER_K_CHEAP: usize = 20;

/// Full GED Rerank 通過後のデフォルト k (Stage 4)
/// RFC §12.3D 推奨: 10
pub const FULL_GED_RERANK_K_FULL: usize = 10;

/// CheapGED 有効化閾値 (Safety Invariant)
/// 候補数がこの値を超える場合にのみ CheapGED を適用する。
pub const CHEAPGED_ENABLE_THRESHOLD: usize = 100;

// --- Applicability Evaluation (RFC §11.3 式6-10) ---

/// 構造スコア減衰係数 λ (Calibration Candidate)
/// RFC §11.3 推奨デフォルト: 4.0, 感度分析推奨範囲: 1.0-10.0
pub const STRUCT_GED_LAMBDA: f64 = 4.0;

/// 意味スコアと構造スコアの加重結合 α (Calibration Candidate)
/// 式(8): S_total = α * S_sem + (1-α) * S_struct
/// RFC §11.3 推奨デフォルト: 0.45, 感度分析推奨範囲: 0.20-0.80
pub const SIMILARITY_ALPHA: f64 = 0.45;

/// 知識認識適格性の知識重み β (Calibration Candidate)
/// 式(10): A_final = A_workflow^β * K^(1-β)
/// RFC §11.3 推奨デフォルト: 0.70, 感度分析推奨範囲: 0.50-0.95
pub const APPLICABILITY_BETA: f64 = 0.70;

// --- フロアパラメータ (Calibration Candidate) ---

/// フロアパラメータ α_S（意味スコア加重）
/// 式(9): max(D, f_D)^αD — 意味スコアフロア指数
/// RFC §11.3 推奨デフォルト: 0.40
pub const APPLICABILITY_ALPHA_S: f64 = 0.40;

/// フロアパラメータ α_D（決定性スコア加重）
/// 式(9): max(D, f_D)^αD — 決定性スコアフロア指数
/// RFC §11.3 推奨デフォルト: 0.30
pub const APPLICABILITY_ALPHA_D: f64 = 0.30;

/// フロアパラメータ α_T（信頼スコア加重）
/// 式(9): max(T, f_T)^αT — 信頼スコアフロア指数
/// RFC §11.3 推奨デフォルト: 0.30
pub const APPLICABILITY_ALPHA_T: f64 = 0.30;

// --- フロア値 (Calibration Candidate) ---

/// 意味スコアフロア f_S (式9)
/// RFC §11.3 推奨デフォルト: 0.10
pub const APPLICABILITY_FLOOR_S: f64 = 0.10;

/// 決定性スコアフロア f_D (式9)
/// RFC §11.3 推奨デフォルト: 0.10
pub const APPLICABILITY_FLOOR_D: f64 = 0.10;

/// 信頼スコアフロア f_T (式9)
/// RFC §11.3 推奨デフォルト: 0.20
pub const APPLICABILITY_FLOOR_T: f64 = 0.20;

/// 適格性判定閾値 (Calibration Candidate)
/// A_workflow がこの値を下回る場合、候補を不適格と判定する。
/// RFC §11.3 推奨デフォルト: 0.50
pub const APPLICABILITY_THRESHOLD: f64 = 0.50;

// ============================================================================
// M1.75-10: Property-based Village Invariant Fuzzing 定数
// ============================================================================

/// proptest デフォルト反復回数 (Environment Policy Knob)
/// 各 invariant test で使用するランダムケース数。
/// Default: 10_000, 調整推奨範囲: 1_000-100_000
pub const PROPTEST_DEFAULT_CASES: u32 = 10_000;

/// village invariant failing seed fixture 出力先ディレクトリ (Environment Policy Knob)
/// FailingSeedEntry の JSON 保存先。Cargo の working directory からの相対パス。
pub const VILLAGE_FIXTURE_DIR: &str = "tests/fixtures/village_invariant_failures";

// ============================================================================
// M1.75-11: Village Calibration Loop 定数
// ============================================================================

/// 目的関数 J_village(θ) の churn 重み a₁ (Calibration Candidate)
/// Default: 0.35, 感度分析推奨範囲: 0.20-0.50
pub const OBJECTIVE_WEIGHT_CHURN: f64 = 0.35;

/// 目的関数 J_village(θ) の JSD 重み a₂ (Calibration Candidate)
/// Default: 0.25, 感度分析推奨範囲: 0.15-0.40
pub const OBJECTIVE_WEIGHT_JSD: f64 = 0.25;

/// 目的関数 J_village(θ) の survival 重み a₃ (Calibration Candidate)
/// Default: 0.25, 感度分析推奨範囲: 0.15-0.40
pub const OBJECTIVE_WEIGHT_SURVIVAL: f64 = 0.25;

/// 目的関数 J_village(θ) の false-new ペナルティ重み a₄ (Calibration Candidate)
/// Default: 0.10, 感度分析推奨範囲: 0.05-0.20
pub const OBJECTIVE_WEIGHT_FALSE_NEW: f64 = 0.10;

/// 目的関数 J_village(θ) の review-load ペナルティ重み a₅ (Calibration Candidate)
/// Default: 0.05, 感度分析推奨範囲: 0.02-0.15
pub const OBJECTIVE_WEIGHT_REVIEW_LOAD: f64 = 0.05;

/// OFAT sweep のデフォルトステップ数 (Environment Policy Knob)
/// Default: 5, 調整推奨範囲: 3-10
pub const SWEEP_OFAT_DEFAULT_STEPS: usize = 5;

/// Grid sweep のデフォルト軸あたり分割数 (Environment Policy Knob)
/// Default: 3, 調整推奨範囲: 2-5
pub const SWEEP_GRID_DEFAULT_DIVISIONS: usize = 3;

/// Latin hypercube sampling のデフォルトサンプル数 (Environment Policy Knob)
/// Default: 20, 調整推奨範囲: 10-100
pub const SWEEP_LHS_DEFAULT_SAMPLES: usize = 20;

// ============================================================================
// M1.76-2: Reciprocity-Aware Survival 定数 (RFC §15.10, Calibration Candidates)
// ============================================================================

/// 直接互恵性 α_h — Help イベントの係数 (F-1) (Calibration Candidate)
/// F-1: α_h * H_{ij}。α_h > 0 (MUST)。
/// Default: 1.0, 感度分析推奨範囲: 0.5-2.0
pub const RECIPROCITY_ALPHA_HELP: f32 = 1.0;

/// 直接互恵性 α_hs — HelpSuccess イベントの係数 (F-1) (Calibration Candidate)
/// F-1: α_hs * HS_{ij}。α_hs > 0 (MUST)。
/// Default: 2.0, 感度分析推奨範囲: 1.0-4.0
pub const RECIPROCITY_ALPHA_SUCCESS: f32 = 2.0;

/// 直接互恵性 α_r — Reject イベントの係数 (F-1) (Calibration Candidate)
/// F-1: -α_r * RJ_{ij}。α_r > 0 (MUST)。
/// Default: 1.0, 感度分析推奨範囲: 0.5-2.0
pub const RECIPROCITY_ALPHA_REJECT: f32 = 1.0;

/// 直接互恵性 α_d — Harm イベントの係数 (F-1) (Calibration Candidate)
/// F-1: -α_d * DMG_{ij}。α_d > 0 (MUST)。
/// Default: 2.0, 感度分析推奨範囲: 1.0-4.0
pub const RECIPROCITY_ALPHA_HARM: f32 = 2.0;

/// 直接互恵性時間減衰 ρ_dir (F-1) (Calibration Candidate)
/// F-1: exp(-ρ_dir * Δt_{ij})。時定数 1/ρ_dir。
/// Default: 0.01, 感度分析推奨範囲: 0.001-0.10
pub const RECIPROCITY_DIRECT_DECAY_RHO: f32 = 0.01;

/// 間接互恵性 β_1 — 中心性 C_i^help の係数 (F-2) (Calibration Candidate)
/// F-2: β_1 * C_i^help。β_1 > 0 (MUST)。
/// Default: 1.0, 感度分析推奨範囲: 0.5-4.0
pub const INDIRECT_BETA_CENTRALITY: f32 = 1.0;

/// 間接互恵性 β_2 — 村参加度 A_i^village の係数 (F-2) (Calibration Candidate)
/// F-2: β_2 * A_i^village。β_2 > 0 (MUST)。
/// Default: 1.0, 感度分析推奨範囲: 0.5-4.0
pub const INDIRECT_BETA_VILLAGE_PARTICIPATION: f32 = 1.0;

/// 間接互恵性 β_3 — 受諾率 U_i^accepted の係数 (F-2) (Calibration Candidate)
/// F-2: β_3 * U_i^accepted。β_3 > 0 (MUST)。
/// Default: 1.0, 感度分析推奨範囲: 0.5-4.0
pub const INDIRECT_BETA_ACCEPTED_RATE: f32 = 1.0;

/// 間接互恵性 β_4 — 成功貢献率 Q_i^success の係数 (F-2) (Calibration Candidate)
/// F-2: β_4 * Q_i^success。β_4 > 0 (MUST)。
/// Default: 2.0, 感度分析推奨範囲: 1.0-4.0
pub const INDIRECT_BETA_SUCCESS_RATE: f32 = 2.0;

/// 間接互恵性 β_5 — 負評価 B_i^harm の係数 (F-2) (Calibration Candidate)
/// F-2: -β_5 * B_i^harm。β_5 > 0 (MUST)。
/// Default: 2.0, 感度分析推奨範囲: 1.0-4.0
pub const INDIRECT_BETA_HARM_SCORE: f32 = 2.0;

/// BenevolenceScore 集約重み w_dir (F-3) (Calibration Candidate)
/// F-3: B_i = w_dir * R_dir + w_ind * R_ind + w_rep * Rep
/// Default: 0.35, 感度分析推奨範囲: 0.20-0.50
pub const REPUTATION_WEIGHT_DIRECT: f32 = 0.35;

/// BenevolenceScore 集約重み w_ind (F-3) (Calibration Candidate)
/// F-3: 間接互恵性 R_ind の重み。
/// Default: 0.35, 感度分析推奨範囲: 0.20-0.50
pub const REPUTATION_WEIGHT_INDIRECT: f32 = 0.35;

/// BenevolenceScore 集約重み w_rep (F-3) (Calibration Candidate)
/// F-3: B_i = w_dir * R_dir + w_ind * R_ind + w_rep * Rep
/// Default: 0.30 (w_dir + w_ind + w_rep = 1.0),
/// 感度分析推奨範囲: 0.15-0.45
pub const REPUTATION_WEIGHT_REPUTATION: f32 = 0.30;

/// LifecycleScore への benevolence 寄与重み (Calibration Candidate)
/// F-6 (推奨案 B): L(G) 不変、GC hazard 側で効かせる。
/// Default: 0.15, 感度分析推奨範囲: 0.05-0.30
pub const LIFECYCLE_WEIGHT_BENEVOLENCE: f32 = 0.15;

/// F-7 GC hazard ベースライン λ_0 (Calibration Candidate)
/// λ_i^GC = softplus(λ_0 - γ_L·L_i - γ_B·B_i - γ_C·C_i^protect)
/// Default: 1.0, 感度分析推奨範囲: 0.1-5.0
pub const GC_HAZARD_LAMBDA_0: f32 = 1.0;

/// F-7 GC hazard LifecycleScore 重み γ_L (Calibration Candidate)
/// Default: 0.5, 感度分析推奨範囲: 0.2-1.0
pub const GC_HAZARD_GAMMA_LIFECYCLE: f32 = 0.5;

/// GC hazard γ_benevolence (F-7) (Calibration Candidate)
/// F-7: λ_gc = softplus(λ_0 - γ_b * B_i - γ_c * P_i)。
/// Default: 0.10, 感度分析推奨範囲: 0.05-0.30
pub const GC_HAZARD_GAMMA_BENEVOLENCE: f32 = 0.10;

/// GC hazard γ_child_protect (F-8) (Calibration Candidate)
/// F-8: child の GC hazard 低減係数。
/// Default: 0.20, 感度分析推奨範囲: 0.10-0.50
pub const GC_HAZARD_GAMMA_CHILD_PROTECT: f32 = 0.20;

// ============================================================================
// F-10: Child protection integration 定数 (Calibration Candidates)
// ============================================================================

/// F-10 基本 child 保護定数 η₁ (Calibration Candidate)
/// C_i^protect = η₁·1[Child(i)] + η₂·H_i^received + η₃·G_i^growth
/// is_child=true のとき最低 η₁ の保護が保証される。
/// Default: 0.50, 感度分析推奨範囲: 0.20-1.00
pub const CHILD_PROTECT_ETA1: f32 = 0.50;

/// F-10 支援受領重み η₂ (Calibration Candidate)
/// C_i^protect における H_i^received (支援受領量) の係数。
/// Default: 0.30, 感度分析推奨範囲: 0.10-0.60
pub const CHILD_PROTECT_ETA2: f32 = 0.30;

/// F-10 成長改善重み η₃ (Calibration Candidate)
/// C_i^protect における G_i^growth (成長改善量) の係数。
/// Default: 0.20, 感度分析推奨範囲: 0.10-0.50
pub const CHILD_PROTECT_ETA3: f32 = 0.20;

/// Helper quality への benevolence 重み (F-11) (Calibration Candidate)
/// F-11: HScore(h) に含まれる benevolence の重み。
/// Default: 0.20, 感度分析推奨範囲: 0.10-0.40
pub const HELP_WEIGHT_BENEVOLENCE: f32 = 0.20;

/// Helper quality への mission suitability 重み w_s (F-11) (Calibration Candidate)
/// F-11: Q = w_s·S + ... における S (mission suitability) の係数。
/// Default: 1.0, 感度分析推奨範囲: 0.5-2.0
pub const HELP_QUALITY_SUITABILITY_WEIGHT: f32 = 1.0;

/// Helper quality への trust 重み w_t (F-11) (Calibration Candidate)
/// F-11: Q = ... + w_t·T + ... における T (trust) の係数。
/// Default: 1.0, 感度分析推奨範囲: 0.5-2.0
pub const HELP_QUALITY_TRUST_WEIGHT: f32 = 1.0;

/// Helper quality への reputation 重み w_r (F-11) (Calibration Candidate)
/// F-11: Q = ... + w_r·Rep + ... における Rep (reputation) の係数。
/// Default: 1.0, 感度分析推奨範囲: 0.5-2.0
pub const HELP_QUALITY_REPUTATION_WEIGHT: f32 = 1.0;

/// Helper quality への child need 重み w_n (F-11) (Calibration Candidate)
/// F-11: Q = ... + w_n·N + ... における N (child_need) の係数。
/// Default: 1.0, 感度分析推奨範囲: 0.5-2.0
pub const HELP_QUALITY_CHILD_NEED_WEIGHT: f32 = 1.0;

/// Helper quality への distance penalty 重み w_d (F-11) (Calibration Candidate)
/// F-11: Q = ... - w_d·d における d (distance penalty) の係数。
/// Default: 1.0, 感度分析推奨範囲: 0.5-2.0
pub const HELP_QUALITY_DISTANCE_PENALTY: f32 = 1.0;

/// Helper softmax selection τ (F-12) (Calibration Candidate)
/// F-12: softmax 温度パラメータ。τ が大きいと等確率に近づく。
/// Default: 1.0, 感度分析推奨範囲: 0.5-2.0
pub const HELP_SOFTMAX_TAU: f32 = 1.0;

/// Remote exploration base rate ε_base (Calibration Candidate)
/// F-13: 遠隔探索の基本確率。
/// Default: 0.05, 感度分析推奨範囲: 0.01-0.20
pub const REMOTE_EXPLORATION_BASE: f32 = 0.05;

/// Remote exploration max rate ε_max (Calibration Candidate)
/// F-13: 遠隔探索の最大確率。
/// Default: 0.20, 感度分析推奨範囲: 0.10-0.50
pub const REMOTE_EXPLORATION_MAX: f32 = 0.20;

/// Remote exploration need coefficient a₁ (F-13) (Calibration Candidate)
/// F-13: ε_remote = clip(ε₀ + a₁·need(c) - a₂·B_local_avg(c)) における child need 係数。
/// Default: 1.0, 感度分析推奨範囲: 0.5-2.0
pub const REMOTE_EXPLORATION_NEED_COEFF: f32 = 1.0;

/// Remote exploration benevolence coefficient a₂ (F-13) (Calibration Candidate)
/// F-13: ε_remote = clip(ε₀ + a₁·need(c) - a₂·B_local_avg(c)) における benevolence 係数。
/// Default: 1.0, 感度分析推奨範囲: 0.5-2.0
pub const REMOTE_EXPLORATION_BENEVOLENCE_COEFF: f32 = 1.0;

// ============================================================================
// M1.76-10: Child growth increment (F-14) + Maturation probability (F-15) 定数
// ============================================================================

/// μ₁ — 自身の mission success が growth に与える重み (F-14) (Calibration Candidate)
/// F-14: ΔG_c = μ₁·MissionSuccess_c + μ₂·Σ_h HelpSuccess(h→c) + μ₃·B̄_helpers(c) - μ₄·FailureBurden_c
/// Default: 0.60, 感度分析推奨範囲: 0.30-0.90
pub const CHILD_GROWTH_MU_MISSION_SUCCESS: f32 = 0.60;

/// μ₂ — 周囲からの help success が growth に与える重み (F-14) (Calibration Candidate)
/// Default: 0.40, 感度分析推奨範囲: 0.20-0.60
pub const CHILD_GROWTH_MU_HELP_SUCCESS: f32 = 0.40;

/// μ₃ — helper の平均 benevolence が growth に与える重み (F-14) (Calibration Candidate)
/// Default: 0.30, 感度分析推奨範囲: 0.15-0.50
pub const CHILD_GROWTH_MU_HELPER_BENEVOLENCE: f32 = 0.30;

/// μ₄ — failure burden が growth を減少させる重み (F-14) (Calibration Candidate)
/// Default: 0.20, 感度分析推奨範囲: 0.10-0.40
pub const CHILD_GROWTH_MU_FAILURE_BURDEN: f32 = 0.20;

/// ν₀ — maturation 確率のバイアス項 (F-15) (Calibration Candidate)
/// F-15: P_mature(c) = σ(ν₀ + ν₁·E_c^norm + ν₂·T_c + ν₃·Rep_c + ν₄·B̄_helpers(c))
/// Default: -2.0, 感度分析推奨範囲: -3.0 〜 -1.0
pub const MATURATION_NU_BIAS: f32 = -2.0;

/// ν₁ — 正規化経験値が maturation 確率に与える重み (F-15) (Calibration Candidate)
/// Default: 1.0, 感度分析推奨範囲: 0.5-2.0
pub const MATURATION_NU_EXPERIENCE: f32 = 1.0;

/// ν₂ — 信頼値が maturation 確率に与える重み (F-15) (Calibration Candidate)
/// Default: 1.0, 感度分析推奨範囲: 0.5-2.0
pub const MATURATION_NU_TRUST: f32 = 1.0;

/// ν₃ — 評判値が maturation 確率に与える重み (F-15) (Calibration Candidate)
/// Default: 1.0, 感度分析推奨範囲: 0.5-2.0
pub const MATURATION_NU_REPUTATION: f32 = 1.0;

/// ν₄ — helper の平均 benevolence が maturation 確率に与える重み (F-15) (Calibration Candidate)
/// Default: 0.30, 感度分析推奨範囲: 0.15-0.50
pub const MATURATION_NU_HELPER_BENEVOLENCE: f32 = 0.30;

// Backward-compat aliases (旧名でもアクセス可能)
pub const CHILD_GROWTH_WEIGHT_HELP_SUCCESS: f32 = CHILD_GROWTH_MU_HELP_SUCCESS;
pub const CHILD_GROWTH_WEIGHT_BENEVOLENT_HELPERS: f32 = MATURATION_NU_HELPER_BENEVOLENCE;

// ============================================================================
// M1.76-5: ReputationProfile recompute (F-4, F-5) 定数
// ============================================================================

/// F-4 reputation 再計算 直接互恵性重み θ_dir (Calibration Candidate)
/// θ_dir * R_i^dir。θ_dir > 0 (MUST unless village-help disabled)。
/// Default: 0.35, 感度分析推奨範囲: 0.20-0.50
pub const REPUTATION_THETA_DIR: f32 = 0.35;

/// F-4 reputation 再計算 間接互恵性重み θ_ind (Calibration Candidate)
/// θ_ind * R_i^ind。θ_ind > 0 (MUST unless village-help disabled)。
/// Default: 0.35, 感度分析推奨範囲: 0.20-0.50
pub const REPUTATION_THETA_IND: f32 = 0.35;

/// F-4 reputation 再計算 経験値重み θ_exp (Calibration Candidate)
/// θ_exp * E_i^norm。古参固定化防止のため過大にしない。
/// Default: 0.20, 感度分析推奨範囲: 0.10-0.35
pub const REPUTATION_THETA_EXP: f32 = 0.20;

/// F-4 reputation 再計算 継承重み θ_inh (Calibration Candidate)
/// θ_inh * I_i。親の影響を限定するため低め。
/// Default: 0.10, 感度分析推奨範囲: 0.05-0.25
pub const REPUTATION_THETA_INHERIT: f32 = 0.10;

/// F-5 経験値正規化飽和率 κ_E (Calibration Candidate)
/// E_i^norm = 1 - exp(-κ_E * experience_count(i))
/// Default: 0.01, 感度分析推奨範囲: 0.001-0.10
pub const REPUTATION_KAPPA_E: f32 = 0.01;

// ============================================================================
// M1.76-11: ReciprocityEventStore 定数 (Environment Policy Knobs)
// ============================================================================

/// ReciprocityEventProjection の登録名 (Environment Policy Knob)
/// DomainProjection に登録する投影名。変更不可。
pub const RECIPROCITY_EVENT_PROJECTION_NAME: &str = "reciprocity_event";

/// ReciprocityEventStore の初期 HashMap 容量 (Environment Policy Knob)
/// 同時アクティブグラフ数を見積もった値。16-1024 の範囲で調整可。
pub const RECIPROCITY_STORE_INITIAL_CAPACITY: usize = 64;

// ============================================================================
// M1.76-16: F-16 多目的較正目的関数 λ 重み (Safety Invariant)
// ============================================================================

/// 式 F-16 λ₁ — AUC_benevolent>nonbenevolent の重み (Safety Invariant)
/// Default: 0.30
pub const F16_LAMBDA_AUC: f64 = 0.30;

/// 式 F-16 λ₂ — HelpSuccessRate の重み (Safety Invariant)
/// Default: 0.25
pub const F16_LAMBDA_HELP_SUCCESS: f64 = 0.25;

/// 式 F-16 λ₃ — VillageChurnP95 のペナルティ重み (Safety Invariant)
/// Default: 0.15
pub const F16_LAMBDA_CHURN: f64 = 0.15;

/// 式 F-16 λ₄ — FalseNewRate のペナルティ重み (Safety Invariant)
/// Default: 0.10
pub const F16_LAMBDA_FALSE_NEW: f64 = 0.10;

/// 式 F-16 λ₅ — ReviewLoad のペナルティ重み (Safety Invariant)
/// Default: 0.10
pub const F16_LAMBDA_REVIEW_LOAD: f64 = 0.10;

/// 式 F-16 λ₆ — InstabilityPenalty のペナルティ重み (Safety Invariant)
/// Default: 0.10
pub const F16_LAMBDA_INSTABILITY: f64 = 0.10;

// ============================================================================
// M1.76-18: 運用メトリクス観測パイプライン 定数 (Safety Invariants)
// ============================================================================

/// benevolent_survival_advantage 計算における上位割合 (Safety Invariant)
/// RFC §41B.20.7 で定義された上位 20% 分割の閾値。変更不可。
pub const BENEVOLENT_TOP_FRACTION: f64 = 0.2;

/// benevolent_survival_advantage 計算における下位割合 (Safety Invariant)
/// RFC §41B.20.7 で定義された下位 20% 分割の閾値。変更不可。
pub const BENEVOLENT_BOTTOM_FRACTION: f64 = 0.2;

// ============================================================================
// M1.76-19: Phase 0-4 較正ロールアウト 定数
// ============================================================================

/// Phase ゲートの最大 Phase 数 (Safety Invariant)
pub const PHASE_GATE_MAX_PHASES: usize = 5;

/// Canary 環境タグ (Environment Policy Knob)
pub const CANARY_ENVIRONMENT_TAG: &str = "canary";

/// Production 環境タグ (Safety Invariant — 不変の本番環境識別子)
pub const PRODUCTION_ENVIRONMENT_TAG: &str = "production";

/// MagnificentSevenParams のパラメータ名一覧 (Calibration Candidate)
/// Phase 3 の sweep で優先的に探索するパラメータ。
pub const SWEEP_MAGNIFICENT_PARAM_NAMES: &[&str] = &[
    "gamma_benevolence",
    "lambda_gc_base",
    "direct_reciprocity_weight",
    "indirect_reciprocity_weight",
    "softmax_temperature",
    "gc_interval",
    "child_ratio",
];

// ============================================================================
// M1.76-22: Event Architecture メトリクス 定数 (Calibration Candidate)
// ============================================================================

/// EventBus メトリクススループット計算の移動窓サイズ (Calibration Candidate)
/// Default: 100, 感度分析推奨範囲: 10-1000
/// 大きくすると平滑化が強くなり短期的な変動が見えにくくなる。
/// 小さくすると即時的なスループット変動を捉えられるがノイズの影響を受けやすい。
pub const EVENTBUS_METRICS_WINDOW_SIZE: usize = 100;

// ============================================================================
// M1.76-KW1: Kind World 成立条件定数 (RFC §15.9.1, Safety Invariants)
// ============================================================================

/// 最低人口成長率 (Safety Invariant)
/// RFC §15.9.1: 0.01 (1 tick あたり 1%)
pub const KW_MIN_POPULATION_GROWTH_RATE: f64 = 0.01;

/// 最小 Shannon 多様性指数 (Safety Invariant)
/// RFC §15.9.1: 0.5
pub const KW_MIN_CAPABILITY_COVERAGE_SHANNON: f64 = 0.5;

/// 最低再利用比率 (Safety Invariant)
/// RFC §15.9.1: 0.3
pub const KW_MIN_REUSE_RATIO: f64 = 0.3;

/// コスト効率改善比の上限 (Safety Invariant)
/// 1.0 未満で単調減少を意味する。
/// RFC §15.9.1: 0.95
pub const KW_MAX_COST_EFFICIENCY_DECAY: f64 = 0.95;

/// 最低村形成スコア (Safety Invariant)
/// RFC §15.9.1: 0.3
pub const KW_MIN_VILLAGE_FORMATION_SCORE: f64 = 0.3;

/// 適切な村流動性下限 (Safety Invariant)
/// RFC §15.9.1: 0.05
pub const KW_VILLAGE_CHURN_LOWER: f64 = 0.05;

/// 適切な村流動性上限 (Safety Invariant)
/// RFC §15.9.1: 0.30
pub const KW_VILLAGE_CHURN_UPPER: f64 = 0.30;

/// 最小村間相互作用率 (Safety Invariant)
/// RFC §15.9.1: 0.1
pub const KW_CROSS_VILLAGE_INTERACTION_MIN: f64 = 0.1;

/// 村所属判定の距離閾値 (Calibration Candidate)
/// RFC §15.9.1: 0.2, 感度分析推奨範囲: [0.1, 0.5]
pub const VILLAGE_DISTANCE_THRESHOLD: f64 = 0.2;

/// 最小村サイズ (Safety Invariant)
/// RFC §15.9.1: 3 (3 未満の村はクラスタとみなさない)
pub const VILLAGE_MIN_SIZE: usize = 3;

// ============================================================================
// M1.76-KW4: Kind World 較正ループ 探索範囲 (Calibration Candidates)
// ============================================================================

/// 慈悲スコア重みの探索範囲 (Calibration Candidate)
/// RFC §15.9.1: [0.0, 0.5] → 拡張: [0.0, 0.8]
pub const KW4_GAMMA_BENEVOLENCE_RANGE: (f64, f64) = (0.0, 0.8);

/// GC ベースハザードの探索範囲 (Calibration Candidate)
/// RFC §15.9.1: [0.1, 2.0]
pub const KW4_LAMBDA_GC_BASE_RANGE: (f64, f64) = (0.1, 2.0);

/// 直接互恵性重みの探索範囲 (Calibration Candidate)
/// RFC §15.9.1: [0.1, 0.8]
pub const KW4_DIRECT_RECIPROCITY_WEIGHT_RANGE: (f64, f64) = (0.1, 0.8);

/// 間接互恵性重みの探索範囲 (Calibration Candidate)
/// RFC §15.9.1: [0.1, 0.8]
pub const KW4_INDIRECT_RECIPROCITY_WEIGHT_RANGE: (f64, f64) = (0.1, 0.8);

/// ソフトマックス温度の探索範囲 (Calibration Candidate)
/// RFC §15.9.1: [0.1, 1.0] → 拡張: [0.1, 5.0]（Nelder-Mead 連続空間探索のため）
pub const KW4_SOFTMAX_TEMPERATURE_RANGE: (f64, f64) = (0.1, 5.0);

/// GC 実行間隔の探索範囲 (Calibration Candidate)
/// RFC §15.9.1: [1, 10]（整数 → f64 として扱い sim_config で四捨五入）
pub const KW4_GC_INTERVAL_RANGE: (f64, f64) = (1.0, 10.0);

/// 子ワークフロー比率の探索範囲 (Calibration Candidate)
/// RFC §15.9.1: [0.1, 0.5]
pub const KW4_CHILD_RATIO_RANGE: (f64, f64) = (0.1, 0.5);

/// Nelder-Mead 最大反復回数 (Algorithm Constant)
pub const KW4_NELDER_MEAD_MAX_ITERATIONS: usize = 200;

/// Nelder-Mead 収束判定 ε (Algorithm Constant)
/// 頂点間の J_kw 分散がこの値未満で収束と判定。
pub const KW4_NELDER_MEAD_CONVERGENCE_EPSILON: f64 = 1e-6;

/// Nelder-Mead 初期摂動幅 (Algorithm Constant)
/// 初期シンプレックス生成時の各次元の変位割合。
pub const KW4_NELDER_MEAD_INITIAL_PERTURBATION: f64 = 0.10;

/// シミュレーション tick 数 (Calibration Candidate)
/// 外側ループで調整される。長い tick ほど社会発展の余地が広がるが計算量が増える。
pub const KW4_SIMULATION_TICKS: u64 = 100;

/// 観測間隔 (Calibration Candidate)
/// tick_to_convergence 計算のため、この tick 間隔で mid-simulation メトリクスをサンプリングする。
pub const KW4_OBSERVATION_INTERVAL: u64 = 10;

/// 収束閾値 (Calibration Candidate)
/// s_growth × j_cov がこの値を超えた最初の tick を tick_to_convergence として記録する。
pub const KW4_CONVERGENCE_THRESHOLD: f64 = 0.8;

/// 初期慈悲スコア重み (Calibration Candidate)
/// 外側ループの初期中心点として使用。
pub const KW4_INITIAL_GAMMA_BENEVOLENCE: f64 = 0.30;

/// 初期子ワークフロー比率 (Calibration Candidate)
pub const KW4_INITIAL_CHILD_RATIO: f64 = 0.40;

/// 初期ソフトマックス温度 (Calibration Candidate)
pub const KW4_INITIAL_SOFTMAX_TEMPERATURE: f64 = 0.30;

// ============================================================================
// M1.76-KW2: Ecosystem Growth Metrics 定数 (Calibration Candidates)
// ============================================================================

/// Shannon 多様性指数計算のグリッド分割数 (Calibration Candidate)
/// RFC §15.9.3: 10 (10×10 グリッドで能力空間を量子化)
/// 感度分析推奨範囲: 5-20。大きくすると粒度が細かくなるがサンプル不足に弱くなる。
pub const ECOSYSTEM_GRID_DIVISIONS: usize = 10;

/// BlendedFreshness 人間時間減衰の半減期 (ミリ秒) (Calibration Candidate)
/// F_H = exp(-human_time_ms / HUMAN_FRESHNESS_HALFLIFE_MS)
/// デフォルト: 86,400,000ms = 24時間。24時間で 0.37 に減衰。
pub const HUMAN_FRESHNESS_HALFLIFE_MS: f64 = 86_400_000.0;

/// BlendedFreshness 仮想時刻減衰の半減期 (tick) (Calibration Candidate)
/// F_V = exp(-virtual_ticks / VIRTUAL_FRESHNESS_HALFLIFE)
/// デフォルト: 100 tick。100 tick で 0.37 に減衰。
pub const VIRTUAL_FRESHNESS_HALFLIFE: f64 = 100.0;

// ============================================================================
// M1.76-KW-REAL-P2: GMR 抽象化層 定数 (Calibration Candidates)
// RFC §10.2, §10.3, §10.4 参照
// ============================================================================

/// SoftMin 集約の鋭さ β (Calibration Candidate)
/// RFC §10.2 指定値: 5.0
/// 上げると最も決定論性が低いノードのスコアに全体が引っ張られる。
/// 下げると全ノードの平均的な決定論性が反映される。
pub const SOFT_MIN_BETA: f64 = 5.0;

/// DeterminismScore 拒否閾値 (Safety Invariant)
/// RFC §10.2 指定値: 0.50
/// この値未満の DeterminismScore を持つワークフローを非決定論的として拒否。
pub const DETERMINISM_THRESHOLD: f64 = 0.50;

/// AG-01 RewardSignalChannel デフォルト成功率 (Calibration Candidate)
/// 履歴がない場合の初期値。
pub const AG01_DEFAULT_SUCCESS_RATE: f64 = 0.50;

/// AG-02 UtilityChannel デフォルト期待効用 (Calibration Candidate)
/// 履歴がない場合の初期値。
pub const AG02_DEFAULT_UTILITY: f64 = 0.50;

/// AG-03 NoveltyChannel コサイン距離閾値 (Calibration Candidate)
/// この値未満の距離を「新奇」とみなす。
pub const AG03_NOVELTY_THRESHOLD: f64 = 0.30;

/// AG-04 UrgencyChannel デフォルトデッドライン (tick) (Calibration Candidate)
/// デフォルトの残り tick 数。
pub const AG04_DEFAULT_DEADLINE: f64 = 10.0;

/// AG-05 SafetyChannel 最大リスクスコア (Safety Invariant)
/// この値を超えるリスクスコアは安全でないとみなす。
pub const AG05_MAX_RISK_SCORE: f64 = 0.80;

/// Stage5 分岐: REUSE 選択の中間スコア閾値 (Calibration Candidate)
/// total_score がこの値以上で REUSE が選ばれやすくなる。
pub const STAGE5_REUSE_THRESHOLD: f64 = 0.70;

/// Stage5 分岐: ABORT 選択の低スコア閾値 (Calibration Candidate)
/// total_score がこの値未満で ABORT が選ばれやすくなる。
pub const STAGE5_ABORT_THRESHOLD: f64 = 0.30;

/// GMR 能力拡散確率 (Calibration Candidate)
/// DeterminismScore が閾値を超えた際に能力拡散が発生する確率。
/// Default: 0.30, 感度分析推奨範囲: 0.10-0.90
pub const GMR_DIFFUSION_PROBABILITY: f64 = 0.30;

// ============================================================================
// M1.76-KW-ACCEL: J_kw 社会加速度指標 計算定数
// ============================================================================

/// クラスター係数計算の k 近傍数 (Calibration Candidate)
///
/// Watts-Strogatz 型クラスター係数の近傍サイズとして使用。
/// k が小さいほど局所的な三角形のみを捕捉し、大きいほど大域的な構造を捕捉する。
/// Default: 5, 感度分析推奨範囲: 3-20
/// 社会加速度定義③: 空間クラスター係数の計算精度に影響。
pub const KW_ACCEL_K_NEAREST: usize = 5;

/// 局所密度計算の閾値半径 (Calibration Candidate)
///
/// この値未満の L2 距離を近傍とみなす。
/// 値が小さいほど密なクラスタのみを検出し、大きいほど緩い集積も密度として計上する。
/// Default: 0.3, 感度分析推奨範囲: 0.1-0.8
/// 社会加速度定義③: 局所密度の空間解像度を決定。
pub const KW_ACCEL_DENSITY_RADIUS: f64 = 0.3;

/// ノード密度正規化係数 (Calibration Candidate)
///
/// 理論上の最大ノード数。この値以上で密度 1.0 に漸近することを仮定。
/// グラフが小さい（KW_ACCEL_NODE_DENSITY_MAX 未満）場合、密度は比例的に低下する。
/// Default: 50.0, 感度分析推奨範囲: 20.0-200.0
/// 社会加速度定義②: ワークフロー多層密度の正規化基準として使用。
pub const KW_ACCEL_NODE_DENSITY_MAX: f64 = 50.0;
