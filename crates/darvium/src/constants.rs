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
