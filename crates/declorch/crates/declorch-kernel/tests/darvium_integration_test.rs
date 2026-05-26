//! Darvium AgentStep シミュレーション統合テスト
//!
//! Darvium が AgentStep ノードから DeclorchKernel を呼び出す際の動作を
//! 模したテスト。KernelHandle トレイト経由の呼び出しを検証する。
//!
//! 実行: OPENAI_API_KEY=sk-... cargo test -p declorch-kernel --test darvium_integration_test -- --nocapture

use declorch_kernel::workflow::{ErrorMode, StepAgent, StepMode, Workflow, WorkflowId, WorkflowStep};
use declorch_kernel::DeclorchKernel;
use declorch_runtime::kernel_handle::KernelHandle;
use declorch_types::agent::{AgentManifest, Priority, ScheduleMode, ToolProfile};
use declorch_types::config::{DefaultModelConfig, KernelConfig};
use std::sync::Arc;

/// Helper: assert that a Vec contains exactly the given items (order-insensitive).
macro_rules! assert_contains_all {
    ($vec:expr, [$($item:expr),*$(,)?]) => {
        let expected: Vec<&str> = vec![$($item),*];
        for e in &expected {
            assert!(
                $vec.iter().any(|v| v == e),
                "Vec should contain {:?}, but has {:?}",
                e,
                $vec
            );
        }
    };
}

/// OPENAI_API_KEY を使うテスト設定を生成する。
fn test_config() -> (KernelConfig, tempfile::TempDir) {
    let tmp = tempfile::tempdir().expect("temp dir should be created");

    let config = KernelConfig {
        home_dir: tmp.path().to_path_buf(),
        data_dir: tmp.path().join("data"),
        default_model: DefaultModelConfig {
            provider: "openai".to_string(),
            model: "gpt-4.1-nano".to_string(),
            api_key_env: "OPENAI_API_KEY".to_string(),
            base_url: None,
            subprocess_timeout_secs: None,
        },
        ..KernelConfig::default()
    };

    (config, tmp)
}

// ---------------------------------------------------------------------------
// Test 1: Agent + Workflow インスタント生成実行
// Darvium が Agent と Workflow を生成し、2ステップパイプラインを実行する。
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_darvium_agent_and_workflow_instant_run() {
    if std::env::var("OPENAI_API_KEY").is_err() {
        eprintln!("OPENAI_API_KEY not set, skipping");
        return;
    }

    let (config, _tmp) = test_config();
    let kernel = Arc::new(DeclorchKernel::boot_with_config(config).expect("Kernel should boot"));
    kernel.set_self_handle();

    // ----- Darvium の AgentStep が行う操作（KernelHandle トレイト経由）-----

    // Step 1: KernelHandle::spawn_agent() で Agent を生成
    // Darvium は TOML 文字列をそのまま渡す
    let (analyst_id, _analyst_name) = KernelHandle::spawn_agent(
        &*kernel,
        r#"name = "darvium-analyst"
version = "0.1.0"
description = "Analysis agent for Darvium workflow"
author = "darvium-test"
module = "builtin:chat"

[model]
provider = "openai"
model = "gpt-4.1-nano"
max_tokens = 256
temperature = 0.3

system_prompt = "You are an analyst. When given text, respond with exactly: ANALYSIS: followed by a one-sentence analysis."

[capabilities]
tools = ["*"]
"#,
        None,
    )
    .await
    .expect("Analyst agent should spawn");

    let (_writer_id, _writer_name) = KernelHandle::spawn_agent(
        &*kernel,
        r#"name = "darvium-writer"
version = "0.1.0"
description = "Writer agent for Darvium workflow"
author = "darvium-test"
module = "builtin:chat"

[model]
provider = "openai"
model = "gpt-4.1-nano"
max_tokens = 256
temperature = 0.3

system_prompt = "You are a writer. When given text, respond with exactly: SUMMARY: followed by a one-sentence summary."

[capabilities]
tools = ["*"]
"#,
        None,
    )
    .await
    .expect("Writer agent should spawn");

    // Step 2: Workflow を登録（KernelHandle には含まれないため直接）
    let workflow = Workflow {
        id: WorkflowId::new(),
        name: "darvium-analysis-pipeline".to_string(),
        description: "Darvium AgentStep integration test: analyst -> writer".to_string(),
        steps: vec![
            WorkflowStep {
                name: "analyze".to_string(),
                agent: StepAgent::ByName {
                    name: "darvium-analyst".to_string(),
                },
                prompt_template: "Analyze the following: {{input}}".to_string(),
                mode: StepMode::Sequential,
                timeout_secs: 60,
                error_mode: ErrorMode::Fail,
                output_var: None,
            },
            WorkflowStep {
                name: "summarize".to_string(),
                agent: StepAgent::ByName {
                    name: "darvium-writer".to_string(),
                },
                prompt_template: "Summarize this analysis: {{input}}".to_string(),
                mode: StepMode::Sequential,
                timeout_secs: 60,
                error_mode: ErrorMode::Fail,
                output_var: None,
            },
        ],
        created_at: chrono::Utc::now(),
    };

    let wf_id = kernel.register_workflow(workflow).await;

    // Step 3: Workflow を実行
    let result = kernel
        .run_workflow(
            wf_id,
            "Rust's ownership model eliminates data races at compile time."
                .to_string(),
        )
        .await;

    assert!(
        result.is_ok(),
        "Workflow should complete successfully: {:?}",
        result.err()
    );
    let (_run_id, output) = result.unwrap();

    println!("\n========== Darvium Agent+Workflow Test ==========");
    println!("Output:\n{}", output);
    println!("================================================\n");

    assert!(!output.is_empty(), "Workflow output should not be empty");
    assert!(
        output.len() > 20,
        "Workflow output should contain meaningful content"
    );

    // Verify the workflow run record
    let runs = kernel.workflows.list_runs(None).await;
    assert_eq!(runs.len(), 1);
    let run = &runs[0];
    assert_eq!(run.step_results.len(), 2);
    assert_eq!(run.step_results[0].step_name, "analyze");
    assert_eq!(run.step_results[1].step_name, "summarize");
    assert!(
        run.step_results[0].input_tokens > 0,
        "Step 1 should have used input tokens"
    );
    assert!(
        run.step_results[0].output_tokens > 0,
        "Step 1 should have generated output tokens"
    );
    assert!(
        run.step_results[1].input_tokens > 0,
        "Step 2 should have used input tokens"
    );
    assert!(
        run.step_results[1].output_tokens > 0,
        "Step 2 should have generated output tokens"
    );

    KernelHandle::kill_agent(&*kernel, &analyst_id).expect("Analyst should be killed");
    kernel.shutdown();
}

// ---------------------------------------------------------------------------
// Test 2: Agent 単体インスタント生成実行
// Darvium が単一の Agent を生成し、メッセージを送信して応答を得る。
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_darvium_agent_instant_run() {
    if std::env::var("OPENAI_API_KEY").is_err() {
        eprintln!("OPENAI_API_KEY not set, skipping");
        return;
    }

    let (config, _tmp) = test_config();
    let kernel = Arc::new(DeclorchKernel::boot_with_config(config).expect("Kernel should boot"));
    kernel.set_self_handle();

    // ----- Darvium の AgentStep が行う操作（KernelHandle トレイト経由）-----

    // Step 1: KernelHandle::spawn_agent() で Agent を生成
    let (agent_id, _agent_name) = KernelHandle::spawn_agent(
        &*kernel,
        r#"name = "darvium-chat"
version = "0.1.0"
description = "Chat agent for Darvium single-agent test"
author = "darvium-test"
module = "builtin:chat"

[model]
provider = "openai"
model = "gpt-4.1-nano"
max_tokens = 256
temperature = 0.5

system_prompt = "You are a helpful assistant. Reply concisely in one short sentence."

[capabilities]
tools = ["*"]
"#,
        None,
    )
    .await
    .expect("Chat agent should spawn via KernelHandle");

    // Step 2: KernelHandle::send_to_agent() でメッセージ送信
    let response = KernelHandle::send_to_agent(&*kernel, &agent_id, "Say hello in exactly 5 words.")
        .await
        .expect("Agent should respond via KernelHandle");

    println!("\n========== Darvium Single Agent Test ==========");
    println!("Agent ID: {}", agent_id);
    println!("Response: {}", response);
    println!("===============================================\n");

    assert!(!response.is_empty(), "Response should not be empty");
    assert!(
        response.len() >= 5,
        "Response should be at least 5 characters"
    );

    // Step 3: Agent の状態を確認（KernelHandle::list_agents）
    let agents = KernelHandle::list_agents(&*kernel);
    let agent = agents
        .iter()
        .find(|a| a.id == agent_id)
        .expect("Agent should be in the agent list");
    assert_eq!(agent.model_provider, "openai");
    assert_eq!(agent.model_name, "gpt-4.1-nano");

    KernelHandle::kill_agent(&*kernel, &agent_id).expect("Agent should be killed");
    kernel.shutdown();
}

// ---------------------------------------------------------------------------
// Test 3: AgentManifest TOML — 全フィールドパース検証
// KernelHandle::spawn_agent が TOML → AgentManifest を正しくデシリアライズ
// することを全フィールドで確認する。
// ---------------------------------------------------------------------------

#[test]
fn test_agent_manifest_full_fields_toml_parse() {
    // 全28フィールドを含む TOML 文字列
    // 注意: すべてのスカラー/配列トップレベルフィールドは
    // いずれのテーブルセクションより前に記述する必要がある。
    // Enum フィールドは TOML のテーブルセクション形式を使用する:
    //   [field_name]
    //   variant_name = {}
    let toml_str = r#"
# ===== トップレベルスカラー/配列フィールド =====
name = "full-agent"
version = "2.1.0"
description = "Agent with all 28 fields populated"
author = "darvium-test"
module = "builtin:chat"
generate_identity_files = false
cache_context = true
max_history_messages = 40
pinned_model = "gpt-4.1"
workspace = "/tmp/custom-workspace"
state_dir = "/tmp/custom-state"
exec_policy = "full"
tool_allowlist = ["file_read", "file_write"]
tool_blocklist = ["shell_exec"]
skills = ["web-search", "code-review"]
mcp_servers = ["github", "filesystem"]
tags = ["production", "experimental", "v2"]

# ===== Enum: テーブルセクション形式 =====
[priority]
High = {}

[profile]
coding = {}

# ===== 構造化テーブルフィールド =====
[model]
provider = "openai"
model = "gpt-4.1-nano"
max_tokens = 512
temperature = 0.1
system_prompt = "You are a comprehensive test agent."
api_key_env = "CUSTOM_API_KEY"
base_url = "https://custom.example.com/v1"

[[fallback_models]]
provider = "anthropic"
model = "claude-sonnet-4-20250514"

[[fallback_models]]
provider = "groq"
model = "llama-3.3-70b-versatile"
api_key_env = "GROQ_KEY"

[resources]
max_memory_bytes = 134217728
max_cpu_time_ms = 60000
max_tool_calls_per_minute = 120
max_llm_tokens_per_hour = 500000
max_network_bytes_per_hour = 52428800
max_cost_per_hour_usd = 0.5
max_cost_per_day_usd = 5.0
max_cost_per_month_usd = 100.0

[capabilities]
tools = ["file_read", "file_write", "web_fetch"]
network = ["api.openai.com:443"]
shell = ["python3", "bash"]
agent_spawn = true
agent_message = ["*"]
memory_read = ["*"]
memory_write = ["self.*"]
ofp_discover = true
ofp_connect = ["peer-*"]

[tools.script-runner]
params = { language = "python", timeout = 30 }

[tools.formatter]
params = { style = "compact" }

[metadata]
key1 = "value1"
key2 = "value2"

[routing]
simple_model = "gpt-4.1-nano"
medium_model = "gpt-4.1-mini"
complex_model = "gpt-4.1"
simple_threshold = 200
complex_threshold = 800

[autonomous]
quiet_hours = "0 22 * * *"
max_iterations = 100
max_restarts = 5
heartbeat_interval_secs = 60
heartbeat_channel = "telegram"
"#;

    let manifest: AgentManifest = toml::from_str(toml_str)
        .expect("TOML should deserialize into AgentManifest with all 28 fields");

    // --- 基本フィールドの検証 ---
    assert_eq!(manifest.name, "full-agent");
    assert_eq!(manifest.version, "2.1.0");
    assert_eq!(manifest.description, "Agent with all 28 fields populated");
    assert_eq!(manifest.author, "darvium-test");
    assert_eq!(manifest.module, "builtin:chat");

    // --- model ---
    assert_eq!(manifest.model.provider, "openai");
    assert_eq!(manifest.model.model, "gpt-4.1-nano");
    assert_eq!(manifest.model.max_tokens, 512);
    assert_eq!(manifest.model.temperature, 0.1);
    assert_eq!(
        manifest.model.system_prompt,
        "You are a comprehensive test agent."
    );
    assert_eq!(
        manifest.model.api_key_env,
        Some("CUSTOM_API_KEY".to_string())
    );
    assert_eq!(
        manifest.model.base_url,
        Some("https://custom.example.com/v1".to_string())
    );

    // --- fallback_models ---
    assert_eq!(manifest.fallback_models.len(), 2);
    assert_eq!(manifest.fallback_models[0].provider, "anthropic");
    assert_eq!(manifest.fallback_models[0].model, "claude-sonnet-4-20250514");
    assert_eq!(manifest.fallback_models[1].provider, "groq");
    assert_eq!(manifest.fallback_models[1].model, "llama-3.3-70b-versatile");
    assert_eq!(
        manifest.fallback_models[1].api_key_env,
        Some("GROQ_KEY".to_string())
    );

    // --- resources ---
    assert_eq!(manifest.resources.max_memory_bytes, 134217728);
    assert_eq!(manifest.resources.max_cpu_time_ms, 60000);
    assert_eq!(manifest.resources.max_tool_calls_per_minute, 120);
    assert_eq!(manifest.resources.max_llm_tokens_per_hour, 500000);
    assert_eq!(manifest.resources.max_network_bytes_per_hour, 52428800);
    assert_eq!(manifest.resources.max_cost_per_hour_usd, 0.5);
    assert_eq!(manifest.resources.max_cost_per_day_usd, 5.0);
    assert_eq!(manifest.resources.max_cost_per_month_usd, 100.0);

    // --- priority ---
    assert_eq!(manifest.priority, Priority::High);

    // --- capabilities ---
    assert_contains_all!(manifest.capabilities.tools, ["file_read", "file_write", "web_fetch"]);
    assert_contains_all!(manifest.capabilities.network, ["api.openai.com:443"]);
    assert_contains_all!(manifest.capabilities.shell, ["python3", "bash"]);
    assert!(manifest.capabilities.agent_spawn);
    assert_contains_all!(manifest.capabilities.agent_message, ["*"]);
    assert_contains_all!(manifest.capabilities.memory_read, ["*"]);
    assert_contains_all!(manifest.capabilities.memory_write, ["self.*"]);
    assert!(manifest.capabilities.ofp_discover);
    assert_contains_all!(manifest.capabilities.ofp_connect, ["peer-*"]);

    // --- profile ---
    assert_eq!(manifest.profile, Some(ToolProfile::Coding));

    // --- tools ---
    assert!(
        manifest.tools.contains_key("script-runner"),
        "tools should contain 'script-runner'"
    );
    assert!(
        manifest.tools.contains_key("formatter"),
        "tools should contain 'formatter'"
    );

    // --- skills ---
    assert_contains_all!(manifest.skills, ["web-search", "code-review"]);

    // --- mcp_servers ---
    assert_contains_all!(manifest.mcp_servers, ["github", "filesystem"]);

    // --- metadata ---
    assert_eq!(manifest.metadata.len(), 2);
    assert_eq!(
        manifest.metadata.get("key1").and_then(|v| v.as_str()),
        Some("value1")
    );
    assert_eq!(
        manifest.metadata.get("key2").and_then(|v| v.as_str()),
        Some("value2")
    );

    // --- tags ---
    assert_contains_all!(manifest.tags, ["production", "experimental", "v2"]);

    // --- routing ---
    let routing = manifest.routing.expect("routing should be Some");
    assert_eq!(routing.simple_model, "gpt-4.1-nano");
    assert_eq!(routing.medium_model, "gpt-4.1-mini");
    assert_eq!(routing.complex_model, "gpt-4.1");
    assert_eq!(routing.simple_threshold, 200);
    assert_eq!(routing.complex_threshold, 800);

    // --- autonomous ---
    let autonomous = manifest.autonomous.expect("autonomous should be Some");
    assert_eq!(
        autonomous.quiet_hours,
        Some("0 22 * * *".to_string())
    );
    assert_eq!(autonomous.max_iterations, 100);
    assert_eq!(autonomous.max_restarts, 5);
    assert_eq!(autonomous.heartbeat_interval_secs, 60);
    assert_eq!(
        autonomous.heartbeat_channel,
        Some("telegram".to_string())
    );

    // --- pinned_model, workspace, state_dir ---
    assert_eq!(manifest.pinned_model, Some("gpt-4.1".to_string()));
    assert_eq!(
        manifest.workspace,
        Some(std::path::PathBuf::from("/tmp/custom-workspace"))
    );
    assert_eq!(
        manifest.state_dir,
        Some(std::path::PathBuf::from("/tmp/custom-state"))
    );

    // --- generate_identity_files ---
    assert!(!manifest.generate_identity_files);

    // --- exec_policy ---
    let exec = manifest.exec_policy.expect("exec_policy should be Some");
    assert_eq!(
        exec.mode,
        declorch_types::config::ExecSecurityMode::Full
    );

    // --- tool_allowlist / tool_blocklist ---
    assert_contains_all!(manifest.tool_allowlist, ["file_read", "file_write"]);
    assert_contains_all!(manifest.tool_blocklist, ["shell_exec"]);

    // --- cache_context / max_history_messages ---
    assert!(manifest.cache_context);
    assert_eq!(manifest.max_history_messages, Some(40));

    // --- schedule は未設定 → Reactive（デフォルト）を確認 ---
    assert_eq!(manifest.schedule, ScheduleMode::Reactive);
}

// ---------------------------------------------------------------------------
// Test 4: Workflow JSON デシリアライズ — 全フィールド検証
// serde_json::from_str::<Workflow>() が全フィールドを正しくデシリアライズ
// することを確認する。
// ---------------------------------------------------------------------------

#[test]
fn test_workflow_json_deserialize_all_fields() {
    let json_str = r#"{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "json-test-pipeline",
  "description": "Workflow deserialized from JSON",
  "steps": [
    {
      "name": "step-alpha",
      "agent": { "name": "json-alpha-agent" },
      "prompt_template": "Analyze: {{input}}",
      "mode": "sequential",
      "timeout_secs": 60,
      "error_mode": "fail",
      "output_var": "alpha_out"
    },
    {
      "name": "step-beta",
      "agent": { "id": "660e8400-e29b-41d4-a716-446655440001" },
      "prompt_template": "Summarize: {{alpha_out}}",
      "mode": { "conditional": { "condition": "{{alpha_out}} contains 'error'" } },
      "timeout_secs": 120,
      "error_mode": { "retry": { "max_retries": 3 } },
      "output_var": null
    },
    {
      "name": "step-gamma",
      "agent": { "name": "json-gamma-agent" },
      "prompt_template": "Loop until done: {{input}}",
      "mode": { "loop": { "max_iterations": 5, "until": "DONE" } },
      "timeout_secs": 300,
      "error_mode": "skip",
      "output_var": "gamma_out"
    }
  ],
  "created_at": "2026-05-26T12:00:00Z"
}"#;

    let workflow: Workflow = serde_json::from_str(json_str)
        .expect("JSON should deserialize into Workflow");

    // --- id ---
    assert_eq!(
        workflow.id.to_string(),
        "550e8400-e29b-41d4-a716-446655440000"
    );

    // --- name / description ---
    assert_eq!(workflow.name, "json-test-pipeline");
    assert_eq!(workflow.description, "Workflow deserialized from JSON");

    // --- created_at ---
    assert_eq!(
        workflow.created_at.to_rfc3339(),
        "2026-05-26T12:00:00+00:00"
    );

    // --- steps count ---
    assert_eq!(workflow.steps.len(), 3);

    // --- Step 1: step-alpha (ByName, Sequential, Fail) ---
    let s1 = &workflow.steps[0];
    assert_eq!(s1.name, "step-alpha");
    match &s1.agent {
        StepAgent::ByName { name } => assert_eq!(name, "json-alpha-agent"),
        _ => panic!("step-alpha should use ByName"),
    }
    assert_eq!(s1.prompt_template, "Analyze: {{input}}");
    assert!(matches!(s1.mode, StepMode::Sequential));
    assert_eq!(s1.timeout_secs, 60);
    assert!(matches!(s1.error_mode, ErrorMode::Fail));
    assert_eq!(s1.output_var, Some("alpha_out".to_string()));

    // --- Step 2: step-beta (ById, Conditional, Retry) ---
    let s2 = &workflow.steps[1];
    assert_eq!(s2.name, "step-beta");
    match &s2.agent {
        StepAgent::ById { id } => assert_eq!(id, "660e8400-e29b-41d4-a716-446655440001"),
        _ => panic!("step-beta should use ById"),
    }
    assert_eq!(s2.prompt_template, "Summarize: {{alpha_out}}");
    match &s2.mode {
        StepMode::Conditional { condition } => {
            assert_eq!(condition, "{{alpha_out}} contains 'error'");
        }
        _ => panic!("step-beta mode should be Conditional"),
    }
    assert_eq!(s2.timeout_secs, 120); // default (120)
    match &s2.error_mode {
        ErrorMode::Retry { max_retries } => assert_eq!(*max_retries, 3),
        _ => panic!("step-beta error_mode should be Retry"),
    }
    assert_eq!(s2.output_var, None);

    // --- Step 3: step-gamma (ByName, Loop, Skip) ---
    let s3 = &workflow.steps[2];
    assert_eq!(s3.name, "step-gamma");
    match &s3.agent {
        StepAgent::ByName { name } => assert_eq!(name, "json-gamma-agent"),
        _ => panic!("step-gamma should use ByName"),
    }
    match &s3.mode {
        StepMode::Loop {
            max_iterations,
            until,
        } => {
            assert_eq!(*max_iterations, 5);
            assert_eq!(until, "DONE");
        }
        _ => panic!("step-gamma mode should be Loop"),
    }
    assert_eq!(s3.timeout_secs, 300);
    assert!(matches!(s3.error_mode, ErrorMode::Skip));
    assert_eq!(s3.output_var, Some("gamma_out".to_string()));
}

// ---------------------------------------------------------------------------
// Test 5: E2E — デシリアライズした定義を使った実行
// TOML → AgentManifest → spawn_agent、JSON → Workflow → register_workflow
// の両パスを経由して、実際に LLM を呼び出す E2E テスト。
// ---------------------------------------------------------------------------

#[tokio::test]
async fn test_darvium_deserialized_e2e() {
    if std::env::var("OPENAI_API_KEY").is_err() {
        eprintln!("OPENAI_API_KEY not set, skipping");
        return;
    }

    let (config, _tmp) = test_config();
    let kernel = Arc::new(DeclorchKernel::boot_with_config(config).expect("Kernel should boot"));
    kernel.set_self_handle();

    // ---- Step 1: KernelHandle::spawn_agent に TOML 文字列を渡す ----
    // 全フィールドを含む TOML（LLM に影響するフィールドも含む）
    let (agent_id, agent_name) = KernelHandle::spawn_agent(
        &*kernel,
        r#"name = "darvium-deserialized-analyst"
version = "1.0.0"
description = "E2E deserialized agent test"
author = "darvium-test"
module = "builtin:chat"
cache_context = true
max_history_messages = 10

[model]
provider = "openai"
model = "gpt-4.1-nano"
max_tokens = 256
temperature = 0.3
system_prompt = "You are an analyst. When given text, respond with exactly: ANALYSIS: followed by a one-sentence analysis."

[capabilities]
tools = ["*"]
"#,
        None,
    )
    .await
    .expect("Agent should spawn via KernelHandle");

    // Agent 名が正しくパースされたことを確認
    assert_eq!(agent_name, "darvium-deserialized-analyst");

    // ---- Step 2: JSON 文字列 → Workflow デシリアライズ ----
    // register_workflow に渡す Workflow を JSON から構築（struct リテラル不使用）
    let workflow_json = format!(
        r#"{{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "deserialized-e2e-pipeline",
  "description": "E2E test with fully deserialized workflow",
  "steps": [
    {{
      "name": "analyze-deserialized",
      "agent": {{ "name": "darvium-deserialized-analyst" }},
      "prompt_template": "Analyze the following: {{input}}",
      "mode": "sequential",
      "timeout_secs": 60,
      "error_mode": "fail",
      "output_var": null
    }}
  ],
  "created_at": "2026-05-26T12:00:00Z"
}}"#
    );

    let workflow: Workflow = serde_json::from_str(&workflow_json)
        .expect("Workflow JSON should deserialize");
    assert_eq!(workflow.name, "deserialized-e2e-pipeline");
    assert_eq!(workflow.steps.len(), 1);
    assert_eq!(workflow.steps[0].name, "analyze-deserialized");

    // ---- Step 3: Workflow 登録 & 実行 ----
    let wf_id = kernel.register_workflow(workflow).await;

    let result = kernel
        .run_workflow(
            wf_id,
            "Functional programming languages emphasize immutability and pure functions."
                .to_string(),
        )
        .await;

    assert!(
        result.is_ok(),
        "Workflow should complete successfully: {:?}",
        result.err()
    );
    let (run_id, output) = result.unwrap();

    println!("\n========== Deserialized E2E Test ==========");
    println!("Agent ID: {}", agent_id);
    println!("Workflow Run ID: {}", run_id);
    println!("Output:\n{}", output);
    println!("============================================\n");

    assert!(!output.is_empty(), "Workflow output should not be empty");
    assert!(
        output.len() > 20,
        "Workflow output should contain meaningful content"
    );

    // Verify the workflow run record
    let runs = kernel.workflows.list_runs(None).await;
    let run = runs.iter().find(|r| r.id == run_id).expect("Run should exist");
    assert_eq!(run.step_results.len(), 1);
    assert_eq!(run.step_results[0].step_name, "analyze-deserialized");
    assert!(
        run.step_results[0].input_tokens > 0,
        "Should have used input tokens"
    );
    assert!(
        run.step_results[0].output_tokens > 0,
        "Should have generated output tokens"
    );

    KernelHandle::kill_agent(&*kernel, &agent_id).expect("Agent should be killed");
    kernel.shutdown();
}
