use crate::{
    discover_ffmpeg_binary, run_ffmpeg_process, sanitize_path_component, unix_millis, ProjectStore,
    TaskHandle,
};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::process::Command;

const MAX_FRAME_COUNT: usize = 240;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryboardFrameRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryboardToVideoRequest {
    pub project_id: String,
    pub source_path: String,
    pub source_width: u32,
    pub source_height: u32,
    pub frames: Vec<StoryboardFrameRect>,
    pub fps: u32,
    pub output_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoryboardToVideoResult {
    pub managed_path: String,
    pub url: String,
    pub file_name: String,
    pub fingerprint: String,
    pub duration: f64,
    pub width: u32,
    pub height: u32,
    pub frame_count: usize,
    pub warnings: Vec<String>,
}

pub fn storyboard_to_video(
    store: &ProjectStore,
    request: &StoryboardToVideoRequest,
    task: &TaskHandle,
) -> Result<StoryboardToVideoResult, String> {
    validate_request(store, request)?;
    let ffmpeg = discover_ffmpeg_binary("ffmpeg")
        .ok_or_else(|| "找不到 FFmpeg。请安装 FFmpeg 并确认它可从 PATH 访问。".to_string())?;
    let source_path = fs::canonicalize(&request.source_path)
        .map_err(|error| format!("无法读取分镜图：{error}"))?;
    let media_dir = store.media_dir(&request.project_id)?;
    let safe_stem =
        sanitize_path_component(request.output_name.trim_end_matches(".mp4"), "storyboard");
    let file_name = format!("{}-{}.mp4", safe_stem, unix_millis());
    let final_path = media_dir.join(&file_name);
    let temporary_path = media_dir.join(format!(".{file_name}.part"));
    let frame_width = request
        .frames
        .iter()
        .map(|frame| frame.width)
        .min()
        .unwrap()
        & !1;
    let frame_height = request
        .frames
        .iter()
        .map(|frame| frame.height)
        .min()
        .unwrap()
        & !1;
    let duration = request.frames.len() as f64 / request.fps as f64;
    let filter = build_filter(request, frame_width, frame_height);

    task.set_progress(4.0, "正在准备分镜帧");
    let mut command = Command::new(ffmpeg);
    command.args(["-v", "error", "-y", "-loop", "1", "-framerate"]);
    command.arg(request.fps.to_string());
    command.arg("-i").arg(&source_path);
    command.arg("-filter_complex").arg(filter);
    command.args(["-map", "[outv]", "-an", "-r"]);
    command.arg(request.fps.to_string());
    command.args([
        "-frames:v",
        &request.frames.len().to_string(),
        "-c:v",
        "libx264",
        "-pix_fmt",
        "yuv420p",
        "-movflags",
        "+faststart",
        "-f",
        "mp4",
    ]);
    command.arg(&temporary_path);

    let encode_result = run_ffmpeg_process(command, "正在生成分镜视频", task, 5.0, 96.0, duration);
    if let Err(error) = encode_result {
        let _ = fs::remove_file(&temporary_path);
        return Err(error);
    }
    if task.is_cancel_requested() {
        let _ = fs::remove_file(&temporary_path);
        task.mark_cancelled();
        return Err("任务已取消。".to_string());
    }
    fs::rename(&temporary_path, &final_path).map_err(|error| {
        let _ = fs::remove_file(&temporary_path);
        format!("无法保存生成的视频：{error}")
    })?;
    let fingerprint = file_fingerprint(&final_path)?;
    task.set_progress(100.0, "分镜视频已加入素材库");

    Ok(StoryboardToVideoResult {
        managed_path: final_path.to_string_lossy().to_string(),
        url: format!("file://{}", final_path.to_string_lossy()),
        file_name,
        fingerprint,
        duration,
        width: frame_width,
        height: frame_height,
        frame_count: request.frames.len(),
        warnings: Vec::new(),
    })
}

fn validate_request(
    store: &ProjectStore,
    request: &StoryboardToVideoRequest,
) -> Result<(), String> {
    if !(1..=MAX_FRAME_COUNT).contains(&request.frames.len()) {
        return Err(format!("有效帧数必须在 1 到 {MAX_FRAME_COUNT} 之间。"));
    }
    if !(1..=120).contains(&request.fps) {
        return Err("帧率必须在 1 到 120 FPS 之间。".to_string());
    }
    if request.source_width == 0 || request.source_height == 0 {
        return Err("分镜图尺寸无效。".to_string());
    }
    let source = fs::canonicalize(&request.source_path)
        .map_err(|error| format!("无法读取分镜图：{error}"))?;
    let project_dir = fs::canonicalize(store.project_dir(&request.project_id))
        .map_err(|_| "当前工程媒体目录不存在，请先导入分镜图。".to_string())?;
    if !source.starts_with(&project_dir) {
        return Err("分镜图必须先导入当前 Linglux 工程。".to_string());
    }
    for (index, frame) in request.frames.iter().enumerate() {
        let right = frame.x.checked_add(frame.width);
        let bottom = frame.y.checked_add(frame.height);
        if frame.width < 2
            || frame.height < 2
            || right.is_none()
            || bottom.is_none()
            || right.unwrap() > request.source_width
            || bottom.unwrap() > request.source_height
        {
            return Err(format!("第 {} 帧的裁切区域超出源图范围。", index + 1));
        }
    }
    let min_width = request
        .frames
        .iter()
        .map(|frame| frame.width)
        .min()
        .unwrap()
        & !1;
    let min_height = request
        .frames
        .iter()
        .map(|frame| frame.height)
        .min()
        .unwrap()
        & !1;
    if min_width < 2 || min_height < 2 {
        return Err("裁切后的画格尺寸过小。".to_string());
    }
    Ok(())
}

fn build_filter(request: &StoryboardToVideoRequest, width: u32, height: u32) -> String {
    let split_outputs = (0..request.frames.len())
        .map(|index| format!("[s{index}]"))
        .collect::<String>();
    let mut parts = vec![format!(
        "[0:v]split={}{}",
        request.frames.len(),
        split_outputs
    )];
    for (index, frame) in request.frames.iter().enumerate() {
        parts.push(format!(
            "[s{index}]crop={}:{}:{}:{},scale={width}:{height}:flags=lanczos,trim=end_frame=1,setpts=PTS-STARTPTS[f{index}]",
            frame.width, frame.height, frame.x, frame.y
        ));
    }
    let inputs = (0..request.frames.len())
        .map(|index| format!("[f{index}]"))
        .collect::<String>();
    parts.push(format!(
        "{inputs}concat=n={}:v=1:a=0[outv]",
        request.frames.len()
    ));
    parts.join(";")
}

fn file_fingerprint(path: &Path) -> Result<String, String> {
    let metadata = fs::metadata(path).map_err(|error| error.to_string())?;
    let modified = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    Ok(format!(
        "{:016x}-{:x}-{:x}",
        fnv1a(path.to_string_lossy().as_bytes()),
        metadata.len(),
        modified
    ))
}

fn fnv1a(bytes: &[u8]) -> u64 {
    bytes.iter().fold(0xcbf29ce484222325_u64, |hash, byte| {
        (hash ^ *byte as u64).wrapping_mul(0x100000001b3)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{ProjectStore, TaskKind, TaskManager};

    #[test]
    fn filter_keeps_row_major_frame_order() {
        let request = StoryboardToVideoRequest {
            project_id: "p".into(),
            source_path: "x.png".into(),
            source_width: 100,
            source_height: 100,
            frames: vec![
                StoryboardFrameRect {
                    x: 1,
                    y: 2,
                    width: 20,
                    height: 22,
                },
                StoryboardFrameRect {
                    x: 30,
                    y: 2,
                    width: 20,
                    height: 22,
                },
            ],
            fps: 24,
            output_name: "demo".into(),
        };
        let filter = build_filter(&request, 20, 22);
        assert!(filter.contains("[s0]crop=20:22:1:2"));
        assert!(filter.ends_with("[f0][f1]concat=n=2:v=1:a=0[outv]"));
    }

    #[test]
    fn creates_a_video_when_ffmpeg_is_available() {
        let Some(ffmpeg) = discover_ffmpeg_binary("ffmpeg") else {
            return;
        };
        let root = std::env::temp_dir().join(format!("linglux-storyboard-test-{}", unix_millis()));
        let store = ProjectStore::new(root.join("projects")).expect("project store");
        let media_dir = store.media_dir("project-1").expect("media dir");
        let source_path = media_dir.join("grid.png");
        let status = Command::new(ffmpeg)
            .args([
                "-v",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "color=c=red:s=128x64",
                "-frames:v",
                "1",
            ])
            .arg(&source_path)
            .status()
            .expect("create source image");
        assert!(status.success());
        let task = TaskManager::new().create(
            TaskKind::FrameSequence,
            Some("project-1".into()),
            "storyboard",
            None,
        );
        task.start("storyboard");
        let result = storyboard_to_video(
            &store,
            &StoryboardToVideoRequest {
                project_id: "project-1".into(),
                source_path: source_path.to_string_lossy().to_string(),
                source_width: 128,
                source_height: 64,
                frames: vec![
                    StoryboardFrameRect {
                        x: 0,
                        y: 0,
                        width: 64,
                        height: 64,
                    },
                    StoryboardFrameRect {
                        x: 64,
                        y: 0,
                        width: 64,
                        height: 64,
                    },
                ],
                fps: 24,
                output_name: "sequence".into(),
            },
            &task,
        )
        .expect("create storyboard video");
        assert_eq!(result.frame_count, 2);
        assert!((result.duration - 2.0 / 24.0).abs() < f64::EPSILON);
        assert!(Path::new(&result.managed_path).is_file());
        let _ = fs::remove_dir_all(root);
    }
}
