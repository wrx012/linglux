use linglux_media_core::{MediaCore, TaskEvent, TaskKind, TaskSnapshot};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::Write as IoWrite;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::async_runtime::JoinHandle;
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};

const AGENT_CONVERSATION_SCHEMA_VERSION: u32 = 1;
const AGENT_MAX_MESSAGES: usize = 200;
const AGENT_MAX_CONVERSATION_BYTES: usize = 1024 * 1024;
const AGENT_MAX_MODEL_ROUNDS: usize = 4;
const AGENT_MAX_TOOL_CALLS: usize = 12;
const AGENT_MAX_PLAN_OPERATIONS: usize = 64;
const AGENT_MAX_PROMPT_CHARS: usize = 20_000;
const AGENT_MAX_TOOL_ITEMS: usize = 200;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentProviderSettingsSummary {
    provider: String,
    base_url: String,
    model: String,
    has_api_key: bool,
    masked_api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveAgentProviderSettingsRequest {
    provider: String,
    base_url: String,
    model: String,
    api_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AgentProviderMetadata {
    provider: String,
    base_url: String,
    model: String,
}

impl Default for AgentProviderMetadata {
    fn default() -> Self {
        Self {
            provider: "deepseek".to_string(),
            base_url: "https://api.deepseek.com".to_string(),
            model: "deepseek-v4-flash".to_string(),
        }
    }
}

impl AgentProviderMetadata {
    fn normalized(mut self) -> Self {
        let fallback = Self::default();
        self.provider = self.provider.trim().to_ascii_lowercase();
        self.base_url = self.base_url.trim().trim_end_matches('/').to_string();
        self.model = self.model.trim().to_string();

        if self.provider.is_empty() {
            self.provider = fallback.provider;
        }
        if self.base_url.is_empty() {
            self.base_url = fallback.base_url;
        }
        if self.model.is_empty() {
            self.model = fallback.model;
        }

        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LegacyApiKeySettings {
    provider: String,
    base_url: String,
    api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentProjectAsset {
    id: String,
    name: String,
    #[serde(rename = "type")]
    asset_type: String,
    duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentProjectClip {
    id: String,
    asset_id: String,
    name: String,
    #[serde(rename = "type")]
    clip_type: String,
    track_id: String,
    timeline_start_ms: u64,
    timeline_duration_ms: u64,
    source_in_ms: u64,
    source_out_ms: u64,
    speed: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentProjectTrack {
    id: String,
    label: String,
    #[serde(rename = "type")]
    track_type: String,
    locked: bool,
    clips: Vec<AgentProjectClip>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentCharacterVoiceProfile {
    id: String,
    name: String,
    color: String,
    voice: String,
    default_emotion: String,
    default_speed: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentDynamicComicShot {
    id: String,
    order: u64,
    character_id: Option<String>,
    emotion: Option<String>,
    speech_speed: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentProjectSnapshot {
    project_id: String,
    project_name: String,
    editor_version: u64,
    duration_ms: u64,
    playhead_ms: u64,
    selected_clip_id: Option<String>,
    selected_asset_ids: Vec<String>,
    main_track_magnet_enabled: bool,
    assets: Vec<AgentProjectAsset>,
    tracks: Vec<AgentProjectTrack>,
    #[serde(default)]
    character_voice_profiles: Vec<AgentCharacterVoiceProfile>,
    #[serde(default)]
    dynamic_comic_shots: Vec<AgentDynamicComicShot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTurnRequest {
    project: AgentProjectSnapshot,
    prompt: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentConversation {
    schema_version: u32,
    project_id: String,
    messages: Vec<AgentChatMessage>,
    updated_at: String,
}

impl AgentConversation {
    fn empty(project_id: impl Into<String>) -> Self {
        Self {
            schema_version: AGENT_CONVERSATION_SCHEMA_VERSION,
            project_id: project_id.into(),
            messages: Vec::new(),
            updated_at: now_stamp(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentChatMessage {
    id: String,
    role: String,
    content: String,
    created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    plan: Option<AgentEditPlan>,
    #[serde(skip_serializing_if = "Option::is_none")]
    plan_state: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentEditPlan {
    id: String,
    project_id: String,
    base_editor_version: u64,
    summary: String,
    operations: Vec<AgentEditOperation>,
    warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum AgentEditOperation {
    AddAssetRange {
        id: String,
        asset_id: String,
        track_id: String,
        source_in_ms: u64,
        source_out_ms: u64,
        timeline_start_ms: Option<u64>,
    },
    KeepClipSourceRange {
        id: String,
        clip_id: String,
        source_in_ms: u64,
        source_out_ms: u64,
    },
    RemoveClipSourceRange {
        id: String,
        clip_id: String,
        source_in_ms: u64,
        source_out_ms: u64,
    },
    SplitClipAtTimeline {
        id: String,
        clip_id: String,
        timeline_time_ms: u64,
    },
    DeleteClip {
        id: String,
        clip_id: String,
    },
    MoveClip {
        id: String,
        clip_id: String,
        track_id: String,
        timeline_start_ms: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProposedEditPlan {
    summary: String,
    operations: Vec<AgentEditOperation>,
    #[serde(default)]
    warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTurnResult {
    conversation: AgentConversation,
    #[serde(skip_serializing_if = "Option::is_none")]
    plan: Option<AgentEditPlan>,
    #[serde(skip_serializing_if = "Option::is_none")]
    clarification: Option<String>,
}

enum AgentModelOutcome {
    Plan(AgentEditPlan),
    Clarification(String),
}

struct RunningAgentTask {
    project_id: String,
    handle: Option<JoinHandle<()>>,
    cancel_requested: bool,
}

trait SecretStore: Send + Sync {
    fn get(&self, provider: &str) -> Result<Option<String>, String>;
    fn set(&self, provider: &str, api_key: &str) -> Result<(), String>;
    fn delete(&self, provider: &str) -> Result<(), String>;
}

struct SessionOnlySecretStore;

impl SecretStore for SessionOnlySecretStore {
    fn get(&self, _provider: &str) -> Result<Option<String>, String> {
        Ok(None)
    }

    fn set(&self, _provider: &str, _api_key: &str) -> Result<(), String> {
        Err("系统凭据存储已禁用；API Key 仅在当前应用会话中保存。".to_string())
    }

    fn delete(&self, _provider: &str) -> Result<(), String> {
        Ok(())
    }
}

struct AgentRuntimeInner {
    client: Client,
    secret_store: Arc<dyn SecretStore>,
    running: Mutex<HashMap<String, RunningAgentTask>>,
    session_keys: Mutex<HashMap<String, String>>,
}

#[derive(Clone)]
pub struct AgentRuntime {
    inner: Arc<AgentRuntimeInner>,
}

impl AgentRuntime {
    pub fn new() -> Result<Self, String> {
        let client = Client::builder()
            .timeout(Duration::from_secs(60))
            .user_agent("Linglux/0.1 editor-agent")
            .build()
            .map_err(|error| error.to_string())?;
        Ok(Self::with_parts(client, Arc::new(SessionOnlySecretStore)))
    }

    fn with_parts(client: Client, secret_store: Arc<dyn SecretStore>) -> Self {
        Self {
            inner: Arc::new(AgentRuntimeInner {
                client,
                secret_store,
                running: Mutex::new(HashMap::new()),
                session_keys: Mutex::new(HashMap::new()),
            }),
        }
    }

    fn try_register_pending(&self, task_id: &str, project_id: &str) -> bool {
        let mut running = self
            .inner
            .running
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        if running.values().any(|task| task.project_id == project_id) {
            return false;
        }

        running.insert(
            task_id.to_string(),
            RunningAgentTask {
                project_id: project_id.to_string(),
                handle: None,
                cancel_requested: false,
            },
        );
        true
    }

    fn attach_handle(&self, task_id: &str, handle: JoinHandle<()>) {
        let mut running = self
            .inner
            .running
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let should_abort = match running.get_mut(task_id) {
            Some(task) if task.cancel_requested => true,
            Some(task) => {
                task.handle = Some(handle);
                return;
            }
            None => true,
        };

        if should_abort {
            running.remove(task_id);
            drop(running);
            handle.abort();
        }
    }

    fn finish(&self, task_id: &str) {
        self.inner
            .running
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(task_id);
    }

    fn abort(&self, task_id: &str) -> bool {
        let mut running = self
            .inner
            .running
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        if let Some(task) = running.get_mut(task_id) {
            if let Some(handle) = task.handle.take() {
                running.remove(task_id);
                drop(running);
                handle.abort();
            } else {
                task.cancel_requested = true;
            }
            true
        } else {
            false
        }
    }

    fn set_session_key(&self, provider: &str, api_key: String) {
        self.inner
            .session_keys
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(provider.to_string(), api_key);
    }

    fn session_key(&self, provider: &str) -> Option<String> {
        self.inner
            .session_keys
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(provider)
            .cloned()
    }

    fn clear_session_key(&self, provider: &str) {
        self.inner
            .session_keys
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .remove(provider);
    }

    fn store_api_key(&self, provider: &str, api_key: String) {
        match self.inner.secret_store.set(provider, &api_key) {
            Ok(()) => self.clear_session_key(provider),
            Err(_) => self.set_session_key(provider, api_key),
        }
    }

    fn load_api_key(&self, provider: &str) -> Option<String> {
        self.session_key(provider).or_else(|| {
            self.inner
                .secret_store
                .get(provider)
                .ok()
                .flatten()
                .filter(|value| !value.trim().is_empty())
        })
    }

    fn clear_api_key(&self, provider: &str) -> Result<(), String> {
        let had_session_key = self.session_key(provider).is_some();
        self.clear_session_key(provider);
        match self.inner.secret_store.delete(provider) {
            Ok(()) => Ok(()),
            Err(_) if had_session_key => Ok(()),
            Err(error) => Err(error),
        }
    }
}

#[tauri::command]
pub fn load_agent_provider_settings(
    app: AppHandle,
    runtime: State<'_, AgentRuntime>,
) -> Result<AgentProviderSettingsSummary, String> {
    let metadata = load_and_migrate_provider_metadata(&app, &runtime)?;
    Ok(provider_summary(&metadata, &runtime))
}

#[tauri::command]
pub fn save_agent_provider_settings(
    app: AppHandle,
    runtime: State<'_, AgentRuntime>,
    settings: SaveAgentProviderSettingsRequest,
) -> Result<AgentProviderSettingsSummary, String> {
    if settings.model.trim().is_empty() {
        return Err("聊天模型 ID 不能为空。".to_string());
    }
    validate_provider_base_url(&settings.base_url)?;

    let metadata = AgentProviderMetadata {
        provider: settings.provider,
        base_url: settings.base_url,
        model: settings.model,
    }
    .normalized();

    if let Some(api_key) = settings.api_key.map(|value| value.trim().to_string()) {
        if !api_key.is_empty() {
            runtime.store_api_key(&metadata.provider, api_key);
        }
    }

    if load_provider_api_key(&metadata.provider, &runtime).is_none() {
        return Err("API Key 不能为空；密钥仅在当前应用会话中保存。".to_string());
    }

    save_provider_metadata(&app, &metadata)?;
    Ok(provider_summary(&metadata, &runtime))
}

#[tauri::command]
pub fn clear_agent_provider_settings(
    app: AppHandle,
    runtime: State<'_, AgentRuntime>,
) -> Result<AgentProviderSettingsSummary, String> {
    let metadata = load_provider_metadata(&app).unwrap_or_default();
    runtime
        .clear_api_key(&metadata.provider)
        .map_err(|error| format!("无法清除当前会话的 API Key：{error}"))?;

    Ok(provider_summary(&metadata, &runtime))
}

#[tauri::command]
pub fn load_agent_conversation(
    core: State<'_, MediaCore>,
    project_id: String,
) -> AgentConversation {
    load_conversation(&core, &project_id)
}

#[tauri::command]
pub fn clear_agent_conversation(
    core: State<'_, MediaCore>,
    project_id: String,
) -> Result<AgentConversation, String> {
    core.projects().clear_agent_conversation(&project_id)?;
    Ok(AgentConversation::empty(project_id))
}

#[tauri::command]
pub fn update_agent_plan_state(
    core: State<'_, MediaCore>,
    project_id: String,
    plan_id: String,
    state: String,
) -> Result<AgentConversation, String> {
    if !matches!(state.as_str(), "applied" | "rejected" | "stale") {
        return Err("不支持的 Agent 计划状态。".to_string());
    }

    let mut conversation = load_conversation(&core, &project_id);
    let mut found = false;

    for message in &mut conversation.messages {
        if message.plan.as_ref().map(|plan| plan.id.as_str()) == Some(plan_id.as_str()) {
            if let Some(existing) = message.plan_state.as_deref() {
                if existing != "pending" && existing != state {
                    return Err("该 Agent 计划已经进入终态，不能再次更改。".to_string());
                }
            }
            message.plan_state = Some(state.clone());
            found = true;
        }
    }

    if !found {
        return Err("找不到要更新的 Agent 计划。".to_string());
    }

    save_conversation(&core, &mut conversation)?;
    Ok(conversation)
}

#[tauri::command]
pub fn start_editor_agent_turn(
    app: AppHandle,
    core: State<'_, MediaCore>,
    runtime: State<'_, AgentRuntime>,
    request: AgentTurnRequest,
    on_event: Channel<TaskEvent>,
) -> Result<TaskSnapshot, String> {
    let prompt = request.prompt.trim().to_string();

    if prompt.is_empty() {
        return Err("请输入剪辑指令。".to_string());
    }
    if prompt.chars().count() > AGENT_MAX_PROMPT_CHARS {
        return Err("剪辑指令过长，请缩短后重试。".to_string());
    }
    let runtime_state = runtime.inner().clone();
    let metadata = load_and_migrate_provider_metadata(&app, &runtime_state)?;
    validate_provider_base_url(&metadata.base_url)?;
    let api_key = load_provider_api_key(&metadata.provider, &runtime_state)
        .ok_or_else(|| "请先在模型服务设置中配置 API Key。".to_string())?;
    let core = core.inner().clone();
    let project_id = request.project.project_id.clone();

    let listener = Arc::new(move |event: TaskEvent| {
        let _ = on_event.send(event);
    });
    let task = core.tasks().create(
        TaskKind::Agent,
        Some(project_id.clone()),
        "AI 剪辑规划",
        Some(listener),
    );
    let snapshot = task.snapshot();
    let task_id = snapshot.id.clone();
    if !runtime_state.try_register_pending(&task_id, &project_id) {
        task.fail("当前工程已有 Agent 请求正在运行。");
        return Err("当前工程已有 Agent 请求正在运行。".to_string());
    }

    let mut conversation = load_conversation(&core, &project_id);
    conversation.messages.push(AgentChatMessage {
        id: next_message_id("user"),
        role: "user".to_string(),
        content: prompt,
        created_at: now_stamp(),
        plan: None,
        plan_state: None,
    });
    if let Err(error) = save_conversation(&core, &mut conversation) {
        runtime_state.finish(&task_id);
        task.fail(error.clone());
        return Err(error);
    }

    let worker_task = task.clone();
    let worker_task_id = task_id.clone();
    let worker_runtime = runtime_state.clone();
    let handle = tauri::async_runtime::spawn(async move {
        worker_task.start("正在读取工程");
        worker_task.set_progress(15.0, "正在查询素材与时间线");

        let outcome = run_agent_model_turn(
            &worker_runtime.inner.client,
            &metadata,
            &api_key,
            &request.project,
            &conversation,
        )
        .await;

        match outcome {
            Ok(outcome) => {
                worker_task.set_progress(88.0, "正在校验剪辑计划");
                let (content, plan, clarification) = match outcome {
                    AgentModelOutcome::Plan(plan) => (plan.summary.clone(), Some(plan), None),
                    AgentModelOutcome::Clarification(message) => {
                        (message.clone(), None, Some(message))
                    }
                };
                conversation.messages.push(AgentChatMessage {
                    id: next_message_id("assistant"),
                    role: "assistant".to_string(),
                    content,
                    created_at: now_stamp(),
                    plan: plan.clone(),
                    plan_state: plan.as_ref().map(|_| "pending".to_string()),
                });

                match save_conversation(&core, &mut conversation).and_then(|_| {
                    serde_json::to_value(AgentTurnResult {
                        conversation,
                        plan,
                        clarification,
                    })
                    .map_err(|error| error.to_string())
                }) {
                    Ok(result) => worker_task.succeed(result),
                    Err(error) => worker_task.fail(error),
                }
            }
            Err(_error) if worker_task.is_cancel_requested() => worker_task.mark_cancelled(),
            Err(error) => worker_task.fail(redact_provider_error(&error, &api_key)),
        }

        worker_runtime.finish(&worker_task_id);
    });
    runtime_state.attach_handle(&task_id, handle);

    Ok(snapshot)
}

#[tauri::command]
pub fn cancel_editor_agent_turn(
    core: State<'_, MediaCore>,
    runtime: State<'_, AgentRuntime>,
    task_id: String,
) -> bool {
    let requested = core.tasks().cancel(&task_id);
    let aborted = runtime.abort(&task_id);

    if requested || aborted {
        core.tasks().mark_cancelled(&task_id);
        true
    } else {
        false
    }
}

async fn run_agent_model_turn(
    client: &Client,
    settings: &AgentProviderMetadata,
    api_key: &str,
    project: &AgentProjectSnapshot,
    conversation: &AgentConversation,
) -> Result<AgentModelOutcome, String> {
    let mut messages = vec![json!({
        "role": "system",
        "content": agent_system_prompt(project)
    })];

    for message in conversation
        .messages
        .iter()
        .rev()
        .take(20)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
    {
        messages.push(json!({
            "role": message.role,
            "content": message.content,
        }));
    }

    let tools = agent_tools();
    let mut tool_call_count = 0usize;
    let mut repaired_invalid_plan = false;

    for _round in 0..AGENT_MAX_MODEL_ROUNDS {
        let provider_message =
            request_provider_message(client, settings, api_key, &messages, &tools).await?;
        let tool_calls = provider_message
            .get("tool_calls")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        if tool_calls.is_empty() {
            let content = provider_message
                .get("content")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim();

            if content.is_empty() {
                return Err("模型没有返回剪辑计划或澄清问题。".to_string());
            }

            return Ok(AgentModelOutcome::Clarification(content.to_string()));
        }

        tool_call_count += tool_calls.len();
        if tool_call_count > AGENT_MAX_TOOL_CALLS {
            return Err("Agent 工具调用次数超过安全上限。".to_string());
        }
        messages.push(provider_message.clone());

        for tool_call in tool_calls {
            let tool_call_id = tool_call
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("unknown-tool-call");
            let function = tool_call
                .get("function")
                .and_then(Value::as_object)
                .ok_or_else(|| "模型返回了无效的工具调用。".to_string())?;
            let name = function
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let arguments = function
                .get("arguments")
                .and_then(Value::as_str)
                .unwrap_or("{}");
            let parsed_arguments = match serde_json::from_str::<Value>(arguments) {
                Ok(arguments) => arguments,
                Err(_) if !repaired_invalid_plan => {
                    repaired_invalid_plan = true;
                    messages.push(tool_result_message(
                        tool_call_id,
                        json!({
                            "ok": false,
                            "error": format!("工具 {name} 的参数不是有效 JSON。"),
                            "instruction": "请修正参数后重试；只允许一次修复。"
                        }),
                    ));
                    continue;
                }
                Err(_) => return Err(format!("工具 {name} 的参数不是有效 JSON。")),
            };

            if name == "propose_edit_plan" {
                match serde_json::from_value::<ProposedEditPlan>(parsed_arguments)
                    .map_err(|error| format!("剪辑计划格式无效：{error}"))
                    .and_then(|proposal| validate_proposed_plan(project, proposal))
                {
                    Ok(plan) => return Ok(AgentModelOutcome::Plan(plan)),
                    Err(error) if !repaired_invalid_plan => {
                        repaired_invalid_plan = true;
                        messages.push(tool_result_message(tool_call_id, json!({
                            "ok": false,
                            "error": error,
                            "instruction": "请修正后再次调用 propose_edit_plan；不要猜测不存在的 ID 或越界时间。"
                        })));
                    }
                    Err(error) => return Err(error),
                }
                continue;
            }

            let result = execute_read_tool(project, name, &parsed_arguments);
            messages.push(tool_result_message(tool_call_id, result));
        }
    }

    Err("Agent 未能在限定轮次内生成有效计划。".to_string())
}

async fn request_provider_message(
    client: &Client,
    settings: &AgentProviderMetadata,
    api_key: &str,
    messages: &[Value],
    tools: &[Value],
) -> Result<Value, String> {
    let endpoint = format!(
        "{}/chat/completions",
        settings.base_url.trim_end_matches('/')
    );
    let mut body = json!({
        "model": settings.model,
        "messages": messages,
        "tools": tools,
        "tool_choice": "auto",
        "stream": false
    });
    if let Some(body) = body.as_object_mut() {
        let token_field = if settings.provider == "openai" {
            "max_completion_tokens"
        } else {
            "max_tokens"
        };
        body.insert(token_field.to_string(), json!(4096));
    }
    let response = client
        .post(endpoint)
        .bearer_auth(api_key)
        .json(&body)
        .send()
        .await
        .map_err(|error| format!("模型服务连接失败：{error}"))?;
    let status = response.status();

    if !status.is_success() {
        let detail = match status.as_u16() {
            401 => "API Key 无效或没有访问权限。",
            429 => "请求过于频繁或额度不足，请稍后重试。",
            _ => "请检查 Provider、Base URL 和模型 ID。",
        };
        return Err(format!("模型服务返回 HTTP {}。{detail}", status.as_u16()));
    }

    let response_text = response
        .text()
        .await
        .map_err(|error| format!("无法读取模型响应：{error}"))?;
    let response_value = serde_json::from_str::<Value>(&response_text)
        .map_err(|_| "模型服务返回了无效 JSON。".to_string())?;
    response_value
        .get("choices")
        .and_then(Value::as_array)
        .and_then(|choices| choices.first())
        .and_then(|choice| choice.get("message"))
        .cloned()
        .ok_or_else(|| "模型服务响应中缺少 assistant message。".to_string())
}

fn execute_read_tool(project: &AgentProjectSnapshot, name: &str, arguments: &Value) -> Value {
    match name {
        "list_assets" => {
            let query = arguments
                .get("query")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .trim()
                .to_lowercase();
            let mut assets = project
                .assets
                .iter()
                .filter(|asset| query.is_empty() || asset.name.to_lowercase().contains(&query))
                .take(AGENT_MAX_TOOL_ITEMS + 1)
                .collect::<Vec<_>>();
            let truncated = assets.len() > AGENT_MAX_TOOL_ITEMS;
            assets.truncate(AGENT_MAX_TOOL_ITEMS);
            json!({
                "assets": assets,
                "truncated": truncated
            })
        }
        "inspect_timeline" => {
            let asset_id = arguments.get("assetId").and_then(Value::as_str);
            let tracks = project
                .tracks
                .iter()
                .map(|track| {
                    let clips = track
                        .clips
                        .iter()
                        .filter(|clip| asset_id.map(|id| clip.asset_id == id).unwrap_or(true))
                        .take(AGENT_MAX_TOOL_ITEMS)
                        .collect::<Vec<_>>();
                    json!({
                        "id": track.id,
                        "label": track.label,
                        "type": track.track_type,
                        "locked": track.locked,
                        "clips": clips
                    })
                })
                .collect::<Vec<_>>();
            json!({ "tracks": tracks })
        }
        "inspect_selection" => json!({
            "playheadMs": project.playhead_ms,
            "selectedClipId": project.selected_clip_id,
            "selectedAssetIds": project.selected_asset_ids,
            "projectDurationMs": project.duration_ms
        }),
        _ => json!({ "error": format!("未知的只读工具：{name}") }),
    }
}

fn validate_proposed_plan(
    project: &AgentProjectSnapshot,
    proposal: ProposedEditPlan,
) -> Result<AgentEditPlan, String> {
    if proposal.summary.trim().is_empty() {
        return Err("剪辑计划缺少摘要。".to_string());
    }
    if proposal.operations.is_empty() {
        return Err("剪辑计划没有任何操作。".to_string());
    }
    if proposal.operations.len() > AGENT_MAX_PLAN_OPERATIONS {
        return Err(format!(
            "剪辑计划一次最多包含 {AGENT_MAX_PLAN_OPERATIONS} 个操作。"
        ));
    }

    let assets = project
        .assets
        .iter()
        .map(|asset| (asset.id.as_str(), asset))
        .collect::<HashMap<_, _>>();
    let tracks = project
        .tracks
        .iter()
        .map(|track| (track.id.as_str(), track))
        .collect::<HashMap<_, _>>();
    let clips = project
        .tracks
        .iter()
        .flat_map(|track| {
            track
                .clips
                .iter()
                .map(move |clip| (clip.id.as_str(), (track, clip)))
        })
        .collect::<HashMap<_, _>>();

    for operation in &proposal.operations {
        match operation {
            AgentEditOperation::AddAssetRange {
                asset_id,
                track_id,
                source_in_ms,
                source_out_ms,
                ..
            } => {
                let asset = assets
                    .get(asset_id.as_str())
                    .ok_or_else(|| format!("找不到素材 ID {asset_id}。"))?;
                let track = tracks
                    .get(track_id.as_str())
                    .ok_or_else(|| format!("找不到轨道 ID {track_id}。"))?;
                validate_unlocked_track(track)?;
                validate_track_compatibility(asset, track)?;
                validate_source_range(asset, *source_in_ms, *source_out_ms)?;
            }
            AgentEditOperation::KeepClipSourceRange {
                clip_id,
                source_in_ms,
                source_out_ms,
                ..
            }
            | AgentEditOperation::RemoveClipSourceRange {
                clip_id,
                source_in_ms,
                source_out_ms,
                ..
            } => {
                let (track, clip) = clips
                    .get(clip_id.as_str())
                    .ok_or_else(|| format!("找不到片段 ID {clip_id}。"))?;
                validate_unlocked_track(track)?;
                let asset = assets
                    .get(clip.asset_id.as_str())
                    .ok_or_else(|| format!("片段 {clip_id} 的素材不存在。"))?;
                validate_source_range(asset, *source_in_ms, *source_out_ms)?;
            }
            AgentEditOperation::SplitClipAtTimeline {
                clip_id,
                timeline_time_ms,
                ..
            } => {
                let (track, clip) = clips
                    .get(clip_id.as_str())
                    .ok_or_else(|| format!("找不到片段 ID {clip_id}。"))?;
                validate_unlocked_track(track)?;
                let end = clip
                    .timeline_start_ms
                    .saturating_add(clip.timeline_duration_ms);
                if *timeline_time_ms <= clip.timeline_start_ms || *timeline_time_ms >= end {
                    return Err(format!("分割点不在片段 {clip_id} 内部。"));
                }
            }
            AgentEditOperation::DeleteClip { clip_id, .. } => {
                let (track, _) = clips
                    .get(clip_id.as_str())
                    .ok_or_else(|| format!("找不到片段 ID {clip_id}。"))?;
                validate_unlocked_track(track)?;
            }
            AgentEditOperation::MoveClip {
                clip_id, track_id, ..
            } => {
                let (source_track, clip) = clips
                    .get(clip_id.as_str())
                    .ok_or_else(|| format!("找不到片段 ID {clip_id}。"))?;
                let target_track = tracks
                    .get(track_id.as_str())
                    .ok_or_else(|| format!("找不到轨道 ID {track_id}。"))?;
                let asset = assets
                    .get(clip.asset_id.as_str())
                    .ok_or_else(|| format!("片段 {clip_id} 的素材不存在。"))?;
                validate_unlocked_track(source_track)?;
                validate_unlocked_track(target_track)?;
                validate_track_compatibility(asset, target_track)?;
            }
        }
    }

    Ok(AgentEditPlan {
        id: next_message_id("plan"),
        project_id: project.project_id.clone(),
        base_editor_version: project.editor_version,
        summary: proposal.summary.trim().to_string(),
        operations: proposal.operations,
        warnings: proposal.warnings,
    })
}

fn validate_unlocked_track(track: &AgentProjectTrack) -> Result<(), String> {
    if track.locked {
        Err(format!("轨道 {} 已锁定。", track.label))
    } else {
        Ok(())
    }
}

fn validate_track_compatibility(
    asset: &AgentProjectAsset,
    track: &AgentProjectTrack,
) -> Result<(), String> {
    let compatible = match asset.asset_type.as_str() {
        "audio" => track.track_type == "audio",
        "caption" => track.track_type == "caption",
        _ => matches!(track.track_type.as_str(), "video" | "overlay"),
    };

    if compatible {
        Ok(())
    } else {
        Err(format!(
            "素材 {} 与轨道 {} 不兼容。",
            asset.name, track.label
        ))
    }
}

fn validate_source_range(
    asset: &AgentProjectAsset,
    source_in_ms: u64,
    source_out_ms: u64,
) -> Result<(), String> {
    if source_out_ms <= source_in_ms {
        return Err(format!("素材 {} 的源区间顺序无效。", asset.name));
    }
    if source_out_ms > asset.duration_ms {
        return Err(format!(
            "素材 {} 的源出点 {}ms 超过时长 {}ms。",
            asset.name, source_out_ms, asset.duration_ms
        ));
    }
    Ok(())
}

fn agent_system_prompt(project: &AgentProjectSnapshot) -> String {
    let dynamic_comic_metadata = serde_json::to_string(&json!({
        "characterVoiceProfiles": project.character_voice_profiles,
        "shots": project.dynamic_comic_shots,
    }))
    .unwrap_or_else(|_| "{}".to_string());
    format!(
        "你是 Linglux 剪辑 Agent。你只能读取工程元数据并提出结构化剪辑计划，不能执行计划、不能操作鼠标、不能运行命令、不能访问文件。\n\
         当前工程 ID：{}，名称：{}，编辑版本：{}。\n\
         时间一律输出非负整数毫秒。用户点名素材时先调用 list_assets；涉及时间线时调用 inspect_timeline；“这个片段”等指代调用 inspect_selection。\n\
         只有 ID 唯一、时间明确且范围有效时才调用 propose_edit_plan。素材未上时间线时可用 addAssetRange；用户未指定轨道时选择主视频轨，未指定时间则省略 timelineStartMs，由本地执行器在空时间线放到 0、否则追加到轨道末尾。\n\
         如果同名素材、同一素材有多个片段、时间表达可能有两种解释、轨道锁定或信息不足，直接用简短中文提出一个澄清问题，不要调用 propose_edit_plan。\n\
         用户说“1:03 前面的不要，3:02 后面的不要”表示保留源区间 63000–182000ms；“结尾 3:02 不要”本身含糊，必须追问。\n\
         动态漫角色与镜头安全元数据：{}",
        project.project_id, project.project_name, project.editor_version, dynamic_comic_metadata
    )
}

fn agent_tools() -> Vec<Value> {
    vec![
        json!({
            "type": "function",
            "function": {
                "name": "list_assets",
                "description": "按名称查询工程素材，只返回安全元数据。",
                "parameters": {
                    "type": "object",
                    "properties": { "query": { "type": "string" } },
                    "additionalProperties": false
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "inspect_timeline",
                "description": "查看时间线轨道和片段，可按素材 ID 过滤。",
                "parameters": {
                    "type": "object",
                    "properties": { "assetId": { "type": "string" } },
                    "additionalProperties": false
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "inspect_selection",
                "description": "读取当前素材、片段选择和播放头。",
                "parameters": {
                    "type": "object",
                    "properties": {},
                    "additionalProperties": false
                }
            }
        }),
        json!({
            "type": "function",
            "function": {
                "name": "propose_edit_plan",
                "description": "提交等待用户确认的精确剪辑计划。绝不直接执行。",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "summary": { "type": "string" },
                        "warnings": { "type": "array", "items": { "type": "string" } },
                        "operations": {
                            "type": "array",
                            "items": {
                                "type": "object",
                                "properties": {
                                    "type": {
                                        "type": "string",
                                        "enum": [
                                            "addAssetRange",
                                            "keepClipSourceRange",
                                            "removeClipSourceRange",
                                            "splitClipAtTimeline",
                                            "deleteClip",
                                            "moveClip"
                                        ]
                                    },
                                    "id": { "type": "string" },
                                    "assetId": { "type": "string" },
                                    "clipId": { "type": "string" },
                                    "trackId": { "type": "string" },
                                    "sourceInMs": { "type": "integer" },
                                    "sourceOutMs": { "type": "integer" },
                                    "timelineStartMs": { "type": "integer" },
                                    "timelineTimeMs": { "type": "integer" }
                                },
                                "required": ["type", "id"],
                                "additionalProperties": false
                            }
                        }
                    },
                    "required": ["summary", "operations", "warnings"],
                    "additionalProperties": false
                }
            }
        }),
    ]
}

fn tool_result_message(tool_call_id: &str, content: Value) -> Value {
    json!({
        "role": "tool",
        "tool_call_id": tool_call_id,
        "content": content.to_string()
    })
}

fn validate_provider_base_url(base_url: &str) -> Result<(), String> {
    let parsed = reqwest::Url::parse(base_url.trim())
        .map_err(|_| "Base URL 必须是有效的 HTTPS 地址。".to_string())?;

    if parsed.scheme() != "https" || parsed.host_str().is_none() {
        return Err("Base URL 必须使用 https://。".to_string());
    }
    if parsed.query().is_some() || parsed.fragment().is_some() {
        return Err("Base URL 不能包含查询参数或片段。".to_string());
    }

    Ok(())
}

fn provider_settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|error| error.to_string())?;
    fs::create_dir_all(&config_dir).map_err(|error| error.to_string())?;
    Ok(config_dir.join("agent-provider-settings.json"))
}

fn legacy_settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|error| error.to_string())?;
    Ok(config_dir.join("api-key-settings.json"))
}

fn load_provider_metadata(app: &AppHandle) -> Option<AgentProviderMetadata> {
    let path = provider_settings_path(app).ok()?;
    let backup = path.with_extension("json.bak");

    [path, backup].into_iter().find_map(|candidate| {
        let bytes = fs::read(candidate).ok()?;
        serde_json::from_slice::<AgentProviderMetadata>(&bytes)
            .ok()
            .map(AgentProviderMetadata::normalized)
    })
}

fn save_provider_metadata(app: &AppHandle, metadata: &AgentProviderMetadata) -> Result<(), String> {
    let path = provider_settings_path(app)?;
    let next = path.with_extension("json.next");
    let backup = path.with_extension("json.bak");
    let bytes = serde_json::to_vec_pretty(metadata).map_err(|error| error.to_string())?;
    let mut file = File::create(&next).map_err(|error| error.to_string())?;
    file.write_all(&bytes).map_err(|error| error.to_string())?;
    file.sync_all().map_err(|error| error.to_string())?;

    if backup.exists() {
        fs::remove_file(&backup).map_err(|error| error.to_string())?;
    }
    if path.exists() {
        fs::rename(&path, &backup).map_err(|error| error.to_string())?;
    }
    if let Err(error) = fs::rename(&next, &path) {
        if backup.exists() && !path.exists() {
            let _ = fs::rename(&backup, &path);
        }
        return Err(error.to_string());
    }

    Ok(())
}

fn load_and_migrate_provider_metadata(
    app: &AppHandle,
    runtime: &AgentRuntime,
) -> Result<AgentProviderMetadata, String> {
    if let Some(metadata) = load_provider_metadata(app) {
        return Ok(metadata);
    }

    let legacy_path = legacy_settings_path(app)?;
    let legacy = fs::read(&legacy_path)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<LegacyApiKeySettings>(&bytes).ok());

    let metadata = if let Some(mut legacy) = legacy {
        let provider = if legacy.provider == "openai" {
            "openai".to_string()
        } else {
            legacy.provider.clone()
        };
        let metadata = AgentProviderMetadata {
            provider: provider.clone(),
            base_url: legacy.base_url.clone(),
            model: if provider == "deepseek" {
                "deepseek-v4-flash".to_string()
            } else {
                "gpt-5-mini".to_string()
            },
        }
        .normalized();
        let api_key = legacy.api_key.trim().to_string();

        if !api_key.is_empty() {
            runtime.store_api_key(&metadata.provider, api_key);
            legacy.api_key.clear();
            let sanitized =
                serde_json::to_vec_pretty(&legacy).map_err(|error| error.to_string())?;
            fs::write(&legacy_path, sanitized).map_err(|error| error.to_string())?;
        }

        metadata
    } else {
        AgentProviderMetadata::default()
    };

    save_provider_metadata(app, &metadata)?;
    Ok(metadata)
}

fn provider_summary(
    metadata: &AgentProviderMetadata,
    runtime: &AgentRuntime,
) -> AgentProviderSettingsSummary {
    let api_key = load_provider_api_key(&metadata.provider, runtime);
    AgentProviderSettingsSummary {
        provider: metadata.provider.clone(),
        base_url: metadata.base_url.clone(),
        model: metadata.model.clone(),
        has_api_key: api_key.is_some(),
        masked_api_key: api_key
            .as_deref()
            .map(mask_api_key)
            .unwrap_or_else(|| "未配置".to_string()),
    }
}

fn load_provider_api_key(provider: &str, runtime: &AgentRuntime) -> Option<String> {
    runtime.load_api_key(provider)
}

fn mask_api_key(api_key: &str) -> String {
    let chars = api_key.chars().collect::<Vec<_>>();
    if chars.len() <= 6 {
        return "••••••".to_string();
    }
    let start = chars.iter().take(3).collect::<String>();
    let end = chars.iter().rev().take(3).rev().collect::<String>();
    format!("{start}••••{end}")
}

fn load_conversation(core: &MediaCore, project_id: &str) -> AgentConversation {
    core.projects()
        .load_agent_conversation(project_id)
        .and_then(|value| serde_json::from_value::<AgentConversation>(value).ok())
        .filter(|conversation| {
            conversation.project_id == project_id
                && conversation.schema_version <= AGENT_CONVERSATION_SCHEMA_VERSION
        })
        .unwrap_or_else(|| AgentConversation::empty(project_id))
}

fn save_conversation(core: &MediaCore, conversation: &mut AgentConversation) -> Result<(), String> {
    conversation.schema_version = AGENT_CONVERSATION_SCHEMA_VERSION;
    conversation.updated_at = now_stamp();
    trim_conversation(conversation);
    let value = serde_json::to_value(&conversation).map_err(|error| error.to_string())?;
    core.projects()
        .save_agent_conversation(&conversation.project_id, &value)
}

fn trim_conversation(conversation: &mut AgentConversation) {
    if conversation.messages.len() > AGENT_MAX_MESSAGES {
        let remove_count = conversation.messages.len() - AGENT_MAX_MESSAGES;
        conversation.messages.drain(0..remove_count);
    }

    while !conversation.messages.is_empty()
        && serde_json::to_vec(conversation)
            .map(|bytes| bytes.len() > AGENT_MAX_CONVERSATION_BYTES)
            .unwrap_or(false)
    {
        conversation.messages.remove(0);
    }
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or_default()
}

fn now_stamp() -> String {
    format!("unix-{}", now_millis())
}

fn next_message_id(prefix: &str) -> String {
    format!("{prefix}-{}", now_millis())
}

fn redact_provider_error(error: &str, api_key: &str) -> String {
    if api_key.is_empty() {
        return error.to_string();
    }
    error.replace(api_key, "[REDACTED]")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    struct UnavailableSecretStore;

    impl SecretStore for UnavailableSecretStore {
        fn get(&self, _provider: &str) -> Result<Option<String>, String> {
            Err("secret store unavailable".to_string())
        }

        fn set(&self, _provider: &str, _api_key: &str) -> Result<(), String> {
            Err("secret store unavailable".to_string())
        }

        fn delete(&self, _provider: &str) -> Result<(), String> {
            Err("secret store unavailable".to_string())
        }
    }

    fn mock_provider(responses: Vec<(u16, String, Duration)>) -> (String, thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock provider");
        let address = listener.local_addr().expect("mock provider address");
        let server = thread::spawn(move || {
            for (status, body, delay) in responses {
                let (mut stream, _) = listener.accept().expect("accept request");
                let mut request = [0_u8; 16384];
                let _ = stream.read(&mut request).expect("read request");

                if !delay.is_zero() {
                    thread::sleep(delay);
                }

                let response = format!(
                    "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });

        (format!("http://{address}"), server)
    }

    fn test_project() -> AgentProjectSnapshot {
        AgentProjectSnapshot {
            project_id: "project-test".to_string(),
            project_name: "Test".to_string(),
            editor_version: 4,
            duration_ms: 240_000,
            playhead_ms: 0,
            selected_clip_id: None,
            selected_asset_ids: vec![],
            main_track_magnet_enabled: true,
            assets: vec![AgentProjectAsset {
                id: "asset-x".to_string(),
                name: "x.mp4".to_string(),
                asset_type: "video".to_string(),
                duration_ms: 240_000,
            }],
            tracks: vec![AgentProjectTrack {
                id: "track-video".to_string(),
                label: "视频轨".to_string(),
                track_type: "video".to_string(),
                locked: false,
                clips: vec![],
            }],
            character_voice_profiles: vec![],
            dynamic_comic_shots: vec![],
        }
    }

    #[test]
    fn validates_example_source_range() {
        assert!(validate_provider_base_url("https://api.deepseek.com").is_ok());
        assert!(validate_provider_base_url("http://api.deepseek.com").is_err());
        assert!(validate_provider_base_url("https://api.deepseek.com?key=secret").is_err());

        let proposal = ProposedEditPlan {
            summary: "保留 x.mp4 的 1:03–3:02".to_string(),
            operations: vec![AgentEditOperation::AddAssetRange {
                id: "add-x".to_string(),
                asset_id: "asset-x".to_string(),
                track_id: "track-video".to_string(),
                source_in_ms: 63_000,
                source_out_ms: 182_000,
                timeline_start_ms: None,
            }],
            warnings: vec![],
        };
        let plan = validate_proposed_plan(&test_project(), proposal).expect("valid plan");

        assert_eq!(plan.base_editor_version, 4);
        assert_eq!(plan.operations.len(), 1);
        let serialized = serde_json::to_value(&plan).expect("serialize plan");
        assert_eq!(serialized["operations"][0]["assetId"], "asset-x");
        assert!(serialized["operations"][0].get("asset_id").is_none());
    }

    #[test]
    fn rejects_unknown_and_out_of_range_assets() {
        let mut proposal = ProposedEditPlan {
            summary: "invalid".to_string(),
            operations: vec![AgentEditOperation::AddAssetRange {
                id: "bad".to_string(),
                asset_id: "asset-missing".to_string(),
                track_id: "track-video".to_string(),
                source_in_ms: 0,
                source_out_ms: 400_000,
                timeline_start_ms: None,
            }],
            warnings: vec![],
        };
        assert!(validate_proposed_plan(&test_project(), proposal.clone()).is_err());

        if let AgentEditOperation::AddAssetRange { asset_id, .. } = &mut proposal.operations[0] {
            *asset_id = "asset-x".to_string();
        }
        assert!(validate_proposed_plan(&test_project(), proposal).is_err());
    }

    #[test]
    fn trims_conversation_by_count_and_redacts_errors() {
        let mut conversation = AgentConversation::empty("project-test");
        for index in 0..220 {
            conversation.messages.push(AgentChatMessage {
                id: format!("message-{index}"),
                role: "user".to_string(),
                content: "test".to_string(),
                created_at: now_stamp(),
                plan: None,
                plan_state: None,
            });
        }
        trim_conversation(&mut conversation);

        assert_eq!(conversation.messages.len(), AGENT_MAX_MESSAGES);
        assert_eq!(
            redact_provider_error("request used secret-key", "secret-key"),
            "request used [REDACTED]"
        );

        conversation.messages = vec![AgentChatMessage {
            id: "oversized".to_string(),
            role: "assistant".to_string(),
            content: "x".repeat(AGENT_MAX_CONVERSATION_BYTES + 1),
            created_at: now_stamp(),
            plan: None,
            plan_state: None,
        }];
        trim_conversation(&mut conversation);
        assert!(conversation.messages.is_empty());
    }

    #[test]
    fn uses_session_only_secret_when_system_store_is_unavailable() {
        let client = Client::builder().build().expect("client");
        let runtime = AgentRuntime::with_parts(client, Arc::new(UnavailableSecretStore));

        runtime.store_api_key("deepseek", "session-secret".to_string());
        assert_eq!(
            runtime.load_api_key("deepseek").as_deref(),
            Some("session-secret")
        );
        assert!(runtime.clear_api_key("deepseek").is_ok());
        assert!(runtime.load_api_key("deepseek").is_none());
    }

    #[test]
    fn enforces_one_running_turn_per_project_and_releases_on_cancel() {
        let client = Client::builder().build().expect("client");
        let runtime = AgentRuntime::with_parts(client, Arc::new(UnavailableSecretStore));

        assert!(runtime.try_register_pending("task-1", "project-1"));
        assert!(!runtime.try_register_pending("task-2", "project-1"));
        let handle = tauri::async_runtime::spawn(std::future::pending::<()>());
        runtime.attach_handle("task-1", handle);
        assert!(runtime.abort("task-1"));
        assert!(runtime.try_register_pending("task-2", "project-1"));
        runtime.finish("task-2");

        assert!(runtime.try_register_pending("task-3", "project-1"));
        assert!(runtime.abort("task-3"));
        let pending_handle = tauri::async_runtime::spawn(std::future::pending::<()>());
        runtime.attach_handle("task-3", pending_handle);
        assert!(runtime.try_register_pending("task-4", "project-1"));
        runtime.finish("task-4");
    }

    #[test]
    fn normalizes_401_429_and_timeout_provider_failures() {
        for status in [401_u16, 429_u16] {
            let (base_url, server) = mock_provider(vec![(
                status,
                json!({ "error": { "message": "provider rejected request" } }).to_string(),
                Duration::ZERO,
            )]);
            let client = Client::builder()
                .timeout(Duration::from_secs(2))
                .build()
                .expect("client");
            let settings = AgentProviderMetadata {
                provider: "custom".to_string(),
                base_url,
                model: "test-model".to_string(),
            };
            let error = tauri::async_runtime::block_on(request_provider_message(
                &client,
                &settings,
                "secret",
                &[json!({ "role": "user", "content": "test" })],
                &[],
            ))
            .expect_err("provider status should fail");

            assert!(error.contains(&format!("HTTP {status}")));
            server.join().expect("mock server");
        }

        let (base_url, server) = mock_provider(vec![(
            200,
            json!({ "choices": [] }).to_string(),
            Duration::from_millis(120),
        )]);
        let client = Client::builder()
            .timeout(Duration::from_millis(30))
            .build()
            .expect("client");
        let settings = AgentProviderMetadata {
            provider: "custom".to_string(),
            base_url,
            model: "test-model".to_string(),
        };
        let error = tauri::async_runtime::block_on(request_provider_message(
            &client,
            &settings,
            "secret",
            &[json!({ "role": "user", "content": "test" })],
            &[],
        ))
        .expect_err("timeout should fail");

        assert!(error.contains("模型服务连接失败"));
        server.join().expect("mock server");
    }

    #[test]
    fn repairs_malformed_tool_arguments_once() {
        let malformed = json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call-invalid",
                        "type": "function",
                        "function": {
                            "name": "propose_edit_plan",
                            "arguments": "{invalid"
                        }
                    }]
                }
            }]
        })
        .to_string();
        let repaired_arguments = json!({
            "summary": "保留 x.mp4 的 1:03–3:02",
            "warnings": [],
            "operations": [{
                "id": "add-range",
                "type": "addAssetRange",
                "assetId": "asset-x",
                "trackId": "track-video",
                "sourceInMs": 63000,
                "sourceOutMs": 182000
            }]
        })
        .to_string();
        let repaired = json!({
            "choices": [{
                "message": {
                    "role": "assistant",
                    "content": null,
                    "tool_calls": [{
                        "id": "call-repaired",
                        "type": "function",
                        "function": {
                            "name": "propose_edit_plan",
                            "arguments": repaired_arguments
                        }
                    }]
                }
            }]
        })
        .to_string();
        let (base_url, server) = mock_provider(vec![
            (200, malformed, Duration::ZERO),
            (200, repaired, Duration::ZERO),
        ]);
        let client = Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .expect("client");
        let settings = AgentProviderMetadata {
            provider: "custom".to_string(),
            base_url,
            model: "test-model".to_string(),
        };
        let outcome = tauri::async_runtime::block_on(run_agent_model_turn(
            &client,
            &settings,
            "secret",
            &test_project(),
            &AgentConversation::empty("project-test"),
        ))
        .expect("repair plan");

        match outcome {
            AgentModelOutcome::Plan(plan) => assert_eq!(plan.operations.len(), 1),
            AgentModelOutcome::Clarification(_) => panic!("expected plan"),
        }
        server.join().expect("mock server");
    }

    #[test]
    fn parses_openai_compatible_tool_responses_from_mock_server() {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind mock provider");
        let address = listener.local_addr().expect("mock provider address");
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().expect("accept request");
            let mut request = [0_u8; 8192];
            let count = stream.read(&mut request).expect("read request");
            let request = String::from_utf8_lossy(&request[..count]);
            assert!(request
                .to_ascii_lowercase()
                .contains("authorization: bearer test-secret"));
            assert!(request.contains("\"model\":\"test-model\""));

            let body = json!({
                "choices": [{
                    "message": {
                        "role": "assistant",
                        "content": null,
                        "tool_calls": [{
                            "id": "call-1",
                            "type": "function",
                            "function": {
                                "name": "inspect_selection",
                                "arguments": "{}"
                            }
                        }]
                    }
                }]
            })
            .to_string();
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            stream
                .write_all(response.as_bytes())
                .expect("write response");
        });
        let client = Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .expect("mock client");
        let settings = AgentProviderMetadata {
            provider: "custom".to_string(),
            base_url: format!("http://{address}"),
            model: "test-model".to_string(),
        };
        let message = tauri::async_runtime::block_on(request_provider_message(
            &client,
            &settings,
            "test-secret",
            &[json!({ "role": "user", "content": "test" })],
            &[],
        ))
        .expect("provider message");

        assert_eq!(
            message["tool_calls"][0]["function"]["name"],
            "inspect_selection"
        );
        server.join().expect("mock server");
    }
}
