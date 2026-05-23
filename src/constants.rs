//! プロジェクト共通の定数定義
//!
//! # 唯一の真実 (Single Source of Truth)
//! このファイルで定義された `pub const` な定数は、ビルド時に自動的に TypeScript/JavaScript 環境へ
//! 同期されます。これにより、言語を跨いだ定数の二重管理と不整合を防止します。
//!
//! # 同期の流れ
//! 1. `make build-sdk-ts` 実行時、`scripts/gen-ts-constants.sh` が本ファイルを解析します。
//! 2. `sdk-ts/src/generated_constants.ts` が自動生成されます。
//! 3. TypeScript SDK は生成されたファイルをインポートして利用します。
//! 4. 最終的に `include_bytes!` によって SDK (JS) が Rust バイナリに取り込まれます。
//!
//! 各値の変更は本ファイルのみで行ってください。
use axum::http::StatusCode;
use std::time::Duration;

pub const WINDOW_WIDTH: f64 = 390.0;
pub const WINDOW_HEIGHT: f64 = 750.0;

/// MYCUTE OS のバージョン番号。
pub const MYCUTE_VERSION: &str = "v0.24.153";

/// アプリケーション名（ロックファイル等で使用）
/// ビルド時に APP_SLUG 環境変数が注入された場合はその値を使用し、なければ "mycute" をデフォルトとする。
pub const APP_NAME: &str = match option_env!("APP_SLUG") {
    Some(v) => v,
    None => "mycute",
};

/// アプリケーション表示名（証明書のCNやUI表示で使用）
pub const APP_DISPLAY_NAME: &str = match option_env!("APP_DISPLAY_NAME") {
    Some(v) => v,
    None => "MYCUTE",
};

/// サーバーアプリケーション名
pub const APP_SERVER_NAME: &str = match option_env!("APP_SERVER_NAME") {
    Some(v) => v,
    None => "mycute-server",
};

pub const ENGINE_OPENAI: &str = "openai";
pub const ENGINE_OS: &str = "os";

/// 設定値におけるダミー判定用文字列
pub const DUMMY_STRING: &str = "dummy";

/// LMGW 経由で使用するデフォルトモデル名
pub const DEFAULT_LLM_MODEL: &str = "openai/gpt-4.1-nano";

/// 擬態（パススルー）モード時に「処理している感」を出すための装飾的ウェイト（ミリ秒）
pub const LLM_MIMICRY_DELAY_MS: u64 = 300;

/// サーバー/マイグレーション用のシングルトンロックファイル名
/// main_of_rt（サーバー起動）および main_of_am（マイグレーション）で使用。
/// 複数サーバーの同時起動や、サーバー稼働中のマイグレーション実行を防止する。
// pub const LOCK_FILE_SERVER: &str = "mycute.lock";

/// GUIアプリケーション用のシングルトンロックファイル名
/// main_of_cl（GUIロール）で使用。2つ目のGUIウィンドウの起動を防止する。
/// GUIプロセスはサーバーを子プロセスとして生成するため、サーバー用ロックとは分離する。
// pub const LOCK_FILE_APP: &str = "mycute-app.lock";

/// 音声認識のタイムアウト時間（秒）。
/// Windows/Mac両方でこの秒数の沈黙が続くとセッションを終了し、結果をコミットします。
pub const SPEECH_TIMEOUT_SEC: f64 = 30.0;

/// Windows専用: 文末句読点を自動付与するための無音タイムアウト時間（ミリ秒）
pub const STT_TIMEOUT_PUNCTUATION_MS: u64 = 500;

/// 削除キー送信後のクールダウン時間（ミリ秒）: Mac用
/// 削除後のOS/アプリ側の画面更新待ち（セトリング時間）のベース値です。
pub const DELETION_COOLDOWN_MS_MAC: u64 = 30;

/// 1文字削除あたりの追加待機時間（ミリ秒）: Mac用
pub const DELETION_WEIGHT_MS_MAC: u64 = 5;

/// 削除キー送信後のクールダウン時間（ミリ秒）: Windows用
pub const DELETION_COOLDOWN_MS_WIN: u64 = 30;

/// 1文字削除あたりの追加待機時間（ミリ秒）: Windows用
pub const DELETION_WEIGHT_MS_WIN: u64 = 5;

/// キー押下（Down）および解放（Up）の後の待機時間（ミリ秒）: Mac用
/// 物理的な打鍵挙動を模倣し、OS/アプリが入力を取りこぼさないようにします。
pub const KEY_DELAY_MS_MAC: u64 = 1;

/// キー押下（Down）および解放（Up）の後の待機時間（ミリ秒）: Windows用
pub const KEY_DELAY_MS_WIN: u64 = 5;

/// ダブルタップ（2回連続押下）判定の最小間隔（ミリ秒）。
/// 短すぎるとチャタリングとして無視します。
pub const HOTKEY_DOUBLE_TAP_MIN_MS: u64 = 10;

/// ダブルタップ（2回連続押下）判定の最大間隔（ミリ秒）。
/// この時間内に2回目の押下が発生した場合にアクションを発火させます。
pub const HOTKEY_DOUBLE_TAP_MAX_MS: u64 = 500;

/// 最終補正の実行を保留し、沈黙を待機する猶予時間（ミリ秒）。
pub const POST_CORRECTION_SILENCE_WAIT_MS: u64 = 850;

/// STT デコレーションアニメーション（… ?）の更新間隔（ミリ秒）。
pub const STT_DECORATION_INTERVAL_MS: u64 = 180;

/// OpenAIモードにおいて、無線ヘッドセットの立ち上がりを待つための開始音遅延時間（ミリ秒）。
pub const OPENAI_READY_DELAY_MS: u64 = 250;

/// SDKのファイル名。静的Webサーバーでの配信と、Auto-injectionでの参照に使用されます。
pub const MYCUTE_SDK_FILENAME: &str = "mycute_sdk.js";

/// Service Workerのファイル名。
pub const MYCUTE_SW_FILENAME: &str = "mycute_sw.js";

/// SDKやService Workerが所属する本来のドメイン。
pub const MYCUTE_ORIGIN: &str = "https://mycute.app";

/// ドメインサフィックス方式の識別子（Phase 8）。
/// HTTP/HTTPS プロトコルを維持したまま、このサフィックスを持つリクエストをプロキシ対象とします。
pub const MYCUTE_PROXY_SUFFIX: &str = ".mc.shyme.net";

/// [Deprecated] カスタムプロトコルスキーム（HTTP相当）。
/// Phase 8 以降、.mc.shyme.net サフィックス方式へ移行するため非推奨。
#[deprecated(
    note = "Use standard https scheme with MYCUTE_PROXY_SUFFIX (.mc.shyme.net) instead (Phase 8 migration)"
)]
pub const MYCUTE_SCHEME_HTTP: &str = "mycute";

/// [Deprecated] カスタムプロトコルスキーム（HTTPS相当）。
/// Phase 8 以降、.mc.shyme.net サフィックス方式へ移行するため非推奨。
#[deprecated(
    note = "Use standard https scheme with MYCUTE_PROXY_SUFFIX (.mc.shyme.net) instead (Phase 8 migration)"
)]
pub const MYCUTE_SCHEME_HTTPS: &str = "mycutes";

/// WebSocketプロキシ用のエンドポイントパス。
pub const MYCUTE_WS_PROXY_PATH: &str = "/mycute_proxy_ws";

/// SSE (EventSource) プロキシ用のエンドポイントパス。
pub const MYCUTE_SSE_PROXY_PATH: &str = "/mycute_proxy_sse";

/// SSE 内部チャンネルのキャパシティ（最大保留イベント数）
pub const SSE_CHANNEL_CAPACITY: usize = 250;

/// SSE の明示的なハートビート送信間隔
pub const SSE_HEARTBEAT_INTERVAL: Duration = Duration::from_secs(15);
/// SSE のクライアント側での無通信タイムアウト（これが経過すると再接続）
pub const SSE_TIMEOUT_DURATION: Duration = Duration::from_secs(30);

/// 開発・デバッグ中に不確定要素を排除し、PACファイル等での指定を容易にします。
pub const MYCUTE_PROXY_PORT: u16 = 58300;

/// RT (Runtime Server) のデフォルトポート
pub const DEFAULT_RT_PORT: u16 = 3910;

/// SW (Service Worker / Proxy Helper) のデフォルトポート
pub const DEFAULT_SW_PORT: u16 = 3911;

/// Bifrost (LLM Proxy) のデフォルトポート
pub const DEFAULT_BIFROST_PORT: u16 = 3912;

/// ZeroClaw (Agent Gateway) のデフォルトポート
pub const DEFAULT_ZEROCLAW_PORT: u16 = 3913;

// --- ZeroClaw JWT Settings ---
pub const ZEROCLAW_JWT_AID: u32 = 10001;
pub const ZEROCLAW_JWT_VID: u32 = 10002;
pub const ZEROCLAW_JWT_UID: u32 = 10003;
pub const ZEROCLAW_JWT_EMAIL: &str = "zeroclaw@mycute.internal";
pub const ZEROCLAW_JWT_EXPIRE_HOURS: u32 = 26280; // 3 years

// --- LmgwClient JWT Settings (CL モードが LMGW へアクセスするための内部 JWT) ---
/// CL モードが LMGW (RT プロキシ) にアクセスするための内部的なシステム JWT で使用するダミー識別子。
/// ZeroClaw と区別するため別の定数セットとして定義する。
pub const LMGW_CLIENT_JWT_AID: u32 = 10004;
pub const LMGW_CLIENT_JWT_VID: u32 = 10005;
pub const LMGW_CLIENT_JWT_UID: u32 = 10006;
pub const LMGW_CLIENT_JWT_EMAIL: &str = "lmgw-client@mycute.internal";
pub const LMGW_CLIENT_JWT_EXPIRE_HOURS: u32 = 26280; // 3 years


/// Windows プロセス生成フラグ: コンソールウィンドウを表示しない (CREATE_NO_WINDOW)
pub const WIN_CREATE_NO_WINDOW: u32 = 0x08000000;

/// デフォルトのサイン用シークレットキー
pub const DEFAULT_SKEY: &str = "6JsfNZwZgc4VvDZyvhebvjVz/+J3IkKpvkb++HYc39Y/=";
/// デフォルトの暗号化キー
pub const DEFAULT_CRYPTO_KEY: &str = "kS9yzX2!vB5*mN8@qW0&eP3_rY6*tU9!";

// --- Proxy Leak Diagnostics (プロキシ漏洩検知) ---

/// システムイベント: プロキシ漏れ検知
pub const EVENT_PROXY_LEAK: &str = "mycute://kernel/proxy-leak";

/// CSP違反レポート受信パス
pub const PATH_PROXY_LEAK_CSP: &str = "/v1/mycute_proxy_leak/csp";

/// SW検知レポート受信パス
pub const PATH_PROXY_LEAK_SW: &str = "/v1/mycute_proxy_leak/sw";

/// プロキシ経由であることを示すカスタムヘッダー
pub const HEADER_X_IS_MYCUTE: &str = "X-IS-MYCUTE";

/// CSP Report-Only ヘッダー名
pub const HEADER_CSP_REPORT_ONLY: &str = "Content-Security-Policy-Report-Only";

/// プロトコル(スキーム)伝達用カスタムヘッダー
/// SDK/SWがプロキシへ元のスキーム(http/https)を伝えるために使用する
pub const HEADER_X_MYCUTE_SCHEME: &str = "X-Mycute-Origin-Scheme";

// --- Tauri Event Names ---
pub const EVENT_STT_PARTIAL: &str = "stt-partial";
pub const EVENT_STT_FINAL: &str = "stt-final";
pub const EVENT_STT_UPDATE: &str = "stt-update";
pub const EVENT_STT_COMMIT: &str = "stt-commit";
pub const EVENT_APP_STATUS: &str = "app-status";
pub const EVENT_APP_ERROR: &str = "app-error";
pub const EVENT_APP_STATE: &str = "app-state";
pub const EVENT_APP_LOCALE_CHANGED: &str = "app-locale-changed";
pub const EVENT_APP_STT_ENGINE_CHANGED: &str = "app-stt-engine-changed";
// EVENT_APP_LLMS_CHANGED は LMGW 移行に伴い廃止済み
pub const EVENT_APP_OWNER_STATUS_CHANGED: &str = "app-owner-status-changed";
pub const EVENT_APP_CA_STATUS_CHANGED: &str = "app-ca-status-changed";
pub const EVENT_APP_LICENSES_CHANGED: &str = "app-licenses-changed";
pub const EVENT_APP_LMGW_PROVIDERS_CHANGED: &str = "app-lmgw-providers-changed";
pub const EVENT_APP_OVERLAY_VISIBILITY: &str = "app-overlay-visibility";
/// Windows 音声入力設定のヘルスチェック結果を通知するイベント名
pub const EVENT_WIN_HEALTH_CHECK: &str = "windows-health-check";
/// オーケストレーターオーバーレイの表示/非表示を通知するイベント名
pub const EVENT_ORCHESTRATOR_DISPLAY: &str = "orchestrator-display";
/// オーケストレーターの認識テキスト（部分/最終）を通知するイベント名
pub const EVENT_ORCHESTRATOR_TEXT: &str = "orchestrator-text";
/// オーケストレーターからの応答を通知するイベント名
pub const EVENT_ORCHESTRATOR_RESPONSE: &str = "orchestrator-response";
/// オーケストレーターがタスク完了と判断したことを通知するイベント名
pub const EVENT_ORCHESTRATOR_TASK_COMPLETED: &str = "orchestrator-task-completed";

// --- Tauri Window Labels ---
pub const WINDOW_LABEL_MAIN: &str = "main";

// --- App Statuses & States ---
pub const APP_STATUS_STOPPED: &str = "stopped";
pub const APP_STATE_IDLE: &str = "Idle";
pub const APP_STATE_RECORDING: &str = "Recording";

// --- Basic Protocols & Domains ---

/// HTTP プロトコル名
pub const PROTOCOL_HTTP: &str = "http";
/// HTTPS プロトコル名
pub const PROTOCOL_HTTPS: &str = "https";
/// WebSocket プロトコル名
pub const PROTOCOL_WS: &str = "ws";
/// WebSocket (Secure) プロトコル名
pub const PROTOCOL_WSS: &str = "wss";
/// HTTP スキームプレフィックス
pub const SCHEME_PREFIX_HTTP: &str = "http://";
/// HTTPS スキームプレフィックス
pub const SCHEME_PREFIX_HTTPS: &str = "https://";

/// Localhost ドメイン
pub const DOMAIN_LOCALHOST: &str = "localhost";
/// Localhost IP アドレス
pub const IP_LOCALHOST: &str = "127.0.0.1";

// --- Database Configuration ---
/// システム共通のデータベース名
pub const DB_NAME: &str = "mycute";
/// SQLite のデフォルトファイル名
pub const SQLITE_DEFAULT_FILENAME: &str = "mycute.sqlite";
/// データベースファイルのデフォルト保存ディレクトリ名 (relative to MYCUTE_HOME)
pub const DB_DEFAULT_DIRNAME: &str = "db";

/// MySQL の標準ポート
pub const DB_PORT_MYSQL: &str = "3306";
/// PostgreSQL の標準ポート
pub const DB_PORT_POSTGRES: &str = "5432";

/// OSCA 証明書の一時ディレクトリ用プレフィックス
pub const MYCUTE_OSCA_TEMP_DIR_PREFIX: &str = match option_env!("APP_OSCA_PREFIX") {
    Some(v) => v,
    None => "mycute-osca-",
};

/// mkcert/fastcert が参照するルート OSCA ディレクトリの環境変数名
pub const ENV_OSCAROOT: &str = "CAROOT";

/// Bifrost の認証シークレットを注入するための環境変数名
pub const ENV_BIFROST_AUTH_SECRET: &str = "BIFROST_AUTH_SECRET";

/// ZeroClaw の認証トークンを注入するための環境変数名
pub const ENV_ZEROCLAW_API_KEY: &str = "ZEROCLAW_API_KEY";

/// OSCA 証明書のダウンロードパス
pub const PATH_OSCA_CERT_DOWNLOAD: &str = match option_env!("APP_OSCA_PATH") {
    Some(v) => v,
    None => "/mycute-osca.pem",
};

/// OSCA URL 取得 API パス
pub const PATH_API_OSCA_URL: &str = "/osca/url";

/// PEM 形式の開始マーカー
pub const PEM_BEGIN: &str = "-----BEGIN";

// --- Identity & Voting ---
/// Ed448 公開鍵の Hex 文字列長 (57 bytes * 2)
pub const ED448_PUBKEY_HEX_LEN: usize = 114;
/// Ed448 秘密鍵の Hex 文字列長 (57 bytes * 2)
pub const ED448_SECKEY_HEX_LEN: usize = 114;
/// Ed448 鍵のバイト長 (57 bytes)
pub const ED448_KEY_BYTES_LEN: usize = 57;
/// Ed448 署名のバイト長 (114 bytes)
pub const ED448_SIGNATURE_BYTES_LEN: usize = 114;
/// Ed448 署名の Hex 文字列長 (114 bytes * 2)
pub const ED448_SIGNATURE_HEX_LEN: usize = 228;

/// 投票用初期クレジット（予算）
pub const VOTING_INITIAL_BALANCE: i32 = 15;

/// 標準的な日時のフォーマット
pub const DATE_FORMAT_STANDARD: &str = "%Y-%m-%d %H:%M:%S";

/// HTTP クライアントのデフォルトタイムアウト（秒）
pub const HTTP_TIMEOUT_SEC: u64 = 10;

// --- Partition & Status ---
/// 特定の組織に属さない分離された ApxID
pub const APX_ID_ISOLATED: i32 = 0;
/// 特定の組織に属さない分離された VdrID
pub const VDR_ID_ISOLATED: i32 = 0;
/// 候補者フラグ: False
pub const IS_CANDIDATE_FALSE: i8 = 0;
/// 候補者フラグ: True
pub const IS_CANDIDATE_TRUE: i8 = 1;

// --- Setting Keys ---
// settings.json および DB の設定テーブル（settings）で使用する一意のキー名。

/// ホットキーの設定（有効化フラグやキーアサインなど）。
pub const SETTING_KEY_HOTKEYS: &str = "hotkeys";
/// 音声認識エンジンの選択（openai, os 等）。
pub const SETTING_KEY_STT_ENGINE: &str = "stt_engine";
/// ユーザーインターフェースの言語設定（ja-JP, en-US 等）。
pub const SETTING_KEY_LOCALE: &str = "locale";
// SETTING_KEY_LLMS は LMGW 移行に伴い廃止済み
/// 音声認識（STT）に関する詳細なパラメータ（感度、タイムアウト等）。
pub const SETTING_KEY_STT: &str = "stt";
/// サーバーおよびネットワークインフラの設定（ポート、ベースURL等）。
pub const SETTING_KEY_SERVER: &str = "server";
/// ストレージおよびディレクトリのパス設定。
pub const SETTING_KEY_STORAGE: &str = "storage";
// SETTING_KEY_CUBER は cuber 削除に伴い廃止済み
/// メインウィンドウの表示位置およびサイズ情報。
pub const SETTING_KEY_WINDOW_POSITION: &str = "window_position";
/// 前回の実行時のアプリケーションバージョン（MacOSの権限リセット検知用）。
pub const SETTING_KEY_LAST_RUN_VERSION: &str = "last_run_version";

// -- 以下、リセット時にも保護される重要アイデンティティ / 証明書関連 --

/// プロキシ通信で使用するサーバー証明書（PEM）。
pub const SETTING_KEY_PROXY_CERT: &str = "proxy_certificate";
/// プロキシ通信で使用するサーバー秘密鍵（PEM）。
pub const SETTING_KEY_PROXY_SEC: &str = "proxy_private_key";
/// ルート認証局（OSCA）の公開証明書（PEM）。
pub const SETTING_KEY_OSCA_CERT: &str = "osca_certificate";
/// ルート認証局（OSCA）の秘密鍵（PEM）。
pub const SETTING_KEY_OSCA_SEC: &str = "osca_private_key";
/// ルート認証局（OSCA）証明書の有効期限（RFC3339形式）。
pub const SETTING_KEY_OSCA_EXPIRE: &str = "osca_expire";
/// ルート認証局（OSCA）の Common Name (CN)。OS の信頼チェックに使用。
pub const SETTING_KEY_OSCA_CN: &str = "osca_cn";

/// このノードを一意に識別する Ed448 公開鍵（Node ID）。
pub const SETTING_KEY_MY_PUB: &str = "my_pub";
/// このノードの所有権を証明する Ed448 秘密鍵。
pub const SETTING_KEY_MY_SEC: &str = "my_sec";
/// ネットワーク内での投票に使用可能な残クレジット（残高）。
pub const SETTING_KEY_MY_REM: &str = "my_rem";
/// 認証済みアイデンティティを示す CA 発行のトークン。
pub const SETTING_KEY_MY_CAT: &str = "my_cat";
/// CA から発行されたライセンスの配列（複数保持可能）。
pub const SETTING_KEY_MY_LICS: &str = "my_lics";

// --- API Paths ---

// --- P2P Middleware Tag Markers ---
pub const TAG_MARKER_P2P_STRICT: &str = ":p2p_strict";
pub const TAG_MARKER_P2P_OPTIONAL: &str = ":p2p_optional";
#[macro_export]
macro_rules! TAG_MACRO_P2P_STRICT {
    () => {
        ":p2p_strict"
    };
}
#[macro_export]
macro_rules! TAG_MACRO_P2P_OPTIONAL {
    () => {
        ":p2p_optional"
    };
}

/// ヘルスチェックパス
pub const PATH_HEALTH: &str = "/v1/health";

/// Bifrost (LMGW) 経由の OpenAI 互換 API エンドポイント（v1）のベースパス
pub const PATH_LMGW_OPENAI_V1: &str = "/v1/lmgw/v1";

/// MYCUTE WS パス
pub const PATH_MYCUTE_WS: &str = "/v1/mycute/events/ws";
/// 言語設定 API パス
pub const PATH_MYCUTE_LANG: &str = "/v1/mycute/lang";
/// WebSocket ステータス確認 API パス
pub const PATH_MYCUTE_WS_STATUS: &str = "/v1/mycute/events/ws/status";

/// オーナーモードアクティベート API パス
pub const PATH_OWNER_ACTIVATE: &str = "/v1/owner/activate";
/// オーナーモードステータス取得 API パス
pub const PATH_OWNER_STATUS: &str = "/v1/owner/status";
/// オーナーモード解除 API パス
pub const PATH_OWNER_DEACTIVATE: &str = "/v1/owner/deactivate";

/// CA アイデンティティエントリーパス
pub const PATH_CA_IDENTITIES_ENTRY: &str = "/v1/ca/identities/entry";
pub const PATH_CA_IDENTITIES_APPLY: &str = "/v1/ca/identities/apply";

/// Node アイデンティティエントリーパス
pub const PATH_NODE_IDENTITIES_ENTRY: &str = "/v1/node/identities/entry";

/// アイデンティティ公開鍵取得パス (Node Side)
pub const PATH_IDENTITIES_PUBKEY: &str = "/v1/node/identities/pubkey";

/// 検証済みアイデンティティ取得パス (CA Side)
pub const PATH_IDENTITIES_SYNC: &str = "/v1/ca/identities/sync";

/// アプリ公開（広告）パス (CA Side)
pub const PATH_CA_APPS_ADVERTISE: &str = "/v1/ca/apps/advertise";

/// アプリ検索パス (CA Side)
pub const PATH_CA_APPS_DISCOVER: &str = "/v1/ca/apps/discover";

/// アプリ投票パス (CA Side)
pub const PATH_CA_APPS_VOTE: &str = "/v1/ca/apps/vote";

/// アプリビルド API の multipart パラメータ名
pub const APP_BUILD_ZIP_PARAM: &str = "zip";

/// アプリビルド時のデフォルトの ZIP ファイル名
pub const APP_BUILD_ZIP_DEFAULT_FILENAME: &str = "app.zip";

/// mycute アプリケーションパッケージの拡張子
pub const APP_BUILD_FILE_EXTENSION: &str = "mycute";

/// アプリケーションマニフェストのファイル名
pub const APP_MANIFEST_FILENAME: &str = "mycute.json";

/// アプリビルド時の一時ディレクトリ用プレフィックス
pub const APP_BUILD_TEMP_DIR_PREFIX: &str = "mycute-build-";

/// アプリビルド時の作業ディレクトリ名
pub const APP_BUILD_WORK_DIRNAME: &str = "src";

/// アプリビルド時の出力ディレクトリ名
pub const APP_BUILD_DIST_DIRNAME: &str = "dist";

/// アプリビルド時のデフォルト出力ファイル名
pub const APP_BUILD_DEFAULT_FILENAME: &str = "app.mycute";

/// アプリビルド時の Zstd 圧縮レベル (19: 最高効率)
pub const APP_BUILD_ZSTD_LEVEL: i32 = 19;
/// パッケージペイロードの暗号化（難読化）に使用するシステム共通ソルト
pub const APP_PACKAGE_KEY_SALT: &str =
    "X7a#9vP2*kL&5mR!qN9zE8sC6jD4fH1uG3oI0yT2xW5bQ8nV0pA9cZ7kX2vM1jB5";

/// MYCUTE OS のデータディレクトリ名
/// ビルド時に APP_DATA_DIR 環境変数が注入された場合はその値を使用し、なければ ".mycute" をデフォルトとする。
pub const MYCUTE_DATA_DIRNAME: &str = match option_env!("APP_DATA_DIR") {
    Some(v) => v,
    None => ".mycute",
};

/// ログの保存ディレクトリ名 (relative to MYCUTE_HOME)
pub const MYCUTE_LOG_DIRNAME: &str = "log";

/// S3 ローカルストレージのディレクトリ名 (relative to MYCUTE_HOME)
pub const MYCUTE_S3_DIRNAME: &str = "s3";

/// ダウンロードファイルのディレクトリ名 (relative to MYCUTE_HOME)
pub const MYCUTE_DL_DIRNAME: &str = "dl";

/// AIモデルの保存ディレクトリ名 (relative to MYCUTE_HOME)
pub const MYCUTE_MODELS_DIRNAME: &str = "models";

// --- Model Filenames ---
pub const MODEL_FILENAME_GTCRN: &str = "gtcrn.onnx";
pub const MODEL_FILENAME_SILERO_VAD: &str = "silero_vad.onnx";
pub const MODEL_FILENAME_SILERO_VAD_INT8: &str = "silero_vad.int8.onnx";
pub const MODEL_FILENAME_TEN_VAD: &str = "ten_vad.onnx";
pub const MODEL_FILENAME_TEN_VAD_INT8: &str = "ten-vad.int8.onnx";
pub const MODEL_FILENAME_TOKENS: &str = "tokens.txt";


/// アプリケーションのインストール先ディレクトリ名
pub const MYCUTE_APPS_DIRNAME: &str = "apps";

/// スクリプトの実行・保存用ディレクトリ名 (relative to MYCUTE_HOME)
pub const MYCUTE_SCRIPTS_DIRNAME: &str = "scripts";

/// アプリインストール API の multipart パラメータ名
pub const APP_INSTALL_MYCUTE_PARAM: &str = "mycute";

/// アプリインストール時のパッケージ保存名
pub const APP_INSTALL_PACKAGE_FILENAME: &str = "uploaded.mycute";

/// アプリインストール時の一時ディレクトリ用プレフィックス
pub const APP_INSTALL_TEMP_DIR_PREFIX: &str = "mycute-install-";

/// アプリ検証時の一時ディレクトリ用プレフィックス
pub const APP_VERIFY_TEMP_DIR_PREFIX: &str = "mycute-verify-";

/// アプリ検証時のパッケージ保存名
pub const APP_VERIFY_PACKAGE_FILENAME: &str = "verify.mycute";

/// アプリ操作（インストール/検証）時の一時展開ディレクトリ名
pub const APP_TEMP_EXTRACT_DIRNAME: &str = "extracted";

/// アプリケーションのレイヤー: システム (削除不可)
pub const APP_LAYER_PREINSTALL: &str = "Preinstall";

/// アプリケーションのレイヤー: ローカルインストール
pub const APP_LAYER_LOCAL: &str = "Local";

/// アプリケーションのレイヤー: リモートサーバー
pub const APP_LAYER_REMOTE: &str = "Remote";

// --- Ticket / Identity Keys ---
pub const KEY_TICKET_NODE_PUBKEY: &str = "node_pubkey";
pub const KEY_TICKET_INITIAL_BALANCE: &str = "initial_balance";
pub const KEY_TICKET_ISSUED_AT: &str = "issued_at";
pub const KEY_TICKET_SIGNATURE: &str = "signature";
pub const KEY_TICKET_CA_PUBKEY: &str = "ca_pubkey";
pub const KEY_TICKET_FORUM_ID: &str = "forum_id";
pub const KEY_TICKET_FORUM_NAME: &str = "forum_name";
pub const KEY_TICKET_FORUM_DESC: &str = "forum_description";
pub const KEY_TICKET_CA_BASE_URL: &str = "ca_base_url";

// --- Project-wide Messages ---
pub const MSG_MY_BASE_URL_FATAL: &str = "FATAL: 'my_base_url' is not configured in settings. Every node must declare its own public URL to participate in the MYCUTE network. Please set 'server.my_base_url' to your node's reachable URL (e.g., \"http://localhost:3910\" for local development).";

// --- Project-wide Error Codes ---
pub const ERR_ANCHOR_KEY: &str = "ERR_ANCHOR_KEY";
pub const ERR_APP_NOT_FOUND: &str = "ERR_APP_NOT_FOUND";
pub const ERR_BUILD_FAILED: &str = "ERR_BUILD_FAILED";
pub const ERR_CA_CONNECT: &str = "ERR_CA_CONNECT";
pub const ERR_CA_ERROR: &str = "ERR_CA_ERROR";
pub const ERR_CA_PARSE: &str = "ERR_CA_PARSE";
pub const ERR_CA_PUBKEY: &str = "ERR_CA_PUBKEY";
pub const ERR_CA_RESPONSE: &str = "ERR_CA_RESPONSE";
pub const ERR_CA_TRUST_FAIL: &str = "ERR_CA_TRUST_FAIL";
pub const ERR_CA_UNREACHABLE: &str = "ERR_CA_UNREACHABLE";
pub const ERR_COMPRESS_VOTES: &str = "ERR_COMPRESS_VOTES";
pub const ERR_DB: &str = "ERR_DB";
pub const ERR_DECODE: &str = "ERR_DECODE";
pub const ERR_DECODE_VOTES: &str = "ERR_DECODE_VOTES";
pub const ERR_DECOMPRESS_VOTES: &str = "ERR_DECOMPRESS_VOTES";
pub const ERR_DECRYPT: &str = "ERR_DECRYPT";
pub const ERR_DECRYPT_VOTES: &str = "ERR_DECRYPT_VOTES";
pub const ERR_EMPTY_FILE: &str = "ERR_EMPTY_FILE";
pub const ERR_ENCRYPT: &str = "ERR_ENCRYPT";
pub const ERR_ENCRYPT_VOTES: &str = "ERR_ENCRYPT_VOTES";
pub const ERR_EXTRACT_FAILED: &str = "ERR_EXTRACT_FAILED";
pub const ERR_EXTRACT_ZIP: &str = "ERR_EXTRACT_ZIP";
pub const ERR_HTTP_CLIENT: &str = "ERR_HTTP_CLIENT";
pub const ERR_IDENTITY: &str = "ERR_IDENTITY";
pub const ERR_IDENTITY_GEN: &str = "ERR_IDENTITY_GEN";
pub const ERR_INSTALL_IO: &str = "ERR_INSTALL_IO";
pub const ERR_INVALID_CA_KEY: &str = "ERR_INVALID_CA_KEY";
pub const ERR_INVALID_CA_RESPONSE: &str = "ERR_INVALID_CA_RESPONSE";
pub const ERR_INVALID_CA_TOKEN: &str = "ERR_INVALID_CA_TOKEN";
pub const ERR_INVALID_KEY: &str = "ERR_INVALID_KEY";
pub const ERR_INVALID_PUBKEY: &str = "ERR_INVALID_PUBKEY";
pub const ERR_INVALID_RESP: &str = "ERR_INVALID_RESP";
pub const ERR_INVALID_SIG: &str = "ERR_INVALID_SIG";
pub const ERR_INVALID_ZIP: &str = "ERR_INVALID_ZIP";
pub const ERR_INSUFFICIENT_FUNDS: &str = "ERR_INSUFFICIENT_FUNDS";
pub const ERR_IO: &str = "ERR_IO";
pub const ERR_LOW_LEVEL: &str = "ERR_LOW_LEVEL";
pub const ERR_MULTIPART: &str = "ERR_MULTIPART";
pub const ERR_MY_PUBKEY: &str = "ERR_MY_PUBKEY";
pub const ERR_NOT_IDENTIFIED: &str = "ERR_NOT_IDENTIFIED";
pub const ERR_NO_IDENTITY: &str = "ERR_NO_IDENTITY";
pub const ERR_NO_OWNER_KEY: &str = "ERR_NO_OWNER_KEY";
pub const ERR_OWNER_MODE: &str = "ERR_OWNER_MODE";
pub const ERR_OWNER_MODE_REQUIRED: &str = "ERR_OWNER_MODE_REQUIRED";
pub const ERR_PARSE_MANIFEST: &str = "ERR_PARSE_MANIFEST";
pub const ERR_PARSE_VOTES: &str = "ERR_PARSE_VOTES";
pub const ERR_READ_FILE: &str = "ERR_READ_FILE";
pub const ERR_READ_MANIFEST: &str = "ERR_READ_MANIFEST";
pub const ERR_READ_OUTPUT: &str = "ERR_READ_OUTPUT";
pub const ERR_REMOVE_OLD: &str = "ERR_REMOVE_OLD";
pub const ERR_SAVE: &str = "ERR_SAVE";
pub const ERR_SERIALIZE_VOTES: &str = "ERR_SERIALIZE_VOTES";
pub const ERR_SIGN: &str = "ERR_SIGN";
pub const ERR_SIGNING: &str = "ERR_SIGNING";
pub const ERR_SIG_FAIL: &str = "ERR_SIG_FAIL";
pub const ERR_TARGET_ERROR: &str = "ERR_TARGET_ERROR";
pub const ERR_TARGET_RESPONSE: &str = "ERR_TARGET_RESPONSE";
pub const ERR_TARGET_UNREACHABLE: &str = "ERR_TARGET_UNREACHABLE";
pub const ERR_TICKET_PARSE: &str = "ERR_TICKET_PARSE";
pub const ERR_EXPECTED_RECORD: &str = "ERR_EXPECTED_RECORD";
pub const ERR_VERIFICATION_PENDING: &str = "ERR_VERIFICATION_PENDING";
pub const ERR_WRITE_PKG: &str = "ERR_WRITE_PKG";

// ==========================================
// Mutual Verification & Blacklist
// ==========================================
pub const HEADER_X_MYCUTE_TIMESTAMP: &str = "X-MyCute-Timestamp";
pub const HEADER_X_MYCUTE_CA_BASE_URL: &str = "X-MyCute-CA-Base-URL";
pub const HEADER_X_MYCUTE_SIGNATURE: &str = "X-MyCute-Signature";
pub const HEADER_X_MYCUTE_SENDER_PUBKEY: &str = "X-MyCute-Sender-Pubkey";

/// Blacklist Sync & Report Paths
pub const PATH_BLACKLISTS_SYNC: &str = "/v1/blacklists/sync";
pub const PATH_BLACKLISTS_REPORT: &str = "/v1/blacklists/report";

/// 時刻許容誤差 (ms): 30秒
pub const TIMESTAMP_TOLERANCE_MS: i64 = 30_000;

/// ブラックリスト削除時の安全マージン (時間)
/// 刑期 + この時間が経過したレコードを物理削除する。
pub const BLACKLIST_CLEANUP_MARGIN_HOURS: i64 = 1;

/// 定期クリーナータスクの実行間隔 (秒)
pub const CLEANER_TASK_INTERVAL_SEC: u64 = 3600;

/// アイデンティティレイヤー判定のキャッシュ有効期限 (秒)
pub const IDENTITY_LAYER_CACHE_TTL_SEC: u64 = 3600; // 60分

/// アイデンティティレイヤー判定のキャッシュ最大件数
pub const IDENTITY_LAYER_CACHE_MAX_SIZE: usize = 10000;

/// 定期的な情報保持タスクの実行間隔 (秒)
pub const PERIODICAL_STORE_INTERVAL_SEC: u64 = 3600; // 60分

/// P2P ブラックリスト同期対象CAの選択最大数
/// 受信したCAのBASE_URLリストの中から、この数だけランダムに選んで同期を実行する。
pub const P2P_BLACKLIST_SYNC_TARGET_MAX: usize = 3;

// --- StatusCode Aliases (Rust only, avoid TS sync if possible) ---
pub const ST_OK: StatusCode = StatusCode::OK;
pub const ST_CREATED: StatusCode = StatusCode::CREATED;
pub const ST_ACCEPTED: StatusCode = StatusCode::ACCEPTED;
pub const ST_BAD_REQUEST: StatusCode = StatusCode::BAD_REQUEST;
pub const ST_UNAUTHORIZED: StatusCode = StatusCode::UNAUTHORIZED;
pub const ST_FORBIDDEN: StatusCode = StatusCode::FORBIDDEN;
pub const ST_NOT_FOUND: StatusCode = StatusCode::NOT_FOUND;
pub const ST_INTERNAL_SERVER_ERROR: StatusCode = StatusCode::INTERNAL_SERVER_ERROR;
pub const ST_BAD_GATEWAY: StatusCode = StatusCode::BAD_GATEWAY;
pub const ST_UNPROCESSABLE_ENTITY: StatusCode = StatusCode::UNPROCESSABLE_ENTITY;
pub const ST_CONFLICT: StatusCode = StatusCode::CONFLICT;
