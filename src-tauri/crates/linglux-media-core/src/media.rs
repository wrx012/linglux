use crate::{discover_ffmpeg_binary, run_ffmpeg_process, CacheStore, ProjectStore, TaskHandle};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::{self, File};
use std::io::{BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;

const WAVEFORM_SAMPLE_RATE: u64 = 8_000;
const WAVEFORM_PEAK_COUNT: usize = 1_024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum MediaKind {
    Video,
    Image,
    Audio,
    Caption,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaMetadata {
    pub duration: f64,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub has_video: bool,
    pub has_audio: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaDerivatives {
    pub metadata: MediaMetadata,
    pub thumbnail_path: Option<String>,
    pub proxy_path: Option<String>,
    pub waveform_peaks: Option<Vec<f64>>,
    pub warnings: Vec<String>,
}

pub fn generate_media_derivatives(
    projects: &ProjectStore,
    cache: &CacheStore,
    project_id: &str,
    source_path: &Path,
    media_kind: MediaKind,
    fingerprint: &str,
    fallback_duration: f64,
    generate_waveform_cache: bool,
    generate_proxy_cache: bool,
    task: &TaskHandle,
) -> Result<MediaDerivatives, String> {
    if !source_path.is_file() {
        return Err("媒体文件不存在或已被移动。".to_string());
    }

    let ffmpeg = discover_ffmpeg_binary("ffmpeg")
        .ok_or_else(|| "未找到 FFmpeg，无法生成媒体缓存。".to_string())?;
    let ffprobe = discover_ffmpeg_binary("ffprobe");
    task.set_progress(2.0, "读取媒体信息");
    let metadata = probe_media(
        ffprobe.as_deref(),
        source_path,
        media_kind.clone(),
        fallback_duration,
    );
    let mut warnings = Vec::new();
    let thumbnail_path = if matches!(media_kind, MediaKind::Video | MediaKind::Image) {
        match generate_thumbnail(&ffmpeg, cache, source_path, fingerprint, &metadata, task) {
            Ok(path) => Some(path.to_string_lossy().to_string()),
            Err(error) => {
                warnings.push(error);
                None
            }
        }
    } else {
        task.set_progress(20.0, "无需生成缩略图");
        None
    };
    let waveform_peaks = if generate_waveform_cache
        && metadata.has_audio
        && matches!(media_kind, MediaKind::Video | MediaKind::Audio)
    {
        match generate_waveform(
            &ffmpeg,
            cache,
            source_path,
            fingerprint,
            metadata.duration,
            task,
            if media_kind == MediaKind::Video {
                42.0
            } else {
                98.0
            },
        ) {
            Ok(peaks) => Some(peaks),
            Err(error) => {
                warnings.push(error);
                None
            }
        }
    } else {
        task.set_progress(42.0, "无需生成波形");
        None
    };
    let proxy_path = if generate_proxy_cache && media_kind == MediaKind::Video {
        match generate_proxy(
            &ffmpeg,
            projects,
            project_id,
            source_path,
            fingerprint,
            metadata.duration,
            task,
        ) {
            Ok(path) => Some(path.to_string_lossy().to_string()),
            Err(error) => {
                warnings.push(error);
                None
            }
        }
    } else {
        task.set_progress(100.0, "媒体缓存已就绪");
        None
    };

    Ok(MediaDerivatives {
        metadata,
        thumbnail_path,
        proxy_path,
        waveform_peaks,
        warnings,
    })
}

fn probe_media(
    ffprobe: Option<&Path>,
    source_path: &Path,
    media_kind: MediaKind,
    fallback_duration: f64,
) -> MediaMetadata {
    let fallback = MediaMetadata {
        duration: fallback_duration.max(0.0),
        width: None,
        height: None,
        has_video: matches!(media_kind, MediaKind::Video | MediaKind::Image),
        has_audio: matches!(media_kind, MediaKind::Audio | MediaKind::Video),
    };
    let Some(ffprobe) = ffprobe else {
        return fallback;
    };
    let Ok(output) = Command::new(ffprobe)
        .args([
            "-v",
            "error",
            "-show_entries",
            "format=duration:stream=codec_type,width,height",
            "-of",
            "json",
        ])
        .arg(source_path)
        .output()
    else {
        return fallback;
    };

    if !output.status.success() {
        return fallback;
    }

    let Ok(value) = serde_json::from_slice::<Value>(&output.stdout) else {
        return fallback;
    };
    let streams = value["streams"].as_array().cloned().unwrap_or_default();
    let video_stream = streams
        .iter()
        .find(|stream| stream["codec_type"].as_str() == Some("video"));
    let duration = value["format"]["duration"]
        .as_str()
        .and_then(|duration| duration.parse::<f64>().ok())
        .filter(|duration| duration.is_finite() && *duration > 0.0)
        .unwrap_or(fallback.duration);

    MediaMetadata {
        duration,
        width: video_stream
            .and_then(|stream| stream["width"].as_u64())
            .and_then(|width| u32::try_from(width).ok()),
        height: video_stream
            .and_then(|stream| stream["height"].as_u64())
            .and_then(|height| u32::try_from(height).ok()),
        has_video: video_stream.is_some(),
        has_audio: streams
            .iter()
            .any(|stream| stream["codec_type"].as_str() == Some("audio")),
    }
}

fn generate_thumbnail(
    ffmpeg: &Path,
    cache: &CacheStore,
    source_path: &Path,
    fingerprint: &str,
    metadata: &MediaMetadata,
    task: &TaskHandle,
) -> Result<PathBuf, String> {
    let output_path = cache.path_for("thumbnails", fingerprint, "jpg")?;

    if is_nonempty_file(&output_path) {
        task.set_progress(20.0, "复用缩略图缓存");
        return Ok(output_path);
    }

    let temporary_path = temporary_derivative_path(&output_path, task, "jpg");

    let seek_time = if metadata.duration > 0.0 {
        (metadata.duration * 0.2).min(3.0)
    } else {
        0.0
    };
    let mut command = Command::new(ffmpeg);
    command.args(["-y", "-ss", &format!("{seek_time:.3}"), "-i"]);
    command.arg(source_path);
    command.args([
        "-frames:v",
        "1",
        "-vf",
        "scale=min(640\\,iw):-2",
        "-q:v",
        "3",
    ]);
    command.arg(&temporary_path);

    if let Err(error) = run_ffmpeg_process(command, "生成缩略图", task, 4.0, 20.0, 1.0) {
        let _ = fs::remove_file(&temporary_path);
        return Err(error);
    }

    publish_derivative(&temporary_path, &output_path)?;
    Ok(output_path)
}

fn generate_proxy(
    ffmpeg: &Path,
    projects: &ProjectStore,
    project_id: &str,
    source_path: &Path,
    fingerprint: &str,
    duration: f64,
    task: &TaskHandle,
) -> Result<PathBuf, String> {
    let output_path = projects
        .proxies_dir(project_id)?
        .join(format!("{fingerprint}-720p.mp4"));

    if is_nonempty_file(&output_path) {
        task.set_progress(100.0, "复用代理媒体缓存");
        return Ok(output_path);
    }

    let temporary_path = temporary_derivative_path(&output_path, task, "mp4");

    let mut command = Command::new(ffmpeg);
    command.args(["-y", "-i"]);
    command.arg(source_path);
    command.args([
        "-vf",
        "scale=min(1280\\,iw):-2",
        "-c:v",
        "libx264",
        "-preset",
        "veryfast",
        "-crf",
        "23",
        "-g",
        "12",
        "-keyint_min",
        "12",
        "-an",
        "-movflags",
        "+faststart",
    ]);
    command.arg(&temporary_path);

    if let Err(error) = run_ffmpeg_process(
        command,
        "生成代理媒体",
        task,
        42.0,
        100.0,
        duration.max(1.0),
    ) {
        let _ = fs::remove_file(&temporary_path);
        return Err(error);
    }

    publish_derivative(&temporary_path, &output_path)?;
    Ok(output_path)
}

fn generate_waveform(
    ffmpeg: &Path,
    cache: &CacheStore,
    source_path: &Path,
    fingerprint: &str,
    duration: f64,
    task: &TaskHandle,
    progress_end: f64,
) -> Result<Vec<f64>, String> {
    let cache_path = cache.path_for("waveforms", fingerprint, "json")?;

    if let Ok(bytes) = fs::read(&cache_path) {
        if let Ok(peaks) = serde_json::from_slice::<Vec<f64>>(&bytes) {
            task.set_progress(progress_end, "复用波形缓存");
            return Ok(peaks);
        }
    }

    let mut child = Command::new(ffmpeg)
        .args(["-v", "error", "-i"])
        .arg(source_path)
        .args([
            "-vn",
            "-ac",
            "1",
            "-ar",
            &WAVEFORM_SAMPLE_RATE.to_string(),
            "-f",
            "f32le",
            "pipe:1",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("生成波形失败：{error}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| "生成波形失败：无法读取音频样本。".to_string())?;
    let stderr = child.stderr.take();
    let stderr_thread = thread::spawn(move || {
        let mut message = String::new();

        if let Some(stderr) = stderr {
            let _ = BufReader::new(stderr).read_to_string(&mut message);
        }

        message
    });
    let expected_samples = (duration.max(1.0) * WAVEFORM_SAMPLE_RATE as f64).ceil() as u64;
    let samples_per_peak = expected_samples.saturating_add(WAVEFORM_PEAK_COUNT as u64 - 1)
        / WAVEFORM_PEAK_COUNT as u64;
    let mut reader = BufReader::new(stdout);
    let mut peaks = vec![0.0_f64; WAVEFORM_PEAK_COUNT];
    let mut bytes = [0_u8; 4];
    let mut sample_index = 0_u64;

    loop {
        if task.is_cancel_requested() {
            let _ = child.kill();
            let _ = child.wait();
            let _ = stderr_thread.join();
            task.mark_cancelled();
            return Err("任务已取消。".to_string());
        }

        match reader.read_exact(&mut bytes) {
            Ok(()) => {
                let amplitude = f32::from_le_bytes(bytes).abs() as f64;
                let index = (sample_index / samples_per_peak.max(1))
                    .min(WAVEFORM_PEAK_COUNT as u64 - 1) as usize;
                peaks[index] = peaks[index].max(amplitude.min(1.0));
                sample_index = sample_index.saturating_add(1);

                if sample_index % (WAVEFORM_SAMPLE_RATE / 2) == 0 {
                    let ratio =
                        (sample_index as f64 / expected_samples.max(1) as f64).clamp(0.0, 1.0);
                    task.set_progress(20.0 + (progress_end - 20.0) * ratio, "生成音频波形");
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(error) => return Err(format!("生成波形失败：{error}")),
        }
    }

    let status = child
        .wait()
        .map_err(|error| format!("生成波形失败：{error}"))?;
    let stderr = stderr_thread.join().unwrap_or_default();

    if !status.success() {
        return Err(if stderr.trim().is_empty() {
            "生成波形失败：FFmpeg 没有返回详情。".to_string()
        } else {
            format!(
                "生成波形失败：{}",
                stderr.lines().last().unwrap_or_default()
            )
        });
    }

    let maximum = peaks.iter().copied().fold(0.0_f64, f64::max);

    if maximum > 0.0 {
        for peak in &mut peaks {
            *peak = (*peak / maximum * 10_000.0).round() / 10_000.0;
        }
    }

    let serialized = serde_json::to_vec(&peaks).map_err(|error| error.to_string())?;
    let temporary_path = temporary_derivative_path(&cache_path, task, "json");
    let mut file = File::create(&temporary_path).map_err(|error| error.to_string())?;
    use std::io::Write;
    file.write_all(&serialized)
        .map_err(|error| error.to_string())?;
    file.sync_all().map_err(|error| error.to_string())?;
    drop(file);
    publish_derivative(&temporary_path, &cache_path)?;
    task.set_progress(progress_end, "波形缓存已就绪");
    Ok(peaks)
}

fn is_nonempty_file(path: &Path) -> bool {
    path.metadata()
        .map(|metadata| metadata.is_file() && metadata.len() > 0)
        .unwrap_or(false)
}

fn temporary_derivative_path(output_path: &Path, task: &TaskHandle, extension: &str) -> PathBuf {
    let task_id = task.snapshot().id;
    let stem = output_path
        .file_stem()
        .and_then(|value| value.to_str())
        .unwrap_or("derivative");

    output_path.with_file_name(format!(".{stem}-{task_id}.tmp.{extension}"))
}

fn publish_derivative(temporary_path: &Path, output_path: &Path) -> Result<(), String> {
    if is_nonempty_file(output_path) {
        let _ = fs::remove_file(temporary_path);
        return Ok(());
    }

    if output_path.exists() {
        fs::remove_file(output_path).map_err(|error| error.to_string())?;
    }

    fs::rename(temporary_path, output_path).map_err(|error| error.to_string())
}
