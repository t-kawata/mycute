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
    MYCUTE_S3_DIRNAME, SETTING_KEY_MY_CAT, SETTING_KEY_MY_LICS, SETTING_KEY_MY_PUB,
    SETTING_KEY_MY_REM, SETTING_KEY_MY_SEC, SETTING_KEY_OSCA_CERT, SETTING_KEY_OSCA_EXPIRE,
    SETTING_KEY_OSCA_SEC, SETTING_KEY_PROXY_CERT, SETTING_KEY_PROXY_SEC, ST_BAD_REQUEST,
    ST_INTERNAL_SERVER_ERROR, SETTING_KEY_OSCA_CN, SETTING_KEY_SERVER, DEFAULT_ZEROCLAW_PORT,
    DEFAULT_RT_PORT, DEFAULT_SW_PORT, DEFAULT_BIFROST_PORT
};
use crate::mode::rt::rtbl::replaces_bl;
use crate::mode::rt::rtres::errs_res::ApiError;
use crate::utils::crypto::{self, Ed448KeyValuePair};
use crate::utils::my_path::get_mycute_home;
use crate::utils::db::DbPools;
use crate::myproxy::ssl::setup::{is_cert_expired_with_buffer, is_cert_trusted_by_os};
use anyhow::Context;
use base64::{self, Engine, engine::general_purpose::STANDARD as BASE64_STANDARD};
use hex;
use indexmap::IndexMap;
use moka::sync::Cache;
use parking_lot::RwLock;
use sea_orm::{
    sea_query::{Alias, OnConflict, Query},
    ConnectionTrait, DatabaseConnection, EntityTrait, TransactionTrait,
};
use crate::entities::settings;
use serde::{Deserialize, Serialize};
use serde_json;
// use utoipa::ToSchema; // LlmEndpoint 廃止に伴い不要になった
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;
use std::time::Duration;
use uuid::Uuid;
use crate::utils::process as proc_utils;
use crate::utils::time;

impl ConfigManager {
    /// バックエンドが使用する全ての主要ポートをクリーンアップします。
    /// 
    /// 【重要：今後の拡張ガイドライン】
    /// 今後、新しいポートを LISTEN するコンポーネントを追加した場合は、必ずこの関数の
    /// 取得するポートリストにそのポートを追加してください。
    /// 
    /// これにより、以下の全ての局面でポートの解放が確実に行われるようになります：
    /// 1. RT 起動時の初期クリーンアップ (`main_of_rt.rs`)
    /// 2. RT 側の運命共同体監視による自死時のクリーンアップ (`main_of_rt.rs`)
    /// 3. CL 側のガードによるバックエンド終了時のクリーンアップ (`src/utils/auth.rs`)
    pub fn cleanup_all_backend_ports(&self, tag: &str) {
        let ports = {
            let s = self.settings.read();
            // RT, Bifrost, ZeroClaw の全ポートを対象とする
            vec![s.server.rt_port, s.server.bifrost_port, s.server.zeroclaw_port]
        };
        
        // 共通ユーティリティを使用して一括クリーンアップを実行
        proc_utils::kill_processes_on_ports(&ports, tag);
    }
}

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

// LlmEndpoint は LMGW 移行に伴い廃止済み
// LLM プロバイダー設定は Bifrost のプロバイダー管理画面で行う。

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
    pub fn resolve_path(&self, path: &str) -> anyhow::Result<String> {
        if path.is_empty() {
            return Ok(String::new());
        }

        let p = Path::new(path);
        if p.is_absolute() {
            return Ok(path.to_string());
        }

        let dir_str = self
            .model_dir
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("CRITICAL: model_dir must be set before resolving paths"))?;
        let dir = PathBuf::from(dir_str);
        Ok(dir.join(path).to_string_lossy().into_owned())
    }

    /// Denoiserモデルパスを取得 (設定されたパスを解決。空なら例外)
    pub fn get_denoiser_path(&self) -> anyhow::Result<String> {
        if self.denoiser_model_path.is_empty() {
            anyhow::bail!("CRITICAL: denoiser_model_path is empty");
        }
        self.resolve_path(&self.denoiser_model_path)
    }

    /// VADモデルパスを取得
    pub fn get_vad_path(&self) -> anyhow::Result<String> {
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
    0.2
}

fn default_vad_min_speech_duration() -> f32 {
    0.05
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
    3000
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
            bottom: 20,
            left: 20,
            right: 0,
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
    #[serde(default = "default_bifrost_port")]
    pub bifrost_port: u16,
    #[serde(default = "default_zeroclaw_port")]
    pub zeroclaw_port: u16,
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
    #[serde(rename = "hotkeys", default)]
    pub hotkeys: HotkeyConfig,
    #[serde(skip, rename = "stt_engine", default)]
    pub stt_engine: SttEngine,
    #[serde(skip, rename = "locale", default)]
    pub locale: LocaleCode,
    // llms は LMGW 移行に伴い廃止済み（旧 settings.json の llms フィールドはデシリアライズ時に黙して無視される）
    #[serde(rename = "stt", default)]
    pub stt: SttSettings,
    // Server & Infra integration
    #[serde(rename = "server", default)]
    pub server: ServerSettings,
    #[serde(rename = "storage", default)]
    pub storage: StorageSettings,
    #[serde(rename = "cuber", default)]
    pub cuber: CuberSettings,
    #[serde(rename = "window_position", default)]
    pub window_position: WindowPositionConfig,
    #[serde(rename = "proxy_certificate", default)]
    /// プロキシサーバーが実際に使用するサーバー証明書 (Base64)
    pub proxy_certificate: Option<String>,
    #[serde(rename = "proxy_private_key", default)]
    /// プロキシサーバーが実際に使用する秘密鍵 (Base64)
    pub proxy_private_key: Option<String>,
    #[serde(rename = "osca_certificate", default)]
    /// サーバー証明書を発行するためのルート認証局 (OSCA) の証明書 (Base64)
    pub osca_certificate: Option<String>,
    #[serde(rename = "osca_private_key", default)]
    /// サーバー証明書を発行するためのルート認証局 (OSCA) の秘密鍵 (Base64)
    pub osca_private_key: Option<String>,
    #[serde(rename = "osca_expire", default)]
    /// ルート認証局 (OSCA) 証明書の有効期限 (RFC3339形式)
    pub osca_expire: Option<String>,
    #[serde(rename = "osca_cn", default)]
    /// ルート認証局 (OSCA) の Common Name (CN)。OS の信頼チェックに使用。
    pub osca_cn: Option<String>,
    #[serde(rename = "my_pub", default)]
    /// Node Identity Public Key (Encrypted Base64)
    pub my_pub: Option<String>,
    #[serde(rename = "my_sec", default)]
    /// Node Identity Private Key (Encrypted Base64)
    pub my_sec: Option<String>,
    #[serde(rename = "my_rem", default)]
    /// Remaining Voting Credits (Encrypted: "{credits}:{signature}")
    pub my_rem: Option<String>,
    #[serde(rename = "my_cat", default)]
    /// My CA Token (Encrypted Base64/Hex)
    pub my_cat: Option<String>,
    #[serde(rename = "my_lics", default)]
    /// My Licenses (Encrypted Base64/Hex strings, multiple CAs)
    pub my_lics: Vec<String>,
}

fn default_rt_proto() -> String {
    "http".to_string()
}
fn default_ca_renew_window_days() -> u32 {
    7
}

impl Settings {
    /// リセット操作（do_reset_db）を行っても削除すべきでない、永続的に保持すべき設定キーのリストを返す。
    pub fn protected_settings_keys() -> Vec<&'static str> {
        vec![
            SETTING_KEY_MY_PUB,
            SETTING_KEY_MY_SEC,
            SETTING_KEY_MY_REM,
            SETTING_KEY_MY_CAT,
            SETTING_KEY_MY_LICS,
            SETTING_KEY_SERVER,
            SETTING_KEY_PROXY_CERT,
            SETTING_KEY_PROXY_SEC,
            SETTING_KEY_OSCA_CERT,
            SETTING_KEY_OSCA_SEC,
            SETTING_KEY_OSCA_EXPIRE,
            SETTING_KEY_OSCA_CN,
        ]
    }

    /// ノード自身のベースURL（my_base_url）が設定されているか検証する。
    /// 未設定の場合は、エラーを返す。
    pub fn validate_my_base_url(&self) -> anyhow::Result<()> {
        let my_base_url = &self.server.my_base_url;
        if my_base_url.is_none()
            || my_base_url
                .as_ref()
                .map(|u| u.trim().is_empty())
                .unwrap_or(true)
        {
            anyhow::bail!("{}", MSG_MY_BASE_URL_FATAL);
        }
        log::info!("[Startup] My Base URL: {}", my_base_url.as_deref().unwrap_or("Not Set"));
        Ok(())
    }
}
fn default_rt_host() -> String {
    "localhost".to_string()
}
fn default_rt_port() -> u16 {
    DEFAULT_RT_PORT
}
fn default_sw_port() -> u16 {
    DEFAULT_SW_PORT
}
fn default_bifrost_port() -> u16 {
    DEFAULT_BIFROST_PORT
}
fn default_zeroclaw_port() -> u16 {
    DEFAULT_ZEROCLAW_PORT
}
fn default_rt_skey() -> String {
    "6JsfNZwZgc4VvDZyvhebvjVz/+J3IkKpvkb++HYc39Y/=".to_string()
}
fn default_rt_crypto_key() -> String {
    "u0=-yJK67Q%zBE)68g1+2326qd)kZysl".to_string()
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
            bifrost_port: default_bifrost_port(),
            zeroclaw_port: default_zeroclaw_port(),
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
    pub correct: Vec<String>,
    pub summarize: Vec<String>,
    #[serde(default = "default_buffer_start")]
    pub buffer_start: Vec<String>, // e.g., ["Option", "KeyB"]
    #[serde(default = "default_buffer_flush")]
    pub buffer_flush: Vec<String>, // e.g., ["Option", "KeyF"]
    #[serde(default = "default_orchestrator_input")]
    pub orchestrator_input: Vec<String>, // e.g., ["Control", "Alt"]
}

fn default_buffer_start() -> Vec<String> {
    vec!["Option".to_string(), "KeyB".to_string()]
}

fn default_buffer_flush() -> Vec<String> {
    vec!["Option".to_string(), "KeyF".to_string()]
}

fn default_orchestrator_input() -> Vec<String> {
    vec!["Control".to_string(), "Alt".to_string()]
}

impl Default for HotkeyConfig {
    fn default() -> Self {
        Self {
            correct: vec!["Option".to_string(), "KeyH".to_string()],
            summarize: vec!["Option".to_string(), "KeyM".to_string()],
            buffer_start: default_buffer_start(),
            buffer_flush: default_buffer_flush(),
            orchestrator_input: default_orchestrator_input(),
        }
    }
}

pub struct ConfigManager {
    pub home_dir: PathBuf,
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
    /// DB 接続プール
    pub db_pools: Arc<RwLock<Option<DbPools>>>,
    /// LMGW (Bifrost) への認証に使用する静的シークレット。
    /// 起動時に UUID で生成し、メモリ上のみに保持する（DBや設定ファイルには保存しない）。
    /// Bifrost プロセスの起動時にも同じ値を環境変数 BIFROST_AUTH_SECRET で注入する。
    lmgw_secret: Arc<RwLock<Option<String>>>,
    /// ZeroClawプロセスをRT LMGWプロキシ経由で認証させるためのJWT
    zeroclaw_jwt_for_rt: Arc<RwLock<Option<String>>>,
}

impl ConfigManager {
    /// 埋め込まれたデフォルトの設定ファイル内容 (settings.json.example)
    const DEFAULT_SETTINGS: &'static str = include_str!("../settings.json.example");

    pub fn is_owner_active(&self) -> bool {
        self.owner_key.read().is_some()
    }

    /// LMGW シークレットを返す。
    /// 起動時に set_lmgw_secret() で設定されていない場合は None を返す。
    pub fn get_lmgw_secret(&self) -> Option<String> {
        self.lmgw_secret.read().clone()
    }

    /// LMGW シークレットを設定する。
    /// main_of_rt.rs の起動プロセスにおいて、Bifrost 起動前に一度だけ呼ぶこと。
    pub fn set_lmgw_secret(&self, secret: String) {
        *self.lmgw_secret.write() = Some(secret);
    }

    /// ZeroClaw用のJWTを取得する。
    pub fn get_zeroclaw_jwt_for_rt(&self) -> Option<String> {
        self.zeroclaw_jwt_for_rt.read().clone()
    }

    /// ZeroClaw用のJWTを設定する。
    pub fn set_zeroclaw_jwt_for_rt(&self, jwt: String) {
        *self.zeroclaw_jwt_for_rt.write() = Some(jwt);
    }

    /// 設定のパスを正規化し、動的なデフォルト値を適用します。
    pub fn normalize_paths(home_dir: &Path, settings: &mut Settings) {
        // [Storage Paths]
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
            }
        }

        // [Model Dir]
        // model_dir は設定ファイルからは読み込まず、常に強制的に ~/.mycute/models を設定する
        let mut s = settings.stt.clone();
        if s.model_dir.is_none() || s.model_dir.as_ref().map(|s| s.is_empty()).unwrap_or(false) {
            let default_models = home_dir
                .join(MYCUTE_MODELS_DIRNAME)
                .to_string_lossy()
                .to_string();
            log::debug!("Setting dynamic model_dir: {}", default_models);
            s.model_dir = Some(default_models);
        }
        settings.stt = s;
    }

    /// [Bootstrap] データベース接続が確立される前の、初期化用の最小限の設定マネージャーを生成します。
    /// この段階では DB 操作は行えません。
    pub fn new_bootstrap(home_override: Option<PathBuf>) -> anyhow::Result<Self> {
        Self::new_with_db(None, home_override)
    }

    /// [Live] データベース接続を伴う、実運用用の設定マネージャーを生成します。
    pub fn new_live(db_pools: DbPools, home_override: Option<PathBuf>) -> anyhow::Result<Self> {
        Self::new_with_db(Some(db_pools), home_override)
    }

    #[deprecated(note = "Use new_bootstrap() or new_live() instead")]
    pub fn new() -> Self {
        Self::new_bootstrap(None).unwrap_or_else(|e| {
            log::error!("[Startup] Critical failure during bootstrap: {}", e);
            Self::new_with_db_fallback()
        })
    }

    /// 完全にデフォルトの値を生成するフォールバック
    fn new_with_db_fallback() -> Self {
        let settings = Settings::new_with_home(None);
        Self::new_with_settings_and_db(settings, None, None).unwrap_or_else(|_| {
            // ここまで来ると致命的だが、何とかして空の構造体を返す
            let home_dir = PathBuf::from(".");
            Self {
                home_dir: home_dir.clone(),
                settings: Arc::new(parking_lot::RwLock::new(Settings::new_with_home(None))),
                owner_key: Arc::new(parking_lot::RwLock::new(None)),
                ca_selection_counter: AtomicUsize::new(0),
                identity_layer_cache: Cache::builder().build(),
                reliable_ca_cache: Arc::new(parking_lot::RwLock::new(None)),
                replaces: Arc::new(parking_lot::RwLock::new(IndexMap::new())),
                replaces_active_ids: Arc::new(parking_lot::RwLock::new(Vec::new())),
                db_pools: Arc::new(parking_lot::RwLock::new(None)),
                lmgw_secret: Arc::new(parking_lot::RwLock::new(None)),
                zeroclaw_jwt_for_rt: Arc::new(parking_lot::RwLock::new(None)),
            }
        })
    }

    pub fn new_with_db(
        db_pools: Option<DbPools>,
        home_override: Option<PathBuf>,
    ) -> anyhow::Result<Self> {
        // [Default Bootstrap Initializer]
        // 初回起動時や DB 接続前（Bootstrap）でも、バリデーションや設定参照が正しく行えるよう、
        // 外部ファイルではなくバイナリに埋め込まれた settings.json.example を初期値としてパースする。
        let settings = serde_json::from_str::<Settings>(Self::DEFAULT_SETTINGS).unwrap_or_else(|e| {
            log::error!("CRITICAL: Failed to parse embedded DEFAULT_SETTINGS (example): {}", e);
            Settings::new_with_home(home_override.clone())
        });
        Self::new_with_settings_and_db(settings, db_pools, home_override)
    }

    pub fn new_with_settings_and_db(
        mut settings: Settings,
        db_pools: Option<DbPools>,
        home_override: Option<PathBuf>,
    ) -> anyhow::Result<Self> {
        let home_dir = get_mycute_home(home_override);

        // [Path Normalization]
        Self::normalize_paths(&home_dir, &mut settings);

        let manager = Self {
            home_dir,
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
            db_pools: Arc::new(RwLock::new(db_pools)),
            lmgw_secret: Arc::new(RwLock::new(None)),
            zeroclaw_jwt_for_rt: Arc::new(RwLock::new(None)),
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
                        anyhow::bail!(
                            "CRITICAL: my_rem corrupted or tampered. Node cannot start. Cause: {}",
                            e
                        );
                    }
                }
            }
        }

        Ok(manager)
    }

    pub async fn get_value_from_db(&self, key: &str) -> anyhow::Result<Option<serde_json::Value>> {
        let pools = self.db_pools.read().clone().context("DB pools not initialized")?;
        let db = pools.get_ro().map_err(|e| anyhow::anyhow!("Failed to get RO connection: {}", e))?;
        self.get_value_from_db_with_conn(db, key).await
    }

    pub async fn get_value_from_db_with_conn(
        &self,
        conn: &impl ConnectionTrait,
        key: &str,
    ) -> anyhow::Result<Option<serde_json::Value>> {
        let model = settings::Entity::find_by_id(key.to_string())
            .one(conn)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to query settings table with conn: {}", e))?;

        Ok(model.map(|m| m.value))
    }

    pub async fn set_value_to_db(&self, key: &str, value: serde_json::Value) -> anyhow::Result<()> {
        let pools = self.db_pools.read().clone().context("DB pools not initialized")?;
        let db = pools.get_rw().map_err(|e| anyhow::anyhow!("Failed to get RW connection: {}", e))?;
        self.set_value_to_db_with_conn(db, key, value).await
    }

    pub async fn set_value_to_db_with_conn(
        &self,
        conn: &impl ConnectionTrait,
        key: &str,
        value: serde_json::Value,
    ) -> anyhow::Result<()> {
        let backend = conn.get_database_backend();
        let query = Query::insert()
            .into_table(Alias::new("settings"))
            .columns([Alias::new("key"), Alias::new("value")])
            .values_panic([key.into(), value.into()])
            .on_conflict(
                OnConflict::column(Alias::new("key"))
                    .update_column(Alias::new("value"))
                    .to_owned()
            )
            .to_owned();

        let stmt = backend.build(&query);
        conn.execute(stmt)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to upsert setting '{}' to DB via direct exec: {}", key, e))?;

        Ok(())
    }

    /// DB から全ての設定を読み込み、Settings 構造体を復元する
    pub async fn load_all_from_db(&self) -> anyhow::Result<Option<Settings>> {
        use sea_orm::EntityTrait;
        let pools = self.db_pools.read().clone().context("DB pools not initialized")?;
        let db = pools.get_ro().map_err(|e| anyhow::anyhow!("Failed to get RO connection: {}", e))?;
        
        // データが存在するかチェック
        let models = settings::Entity::find()
            .all(db)
            .await
            .context("Failed to load all settings from DB")?;

        if models.is_empty() {
            return Ok(None);
        }

        let mut map = serde_json::Map::new();
        for m in models {
            map.insert(m.key, m.value);
        }

        let settings = serde_json::from_value::<Settings>(serde_json::Value::Object(map))
            .map_err(|e| anyhow::anyhow!("Failed to deserialize Settings from DB data: {}", e))?;

        Ok(Some(settings))
    }

    /// DB から全ての設定を読み込み、メモリ上の settings に反映する
    pub async fn load_to_memory_from_db(&self) -> anyhow::Result<()> {
        if let Some(mut settings) = self.load_all_from_db().await? {
            Self::normalize_paths(&self.home_dir, &mut settings);
            let mut current = self.settings.write();
            *current = settings;
            log::debug!("<ConfigManager> Memory settings updated from DB.");
            Ok(())
        } else {
            anyhow::bail!("No settings found in DB")
        }
    }

    /// DB 接続プールを後から注入する
    pub fn inject_db_pools(&self, db_pools: DbPools) {
        let mut guard = self.db_pools.write();
        *guard = Some(db_pools);
    }

    /// Settings 構造体をキー単位の JSON Value に分解する
    pub fn decompose_settings(settings: &Settings) -> anyhow::Result<IndexMap<String, serde_json::Value>> {
        let mut items = IndexMap::new();
        let val = serde_json::to_value(settings)
            .map_err(|e| anyhow::anyhow!("Failed to serialize Settings for decomposition: {}", e))?;
        
        if let serde_json::Value::Object(map) = val {
            for (k, v) in map {
                items.insert(k, v);
            }
        }
        Ok(items)
    }

    /// 複数の設定を一括で DB に保存する (upsert)
    pub async fn upsert_to_db(&self, items: IndexMap<String, serde_json::Value>) -> anyhow::Result<()> {
        let pools = self.db_pools.read().clone().context("DB pools not initialized")?;
        let db = pools.get_rw()?;
        let txn = db.begin().await?;
        for (k, v) in items {
            self.set_value_to_db_with_conn(&txn, &k, v).await?;
        }
        txn.commit().await?;
        Ok(())
    }

    /// `rt_skey` および `rt_crypto_key` がデフォルト値のままであれば、
    /// ランダムなユニーク値を自動生成して上書きする。
    ///
    /// # 動作方針
    /// - これら2つの鍵はシステムが自律的に管理するセキュリティ基盤であり、
    ///   ユーザーが外部から設定すべき値ではない。
    /// - `settings.json.example` には記載していないため、JSON デシリアライズ後は
    ///   必ずコード上の `default_rt_skey()` / `default_rt_crypto_key()` の値になる。
    /// - その状態を検知した場合にのみ自動生成し、DB への保存（呼び出し元が行う）を促す。
    ///
    /// # 重要: `rt_crypto_key` 生成時は `last_rotated_at` を必ず設定すること
    /// - `last_rotated_at` が `None` のまま保存されると、Headless 起動（`--parent-pid` なし）時に
    ///   `rotation_bl::check_and_rotate_keys` が「ローテーション未実施」と誤判定し、
    ///   直後に `rt_crypto_key` を別の値で上書きしてしまう。
    /// - これにより `my_pub`/`my_sec` を旧キーで暗号化したまま新キーでの復号が試みられ、
    ///   アイデンティティが消失する（UI 上で「公開鍵なし」になる）。
    ///
    /// # 戻り値
    /// `true`: 少なくとも一方の鍵を生成・上書きした場合（DB への再保存が必要）
    /// `false`: どちらも既にユニークな値が設定されていた場合（DB 操作不要）
    fn ensure_unique_secret_keys(settings: &mut Settings) -> bool {
        let mut changed = false;

        // rt_skey チェック: デフォルト値と一致する場合は新しい鍵を生成する
        if settings.server.rt_skey == default_rt_skey() {
            let new_key = crypto::generate_random_b64_key_32();
            log::info!(
                "[Startup] rt_skey is at default value. Generating a unique key for this node."
            );
            settings.server.rt_skey = new_key;
            changed = true;
        }

        // rt_crypto_key チェック: デフォルト値と一致する場合は新しい鍵を生成する
        if settings.server.rt_crypto_key == default_rt_crypto_key() {
            let new_key = crypto::generate_random_alphanumeric(32);
            log::info!(
                "[Startup] rt_crypto_key is at default value. Generating a unique key for this node: {}",
                new_key,
            );
            settings.server.rt_crypto_key = new_key;
            // [CRITICAL FIX] `last_rotated_at` を現在時刻で設定する。
            // これを設定しないと、次回の Headless 起動時に `check_and_rotate_keys` が
            // `last_rotated_at = null` を検出し「ローテーション未実施」と判断して
            // `rt_crypto_key` を再生成してしまう。その結果、`my_pub`/`my_sec` が
            // 旧キーで暗号化されたまま復号不能になり、公開鍵が UI 上で消失する。
            settings.server.last_rotated_at = Some(time::naive_to_str(&time::now()));
            changed = true;
        } else {
            log::debug!(
                "[DIAG] ensure_unique_secret_keys: rt_crypto_key is non-default (len={}). last_rotated_at={:?}. NOT regenerating.",
                settings.server.rt_crypto_key.len(),
                settings.server.last_rotated_at,
            );
        }

        changed
    }

    /// DB が空の場合、埋め込まれた DEFAULT_SETTINGS (example) を投入し、メモリを更新する
    pub async fn initialize_settings_in_db(&self) -> anyhow::Result<()> {
        let _pools = self.db_pools.read().clone().context("DB pools not initialized")?;

        // 1. DB に既にデータがあるか確認
        if let Some(existing) = self.load_all_from_db().await? {
            log::debug!("Database already has settings data. Skip seeding.");
            let mut existing = existing;
            Self::normalize_paths(&self.home_dir, &mut existing);

            // デフォルト値のままの秘密鍵が DB に残存している場合は自動生成・再保存する
            if Self::ensure_unique_secret_keys(&mut existing) {
                log::info!("[Startup] Persisting auto-generated secret keys to DB...");
                let items = Self::decompose_settings(&existing)
                    .context("Failed to decompose settings for key update")?;
                self.upsert_to_db(items)
                    .await
                    .context("Failed to persist auto-generated secret keys to DB")?;
                log::info!("[Startup] Auto-generated secret keys persisted successfully.");
            }

            // DIAG: Loaded key trace
            {
                let k = &existing.server.rt_crypto_key;
                let is_def = k == &default_rt_crypto_key();
                log::info!(
                    "[DIAG] init_from_db: key={}{}, last_rotated_at={:?}",
                    &k[..k.len().min(16)],
                    if is_def { " (DEFAULT!)" } else { "" },
                    existing.server.last_rotated_at,
                );
            }

            // メモリ上の設定を DB の内容（鍵更新済み）で更新
            let mut settings = self.settings.write();
            *settings = existing;
            return Ok(());
        }

        // 2. DB にデータがない場合、DEFAULT_SETTINGS (埋め込み example) から投入を実行
        log::info!("No settings found in DB. Initializing DB with embedded default settings...");

        // Settings::new_with_home() でも良いが、ユーザーの意図通り settings.json.example の内容を尊重する。
        // DEFAULT_SETTINGS をデシリアライズして正規化した後、再度保存する。
        let mut settings = serde_json::from_str::<Settings>(Self::DEFAULT_SETTINGS)
            .context("Failed to parse embedded DEFAULT_SETTINGS (example)")?;

        Self::normalize_paths(&self.home_dir, &mut settings);

        // DB 初回投入前に秘密鍵を自動生成する（デフォルト値がそのまま保存されることを防ぐ）
        Self::ensure_unique_secret_keys(&mut settings);

        // DIAG: Seeded key trace
        {
            let k = &settings.server.rt_crypto_key;
            let is_def = k == &default_rt_crypto_key();
            log::info!(
                "[DIAG] init_seed: key={}{}, last_rotated_at={:?}",
                &k[..k.len().min(16)],
                if is_def { " (DEFAULT!)" } else { "" },
                settings.server.last_rotated_at,
            );
        }

        let items = Self::decompose_settings(&settings).context("Failed to decompose settings")?;
        self.upsert_to_db(items).await.context("Failed to initialize settings in DB")?;

        log::info!("Initial settings seeded to DB successfully.");

        // メモリ上の設定を更新
        let mut current = self.settings.write();
        *current = settings;

        Ok(())
    }

    /// DB 接続が確立された後の最終的な初期化 (非同期)
    pub async fn ensure_initialized_with_db(&self) -> anyhow::Result<()> {
        self.initialize_settings_in_db().await?;
        Ok(())
    }

    /// 現在のオンメモリ設定を DB に保存する (非同期)
    pub async fn save_db(&self) -> anyhow::Result<()> {
        let pools = self.db_pools.read().clone().context("CRITICAL: Attempted to save_db, but DB pools are not initialized. This ConfigManager is read-only.")?;
        let conn = pools.get_rw().map_err(|e| anyhow::anyhow!("Failed to get RW connection for save_db: {}", e))?;
        self.save_db_with_conn(conn).await
    }

    /// 指定された接続（トランザクション等）を使用して、現在のオンメモリ設定を DB に保存する
    pub async fn save_db_with_conn(&self, conn: &impl sea_orm::ConnectionTrait) -> anyhow::Result<()> {
        let settings = self.settings.read().clone();
        let items = Self::decompose_settings(&settings)?;
        for (k, v) in items {
            self.set_value_to_db_with_conn(conn, &k, v).await?;
        }
        Ok(())
    }

    /// 証明書の更新や再登録のために特権昇格が必要かどうかを判定する
    pub fn needs_elevation_for_cert(&self) -> bool {
        let (osca_cert, osca_expire, osca_cn) = {
            let s = self.settings.read();
            (s.osca_certificate.clone(), s.osca_expire.clone(), s.osca_cn.clone())
        };
        
        // 1. 証明書の存在チェック
        let osca_cert = match &osca_cert {
            Some(c) => c,
            None => {
                log::info!("<ElevationCheck> osca_certificate not found in settings. Elevation REQUIRED.");
                return true;
            }
        };
        let osca_expire = match &osca_expire {
            Some(e) => e,
            None => {
                log::info!("<ElevationCheck> osca_expire not found in settings. Elevation REQUIRED.");
                return true;
            }
        };

        // 2. 期限チェック (7日間のバッファ)
        if is_cert_expired_with_buffer(osca_expire, 7) {
            log::info!("<ElevationCheck> Certificate is expired or expiring soon. Elevation REQUIRED.");
            return true;
        }

        // 3. OS の信頼状態チェック
        if let Ok(c_pem_bytes) = BASE64_STANDARD.decode(osca_cert) {
            if let Ok(c_pem) = String::from_utf8(c_pem_bytes) {
                // PEM から Common Name (CN) を抽出
                // 本来は X509 パースが必要だが、外部クレート追加を避けるため、
                // 簡易的な文字列検索（Subject: や CN = ）で試みるか、
                // setup.rs で生成される形式が既知であることを利用する。
                // ログによると実際には "fastcert ..." という形式。
                // 安全のため、PEM の中身から CN を探す簡易実装を行う。
                // 1. 保存されている CN があれば優先的に使用
                let cn = if let Some(saved_cn) = osca_cn {
                    log::debug!("<ElevationCheck> Using saved OSCA CN: {}", saved_cn);
                    Some(saved_cn)
                } else {
                    // 2. なければ PEM から抽出を試みる (フォールバック)
                    log::debug!("<ElevationCheck> Saved CN not found. Attempting extraction from PEM...");
                    Self::extract_cn_from_pem(&c_pem)
                };

                if let Some(cn) = cn {
                    let trusted = is_cert_trusted_by_os(&cn);
                    log::info!("<ElevationCheck> Checking OS trust for CN '{}': trusted={}", &cn, trusted);
                    if !trusted {
                        log::info!("<ElevationCheck> OS trust check failed. Elevation REQUIRED.");
                        return true;
                    }
                } else {
                    // CN が抽出できない場合は安全のため昇格を要求
                    log::info!("<ElevationCheck> Could not extract CN from PEM. Elevation REQUIRED.");
                    return true;
                }
            }
        }

        log::info!("<ElevationCheck> All checks passed. Elevation NOT required.");
        false
    }

    /// PEM 文字列から Common Name (CN) を簡易的に抽出する。
    /// 本来は ASN.1 パースが必要だが、MyCute が生成する形式に対して
    /// 最小限の文字列処理で対応する。
    fn extract_cn_from_pem(pem: &str) -> Option<String> {
        // Rust の X509 クレート等を使わずに抽出するため、
        // 「Subject: 」または「CN = 」などのパターンを探す。
        // ※ OS の `security` や `certutil` が認識する形式に合わせる必要がある。
        // MyCute (fastcert) の生成する PEM には Subject 行が含まれる。
        for line in pem.lines() {
            let line = line.trim();
            if line.contains("Subject:") || line.contains("CN=") || line.contains("CN =") {
                if let Some(idx) = line.find("CN=") {
                    let val = &line[idx + 3..];
                    return Some(val.split(',').next()?.trim().to_string());
                }
                if let Some(idx) = line.find("CN =") {
                    let val = &line[idx + 4..];
                    return Some(val.split(',').next()?.trim().to_string());
                }
            }
        }
        
        // 文字列で見つからない場合、暫定的なフォールバックとして
        // ログから判明している "fastcert" プレフィックスを試みる。
        // 本来は生成時に CN を別フィールドで保存しておくのが理想。
        None
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

        let denoiser_path = settings.stt.get_denoiser_path().map_err(|e| e.to_string())?;
        if !Path::new(&denoiser_path).exists() {
            return Err(format!("Denoiser model file not found: {}", denoiser_path));
        }

        let vad_path = settings.stt.get_vad_path().map_err(|e| e.to_string())?;
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
    /// オーナーモードの有無に関わらず、このノード自身のアイデンティティは常に取得可能。
    /// オーナーとしての操作が必要な場合は `owner_key` フィールドを直接参照すること。
    pub fn get_node_keypair(&self) -> Result<Ed448KeyValuePair, ApiError> {
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
        self.save_db()
            .await
            .map_err(|e| ApiError::new_system(ST_INTERNAL_SERVER_ERROR, ERR_DB, e.to_string()))?;
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
            // llms は LMGW 移行に伴い廃止済み
            stt: SttSettings::default(),
            server: ServerSettings::default(),
            storage: StorageSettings::default(),
            cuber: CuberSettings::default(),
            window_position: WindowPositionConfig::default(),
            proxy_certificate: None,
            proxy_private_key: None,
            osca_certificate: None,
            osca_private_key: None,
            osca_expire: None,
            osca_cn: None,
            my_pub: None,
            my_sec: None,
            my_rem: None,
            my_cat: None,
            my_lics: Vec::new(),
        }
    }

    pub fn new_with_home(home_override: Option<PathBuf>) -> Self {
        let home = get_mycute_home(home_override);
        let mut s = Self::new_default();
        s.storage = StorageSettings::new_with_home(&home);
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDateTime;
    use crate::migration::{Migrator, MigratorTrait};
    use crate::constants::SQLITE_DEFAULT_FILENAME;
    use crate::utils::rotation_bl;
    use sea_orm::Database;
    use std::sync::atomic::AtomicUsize;
    use std::sync::Arc;
    use tempfile::{tempdir, TempDir};

    // ================================================================
    // protected_settings_keys の検証
    // ================================================================

    /// リセット操作（do_reset_db）で rt_crypto_key を含む server 行が
    /// 誤って削除されないよう、"server" が protected リストに含まれていることを確認する。
    #[test]
    fn test_protected_settings_keys_includes_server() {
        let keys = Settings::protected_settings_keys();
        assert!(
            keys.contains(&SETTING_KEY_SERVER),
            "SETTING_KEY_SERVER ({}) must be in protected_settings_keys, \
             or reset_application will delete rt_crypto_key/last_rotated_at from DB",
            SETTING_KEY_SERVER,
        );
    }

    // ================================================================
    // ensure_unique_secret_keys の単体テスト
    // ================================================================

    /// `rt_crypto_key` がデフォルト値の場合、新しい鍵が生成され `last_rotated_at` が設定される
    #[test]
    fn test_ensure_unique_replaces_default_crypto_key() {
        let mut settings = Settings::new_default();
        assert_eq!(settings.server.rt_crypto_key, default_rt_crypto_key());
        assert!(settings.server.last_rotated_at.is_none());

        let changed = ConfigManager::ensure_unique_secret_keys(&mut settings);

        assert!(changed);
        assert_ne!(settings.server.rt_crypto_key, default_rt_crypto_key());
        assert_eq!(settings.server.rt_crypto_key.len(), 32);
        // [CRITICAL FIX] last_rotated_at が設定されないと headless 起動時に key rotation が走る
        assert!(
            settings.server.last_rotated_at.is_some(),
            "last_rotated_at MUST be set when regenerating rt_crypto_key, or check_and_rotate_keys will regenerate it again on next headless boot"
        );
    }

    /// `rt_crypto_key` と `rt_skey` が既にユニークな値の場合、変更されない
    #[test]
    fn test_ensure_unique_preserves_non_default_crypto_key() {
        let mut settings = Settings::new_default();
        let original_key = "MyUniqueKey_1234567890abcdefghij".to_string();
        settings.server.rt_crypto_key = original_key.clone();
        settings.server.rt_skey = "NonDefaultRtSkey_111111111111111".to_string();
        settings.server.last_rotated_at = Some("2026-01-01T00:00:00".to_string());

        let changed = ConfigManager::ensure_unique_secret_keys(&mut settings);

        assert!(!changed);
        assert_eq!(settings.server.rt_crypto_key, original_key);
        assert_eq!(
            settings.server.last_rotated_at,
            Some("2026-01-01T00:00:00".to_string())
        );
    }

    /// `rt_skey` のみデフォルトの場合、`rt_crypto_key` は変更されない
    #[test]
    fn test_ensure_unique_replaces_only_rt_skey() {
        let mut settings = Settings::new_default();
        settings.server.rt_crypto_key = "ExistingCryptoKeyNotDefault!!".to_string();

        let changed = ConfigManager::ensure_unique_secret_keys(&mut settings);

        assert!(changed); // rt_skey が変更された
        assert_ne!(settings.server.rt_skey, default_rt_skey());
        // rt_crypto_key はそのまま
        assert_eq!(
            settings.server.rt_crypto_key,
            "ExistingCryptoKeyNotDefault!!"
        );
    }

    /// 両方の鍵が既にユニークな場合、何も変更されない
    #[test]
    fn test_ensure_unique_both_keys_already_unique() {
        let mut settings = Settings::new_default();
        settings.server.rt_skey = "NonDefaultRtSkey_111111111111111".to_string();
        settings.server.rt_crypto_key = "NonDefaultCryptoKey_22222222222222".to_string();
        settings.server.last_rotated_at = Some("2026-03-15T00:00:00".to_string());

        let changed = ConfigManager::ensure_unique_secret_keys(&mut settings);

        assert!(!changed);
    }

    /// `rt_crypto_key` を再生成した際の `last_rotated_at` の書式が Naive 形式である
    /// （RFC3339 と Naive の2形式のパーサーのうち Naive 側でパースできること）
    #[test]
    fn test_ensure_unique_last_rotated_at_format_is_naive() {
        let mut settings = Settings::new_default();
        ConfigManager::ensure_unique_secret_keys(&mut settings);

        let last_rotated = settings.server.last_rotated_at.unwrap();
        // Naive 書式 (time::naive_to_str) でパースできることを確認
        let parsed = NaiveDateTime::parse_from_str(&last_rotated, "%Y-%m-%dT%H:%M:%S");
        assert!(
            parsed.is_ok(),
            "last_rotated_at format must be NaiveDateTime parseable. Got: {}",
            last_rotated
        );
    }

    // ================================================================
    // ラウンドトリップと DB 初期化の結合テスト
    // ================================================================

    /// ConfigManager をテンポラリ環境で構築するヘルパー
    async fn setup_config_manager() -> (ConfigManager, TempDir) {
        let dir = tempdir().unwrap();
        let home_dir = dir.path().to_path_buf();

        // DB ディレクトリを作成
        let db_dir = home_dir.join(DB_DEFAULT_DIRNAME);
        std::fs::create_dir_all(&db_dir).unwrap();

        // SQLite DB を作成
        let db_path = db_dir.join(SQLITE_DEFAULT_FILENAME);
        let url = format!("sqlite://{}?mode=rwc", db_path.to_string_lossy());
        let conn = Database::connect(&url).await.unwrap();

        // Migration 実行
        Migrator::up(&conn, None).await.unwrap();

        let pools = DbPools {
            rw: conn.clone(),
            ro: vec![],
            ro_index: AtomicUsize::new(0),
        };

        let settings = Settings::new_with_home(Some(home_dir.clone()));
        let cm = ConfigManager::new_with_settings_and_db(settings, Some(pools), Some(home_dir))
            .unwrap();

        (cm, dir)
    }

    /// decompose_settings → upsert_to_db → load_all_from_db のラウンドトリップで
    /// rt_crypto_key と last_rotated_at が保存される
    #[tokio::test]
    async fn test_roundtrip_preserves_crypto_key() {
        let (cm, _dir) = setup_config_manager().await;

        // 特定の鍵を設定して保存
        {
            let mut s = cm.settings.write();
            s.server.rt_crypto_key = "RoundTripKey_Test_9876543210".to_string();
            s.server.last_rotated_at = Some("2026-06-01T00:00:00".to_string());
        }
        let items = ConfigManager::decompose_settings(&cm.settings.read()).unwrap();
        cm.upsert_to_db(items).await.unwrap();

        // DB から読み直し
        let loaded = cm.load_all_from_db().await.unwrap().unwrap();

        assert_eq!(
            loaded.server.rt_crypto_key, "RoundTripKey_Test_9876543210",
            "rt_crypto_key must survive DB round-trip"
        );
        assert_eq!(
            loaded.server.last_rotated_at,
            Some("2026-06-01T00:00:00".to_string()),
            "last_rotated_at must survive DB round-trip"
        );
    }

    /// 空DBからの初回 initialize_settings_in_db 呼び出しで、
    /// rt_crypto_key が生成され last_rotated_at が設定される
    #[tokio::test]
    async fn test_initialize_generates_key_on_first_call() {
        let (cm, _dir) = setup_config_manager().await;

        // DB は空（migration 直後で settings テーブルにデータなし）
        cm.ensure_initialized_with_db().await.unwrap();

        let s = cm.settings.read();
        assert_ne!(
            s.server.rt_crypto_key,
            default_rt_crypto_key(),
            "rt_crypto_key must not be default after first init"
        );
        assert!(
            s.server.last_rotated_at.is_some(),
            "last_rotated_at must be set after first init"
        );
        drop(s);
    }

    /// 2回目の initialize_settings_in_db 呼び出しでは
    /// 既存の rt_crypto_key が維持される
    #[tokio::test]
    async fn test_initialize_preserves_key_on_second_call() {
        let (cm, _dir) = setup_config_manager().await;

        // 1回目: 空DB → 鍵生成
        cm.ensure_initialized_with_db().await.unwrap();
        let first_key = {
            let s = cm.settings.read();
            s.server.rt_crypto_key.clone()
        };
        let first_last_rotated = {
            let s = cm.settings.read();
            s.server.last_rotated_at.clone()
        };

        // 2回目: 既存DB → 鍵維持
        cm.ensure_initialized_with_db().await.unwrap();
        let second_key = {
            let s = cm.settings.read();
            s.server.rt_crypto_key.clone()
        };
        let second_last_rotated = {
            let s = cm.settings.read();
            s.server.last_rotated_at.clone()
        };

        assert_eq!(
            first_key, second_key,
            "rt_crypto_key MUST be preserved across repeated initialize calls"
        );
        assert_eq!(
            first_last_rotated, second_last_rotated,
            "last_rotated_at MUST be preserved across repeated initialize calls"
        );
    }

    /// DB にデフォルト鍵が保存されていた場合（旧バージョンなどからの移行）、
    /// initialize_settings_in_db が新しい鍵を生成して上書きする
    #[tokio::test]
    async fn test_initialize_replaces_default_key_in_db() {
        let (cm, _dir) = setup_config_manager().await;

        // DB にデフォルト鍵を直接書き込む（旧バージョンからの移行シミュレート）
        {
            let mut s = cm.settings.write();
            s.server.rt_crypto_key = default_rt_crypto_key();
            s.server.last_rotated_at = None;
        }
        let items = ConfigManager::decompose_settings(&cm.settings.read()).unwrap();
        cm.upsert_to_db(items).await.unwrap();

        // initialize: デフォルト鍵を検出して新しい鍵を生成する
        cm.ensure_initialized_with_db().await.unwrap();

        let s = cm.settings.read();
        assert_ne!(
            s.server.rt_crypto_key,
            default_rt_crypto_key(),
            "Default rt_crypto_key in DB must be replaced with a unique key"
        );
        assert!(
            s.server.last_rotated_at.is_some(),
            "last_rotated_at must be set when replacing default key in DB"
        );
        drop(s);
    }

    // ================================================================
    // Serde デフォルト注入とキーローテーションスキップの検証
    // ================================================================

    /// ServerSettings JSON から `rt_crypto_key` フィールドが欠落している場合、
    /// serde が `#[serde(default = "default_rt_crypto_key")]` により
    /// デフォルト鍵を注入することを確認する。
    #[test]
    fn test_serde_fills_default_when_crypto_key_missing_from_server_json() {
        // server JSON から rt_crypto_key を意図的に除外
        let server_json = serde_json::json!({
            "rt_proto": "http",
            "rt_host": "127.0.0.1",
            "rt_port": 3910,
            "rt_skey": "some_unique_skey_12345",
            "rt_crypto_key_rotation_days": 90,
        });

        let server: ServerSettings = serde_json::from_value(server_json).unwrap();

        // serde(default) によりデフォルト鍵が注入されている
        assert_eq!(
            server.rt_crypto_key,
            default_rt_crypto_key(),
            "rt_crypto_key must be filled by serde(default) when missing from JSON"
        );
        assert!(
            server.last_rotated_at.is_none(),
            "last_rotated_at must be None when missing from JSON"
        );

        // この状態で ensure_unique_secret_keys が検出可能であること
        let mut settings = Settings::new_default();
        settings.server.rt_crypto_key = server.rt_crypto_key; // = default_rt_crypto_key()
        settings.server.last_rotated_at = server.last_rotated_at; // = None
        settings.server.rt_skey = "some_unique_skey_12345".to_string(); // 既にユニーク

        let changed = ConfigManager::ensure_unique_secret_keys(&mut settings);
        assert!(
            changed,
            "ensure_unique_secret_keys MUST detect serde-injected default rt_crypto_key"
        );
        assert_ne!(
            settings.server.rt_crypto_key,
            default_rt_crypto_key(),
            "rt_crypto_key must be replaced with a unique key"
        );
        assert!(
            settings.server.last_rotated_at.is_some(),
            "last_rotated_at MUST be set when replacing serde-injected default key"
        );
    }

    /// 修正の核心: `initialize_settings_in_db` (ensure_unique_secret_keys を含む) で
    /// デフォルト鍵が置き換えられた後、`check_and_rotate_keys` が
    /// 新たなキーローテーションを実行しないことを確認する。
    ///
    /// 旧バグ: ensure_unique_secret_keys が last_rotated_at を設定しなかったため、
    /// check_and_rotate_keys が「未ローテーション」と判断し鍵を再生成していた。
    #[tokio::test]
    async fn test_default_key_in_db_replaced_then_no_rerotation() {
        let (cm, _dir) = setup_config_manager().await;

        // DB にデフォルト鍵を書き込む（旧バージョンからの移行やserde注入をシミュレート）
        {
            let mut s = cm.settings.write();
            s.server.rt_crypto_key = default_rt_crypto_key();
            s.server.last_rotated_at = None;
            s.server.rt_crypto_key_rotation_days = 90;
        }
        let items = ConfigManager::decompose_settings(&cm.settings.read()).unwrap();
        cm.upsert_to_db(items).await.unwrap();

        // initialize: デフォルト鍵を検出 → ユニーク鍵に置き換え + last_rotated_at 設定
        cm.ensure_initialized_with_db().await.unwrap();
        let replaced_key = cm.settings.read().server.rt_crypto_key.clone();
        assert_ne!(replaced_key, default_rt_crypto_key());

        // check_and_rotate_keys を実行 → last_rotated_at が設定されているためスキップされる
        let db = {
            let pools = cm.db_pools.read();
            pools.as_ref().unwrap().rw.clone()
        };
        let cm_arc = Arc::new(cm);
        rotation_bl::check_and_rotate_keys(cm_arc.clone(), &db)
            .await
            .unwrap();

        let final_key = cm_arc.settings.read().server.rt_crypto_key.clone();
        assert_eq!(
            replaced_key, final_key,
            "check_and_rotate_keys MUST NOT change the key when last_rotated_at is set \
             (ensure_unique_secret_keys must have set it)"
        );
    }

    /// ConfigManager の再生成（2ブート系列）をシミュレートし、
    /// rt_crypto_key が維持されることを確認する。
    #[tokio::test]
    async fn test_initialize_preserves_key_across_config_manager_restart() {
        let dir = tempdir().unwrap();
        let home_dir = dir.path().to_path_buf();
        let db_dir = home_dir.join(DB_DEFAULT_DIRNAME);
        std::fs::create_dir_all(&db_dir).unwrap();
        let db_path = db_dir.join(SQLITE_DEFAULT_FILENAME);
        let url = format!("sqlite://{}?mode=rwc", db_path.to_string_lossy());

        // === BOOT 1: 初回 ConfigManager → initialize ===
        let conn1 = Database::connect(&url).await.unwrap();
        Migrator::up(&conn1, None).await.unwrap();
        let pools1 = DbPools {
            rw: conn1,
            ro: vec![],
            ro_index: AtomicUsize::new(0),
        };
        let settings1 = Settings::new_with_home(Some(home_dir.clone()));
        let cm1 = ConfigManager::new_with_settings_and_db(
            settings1,
            Some(pools1),
            Some(home_dir.clone()),
        )
        .unwrap();
        cm1.ensure_initialized_with_db().await.unwrap();
        let boot1_key = cm1.settings.read().server.rt_crypto_key.clone();
        let boot1_last_rotated = cm1.settings.read().server.last_rotated_at.clone();
        assert_ne!(boot1_key, default_rt_crypto_key());
        assert!(
            boot1_last_rotated.is_some(),
            "last_rotated_at must be set after boot 1"
        );
        // cm1 をドロップ（コネクション切断 + メモリ解放）
        drop(cm1);

        // === BOOT 2: 新規 ConfigManager → 同じ DB で initialize ===
        let conn2 = Database::connect(&url).await.unwrap();
        let pools2 = DbPools {
            rw: conn2,
            ro: vec![],
            ro_index: AtomicUsize::new(0),
        };
        let settings2 = Settings::new_with_home(Some(home_dir.clone()));
        let cm2 = ConfigManager::new_with_settings_and_db(
            settings2,
            Some(pools2),
            Some(home_dir),
        )
        .unwrap();
        cm2.ensure_initialized_with_db().await.unwrap();
        let boot2_key = cm2.settings.read().server.rt_crypto_key.clone();
        let boot2_last_rotated = cm2.settings.read().server.last_rotated_at.clone();

        assert_eq!(
            boot1_key, boot2_key,
            "rt_crypto_key MUST be preserved across two separate boot sequences \
             (ConfigManager restart with same DB)"
        );
        assert_eq!(
            boot1_last_rotated, boot2_last_rotated,
            "last_rotated_at MUST be preserved across two separate boot sequences"
        );
    }
}
