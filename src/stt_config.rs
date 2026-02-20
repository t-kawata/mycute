//! Configuration structs for the application.
//!
//! This module defines the `Settings` struct and its sub-structs, which are used to
//! store and manage the application's configuration settings. It also includes
//! default values for various settings.

use crate::constants::{
    DB_DEFAULT_DIRNAME, ED448_KEY_BYTES_LEN, ED448_SIGNATURE_BYTES_LEN, ERR_DB, ERR_DECODE,
    ERR_DECRYPT, ERR_ENCRYPT, ERR_INVALID_SIG, ERR_PARSE_VOTES, ERR_SIGN,
    IDENTITY_LAYER_CACHE_MAX_SIZE, IDENTITY_LAYER_CACHE_TTL_SEC, MODEL_FILENAME_GTCRN,
    MODEL_FILENAME_SILERO_VAD, MODEL_FILENAME_SILERO_VAD_INT8, MODEL_FILENAME_TEN_VAD,
    MODEL_FILENAME_TEN_VAD_INT8, MSG_MY_BASE_URL_FATAL, MYCUTE_DL_DIRNAME, MYCUTE_MODELS_DIRNAME,
    MYCUTE_S3_DIRNAME, MYCUTE_SETTINGS_FILENAME, ST_BAD_REQUEST, ST_INTERNAL_SERVER_ERROR,
};
use crate::mode::rt::rtbl::replaces_bl;
use crate::mode::rt::rtres::errs_res::ApiError;
use crate::utils::crypto::{self, Ed448KeyValuePair};
use crate::utils::my_path::get_mycute_home;
use hex;
use indexmap::IndexMap;
use moka::sync::Cache;
use parking_lot::RwLock;
use sea_orm::DatabaseConnection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum SttEngine {
    OpenAI, // OpenAI 疑似ストリーミング
    #[default]
    Os, // OS ネイティブ音声認識 (macOS: SFSpeechRecognizer / Windows: WinRT SpeechRecognizer)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum VadType {
    #[default]
    SileroInt8,
    Silero,
    TenInt8,
    Ten,
}

impl VadType {
    pub fn filename(&self) -> &'static str {
        match self {
            VadType::SileroInt8 => MODEL_FILENAME_SILERO_VAD_INT8,
            VadType::Silero => MODEL_FILENAME_SILERO_VAD,
            VadType::TenInt8 => MODEL_FILENAME_TEN_VAD_INT8,
            VadType::Ten => MODEL_FILENAME_TEN_VAD,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LlmEndpoint {
    pub name: String,
    pub base_url: String,
    pub api_key: Option<String>,
    pub model: String,
}

// ========================================
// SttSettings: 汎用的な音声処理パイプライン設定 (VAD, Denoiser, etc.)
// ========================================

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SttSettings {
    #[serde(skip)]
    pub model_dir: Option<String>,
    #[serde(default = "default_num_threads")]
    pub num_threads: i32,

    // VAD 設定
    #[serde(default)]
    pub vad_type: VadType, // "silero_int8", "silero", "ten_int8", "ten"
    pub vad_model_path: Option<String>,
    #[serde(default = "default_vad_threshold")]
    pub vad_threshold: f32,
    #[serde(default = "default_vad_min_silence_duration")]
    pub vad_min_silence_duration: f32,
    #[serde(default = "default_vad_min_speech_duration")]
    pub vad_min_speech_duration: f32,

    /// VAD インスタンスが内部的に確保するリングバッファの長さ（秒）および最大発話時間。
    ///
    /// このバッファは、VAD が発話区間を判定するために使用する音声データを一時的に蓄積するためのものです。
    /// 一つの連続した発話セグメント（前後のパディングを含む）がこの秒数を超えると、バッファが溢れて
    /// 正常にチャンクを切り出せなくなる可能性があります。
    ///
    /// また、この値は「VAD が発話中と判定し続けても、強制的に発話を終了させるタイムアウト」
    /// の基準としても使用されます。環境ノイズ等で VAD が「発話終了」を検出できない
    /// 場合でも、この時間を超えれば強制的に認識結果を確定させます。
    ///
    /// ※ 注意：これはモデル（Silero/TEN）側の制約ではなく、アプリケーション側で「最大何秒までの
    ///   一息の嗋りを許容するか」を決定するパラメータです。秒数に比例して VAD 用メモリの確保量が
    ///   増加します。OpenAI API を使用する場合は、長すぎるとタイムアウトやメモリ不足の原因に
    ///   なりうるため、適切な値（15～30秒程度）を設定してください。
    ///
    /// デフォルト: 25.0秒
    #[serde(default = "default_vad_max_speech_duration")]
    pub vad_max_speech_duration: f32,

    // 発話区間バッファリング設定
    #[serde(default = "default_vad_pre_padding_ms")]
    pub vad_pre_padding_ms: u64,

    // 窓（ウィンドウ）管理設定
    #[serde(default = "default_utterance_min_ms")]
    pub utterance_min_ms: u64,
    #[serde(default = "default_window_max_ms")]
    pub window_max_ms: u64,

    // その他
    #[serde(default = "default_true")]
    pub use_punctuation: bool,
    #[serde(default = "default_true")]
    pub use_script_filter: bool,

    // ノイズ除去 (GTCRN)
    #[serde(default = "default_true")]
    pub use_denoiser: bool,
    #[serde(default = "default_denoiser_model_path")]
    pub denoiser_model_path: String,

    // 最終補正レイヤー設定
    #[serde(default = "default_fuzzy_threshold")]
    pub fuzzy_threshold: f32,

    // ========================================================================
    // 信号品質チェック・安定化設定
    // ========================================================================
    /// 信号品質チェックを有効にするか（デフォルト: true）
    ///
    /// `true` の場合、ASR に音声を渡す前に「意味のある音声が含まれているか」を
    /// 軽量な計算でチェックします。これにより、窓の端に残った残響や
    /// 極小の音（「はい」と誤認されやすい）が ASR に届くのを防ぎます。
    #[serde(default)]
    pub signal_check_enabled: Option<bool>,

    /// 最小 RMS 閾値（デフォルト: 0.005）
    ///
    /// RMS (Root Mean Square) は音声信号全体の「平均的な音量」を示す指標です。
    /// この値未満の信号は「ほぼ無音」と判断され、ASR に渡されません。
    ///
    /// - 小さい値 (例: 0.001): 非常に静かな発声も許容（誤検知が増える可能性）
    /// - 大きい値 (例: 0.01): ある程度はっきりした発声のみ許容（声が小さい人はカットされる可能性）
    /// - 推奨: 0.005（通常の室内環境での適正値）
    #[serde(default)]
    pub signal_rms_threshold: Option<f32>,

    /// 有意な音声の占有率閾値（デフォルト: 0.15）
    ///
    /// 窓（ウィンドウ）全体に対し、「音圧が RMS 閾値を超えているサンプル」の割合を示します。
    /// 例えば 0.15 = 15% の場合、窓全体の 15% 以上が有意な音であれば ASR に渡します。
    ///
    /// - 小さい値 (例: 0.05): ごく短い音でも ASR に渡す（誤認識リスク増）
    /// - 大きい値 (例: 0.30): ある程度まとまった発話がないと ASR に渡さない
    /// - 推奨: 0.15（窓の約 1/6 以上に音がある場合のみ認識）
    #[serde(default)]
    pub signal_occupancy_ratio: Option<f32>,

    // Post Correction Settings
    /// 最終補正を起動する文数の閾値
    #[serde(default = "default_post_correction_sentence_count_threshold")]
    pub post_correction_sentence_count_threshold: usize,
    /// 最終補正を起動する最小文字数（文数条件の補助）
    #[serde(default = "default_post_correction_min_text_length")]
    pub post_correction_min_text_length: usize,
    /// 最終補正を起動する最小経過時間（ミリ秒）
    #[serde(default = "default_post_correction_interval_ms")]
    pub post_correction_interval_ms: u64,
}

impl Default for SttSettings {
    fn default() -> Self {
        Self {
            model_dir: None, // ConfigManager::new で ~/.mycute/models に設定される
            num_threads: default_num_threads(),
            vad_type: VadType::default(),
            vad_model_path: None,
            vad_threshold: default_vad_threshold(),
            vad_min_silence_duration: default_vad_min_silence_duration(),
            vad_min_speech_duration: default_vad_min_speech_duration(),
            vad_max_speech_duration: default_vad_max_speech_duration(),
            vad_pre_padding_ms: default_vad_pre_padding_ms(),
            utterance_min_ms: default_utterance_min_ms(),
            window_max_ms: default_window_max_ms(),
            use_punctuation: true,
            use_script_filter: true,
            use_denoiser: true,
            denoiser_model_path: default_denoiser_model_path(),
            fuzzy_threshold: default_fuzzy_threshold(),
            signal_check_enabled: None,
            signal_rms_threshold: None,
            signal_occupancy_ratio: None,
            post_correction_sentence_count_threshold:
                default_post_correction_sentence_count_threshold(),
            post_correction_min_text_length: default_post_correction_min_text_length(),
            post_correction_interval_ms: default_post_correction_interval_ms(),
        }
    }
}

impl SttSettings {
    /// モデルディレクトリベースでパスを解決
    pub fn resolve_path(&self, path: &str) -> String {
        if path.is_empty() {
            return String::new();
        }

        let p = Path::new(path);
        if p.is_absolute() {
            return path.to_string();
        }

        let dir_str = self
            .model_dir
            .as_ref()
            .expect("CRITICAL: model_dir must be set before resolving paths");
        let dir = PathBuf::from(dir_str);
        dir.join(path).to_string_lossy().into_owned()
    }

    /// Denoiserモデルパスを取得 (設定されたパスを解決。空なら例外)
    pub fn get_denoiser_path(&self) -> String {
        if self.denoiser_model_path.is_empty() {
            panic!("CRITICAL: denoiser_model_path is empty");
        }
        self.resolve_path(&self.denoiser_model_path)
    }

    /// VADモデルパスを取得
    pub fn get_vad_path(&self) -> String {
        if let Some(ref path) = self.vad_model_path {
            if !path.is_empty() {
                return self.resolve_path(path);
            }
        }
        let filename = self.vad_type.filename();
        self.resolve_path(filename)
    }
}

fn default_true() -> bool {
    true
}

fn default_num_threads() -> i32 {
    4
}

fn default_vad_threshold() -> f32 {
    0.5
}

fn default_denoiser_model_path() -> String {
    MODEL_FILENAME_GTCRN.to_string()
}

fn default_vad_min_silence_duration() -> f32 {
    0.5
}

fn default_vad_min_speech_duration() -> f32 {
    0.25
}

fn default_vad_max_speech_duration() -> f32 {
    25.0
}

// ASR パイプライン共通デフォルト関数

fn default_vad_pre_padding_ms() -> u64 {
    200
}

fn default_utterance_min_ms() -> u64 {
    300 // 300ms
}

fn default_window_max_ms() -> u64 {
    25000 // 25秒（モデル限界30秒に対して余裕を持たせる）
}

fn default_fuzzy_threshold() -> f32 {
    0.3
}

fn default_post_correction_sentence_count_threshold() -> usize {
    3
}

fn default_post_correction_min_text_length() -> usize {
    10
}

fn default_post_correction_interval_ms() -> u64 {
    2000
}

pub use crate::types::LocaleCode;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "snake_case")]
pub enum WindowPositionMode {
    TopLeft,
    #[default]
    BottomLeft,
    TopRight,
    BottomRight,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct WindowPositionConfig {
    #[serde(default)]
    pub mode: WindowPositionMode,
    #[serde(default = "default_pos_offset")]
    pub top: i32,
    #[serde(default = "default_pos_offset")]
    pub bottom: i32,
    #[serde(default = "default_pos_offset")]
    pub left: i32,
    #[serde(default = "default_pos_offset")]
    pub right: i32,
}

fn default_pos_offset() -> i32 {
    0
}

impl Default for WindowPositionConfig {
    fn default() -> Self {
        Self {
            mode: WindowPositionMode::default(),
            top: 0,
            bottom: 50,
            left: 20,
            right: 0,
        }
    }
}

/// オーバーレイウィンドウの表示状態設定
///
/// 【重要】 全値論理ピクセル管理
/// - 位置 (x, y): メインディスプレイ基準の「論理ピクセル」相対座標。
/// - サイズ (width, height): 「論理ピクセル」絶対値。
/// OSが各ディスプレイのスケールに応じた物理描画を自動的に行うため、
/// 論理値を保存することで「見かけのサイズ」が環境に依存せず一定に維持される。
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OverlayStateConfig {
    /// メインディスプレイの左上を原点 (0,0) とした論理X座標
    pub x: i32,
    /// メインディスプレイの左上を原点 (0,0) とした論理Y座標
    pub y: i32,
    /// ウィンドウの論理幅
    pub width: f64,
    /// ウィンドウの論理高さ
    pub height: f64,
}

impl Default for OverlayStateConfig {
    /// デフォルトではメインディスプレイの左上から指定の余白（論理ピクセル）を開けた位置に配置
    fn default() -> Self {
        Self {
            x: OVERLAY_RESET_MARGIN_X,
            y: OVERLAY_RESET_MARGIN_Y,
            width: DEFAULT_OVERLAY_WIDTH,
            height: DEFAULT_OVERLAY_HEIGHT,
        }
    }
}

#[derive(serde::Serialize, serde::Deserialize, Default, Debug, Clone, PartialEq)]
pub struct ForumState {
    /// このフォーラムにおける残り予算 (初期値はフォーラム設定による)
    pub balance: i32,
    /// このフォーラムにおけるアプリへの投票履歴
    /// キー: app_id (UUID), 値: 投票数
    #[serde(default)]
    pub votes: HashMap<String, i32>,
}

#[derive(serde::Serialize, serde::Deserialize, Default, Debug, Clone, PartialEq)]
pub struct CaEntry {
    /// この CA との最終ブラックリスト同期時刻 (ミリ秒)
    #[serde(default)]
    pub last_blacklist_sync_ts: i64,
    /// フォーラムごとの状態
    /// キー: forum_id (UUID), 値: ForumState
    /// HashMap<forum_id, ForumState>
    #[serde(default)]
    pub forum_states: HashMap<String, ForumState>,
}

/// ノードが持つ全ての CA に対するステータスを集約した構造体。
/// 暗号化・署名された状態で設定ファイル (my_rem) に保存される。
#[derive(serde::Serialize, serde::Deserialize, Default, Debug, Clone)]
pub struct MyRemPayload {
    /// CA のベース URL をキーとしたマップ。
    /// 各エントリには、その CA 固有の同期情報と、フォーラムごとの財布(ForumState)が格納される。
    /// HashMap<ca_base_url, CaEntry { last_blacklist_sync_ts, forum_states: HashMap<forum_id, ForumState> }>
    #[serde(default)]
    pub ca_entries: HashMap<String, CaEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum RunRole {
    #[default]
    Client, // -r c
    Server,       // -r s
    ClientServer, // -r cs
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ServerSettings {
    #[serde(default = "default_rt_proto")]
    pub rt_proto: String,
    #[serde(default = "default_rt_host")]
    pub rt_host: String,
    #[serde(default = "default_rt_port")]
    pub rt_port: u16,
    #[serde(default = "default_rt_skey")]
    pub rt_skey: String,
    #[serde(default = "default_rt_crypto_key")]
    pub rt_crypto_key: String,
    #[serde(default = "default_sw_port")]
    pub sw_port: u16,
    #[serde(default = "default_cors_on_rt")]
    pub cors_on_rt: bool,
    #[serde(default = "default_rotation_days")]
    pub rt_crypto_key_rotation_days: u64,
    #[serde(default)]
    pub last_rotated_at: Option<String>,
    /// ノード自身の公式ベースURL。
    /// CA として振る舞う際、エントリーしたノードに返却される。
    /// 起動時に設定が必須であり、未設定の場合はノードが起動しない。
    #[serde(default)]
    pub my_base_url: Option<String>,
    /// CAが証明書の更新申請を受け付ける残り日数。
    /// 期限切れ、または期限までこの日数を切っている場合に再申請を許可する。
    #[serde(default = "default_ca_renew_window_days")]
    pub ca_renew_window_days: u32,
}

impl ServerSettings {
    pub fn api_base_url(&self) -> String {
        format!("{}://{}:{}", self.rt_proto, self.rt_host, self.rt_port)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum DbDriver {
    Mysql,
    Postgres,
    Sqlite,
}

impl Default for DbDriver {
    fn default() -> Self {
        Self::Sqlite
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct DbInfo {
    #[serde(default)]
    pub driver: DbDriver, // mysql, postgres, sqlite
    #[serde(default)]
    pub host: String, // SQLiteの場合はファイル名として使用
    #[serde(default)]
    pub port: String,
    #[serde(default)]
    pub user: String,
    #[serde(default)]
    pub pass: String,
}

// オーバーレイのデフォルトサイズとセーフマージン
pub const DEFAULT_OVERLAY_WIDTH: f64 = 400.0;
pub const DEFAULT_OVERLAY_HEIGHT: f64 = 200.0;
pub const OVERLAY_RESET_MARGIN_X: i32 = 100;
pub const OVERLAY_RESET_MARGIN_Y: i32 = 100;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StorageSettings {
    #[serde(default = "default_db_dir_path")]
    pub db_dir_path: String,
    #[serde(default = "default_false")]
    pub s3_use_local: bool,
    #[serde(default = "default_s3_local_dir")]
    pub s3_local_dir: String,
    #[serde(default = "default_s3_down_dir")]
    pub s3_down_dir: String,
    #[serde(default = "default_dummy")]
    pub s3_access_key: String,
    #[serde(default = "default_dummy")]
    pub s3_secret_access_key: String,
    #[serde(default = "default_dummy")]
    pub s3_region: String,
    #[serde(default = "default_dummy")]
    pub s3_bucket: String,
    #[serde(default = "default_s3_min_free_disk")]
    pub s3_min_free_disk: u64,
    #[serde(default)]
    pub rw_db: DbInfo,
    #[serde(default)]
    pub ro_dbs: Vec<DbInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CuberSettings {
    #[serde(default = "default_cuber_crypto_secret_key")]
    pub crypto_secret_key: String,
    #[serde(default = "default_true")]
    pub debug: bool,
    #[serde(default = "default_60")]
    pub storage_idle_timeout_minutes: u64,
    #[serde(default = "default_60")]
    pub s3_cleanup_interval_minutes: u64,
    #[serde(default = "default_24")]
    pub s3_retention_hours: u64,
    #[serde(default = "default_50000")]
    pub memify_max_chars: usize,
    #[serde(default = "default_20")]
    pub memify_overlap_percent: usize,
    #[serde(default = "default_5000")]
    pub memify_batch_min_chars: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Settings {
    #[serde(default)]
    pub hotkeys: HotkeyConfig,
    #[serde(default)]
    pub stt_engine: SttEngine,
    #[serde(default)]
    pub locale: LocaleCode,
    #[serde(default)]
    pub llms: Vec<LlmEndpoint>,
    #[serde(default)]
    pub stt: SttSettings,
    // Server & Infra integration
    #[serde(default)]
    pub server: ServerSettings,
    #[serde(default)]
    pub storage: StorageSettings,
    #[serde(default)]
    pub cuber: CuberSettings,
    #[serde(default)]
    pub window_position: WindowPositionConfig,
    #[serde(default)]
    pub overlay_state: OverlayStateConfig,
    #[serde(default)]
    /// プロキシサーバーが実際に使用するサーバー証明書 (Base64)
    pub proxy_certificate: Option<String>,
    #[serde(default)]
    /// プロキシサーバーが実際に使用する秘密鍵 (Base64)
    pub proxy_private_key: Option<String>,
    #[serde(default)]
    /// サーバー証明書を発行するためのルート認証局 (OSCA) の証明書 (Base64)
    pub osca_certificate: Option<String>,
    #[serde(default)]
    /// サーバー証明書を発行するためのルート認証局 (OSCA) の秘密鍵 (Base64)
    pub osca_private_key: Option<String>,
    #[serde(default)]
    /// ルート認証局 (OSCA) 証明書の有効期限 (RFC3339形式)
    pub osca_expire: Option<String>,

    #[serde(default)]
    /// Node Identity Public Key (Encrypted Base64)
    pub my_pub: Option<String>,
    #[serde(default)]
    /// Node Identity Private Key (Encrypted Base64)
    pub my_sec: Option<String>,
    #[serde(default)]
    /// Remaining Voting Credits (Encrypted: "{credits}:{signature}")
    pub my_rem: Option<String>,
    #[serde(default)]
    /// My CA Token (Encrypted Base64/Hex)
    pub my_cat: Option<String>,
}

fn default_rt_proto() -> String {
    "http".to_string()
}
fn default_ca_renew_window_days() -> u32 {
    7
}

impl Settings {
    /// ノード自身のベースURL（my_base_url）が設定されているか検証する。
    /// 未設定かつオーナーノードでない場合は、致命的エラーとしてパニックさせる。
    pub fn validate_my_base_url(&self, is_owner: bool) {
        if is_owner {
            // オーナーノードは特殊なため検証をスキップ
            return;
        }

        let my_base_url = &self.server.my_base_url;
        if my_base_url.is_none()
            || my_base_url
                .as_ref()
                .map(|u| u.trim().is_empty())
                .unwrap_or(true)
        {
            panic!("{}", MSG_MY_BASE_URL_FATAL);
        }
        log::info!("[Startup] My Base URL: {}", my_base_url.as_ref().unwrap());
    }
}
fn default_rt_host() -> String {
    "localhost".to_string()
}
fn default_rt_port() -> u16 {
    8888
}
fn default_sw_port() -> u16 {
    8889
}
fn default_rt_skey() -> String {
    "6JsfNZwZgc4VvDZyvhebvjVz/+J3IkKpvkb++HYc39Y/=".to_string()
}
fn default_rt_crypto_key() -> String {
    "kS9yzX2!vB5*mN8@qW0&eP3_rY6*tU9!".to_string()
}
fn default_cors_on_rt() -> bool {
    true
}
fn default_rotation_days() -> u64 {
    90
}

fn default_false() -> bool {
    false
}
fn default_dummy() -> String {
    "dummy".to_string()
}
fn default_db_dir_path() -> String {
    // デフォルトは ~/.mycute/db だが、ここではダミーを返し
    // 実際のパスは ConfigManager または new_with_home で上書きする
    DB_DEFAULT_DIRNAME.to_string()
}

fn default_s3_local_dir() -> String {
    MYCUTE_S3_DIRNAME.to_string()
}
fn default_s3_down_dir() -> String {
    MYCUTE_DL_DIRNAME.to_string()
}

fn default_s3_min_free_disk() -> u64 {
    15
}

fn default_cuber_crypto_secret_key() -> String {
    "bDRe9DD3tBaG47Ygb8-c6Fn9_3F-LyhM".to_string()
}
fn default_60() -> u64 {
    60
}
fn default_24() -> u64 {
    24
}
fn default_50000() -> usize {
    50000
}
fn default_20() -> usize {
    20
}
fn default_5000() -> usize {
    5000
}

impl Default for ServerSettings {
    fn default() -> Self {
        Self {
            rt_proto: default_rt_proto(),
            rt_host: default_rt_host(),
            rt_port: default_rt_port(),
            rt_skey: default_rt_skey(),
            rt_crypto_key: default_rt_crypto_key(),
            sw_port: default_sw_port(),
            cors_on_rt: default_cors_on_rt(),
            rt_crypto_key_rotation_days: default_rotation_days(),
            last_rotated_at: None,
            my_base_url: None,
            ca_renew_window_days: default_ca_renew_window_days(),
        }
    }
}

impl Default for StorageSettings {
    fn default() -> Self {
        Self {
            db_dir_path: default_db_dir_path(),
            s3_use_local: default_false(),
            s3_local_dir: default_s3_local_dir(),
            s3_down_dir: default_s3_down_dir(),
            s3_access_key: default_dummy(),
            s3_secret_access_key: default_dummy(),
            s3_region: default_dummy(),
            s3_bucket: default_dummy(),
            s3_min_free_disk: default_s3_min_free_disk(),
            rw_db: DbInfo::default(),
            ro_dbs: Vec::new(),
        }
    }
}

impl StorageSettings {
    pub fn new_with_home(home: &Path) -> Self {
        Self {
            db_dir_path: home.join(DB_DEFAULT_DIRNAME).to_string_lossy().to_string(),
            s3_local_dir: home.join(MYCUTE_S3_DIRNAME).to_string_lossy().to_string(),
            s3_down_dir: home.join(MYCUTE_DL_DIRNAME).to_string_lossy().to_string(),
            ..Default::default()
        }
    }
}

impl Default for CuberSettings {
    fn default() -> Self {
        Self {
            crypto_secret_key: default_cuber_crypto_secret_key(),
            debug: default_true(),
            storage_idle_timeout_minutes: default_60(),
            s3_cleanup_interval_minutes: default_60(),
            s3_retention_hours: default_24(),
            memify_max_chars: default_50000(),
            memify_overlap_percent: default_20(),
            memify_batch_min_chars: default_5000(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct HotkeyConfig {
    pub start: Vec<String>, // e.g., ["Option", "KeyS"]
    pub correct: Vec<String>,
    pub summarize: Vec<String>,
    #[serde(default = "default_toggle_locale")]
    pub toggle_locale: Vec<String>,
    #[serde(default = "default_buffer_start")]
    pub buffer_start: Vec<String>, // e.g., ["Option", "KeyB"]
    #[serde(default = "default_buffer_flush")]
    pub buffer_flush: Vec<String>, // e.g., ["Option", "KeyF"]
    #[serde(default = "default_settings_hotkey")]
    pub settings: Vec<String>, // e.g., ["Option", "KeyJ"]
    #[serde(default = "default_help_hotkey")]
    pub help: Vec<String>, // e.g., ["Option", "KeyC"]
    #[serde(default = "default_usage_stats_hotkey")]
    pub usage_stats: Vec<String>, // e.g., ["Option", "KeyU"]
}

fn default_settings_hotkey() -> Vec<String> {
    vec!["Option".to_string(), "KeyJ".to_string()]
}

fn default_toggle_locale() -> Vec<String> {
    vec!["Option".to_string(), "KeyL".to_string()]
}

fn default_help_hotkey() -> Vec<String> {
    vec!["Option".to_string(), "KeyC".to_string()]
}

fn default_buffer_start() -> Vec<String> {
    vec!["Option".to_string(), "KeyB".to_string()]
}

fn default_buffer_flush() -> Vec<String> {
    vec!["Option".to_string(), "KeyF".to_string()]
}

fn default_usage_stats_hotkey() -> Vec<String> {
    vec!["Option".to_string(), "KeyU".to_string()]
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            start: vec!["Option".to_string(), "KeyS".to_string()],
            correct: vec!["Option".to_string(), "KeyH".to_string()],
            summarize: vec!["Option".to_string(), "KeyM".to_string()],
            toggle_locale: default_toggle_locale(),
            buffer_start: default_buffer_start(),
            buffer_flush: default_buffer_flush(),
            settings: default_settings_hotkey(),
            help: default_help_hotkey(),
            usage_stats: default_usage_stats_hotkey(),
        }
    }
}

pub struct ConfigManager {
    pub home_dir: PathBuf,
    pub path: PathBuf,
    pub settings: Arc<RwLock<Settings>>,
    /// メモリ上のみに存在するオーナー秘密鍵 (Owner Mode用)
    pub owner_key: Arc<RwLock<Option<crypto::Ed448RawKeyPair>>>,
    /// CA 選定用のラウンドロビンカウンタ
    pub ca_selection_counter: AtomicUsize,
    /// アイデンティティレイヤーの判定キャッシュ (LRU / TTL 管理)
    /// Key: (NodePubKey, CAPubKey), Value: IdentityLayer (as u8)
    pub identity_layer_cache: Cache<(String, String), u8>,
    /// 推奨 CA URL のリストキャッシュ (Periodical Store Task により更新される)
    pub reliable_ca_cache: Arc<RwLock<Option<Vec<String>>>>,
    /// DBから読み込まれたアクティブな置換辞書
    /// キー: 置換後のテキスト, 値: 置換前のテキストのリスト
    pub replaces: Arc<RwLock<IndexMap<String, Vec<String>>>>,
    /// 現在アクティブな辞書セットIDのリスト
    pub replaces_active_ids: Arc<RwLock<Vec<Uuid>>>,
}

impl ConfigManager {
    pub fn new(forced_home: Option<String>, forced_settings: Option<String>) -> Self {
        let home_dir = get_mycute_home(forced_home);

        let config_path = if let Some(s) = forced_settings {
            PathBuf::from(s)
        } else {
            home_dir.join(MYCUTE_SETTINGS_FILENAME)
        };

        let mut settings = if config_path.exists() {
            let content = fs::read_to_string(&config_path).unwrap_or_default();
            serde_json::from_str::<Settings>(&content).unwrap_or_else(|e| {
                log::error!("Failed to parse settings at {:?}: {}", config_path, e);
                Settings::new_with_home(&home_dir)
            })
        } else {
            Settings::new_with_home(&home_dir)
        };

        // [Path Normalization]
        // 設定ファイルから読み込まれた、あるいはデフォルト値の「相対パス」を
        // 確定した home_dir を基準とした絶対パスに変換する。
        // これにより、settings.json からパス設定を削除しても意図通り ~/.mycute/ 以下が使われる。
        {
            let mut s = settings.storage.clone();
            let mut changed = false;

            let mut normalize = |path: &mut String| {
                let p = Path::new(path);
                if p.is_relative() {
                    let abs = home_dir.join(p).to_string_lossy().to_string();
                    log::debug!("Normalizing path: {} -> {}", path, abs);
                    *path = abs;
                    changed = true;
                }
            };

            normalize(&mut s.db_dir_path);
            normalize(&mut s.s3_local_dir);
            normalize(&mut s.s3_down_dir);

            if changed {
                settings.storage = s;
                // 設定ファイル自体は書き換えず、オンメモリの設定のみを正規化する
                // (ユーザーが相対パスを書いたつもりでも実行時は絶対パスで動く)
            }

            // [Model Dir Normalization]
            // model_dir は設定ファイルからは読み込まず、常に強制的に ~/.mycute/models を設定する
            // これにより OS 間のポータビリティを確保する
            let mut s = settings.stt.clone();
            if s.model_dir.is_none() || s.model_dir.as_ref().map(|s| s.is_empty()).unwrap_or(false)
            {
                let default_models = home_dir
                    .join(MYCUTE_MODELS_DIRNAME)
                    .to_string_lossy()
                    .to_string();
                log::debug!("Setting dynamic model_dir: {}", default_models);
                s.model_dir = Some(default_models);
            }
            settings.stt = s;
        }

        let manager = Self {
            home_dir,
            path: config_path,
            settings: Arc::new(RwLock::new(settings)),
            owner_key: Arc::new(RwLock::new(None)),
            ca_selection_counter: AtomicUsize::new(0),
            identity_layer_cache: Cache::builder()
                .max_capacity(IDENTITY_LAYER_CACHE_MAX_SIZE as u64)
                .time_to_live(Duration::from_secs(IDENTITY_LAYER_CACHE_TTL_SEC))
                .build(),
            reliable_ca_cache: Arc::new(RwLock::new(None)),
            replaces: Arc::new(RwLock::new(IndexMap::new())),
            replaces_active_ids: Arc::new(RwLock::new(Vec::new())),
        };

        // 必須ディレクトリ構造の強制作成
        if let Err(e) = manager.ensure_dir_structure() {
            log::error!("Failed to ensure directory structure: {}", e);
        }

        // [Strict Anti-Tampering]
        // 起動時に my_rem の整合性をチェックする。
        // キーペアが存在する場合（＝ID確立済み）のみチェックを行う。
        if let Ok(kp) = manager.get_node_keypair() {
            match manager.load_my_rem_payload(&kp) {
                Ok(_) => {
                    // 正常: エントリー済みかつデータ健全
                    log::info!("Identity Integrity Check Passed.");
                }
                Err(e) => {
                    // エラーの種別判定
                    // NOT_FOUND (データなし) は正常（未エントリー状態）
                    // それ以外（復号失敗、署名不一致など）は致命的不正
                    // ApiError は errors: Vec<ErrorDetail> を持つため、その中身を確認する
                    if e.errors.iter().any(|d| d.code == "NOT_FOUND") {
                        log::info!(
                            "Identity established but not entered to any CA yet (No my_rem)."
                        );
                    } else {
                        log::error!("CRITICAL: my_rem integrity check failed on startup: {}", e);
                        panic!(
                            "CRITICAL: my_rem corrupted or tampered. Node cannot start. Cause: {}",
                            e
                        );
                    }
                }
            }
        }

        manager
    }

    pub fn save(&self) -> Result<(), String> {
        let settings = self.settings.read();
        let content = serde_json::to_string_pretty(&*settings)
            .map_err(|e| format!("Failed to serialize settings: {}", e))?;

        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }

        fs::write(&self.path, content).map_err(|e| format!("Failed to write settings: {}", e))
    }

    /// アプリケーション実行に必要なディレクトリ構造を強制的に作成します。
    pub fn ensure_dir_structure(&self) -> Result<(), String> {
        let settings = self.settings.read();
        let dirs = [
            &settings.storage.db_dir_path,
            &settings.storage.s3_local_dir,
            &settings.storage.s3_down_dir,
        ];

        for dir in dirs {
            let p = Path::new(dir);
            if !p.exists() {
                log::info!("Creating directory: {}", dir);
                fs::create_dir_all(p)
                    .map_err(|e| format!("Failed to create dir {}: {}", dir, e))?;
            }
        }

        if let Some(ref model_dir) = settings.stt.model_dir {
            let p = Path::new(model_dir);
            if !p.exists() {
                log::info!("Creating models directory: {}", model_dir);
                fs::create_dir_all(p)
                    .map_err(|e| format!("Failed to create models dir {}: {}", model_dir, e))?;
            }
        }

        Ok(())
    }

    /// 設定されたモデルファイルが物理的に存在することを厳格にチェックします。
    pub fn validate_models(&self) -> Result<(), String> {
        let settings = self.settings.read();
        if settings.stt_engine != SttEngine::OpenAI {
            return Ok(());
        }

        let denoiser_path = settings.stt.get_denoiser_path();
        if !Path::new(&denoiser_path).exists() {
            return Err(format!("Denoiser model file not found: {}", denoiser_path));
        }

        let vad_path = settings.stt.get_vad_path();
        if !Path::new(&vad_path).exists() {
            return Err(format!("VAD model file not found: {}", vad_path));
        }

        log::debug!(
            "[Validation] All models present: denoiser={}, vad={}",
            denoiser_path,
            vad_path
        );
        Ok(())
    }

    /// ノードの Ed448 鍵ペア（公開鍵・秘密鍵）を取得する。
    /// Owner Mode の場合はエラーを返す。
    pub fn get_node_keypair(&self) -> Result<Ed448KeyValuePair, ApiError> {
        {
            let guard = self.owner_key.read();
            if guard.is_some() {
                return Err(ApiError::new_system(
                    ST_BAD_REQUEST,
                    "OWNER_MODE",
                    "Node identity not available in Owner Mode.",
                ));
            }
        }

        let settings = self.settings.read();
        let my_pub_enc = settings.my_pub.clone().ok_or_else(|| {
            ApiError::new_system(
                ST_INTERNAL_SERVER_ERROR,
                "NO_IDENTITY",
                "Node public identity not found.",
            )
        })?;
        let my_sec_enc = settings.my_sec.clone().ok_or_else(|| {
            ApiError::new_system(
                ST_INTERNAL_SERVER_ERROR,
                "NO_IDENTITY",
                "Node secret identity not found.",
            )
        })?;
        let crypto_key = &settings.server.rt_crypto_key;

        let pub_hex = crypto::decrypt(&my_pub_enc, crypto_key).map_err(|e| {
            ApiError::new_system(
                ST_INTERNAL_SERVER_ERROR,
                ERR_DECRYPT,
                format!("Failed to decrypt public key: {}", e),
            )
        })?;
        let pub_bytes = hex::decode(pub_hex).map_err(|e| {
            ApiError::new_system(
                ST_INTERNAL_SERVER_ERROR,
                ERR_DECODE,
                format!("Failed to decode public key: {}", e),
            )
        })?;

        let sec_hex = crypto::decrypt(&my_sec_enc, crypto_key).map_err(|e| {
            ApiError::new_system(
                ST_INTERNAL_SERVER_ERROR,
                ERR_DECRYPT,
                format!("Failed to decrypt secret key: {}", e),
            )
        })?;
        let sec_bytes = hex::decode(sec_hex).map_err(|e| {
            ApiError::new_system(
                ST_INTERNAL_SERVER_ERROR,
                ERR_DECODE,
                format!("Failed to decode secret key: {}", e),
            )
        })?;

        if pub_bytes.len() != ED448_KEY_BYTES_LEN || sec_bytes.len() != ED448_KEY_BYTES_LEN {
            return Err(ApiError::new_system(
                ST_INTERNAL_SERVER_ERROR,
                "INVALID_KEY",
                "Invalid key length.",
            ));
        }

        let mut public = [0u8; ED448_KEY_BYTES_LEN];
        public.copy_from_slice(&pub_bytes);
        let mut secret = [0u8; ED448_KEY_BYTES_LEN];
        secret.copy_from_slice(&sec_bytes);

        Ok(Ed448KeyValuePair { secret, public })
    }

    /// my_rem ペイロードを復号してパースする。
    pub fn load_my_rem_payload(
        &self,
        keypair: &Ed448KeyValuePair,
    ) -> Result<MyRemPayload, ApiError> {
        let (my_rem_opt, crypto_key) = {
            let s = self.settings.read();
            (s.my_rem.clone(), s.server.rt_crypto_key.clone())
        };

        let Some(rem_enc) = my_rem_opt else {
            return Err(ApiError::new_system(
                ST_BAD_REQUEST,
                "NOT_FOUND",
                "Identity not established. You must perform 'Entry' with CA to receive your initial balance."
            ));
        };

        let rem_dec = crypto::decrypt(&rem_enc, &crypto_key).map_err(|e| {
            ApiError::new_system(
                ST_INTERNAL_SERVER_ERROR,
                ERR_DECRYPT,
                format!("Failed to decrypt my_rem: {}", e),
            )
        })?;

        let parts: Vec<&str> = rem_dec.splitn(2, ':').collect();
        if parts.len() != 2 {
            return Err(ApiError::new_system(
                ST_INTERNAL_SERVER_ERROR,
                ERR_INVALID_SIG,
                "Invalid my_rem format.",
            ));
        }
        let json_str = parts[0];
        let sig_hex = parts[1];

        // 署名検証
        let sig_bytes = hex::decode(sig_hex).map_err(|_| {
            ApiError::new_system(
                ST_INTERNAL_SERVER_ERROR,
                ERR_INVALID_SIG,
                "Invalid my_rem signature hex.",
            )
        })?;
        if sig_bytes.len() != ED448_SIGNATURE_BYTES_LEN {
            return Err(ApiError::new_system(
                ST_INTERNAL_SERVER_ERROR,
                ERR_INVALID_SIG,
                "Invalid my_rem signature length.",
            ));
        }
        let mut sig_arr = [0u8; ED448_SIGNATURE_BYTES_LEN];
        sig_arr.copy_from_slice(&sig_bytes);
        let sig_struct = crypto::Ed448Signature { signature: sig_arr };

        if !crypto::verify_signature(&keypair.public, json_str.as_bytes(), &sig_struct)
            .unwrap_or(false)
        {
            return Err(ApiError::new_system(
                ST_INTERNAL_SERVER_ERROR,
                ERR_INVALID_SIG,
                "my_rem integrity check failed.",
            ));
        }

        let payload: MyRemPayload = serde_json::from_str(json_str).map_err(|e| {
            ApiError::new_system(
                ST_BAD_REQUEST, 
                "NOT_FOUND", 
                format!("Failed to parse my_rem. If you upgraded from an older version, please perform 'Entry' with your CA again to migrate your data. Parse error: {}", e)
            )
        })?;

        if payload.ca_entries.is_empty() {
            return Err(ApiError::new_system(
                ST_BAD_REQUEST, 
                "NOT_FOUND", 
                "No CA entries found in my_rem. Please perform 'Entry' with a CA to receive your initial balance."
            ));
        }

        Ok(payload)
    }

    /// my_rem ペイロードを署名・暗号化して文字列化する。
    pub fn encode_my_rem_payload(
        &self,
        payload: &MyRemPayload,
        keypair: &Ed448KeyValuePair,
    ) -> Result<String, ApiError> {
        let crypto_key = {
            let s = self.settings.read();
            s.server.rt_crypto_key.clone()
        };

        let json_str = serde_json::to_string(payload).map_err(|e| {
            ApiError::new_system(
                ST_INTERNAL_SERVER_ERROR,
                ERR_PARSE_VOTES,
                format!("Failed to serialize my_rem: {}", e),
            )
        })?;

        let sig = keypair
            .sign(json_str.as_bytes())
            .map_err(|e| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, ERR_SIGN, e.to_string()))?;
        let sig_hex = hex::encode(sig.signature);

        let rem_payload = format!("{}:{}", json_str, sig_hex);
        let encrypted = crypto::encrypt(&rem_payload, &crypto_key).map_err(|e| {
            ApiError::new_system(ST_INTERNAL_SERVER_ERROR, ERR_ENCRYPT, e.to_string())
        })?;

        Ok(encrypted)
    }

    pub async fn get_ca_entry(&self, ca_base_url: &str) -> Result<CaEntry, ApiError> {
        let kp = self.get_node_keypair()?;
        let payload = self.load_my_rem_payload(&kp)?;
        Ok(payload
            .ca_entries
            .get(ca_base_url)
            .cloned()
            .unwrap_or_default())
    }

    pub async fn set_ca_entry(&self, ca_base_url: &str, entry: CaEntry) -> Result<(), ApiError> {
        let kp = self.get_node_keypair()?;
        let mut payload = self.load_my_rem_payload(&kp)?;

        payload.ca_entries.insert(ca_base_url.to_string(), entry);

        let encrypted = self.encode_my_rem_payload(&payload, &kp)?;

        {
            let mut settings = self.settings.write();
            settings.my_rem = Some(encrypted);
        }
        self.save()
            .map_err(|e| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, ERR_DB, e))?;
        Ok(())
    }

    /// データベースからアクティブな置換辞書をメモリ上にリロードする。
    pub async fn reload_replaces(&self, db: &DatabaseConnection) -> Result<(), String> {
        log::info!("Reloading replaces from DB...");
        match replaces_bl::get_active_replaces_map(db).await {
            Ok((map, ids)) => {
                let count = map.len();
                let set_count = ids.len();
                {
                    let mut guard = self.replaces.write();
                    *guard = map;
                }
                {
                    let mut guard = self.replaces_active_ids.write();
                    *guard = ids;
                }
                log::info!(
                    "Reloaded {} active replace rules from {} sets.",
                    count,
                    set_count
                );
                Ok(())
            }
            Err(e) => {
                let msg = format!("Failed to reload replaces: {}", e);
                log::error!("{}", msg);
                Err(msg)
            }
        }
    }

    pub fn is_active_replace_set(&self, id: &Uuid) -> bool {
        let guard = self.replaces_active_ids.read();
        guard.contains(id)
    }
}

impl Settings {
    pub fn new_default() -> Self {
        Self {
            hotkeys: HotkeyConfig::default(),
            stt_engine: SttEngine::default(),
            locale: LocaleCode::default(),
            llms: Vec::new(),
            stt: SttSettings::default(),
            server: ServerSettings::default(),
            storage: StorageSettings::default(),
            cuber: CuberSettings::default(),
            window_position: WindowPositionConfig::default(),
            overlay_state: OverlayStateConfig::default(),
            proxy_certificate: None,
            proxy_private_key: None,
            osca_certificate: None,
            osca_private_key: None,
            osca_expire: None,
            my_pub: None,
            my_sec: None,
            my_rem: None,
            my_cat: None,
        }
    }

    pub fn new_with_home(home: &Path) -> Self {
        let mut s = Self::new_default();
        s.storage = StorageSettings::new_with_home(home);
        s
    }
}
