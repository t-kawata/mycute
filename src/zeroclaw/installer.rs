use crate::zeroclaw::assets::{get_zeroclaw_asset, ArchiveFormat};
use crate::zeroclaw::error::ZeroClawError;
use crate::zeroclaw::ZEROCLAW_DIRNAME;
use crate::constants::IP_LOCALHOST;
use anyhow::{Context, Result};
use flate2::read::GzDecoder;
use std::fs;
use std::io::Cursor;
use std::path::{Path, PathBuf};
#[cfg(target_os = "macos")]
use std::process::Command;
use tar::Archive;
use zip::ZipArchive;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

pub struct InstallResult {
    pub root_dir: PathBuf,
    pub install_dir: PathBuf,
}

/// config.toml の完全なテンプレート。
/// docs/INFO-ZEROCLAW-SETTINGS.md の Q4 節にある日本語コメントと全設定項目を完全に網羅。
const ZEROCLAW_CONFIG_TEMPLATE: &str = r#"# =============================================================================
# ZeroClaw 設定ファイル
# このファイルには設定可能な全項目を日本語の説明付きで記載しています
# =============================================================================

# -----------------------------------------------------------------------------
# 基本設定
# -----------------------------------------------------------------------------

# APIキー（選択したプロバイダ用）
# 環境変数 ZEROCLAW_API_KEY または API_KEY で上書き可能
# 例: "sk-ant-...", "sk-proj-..."
api_key = ""

# デフォルトのプロバイダIDまたはエイリアス
# 利用可能な値: "openai" (Bifrost連携時はこれを推奨), "anthropic", "gemini", "ollama" 等
# 環境変数 ZEROCLAW_PROVIDER で上書き可能
default_provider = "openai"

# デフォルトモデル（Bifrost側で設定されている有効なモデル名、または provider/model形式を推奨）
# 例: "openai/gpt-5.4-mini", "anthropic/claude-3-5-sonnet", "gemini/gemini-1.5-pro"
default_model = "openai/gpt-5.4-mini"

# デフォルトのモデル温度（0.0〜2.0）
# 0.0: 最も決定論的、2.0: 最もランダム
default_temperature = 0.7

# プロバイダAPI呼び出しのHTTPタイムアウト（秒）
# 遅いバックエンド（llama.cppなど）では増やす
provider_timeout_secs = 120

# プロバイダAPIリクエストに含める最大出力トークン数
# OpenRouterなどで重要（デフォルト65536がエラーになる場合あり）
# provider_max_tokens = 4096

# OpenAI互換プロバイダーのエンドポイント (Bifrost連携)
api_url = "http://{ip_localhost}:{bifrost_port}/v1"

# ワークスペースディレクトリ
workspace_dir = "{workspace_dir}"

# プロバイダAPIリクエストに含める追加HTTPヘッダー
# 例: { "User-Agent" = "ZeroClaw/1.0", "HTTP-Referer" = "https://example.com" }
# [extra_headers]

# -----------------------------------------------------------------------------
# [observability] - 監視設定
# -----------------------------------------------------------------------------

[observability]
# 監視バックエンド
# 利用可能な値: "none", "noop", "log", "prometheus", "otel", "opentelemetry", "otlp"
backend = "none"

# OTLP HTTPエンドポイント（backendが"otel"の場合）
# 例: "http://localhost:4318"
otel_endpoint = "http://localhost:4318"

# OTLPコレクターに送信するサービス名
otel_service_name = "zeroclaw"

# ランタイムトレース保存モード
# 利用可能な値: "none", "rolling", "full"
runtime_trace_mode = "none"

# ランタイムトレースJSONLパス（ワークスペース相対または絶対パス）
runtime_trace_path = "state/runtime-trace.jsonl"

# runtime_trace_mode="rolling"時の最大保持イベント数
runtime_trace_max_entries = 200

# -----------------------------------------------------------------------------
# [autonomy] - 自律性とセキュリティポリシー
# -----------------------------------------------------------------------------

[autonomy]
# 自律性レベル
# 利用可能な値: "readonly"（読み取り専用）, "supervised"（監視付き、デフォルト）, "full"（完全自律）
level = "supervised"

# ワークスペース内のみでの操作を制限
workspace_only = false

# 許可するコマンドリスト（空の場合はすべて拒否）
allowed_commands = [
    "git", "npm", "cargo", "mkdir", "touch", "cp", "mv", "ls", "cat", "grep", 
    "find", "echo", "pwd", "wc", "head", "tail", "date"
]

# コマンド実行のコンテキストルール
# command_context_rules = []

# 禁止するパスリスト
forbidden_paths = [
    "/etc", "/root", "/usr", "/bin", "/sbin", "/lib", "/opt", "/boot", "/dev"
]

# 自動承認するツールリスト
auto_approve = [
    "file_read", "file_write", "file_edit", "memory_recall", "memory_store",
    "web_search_tool", "web_fetch", "calculator", "glob_search", "content_search"
]

# -----------------------------------------------------------------------------
# [security] - セキュリティサブシステム
# -----------------------------------------------------------------------------

[security]
# 有効化
enabled = false

# -----------------------------------------------------------------------------
# [agent] - エージェント動作設定
# -----------------------------------------------------------------------------

[agent]
# コンパクトコンテキストモード（13B以下のモデル向け）
# trueの場合: bootstrap_max_chars=6000, rag_chunk_limit=2
compact_context = true

# ユーザーメッセージあたりの最大ツール呼び出しループ回数
# 0の場合は安全なデフォルト値10を使用
max_tool_iterations = 10

# セッションあたりの最大会話履歴メッセージ数
max_history_messages = 50

# 1回のイテレーション内での並列ツール実行を有効化
parallel_tools = false

# ツールディスパッチ戦略
# 利用可能な値: "auto", "native", "xml"
tool_dispatcher = "auto"

# イテレーション内重複呼び出し抑制から免除するツール名
tool_call_dedup_exempt = []

# ターンごとのMCPツールスキーマフィルタグループ
tool_filter_groups = []

# -----------------------------------------------------------------------------
# [gateway] - ゲートウェイ（HTTP/WebSocketサーバー）設定
# -----------------------------------------------------------------------------

[gateway]
# ゲートウェイがリッスンするポート (DEFAULT_ZEROCLAW_PORT: 3913)
port = {zeroclaw_port}

# バインドするホストアドレス
# "[::]" ならIPv6とIPv4の両方をリッスン
host = "{ip_localhost}" # ローカルインターフェースでのみリッスン

# パブリックバインドを許可（セキュリティ上注意）
allow_public_bind = false

# ペアリングを要求
require_pairing = true

# -----------------------------------------------------------------------------
# [channels_config] - チャネル設定
# -----------------------------------------------------------------------------

[channels_config]
# 有効化
enabled = true

# CLIチャネルを有効化
cli = true

# Telegramチャネル設定
# [channels_config.telegram]
# bot_token = "123456:ABC-DEF..."

# Discordチャネル設定
# [channels_config.discord]
# token = "your-bot-token"

# Slackチャネル設定
# [channels_config.slack]
# bot_token = "xoxb-..."
# app_token = "xapp-..."

# WhatsAppチャネル設定
# [channels_config.whatsapp]
# phone_id = "..."
# access_token = "..."
# webhook_verify_token = "..."

# -----------------------------------------------------------------------------
# [memory] - メモリバックエンド設定
# -----------------------------------------------------------------------------

[memory]
# SQLiteデータベースは自動的に{workspace}/memory/brain.dbに作成されます  
# ワークスペースディレクトリ内のmemoryサブディレクトリに保存  
# パスをカスタマイズする場合は環境変数やストレージ設定を使用  
# データベースファイルは直接操作せず、ZeroClawのAPI経由でアクセス

# メモリバックエンドの種類を指定  
# 利用可能な値: "sqlite", "markdown", "lucid", "qdrant", "none"  
# sqlite: SQLiteデータベースを使用（推奨）  
# markdown: Markdownファイルとして保存  
# lucid: Lucidハイブリッドストレージ  
# qdrant: Qdrantベクターデータベース  
# none: メモリ機能を無効化  
backend = "sqlite"  

# ユーザーの入力を自動的にメモリに保存するかどうか  
# true: 保存する（推奨）、false: 保存しない  
# アシスタントの出力は保存されず、ユーザーの入力のみが保存対象  
auto_save = true  

# SQLiteデータベースパス
# 注意: ZeroClawは常に固定ファイル名 "brain.db" を使用するため、db_path設定は無視されます
# データベースは自動的に {workspace_dir}/memory/brain.db に作成されます

# メモリのハイジーン（清掃）機能を有効にするかどうか  
# true: 古いデータのアーカイブや削除を自動実行  
# false: 手動でのみ実行  
hygiene_enabled = true  

# ハイジーン実行時に、何日経過したデータをアーカイブするか  
# 指定日数経過したデイリーファイルをアーカイブ対象にする  
archive_after_days = 7  

# ハイジーン実行時に、何日経過したアーカイブを削除するか  
# 指定日数経過したアーカイブファイルを完全に削除  
purge_after_days = 30  

# SQLiteバックエンドの場合、何日経過した会話データを削除するか  
# データベース内の古い会話レコードを自動的にクリーンアップ  
conversation_retention_days = 30  

# 埋め込みプロバイダーの指定  
# "none": 埋め込み機能を使用しない（デフォルト）  
# "openai": OpenAIの埋め込みAPIを使用  
# "custom:URL": カスタムエンドポイントを使用  
embedding_provider = "none"  

# 使用する埋め込みモデルの名前  
# embedding_providerが"none"以外の場合に有効  
# 例: "text-embedding-3-small", "text-embedding-ada-002"  
embedding_model = "text-embedding-3-small"  

# 埋め込みベクトルの次元数  
# モデルによって固定（text-embedding-3-smallは1536次元）  
# ベクター検索の精度に影響  
embedding_dimensions = 1536  

# ハイブリッド検索时的ベクトル類似度の重み（0.0〜1.0）  
# 高い値ほどベクトル検索を重視  
# keyword_weightとの合計が1.0になるように調整  
vector_weight = 0.7  

# ハイブリッド検索時のキーワード検索（BM25）の重み（0.0〜1.0）  
# 高い値ほどキーワード検索を重視  
# vector_weightとの合計が1.0になるように調整  
keyword_weight = 0.3  

# メモリ検索時の最低関連性スコア（0.0〜1.0）  
# このスコア未満のメモリは検索結果から除外  
# 低品質なメモリがコンテキストに混入するのを防ぐ  
min_relevance_score = 0.4  

# 埋め込みキャッシュの最大エントリ数  
# 埋め込み計算の重複を避けるためのキャッシュサイズ  
# 多いほどAPIコールが減少するが、メモリ使用量が増加  
embedding_cache_size = 10000  

# ドキュメント分割時の最大トークン数  
# 長いドキュメントをこのサイズで分割して処理  
# モデルのコンテキスト長に応じて調整  
chunk_max_tokens = 512  

# レスポンスキャッシュを有効にするかどうか  
# true: 同じプロンプトへの応答をキャッシュしてAPIコストを削減  
# false: キャッシュを使用しない  
response_cache_enabled = false  

# レスポンスキャッシュの有効期間（分）  
# この時間経過後、キャッシュエントリは無効化  
response_cache_ttl_minutes = 60  

# レスポンスキャッシュの最大エントリ数  
# この数を超えると、古いエントリから削除（LRU方式）  
response_cache_max_entries = 5000  

# メモリスナップショット機能を有効にするかどうか  
# true: 定期的にMEMORY_SNAPSHOT.mdにエクスポート  
# false: スナップショットを作成しない  
snapshot_enabled = false  

# ハイジーン実行時にスナップショットも作成するかどうか  
# true: ハイジーンと同時にスナップショットを作成  
# false: ハイジーン時にはスナップショットを作成しない  
snapshot_on_hygiene = false  

# brain.dbが存在しない場合、MEMORY_SNAPSHOT.mdから自動復元するか  
# true: 自動で復元を試みる  
# false: 復元しない  
auto_hydrate = true  

# SQLiteのジャーナルモード  
# "wal": Write-Ahead Loggingモード（推奨、並列読み書きが可能）  
# "delete": 従来のDELETEモード  
# "memory": メモリ上でジャーナルを管理  
sqlite_journal_mode = "wal"

# Markdownファイル保存ディレクトリ
markdown_dir = "memory/markdown"

# Lucidハイブリッド設定
# [memory.lucid]
# db_path = "memory/lucid.db"
# embeddings_model = "text-embedding-ada-002"

# -----------------------------------------------------------------------------
# [model_providers] - 名前付きプロバイダプロファイル
# -----------------------------------------------------------------------------

[model_providers]

# OpenAIプロバイダ設定例
# [model_providers.openai]
# api_key = "sk-..."
# base_url = "https://api.openai.com/v1"

# Anthropicプロバイダ設定例
# [model_providers.anthropic]
# api_key = "sk-ant-..."

# カスタムプロバイダ設定例
# [model_providers.custom]
# api_key = "your-key"
# base_url = "https://your-api.com/v1"

# -----------------------------------------------------------------------------
# [cost] - コスト追跡と予算執行
# -----------------------------------------------------------------------------

[cost]
# コスト追跡を有効化
enabled = false

# 1日の利用制限（USD）
daily_limit_usd = 10.0

# 1ヶ月の利用制限（USD）
monthly_limit_usd = 100.0

# 警告を表示する利用率（パーセント）
warn_at_percent = 80

# 上書きを許可
allow_override = false

# モデルごとの価格設定（USD/100万トークン）
# [cost.prices."anthropic/claude-sonnet-4-20250514"]
# input = 3.0
# output = 15.0

# -----------------------------------------------------------------------------
# [backup] - バックアップ設定
# -----------------------------------------------------------------------------

[backup]
# 有効化
enabled = false

# -----------------------------------------------------------------------------
# [data_retention] - データ保持とパージ設定
# -----------------------------------------------------------------------------

[data_retention]
# 有効化
enabled = false

# -----------------------------------------------------------------------------
# [cloud_ops] - クラウド変換アクセラレータ設定
# -----------------------------------------------------------------------------

[cloud_ops]
# 有効化
enabled = false

# -----------------------------------------------------------------------------
# [pacing] - スロー/ローカルLLMワークロードのペーシング制御
# -----------------------------------------------------------------------------

[pacing]
# 有効化
enabled = false

# -----------------------------------------------------------------------------
# [skills] - スキル読み込みとコミュニティリポジトリ動作
# -----------------------------------------------------------------------------

[skills]
# 有効化
enabled = true

# -----------------------------------------------------------------------------
# [pipeline] - パイプラインツール設定
# -----------------------------------------------------------------------------

[pipeline]
# 有効化
enabled = false

# -----------------------------------------------------------------------------
# モデルルーティングルール
# -----------------------------------------------------------------------------

# model_routes = []

# -----------------------------------------------------------------------------
# エンベディングルーティングルール
# -----------------------------------------------------------------------------

# embedding_routes = []

# -----------------------------------------------------------------------------
# [query_classification] - 自動クエリ分類
# -----------------------------------------------------------------------------

[query_classification]
# 有効化
enabled = false

# -----------------------------------------------------------------------------
# [heartbeat] - 定期ヘルスチェック設定
# -----------------------------------------------------------------------------

[heartbeat]
# 有効化
enabled = true

# -----------------------------------------------------------------------------
# [trust] - 信頼スコアリングと回帰検出
# -----------------------------------------------------------------------------

[trust]
# 有効化
enabled = false

# -----------------------------------------------------------------------------
# [composio] - Composio統合設定
# -----------------------------------------------------------------------------

[composio]
# 有効化
enabled = false

# -----------------------------------------------------------------------------
# [microsoft365] - Microsoft 365 Graph API統合
# -----------------------------------------------------------------------------

[microsoft365]
# 有効化
enabled = false

# -----------------------------------------------------------------------------
# [secrets] - シークレット暗号化設定
# -----------------------------------------------------------------------------

[secrets]
# 有効化
enabled = false

# -----------------------------------------------------------------------------
# [browser] - ブラウザ自動化設定
# -----------------------------------------------------------------------------

[browser]
# 有効化
enabled = true

# -----------------------------------------------------------------------------
# [browser_delegate] - ブラウザ委任設定
# -----------------------------------------------------------------------------

[browser_delegate]
# 有効化
enabled = false
# CLIバイナリ
cli_binary = "claude"
# Chromeプロファイルディレクトリ（SSOセッション維持用）
chrome_profile_dir = ""
# 許可するドメインリスト（空の場合はすべて許可）
allowed_domains = []

# -----------------------------------------------------------------------------
# [knowledge] - ナレッジベース設定
# -----------------------------------------------------------------------------

[knowledge]
# 有効化
enabled = false
# データベースパス
db_path = "~/.zeroclaw/knowledge.db"
# 最大ノード数
max_nodes = 100000
# 自動キャプチャ
auto_capture = false
# クエリ時提案
suggest_on_query = true
# ワークスペース横断検索
cross_workspace_search = false

# -----------------------------------------------------------------------------
# [linkedin] - LinkedIn統合設定
# -----------------------------------------------------------------------------

[linkedin]
# 有効化
enabled = false
# APIバージョン（YYYYMM形式）
api_version = "202401"

# [linkedin.content]
# コンテンツ戦略設定

# [linkedin.image]
# 画像生成設定

# -----------------------------------------------------------------------------
# [estop] - 緊急停止設定
# -----------------------------------------------------------------------------

[estop]
# 有効化
enabled = false
# 状態ファイルパス
state_file = "~/.zeroclaw/estop-state.json"
# 再開時にOTPを要求
require_otp_to_resume = true

# -----------------------------------------------------------------------------
# [resource_limits] - リソース制限
# -----------------------------------------------------------------------------

[resource_limits]
# 最大メモリ（MB）
max_memory_mb = 4096
# 最大CPU時間（秒）
max_cpu_time_seconds = 300
# 最大サブプロセス数
max_subprocesses = 10
# メモリ監視を有効化
memory_monitoring = true

# -----------------------------------------------------------------------------
# [audit] - 監査ログ設定
# -----------------------------------------------------------------------------

[audit]
# 有効化
enabled = false
# ログファイルパス（zeroclawディレクトリ相対）
log_path = "logs/audit.log"
# ローテーション前の最大サイズ（MB）
max_size_mb = 100
# イベントにHMAC署名（改ざん検出用）
sign_events = false

# -----------------------------------------------------------------------------
# [peripherals] - ハードウェア周辺機器設定
# -----------------------------------------------------------------------------

[peripherals]
# 有効化
enabled = false
# データシートディレクトリ
datasheet_dir = "docs/datasheets"

# [[peripherals.boards]]
# ボードタイプ: "nucleo-f401re", "rpi-gpio", "esp32" など
# board = "nucleo-f401re"
# トランスポート: "serial", "native", "websocket"
# transport = "serial"
# シリアルパス
# path = "/dev/ttyACM0"
# ボーレート
# baud = 115200

# -----------------------------------------------------------------------------
# [multimodal] - マルチモーダル設定
# -----------------------------------------------------------------------------

[multimodal]
# リモート画像フェッチを許可
allow_remote_fetch = false

# -----------------------------------------------------------------------------
# [tunnel] - トンネル設定
# -----------------------------------------------------------------------------

# [tunnel]
# 有効化
# enabled = false

# -----------------------------------------------------------------------------
# [transcription] - 文字起こし設定
# -----------------------------------------------------------------------------

# [transcription]
# 有効化
# enabled = false

# -----------------------------------------------------------------------------
# [reliability] - 信頼性設定（リトライなど）
# -----------------------------------------------------------------------------

# [reliability]
# 有効化
# enabled = false
"#;

pub fn install(home: &Path, bifrost_port: u16, zeroclaw_port: u16) -> Result<InstallResult> {
    let asset = get_zeroclaw_asset().ok_or_else(|| {
        ZeroClawError::UnsupportedPlatform(
            "No ZeroClaw asset available for this platform".to_string(),
        )
    })?;

    let root_dir = home.join(ZEROCLAW_DIRNAME);
    let install_dir = root_dir.join(asset.version);

    // 共通ディレクトリの作成
    fs::create_dir_all(&root_dir).context("Failed to create ZeroClaw root directory")?;

    // バイナリの展開
    if !install_dir.exists() {
        log::info!("Installing ZeroClaw {} to {:?}", asset.version, install_dir);
        fs::create_dir_all(&install_dir).context("Failed to create install directory")?;

        match asset.format {
            ArchiveFormat::TarGz => {
                extract_tar_gz(asset.bytes, &install_dir)?;
            }
            ArchiveFormat::Zip => {
                extract_zip(asset.bytes, &install_dir)?;
            }
        }

        #[cfg(target_os = "macos")]
        remove_quarantine_flag(&install_dir)?;
    } else {
        log::info!(
            "ZeroClaw {} is already installed at {:?}",
            asset.version,
            install_dir
        );
    }

    // config.toml の生成とディレクトリのスキャフォールディング
    generate_config_toml(&root_dir, bifrost_port, zeroclaw_port)?;

    Ok(InstallResult {
        root_dir,
        install_dir,
    })
}

/// config.toml の生成と、ワークスペースに必要なディレクトリ群の作成
fn generate_config_toml(root_dir: &Path, bifrost_port: u16, zeroclaw_port: u16) -> Result<()> {
    let config_path = root_dir.join("config.toml");
    let workspace_dir = root_dir.join("workspace");

    // スキャフォールディング: ワークスペースに必要なディレクトリを作成
    let subdirs = ["sessions", "memory", "state", "cron", "skills"];
    for subdir in &subdirs {
        fs::create_dir_all(workspace_dir.join(subdir)).context(format!(
            "Failed to create ZeroClaw workspace subdir: {}",
            subdir
        ))?;
    }

    // パスを TOML 文字列として安全にするためスラッシュに正規化 (クロスプラットフォーム対応)
    let workspace_path_str = workspace_dir.to_string_lossy().replace("\\", "/");

    let config_content = ZEROCLAW_CONFIG_TEMPLATE
        .replace("{ip_localhost}", IP_LOCALHOST)
        .replace("{bifrost_port}", &bifrost_port.to_string())
        .replace("{zeroclaw_port}", &zeroclaw_port.to_string())
        .replace("{workspace_dir}", &workspace_path_str);

    log::info!("Generating ZeroClaw config at {:?}", config_path);
    fs::write(&config_path, config_content).context("Failed to write ZeroClaw config.toml")?;

    #[cfg(unix)]
    {
        if let Ok(metadata) = fs::metadata(&config_path) {
            let mut perms = metadata.permissions();
            perms.set_mode(0o600);
            if let Err(e) = fs::set_permissions(&config_path, perms) {
                log::warn!("Failed to set permissions 600 for config.toml: {}", e);
            }
        }
    }

    Ok(())
}

fn extract_tar_gz(bytes: &[u8], target: &Path) -> Result<()> {
    let tar = GzDecoder::new(Cursor::new(bytes));
    let mut archive = Archive::new(tar);

    // ZeroClaw は圧縮ファイル直下にバイナリがある構造のため、
    // nodejs のようにトップレベルディレクトリをスキップせずに展開
    for entry in archive.entries().context("Failed to read tar entries")? {
        let mut entry = entry.context("Failed to get tar entry")?;
        let path = entry
            .path()
            .context("Failed to get entry path")?
            .to_path_buf();
        let dest = target.join(path);

        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }

        entry
            .unpack(&dest)
            .context(format!("Failed to unpack to {:?}", dest))?;

        #[cfg(unix)]
        apply_unix_permissions(&dest)?;
    }

    Ok(())
}

fn extract_zip(bytes: &[u8], target: &Path) -> Result<()> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).context("Failed to read zip archive")?;

    for i in 0..archive.len() {
        let mut file = archive
            .by_index(i)
            .context("Failed to get zip file entry")?;
        let outpath = match file.enclosed_name() {
            Some(path) => path.to_owned(),
            None => continue,
        };

        let dest = target.join(outpath);

        if file.name().ends_with('/') {
            fs::create_dir_all(&dest).context("Failed to create zip directory")?;
        } else {
            if let Some(p) = dest.parent() {
                if !p.exists() {
                    fs::create_dir_all(p)
                        .context("Failed to create parent directory for zip entry")?;
                }
            }
            let mut outfile =
                fs::File::create(&dest).context("Failed to create file for zip entry")?;
            std::io::copy(&mut file, &mut outfile).context("Failed to copy zip entry to file")?;
        }

        #[cfg(unix)]
        apply_unix_permissions(&dest)?;
    }

    Ok(())
}

#[cfg(unix)]
fn apply_unix_permissions(path: &Path) -> Result<()> {
    if path.is_file() {
        // ZeroClaw バイナリ自体に実行権限を付与
        if let Ok(metadata) = fs::metadata(path) {
            let mut perms = metadata.permissions();
            perms.set_mode(0o755);
            let _ = fs::set_permissions(path, perms);
        }
    }
    Ok(())
}

/// macOS の隔離属性 (Quarantine flag) を再帰的に除去します。
#[cfg(target_os = "macos")]
fn remove_quarantine_flag(install_dir: &Path) -> Result<()> {
    log::info!(
        "Removing macOS quarantine flag from ZeroClaw at {:?}",
        install_dir
    );

    let status = Command::new("xattr")
        .arg("-rc")
        .arg(install_dir)
        .status()
        .context("Failed to execute xattr command for ZeroClaw")?;

    if !status.success() {
        log::warn!(
            "xattr command for ZeroClaw failed with status: {:?}",
            status
        );
    }

    Ok(())
}
