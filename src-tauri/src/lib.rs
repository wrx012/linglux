mod agent;

use agent::{
    cancel_editor_agent_turn, clear_agent_conversation, clear_agent_provider_settings,
    load_agent_conversation, load_agent_provider_settings, save_agent_provider_settings,
    start_editor_agent_turn, update_agent_plan_state, AgentRuntime,
};
use linglux_media_core::{
    discover_ffmpeg_binary, generate_media_derivatives, import_media_paths,
    run_ffmpeg_process as run_ffmpeg, storyboard_to_video, MediaCore, MediaKind,
    SpeechSynthesisRequest, StoryboardToVideoRequest, TaskEvent, TaskHandle, TaskKind,
    TaskSnapshot, TaskState, TtsStatus,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::ipc::Channel;
use tauri::{AppHandle, Manager, State};

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
    file_path: Option<String>,
    proxy_path: Option<String>,
    content_fingerprint: Option<String>,
    thumbnail_url: Option<String>,
    #[serde(default)]
    waveform_peaks: Option<Vec<f64>>,
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
struct AudioBeatMarker {
    time: f64,
    intensity: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TextClipStyle {
    font_family: String,
    font_size: f64,
    color: String,
    letter_spacing: f64,
    line_height: f64,
    background_enabled: bool,
    background_color: String,
    #[serde(default = "default_text_background_width")]
    background_width: f64,
    #[serde(default = "default_text_background_height")]
    background_height: f64,
    #[serde(default)]
    background_x_offset: f64,
    #[serde(default)]
    background_y_offset: f64,
    #[serde(default)]
    background_corner_radius: f64,
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
    #[serde(default = "default_true")]
    visible: bool,
    speed: f64,
    transform: ClipTransform,
    effects: Vec<ClipEffect>,
    transition: Option<String>,
    caption_text: Option<String>,
    text_style: Option<TextClipStyle>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    beat_mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    beat_markers: Option<Vec<AudioBeatMarker>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TimelineTrack {
    id: String,
    #[serde(rename = "type")]
    track_type: String,
    label: String,
    muted: bool,
    #[serde(default = "default_true")]
    visible: bool,
    #[serde(default = "default_true")]
    media_enabled: bool,
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
    #[serde(default = "default_true")]
    main_track_magnet_enabled: bool,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    output_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    manifest_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    warnings: Option<Vec<String>>,
}

struct RenderClip<'a> {
    clip: &'a TimelineClip,
    asset: &'a MediaAsset,
}

fn now_stamp() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();

    format!("unix-{seconds}")
}

fn default_true() -> bool {
    true
}

fn default_text_background_width() -> f64 {
    30.0
}

fn default_text_background_height() -> f64 {
    42.0
}

fn linglux_project_root_candidate(path: &Path) -> Option<PathBuf> {
    if path.join("package.json").exists() && path.join("src-tauri").join("Cargo.toml").exists() {
        return Some(path.to_path_buf());
    }

    let parent = path.parent()?;

    if path.file_name().and_then(|name| name.to_str()) == Some("src-tauri")
        && parent.join("package.json").exists()
        && path.join("Cargo.toml").exists()
    {
        return Some(parent.to_path_buf());
    }

    None
}

fn development_project_root() -> Option<PathBuf> {
    if let Ok(current_dir) = std::env::current_dir() {
        if let Some(root) = linglux_project_root_candidate(&current_dir) {
            return Some(root);
        }

        if let Some(parent) = current_dir.parent() {
            if let Some(root) = linglux_project_root_candidate(parent) {
                return Some(root);
            }
        }
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    linglux_project_root_candidate(&manifest_dir)
}

fn export_output_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = if let Some(root) = development_project_root() {
        root.join("output")
    } else {
        app.path()
            .app_data_dir()
            .map_err(|error| error.to_string())?
            .join("output")
    };

    fs::create_dir_all(&dir).map_err(|error| error.to_string())?;
    Ok(dir)
}

fn sanitize_file_name(file_name: &str) -> String {
    let cleaned = file_name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '.' | '-' | '_') {
                character
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('.')
        .to_string();

    if cleaned.is_empty() {
        "linglux-asset.bin".to_string()
    } else {
        cleaned
    }
}

fn unique_child_path(dir: &Path, file_name: &str) -> PathBuf {
    let safe_name = sanitize_file_name(file_name);
    let timestamp = now_stamp().replace("unix-", "");
    let base_path = dir.join(format!("{timestamp}-{safe_name}"));

    if !base_path.exists() {
        return base_path;
    }

    let path = Path::new(&safe_name);
    let stem = path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("linglux-asset");
    let extension = path.extension().and_then(|value| value.to_str());

    for index in 1..1000 {
        let candidate_name = match extension {
            Some(extension) => format!("{timestamp}-{stem}-{index}.{extension}"),
            None => format!("{timestamp}-{stem}-{index}"),
        };
        let candidate = dir.join(candidate_name);

        if !candidate.exists() {
            return candidate;
        }
    }

    dir.join(format!("{timestamp}-{safe_name}"))
}

fn find_ffmpeg() -> Option<PathBuf> {
    discover_ffmpeg_binary("ffmpeg")
}

fn find_ffprobe() -> Option<PathBuf> {
    discover_ffmpeg_binary("ffprobe")
}

fn preset_resolution(preset: &ExportPreset, project: &EditorProject) -> (u32, u32) {
    match preset.resolution.as_str() {
        "720p" => (1280, 720),
        "1080p" => (1920, 1080),
        "1440p" => (2560, 1440),
        "4k" => (3840, 2160),
        _ => (
            project.resolution.width.max(1),
            project.resolution.height.max(1),
        ),
    }
}

fn export_extension(preset: &ExportPreset) -> Result<&str, String> {
    match preset.format.as_str() {
        "mp4" => Ok("mp4"),
        "webm" => Ok("webm"),
        "mov" => Ok("mov"),
        _ => Err(format!("不支持的导出格式：{}", preset.format)),
    }
}

fn project_export_duration(project: &EditorProject) -> f64 {
    let track_duration = project
        .tracks
        .iter()
        .flat_map(|track| track.clips.iter())
        .filter(|clip| clip.visible && clip.duration.is_finite() && clip.duration > 0.0)
        .fold(0.0_f64, |duration, clip| {
            duration.max(clip.start.max(0.0) + clip.duration.max(0.0))
        });

    project.duration.max(track_duration)
}

fn has_exportable_timeline(project: &EditorProject) -> bool {
    project.tracks.iter().any(|track| {
        track.visible
            && track.media_enabled
            && track
                .clips
                .iter()
                .any(|clip| clip.visible && clip.duration.is_finite() && clip.duration > 0.0)
    })
}

fn resolve_asset_path(asset: &MediaAsset) -> Option<PathBuf> {
    if let Some(path) = asset.file_path.as_ref().map(PathBuf::from) {
        if path.exists() {
            return Some(path);
        }
    }

    if let Some(file_url_path) = asset.url.strip_prefix("file://") {
        let path = PathBuf::from(file_url_path);

        if path.exists() {
            return Some(path);
        }
    }

    let url_path = PathBuf::from(&asset.url);

    if url_path.is_absolute() && url_path.exists() {
        return Some(url_path);
    }

    None
}

fn asset_path_or_error(asset: &MediaAsset) -> Result<PathBuf, String> {
    resolve_asset_path(asset).ok_or_else(|| {
        format!(
            "素材「{}」没有可导出的本地文件，请重新导入后再导出。",
            asset.name
        )
    })
}

fn is_virtual_audio_preset(asset: &MediaAsset) -> bool {
    asset.url.starts_with("linglux://audio-presets/")
}

fn media_file_has_audio_stream(ffmpeg: &Path, ffprobe: Option<&Path>, path: &Path) -> bool {
    if let Some(ffprobe) = ffprobe {
        let output = Command::new(ffprobe)
            .args([
                "-v",
                "error",
                "-select_streams",
                "a:0",
                "-show_entries",
                "stream=index",
                "-of",
                "csv=p=0",
            ])
            .arg(path)
            .output();

        if let Ok(output) = output {
            if output.status.success() {
                return !output.stdout.is_empty();
            }
        }
    }

    let output = Command::new(ffmpeg)
        .args(["-hide_banner", "-i"])
        .arg(path)
        .output();

    output
        .map(|output| String::from_utf8_lossy(&output.stderr).contains("Audio:"))
        .unwrap_or(false)
}

fn collect_visual_clips<'a>(project: &'a EditorProject) -> Vec<RenderClip<'a>> {
    let mut clips = project
        .tracks
        .iter()
        .filter(|track| track.visible && track.media_enabled)
        .filter(|track| matches!(track.track_type.as_str(), "video" | "overlay"))
        .flat_map(|track| {
            track.clips.iter().filter_map(|clip| {
                let asset = project
                    .assets
                    .iter()
                    .find(|asset| asset.id == clip.asset_id)?;
                let is_visual_asset = matches!(asset.asset_type.as_str(), "video" | "image");
                let is_visual_clip = matches!(clip.clip_type.as_str(), "video" | "overlay");

                if clip.visible
                    && is_visual_asset
                    && is_visual_clip
                    && clip.duration.is_finite()
                    && clip.duration > 0.0
                {
                    Some(RenderClip { clip, asset })
                } else {
                    None
                }
            })
        })
        .collect::<Vec<_>>();

    clips.sort_by(|left, right| {
        left.clip
            .start
            .partial_cmp(&right.clip.start)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    clips
}

fn collect_caption_clips<'a>(project: &'a EditorProject) -> Vec<&'a TimelineClip> {
    let mut clips = project
        .tracks
        .iter()
        .filter(|track| track.visible && track.media_enabled)
        .filter(|track| track.track_type == "caption")
        .flat_map(|track| {
            track.clips.iter().filter(|clip| {
                clip.visible
                    && clip.duration.is_finite()
                    && clip.duration > 0.0
                    && clip.caption_text.as_deref().unwrap_or("").trim().len() > 0
            })
        })
        .collect::<Vec<_>>();

    clips.sort_by(|left, right| {
        left.start
            .partial_cmp(&right.start)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    clips
}

fn collect_audio_clips<'a>(
    project: &'a EditorProject,
    ffmpeg: &Path,
    ffprobe: Option<&Path>,
    warnings: &mut Vec<String>,
) -> Result<Vec<RenderClip<'a>>, String> {
    let mut clips = Vec::new();

    for track in project
        .tracks
        .iter()
        .filter(|track| track.visible && track.media_enabled && !track.muted)
    {
        for clip in track.clips.iter().filter(|clip| {
            clip.visible
                && !clip.muted
                && clip.volume > 0.0
                && clip.duration.is_finite()
                && clip.duration > 0.0
        }) {
            let Some(asset) = project
                .assets
                .iter()
                .find(|asset| asset.id == clip.asset_id)
            else {
                continue;
            };
            let has_audio_stream = matches!(asset.asset_type.as_str(), "audio" | "video");

            if !has_audio_stream {
                continue;
            }

            if let Some(source_path) = resolve_asset_path(asset) {
                if media_file_has_audio_stream(ffmpeg, ffprobe, &source_path) {
                    clips.push(RenderClip { clip, asset });
                } else if asset.asset_type == "video" {
                    warnings.push(format!(
                        "视频素材「{}」不含音频流，导出时仅使用画面。",
                        asset.name
                    ));
                } else {
                    return Err(format!(
                        "音频素材「{}」没有可读取的音频流，请检查文件后重新导入。",
                        asset.name
                    ));
                }
            } else if is_virtual_audio_preset(asset) {
                warnings.push(format!(
                    "内置音频预设「{}」暂未生成真实音频，导出时已跳过。",
                    asset.name
                ));
            } else if asset.asset_type == "video" {
                warnings.push(format!(
                    "视频素材「{}」没有可导出的本地音频文件，导出时仅使用画面。",
                    asset.name
                ));
            } else {
                return Err(format!(
                    "音频素材「{}」没有可导出的本地文件，请重新导入后再导出。",
                    asset.name
                ));
            }
        }
    }

    clips.sort_by(|left, right| {
        left.clip
            .start
            .partial_cmp(&right.clip.start)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(clips)
}

fn format_seconds(value: f64) -> String {
    format!("{:.3}", value.max(0.0))
}

fn normalized_speed(speed: f64) -> f64 {
    if speed.is_finite() && speed > 0.05 {
        speed.clamp(0.1, 8.0)
    } else {
        1.0
    }
}

fn video_normalize_filter(width: u32, height: u32, fps: u32) -> String {
    format!(
        "scale={width}:{height}:force_original_aspect_ratio=decrease,pad={width}:{height}:(ow-iw)/2:(oh-ih)/2:black,setsar=1,fps={fps},format=yuv420p"
    )
}

fn append_segment_video_args(command: &mut Command, fps: u32) {
    command.args([
        "-an", "-c:v", "libx264", "-preset", "veryfast", "-crf", "19", "-pix_fmt", "yuv420p", "-r",
    ]);
    command.arg(fps.to_string());
}

fn render_black_segment(
    ffmpeg: &Path,
    output_path: &Path,
    width: u32,
    height: u32,
    fps: u32,
    duration: f64,
    task: &TaskHandle,
    progress_start: f64,
    progress_end: f64,
) -> Result<(), String> {
    let mut command = Command::new(ffmpeg);
    command.args(["-y", "-f", "lavfi"]);
    command.arg("-i").arg(format!(
        "color=c=black:s={}x{}:r={}:d={}",
        width,
        height,
        fps,
        format_seconds(duration)
    ));
    append_segment_video_args(&mut command, fps);
    command.arg(output_path);
    run_ffmpeg(
        command,
        "生成空画面片段",
        task,
        progress_start,
        progress_end,
        duration,
    )
}

fn render_visual_segment(
    ffmpeg: &Path,
    render_clip: &RenderClip<'_>,
    output_path: &Path,
    width: u32,
    height: u32,
    fps: u32,
    duration: f64,
    trim_start: f64,
    task: &TaskHandle,
    progress_start: f64,
    progress_end: f64,
) -> Result<(), String> {
    let source_path = asset_path_or_error(render_clip.asset)?;
    let speed = normalized_speed(render_clip.clip.speed);
    let mut command = Command::new(ffmpeg);
    command.arg("-y");

    if render_clip.asset.asset_type == "image" {
        command.args(["-loop", "1", "-t"]);
        command.arg(format_seconds(duration));
        command.arg("-i").arg(source_path);
        command
            .arg("-vf")
            .arg(video_normalize_filter(width, height, fps));
    } else {
        command.arg("-ss").arg(format_seconds(trim_start));
        command.arg("-t").arg(format_seconds(duration * speed));
        command.arg("-i").arg(source_path);

        let filter = if (speed - 1.0).abs() > 0.001 {
            format!(
                "setpts=PTS/{},{}",
                format_seconds(speed),
                video_normalize_filter(width, height, fps)
            )
        } else {
            video_normalize_filter(width, height, fps)
        };

        command.arg("-vf").arg(filter);
    }

    append_segment_video_args(&mut command, fps);
    command.arg(output_path);
    run_ffmpeg(
        command,
        "渲染视频片段",
        task,
        progress_start,
        progress_end,
        duration,
    )
}

fn escape_concat_path(path: &Path) -> String {
    path.to_string_lossy().replace('\'', "'\\''")
}

fn concat_segments(
    ffmpeg: &Path,
    segments: &[PathBuf],
    concat_list_path: &Path,
    output_path: &Path,
    task: &TaskHandle,
) -> Result<(), String> {
    if segments.len() == 1 {
        fs::copy(&segments[0], output_path).map_err(|error| error.to_string())?;
        task.set_progress(75.0, "合并视频片段");
        return Ok(());
    }

    let list_content = segments
        .iter()
        .map(|path| format!("file '{}'", escape_concat_path(path)))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(concat_list_path, list_content).map_err(|error| error.to_string())?;

    let mut command = Command::new(ffmpeg);
    command.args(["-y", "-f", "concat", "-safe", "0"]);
    command.arg("-i").arg(concat_list_path);
    command.args(["-c", "copy"]);
    command.arg(output_path);
    run_ffmpeg(command, "合并视频片段", task, 70.0, 75.0, 1.0)
}

fn render_base_video(
    ffmpeg: &Path,
    project: &EditorProject,
    work_dir: &Path,
    width: u32,
    height: u32,
    fps: u32,
    duration: f64,
    task: &TaskHandle,
) -> Result<PathBuf, String> {
    let visual_clips = collect_visual_clips(project);
    let mut segments = Vec::new();
    let mut cursor = 0.0_f64;

    let clip_count = visual_clips.len().max(1) as f64;

    for (index, render_clip) in visual_clips.iter().enumerate() {
        if cursor >= duration - 0.001 {
            break;
        }

        let item_start = 5.0 + (index as f64 / clip_count) * 62.0;
        let item_end = 5.0 + ((index + 1) as f64 / clip_count) * 62.0;

        let clip_start = render_clip.clip.start.max(0.0);
        let has_gap = clip_start > cursor + 0.001;

        if has_gap {
            let gap_duration = (clip_start - cursor).min(duration - cursor);
            let segment_path = work_dir.join(format!("segment-{index:04}-gap.mp4"));
            render_black_segment(
                ffmpeg,
                &segment_path,
                width,
                height,
                fps,
                gap_duration,
                task,
                item_start,
                (item_start + item_end) / 2.0,
            )?;
            segments.push(segment_path);
            cursor += gap_duration;
        }

        let timeline_start = clip_start.max(cursor);
        let clip_offset = (timeline_start - clip_start).max(0.0);
        let remaining_clip_duration = render_clip.clip.duration - clip_offset;
        let segment_duration = remaining_clip_duration.min(duration - timeline_start);

        if segment_duration <= 0.001 {
            continue;
        }

        let speed = normalized_speed(render_clip.clip.speed);
        let trim_start = render_clip.clip.trim_start.max(0.0) + clip_offset * speed;
        let segment_path = work_dir.join(format!("segment-{index:04}-visual.mp4"));
        render_visual_segment(
            ffmpeg,
            render_clip,
            &segment_path,
            width,
            height,
            fps,
            segment_duration,
            trim_start,
            task,
            if has_gap {
                (item_start + item_end) / 2.0
            } else {
                item_start
            },
            item_end,
        )?;
        segments.push(segment_path);
        cursor = timeline_start + segment_duration;
    }

    if cursor < duration - 0.001 {
        let segment_path = work_dir.join("segment-tail-gap.mp4");
        render_black_segment(
            ffmpeg,
            &segment_path,
            width,
            height,
            fps,
            duration - cursor,
            task,
            67.0,
            70.0,
        )?;
        segments.push(segment_path);
    }

    let base_path = work_dir.join("video-base.mp4");

    if segments.is_empty() {
        render_black_segment(
            ffmpeg, &base_path, width, height, fps, duration, task, 5.0, 70.0,
        )?;
    } else {
        concat_segments(
            ffmpeg,
            &segments,
            &work_dir.join("concat-list.txt"),
            &base_path,
            task,
        )?;
    }

    Ok(base_path)
}

fn escape_filter_value(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace(':', "\\:")
        .replace('\'', "\\'")
        .replace(',', "\\,")
        .replace('[', "\\[")
        .replace(']', "\\]")
}

fn font_file_filter_part() -> String {
    let candidates = [
        "/System/Library/Fonts/PingFang.ttc",
        "/System/Library/Fonts/Supplemental/Arial Unicode.ttf",
        "/Library/Fonts/Arial Unicode.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    ];

    candidates
        .iter()
        .find(|path| Path::new(path).exists())
        .map(|path| format!("fontfile='{}':", escape_filter_value(path)))
        .unwrap_or_default()
}

fn normalize_hex_color(color: &str, fallback: &str) -> String {
    let trimmed = color.trim().trim_start_matches('#');

    if trimmed.len() == 6
        && trimmed
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        format!("0x{}", trimmed.to_ascii_uppercase())
    } else {
        fallback.to_string()
    }
}

fn caption_drawtext_filter(clip: &TimelineClip, width: u32, height: u32) -> String {
    let text = escape_filter_value(clip.caption_text.as_deref().unwrap_or("").trim());
    let opacity = clip.transform.opacity.clamp(0.0, 1.0);
    let font_part = font_file_filter_part();
    let style = clip.text_style.as_ref();
    let base_font_size = style.map(|style| style.font_size).unwrap_or(52.0);
    let font_size = (base_font_size * (height as f64 / 1080.0)).clamp(16.0, 180.0);
    let font_color = normalize_hex_color(
        style.map(|style| style.color.as_str()).unwrap_or("#FFFFFF"),
        "white",
    );
    let line_spacing = style
        .map(|style| ((style.line_height - 1.0) * font_size).max(0.0))
        .unwrap_or(8.0);
    let x_offset = clip.transform.x * (width as f64 / 1920.0);
    let y_offset = clip.transform.y * (height as f64 / 1080.0);
    let mut filter = format!(
        "drawtext={font_part}text='{text}':x=(w-text_w)/2+{}:y=h*0.72-text_h/2+{}:fontsize={}:fontcolor={}@{}:line_spacing={}:borderw=3:bordercolor=black@0.70:enable='between(t,{},{})'",
        format_seconds(x_offset),
        format_seconds(y_offset),
        format_seconds(font_size),
        font_color,
        format_seconds(opacity),
        format_seconds(line_spacing),
        format_seconds(clip.start.max(0.0)),
        format_seconds((clip.start + clip.duration).max(clip.start)),
    );

    if style.map(|style| style.background_enabled).unwrap_or(false) {
        let box_color = normalize_hex_color(
            style
                .map(|style| style.background_color.as_str())
                .unwrap_or("#000000"),
            "black",
        );
        filter.push_str(&format!(":box=1:boxcolor={}@0.62:boxborderw=12", box_color));
    }

    filter
}

fn build_caption_filters(
    project: &EditorProject,
    width: u32,
    height: u32,
) -> (Vec<String>, String) {
    let caption_clips = collect_caption_clips(project);
    let mut filters = Vec::new();
    let mut input_label = "0:v".to_string();

    for (index, clip) in caption_clips.iter().enumerate() {
        let output_label = format!("vcaption{index}");
        filters.push(format!(
            "[{input_label}]{}[{output_label}]",
            caption_drawtext_filter(clip, width, height)
        ));
        input_label = output_label;
    }

    if filters.is_empty() {
        (filters, "0:v".to_string())
    } else {
        (filters, format!("[{input_label}]"))
    }
}

fn audio_tempo_filter_parts(speed: f64) -> Vec<String> {
    let mut remaining = normalized_speed(speed);
    let mut parts = Vec::new();

    while remaining > 2.0 {
        parts.push("atempo=2.000".to_string());
        remaining /= 2.0;
    }

    while remaining < 0.5 {
        parts.push("atempo=0.500".to_string());
        remaining /= 0.5;
    }

    if (remaining - 1.0).abs() > 0.001 {
        parts.push(format!("atempo={}", format_seconds(remaining)));
    }

    parts
}

fn build_audio_filters(
    audio_clips: &[RenderClip<'_>],
    duration: f64,
) -> Result<(Vec<String>, Option<String>), String> {
    if audio_clips.is_empty() {
        return Ok((Vec::new(), None));
    }

    let mut filters = Vec::new();
    let mut labels = Vec::new();

    for (index, render_clip) in audio_clips.iter().enumerate() {
        let input_index = index + 1;
        let label = format!("audio{index}");
        let speed = normalized_speed(render_clip.clip.speed);
        let input_duration = render_clip.clip.duration * speed;
        let delay_ms = (render_clip.clip.start.max(0.0) * 1000.0).round() as u64;
        let mut parts = vec![
            format!(
                "[{input_index}:a]atrim=start={}:duration={}",
                format_seconds(render_clip.clip.trim_start.max(0.0)),
                format_seconds(input_duration)
            ),
            "asetpts=PTS-STARTPTS".to_string(),
        ];

        parts.extend(audio_tempo_filter_parts(speed));
        parts.push(format!(
            "volume={}",
            format_seconds(render_clip.clip.volume.clamp(0.0, 2.0))
        ));
        parts.push(format!("adelay={delay_ms}|{delay_ms}"));
        parts.push(format!("apad[{label}]"));
        filters.push(parts.join(","));
        labels.push(format!("[{label}]"));
    }

    if labels.len() == 1 {
        filters.push(format!(
            "{}atrim=duration={}[aout]",
            labels[0],
            format_seconds(duration)
        ));
    } else {
        filters.push(format!(
            "{}amix=inputs={}:duration=longest:dropout_transition=0,atrim=duration={}[aout]",
            labels.join(""),
            labels.len(),
            format_seconds(duration)
        ));
    }

    Ok((filters, Some("[aout]".to_string())))
}

fn append_final_codec_args(command: &mut Command, preset: &ExportPreset, has_audio: bool) {
    let fps = preset.fps.max(1).to_string();

    if preset.format == "webm" {
        let crf = match preset.quality.as_str() {
            "draft" => "38",
            "high" => "28",
            _ => "32",
        };
        command.args([
            "-c:v",
            "libvpx-vp9",
            "-deadline",
            "good",
            "-cpu-used",
            "4",
            "-crf",
            crf,
            "-b:v",
            "0",
            "-pix_fmt",
            "yuv420p",
            "-r",
            &fps,
        ]);

        if has_audio {
            command.args(["-c:a", "libopus", "-b:a", "160k"]);
        } else {
            command.arg("-an");
        }

        return;
    }

    let crf = match preset.quality.as_str() {
        "draft" => "28",
        "high" => "18",
        _ => "23",
    };
    command.args([
        "-c:v", "libx264", "-preset", "medium", "-crf", crf, "-pix_fmt", "yuv420p", "-r", &fps,
    ]);

    if has_audio {
        command.args(["-c:a", "aac", "-b:a", "192k"]);
    } else {
        command.arg("-an");
    }

    if preset.format == "mp4" {
        command.args(["-movflags", "+faststart"]);
    }
}

fn render_final_export(
    ffmpeg: &Path,
    ffprobe: Option<&Path>,
    base_video_path: &Path,
    output_path: &Path,
    project: &EditorProject,
    preset: &ExportPreset,
    width: u32,
    height: u32,
    duration: f64,
    warnings: &mut Vec<String>,
    task: &TaskHandle,
) -> Result<(), String> {
    let audio_clips = collect_audio_clips(project, ffmpeg, ffprobe, warnings)?;
    let mut command = Command::new(ffmpeg);
    command.arg("-y");
    command.arg("-i").arg(base_video_path);

    for render_clip in &audio_clips {
        command
            .arg("-i")
            .arg(asset_path_or_error(render_clip.asset)?);
    }

    let (mut filters, video_map) = build_caption_filters(project, width, height);
    let (audio_filters, audio_map) = build_audio_filters(&audio_clips, duration)?;
    filters.extend(audio_filters);

    if !filters.is_empty() {
        command.arg("-filter_complex").arg(filters.join(";"));
    }

    command.arg("-map").arg(video_map);

    if let Some(audio_map) = audio_map {
        command.arg("-map").arg(audio_map);
    }

    append_final_codec_args(&mut command, preset, !audio_clips.is_empty());
    command.arg(output_path);
    run_ffmpeg(command, "封装导出文件", task, 76.0, 99.0, duration)
}

fn encode_file_url_path(path: &Path) -> String {
    let path_string = path.to_string_lossy().replace('\\', "/");
    let mut encoded = String::new();

    for byte in path_string.as_bytes() {
        let character = *byte as char;

        if character.is_ascii_alphanumeric()
            || matches!(character, '/' | ':' | '-' | '_' | '.' | '~')
        {
            encoded.push(character);
        } else {
            encoded.push_str(&format!("%{byte:02X}"));
        }
    }

    encoded
}

fn path_to_file_url(path: &Path) -> String {
    let encoded = encode_file_url_path(path);

    #[cfg(target_os = "windows")]
    {
        if encoded.starts_with("//") {
            return format!("file:{encoded}");
        }

        return format!("file:///{}", encoded.trim_start_matches('/'));
    }

    #[cfg(not(target_os = "windows"))]
    format!("file://{encoded}")
}

fn open_file_manager_for_path(path: &Path) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let status = Command::new("open")
        .arg("-R")
        .arg(path)
        .status()
        .map_err(|error| format!("打开 Finder 失败：{error}"))?;

    #[cfg(target_os = "windows")]
    let status = Command::new("explorer.exe")
        .arg(format!("/select,{}", path.to_string_lossy()))
        .status()
        .map_err(|error| format!("打开资源管理器失败：{error}"))?;

    #[cfg(all(unix, not(target_os = "macos")))]
    let status = Command::new("xdg-open")
        .arg(path.parent().unwrap_or(path))
        .status()
        .map_err(|error| format!("打开文件夹失败：{error}"))?;

    if status.success() {
        Ok(())
    } else {
        Err("打开导出目录失败。".to_string())
    }
}

fn canonical_export_file_path(app: &AppHandle, output_path: &str) -> Result<PathBuf, String> {
    let output_dir = export_output_dir(app)?
        .canonicalize()
        .map_err(|error| error.to_string())?;
    let path = PathBuf::from(output_path);

    if !path.exists() {
        return Err("导出文件不存在，可能已被移动或删除。".to_string());
    }

    let path = path.canonicalize().map_err(|error| error.to_string())?;

    if !path.starts_with(&output_dir) {
        return Err("只能打开 Linglux output 目录中的文件。".to_string());
    }

    Ok(path)
}

fn export_project_with_ffmpeg(
    app: &AppHandle,
    request: EditorExportRequest,
    task: Option<&TaskHandle>,
) -> Result<EditorExportResult, String> {
    let task = task.ok_or_else(|| "导出任务上下文缺失。".to_string())?;
    if !has_exportable_timeline(&request.project) {
        return Err("时间线为空，请先把素材拖到时间线。".to_string());
    }

    let duration = project_export_duration(&request.project);

    if !duration.is_finite() || duration <= 0.0 {
        return Err("工程时长无效，无法导出。".to_string());
    }

    let ffmpeg = find_ffmpeg().ok_or_else(|| {
        "未找到 FFmpeg。请先安装 FFmpeg，并确保 ffmpeg 可执行文件在 PATH、/opt/homebrew/bin 或 /usr/local/bin 中。"
            .to_string()
    })?;
    task.set_progress(5.0, "检查媒体与编码器");
    let ffprobe = find_ffprobe();
    let extension = export_extension(&request.preset)?;
    let (width, height) = preset_resolution(&request.preset, &request.project);
    let fps = request.preset.fps.max(1);
    let now = now_stamp();
    let timestamp = now.replace("unix-", "");
    let export_dir = export_output_dir(app)?;
    let project_name = sanitize_file_name(&request.project.name);
    let output_path = unique_child_path(
        &export_dir,
        &format!("{project_name}-{timestamp}.{extension}"),
    );
    let work_dir = export_dir.join(format!(
        ".render-{}-{timestamp}",
        sanitize_file_name(&request.project.id)
    ));
    fs::create_dir_all(&work_dir).map_err(|error| error.to_string())?;

    let mut warnings = Vec::new();
    let render_result = (|| {
        let base_video_path = render_base_video(
            &ffmpeg,
            &request.project,
            &work_dir,
            width,
            height,
            fps,
            duration,
            task,
        )?;
        task.set_progress(76.0, "准备最终封装");
        render_final_export(
            &ffmpeg,
            ffprobe.as_deref(),
            &base_video_path,
            &output_path,
            &request.project,
            &request.preset,
            width,
            height,
            duration,
            &mut warnings,
            task,
        )
    })();
    let _ = fs::remove_dir_all(&work_dir);

    if let Err(error) = render_result {
        let _ = fs::remove_file(&output_path);
        return Err(error);
    }

    let source_node_id = request
        .project
        .source_node_id
        .clone()
        .unwrap_or_else(|| request.session_id.clone());
    let artifact = Artifact {
        id: format!("artifact-edit-{timestamp}"),
        artifact_type: "video".to_string(),
        name: format!("{} · edited.{extension}", request.project.name),
        source_node_id,
        url: path_to_file_url(&output_path),
        duration,
        created_at: now.clone(),
    };
    let manifest_path = output_path.with_extension(format!("{extension}.json"));
    let manifest = serde_json::json!({
        "exportedAt": now,
        "outputPath": output_path.to_string_lossy(),
        "artifact": &artifact,
        "preset": &request.preset,
        "project": &request.project,
        "warnings": &warnings,
    });
    let manifest_json =
        serde_json::to_string_pretty(&manifest).map_err(|error| error.to_string())?;
    fs::write(&manifest_path, manifest_json).map_err(|error| error.to_string())?;

    Ok(EditorExportResult {
        artifact,
        preset: request.preset,
        output_path: Some(output_path.to_string_lossy().to_string()),
        manifest_path: Some(manifest_path.to_string_lossy().to_string()),
        warnings: if warnings.is_empty() {
            None
        } else {
            Some(warnings)
        },
    })
}

fn create_mock_project(seed: EditorSessionSeed, now: String) -> EditorProject {
    let source_node_id = seed.source_node_id;
    let duration = seed.duration.unwrap_or(12.0).max(1.0);

    EditorProject {
        id: format!("project-{}", now.replace("unix-", "")),
        name: source_node_id
            .as_ref()
            .map(|id| format!("剪辑会话 · {id}"))
            .unwrap_or_else(|| "Linglux 剪辑工程".to_string()),
        source_node_id,
        assets: vec![],
        tracks: vec![
            TimelineTrack {
                id: "track-overlay".to_string(),
                track_type: "overlay".to_string(),
                label: "叠加轨".to_string(),
                muted: false,
                visible: true,
                media_enabled: true,
                locked: false,
                clips: vec![],
            },
            TimelineTrack {
                id: "track-video".to_string(),
                track_type: "video".to_string(),
                label: "视频轨".to_string(),
                muted: false,
                visible: true,
                media_enabled: true,
                locked: false,
                clips: vec![],
            },
            TimelineTrack {
                id: "track-caption".to_string(),
                track_type: "caption".to_string(),
                label: "字幕轨".to_string(),
                muted: false,
                visible: true,
                media_enabled: true,
                locked: false,
                clips: vec![],
            },
            TimelineTrack {
                id: "track-audio".to_string(),
                track_type: "audio".to_string(),
                label: "音频轨".to_string(),
                muted: false,
                visible: true,
                media_enabled: true,
                locked: false,
                clips: vec![],
            },
        ],
        main_track_magnet_enabled: true,
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
fn create_edit_session(core: State<'_, MediaCore>, seed: EditorSessionSeed) -> EditSession {
    let now = now_stamp();
    let project_id = seed
        .source_node_id
        .as_deref()
        .map(|source_node_id| format!("project-source-{}", sanitize_file_name(source_node_id)))
        .unwrap_or_else(|| format!("project-{}", now.replace("unix-", "")));
    let restored = core.projects().load(&project_id);
    let saved_at = restored
        .as_ref()
        .map(|document| format!("unix-{}", document.saved_at_ms / 1_000));
    let project = restored
        .and_then(|document| serde_json::from_value::<EditorProject>(document.project).ok())
        .unwrap_or_else(|| {
            let mut project = create_mock_project(seed, now.clone());
            project.id = project_id;
            project
        });

    EditSession {
        id: format!("session-{}", now.replace("unix-", "")),
        source_node_id: project.source_node_id.clone(),
        project,
        saved_at: saved_at.or(Some(now)),
        is_dirty: false,
    }
}

#[tauri::command]
fn load_edit_project(
    core: State<'_, MediaCore>,
    project_id: String,
) -> Result<EditorProject, String> {
    if let Some(document) = core.projects().load(&project_id) {
        return serde_json::from_value(document.project).map_err(|error| error.to_string());
    }

    Err(format!("找不到工程 {project_id}"))
}

#[tauri::command]
async fn save_edit_project(
    core: State<'_, MediaCore>,
    session_id: String,
    mut project: EditorProject,
) -> Result<EditorProject, String> {
    let core = core.inner().clone();
    let project_id = project.id.clone();
    project.updated_at = now_stamp();
    let project_for_save = project.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let value = serde_json::to_value(project_for_save).map_err(|error| error.to_string())?;
        core.projects().save_json(&project_id, value)?;
        let _ = session_id;
        Ok::<(), String>(())
    })
    .await
    .map_err(|error| error.to_string())??;

    Ok(project)
}

#[tauri::command]
fn start_import_media(
    core: State<'_, MediaCore>,
    project_id: String,
    paths: Vec<String>,
    on_event: Channel<TaskEvent>,
) -> Result<TaskSnapshot, String> {
    let core = core.inner().clone();
    let listener = Arc::new(move |event: TaskEvent| {
        let _ = on_event.send(event);
    });
    let task = core.tasks().create(
        TaskKind::Import,
        Some(project_id.clone()),
        "导入媒体",
        Some(listener),
    );
    let snapshot = task.snapshot();
    let worker_task = task.clone();
    let source_paths = paths.into_iter().map(PathBuf::from).collect::<Vec<_>>();
    let job_core = core.clone();

    core.submit(TaskKind::Import, move || {
        worker_task.start("准备导入");

        match import_media_paths(
            job_core.projects(),
            &project_id,
            &source_paths,
            &worker_task,
        ) {
            Ok(imported) => match serde_json::to_value(imported) {
                Ok(result) => worker_task.succeed(result),
                Err(error) => worker_task.fail(error.to_string()),
            },
            Err(_error) if worker_task.snapshot().state == TaskState::Cancelled => {}
            Err(error) => worker_task.fail(error),
        }
    })?;

    Ok(snapshot)
}

#[tauri::command]
fn start_storyboard_to_video(
    core: State<'_, MediaCore>,
    request: StoryboardToVideoRequest,
    on_event: Channel<TaskEvent>,
) -> Result<TaskSnapshot, String> {
    let core = core.inner().clone();
    let listener = Arc::new(move |event: TaskEvent| {
        let _ = on_event.send(event);
    });
    let task = core.tasks().create(
        TaskKind::FrameSequence,
        Some(request.project_id.clone()),
        format!("生成分镜视频 {}", request.output_name),
        Some(listener),
    );
    let snapshot = task.snapshot();
    let worker_task = task.clone();
    let job_core = core.clone();

    core.submit(TaskKind::FrameSequence, move || {
        worker_task.start("准备分镜图");
        match storyboard_to_video(job_core.projects(), &request, &worker_task) {
            Ok(result) => match serde_json::to_value(result) {
                Ok(result) => worker_task.succeed(result),
                Err(error) => worker_task.fail(error.to_string()),
            },
            Err(_error) if worker_task.snapshot().state == TaskState::Cancelled => {}
            Err(error) => worker_task.fail(error),
        }
    })?;

    Ok(snapshot)
}

#[tauri::command]
fn start_export(
    app: AppHandle,
    core: State<'_, MediaCore>,
    request: EditorExportRequest,
    on_event: Channel<TaskEvent>,
) -> Result<TaskSnapshot, String> {
    let core = core.inner().clone();
    let project_id = request.project.id.clone();
    let listener = Arc::new(move |event: TaskEvent| {
        let _ = on_event.send(event);
    });
    let task = core.tasks().create(
        TaskKind::Export,
        Some(project_id),
        format!("导出 {}", request.project.name),
        Some(listener),
    );
    let snapshot = task.snapshot();
    let worker_task = task.clone();

    core.submit(TaskKind::Export, move || {
        worker_task.start("准备素材");
        worker_task.set_progress(3.0, "准备素材");

        match export_project_with_ffmpeg(&app, request, Some(&worker_task)) {
            Ok(result) => match serde_json::to_value(result) {
                Ok(result) => worker_task.succeed(result),
                Err(error) => worker_task.fail(error.to_string()),
            },
            Err(_error) if worker_task.snapshot().state == TaskState::Cancelled => {}
            Err(error) => worker_task.fail(error),
        }
    })?;

    Ok(snapshot)
}

#[tauri::command]
fn start_media_derivatives(
    core: State<'_, MediaCore>,
    project_id: String,
    asset_id: String,
    source_path: String,
    media_kind: MediaKind,
    fingerprint: String,
    fallback_duration: f64,
    generate_waveform: bool,
    generate_proxy: bool,
    on_event: Channel<TaskEvent>,
) -> Result<TaskSnapshot, String> {
    let core = core.inner().clone();
    let task_kind = match (&media_kind, generate_waveform, generate_proxy) {
        (MediaKind::Video, _, true) => TaskKind::Proxy,
        (MediaKind::Video | MediaKind::Audio, true, false) => TaskKind::Waveform,
        _ => TaskKind::Thumbnail,
    };
    let listener = Arc::new(move |event: TaskEvent| {
        let _ = on_event.send(event);
    });
    let task = core.tasks().create(
        task_kind.clone(),
        Some(project_id.clone()),
        format!("生成素材缓存 {asset_id}"),
        Some(listener),
    );
    let snapshot = task.snapshot();
    let worker_task = task.clone();
    let job_core = core.clone();

    core.submit(task_kind, move || {
        worker_task.start("准备媒体缓存");

        match generate_media_derivatives(
            job_core.projects(),
            job_core.cache(),
            &project_id,
            Path::new(&source_path),
            media_kind,
            &fingerprint,
            fallback_duration,
            generate_waveform,
            generate_proxy,
            &worker_task,
        ) {
            Ok(derivatives) => {
                let _ = job_core.cache().prune_to_bytes(2 * 1024 * 1024 * 1024);

                match serde_json::to_value(derivatives) {
                    Ok(result) => worker_task.succeed(result),
                    Err(error) => worker_task.fail(error.to_string()),
                }
            }
            Err(_error) if worker_task.snapshot().state == TaskState::Cancelled => {}
            Err(error) => worker_task.fail(error),
        }
    })?;

    Ok(snapshot)
}

#[tauri::command]
fn cancel_media_task(core: State<'_, MediaCore>, task_id: String) -> bool {
    core.tasks().cancel(&task_id)
}

#[tauri::command]
fn get_media_task(core: State<'_, MediaCore>, task_id: String) -> Option<TaskSnapshot> {
    core.tasks().get(&task_id)
}

#[tauri::command]
fn list_media_tasks(core: State<'_, MediaCore>, project_id: Option<String>) -> Vec<TaskSnapshot> {
    core.tasks().list(project_id.as_deref())
}

#[tauri::command]
fn get_tts_status(core: State<'_, MediaCore>) -> TtsStatus {
    let mut status = core.tts().status();
    let installing = core
        .tasks()
        .list(None)
        .iter()
        .any(|task| task.kind == TaskKind::TtsSetup && !task.state.is_terminal());
    if installing {
        status.state = "installing".to_string();
    }
    status
}

#[tauri::command]
fn start_tts_setup(
    core: State<'_, MediaCore>,
    on_event: Channel<TaskEvent>,
) -> Result<TaskSnapshot, String> {
    let core = core.inner().clone();
    let listener = Arc::new(move |event: TaskEvent| {
        let _ = on_event.send(event);
    });
    let task = core
        .tasks()
        .create(TaskKind::TtsSetup, None, "安装本地语音模型", Some(listener));
    let snapshot = task.snapshot();
    let worker_task = task.clone();
    let job_core = core.clone();
    core.submit(TaskKind::TtsSetup, move || {
        worker_task.start("准备本地语音环境");
        match job_core.tts().setup(&worker_task) {
            Ok(status) => match serde_json::to_value(status) {
                Ok(result) => worker_task.succeed(result),
                Err(error) => worker_task.fail(error.to_string()),
            },
            Err(_error) if worker_task.snapshot().state == TaskState::Cancelled => {}
            Err(error) => worker_task.fail(error),
        }
    })?;
    Ok(snapshot)
}

#[tauri::command]
fn start_speech_synthesis(
    core: State<'_, MediaCore>,
    request: SpeechSynthesisRequest,
    on_event: Channel<TaskEvent>,
) -> Result<TaskSnapshot, String> {
    let core = core.inner().clone();
    let project_id = request.project_id.clone();
    let listener = Arc::new(move |event: TaskEvent| {
        let _ = on_event.send(event);
    });
    let task = core.tasks().create(
        TaskKind::SpeechSynthesis,
        Some(project_id),
        "生成 AI 配音",
        Some(listener),
    );
    let snapshot = task.snapshot();
    let worker_task = task.clone();
    let job_core = core.clone();
    core.submit(TaskKind::SpeechSynthesis, move || {
        worker_task.start("准备生成配音");
        match job_core
            .tts()
            .synthesize(job_core.projects(), &request, &worker_task)
        {
            Ok(result) => match serde_json::to_value(result) {
                Ok(result) => worker_task.succeed(result),
                Err(error) => worker_task.fail(error.to_string()),
            },
            Err(_error) if worker_task.snapshot().state == TaskState::Cancelled => {}
            Err(error) => worker_task.fail(error),
        }
    })?;
    Ok(snapshot)
}

#[tauri::command]
fn reveal_export_file(app: AppHandle, output_path: String) -> Result<(), String> {
    let path = canonical_export_file_path(&app, &output_path)?;

    open_file_manager_for_path(&path)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let core_root = app
                .path()
                .app_data_dir()
                .map_err(|error| Box::<dyn std::error::Error>::from(error))?
                .join("media-core");
            let core = MediaCore::new(core_root).map_err(std::io::Error::other)?;
            app.manage(core);
            app.manage(AgentRuntime::new().map_err(std::io::Error::other)?);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            create_video_plan,
            load_agent_provider_settings,
            save_agent_provider_settings,
            clear_agent_provider_settings,
            load_agent_conversation,
            clear_agent_conversation,
            update_agent_plan_state,
            start_editor_agent_turn,
            cancel_editor_agent_turn,
            create_edit_session,
            load_edit_project,
            save_edit_project,
            start_import_media,
            start_storyboard_to_video,
            start_media_derivatives,
            start_export,
            cancel_media_task,
            get_media_task,
            list_media_tasks,
            get_tts_status,
            start_tts_setup,
            start_speech_synthesis,
            reveal_export_file
        ])
        .run(tauri::generate_context!())
        .expect("error while running Linglux");
}
