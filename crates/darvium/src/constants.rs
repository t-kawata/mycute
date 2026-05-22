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

/// GED ブレンドマージン
pub const GED_BLEND_MARGIN: usize = 5;

/// 最大グラフノード数 (Safety Invariant)
pub const MAX_GRAPH_NODES: usize = 10_000;

/// 最大コンパイルステップ数 (Safety Invariant)
pub const MAX_COMPILED_STEPS: usize = 100_000;

/// 最大パッチ操作数 (Safety Invariant)
pub const MAX_PATCH_OPS: usize = 1_000;

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

/// FakeEmbeddingProvider のデフォルト埋め込み次元数 (Calibration Candidate)
/// Default: 384, 感度分析推奨範囲: 64-1536
pub const FAKE_EMBEDDING_DEFAULT_DIMENSION: usize = 384;

/// VirtualClock のデフォルト開始時刻 (ms) (Safety Invariant)
/// 0 = UNIX epoch (1970-01-01T00:00:00Z)
pub const CLOCK_DEFAULT_START_MS: u64 = 0;

/// 発振検出 最大発振カウント (Calibration Candidate)
/// Default: 3, 感度分析推奨範囲: 1-10
/// Refine↔Retrieve の交互遷移がこの回数に達すると is_oscillating() = true
pub const OSCILLATION_MAX_COUNT: u32 = 3;
