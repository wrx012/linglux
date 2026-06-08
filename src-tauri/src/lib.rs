use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiKeySettings {
    provider: String,
    base_url: String,
    api_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EditorSessionSeed {
    source_node_id: Option<String>,
    asset_name: Option<String>,
    asset_url: Option<String>,
    duration: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Artifact {
    id: String,
    #[serde(rename = "type")]
    artifact_type: String,
    name: String,
    source_node_id: String,
    url: String,
    duration: f64,
    created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MediaAsset {
    id: String,
    #[serde(rename = "type")]
    asset_type: String,
    name: String,
    source_node_id: Option<String>,
    url: String,
    duration: f64,
    width: Option<u32>,
    height: Option<u32>,
    created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClipTransform {
    x: f64,
    y: f64,
    scale: f64,
    rotation: f64,
    opacity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClipEffect {
    id: String,
    #[serde(rename = "type")]
    effect_type: String,
    label: String,
    intensity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TimelineClip {
    id: String,
    asset_id: String,
    track_id: String,
    name: String,
    #[serde(rename = "type")]
    clip_type: String,
    start: f64,
    duration: f64,
    trim_start: f64,
    trim_end: f64,
    volume: f64,
    muted: bool,
    speed: f64,
    transform: ClipTransform,
    effects: Vec<ClipEffect>,
    transition: Option<String>,
    caption_text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TimelineTrack {
    id: String,
    #[serde(rename = "type")]
    track_type: String,
    label: String,
    muted: bool,
    locked: bool,
    clips: Vec<TimelineClip>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EditorResolution {
    width: u32,
    height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EditorProject {
    id: String,
    name: String,
    source_node_id: Option<String>,
    assets: Vec<MediaAsset>,
    tracks: Vec<TimelineTrack>,
    duration: f64,
    fps: u32,
    resolution: EditorResolution,
    created_at: String,
    updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EditSession {
    id: String,
    source_node_id: Option<String>,
    project: EditorProject,
    saved_at: Option<String>,
    is_dirty: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ExportPreset {
    id: String,
    label: String,
    format: String,
    resolution: String,
    fps: u32,
    quality: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EditorExportRequest {
    session_id: String,
    project: EditorProject,
    preset: ExportPreset,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EditorExportResult {
    artifact: Artifact,
    preset: ExportPreset,
}

impl Default for ApiKeySettings {
    fn default() -> Self {
        Self {
            provider: "openai".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            api_key: String::new(),
        }
    }
}

impl ApiKeySettings {
    fn normalized(mut self) -> Self {
        let fallback = Self::default();

        self.provider = self.provider.trim().to_string();
        self.base_url = self.base_url.trim().trim_end_matches('/').to_string();
        self.api_key = self.api_key.trim().to_string();

        if self.provider.is_empty() {
            self.provider = fallback.provider;
        }

        if self.base_url.is_empty() {
            self.base_url = fallback.base_url;
        }

        self
    }
}

fn api_key_settings_path(app: &AppHandle) -> Result<PathBuf, String> {
    let config_dir = app
        .path()
        .app_config_dir()
        .map_err(|error| error.to_string())?;
    std::fs::create_dir_all(&config_dir).map_err(|error| error.to_string())?;
    Ok(config_dir.join("api-key-settings.json"))
}

fn now_stamp() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();

    format!("unix-{seconds}")
}

fn default_transform(opacity: f64) -> ClipTransform {
    ClipTransform {
        x: 0.0,
        y: 0.0,
        scale: 1.0,
        rotation: 0.0,
        opacity,
    }
}

fn default_effects() -> Vec<ClipEffect> {
    vec![
        ClipEffect {
            id: "effect-color-base".to_string(),
            effect_type: "color".to_string(),
            label: "基础调色".to_string(),
            intensity: 32.0,
        },
        ClipEffect {
            id: "effect-soft-cut".to_string(),
            effect_type: "transition".to_string(),
            label: "柔和转场".to_string(),
            intensity: 18.0,
        },
    ]
}

fn create_clip(
    id: &str,
    asset_id: &str,
    track_id: &str,
    name: &str,
    clip_type: &str,
    start: f64,
    duration: f64,
    volume: f64,
    opacity: f64,
    caption_text: Option<String>,
) -> TimelineClip {
    TimelineClip {
        id: id.to_string(),
        asset_id: asset_id.to_string(),
        track_id: track_id.to_string(),
        name: name.to_string(),
        clip_type: clip_type.to_string(),
        start,
        duration,
        trim_start: 0.0,
        trim_end: 0.0,
        volume,
        muted: false,
        speed: 1.0,
        transform: default_transform(opacity),
        effects: default_effects(),
        transition: if clip_type == "video" || clip_type == "overlay" {
            Some("soft".to_string())
        } else {
            None
        },
        caption_text,
    }
}

fn create_mock_project(seed: EditorSessionSeed, now: String) -> EditorProject {
    let source_node_id = seed.source_node_id;
    let duration = seed.duration.unwrap_or(12.0).max(1.0);
    let asset_name = seed
        .asset_name
        .unwrap_or_else(|| "linglux_generated_take.mp4".to_string());
    let asset_url = seed
        .asset_url
        .unwrap_or_else(|| "linglux://editor/seed-video".to_string());
    let asset_id = source_node_id
        .as_ref()
        .map(|id| format!("asset-{id}"))
        .unwrap_or_else(|| "asset-editor-seed".to_string());

    let video_asset = MediaAsset {
        id: asset_id.clone(),
        asset_type: "video".to_string(),
        name: asset_name.clone(),
        source_node_id: source_node_id.clone(),
        url: asset_url,
        duration,
        width: Some(1920),
        height: Some(1080),
        created_at: now.clone(),
    };
    let audio_asset = MediaAsset {
        id: "asset-room-tone".to_string(),
        asset_type: "audio".to_string(),
        name: "ambient_mix.wav".to_string(),
        source_node_id: None,
        url: "linglux://asset/ambient_mix.wav".to_string(),
        duration,
        width: None,
        height: None,
        created_at: now.clone(),
    };
    let caption_asset = MediaAsset {
        id: "asset-caption-bed".to_string(),
        asset_type: "caption".to_string(),
        name: "字幕轨".to_string(),
        source_node_id: None,
        url: "linglux://caption/default".to_string(),
        duration: 4.0,
        width: None,
        height: None,
        created_at: now.clone(),
    };

    EditorProject {
        id: format!("project-{}", now.replace("unix-", "")),
        name: source_node_id
            .as_ref()
            .map(|id| format!("剪辑会话 · {id}"))
            .unwrap_or_else(|| "Linglux 剪辑工程".to_string()),
        source_node_id,
        assets: vec![video_asset, audio_asset, caption_asset],
        tracks: vec![
            TimelineTrack {
                id: "track-overlay".to_string(),
                track_type: "overlay".to_string(),
                label: "叠加轨".to_string(),
                muted: false,
                locked: false,
                clips: vec![create_clip(
                    "clip-overlay-guide",
                    &asset_id,
                    "track-overlay",
                    "片头叠加",
                    "overlay",
                    0.6,
                    2.6,
                    1.0,
                    0.42,
                    None,
                )],
            },
            TimelineTrack {
                id: "track-video".to_string(),
                track_type: "video".to_string(),
                label: "视频轨".to_string(),
                muted: false,
                locked: false,
                clips: vec![create_clip(
                    "clip-main-video",
                    &asset_id,
                    "track-video",
                    &asset_name,
                    "video",
                    0.0,
                    duration,
                    1.0,
                    1.0,
                    None,
                )],
            },
            TimelineTrack {
                id: "track-caption".to_string(),
                track_type: "caption".to_string(),
                label: "字幕轨".to_string(),
                muted: false,
                locked: false,
                clips: vec![create_clip(
                    "clip-caption-1",
                    "asset-caption-bed",
                    "track-caption",
                    "主字幕",
                    "caption",
                    1.0,
                    4.0,
                    1.0,
                    1.0,
                    Some("Linglux 自动剪辑草稿".to_string()),
                )],
            },
            TimelineTrack {
                id: "track-audio".to_string(),
                track_type: "audio".to_string(),
                label: "音频轨".to_string(),
                muted: false,
                locked: false,
                clips: vec![create_clip(
                    "clip-audio-bed",
                    "asset-room-tone",
                    "track-audio",
                    "ambient_mix.wav",
                    "audio",
                    0.0,
                    duration,
                    0.72,
                    1.0,
                    None,
                )],
            },
        ],
        duration,
        fps: 30,
        resolution: EditorResolution {
            width: 1920,
            height: 1080,
        },
        created_at: now.clone(),
        updated_at: now,
    }
}

#[tauri::command]
fn create_video_plan(prompt: String) -> String {
    let subject = prompt.trim();

    if subject.is_empty() {
        return "Add a video brief, then Linglux will create a production plan.".to_string();
    }

    format!(
        "Production plan for {subject}\n\
         1. Write a hook that shows the outcome in the first 3 seconds.\n\
         2. Sort source footage by scene intent, motion, and clarity.\n\
         3. Assemble a rough cut with a hero moment, support moments, and a short end card.\n\
         4. Run enhancement passes for color, sound balance, captions, and export readiness."
    )
}

#[tauri::command]
fn load_api_key_settings(app: AppHandle) -> Result<ApiKeySettings, String> {
    let settings_path = api_key_settings_path(&app)?;

    if !settings_path.exists() {
        return Ok(ApiKeySettings::default());
    }

    let settings = std::fs::read_to_string(settings_path).map_err(|error| error.to_string())?;
    let settings = serde_json::from_str::<ApiKeySettings>(&settings)
        .map_err(|error| error.to_string())?
        .normalized();

    Ok(settings)
}

#[tauri::command]
fn save_api_key_settings(
    app: AppHandle,
    settings: ApiKeySettings,
) -> Result<ApiKeySettings, String> {
    let settings = settings.normalized();

    if settings.api_key.is_empty() {
        return Err("API Key 不能为空".to_string());
    }

    let settings_path = api_key_settings_path(&app)?;
    let settings_json =
        serde_json::to_string_pretty(&settings).map_err(|error| error.to_string())?;
    std::fs::write(settings_path, settings_json).map_err(|error| error.to_string())?;

    Ok(settings)
}

#[tauri::command]
fn clear_api_key_settings(app: AppHandle) -> Result<(), String> {
    let settings_path = api_key_settings_path(&app)?;

    match std::fs::remove_file(settings_path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.to_string()),
    }
}

#[tauri::command]
fn create_edit_session(seed: EditorSessionSeed) -> EditSession {
    let now = now_stamp();
    let project = create_mock_project(seed, now.clone());

    EditSession {
        id: format!("session-{}", now.replace("unix-", "")),
        source_node_id: project.source_node_id.clone(),
        project,
        saved_at: Some(now),
        is_dirty: false,
    }
}

#[tauri::command]
fn load_edit_project(session_id: String) -> EditorProject {
    let now = now_stamp();
    let seed = EditorSessionSeed {
        source_node_id: Some(session_id),
        asset_name: None,
        asset_url: None,
        duration: Some(12.0),
    };

    create_mock_project(seed, now)
}

#[tauri::command]
fn save_edit_project(session_id: String, mut project: EditorProject) -> EditorProject {
    let _session_id = session_id;
    project.updated_at = now_stamp();
    project
}

#[tauri::command]
fn export_edit_project(request: EditorExportRequest) -> EditorExportResult {
    let now = now_stamp();
    let source_node_id = request
        .project
        .source_node_id
        .clone()
        .unwrap_or_else(|| request.session_id.clone());
    let extension = request.preset.format.clone();

    EditorExportResult {
        artifact: Artifact {
            id: format!("artifact-edit-{}", now.replace("unix-", "")),
            artifact_type: "video".to_string(),
            name: format!("{} · edited.{extension}", request.project.name),
            source_node_id,
            url: format!("linglux://exports/{}.{}", request.project.id, extension),
            duration: request.project.duration,
            created_at: now,
        },
        preset: request.preset,
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            create_video_plan,
            load_api_key_settings,
            save_api_key_settings,
            clear_api_key_settings,
            create_edit_session,
            load_edit_project,
            save_edit_project,
            export_edit_project
        ])
        .run(tauri::generate_context!())
        .expect("error while running Linglux");
}
