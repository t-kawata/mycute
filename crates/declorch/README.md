# Declorch — Agent 実行ランタイム

**Declorch** は宣言的オーケストレーションによる Agent 実行ランタイムです。
Darvium などの上位システムから `AgentStep` インターフェースを通じて利用され、
Agent の生成・メッセージ送信・Workflow 実行を提供します。

> **宣言的オーケストレーション (Declarative Orchestration)** が名前の由来です。
> LLM呼び出し・ツール実行・セッション管理の複雑さを隠蔽し、宣言的な設定（TOML）で
> Agent と Workflow を定義・実行します。

---

## 目次

1. [Darvium との接続方式](#1-darvium-との接続方式)
2. [DeclorchKernel の起動](#2-declorchkernel-の起動)
3. [Agent 定義 (TOML リファレンス)](#3-agent-定義-toml-リファレンス)
4. [Agent の操作](#4-agent-の操作)
5. [Workflow 定義 (TOML リファレンス)](#5-workflow-定義-toml-リファレンス)
6. [Workflow の実行](#6-workflow-の実行)
7. [AgentStep (KernelHandle トレイト)](#7-agentstep-kernelhandle-トレイト)
8. [戻り値の型完全リファレンス](#8-戻り値の型完全リファレンス)
9. [イベントシステム](#9-イベントシステム)
10. [エラー型](#10-エラー型)
11. [パフォーマンス特性](#11-パフォーマンス特性)

---

## 1. Darvium との接続方式

Declorch と Darvium は **同一プロセス内 (in-process)** で接続します。
`declorch-kernel` クレートを依存関係に追加し、`DeclorchKernel` を直接インスタンス化します。

### Cargo.toml

```toml
[dependencies]
declorch-kernel = { path = "../declorch/crates/declorch-kernel" }
tokio = { version = "1", features = ["full"] }
```

### アーキテクチャ

```
┌─────────────────────────────────────────────────┐
│  Darvium                                         │
│  ┌───────────────────────────────────────────┐   │
│  │ AgentStep (KernelHandle 実装)              │   │
│  │  ┌─────────────────────────────────────┐  │   │
│  │  │ spawn_agent()                       │  │   │
│  │  │ send_to_agent()                     │  │   │
│  │  │ list_agents()                       │  │   │
│  │  │ kill_agent()                        │  │   │
│  │  │ task_post() / task_claim()          │  │   │
│  │  │ knowledge_add_entity() / query()    │  │   │
│  │  │ spawn_agent_checked()               │  │   │
│  │  └─────────────────────────────────────┘  │   │
│  └──────────┬────────────────────────────────┘   │
│             │ 同一プロセス (Arc<dyn KernelHandle>) │
│             ▼                                    │
│  ┌───────────────────────────────────────────┐   │
│  │ Declorch (DeclorchKernel)                 │   │
│  │ ・Agent ライフサイクル管理                 │   │
│  │ ・LLM 呼び出し (OpenAI/Anthropic/Groq等)  │   │
│  │ ・ツール実行                              │   │
│  │ ・セッション管理                          │   │
│  │ ・Workflow エンジン                       │   │
│  └───────────────────────────────────────────┘   │
└─────────────────────────────────────────────────┘
```

### Darvium での 3 つのユースケース

Darvium の `AgentStep` ノードが `Box<dyn KernelHandle>` を保持し、以下の 3 つのユースケースを実現します。

#### ユースケース 1: 利用可能なツール一覧を取得する

Darvium が Declorch に登録されている全ツール（Hands + ビルトイン）の一覧を取得し、LLM にツール定義として提供するために使用します。

```rust
use declorch_kernel::DeclorchKernel;
use std::sync::Arc;

// Darvium の AgentStep に必要な KernelHandle を保持
let kernel: Arc<DeclorchKernel> = Arc::new(
    DeclorchKernel::boot(None)?
);

// 全ツール定義を取得（ツール名・説明・JSON Schema）
let tools = kernel.get_tool_definitions();

// 結果例:
// [
//   ToolDefinition {
//     name: "file_read",
//     description: "Read a file from disk",
//     input_schema: { "type": "object", "properties": { "path": { "type": "string" } } }
//   },
//   ToolDefinition {
//     name: "web_search",
//     description: "Search the web for information",
//     input_schema: { "type": "object", "properties": { "query": { "type": "string" } } }
//   },
//   // ... 全 Hand + ビルトインツール
// ]
```

`get_tool_definitions()` の戻り値の型:

```rust
/// 各ツールの完全な定義。
pub struct ToolDefinition {
    pub name: String,                    // 一意識別子（ツール名）
    pub description: String,             // LLM 向けの説明文
    pub input_schema: serde_json::Value, // JSON Schema
}
```

#### ユースケース 2: Agent と Workflow を定義して即座に実行する

Agent マニフェスト（TOML）と Workflow 定義から Agent + Workflow を一括生成し実行します。

> **重要**: ワークフローの各ステップはエージェントを名前（`ByName`）または ID（`ById`）で参照します。
> ワークフロー実行時、エンジンは各ステップのエージェントを名前/ID で解決します。
> そのため **全ステップで参照されるエージェントは、実行前に `spawn` 済みでなければなりません。**
> ワークフローエンジンは自動的にエージェントを生成しません。
>
> 以下の例では 2 ステップのワークフローに対し、アナリスト用・ライター用の 2 つのエージェントを事前に spawn しています。

```rust
use declorch_kernel::workflow::{ErrorMode, StepAgent, StepMode, Workflow, WorkflowId, WorkflowStep};
use declorch_kernel::DeclorchKernel;
use declorch_runtime::kernel_handle::KernelHandle;
use std::sync::Arc;

let kernel = Arc::new(DeclorchKernel::boot(None)?);
kernel.set_self_handle();

// ====================================================================
// Step 1: ワークフローで使う全エージェントを spawn する
// ====================================================================
// 各ステップが参照するエージェントは、実行前にすべて登録されていなければならない。
// Darvium は KernelHandle トレイト経由で TOML 文字列から spawn する。
// ====================================================================

let (_analyst_id, _analyst_name) = KernelHandle::spawn_agent(
    &*kernel,
    r#"name = "analyst"
[model]
provider = "openai"
model = "gpt-4.1-nano"
max_tokens = 4096
temperature = 0.7
system_prompt = "You are an analyst. Research topics thoroughly."
"#,
    None,
)
.await?;

let (_writer_id, _writer_name) = KernelHandle::spawn_agent(
    &*kernel,
    r#"name = "writer"
[model]
provider = "openai"
model = "gpt-4.1-nano"
max_tokens = 4096
temperature = 0.7
system_prompt = "You are a writer. Summarize findings concisely."
"#,
    None,
)
.await?;

// ====================================================================
// Step 2: Workflow を定義する（ステップごとにエージェントを指定）
// ====================================================================
// 各 WorkflowStep.agent が ByName / ById でエージェントを参照する。
// このエージェントは Step 1 で既に spawn されていなければならない。
// ====================================================================

let workflow = Workflow {
    id: WorkflowId::new(),
    name: "research-and-summarize",
    description: "Research a topic, then summarize findings",
    steps: vec![
        WorkflowStep {
            name: "research".to_string(),
            agent: StepAgent::ByName {
                name: "analyst".to_string(),
            },
            prompt_template: "Research the following topic thoroughly: {{input}}".to_string(),
            mode: StepMode::Sequential,
            timeout_secs: 60,
            error_mode: ErrorMode::Fail,
            output_var: Some("research_results".to_string()),
        },
        WorkflowStep {
            name: "summarize".to_string(),
            agent: StepAgent::ByName {
                name: "writer".to_string(),
            },
            prompt_template: "Based on: {{research_results}}\n\nWrite a concise summary in markdown."
                .to_string(),
            mode: StepMode::Sequential,
            timeout_secs: 120,
            error_mode: ErrorMode::Skip,
            output_var: None,
        },
    ],
    created_at: chrono::Utc::now(),
};

// ====================================================================
// Step 3: Workflow を登録して実行する
// ====================================================================
// run_workflow() は全ステップの完了を待って結果を返す（非同期）。
// 内部で逐次的にステップを実行し、前ステップの出力が
// 次ステップの {{input}} として渡される。
// ====================================================================

let wf_id = kernel.register_workflow(workflow).await;
let (_run_id, output) = kernel
    .run_workflow(wf_id, "量子コンピューティングの最新動向".to_string())
    .await?;

println!("Workflow output:\n{}", output);
```

#### ユースケース 3: Agent を定義して即座にメッセージを送信する

Agent を spawn し、即座にメッセージを送信して応答を受け取ります。ワークフローを介さず単一 Agent と直接対話する場合に使用します。

```rust
use declorch_kernel::DeclorchKernel;
use declorch_runtime::agent_loop::AgentLoopResult;
use declorch_runtime::kernel_handle::KernelHandle;
use std::sync::Arc;

let kernel = Arc::new(DeclorchKernel::boot(None)?);

// Agent を TOML から spawn
// KernelHandle::spawn_agent() はタスクマネージャーを介して Agent を起動する
let (agent_id, _) = KernelHandle::spawn_agent(&*kernel, r#"
name = "chat-agent"
description = "General purpose chat agent"
author = "declorch"

[model]
provider = "openai"
model = "gpt-4.1-nano"
max_tokens = 4096
temperature = 0.7

system_prompt = "You are a helpful assistant."

[capabilities]
tools = ["*"]
"#, None).await?;

// メッセージを送信して応答を取得
let result: AgentLoopResult = kernel.send_message(
    agent_id,
    "量子コンピューティングについて簡単に説明してください。",
).await?;

println!("Response: {}", result.response);
println!("Cost: {:?}", result.cost_usd);
println!("Tokens: {}+{}",
    result.total_usage.input_tokens,
    result.total_usage.output_tokens);
```

### 戻り値の型: AgentLoopResult

```rust
pub struct AgentLoopResult {
    /// Agent からの最終応答テキスト。
    pub response: String,
    /// 全 LLM 呼び出しの総トークン使用量。
    pub total_usage: TokenUsage,
    /// ループの反復回数（ツール呼び出し → 再LLM呼び出しの回数）。
    pub iterations: u32,
    /// 推定コスト（USD）。
    pub cost_usd: Option<f64>,
    /// Agent が意図的に応答を抑制したか（NO_REPLY トークン）。
    pub silent: bool,
    /// 応答の配送指示。
    pub directives: ReplyDirectives,
}

pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
}

impl TokenUsage {
    pub fn total(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }
}
```

### AgentStep に必要な実装

Darvium が `KernelHandle` トレイトを実装するために、最低限以下を実装する必要があります：

```rust
use async_trait::async_trait;

#[async_trait]
pub trait KernelHandle: Send + Sync {
    /// Agent を TOML マニフェストから spawn する。
    async fn spawn_agent(
        &self,
        manifest_toml: &str,
        parent_id: Option<&str>,
    ) -> Result<(String, String), String>;

    /// 別の Agent にメッセージを送信し、応答を取得する。
    async fn send_to_agent(
        &self,
        agent_id: &str,
        message: &str,
    ) -> Result<String, String>;

    /// 稼働中の全 Agent の一覧を返す。
    fn list_agents(&self) -> Vec<AgentInfo>;

    /// Agent を強制停止する。
    fn kill_agent(&self, agent_id: &str) -> Result<(), String>;
}
```

上記 4 メソッドが最小要件です。それ以外のメソッド（`memory_store`、`task_post`、`knowledge_add_entity` 等）はデフォルト実装が備わっており、必要に応じてオーバーライドします。

完全なトレイト定義は [7. AgentStep (KernelHandle トレイト)](#7-agentstep-kernelhandle-トレイト) を参照してください。

---

## 2. DeclorchKernel の起動

> **重要**: `DeclorchKernel::boot()` は HTTP サーバーを起動しません。
> Declorch は **組込み用ライブラリ** であり、REST API を持ちません。
> Darvium は同一プロセス内でライブラリとして Kernel を呼び出します。
> （もし HTTP 経由で利用する必要がある場合、呼び出し元が独自に Axum 等で
> サーバーを立てて Kernel のメソッドをラップしてください。）

```rust
use declorch_kernel::DeclorchKernel;
use std::path::Path;

// 設定ファイルから起動（推奨）
let kernel = DeclorchKernel::boot(Some(Path::new("~/.declorch/config.toml")))?;

// またはデフォルト設定で起動
let kernel = DeclorchKernel::boot(None)?;
```

### KernelConfig（config.toml の主要項目）

```toml
# ホームディレクトリ（デフォルト: ~/.declorch）
home_dir = "~/.declorch"

# データベース用データディレクトリ（デフォルト: ~/.declorch/data）
data_dir = "~/.declorch/data"

# ログレベル（trace, debug, info, warn, error）
log_level = "info"

# API 待受アドレス（REST API 提供時）
api_listen = "0.0.0.0:4200"

# ネットワーク層の有効化
network_enabled = false

[default_model]
provider = "openai"
model = "gpt-4.1-nano"
```

---

## 3. Agent 定義 (TOML リファレンス)

Agent は TOML 形式の `AgentManifest` で定義します。
以下が全フィールドの網羅的なリファレンスです。

```toml
# ──────────────────────────────────────────────────
# Agent Manifest — 完全フィールドリファレンス
# ──────────────────────────────────────────────────

# 人間可読なエージェント名
name = "my-agent"

# セマンティックバージョン
version = "0.1.0"

# このエージェントの説明（LLM のシステムプロンプトには含まれない）
description = "A helpful research agent"

# 作成者識別子
author = "user@example.com"

# エージェントモジュールのパス
# "builtin:chat" = ビルトインのチャットモード（最も一般的）
# "builtin:agent" = ビルトインの自律エージェントモード
# *.wasm = WASM モジュール
# *.py = Python スクリプト
module = "builtin:chat"

# ── スケジュールモード ──
[schedule]
mode = "reactive"

# ── LLM モデル設定 ──
[model]
# LLM プロバイダ名
# 対応: "openai", "anthropic", "groq", "google", "deepseek", "openrouter", "ollama"
provider = "openai"

# モデル識別子（provider ごとに異なる）
# OpenAI: "gpt-4.1-nano", "gpt-4o", "gpt-4o-mini"
# Anthropic: "claude-sonnet-4-20250514", "claude-haiku-4-5-20251001"
# Groq: "llama-3.3-70b-versatile", "mixtral-8x7b-32768"
model = "gpt-4.1-nano"

# 最大出力トークン数（デフォルト: 4096）
max_tokens = 4096

# サンプリング温度（0.0 = 決定論的, 1.0 = 多様性）
temperature = 0.7

# システムプロンプト — エージェントの振る舞いを定義
system_prompt = """
You are a helpful AI agent.
Respond concisely and accurately.
"""

# API キーの環境変数名（省略時はグローバル設定を使用）
# api_key_env = "OPENAI_API_KEY"

# プロバイダのベースURL上書き（Ollama等のローカルLLM用）
# base_url = "http://localhost:11434/v1"

# ── フォールバックモデルチェーン（省略可）──
# プライマリモデルが失敗した場合に順に試行
# [[fallback_models]]
# provider = "groq"
# model = "mixtral-8x7b-32768"

# ── リソースクォータ ──
[resources]
# 最大 WASM メモリ（バイト） デフォルト: 256MB
max_memory_bytes = 268435456

# 最大 CPU 時間（ミリ秒/呼び出し） デフォルト: 30秒
max_cpu_time_ms = 30000

# 1分あたりの最大ツール呼び出し数 デフォルト: 60
max_tool_calls_per_minute = 60

# 1時間あたりの最大LLMトークン数（0 = 無制限）
max_llm_tokens_per_hour = 0

# 1時間あたりの最大ネットワーク転送量（バイト）（0 = 無制限）
max_network_bytes_per_hour = 104857600

# 1時間あたりの最大コスト（USD）（0.0 = 無制限）
max_cost_per_hour_usd = 0.0

# 1日あたりの最大コスト（USD）（0.0 = 無制限）
max_cost_per_day_usd = 0.0

# 1月あたりの最大コスト（USD）（0.0 = 無制限）
max_cost_per_month_usd = 0.0

# ── 優先度レベル ──
# low = 0, normal = 1（デフォルト）, high = 2, critical = 3
priority = "normal"

# ── 機能許可 (capabilities) ──
[capabilities]
# 許可するネットワークホスト（例: ["api.openai.com:443"]）
network = []

# 許可するツールID（["*"] = 全許可）
tools = ["*"]

# サブエージェント生成の許可
agent_spawn = false

# エージェントメッセージパターン（["*"] = 全エージェントと通信可）
agent_message = []

# シェルコマンドの許可（["*"] = 全許可）
shell = []

# ── ツールプロファイル（省略可）──
# 名前付きプリセット: minimal, coding, research, messaging, automation, full, custom
# profile = "full"

# ── ツール個別設定（省略可）──
# [tools.shell_exec]
# params = { timeout_ms = 30000 }

# ── スキル参照（省略可）──
# skills = ["file_operations", "web_search"]

# ── MCP サーバー許可リスト（省略可）──
# mcp_servers = ["filesystem", "github"]

# ── カスタムメタデータ（省略可）──
# [metadata]
# department = "engineering"
# project = "mycute"

# ── タグ（省略可）──
# tags = ["research", "automation"]

# ── モデルルーティング設定（省略可）──
# [routing]
# simple_model = "gpt-4o-mini"
# medium_model = "gpt-4.1-nano"
# complex_model = "gpt-4.1-nano"
# simple_threshold = 100
# complex_threshold = 500

# ── 自律エージェント設定（省略可）──
# [autonomous]
# quiet_hours = "0 22 * * *"
# max_iterations = 50
# max_restarts = 10
# heartbeat_interval_secs = 30
# heartbeat_channel = "telegram"

# ── 固定モデル上書き（省略可）──
# pinned_model = "gpt-4.1-nano"

# ── ワークスペースディレクトリ（省略可）──
# workspace = "/home/user/Documents"

# ── プライベート状態ディレクトリ（省略可）──
# state_dir = "~/.declorch/workspaces/my-agent"

# ── アイデンティティファイル自動生成（デフォルト: true）──
# generate_identity_files = true

# ── 実行ポリシー上書き（省略可）──
# exec_policy = "deny"

# ── ツール許可リスト（省略可）──
# tool_allowlist = ["file_read", "file_write"]

# ── ツール拒否リスト（許可リストの後に適用）──
# tool_blocklist = ["shell_exec"]

# ── context.md のキャッシュ（デフォルト: false）──
# cache_context = false

# ── 最大履歴メッセージ数上書き（省略可）──
# max_history_messages = 20
```

---

## 4. Agent の操作

### Agent の spawn

```rust
use declorch_types::agent::{AgentManifest, AgentId};

let manifest: AgentManifest = toml::from_str(r#"
    name = "my-agent"
    [model]
    provider = "openai"
    model = "gpt-4.1-nano"
"#)?;

let agent_id: AgentId = kernel.spawn_agent(manifest)?;
println!("Spawned agent: {}", agent_id);  // "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx"
```

### 親子関係のある spawn

```rust
let child_id = kernel.spawn_agent_with_parent(child_manifest, Some(parent_id), None)?;
```

### メッセージ送信（同期的に完了を待つ）

```rust
use declorch_runtime::agent_loop::AgentLoopResult;

let result: AgentLoopResult = kernel.send_message(agent_id, "Hello!").await?;
println!("Response: {}", result.response);       // LLM の応答テキスト
println!("Cost: {:?}", result.cost_usd);         // 推定コスト（USD）
println!("Iterations: {}", result.iterations);   // ループ反復回数
println!("Silent: {}", result.silent);            // NO_REPLY で抑制されたか
```

### メッセージ送信（ストリーミング）

```rust
use declorch_runtime::llm_driver::StreamEvent;
use tokio::sync::mpsc;

let (mut rx, join_handle) = kernel.send_message_streaming(
    agent_id,
    "Explain quantum computing",
    None,  // kernel_handle
    None,  // sender_id
    None,  // sender_name
    None,  // content_blocks
)?;

// ストリーミングイベントを処理
while let Some(event) = rx.recv().await {
    match event {
        StreamEvent::TextDelta { text } => {
            print!("{}", text);  // インクリメンタルなテキスト
        }
        StreamEvent::ToolUseStart { id, name } => {
            println!("\n[Tool: {}]", name);
        }
        StreamEvent::ToolInputDelta { text } => {
            print!("{}", text);  // ツール入力のJSON断片
        }
        StreamEvent::ToolUseEnd { id, name, input } => {
            println!("\n[Tool complete: {}]", name);
        }
        StreamEvent::ThinkingDelta { text } => {
            print!("[thinking] {}", text);  // 推論過程
        }
        StreamEvent::ContentComplete { stop_reason, usage } => {
            println!("\n[Done: {:?}] tokens: {}+{}",
                stop_reason, usage.input_tokens, usage.output_tokens);
        }
        StreamEvent::PhaseChange { phase } => {
            println!("\n[Phase: {}]", phase);  // ライフサイクルフェーズ
        }
    }
}

// 最終結果を取得
let final_result: AgentLoopResult = join_handle.await??;
```

### Agent の一覧取得

```rust
for info in kernel.list_agents() {
    println!("ID: {}, Name: {}, State: {}, Model: {}/{}",
        info.id, info.name, info.state, info.model_provider, info.model_name);
}
```

### Agent の停止

```rust
kernel.kill_agent(agent_id)?;
```

### Agent の再有効化（一時停止・クラッシュから復帰）

```rust
let agent_name = kernel.activate_agent(agent_id)?;
```

---

## 5. Workflow 定義 (TOML リファレンス)

Workflow は複数の Agent ステップをパイプライン実行するための宣言的定義です。

```toml
# ワークフロー名
name = "research-and-summarize"

# 説明
description = "Research a topic, then summarize findings"

# ── ステップ定義 ──
# 各ステップは順次実行される
[[steps]]

# ステップ名（ログ・表示用）
name = "research"

# 実行エージェントの指定方法:
#   ById: { id = "xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx" }
#   ByName: { name = "research-agent" }
[steps.agent]
name = "research-agent"

# プロンプトテンプレート
# {{input}}: 前ステップの出力（またはワークフロー入力）で置換される
# {{変数名}}: output_var で保存された変数で置換される
prompt_template = "Research the following topic thoroughly: {{input}}"

# 実行モード:
#   sequential: 前のステップ完了後に実行（デフォルト）
#   fan_out: 後続の FanOut ステップと並列実行
#   collect: 全 FanOut ステップの結果を集約
#   conditional: 前の出力が条件を含む場合のみ実行
#     { condition = "ERROR" }
#   loop: 条件を満たすまで繰り返し
#     { max_iterations = 5, until = "COMPLETE" }
mode = "sequential"

# ステップのタイムアウト（秒、デフォルト: 120）
timeout_secs = 60

# エラーハンドリングモード:
#   fail: エラー時にワークフロー中断（デフォルト）
#   skip: エラー時にこのステップをスキップして継続
#   retry: リトライ
#     { max_retries = 3 }
error_mode = "fail"

# 出力を保存する変数名（別ステップから {{変数名}} で参照可能）
output_var = "research_results"

# 次のステップ（上と同じ構造を繰り返す）
[[steps]]
name = "summarize"

[steps.agent]
name = "writer-agent"

prompt_template = """
Based on the following research, write a concise summary:

Research: {{research_results}}

Write in markdown format.
"""
mode = "sequential"
timeout_secs = 120
error_mode = "skip"
output_var = "summary"
```

---

## 6. Workflow の実行

```rust
use declorch_kernel::workflow::{Workflow, WorkflowRun, WorkflowRunId};

// Workflow をロード
let workflow: Workflow = serde_json::from_str(workflow_json)?;

// Workflow をカーネルに登録
kernel.workflows.register(workflow);

// Workflow を実行
let run_id: WorkflowRunId = kernel.workflows.run(
    workflow_id,
    "Initial input for the workflow",
    kernel.clone(),
).await?;

// 結果をポーリング
loop {
    let run: Option<WorkflowRun> = kernel.workflows.get_run(run_id);
    match run.state {
        WorkflowRunState::Completed => {
            println!("Output: {:?}", run.output);
            break;
        }
        WorkflowRunState::Failed => {
            eprintln!("Error: {:?}", run.error);
            break;
        }
        _ => tokio::time::sleep(Duration::from_millis(500)).await,
    }
}
```

### WorkflowRun の状態遷移

```
Pending → Running → Completed
                  ↘ Failed
```

---

## 7. AgentStep (KernelHandle トレイト)

Darvium が Declorch と接続するためには、`KernelHandle` トレイトを実装したオブジェクトを
カーネルに注入します。これにより Agent ループから Darvium 側の機能が呼び出せるようになります。

```rust
use async_trait::async_trait;

#[async_trait]
pub trait KernelHandle: Send + Sync {
    /// Agent を TOML マニフェストから spawn する。
    async fn spawn_agent(
        &self,
        manifest_toml: &str,
        parent_id: Option<&str>,
    ) -> Result<(String, String), String>;

    /// 別の Agent にメッセージを送信し、応答を取得する。
    async fn send_to_agent(
        &self,
        agent_id: &str,
        message: &str,
    ) -> Result<String, String>;

    /// 稼働中の全 Agent の一覧を返す。
    fn list_agents(&self) -> Vec<AgentInfo>;

    /// Agent を強制停止する。
    fn kill_agent(&self, agent_id: &str) -> Result<(), String>;

    /// 非アクティブな Agent を再有効化する。
    fn activate_agent(&self, agent_id: &str) -> Result<String, String>;

    /// Agent をクエリで検索する。
    fn find_agents(&self, query: &str) -> Vec<AgentInfo>;

    /// 共有タスクキューにタスクを投稿する。
    async fn task_post(
        &self,
        title: &str,
        description: &str,
        assigned_to: Option<&str>,
        created_by: Option<&str>,
    ) -> Result<String, String>;

    /// 次の未処理タスクを取得する。
    async fn task_claim(
        &self,
        agent_id: &str,
    ) -> Result<Option<serde_json::Value>, String>;

    /// タスクを完了としてマークする。
    async fn task_complete(&self, task_id: &str, result: &str) -> Result<(), String>;

    /// タスク一覧を取得する。
    async fn task_list(
        &self,
        status: Option<&str>,
    ) -> Result<Vec<serde_json::Value>, String>;

    /// カスタムイベントを発行する。
    async fn publish_event(
        &self,
        event_type: &str,
        payload: serde_json::Value,
    ) -> Result<(), String>;

    /// ナレッジグラフにエンティティを追加する。
    async fn knowledge_add_entity(
        &self,
        entity: declorch_types::memory::Entity,
    ) -> Result<String, String>;

    /// ナレッジグラフに関係を追加する。
    async fn knowledge_add_relation(
        &self,
        relation: declorch_types::memory::Relation,
    ) -> Result<String, String>;

    /// ナレッジグラフをクエリする。
    async fn knowledge_query(
        &self,
        pattern: declorch_types::memory::GraphPattern,
    ) -> Result<Vec<declorch_types::memory::GraphMatch>, String>;

    /// Agent の cron ジョブを作成する。
    async fn cron_create(
        &self,
        agent_id: &str,
        job_json: serde_json::Value,
    ) -> Result<String, String>;

    /// Agent の cron ジョブ一覧を取得する。
    async fn cron_list(
        &self,
        agent_id: &str,
    ) -> Result<Vec<serde_json::Value>, String>;

    /// cron ジョブをキャンセルする。
    async fn cron_cancel(&self, job_id: &str) -> Result<(), String>;

    /// ツールが承認を必要とするかチェックする。
    fn requires_approval(&self, tool_name: &str) -> bool;

    /// ツール実行の承認を要求する。
    async fn request_approval(
        &self,
        agent_id: &str,
        tool_name: &str,
        action_summary: &str,
    ) -> Result<bool, String>;

    /// チャンネルのデフォルト送信先を取得する。
    async fn get_channel_default_recipient(&self, channel: &str) -> Option<String>;

    /// チャンネルにメッセージを送信する。
    async fn send_channel_message(
        &self,
        channel: &str,
        recipient: &str,
        message: &str,
        thread_id: Option<&str>,
    ) -> Result<String, String>;

    /// チャンネルにメディアを送信する。
    async fn send_channel_media(
        &self,
        channel: &str,
        recipient: &str,
        media_type: &str,
        media_url: &str,
        caption: Option<&str>,
        filename: Option<&str>,
        thread_id: Option<&str>,
    ) -> Result<String, String>;

    /// チャンネルにローカルファイルを送信する。
    async fn send_channel_file_data(
        &self,
        channel: &str,
        recipient: &str,
        data: Vec<u8>,
        filename: &str,
        mime_type: &str,
        thread_id: Option<&str>,
    ) -> Result<String, String>;

    /// agent_send 時の子エージェント生成で Capability 継承検証を行う。
    async fn spawn_agent_checked(
        &self,
        manifest_toml: &str,
        parent_id: Option<&str>,
        parent_caps: &[declorch_types::capability::Capability],
    ) -> Result<(String, String), String>;

    /// 利用可能な Hands 一覧を取得する。
    async fn hand_list(&self) -> Result<Vec<serde_json::Value>, String>;

    /// Hand を TOML からインストールする。
    async fn hand_install(
        &self,
        toml_content: &str,
        skill_content: &str,
    ) -> Result<serde_json::Value, String>;

    /// Hand をアクティベートする。
    async fn hand_activate(
        &self,
        hand_id: &str,
        config: std::collections::HashMap<String, serde_json::Value>,
    ) -> Result<serde_json::Value, String>;

    /// Hand の状態を確認する。
    async fn hand_status(&self, hand_id: &str) -> Result<serde_json::Value, String>;

    /// Hand を非アクティベートする。
    async fn hand_deactivate(&self, instance_id: &str) -> Result<(), String>;

    /// A2A 外部 Agent 一覧を返す。
    fn list_a2a_agents(&self) -> Vec<(String, String)>;

    /// A2A 外部 Agent の URL を取得する。
    fn get_a2a_agent_url(&self, name: &str) -> Option<String>;
}
```

### AgentStep 実装の最小要件

Darvium が `KernelHandle` を実装する際、**最低限必須**のメソッド：

| メソッド | 必須 | 理由 |
|---------|------|------|
| `spawn_agent()` | **必須** | エージェントループ内で子エージェント生成に使用 |
| `send_to_agent()` | **必須** | agent_send ツールで他エージェントと通信 |
| `list_agents()` | **必須** | agent_list ツールでエージェント一覧表示 |
| `kill_agent()` | **必須** | エージェント停止に使用 |
| それ以外 | 任意 | デフォルト実装が `Err("not available")` を返す |

---

## 8. 戻り値の型完全リファレンス

### StreamEvent

```rust
/// ストリーミング応答のイベント。
pub enum StreamEvent {
    /// インクリメンタルなテキスト内容。
    TextDelta { text: String },
    /// ツール使用ブロック開始。
    ToolUseStart { id: String, name: String },
    /// ツール入力 JSON のインクリメンタルな断片。
    ToolInputDelta { text: String },
    /// ツール使用ブロック完了（パース済み JSON 入力）。
    ToolUseEnd { id: String, name: String, input: serde_json::Value },
    /// 思考/推論テキストのインクリメンタルな断片。
    ThinkingDelta { text: String },
    /// 応答全体の完了。
    ContentComplete { stop_reason: StopReason, usage: TokenUsage },
    /// Agent ライフサイクルフェーズ変更。
    PhaseChange { phase: String },
}
```

### StopReason

```rust
pub enum StopReason {
    EndTurn,      // モデルがターン終了
    ToolUse,      // モデルがツール使用を要求
    MaxTokens,    // トークン上限到達
    StopSequence, // 停止シーケンス到達
}
```

### Session

```rust
/// 会話セッション（メッセージ履歴のコンテナ）。
pub struct Session {
    pub id: SessionId,                     // セッションID (UUID)
    pub agent_id: AgentId,                 // 所有エージェントID
    pub messages: Vec<Message>,            // 会話メッセージ
    pub context_window_tokens: u64,        // コンテキストウィンドウの推定トークン数
    pub label: Option<String>,             // 人間可読なラベル
}
```

### Message

```rust
pub struct Message {
    pub msg_id: String,                          // UUID v4
    pub provider_msg_id: Option<String>,          // LLMプロバイダのメッセージID
    pub role: Role,                               // 送信者の役割
    pub content: MessageContent,                  // メッセージ内容
}

// 簡易生成ヘルパー:
// Message::system("...")
// Message::user("...")
// Message::assistant("...")
// Message::user_with_blocks(vec![...])
// Message::assistant_with_blocks(vec![...])
```

### Role

```rust
pub enum Role {
    System,
    User,
    Assistant,
}
```

### MessageContent

```rust
pub enum MessageContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}
```

### ContentBlock

```rust
pub enum ContentBlock {
    /// テキストブロック
    Text { text: String, provider_metadata: Option<serde_json::Value> },

    /// インラインBase64画像
    Image { media_type: String, data: String },
    // 対応メディアタイプ: image/png, image/jpeg, image/gif, image/webp
    // 最大サイズ: 5MB

    /// LLM からのツール使用要求
    ToolUse {
        id: String,
        name: String,
        input: serde_json::Value,
        provider_metadata: Option<serde_json::Value>,
    },

    /// ツール実行結果
    ToolResult {
        tool_use_id: String,
        tool_name: String,
        content: String,
        is_error: bool,
    },

    /// 拡張思考ブロック（推論過程）
    Thinking {
        thinking: String,
        signature: Option<String>,
        provider_metadata: Option<serde_json::Value>,
    },

    /// Anthropic Redacted Thinking
    RedactedThinking { data: String },

    /// 未認識のブロックタイプ
    Unknown,
}
```

### ToolDefinition

```rust
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}
```

### WorkflowRun

```rust
pub struct WorkflowRun {
    pub id: WorkflowRunId,
    pub workflow_id: WorkflowId,
    pub workflow_name: String,
    pub input: String,
    pub state: WorkflowRunState,
    pub step_results: Vec<StepResult>,
    pub output: Option<String>,
    pub error: Option<String>,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
}
```

### WorkflowRunState

```rust
pub enum WorkflowRunState {
    Pending,
    Running,
    Completed,
    Failed,
}
```

### StepResult

```rust
pub struct StepResult {
    pub step_name: String,
    pub agent_id: String,
    pub agent_name: String,
    pub output: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub duration_ms: u64,
}
```

### AgentInfo

```rust
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    pub state: String,           // "created", "running", "suspended", "terminated", "crashed"
    pub model_provider: String,
    pub model_name: String,
    pub description: String,
    pub tags: Vec<String>,
    pub tools: Vec<String>,
}
```

### AgentEntry（レジストリ内の完全なエージェント情報）

```rust
pub struct AgentEntry {
    pub id: AgentId,
    pub name: String,
    pub manifest: AgentManifest,
    pub state: AgentState,
    pub mode: AgentMode,
    pub created_at: DateTime<Utc>,
    pub last_active: DateTime<Utc>,
    pub parent: Option<AgentId>,
    pub children: Vec<AgentId>,
    pub session_id: SessionId,
    pub tags: Vec<String>,
    pub identity: AgentIdentity,
    pub onboarding_completed: bool,
    pub onboarding_completed_at: Option<DateTime<Utc>>,
}
```

### UsageRecord / UsageSummary

```rust
/// 単一の LLM 呼び出し使用量記録。
pub struct UsageRecord {
    pub agent_id: AgentId,
    pub model: String,
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cost_usd: f64,
    pub tool_calls: u32,
}

/// 期間集計された使用量サマリー。
pub struct UsageSummary {
    pub total_input_tokens: u64,
    pub total_output_tokens: u64,
    pub total_cost_usd: f64,
    pub call_count: u64,
    pub total_tool_calls: u64,
}
```

---

## 9. イベントシステム

Declorch はイベント駆動アーキテクチャを採用しています。
全イベントは `EventBus` を介して配信されます。

```rust
pub struct Event {
    pub id: EventId,
    pub source: AgentId,
    pub target: EventTarget,
    pub payload: EventPayload,
    pub timestamp: DateTime<Utc>,
    pub correlation_id: Option<EventId>,
    pub ttl: Option<Duration>,
}

pub enum EventTarget {
    Agent(AgentId),    // 特定 Agent 宛て
    Broadcast,         // 全 Agent へブロードキャスト
    Pattern(String),   // パターンマッチ（タグベースなど）
    System,            // Kernel/システム宛て
}

pub enum EventPayload {
    Message(AgentMessage),                // Agent 間メッセージ
    ToolResult(ToolOutput),               // ツール実行結果
    Lifecycle(LifecycleEvent),            // Agent ライフサイクル
    Network(NetworkEvent),                // ネットワークイベント
    System(SystemEvent),                  // システムイベント
    Custom(Vec<u8>),                      // ユーザー定義ペイロード
}
```

---

## 10. エラー型

```rust
pub enum DeclorchError {
    AgentNotFound(String),
    AgentAlreadyExists(String),
    CapabilityDenied(String),
    QuotaExceeded(String),
    InvalidState { current: String, operation: String },
    SessionNotFound(String),
    Memory(String),
    ToolExecution { tool_id: String, reason: String },
    LlmDriver(String),
    Config(String),
    ManifestParse(String),
    Sandbox(String),
    Network(String),
    Serialization(String),
    MaxIterationsExceeded(u32),
    ShuttingDown,
    Io(std::io::Error),
    Internal(String),
    AuthDenied(String),
    MeteringError(String),
    InvalidInput(String),
}

pub type DeclorchResult<T> = Result<T, DeclorchError>;
```

---

## 11. パフォーマンス特性

| 操作 | レイテンシ | 備考 |
|------|-----------|------|
| `boot()` | 50-200ms | config 読み込み、初期化 |
| `spawn_agent()` | <1ms | メモリ上のレジストリ登録のみ |
| `send_message()` (同期待機) | LLM時間 + α | 大部分は LLM 応答時間 |
| `send_message_streaming()` | 初回TTFB<1s | 最初の TextDelta が届くまでの時間 |
| `kill_agent()` | <1ms | レジストリからの削除 |
| `list_agents()` | <1ms | DashMap からの読み出し |
| Workflow 1ステップ | LLM時間 + α | 逐次実行の場合は線形増加 |

- LLM 呼び出しは **タイムアウト付き**（デフォルト 120秒/ステップ）
- **per-Agent ロック**で同一 Agent への同時メッセージ送信をシリアライズ
- WASM サンドボックスは **256MB メモリ制限** + **30秒 CPU 制限**
- Tool 呼び出しは **毎分60回** 制限（デフォルト）

---

## ライセンス

Declorch is open-source software. See the LICENSE file for details.
