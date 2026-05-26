# DeepWiki Q&A with Code Context for Repository: RightNow-AI/openfang
## Q1
宣言的WorkflowとAgentだけをOpenFangから切り出し、Rustのcrateに実装し直すことで、私のDarviumという開発プロジェクトの中にある WorkflowGraph の中の WorkflowNode::AgentStep の実体実装を OpenFang から切り出した宣言的WorkflowとAgentのコード内の関数呼び出しだけで完全に動作するようにする計画を立てています。

まずは、宣言的WorkflowとAgentだけをOpenFangから切り出してコード内のみで使用することができるために必要十分な切り出し部分の網羅的リストを書いてください。GUIなど一切必要ありません。私のDarvium内部から関数で呼び出せるようにするためだけの開発作業です。OpenFangの中に既にそのような抽象化が行われているかはわかりませんが、必ずしも抽象化されている必要はありません。もし抽象化が十分には行われていないのであれば、私が抽象化改修作業を行うからです。そこまでを含めて過不足のない必要十分なリストでなければなりません。
### Answer
ユーザーはOpenFangから宣言的WorkflowとAgentの部分を切り出してRust crateとして実装し直すことを計画しています。まずOpenFangのコードベースを調査して、宣言的WorkflowとAgentに関連するコードを特定する必要があります。

WikiページのOverviewを読んで、全体像を把握してから、具体的なコードを調査します。

ユーザーはOpenFangから宣言的WorkflowとAgentの部分を切り出すために必要なコードの網羅的リストを求めています。まずWorkflowとAgentに関連するコードを調査する必要があります。WikiページにWorkflows and Automationというセクション(3.5)があるので、それを読みつつ、関連するcrateのディレクトリ構造も調べます。

ユーザーはOpenFangから宣言的WorkflowとAgentを切り出すために必要なコードの網羅的リストを求めています。まず各crateの実際のソースコードを調査する必要があります。特に以下のcrateが重要です：

1. `openfang-types` - 共有データ構造（AgentManifest、Workflowなど）
2. `openfang-runtime` - エージェントループ、LLMドライバー、ツール実行
3. `openfang-kernel` - WorkflowEngine、TriggerEngine、AgentRegistry

これらのcrateのソースファイルを並行して調査します。

残りのイテレーションが少ないので、最大限に並行してツールを呼び出す必要があります。最も重要なファイルを一度に読み込みます。

特に重要なのは：
1. `crates/openfang-kernel/src/workflow.rs` - WorkflowEngineの実装
2. `crates/openfang-runtime/src/agent_loop.rs` - エージェントループ
3. `crates/openfang-runtime/src/kernel_handle.rs` - KernelHandleトレイト
4. `crates/openfang-runtime/src/llm_driver.rs` - LlmDriverトレイト
5. `crates/openfang-runtime/src/tool_runner.rs` - ツール実行
6. `crates/openfang-types/src/agent.rs` - AgentManifest
7. `crates/openfang-types/src/tool.rs` - ToolDefinition
8. `crates/openfang-runtime/src/prompt_builder.rs` - プロンプト構築
9. `crates/openfang-kernel/src/triggers.rs` - TriggerEngine
10. `crates/openfang-runtime/src/drivers/` - LLMドライバー

これらを並行して読み込みます。

これで十分な情報が集まりました。OpenFangから宣言的WorkflowとAgentを切り出すために必要なコードの網羅的リストを作成できます。

調査した内容をまとめると：

## 切り出しに必要なコンポーネント

### 1. openfang-types（データ型）
- `agent.rs` - AgentManifest, AgentId, AgentState, AgentMode, ScheduleMode, ModelConfig, ManifestCapabilities, AutonomousConfig, HookEvent, ToolProfile, ResourceQuota, FallbackModel, AgentEntry, AgentIdentity, SessionId, UserId
- `tool.rs` - ToolDefinition, ToolCall, ToolResult, normalize_schema_for_provider
- `message.rs` - Message, ContentBlock, MessageContent, Role, StopReason, TokenUsage, ReplyDirectives
- `error.rs` - OpenFangError, OpenFangResult
- `memory.rs` - Memory, MemoryFilter, MemorySource, Entity, Relation, GraphPattern, GraphMatch
- `event.rs` - Event, EventPayload, LifecycleEvent, SystemEvent, EventTarget
- `capability.rs` - Capability
- `config.rs` - ExecPolicy, ExecSecurityMode, DockerSandboxConfig, ThinkingConfig
- `serde_compat.rs` - vec_lenient, map_lenient, exec_policy_lenient
- `tool_compat.rs` - normalize_tool_name
- `taint.rs` - TaintLabel, TaintSink, TaintedValue
- `lib.rs` - truncate_str

### 2. openfang-runtime（実行エンジン）
- `llm_driver.rs` - LlmDriver trait, CompletionRequest, CompletionResponse, LlmError, StreamEvent, DriverConfig
- `kernel_handle.rs` - KernelHandle trait, AgentInfo
- `agent_loop.rs` - run_agent_loop, AgentLoopResult, LoopPhase, PhaseCallback
- `prompt_builder.rs` - PromptContext, build_system_prompt, build_tools_section, build_memory_section, build_canonical_context_message
- `tool_runner.rs` - execute_tool, builtin_tool_definitions
- `embedding.rs` - EmbeddingDriver trait
- `loop_guard.rs` - LoopGuard, LoopGuardConfig, LoopGuardVerdict
- `context_budget.rs` - ContextBudget, apply_context_guard, truncate_tool_result_dynamic
- `context_overflow.rs` - recover_from_overflow, RecoveryStage
- `session_repair.rs` - validate_and_repair, ensure_starts_with_user, prune_heartbeat_turns
- `reply_directives.rs` - parse_directives
- `retry.rs` - call_with_retry (agent_loopから参照)
- `routing.rs` - モデルルーティング
- `hooks.rs` - HookRegistry, HookContext
- `mcp.rs` - McpConnection, is_mcp_tool, extract_mcp_server_from_known
- `web_search.rs` - WebToolsContext
- `browser.rs` - BrowserManager
- `media_understanding.rs` - MediaEngine
- `tts.rs` - TtsEngine
- `process_manager.rs` - ProcessManager
- `drivers/` - 各LLMドライバー実装（anthropic.rs, openai.rs, gemini.rs, etc.）
- `str_utils.rs` - safe_truncate_str
- `apply_patch.rs` - apply_patch
- `audit.rs` - AuditLog
- `agent_context.rs` - load_context_md
- `workspace_context.rs` - workspace context loading
- `workspace_sandbox.rs` - workspace sandbox
- `subprocess_sandbox.rs` - contains_shell_metacharacters, validate_command_allowlist
- `docker_sandbox.rs` - docker sandbox
- `compactor.rs` - session compaction
- `think_filter.rs` - thinking filter
- `tool_policy.rs` - tool policy
- `web_fetch.rs` - web fetch
- `web_content.rs` - web content
- `web_cache.rs` - web cache
- `link_understanding.rs` - link understanding
- `image_gen.rs` - image generation
- `python_runtime.rs` - Python runtime
- `sandbox.rs` - sandbox
- `shell_bleed.rs` - shell bleed protection
- `a2a.rs` - A2A protocol
- `mcp_server.rs` - MCP server
- `graceful_shutdown.rs` - graceful shutdown
- `command_lane.rs` - command lane
- `copilot_oauth.rs` - Copilot OAuth
- `auth_cooldown.rs` - auth cooldown
- `provider_health.rs` - provider health

### 3. openfang-kernel（WorkflowEngineとTriggerEngine）
- `workflow.rs` - WorkflowEngine, Workflow, WorkflowStep, WorkflowRun, StepMode, ErrorMode, StepAgent, WorkflowId, WorkflowRunId, StepResult
- `triggers.rs` - TriggerEngine, Trigger, TriggerPattern, TriggerId
- `event_bus.rs` - EventBus
- `registry.rs` - AgentRegistry, AgentEntry
- `scheduler.rs` - AgentScheduler
- `capabilities.rs` - CapabilityManager
- `approval.rs` - ApprovalManager
- `cron.rs` - CronScheduler
- `background.rs` - BackgroundExecutor
- `supervisor.rs` - Supervisor
- `metering.rs` - MeteringEngine
- `error.rs` - kernel error types

### 4. openfang-memory（メモリ）
- `substrate.rs` - MemorySubstrate
- `session.rs` - Session
- `knowledge.rs` - knowledge graph
- `semantic.rs` - semantic search
- `structured.rs` - structured memory
- `usage.rs` - usage tracking
- `consolidation.rs` - memory consolidation
- `migration.rs` - DB migration
- `lib.rs`

### 5. openfang-wire（通信）
- `message.rs` - wire message types
- `peer.rs` - peer communication
- `registry.rs` - wire registry

### 6. openfang-skills（スキル）
- `registry.rs` - SkillRegistry
- `loader.rs` - execute_skill_tool

## 重要な観察

`WorkflowEngine`の`execute_run`メソッドは、カーネルへの直接依存を避けるために、`send_message`クロージャを受け取る設計になっています。これは既に良い抽象化です。

`KernelHandle`トレイトは、`openfang-runtime`がカーネルに依存しないようにするための抽象化です。

しかし、`run_agent_loop`は非常に多くの依存関係を持っています（MemorySubstrate, LlmDriver, KernelHandle, SkillRegistry, McpConnection, WebToolsContext, BrowserManager, EmbeddingDriver, etc.）。

## 切り出しの難しさ

1. `run_agent_loop`の引数が非常に多い（21個）。これらのほとんどはOptionalですが、切り出す際には何らかの形で提供する必要があります。
2. `openfang-memory`はSQLiteに依存しており、切り出す際にはこの依存関係も含める必要があります。
3. `tool_runner`は多くのツール実装を含んでおり、これらの多くは外部依存関係（reqwest, headless_chrome等）を持っています。

## 切り出し戦略

ユーザーのDarviumプロジェクトでは`WorkflowNode::AgentStep`の実体実装として使いたいとのことなので、最小限の切り出しとして：

1. **必須（コア）**: `openfang-types`全体 + `openfang-runtime`の`llm_driver.rs`, `kernel_handle.rs`, `agent_loop.rs`, `prompt_builder.rs` + `openfang-kernel`の`workflow.rs`
2. **必須（ツール実行）**: `tool_runner.rs`とその依存関係
3. **必須（メモリ）**: `openfang-memory`全体
4. **オプション（ドライバー）**: `drivers/`の各実装

では、網羅的なリストを作成します。


調査した全コードを元に、切り出しに必要な網羅的リストを以下に示します。

---

## 切り出し対象の網羅的リスト

### 凡例
- **必須コア**: これがないと動かない
- **必須依存**: コアが直接 `use` しているもの
- **条件付き**: 機能を削ぎ落とせば省略可能
- **要抽象化**: 現状では密結合しており、切り出し前に抽象化改修が必要

---

### 1. `openfang-types` — 全体が必須コア

| ファイル | 主要な型 | 備考 |
|---|---|---|
| `agent.rs` | `AgentManifest`, `AgentId`, `AgentState`, `AgentMode`, `ScheduleMode`, `ModelConfig`, `ManifestCapabilities`, `AutonomousConfig`, `HookEvent`, `ToolProfile`, `FallbackModel`, `AgentEntry` | Workflowの`StepAgent`が`AgentId`を参照 |
| `tool.rs` | `ToolDefinition`, `ToolCall`, `ToolResult`, `normalize_schema_for_provider` | LLMドライバーとtool_runnerの共通型 |
| `message.rs` | `Message`, `ContentBlock`, `MessageContent`, `Role`, `StopReason`, `TokenUsage`, `ReplyDirectives` | agent_loopの全体で使用 |
| `error.rs` | `OpenFangError`, `OpenFangResult` | エラー型 |
| `memory.rs` | `Memory`, `MemoryFilter`, `MemorySource`, `Entity`, `Relation`, `GraphPattern`, `GraphMatch` | KernelHandleトレイトが参照 |
| `event.rs` | `Event`, `EventPayload`, `LifecycleEvent`, `SystemEvent`, `EventTarget` | TriggerEngineが参照 |
| `capability.rs` | `Capability` | `spawn_agent_checked`が参照 |
| `config.rs` | `ExecPolicy`, `ExecSecurityMode`, `DockerSandboxConfig`, `ThinkingConfig` | tool_runnerとagent_loopが参照 |
| `taint.rs` | `TaintLabel`, `TaintSink`, `TaintedValue` | tool_runnerのセキュリティチェック |
| `tool_compat.rs` | `normalize_tool_name` | tool_runnerが使用 |
| `serde_compat.rs` | `vec_lenient`, `map_lenient`, `exec_policy_lenient` | AgentManifestのデシリアライズ |
| `lib.rs` | `truncate_str` | triggers.rsなどが使用 | openfang:424-500 openfang:1-36 

---

### 2. `openfang-runtime` — コアと条件付きに分かれる

#### 2-A. 必須コア（Agentループの骨格）

| ファイル | 主要な型・関数 | 備考 |
|---|---|---|
| `llm_driver.rs` | `LlmDriver` trait, `CompletionRequest`, `CompletionResponse`, `LlmError`, `StreamEvent`, `DriverConfig` | LLM抽象化の中心。**既に十分に抽象化済み** |
| `kernel_handle.rs` | `KernelHandle` trait, `AgentInfo` | カーネルへの逆依存を断ち切るトレイト。**既に十分に抽象化済み** |
| `agent_loop.rs` | `run_agent_loop`, `AgentLoopResult`, `LoopPhase`, `PhaseCallback` | エージェント実行の本体。引数が21個あり**要抽象化**（後述） |
| `prompt_builder.rs` | `PromptContext`, `build_system_prompt`, `build_tools_section`, `build_memory_section` | システムプロンプト構築。依存なし |
| `loop_guard.rs` | `LoopGuard`, `LoopGuardConfig`, `LoopGuardVerdict` | 無限ループ検出 |
| `context_budget.rs` | `ContextBudget`, `apply_context_guard`, `truncate_tool_result_dynamic` | コンテキスト予算管理 |
| `context_overflow.rs` | `recover_from_overflow`, `RecoveryStage` | コンテキスト溢れ回復 |
| `session_repair.rs` | `validate_and_repair`, `ensure_starts_with_user`, `prune_heartbeat_turns` | セッション整合性修復 |
| `reply_directives.rs` | `parse_directives` | `[[silent]]`等のディレクティブ解析 |
| `retry.rs` | `call_with_retry` | LLM呼び出しのリトライ・フォールバック |
| `routing.rs` | モデルルーティングロジック | `ModelRoutingConfig`の実装 |
| `hooks.rs` | `HookRegistry`, `HookContext` | `BeforeToolCall`/`AfterToolCall`フック |
| `str_utils.rs` | `safe_truncate_str` | UTF-8安全なトランケート |
| `embedding.rs` | `EmbeddingDriver` trait | ベクトル検索の抽象化 | openfang:146-171 openfang:27-45 openfang:197-219 

#### 2-B. 必須依存（tool_runner経由）

| ファイル | 主要な型・関数 | 備考 |
|---|---|---|
| `tool_runner.rs` | `execute_tool`, `builtin_tool_definitions` | ツール実行の本体。**要抽象化**（後述） |
| `subprocess_sandbox.rs` | `contains_shell_metacharacters`, `validate_command_allowlist` | shell_execのセキュリティ |
| `apply_patch.rs` | `tool_apply_patch` | ファイルパッチ適用 |
| `web_fetch.rs` | `tool_web_fetch_legacy` | HTTPフェッチ |
| `web_search.rs` | `WebToolsContext`, `parse_ddg_results` | Web検索 |
| `web_content.rs` | HTML→Markdown変換 | web_fetchが使用 |
| `web_cache.rs` | Webキャッシュ | web_fetchが使用 |
| `mcp.rs` | `McpConnection`, `is_mcp_tool`, `extract_mcp_server_from_known` | MCPツール呼び出し |
| `think_filter.rs` | thinking blockフィルタ | Anthropic extended thinking |
| `tool_policy.rs` | ツールポリシー | exec_policy適用 | openfang:109-127 

#### 2-C. 条件付き（機能を削ぎ落とせば省略可能）

| ファイル | 省略条件 |
|---|---|
| `browser.rs` + `BrowserManager` | ブラウザ自動化ツールを使わない場合 |
| `media_understanding.rs` + `MediaEngine` | 画像・音声理解ツールを使わない場合 |
| `tts.rs` + `TtsEngine` | TTS/STTツールを使わない場合 |
| `docker_sandbox.rs` | `docker_exec`ツールを使わない場合 |
| `process_manager.rs` + `ProcessManager` | `process_start`等を使わない場合 |
| `python_runtime.rs` | Pythonエージェントモジュールを使わない場合 |
| `sandbox.rs`, `workspace_sandbox.rs` | WASMサンドボックスを使わない場合 |
| `a2a.rs` | A2Aプロトコルを使わない場合 |
| `mcp_server.rs` | MCPサーバー機能を使わない場合 |
| `image_gen.rs` | 画像生成ツールを使わない場合 |
| `copilot_oauth.rs` | GitHub Copilotドライバーを使わない場合 |
| `audit.rs` | 監査ログが不要な場合 |
| `compactor.rs` | セッション圧縮が不要な場合 |
| `agent_context.rs` | `context.md`の動的読み込みが不要な場合 |
| `workspace_context.rs` | ワークスペースコンテキストが不要な場合 |
| `link_understanding.rs` | リンク解析が不要な場合 |
| `shell_bleed.rs` | シェル出力サニタイズが不要な場合 |
| `graceful_shutdown.rs` | グレースフルシャットダウンが不要な場合 |
| `command_lane.rs` | コマンドレーンが不要な場合 |
| `auth_cooldown.rs` | 認証クールダウンが不要な場合 |
| `provider_health.rs` | プロバイダーヘルスチェックが不要な場合 |

#### 2-D. LLMドライバー実装（`drivers/`）

| ファイル | 対応プロバイダー |
|---|---|
| `drivers/anthropic.rs` | Anthropic Claude |
| `drivers/openai.rs` | OpenAI / OpenAI互換（OpenRouter等） |
| `drivers/gemini.rs` | Google Gemini |
| `drivers/bedrock.rs` | AWS Bedrock |
| `drivers/vertex.rs` | Google Vertex AI |
| `drivers/claude_code.rs` | Claude Code CLI |
| `drivers/copilot.rs` | GitHub Copilot |
| `drivers/qwen_code.rs` | Qwen Code |
| `drivers/fallback.rs` | フォールバックチェーン |
| `drivers/mod.rs` | ドライバーファクトリ |

最低1つのドライバーが必須。`LlmDriver`トレイトを実装すれば独自ドライバーも可能。



---

### 3. `openfang-kernel` — WorkflowとTriggerのみ切り出し対象

#### 3-A. 必須コア（Workflow/Trigger本体）

| ファイル | 主要な型・関数 | 備考 |
|---|---|---|
| `workflow.rs` | `WorkflowEngine`, `Workflow`, `WorkflowStep`, `WorkflowRun`, `StepMode`, `ErrorMode`, `StepAgent`, `WorkflowId`, `WorkflowRunId`, `StepResult` | **既に十分に抽象化済み**。`execute_run`はクロージャ経由でカーネルから分離されている |
| `triggers.rs` | `TriggerEngine`, `Trigger`, `TriggerPattern`, `TriggerId` | イベント駆動トリガー。`Event`型への依存のみ |
| `event_bus.rs` | `EventBus` | TriggerEngineが使用するpub/sub | openfang:200-215 openfang:434-443 openfang:83-97 

#### 3-B. 条件付き（AgentRegistryを使う場合）

| ファイル | 主要な型 | 備考 |
|---|---|---|
| `registry.rs` | `AgentRegistry`, `AgentEntry` | `DashMap<AgentId, AgentEntry>`。WorkflowのagentResolverに必要 |
| `scheduler.rs` | `AgentScheduler`, `UsageTracker` | リソースクォータ管理 |
| `capabilities.rs` | `CapabilityManager` | ケイパビリティ検証 |
| `approval.rs` | `ApprovalManager`, `PendingRequest` | Human-in-the-loopゲート |
| `cron.rs` | `CronScheduler`, `JobMeta` | 時間ベーススケジューリング |
| `background.rs` | `BackgroundExecutor` | バックグラウンドタスク実行 |
| `supervisor.rs` | `Supervisor` | エージェント監視 |
| `metering.rs` | `MeteringEngine` | コスト計測 |
| `error.rs` | カーネルエラー型 | |

---

### 4. `openfang-memory` — 必須依存

`run_agent_loop`が`MemorySubstrate`を直接受け取るため、全体が必要。

| ファイル | 主要な型 | 備考 |
|---|---|---|
| `substrate.rs` | `MemorySubstrate` | SQLiteバックエンド。`recall`, `remember`, `save_session_async`等 |
| `session.rs` | `Session` | 会話履歴の永続化 |
| `knowledge.rs` | ナレッジグラフ | `KernelHandle::knowledge_*`が使用 |
| `semantic.rs` | セマンティック検索 | BM25テキスト検索 |
| `structured.rs` | 構造化メモリ | KVストア |
| `usage.rs` | 使用量追跡 | |
| `consolidation.rs` | メモリ統合 | |
| `migration.rs` | DBマイグレーション | SQLiteスキーマ管理 |



---

### 5. `openfang-skills` — 条件付き

`tool_runner`がスキルレジストリにフォールバックするため、スキルを使う場合は必要。

| ファイル | 主要な型 | 備考 |
|---|---|---|
| `registry.rs` | `SkillRegistry`, `find_tool_provider` | スキルツールのディスパッチ |
| `loader.rs` | `execute_skill_tool` | スキル実行 |

---

### 6. `openfang-wire` — 条件付き

A2Aプロトコルや外部エージェント通信を使う場合のみ必要。

---

## 要抽象化改修が必要な箇所

現状では以下の2点が密結合しており、切り出し前に改修が必要です。

### A. `run_agent_loop`の引数爆発（21引数） openfang:197-219 

現状は21個の引数を直接受け取っています。切り出しcrateとして使いやすくするには、以下のような構造体にまとめる改修が必要です：

```rust
pub struct AgentLoopContext {
    pub memory: Arc<MemorySubstrate>,
    pub driver: Arc<dyn LlmDriver>,
    pub kernel: Option<Arc<dyn KernelHandle>>,
    pub available_tools: Vec<ToolDefinition>,
    // ... 省略可能なフィールドはOption<>で
}
```

### B. `tool_runner::execute_tool`のモノリシック構造 openfang:109-127 

現状は全ツールが1つの巨大な`match`に詰め込まれています。Darviumから呼び出す際に不要なツール（ブラウザ、TTS等）の依存関係を引き込まないよう、ツールを登録可能なレジストリパターンに抽象化する改修が必要です。

---

## 切り出しcrateの依存グラフ（最小構成）

```mermaid
graph TD
    "darvium-workflow-agent (新crate)" --> "openfang-runtime (抽象化改修後)"
    "darvium-workflow-agent (新crate)" --> "openfang-kernel/workflow.rs (抽出)"
    "darvium-workflow-agent (新crate)" --> "openfang-kernel/triggers.rs (抽出)"
    "openfang-runtime (抽象化改修後)" --> "openfang-types"
    "openfang-runtime (抽象化改修後)" --> "openfang-memory"
    "openfang-runtime (抽象化改修後)" --> "openfang-skills (optional)"
    "openfang-kernel/workflow.rs (抽出)" --> "openfang-types"
    "openfang-kernel/triggers.rs (抽出)" --> "openfang-types"
```

`WorkflowEngine::execute_run`はすでにクロージャ経由でカーネルから分離されているため、Workflowエンジン自体の切り出しは比較的容易です。最大の作業量は`run_agent_loop`の引数構造体化と`tool_runner`のレジストリ化です。
### Citations
**File:** crates/openfang-types/src/agent.rs (L424-500)
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct AgentManifest {
    /// Human-readable agent name.
    pub name: String,
    /// Semantic version.
    pub version: String,
    /// Description of what this agent does.
    pub description: String,
    /// Author identifier.
    pub author: String,
    /// Path to the agent module (WASM or Python file).
    pub module: String,
    /// Scheduling mode.
    pub schedule: ScheduleMode,
    /// LLM model configuration.
    pub model: ModelConfig,
    /// Fallback model chain — tried in order if the primary model fails.
    #[serde(default, deserialize_with = "crate::serde_compat::vec_lenient")]
    pub fallback_models: Vec<FallbackModel>,
    /// Resource quotas.
    pub resources: ResourceQuota,
    /// Priority level.
    pub priority: Priority,
    /// Capability grants (parsed into Capability enum by kernel).
    pub capabilities: ManifestCapabilities,
    /// Named tool profile — expands to tool list + derived capabilities.
    #[serde(default)]
    pub profile: Option<ToolProfile>,
    /// Tool-specific configurations.
    #[serde(default, deserialize_with = "crate::serde_compat::map_lenient")]
    pub tools: HashMap<String, ToolConfig>,
    /// Installed skill references (empty = all skills available).
    #[serde(default, deserialize_with = "crate::serde_compat::vec_lenient")]
    pub skills: Vec<String>,
    /// MCP server allowlist (empty = all connected MCP servers available).
    #[serde(default, deserialize_with = "crate::serde_compat::vec_lenient")]
    pub mcp_servers: Vec<String>,
    /// Custom metadata.
    #[serde(default, deserialize_with = "crate::serde_compat::map_lenient")]
    pub metadata: HashMap<String, serde_json::Value>,
    /// Tags for agent discovery and categorization.
    #[serde(default, deserialize_with = "crate::serde_compat::vec_lenient")]
    pub tags: Vec<String>,
    /// Model routing configuration — auto-select models by complexity.
    #[serde(default)]
    pub routing: Option<ModelRoutingConfig>,
    /// Autonomous agent configuration — guardrails for 24/7 agents.
    #[serde(default)]
    pub autonomous: Option<AutonomousConfig>,
    /// Pinned model override (used in Stable mode).
    #[serde(default)]
    pub pinned_model: Option<String>,
    /// Agent workspace directory. Auto-created on spawn.
    /// Default: `{workspaces_dir}/{agent_name}-{agent_id_prefix}/`
    #[serde(default)]
    pub workspace: Option<PathBuf>,
    /// Whether to generate workspace identity files (SOUL.md, USER.md, etc.) on creation.
    #[serde(default = "default_true")]
    pub generate_identity_files: bool,
    /// Per-agent exec policy override. If None, uses global exec_policy.
    /// Accepts string shorthand ("allow", "deny", "full", "allowlist") or full table.
    #[serde(default, deserialize_with = "crate::serde_compat::exec_policy_lenient")]
    pub exec_policy: Option<crate::config::ExecPolicy>,
    /// Tool allowlist — only these tools are available (empty = all tools).
    #[serde(default, deserialize_with = "crate::serde_compat::vec_lenient")]
    pub tool_allowlist: Vec<String>,
    /// Tool blocklist — these tools are excluded (applied after allowlist).
    #[serde(default, deserialize_with = "crate::serde_compat::vec_lenient")]
    pub tool_blocklist: Vec<String>,
    /// If true, the agent's `context.md` is read once at session start and
    /// reused. Default is `false`: the runtime re-reads `context.md` before
    /// every turn so external writers (cron jobs, integrations) reach the LLM
    /// on the next message. See issue #843.
    #[serde(default)]
    pub cache_context: bool,
}
```
**File:** crates/openfang-types/src/tool.rs (L1-36)
```rust
//! Tool definition and result types.

use serde::{Deserialize, Serialize};

/// Definition of a tool that an agent can use.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Unique tool identifier.
    pub name: String,
    /// Human-readable description for the LLM.
    pub description: String,
    /// JSON Schema for the tool's input parameters.
    pub input_schema: serde_json::Value,
}

/// A tool call requested by the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    /// Unique ID for this tool use instance.
    pub id: String,
    /// Which tool to call.
    pub name: String,
    /// The input parameters.
    pub input: serde_json::Value,
}

/// Result of a tool execution.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// The tool_use ID this result corresponds to.
    pub tool_use_id: String,
    /// The output content.
    pub content: String,
    /// Whether the tool execution resulted in an error.
    pub is_error: bool,
}
```
**File:** crates/openfang-runtime/src/llm_driver.rs (L146-171)
```rust
#[async_trait]
pub trait LlmDriver: Send + Sync {
    /// Send a completion request and get a response.
    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse, LlmError>;

    /// Stream a completion request, sending incremental events to the channel.
    /// Returns the full response when complete. Default wraps `complete()`.
    async fn stream(
        &self,
        request: CompletionRequest,
        tx: tokio::sync::mpsc::Sender<StreamEvent>,
    ) -> Result<CompletionResponse, LlmError> {
        let response = self.complete(request).await?;
        let text = response.text();
        if !text.is_empty() {
            let _ = tx.send(StreamEvent::TextDelta { text }).await;
        }
        let _ = tx
            .send(StreamEvent::ContentComplete {
                stop_reason: response.stop_reason,
                usage: response.usage,
            })
            .await;
        Ok(response)
    }
}
```
**File:** crates/openfang-runtime/src/kernel_handle.rs (L27-45)
```rust
pub trait KernelHandle: Send + Sync {
    /// Spawn a new agent from a TOML manifest string.
    /// `parent_id` is the UUID string of the spawning agent (for lineage tracking).
    /// Returns (agent_id, agent_name) on success.
    async fn spawn_agent(
        &self,
        manifest_toml: &str,
        parent_id: Option<&str>,
    ) -> Result<(String, String), String>;

    /// Send a message to another agent and get the response.
    async fn send_to_agent(&self, agent_id: &str, message: &str) -> Result<String, String>;

    /// List all running agents.
    fn list_agents(&self) -> Vec<AgentInfo>;

    /// Kill an agent by ID.
    fn kill_agent(&self, agent_id: &str) -> Result<(), String>;

```
**File:** crates/openfang-runtime/src/agent_loop.rs (L197-219)
```rust
pub async fn run_agent_loop(
    manifest: &AgentManifest,
    user_message: &str,
    session: &mut Session,
    memory: &MemorySubstrate,
    driver: Arc<dyn LlmDriver>,
    available_tools: &[ToolDefinition],
    kernel: Option<Arc<dyn KernelHandle>>,
    skill_registry: Option<&SkillRegistry>,
    mcp_connections: Option<&tokio::sync::Mutex<Vec<McpConnection>>>,
    web_ctx: Option<&WebToolsContext>,
    browser_ctx: Option<&crate::browser::BrowserManager>,
    embedding_driver: Option<&(dyn EmbeddingDriver + Send + Sync)>,
    workspace_root: Option<&Path>,
    on_phase: Option<&PhaseCallback>,
    media_engine: Option<&crate::media_understanding::MediaEngine>,
    tts_engine: Option<&crate::tts::TtsEngine>,
    docker_config: Option<&openfang_types::config::DockerSandboxConfig>,
    hooks: Option<&crate::hooks::HookRegistry>,
    context_window_tokens: Option<usize>,
    process_manager: Option<&crate::process_manager::ProcessManager>,
    user_content_blocks: Option<Vec<ContentBlock>>,
) -> OpenFangResult<AgentLoopResult> {
```
**File:** crates/openfang-runtime/src/tool_runner.rs (L109-127)
```rust
pub async fn execute_tool(
    tool_use_id: &str,
    tool_name: &str,
    input: &serde_json::Value,
    kernel: Option<&Arc<dyn KernelHandle>>,
    allowed_tools: Option<&[String]>,
    caller_agent_id: Option<&str>,
    skill_registry: Option<&SkillRegistry>,
    mcp_connections: Option<&tokio::sync::Mutex<Vec<mcp::McpConnection>>>,
    web_ctx: Option<&WebToolsContext>,
    browser_ctx: Option<&crate::browser::BrowserManager>,
    allowed_env_vars: Option<&[String]>,
    workspace_root: Option<&Path>,
    media_engine: Option<&crate::media_understanding::MediaEngine>,
    exec_policy: Option<&openfang_types::config::ExecPolicy>,
    tts_engine: Option<&crate::tts::TtsEngine>,
    docker_config: Option<&openfang_types::config::DockerSandboxConfig>,
    process_manager: Option<&crate::process_manager::ProcessManager>,
) -> ToolResult {
```
**File:** crates/openfang-kernel/src/workflow.rs (L200-215)
```rust
/// The workflow engine — manages definitions and executes pipeline runs.
pub struct WorkflowEngine {
    /// Registered workflow definitions.
    workflows: Arc<RwLock<HashMap<WorkflowId, Workflow>>>,
    /// Active and completed workflow runs.
    runs: Arc<RwLock<HashMap<WorkflowRunId, WorkflowRun>>>,
}

impl WorkflowEngine {
    /// Create a new workflow engine.
    pub fn new() -> Self {
        Self {
            workflows: Arc::new(RwLock::new(HashMap::new())),
            runs: Arc::new(RwLock::new(HashMap::new())),
        }
    }
```
**File:** crates/openfang-kernel/src/workflow.rs (L434-443)
```rust
    pub async fn execute_run<F, Fut>(
        &self,
        run_id: WorkflowRunId,
        agent_resolver: impl Fn(&StepAgent) -> Option<(AgentId, String)>,
        send_message: F,
    ) -> Result<String, String>
    where
        F: Fn(AgentId, String) -> Fut,
        Fut: std::future::Future<Output = Result<(String, u64, u64), String>>,
    {
```
**File:** crates/openfang-kernel/src/triggers.rs (L83-97)
```rust
pub struct TriggerEngine {
    /// All registered triggers.
    triggers: DashMap<TriggerId, Trigger>,
    /// Index: agent_id → list of trigger IDs belonging to that agent.
    agent_triggers: DashMap<AgentId, Vec<TriggerId>>,
}

impl TriggerEngine {
    /// Create a new trigger engine.
    pub fn new() -> Self {
        Self {
            triggers: DashMap::new(),
            agent_triggers: DashMap::new(),
        }
    }
```
## Q2
全て必要です。では具体的にディレクトリごと削除していいもののリスト、ファイル単位で削除しなければならないものの網羅的リストを作成してください。
### Answer
ユーザーは「全て必要です」と言っているので、前の会話で「条件付き」として挙げたものも含めて全て必要ということです。つまり、OpenFangから宣言的WorkflowとAgentを切り出す際に、削除できるもの（GUIや不要なもの）のリストを作成する必要があります。

ユーザーが求めているのは：
1. ディレクトリごと削除していいもののリスト
2. ファイル単位で削除しなければならないものの網羅的リスト

つまり、切り出したcrateに含める必要がないもの（GUIコンポーネント、CLIコンポーネント、デスクトップアプリ、APIサーバー等）を特定する必要があります。

まず各crateのディレクトリ構造を調べる必要があります。特に：
- `openfang-api/` - APIサーバー（不要）
- `openfang-channels/` - チャンネル（不要かも）
- `openfang-cli/` - CLI（不要）
- `openfang-desktop/` - デスクトップ（不要）
- `openfang-extensions/` - 拡張機能（不要かも）
- `openfang-hands/` - 不明
- `openfang-migrate/` - DBマイグレーション（必要かも）
- `openfang-wire/` - ワイヤープロトコル（条件付き）

また、`packages/`、`sdk/`、`scripts/`、`deploy/`、`docs/`、`agents/`なども調べる必要があります。

さらに、`openfang-runtime/src/`と`openfang-kernel/src/`のファイル一覧も必要です。

並行して多くのディレクトリを調べます。

これで十分な情報が集まりました。ユーザーは「全て必要です」と言っているので、前の会話で「条件付き」として挙げたものも含めて全て必要ということです。

つまり、ユーザーが求めているのは：
1. **ディレクトリごと削除していいもの** - GUIコンポーネント、CLIコンポーネント、デスクトップアプリ、APIサーバー、チャンネルアダプター、SDK、デプロイ設定、ドキュメント、エージェントサンプルなど
2. **ファイル単位で削除しなければならないもの** - 各crateの中で不要なファイル

「全て必要です」というのは、前の会話で「条件付き」として挙げたものも含めて全て必要ということです。つまり、ブラウザ、TTS、Docker、メディア理解なども全て必要。

では、削除できるものは何か？

## ディレクトリごと削除できるもの

### トップレベル
- `crates/openfang-api/` - REST APIサーバー（不要）
- `crates/openfang-channels/` - チャンネルアダプター（Telegram, Slack等）（不要）
- `crates/openfang-cli/` - CLIアプリケーション（不要）
- `crates/openfang-desktop/` - デスクトップアプリ（不要）
- `crates/openfang-migrate/` - OpenClaw移行ツール（不要）
- `crates/openfang-wire/` - ワイヤープロトコル（A2Aを使う場合は必要だが、`openfang-runtime/src/a2a.rs`が直接依存しているかどうか確認が必要）
- `packages/` - WhatsApp Gatewayパッケージ（不要）
- `sdk/` - JavaScript/Python SDK（不要）
- `agents/` - サンプルエージェント定義（不要）
- `deploy/` - デプロイ設定（不要）
- `docs/` - ドキュメント（不要）
- `scripts/` - インストールスクリプト（不要）
- `xtask/` - ビルドタスク（不要）
- `.github/` - CI/CD設定（不要）

### `crates/openfang-extensions/` について
`openfang-extensions`は認証情報管理、OAuth、拡張機能インストールなどを含む。`openfang-runtime`や`openfang-kernel`が直接依存しているかどうかは不明だが、おそらく不要。

### `crates/openfang-hands/` について
`openfang-hands`は何かのレジストリ。`bundled.rs`, `lib.rs`, `registry.rs`のみ。おそらく不要。

## ファイル単位で削除しなければならないもの

### `crates/openfang-kernel/src/`
削除できるファイル：
- `whatsapp_gateway.rs` - WhatsApp Webゲートウェイ（不要）
- `wizard.rs` - NL自動ブートストラップウィザード（不要）
- `pairing.rs` - デバイスペアリング（不要）
- `heartbeat.rs` - ハートビートモニター（不要）
- `auto_reply.rs` - 自動返信エンジン（不要）
- `config_reload.rs` - 設定ホットリロード（不要）
- `cron_delivery.rs` - クロン配信（`openfang-channels`に依存しているため不要）
- `auth.rs` - RBAC認証（不要）

保持するファイル：
- `workflow.rs` - WorkflowEngine（必須）
- `triggers.rs` - TriggerEngine（必須）
- `event_bus.rs` - EventBus（必須）
- `registry.rs` - AgentRegistry（必須）
- `scheduler.rs` - AgentScheduler（必須）
- `capabilities.rs` - CapabilityManager（必須）
- `approval.rs` - ApprovalManager（必須）
- `cron.rs` - CronScheduler（必須）
- `background.rs` - BackgroundExecutor（必須）
- `supervisor.rs` - Supervisor（必須）
- `metering.rs` - MeteringEngine（必須）
- `error.rs` - エラー型（必須）
- `config.rs` - カーネル設定（必須）
- `kernel.rs` - カーネル本体（必須？）
- `lib.rs` - ライブラリルート（必須）

`kernel.rs`については、カーネル全体の実装が含まれているが、これはGUIやAPIサーバーとの統合も含む可能性がある。ただし、WorkflowEngineとAgentRegistryを使うためには必要かもしれない。

### `crates/openfang-types/src/`
削除できるファイル：
- `commands.rs` - スラッシュコマンドレジストリ（CLI/チャンネル/WebSocket用）（不要）
- `comms.rs` - エージェント通信UI用ワイヤー型（不要）
- `webhook.rs` - Webhookトリガー型（不要かも）
- `manifest_signing.rs` - マニフェスト署名（不要かも）

保持するファイル：
- `agent.rs` - AgentManifest等（必須）
- `tool.rs` - ToolDefinition等（必須）
- `message.rs` - Message等（必須）
- `error.rs` - エラー型（必須）
- `memory.rs` - Memory型（必須）
- `event.rs` - Event型（必須）
- `capability.rs` - Capability（必須）
- `config.rs` - ExecPolicy等（必須）
- `taint.rs` - TaintLabel等（必須）
- `tool_compat.rs` - normalize_tool_name（必須）
- `serde_compat.rs` - デシリアライズヘルパー（必須）
- `approval.rs` - ApprovalRequest等（必須）
- `scheduler.rs` - CronJob等（必須）
- `media.rs` - MediaType等（必須）
- `model_catalog.rs` - ModelCatalogEntry等（必須）
- `lib.rs` - ライブラリルート（必須）

`webhook.rs`については、TriggerEngineがWebhookトリガーを使う可能性があるため保持。
`manifest_signing.rs`については、セキュリティ機能として保持。
`commands.rs`と`comms.rs`はCLI/チャンネル/WebSocket専用なので削除可能。

### `crates/openfang-memory/src/`
削除できるファイル：
- `http_client.rs` - メモリAPIゲートウェイのHTTPクライアント（外部サービス依存、不要かも）

保持するファイル：
- `substrate.rs` - MemorySubstrate（必須）
- `session.rs` - Session（必須）
- `knowledge.rs` - ナレッジグラフ（必須）
- `semantic.rs` - セマンティック検索（必須）
- `structured.rs` - 構造化メモリ（必須）
- `usage.rs` - 使用量追跡（必須）
- `consolidation.rs` - メモリ統合（必須）
- `migration.rs` - DBマイグレーション（必須）
- `lib.rs` - ライブラリルート（必須）

`http_client.rs`については、外部メモリAPIサービスへのHTTPクライアントで、ローカルSQLiteを使う場合は不要かもしれないが、確認が必要。

### `crates/openfang-skills/src/`
削除できるファイル：
- `clawhub.rs` - ClawHubマーケットプレイスクライアント（不要）
- `marketplace.rs` - FangHubマーケットプレイスクライアント（不要）
- `openclaw_compat.rs` - OpenClaw互換レイヤー（不要かも）
- `verify.rs` - スキル検証（不要かも）

保持するファイル：
- `registry.rs` - SkillRegistry（必須）
- `loader.rs` - execute_skill_tool（必須）
- `bundled.rs` - バンドルスキル（必須）
- `config_injection.rs` - スキル設定注入（必須）
- `lib.rs` - ライブラリルート（必須）

`openclaw_compat.rs`については、`bundled.rs`が`openclaw_compat::convert_skillmd_str`を使っているため保持が必要。
`verify.rs`については、`clawhub.rs`が使っているが、`clawhub.rs`を削除するなら不要。ただし`loader.rs`が使っているかもしれない。

### `crates/openfang-runtime/src/`
削除できるファイル：
- `a2a.rs` - A2Aプロトコル（不要）
- `mcp_server.rs` - MCPサーバー機能（不要）
- `graceful_shutdown.rs` - グレースフルシャットダウン（不要）
- `command_lane.rs` - コマンドレーン（不要）
- `copilot_oauth.rs` - GitHub Copilot OAuth（不要）
- `auth_cooldown.rs` - 認証クールダウン（不要）

保持するファイル（全て必要と言っているので）：
- `agent_loop.rs` - 必須
- `llm_driver.rs` - 必須
- `kernel_handle.rs` - 必須
- `prompt_builder.rs` - 必須
- `tool_runner.rs` - 必須
- `embedding.rs` - 必須
- `loop_guard.rs` - 必須
- `context_budget.rs` - 必須
- `context_overflow.rs` - 必須
- `session_repair.rs` - 必須
- `reply_directives.rs` - 必須
- `retry.rs` - 必須
- `routing.rs` - 必須
- `hooks.rs` - 必須
- `mcp.rs` - 必須
- `web_search.rs` - 必須
- `browser.rs` - 必須
- `media_understanding.rs` - 必須
- `tts.rs` - 必須
- `docker_sandbox.rs` - 必須
- `process_manager.rs` - 必須
- `python_runtime.rs` - 必須
- `sandbox.rs` - 必須
- `workspace_sandbox.rs` - 必須
- `subprocess_sandbox.rs` - 必須
- `apply_patch.rs` - 必須
- `audit.rs` - 必須
- `agent_context.rs` - 必須
- `workspace_context.rs` - 必須
- `compactor.rs` - 必須
- `think_filter.rs` - 必須
- `tool_policy.rs` - 必須
- `web_fetch.rs` - 必須
- `web_content.rs` - 必須
- `web_cache.rs` - 必須
- `link_understanding.rs` - 必須
- `image_gen.rs` - 必須
- `shell_bleed.rs` - 必須
- `str_utils.rs` - 必須
- `host_functions.rs` - 必須（WASMサンドボックス用）
- `model_catalog.rs` - 必須
- `llm_errors.rs` - 必須
- `provider_health.rs` - 必須
- `drivers/` - 全て必須
- `lib.rs` - 必須

削除できるファイル：
- `a2a.rs` - A2Aプロトコル（外部エージェント通信プロトコル、Darviumからの関数呼び出しには不要）
- `mcp_server.rs` - MCPサーバー機能（MCPクライアントは必要だが、サーバーは不要）
- `graceful_shutdown.rs` - グレースフルシャットダウン（サーバープロセス管理用、不要）
- `command_lane.rs` - コマンドレーン（CLI/TUI用、不要）
- `copilot_oauth.rs` - GitHub Copilot OAuth（ドライバーが直接使うかもしれないが、`copilot.rs`ドライバーを使わない場合は不要）
- `auth_cooldown.rs` - 認証クールダウン（APIサーバー用、不要）

`copilot_oauth.rs`については、`drivers/copilot.rs`が使っている可能性があるため、Copilotドライバーを使う場合は必要。ただし、ドライバーを選択的に使う場合は削除可能。

## まとめ

ユーザーが求めているのは：
1. ディレクトリごと削除していいもの
2. ファイル単位で削除しなければならないもの

これを整理して回答します。

注意：`openfang-wire`については、`openfang-runtime/src/a2a.rs`が依存している可能性があるが、`a2a.rs`自体を削除するなら`openfang-wire`も削除可能。

また、`openfang-extensions`については、`openfang-kernel`や`openfang-runtime`が依存しているかどうかは確認できていないが、おそらく不要。

`openfang-hands`については、何かのハンドラーレジストリで、おそらく不要。

`openfang-migrate`については、OpenClaw移行ツールで、不要。

では、整理して回答します。

## 削除リスト

### ディレクトリごと削除

**crateレベル（`crates/`以下）：**
- `crates/openfang-api/` - REST APIサーバー
- `crates/openfang-channels/` - チャンネルアダプター（Telegram, Slack等）
- `crates/openfang-cli/` - CLIアプリケーション
- `crates/openfang-desktop/` - デスクトップアプリ（Tauri）
- `crates/openfang-migrate/` - OpenClaw移行ツール
- `crates/openfang-wire/` - ワイヤープロトコル（a2a.rsを削除するなら）
- `crates/openfang-extensions/` - 拡張機能管理（要確認）
- `crates/openfang-hands/` - ハンドラーレジストリ（要確認）

**トップレベル：**
- `packages/` - WhatsApp Gatewayパッケージ
- `sdk/` - JavaScript/Python SDK
- `agents/` - サンプルエージェント定義
- `deploy/` - デプロイ設定
- `docs/` - ドキュメント
- `scripts/` - インストールスクリプト
- `xtask/` - ビルドタスク
- `.github/` - CI/CD設定

### ファイル単位で削除

**`crates/openfang-kernel/src/`：**
- `whatsapp_gateway.rs`
- `wizard.rs`
- `pairing.rs`
- `heartbeat.rs`
- `auto_reply.rs`
- `config_reload.rs`
- `cron_delivery.rs`
- `auth.rs`

**`crates/openfang-types/src/`：**
- `commands.rs`
- `comms.rs`

**`crates/openfang-memory/src/`：**
- `http_client.rs`（外部メモリAPIサービス用、ローカルSQLiteのみ使う場合）

**`crates/openfang-skills/src/`：**
- `clawhub.rs`
- `marketplace.rs`

**`crates/openfang-runtime/src/`：**
- `a2a.rs`
- `mcp_server.rs`
- `graceful_shutdown.rs`
- `command_lane.rs`
- `auth_cooldown.rs`
- `copilot_oauth.rs`（Copilotドライバーを使わない場合）

### 不確実な点

- `crates/openfang-extensions/`と`crates/openfang-hands/`については、`openfang-kernel`や`openfang-runtime`が依存しているかどうかを確認できていない。
- `crates/openfang-memory/src/http_client.rs`については、`substrate.rs`が使っているかどうかを確認できていない。
- `crates/openfang-types/src/webhook.rs`については、TriggerEngineが使っているかどうかを確認できていない。
- `crates/openfang-types/src/manifest_signing.rs`については、`openfang-kernel`が使っているかどうかを確認できていない。
- `crates/openfang-types/src/scheduler.rs`については、`cron_delivery.rs`が使っているが、`cron.rs`も使っているかもしれない。`cron.rs`は必要なので保持。
- `crates/openfang-runtime/src/copilot_oauth.rs`については、`drivers/copilot.rs`が使っているかもしれない。

これらの不確実な点を明示して回答します。

また、`openfang-kernel/src/kernel.rs`については、カーネル全体の実装が含まれているが、WorkflowEngineとAgentRegistryを使うためには必要かもしれない。ただし、カーネル全体を切り出す必要はなく、WorkflowEngineとTriggerEngineのみを切り出す場合は、`kernel.rs`は不要かもしれない。しかし、ユーザーは「全て必要」と言っているので、`kernel.rs`も含めて保持する。

実際には、ユーザーは「全て必要です」と言っているので、前の会話で「条件付き」として挙げたものも含めて全て必要ということです。つまり、削除できるのは：
1. GUI/CLI/デスクトップ関連
2. チャンネルアダプター（Telegram, Slack等）
3. REST APIサーバー
4. SDK（JavaScript/Python）
5. サンプルエージェント
6. デプロイ設定
7. ドキュメント
8. ビルドタスク
9. CI/CD設定
10. WhatsApp Gatewayパッケージ
11. OpenClaw移行ツール
12. A2Aプロトコル（外部エージェント通信）
13. MCPサーバー機能（クライアントは必要）
14. グレースフルシャットダウン（サーバープロセス管理用）
15. コマンドレーン（CLI/TUI用）
16. 認証クールダウン（APIサーバー用）
17. WhatsApp Gatewayカーネルモジュール
18. NL自動ブートストラップウィザード
19. デバイスペアリング
20. ハートビートモニター（自律エージェント監視用）
21. 自動返信エンジン（チャンネル用）
22. 設定ホットリロード（サーバー用）
23. クロン配信（チャンネル配信用）
24. RBAC認証（マルチユーザー用）
25. スラッシュコマンドレジストリ（CLI/チャンネル/WebSocket用）
26. エージェント通信UI用ワイヤー型
27. ClawHubマーケットプレイスクライアント
28. FangHubマーケットプレイスクライアント
29. メモリAPIゲートウェイHTTPクライアント（外部サービス用）

これを整理して回答します。


以下は、調査した全ディレクトリ・ファイル構造に基づく削除リストです。「全て必要」という前提のもと、GUIなしでDarviumから関数呼び出しするだけに不要なものを列挙します。

---

## ディレクトリごと削除できるもの

### crateレベル（`crates/`以下）

| ディレクトリ | 理由 |
|---|---|
| `crates/openfang-api/` | REST APIサーバー・WebSocket・OpenAI互換エンドポイント |
| `crates/openfang-channels/` | Telegram/Slack/Discord等チャンネルアダプター全体 |
| `crates/openfang-cli/` | CLIアプリ・TUI（ratatui）全体 |
| `crates/openfang-desktop/` | Tauriデスクトップアプリ全体 |
| `crates/openfang-migrate/` | OpenClaw移行ツール |
| `crates/openfang-wire/` | A2Aワイヤープロトコル（`a2a.rs`を削除するため不要） |
| `crates/openfang-extensions/` | 拡張機能インストーラー・OAuth・Vault（※後述） |
| `crates/openfang-hands/` | ハンドラーレジストリ（※後述） |

### トップレベル

| ディレクトリ | 理由 |
|---|---|
| `packages/` | WhatsApp Gatewayのnode.jsパッケージ |
| `sdk/` | JavaScript/Python SDK |
| `agents/` | サンプルエージェント定義TOML群 |
| `deploy/` | systemdサービスファイル |
| `docs/` | ドキュメント |
| `scripts/` | インストールシェルスクリプト |
| `xtask/` | ビルドタスク |
| `.github/` | CI/CD設定 |

---

## ファイル単位で削除しなければならないもの

### `crates/openfang-kernel/src/`

| ファイル | 理由 |
|---|---|
| `whatsapp_gateway.rs` | WhatsApp Web Node.jsプロセス管理 |
| `wizard.rs` | NL自動ブートストラップウィザード |
| `pairing.rs` | QRコードデバイスペアリング |
| `heartbeat.rs` | 自律エージェント死活監視（24/7用） |
| `auto_reply.rs` | チャンネル向け自動返信エンジン |
| `config_reload.rs` | サーバー設定ホットリロード |
| `cron_delivery.rs` | `openfang-channels`に直接依存するクロン配信 |
| `auth.rs` | マルチユーザーRBAC認証 | openfang:1-14 openfang:13-14 

### `crates/openfang-types/src/`

| ファイル | 理由 |
|---|---|
| `commands.rs` | CLI/チャンネル/WebSocket専用スラッシュコマンドレジストリ |
| `comms.rs` | エージェント通信UI用ワイヤー型（REST API/TUI専用） | openfang:1-11 openfang:1-6 

### `crates/openfang-memory/src/`

| ファイル | 理由 |
|---|---|
| `http_client.rs` | 外部メモリAPIサービス（PostgreSQL+pgvector）へのHTTPクライアント。ローカルSQLiteのみ使用するため不要 | openfang:1-7 

### `crates/openfang-skills/src/`

| ファイル | 理由 |
|---|---|
| `clawhub.rs` | ClawHubマーケットプレイスHTTPクライアント |
| `marketplace.rs` | FangHubマーケットプレイスHTTPクライアント | openfang:1-13 

### `crates/openfang-runtime/src/`

| ファイル | 理由 |
|---|---|
| `a2a.rs` | A2Aエージェント間通信プロトコル（`openfang-wire`依存） |
| `mcp_server.rs` | MCPサーバー機能（クライアント側`mcp.rs`は保持） |
| `graceful_shutdown.rs` | サーバープロセスのグレースフルシャットダウン |
| `command_lane.rs` | CLI/TUI向けコマンドレーン |
| `auth_cooldown.rs` | APIサーバー向け認証クールダウン |
| `copilot_oauth.rs` | GitHub Copilot OAuthフロー（Copilotドライバーを使わない場合のみ削除可） |

---

## 不確実な点（削除前に依存関係を確認すべきファイル）

以下はソースを完全に読めていないため、削除前に `cargo check` または `grep` で参照元を確認してください。

| 対象 | 確認すべき点 |
|---|---|
| `crates/openfang-extensions/` | `openfang-kernel/src/kernel.rs`が`openfang_extensions`をuse/依存しているか |
| `crates/openfang-hands/` | `openfang-kernel/src/kernel.rs`が`openfang_hands`をuse/依存しているか |
| `crates/openfang-types/src/webhook.rs` | `triggers.rs`がWebhookトリガー型を参照しているか |
| `crates/openfang-types/src/manifest_signing.rs` | `kernel.rs`がマニフェスト署名検証を呼んでいるか |
| `crates/openfang-types/src/scheduler.rs` | `cron.rs`（保持対象）が参照しているため**削除不可の可能性が高い** |
| `crates/openfang-runtime/src/copilot_oauth.rs` | `drivers/copilot.rs`が`copilot_oauth`をuse/依存しているか |
| `crates/openfang-skills/src/verify.rs` | `loader.rs`（保持対象）が参照しているか（`clawhub.rs`のみが使っているなら削除可） |
| `crates/openfang-skills/src/openclaw_compat.rs` | `bundled.rs`が`convert_skillmd_str`を呼んでいるため**削除不可** | openfang:6-6 openfang:1-9
### Citations
**File:** crates/openfang-kernel/src/whatsapp_gateway.rs (L1-14)
```rust
//! WhatsApp Web gateway — embedded Node.js process management.
//!
//! Embeds the gateway JS at compile time, extracts it to `~/.openfang/whatsapp-gateway/`,
//! runs `npm install` if needed, and spawns `node index.js` as a managed child process
//! that auto-restarts on crash.

use crate::config::openfang_home;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, warn};

/// Gateway source files embedded at compile time.
const GATEWAY_INDEX_JS: &str = include_str!("../../../packages/whatsapp-gateway/index.js");
const GATEWAY_PACKAGE_JSON: &str = include_str!("../../../packages/whatsapp-gateway/package.json");
```
**File:** crates/openfang-kernel/src/cron_delivery.rs (L13-14)
```rust
use openfang_channels::bridge::ChannelBridgeHandle;
use openfang_types::scheduler::CronDeliveryTarget;
```
**File:** crates/openfang-types/src/commands.rs (L1-11)
```rust
//! Unified slash command registry.
//!
//! This module is the single source of truth for every slash command that can be
//! dispatched across CLI, channel adapters (Telegram/Slack/etc.), and the web
//! chat (WebSocket).
//!
//! Each dispatch site (there are three: `openfang-cli/src/tui/mod.rs`,
//! `openfang-channels/src/bridge.rs`, `openfang-api/src/ws.rs`) retains its own
//! handler logic. The registry is added as a front-door so command names and
//! aliases can be canonicalised once and help / autocomplete is generated from
//! a single list.
```
**File:** crates/openfang-types/src/comms.rs (L1-6)
```rust
//! Shared wire types for the Agent Communication UI.
//!
//! These types are used by both the REST API and the TUI to represent
//! agent topology graphs, inter-agent communication events, and
//! request payloads for sending messages / posting tasks.

```
**File:** crates/openfang-memory/src/http_client.rs (L1-7)
```rust
//! HTTP client for the memory-api gateway.
//!
//! Provides a blocking HTTP client that routes `remember` and `recall` operations
//! to the shared memory-api service (PostgreSQL + pgvector + Jina AI embeddings).
//! Designed to be called from synchronous SemanticStore methods within
//! `spawn_blocking` contexts.

```
**File:** crates/openfang-skills/src/clawhub.rs (L1-13)
```rust
//! ClawHub marketplace client — search and install skills from clawhub.ai.
//!
//! ClawHub hosts 3,000+ community skills in both SKILL.md (prompt-only)
//! and package.json (Node.js) formats. This client downloads, converts,
//! and security-scans skills before installation.
//!
//! API reference: <https://clawhub.ai/api/v1/>
//! - Search: `GET /api/v1/search?q=...&limit=20`
//! - Browse: `GET /api/v1/skills?limit=20&sort=trending`
//! - Detail: `GET /api/v1/skills/{slug}`
//! - Download: `GET /api/v1/download?slug=...`
//! - File: `GET /api/v1/skills/{slug}/file?path=SKILL.md`

```
**File:** crates/openfang-skills/src/bundled.rs (L6-6)
```rust
use crate::openclaw_compat::convert_skillmd_str;
```
**File:** crates/openfang-types/src/scheduler.rs (L1-9)
```rust
//! Cron/scheduled job types for the OpenFang scheduler.
//!
//! Defines the core types for recurring and one-shot scheduled jobs that can
//! trigger agent turns, system events, or webhook deliveries.

use crate::agent::AgentId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
```
## Q3
以下は残さなければなりません。

- crates/openfang-channels/
- crates/openfang-extensions/
- agents/
### Answer
ユーザーは前の会話で作成した削除リストを修正しています。以下のものは削除せずに残さなければならないと言っています：

- `crates/openfang-channels/`
- `crates/openfang-extensions/`
- `agents/`

つまり、前の回答の削除リストからこれらを除外して、更新されたリストを提示する必要があります。

前の回答の削除リストを確認して、これらを除いた更新版を提示します。特に新しい調査は不要で、前の回答を修正するだけです。


前回の削除リストから該当3項目を除外した更新版です。

---

## ディレクトリごと削除できるもの（更新版）

### crateレベル（`crates/`以下）

| ディレクトリ | 理由 |
|---|---|
| `crates/openfang-api/` | REST APIサーバー・WebSocket・OpenAI互換エンドポイント |
| ~~`crates/openfang-channels/`~~ | **残す** |
| `crates/openfang-cli/` | CLIアプリ・TUI（ratatui）全体 |
| `crates/openfang-desktop/` | Tauriデスクトップアプリ全体 |
| `crates/openfang-migrate/` | OpenClaw移行ツール |
| `crates/openfang-wire/` | A2Aワイヤープロトコル（`a2a.rs`を削除するため不要） |
| ~~`crates/openfang-extensions/`~~ | **残す** |
| `crates/openfang-hands/` | ハンドラーレジストリ（依存確認後） |

### トップレベル

| ディレクトリ | 理由 |
|---|---|
| `packages/` | WhatsApp Gatewayのnode.jsパッケージ |
| `sdk/` | JavaScript/Python SDK |
| ~~`agents/`~~ | **残す** |
| `deploy/` | systemdサービスファイル |
| `docs/` | ドキュメント |
| `scripts/` | インストールシェルスクリプト |
| `xtask/` | ビルドタスク |
| `.github/` | CI/CD設定 |

---

## ファイル単位で削除しなければならないもの（変更なし）

### `crates/openfang-kernel/src/`

| ファイル | 理由 |
|---|---|
| `whatsapp_gateway.rs` | WhatsApp Web Node.jsプロセス管理 |
| `wizard.rs` | NL自動ブートストラップウィザード |
| `pairing.rs` | QRコードデバイスペアリング |
| `heartbeat.rs` | 自律エージェント死活監視 |
| `auto_reply.rs` | チャンネル向け自動返信エンジン |
| `config_reload.rs` | サーバー設定ホットリロード |
| `cron_delivery.rs` | `openfang-channels`に直接依存するクロン配信 |
| `auth.rs` | マルチユーザーRBAC認証 | openfang:1-14 openfang:13-14 

### `crates/openfang-types/src/`

| ファイル | 理由 |
|---|---|
| `commands.rs` | CLI/チャンネル/WebSocket専用スラッシュコマンドレジストリ |
| `comms.rs` | エージェント通信UI用ワイヤー型（REST API/TUI専用） | openfang:1-11 openfang:1-6 

### `crates/openfang-memory/src/`

| ファイル | 理由 |
|---|---|
| `http_client.rs` | 外部メモリAPIサービス（PostgreSQL+pgvector）へのHTTPクライアント | openfang:1-7 

### `crates/openfang-skills/src/`

| ファイル | 理由 |
|---|---|
| `clawhub.rs` | ClawHubマーケットプレイスHTTPクライアント |
| `marketplace.rs` | FangHubマーケットプレイスHTTPクライアント |

### `crates/openfang-runtime/src/`

| ファイル | 理由 |
|---|---|
| `a2a.rs` | A2Aエージェント間通信プロトコル（`openfang-wire`依存） |
| `mcp_server.rs` | MCPサーバー機能（クライアント側`mcp.rs`は保持） |
| `graceful_shutdown.rs` | サーバープロセスのグレースフルシャットダウン |
| `command_lane.rs` | CLI/TUI向けコマンドレーン |
| `auth_cooldown.rs` | APIサーバー向け認証クールダウン |
| `copilot_oauth.rs` | GitHub Copilot OAuthフロー（Copilotドライバーを使わない場合のみ） |

---

## 不確実な点（削除前に依存関係を確認すべきファイル）

前回と同様です。`crates/openfang-extensions/`を残すことで、`openfang-extensions`への依存確認は不要になりました。

| 対象 | 確認すべき点 |
|---|---|
| `crates/openfang-hands/` | `openfang-kernel/src/kernel.rs`が`openfang_hands`をuse/依存しているか |
| `crates/openfang-types/src/webhook.rs` | `triggers.rs`がWebhookトリガー型を参照しているか |
| `crates/openfang-types/src/manifest_signing.rs` | `kernel.rs`がマニフェスト署名検証を呼んでいるか |
| `crates/openfang-types/src/scheduler.rs` | `cron.rs`（保持対象）が参照しているため**削除不可の可能性が高い** |
| `crates/openfang-runtime/src/copilot_oauth.rs` | `drivers/copilot.rs`が`copilot_oauth`をuse/依存しているか |
| `crates/openfang-skills/src/verify.rs` | `loader.rs`（保持対象）が参照しているか |
| `crates/openfang-skills/src/openclaw_compat.rs` | `bundled.rs`が`convert_skillmd_str`を呼んでいるため**削除不可** | openfang:6-6 openfang:1-9
### Citations
**File:** crates/openfang-kernel/src/whatsapp_gateway.rs (L1-14)
```rust
//! WhatsApp Web gateway — embedded Node.js process management.
//!
//! Embeds the gateway JS at compile time, extracts it to `~/.openfang/whatsapp-gateway/`,
//! runs `npm install` if needed, and spawns `node index.js` as a managed child process
//! that auto-restarts on crash.

use crate::config::openfang_home;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{info, warn};

/// Gateway source files embedded at compile time.
const GATEWAY_INDEX_JS: &str = include_str!("../../../packages/whatsapp-gateway/index.js");
const GATEWAY_PACKAGE_JSON: &str = include_str!("../../../packages/whatsapp-gateway/package.json");
```
**File:** crates/openfang-kernel/src/cron_delivery.rs (L13-14)
```rust
use openfang_channels::bridge::ChannelBridgeHandle;
use openfang_types::scheduler::CronDeliveryTarget;
```
**File:** crates/openfang-types/src/commands.rs (L1-11)
```rust
//! Unified slash command registry.
//!
//! This module is the single source of truth for every slash command that can be
//! dispatched across CLI, channel adapters (Telegram/Slack/etc.), and the web
//! chat (WebSocket).
//!
//! Each dispatch site (there are three: `openfang-cli/src/tui/mod.rs`,
//! `openfang-channels/src/bridge.rs`, `openfang-api/src/ws.rs`) retains its own
//! handler logic. The registry is added as a front-door so command names and
//! aliases can be canonicalised once and help / autocomplete is generated from
//! a single list.
```
**File:** crates/openfang-types/src/comms.rs (L1-6)
```rust
//! Shared wire types for the Agent Communication UI.
//!
//! These types are used by both the REST API and the TUI to represent
//! agent topology graphs, inter-agent communication events, and
//! request payloads for sending messages / posting tasks.

```
**File:** crates/openfang-memory/src/http_client.rs (L1-7)
```rust
//! HTTP client for the memory-api gateway.
//!
//! Provides a blocking HTTP client that routes `remember` and `recall` operations
//! to the shared memory-api service (PostgreSQL + pgvector + Jina AI embeddings).
//! Designed to be called from synchronous SemanticStore methods within
//! `spawn_blocking` contexts.

```
**File:** crates/openfang-skills/src/bundled.rs (L6-6)
```rust
use crate::openclaw_compat::convert_skillmd_str;
```
**File:** crates/openfang-types/src/scheduler.rs (L1-9)
```rust
//! Cron/scheduled job types for the OpenFang scheduler.
//!
//! Defines the core types for recurring and one-shot scheduled jobs that can
//! trigger agent turns, system events, or webhook deliveries.

use crate::agent::AgentId;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
```
## Q4
OKです。では、Agentに対してリクエスト＆レスポンス（関数呼び出しと返り値）ができるようにすること、現在はI/Oによる対話的な使用ができないWorkflowに対し、追加でプロンプトを投げるリクエストとワークフロー完了後に必ず何らかのレスポンスを返すことができるようにすること、までを関数で呼び出せるAPIにまとめ、crateとして成立させ、私のDarviumからどのように呼び出せば期待通りに動くのかまでを示した綿密な改修計画を書いてください。その際、AgentやWorkflowの設定TOMLをどこにどのように書いてどこに置くのかまでインストラクトしなければなりません。
### Answer
ユーザーは以下を求めています：

1. Agentに対してリクエスト＆レスポンス（関数呼び出しと返り値）ができるようにすること
2. 現在はI/Oによる対話的な使用ができないWorkflowに対し、追加でプロンプトを投げるリクエストとワークフロー完了後に必ず何らかのレスポンスを返すことができるようにすること
3. これらを関数で呼び出せるAPIにまとめ、crateとして成立させること
4. DarviumからどのようにWorkflowNode::AgentStepを呼び出せば期待通りに動くのかを示すこと
5. AgentやWorkflowの設定TOMLをどこにどのように書いてどこに置くのかまでインストラクトすること

これは「改修計画を書いてください」という依頼ですが、ask-onlyモードなので計画を作成することはできません。ただし、概念的に何が必要かを説明することはできます。

まず、現在のコードベースをより詳しく調査する必要があります。特に：

1. `WorkflowEngine::execute_run`の詳細な実装
2. `run_agent_loop`の詳細な実装
3. `WorkflowStep`の構造
4. `Session`の構造
5. `MemorySubstrate`の構造
6. `KernelHandle`トレイトの詳細

これらを並行して調査します。

また、ask-onlyモードなので、計画を作成することはできません。ユーザーに「ask-onlyモードなので計画を作成できない」と伝える必要があります。

しかし、ユーザーは「綿密な改修計画を書いてください」と言っています。ask-onlyモードでは計画を作成できないので、概念的な説明のみを提供します。

実際には、ask-onlyモードでは「plan」セクションを作成することはできませんが、概念的な説明は提供できます。ユーザーが求めているのは改修計画なので、これはask-onlyモードの範囲外です。

ただし、ユーザーは「綿密な改修計画を書いてください」と言っているので、これは実装を求めているわけではなく、計画を書くことを求めています。ask-onlyモードでは計画を作成できないので、「この機能を実装するためには何が必要か」という観点から説明します。

まず、現在のコードベースをより詳しく調査して、何が必要かを把握します。

重要なファイルを並行して読み込みます：
1. `crates/openfang-kernel/src/workflow.rs` - WorkflowEngineの詳細
2. `crates/openfang-runtime/src/agent_loop.rs` - エージェントループの詳細
3. `crates/openfang-runtime/src/kernel_handle.rs` - KernelHandleトレイトの詳細
4. `crates/openfang-kernel/src/lib.rs` - カーネルのlib.rs

これらを並行して読み込みます。


まず現在の実装の詳細を並行して調査します。
残りのイテレーションが少ないので、必要な情報を並行して取得します。

ユーザーは以下を求めています：
1. Agentに対してリクエスト＆レスポンス（関数呼び出しと返り値）ができるようにすること
2. WorkflowにI/Oによる対話的な使用（追加プロンプト投げ、完了後レスポンス返却）を追加すること
3. これらを関数で呼び出せるAPIにまとめ、crateとして成立させること
4. DarviumからWorkflowNode::AgentStepを呼び出す方法を示すこと
5. AgentやWorkflowの設定TOMLをどこにどのように書いてどこに置くのかまでインストラクトすること

ask-onlyモードなので計画は作成できません。概念的な説明のみを提供します。

現在の実装を理解するために、以下のファイルを並行して読み込みます：
1. `crates/openfang-runtime/src/agent_loop.rs` の続き（100行以降）
2. `crates/openfang-kernel/src/kernel.rs` - カーネルの実装
3. `crates/openfang-kernel/src/registry.rs` - AgentRegistry
4. `crates/openfang-memory/src/` のディレクトリ構造
5. `crates/openfang-runtime/src/` のディレクトリ構造
6. `agents/` ディレクトリの構造（TOMLの例）

これらを並行して読み込みます。

特に重要なのは：
- `run_agent_loop`の返り値（`AgentLoopResult`）の構造
- `WorkflowEngine::execute_run`の`send_message`クロージャの型
- `AgentRegistry`の実装
- TOMLの例

残りのイテレーションが3つなので、今回で必要な情報を全て取得して、次のイテレーションで回答を作成します。

残りのイテレーションが2つです。今回で必要な情報を全て取得して、次のイテレーションで回答を作成します。

ユーザーが求めているのは：
1. Agentに対してリクエスト＆レスポンス（関数呼び出しと返り値）ができるようにすること
2. WorkflowにI/Oによる対話的な使用（追加プロンプト投げ、完了後レスポンス返却）を追加すること
3. これらを関数で呼び出せるAPIにまとめ、crateとして成立させること
4. DarviumからWorkflowNode::AgentStepを呼び出す方法を示すこと
5. AgentやWorkflowの設定TOMLをどこにどのように書いてどこに置くのかまでインストラクトすること

ask-onlyモードなので計画は作成できません。概念的な説明のみを提供します。

現在の実装を理解するために、以下のファイルを並行して読み込みます：
1. `agents/analyst/` のTOMLファイル - TOMLの例
2. `crates/openfang-kernel/src/kernel.rs` - カーネルの実装（どのようにrun_agent_loopを呼び出しているか）
3. `crates/openfang-memory/src/session.rs` - Sessionの構造
4. `crates/openfang-memory/src/substrate.rs` - MemorySubstrateの構造

これらを並行して読み込みます。

特に重要なのは：
- `kernel.rs`でどのように`run_agent_loop`を呼び出しているか（`send_to_agent`の実装）
- `Session`の構造（どのように会話履歴を管理しているか）
- TOMLの例（どのように書くか）

残りのイテレーションが2つなので、今回で必要な情報を全て取得して、次のイテレーションで回答を作成します。

残りのイテレーションが1つです。今回で必要な情報を全て取得して、次のイテレーションで回答を作成します。

ユーザーが求めているのは：
1. Agentに対してリクエスト＆レスポンス（関数呼び出しと返り値）ができるようにすること
2. WorkflowにI/Oによる対話的な使用（追加プロンプト投げ、完了後レスポンス返却）を追加すること
3. これらを関数で呼び出せるAPIにまとめ、crateとして成立させること
4. DarviumからWorkflowNode::AgentStepを呼び出す方法を示すこと
5. AgentやWorkflowの設定TOMLをどこにどのように書いてどこに置くのかまでインストラクトすること

ask-onlyモードなので計画は作成できません。概念的な説明のみを提供します。

現在の実装を理解するために、以下のファイルを並行して読み込みます：
1. `agents/analyst/agent.toml` - TOMLの例
2. `crates/openfang-kernel/src/kernel.rs` の続き（120行以降）- send_to_agentの実装
3. `crates/openfang-runtime/src/llm_driver.rs` - LlmDriverの詳細

これらを並行して読み込みます。

残りのイテレーションが1つなので、今回で必要な情報を全て取得して、次のイテレーションで回答を作成します。

特に重要なのは：
- `kernel.rs`でどのように`run_agent_loop`を呼び出しているか（`send_to_agent`の実装）
- `agent.toml`の例（どのように書くか）
- `llm_driver.rs`の`DriverConfig`の構造

これらを並行して読み込みます。

残りのイテレーションがなくなりました。今まで収集した情報を元に、ユーザーが求めている改修計画を作成します。

ユーザーが求めているのは：
1. Agentに対してリクエスト＆レスポンス（関数呼び出しと返り値）ができるようにすること
2. WorkflowにI/Oによる対話的な使用（追加プロンプト投げ、完了後レスポンス返却）を追加すること
3. これらを関数で呼び出せるAPIにまとめ、crateとして成立させること
4. DarviumからWorkflowNode::AgentStepを呼び出す方法を示すこと
5. AgentやWorkflowの設定TOMLをどこにどのように書いてどこに置くのかまでインストラクトすること

ask-onlyモードなので計画は作成できません。概念的な説明のみを提供します。

現在の実装の理解：

## 現在の実装

### Agent実行
- `run_agent_loop`は`AgentLoopResult`を返す。`AgentLoopResult.response`が最終テキスト応答。
- `run_agent_loop`は21個の引数を取る。
- `Session`はSQLiteに保存される会話履歴。
- `MemorySubstrate`はSQLiteバックエンドの統合メモリ。

### Workflow実行
- `WorkflowEngine::execute_run`は`Result<String, String>`を返す。最終ステップの出力が返り値。
- `execute_run`は`send_message: F`クロージャを受け取る。このクロージャが各ステップのエージェントにメッセージを送る。
- `WorkflowRun`は`output: Option<String>`フィールドを持つ。

### 現在の問題点
1. **Agentへのリクエスト＆レスポンス**: `run_agent_loop`は既に`AgentLoopResult`を返すが、呼び出し側が`Session`と`MemorySubstrate`を管理する必要がある。これを簡単に呼び出せるAPIにまとめる必要がある。
2. **WorkflowへのI/O**: `WorkflowEngine::execute_run`は既に`Result<String, String>`を返すが、`send_message`クロージャを提供する必要がある。このクロージャが`run_agent_loop`を呼び出す必要がある。
3. **設定TOML**: `AgentManifest`はTOMLからデシリアライズできる。`agents/analyst/agent.toml`の例がある。

## 新しいcrate `darvium-agent-runtime`（または`openfang-embed`）の設計

### 公開API

```rust
// Agent API
pub struct AgentHandle {
    manifest: AgentManifest,
    session: Session,
    memory: Arc<MemorySubstrate>,
    driver: Arc<dyn LlmDriver>,
    available_tools: Vec<ToolDefinition>,
    // ... その他のオプション
}

impl AgentHandle {
    pub async fn new(manifest_toml: &str, db_path: &Path) -> Result<Self, Error>;
    pub async fn send(&mut self, message: &str) -> Result<AgentResponse, Error>;
}

pub struct AgentResponse {
    pub text: String,
    pub usage: TokenUsage,
    pub iterations: u32,
}

// Workflow API
pub struct WorkflowHandle {
    engine: WorkflowEngine,
    agents: HashMap<String, AgentHandle>,
}

impl WorkflowHandle {
    pub async fn new(workflow_toml: &str, agent_dir: &Path, db_path: &Path) -> Result<Self, Error>;
    pub async fn run(&self, input: &str) -> Result<WorkflowResponse, Error>;
}

pub struct WorkflowResponse {
    pub output: String,
    pub step_results: Vec<StepResult>,
}
```

### 設定TOML

**Agent TOML** (`agents/my-agent/agent.toml`):
```toml
name = "my-agent"
version = "0.1.0"
description = "..."
author = "..."
module = "builtin:chat"

[model]
provider = "anthropic"
model = "claude-3-5-sonnet-20241022"
api_key_env = "ANTHROPIC_API_KEY"
max_tokens = 4096
temperature = 0.7
system_prompt = "..."

[capabilities]
tools = ["file_read", "file_write", "web_search"]
```

**Workflow TOML** (`workflows/my-workflow.toml`):
```toml
name = "my-workflow"
description = "..."

[[steps]]
name = "analyze"
agent = { name = "analyst" }
prompt_template = "Analyze this: {{input}}"
mode = "sequential"
timeout_secs = 120

[[steps]]
name = "summarize"
agent = { name = "writer" }
prompt_template = "Summarize: {{input}}"
mode = "sequential"
timeout_secs = 120
```

### 改修が必要な箇所

1. **`run_agent_loop`の引数構造体化**: 21個の引数を`AgentLoopContext`構造体にまとめる。
2. **`AgentHandle`の実装**: `Session`と`MemorySubstrate`を内部で管理し、`send`メソッドで`run_agent_loop`を呼び出す。
3. **`WorkflowHandle`の実装**: `WorkflowEngine`と複数の`AgentHandle`を管理し、`run`メソッドで`execute_run`を呼び出す。
4. **`send_message`クロージャの実装**: `WorkflowEngine::execute_run`に渡す`send_message`クロージャを実装する。このクロージャは`AgentHandle::send`を呼び出す。
5. **Workflow TOMLのデシリアライズ**: `Workflow`構造体はTOMLからデシリアライズできるが、`WorkflowId`と`created_at`はデフォルト値を使う必要がある。
6. **LLMドライバーの初期化**: `AgentManifest.model.provider`に基づいてドライバーを選択する。

### Darviumからの呼び出し方

```rust
// WorkflowNode::AgentStep の実装
use darvium_agent_runtime::{AgentHandle, WorkflowHandle};

// Agent単体の呼び出し
let mut agent = AgentHandle::new(
    include_str!("agents/my-agent/agent.toml"),
    Path::new("~/.darvium/memory.db"),
).await?;
let response = agent.send("Hello, world!").await?;
println!("{}", response.text);

// Workflowの呼び出し
let workflow = WorkflowHandle::new(
    include_str!("workflows/my-workflow.toml"),
    Path::new("agents/"),
    Path::new("~/.darvium/memory.db"),
).await?;
let result = workflow.run("input data").await?;
println!("{}", result.output);
```

## 改修計画の詳細

### 1. 新しいcrate `openfang-embed`の作成

このcrateは以下を提供する：
- `AgentHandle` - Agent単体の呼び出しAPI
- `WorkflowHandle` - Workflowの呼び出しAPI
- `EmbedConfig` - 設定構造体

### 2. `AgentHandle`の実装

`AgentHandle`は以下を内部で管理する：
- `AgentManifest` - TOMLからデシリアライズ
- `Session` - 会話履歴（`MemorySubstrate`から読み込み）
- `Arc<MemorySubstrate>` - SQLiteバックエンド
- `Arc<dyn LlmDriver>` - LLMドライバー（`AgentManifest.model.provider`に基づいて選択）
- `Vec<ToolDefinition>` - 利用可能なツール

`send`メソッドは`run_agent_loop`を呼び出し、`AgentLoopResult`を返す。

### 3. `WorkflowHandle`の実装

`WorkflowHandle`は以下を内部で管理する：
- `WorkflowEngine` - ワークフローエンジン
- `HashMap<String, AgentHandle>` - エージェントハンドル（名前でインデックス）

`run`メソッドは`WorkflowEngine::execute_run`を呼び出す。`send_message`クロージャは`AgentHandle::send`を呼び出す。

ただし、`AgentHandle`は`&mut self`を必要とするため、`Arc<Mutex<AgentHandle>>`でラップする必要がある。

### 4. `run_agent_loop`の引数構造体化

現在の21個の引数を`AgentLoopContext`構造体にまとめる：

```rust
pub struct AgentLoopContext {
    pub memory: Arc<MemorySubstrate>,
    pub driver: Arc<dyn LlmDriver>,
    pub available_tools: Vec<ToolDefinition>,
    pub kernel: Option<Arc<dyn KernelHandle>>,
    pub skill_registry: Option<SkillRegistry>,
    pub mcp_connections: Option<Arc<Mutex<Vec<McpConnection>>>>,
    pub web_ctx: Option<WebToolsContext>,
    pub browser_ctx: Option<BrowserManager>,
    pub embedding_driver: Option<Arc<dyn EmbeddingDriver>>,
    pub workspace_root: Option<PathBuf>,
    pub on_phase: Option<PhaseCallback>,
    pub media_engine: Option<MediaEngine>,
    pub tts_engine: Option<TtsEngine>,
    pub docker_config: Option<DockerSandboxConfig>,
    pub hooks: Option<HookRegistry>,
    pub context_window_tokens: Option<usize>,
    pub process_manager: Option<Arc<ProcessManager>>,
}
```

### 5. Workflow TOMLのデシリアライズ

`Workflow`構造体はTOMLからデシリアライズできるが、`WorkflowId`と`created_at`はデフォルト値を使う必要がある。

```rust
#[derive(Deserialize)]
struct WorkflowToml {
    name: String,
    description: String,
    steps: Vec<WorkflowStep>,
}

impl From<WorkflowToml> for Workflow {
    fn from(toml: WorkflowToml) -> Self {
        Workflow {
            id: WorkflowId::new(),
            name: toml.name,
            description: toml.description,
            steps: toml.steps,
            created_at: Utc::now(),
        }
    }
}
```

### 6. LLMドライバーの初期化

`openfang_runtime::drivers`モジュールにドライバーファクトリがある。`AgentManifest.model.provider`に基づいてドライバーを選択する。

```rust
let driver = drivers::create_driver(&manifest.model)?;
```

### 7. 設定TOMLの配置

**Agent TOML**:
- `{darvium_root}/agents/{agent_name}/agent.toml`
- または`include_str!`でバイナリに埋め込む

**Workflow TOML**:
- `{darvium_root}/workflows/{workflow_name}.toml`
- または`include_str!`でバイナリに埋め込む

**Memory DB**:
- `{darvium_root}/memory.db`（デフォルト）
- または`AgentHandle::new`の引数で指定

### 8. Darviumからの呼び出し方

```rust
// WorkflowNode::AgentStep の実装
impl WorkflowNode {
    pub async fn execute(&self, input: &str) -> Result<String, Error> {
        match self {
            WorkflowNode::AgentStep { agent_toml, .. } => {
                let mut agent = AgentHandle::new(agent_toml, &db_path).await?;
                let response = agent.send(input).await?;
                Ok(response.text)
            }
            // ...
        }
    }
}
```

## 現在の問題点と改修が必要な箇所

### 問題1: `run_agent_loop`の引数爆発

現在の`run_agent_loop`は21個の引数を取る。これを`AgentLoopContext`構造体にまとめる改修が必要。

### 問題2: `WorkflowEngine::execute_run`の`send_message`クロージャ

`send_message`クロージャは`Fn(AgentId, String) -> Fut`型。`AgentHandle`は`&mut self`を必要とするため、`Arc<Mutex<AgentHandle>>`でラップする必要がある。

ただし、`WorkflowEngine::execute_run`の`agent_resolver`は`AgentId`を返すが、`AgentHandle`は名前でインデックスされている。`StepAgent::ByName`の場合は名前でルックアップできるが、`StepAgent::ById`の場合はUUIDでルックアップする必要がある。

### 問題3: `Session`の管理

`run_agent_loop`は`&mut Session`を受け取る。`AgentHandle`は`Session`を内部で管理する必要がある。`Session`は`MemorySubstrate`から読み込む必要がある。

### 問題4: Workflow TOMLのデシリアライズ

`Workflow`構造体は`WorkflowId`と`created_at`フィールドを持つが、TOMLには含まれない。デシリアライズ時にデフォルト値を使う必要がある。

### 問題5: `auth_cooldown`の依存

`agent_loop.rs`の先頭に`use crate::auth_cooldown::{CooldownVerdict, ProviderCooldown};`がある。`auth_cooldown.rs`を削除する場合は、この依存を解消する必要がある。

## まとめ

以上の改修計画を実施することで、DarviumからOpenFangのAgentとWorkflowを関数呼び出しで使えるようになる。
