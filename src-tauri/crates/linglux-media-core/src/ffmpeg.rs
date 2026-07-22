use crate::TaskHandle;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

pub fn discover_ffmpeg_binary(binary_name: &str) -> Option<PathBuf> {
    let candidates = [
        PathBuf::from(binary_name),
        PathBuf::from(format!("/opt/homebrew/bin/{binary_name}")),
        PathBuf::from(format!("/usr/local/bin/{binary_name}")),
        PathBuf::from(format!("/usr/bin/{binary_name}")),
    ];

    for path in candidates {
        if path.is_absolute() && !path.exists() {
            continue;
        }

        if Command::new(&path)
            .arg("-version")
            .output()
            .map(|output| output.status.success())
            .unwrap_or(false)
        {
            return Some(path);
        }
    }

    None
}

pub fn run_ffmpeg_process(
    mut command: Command,
    action: &str,
    task: &TaskHandle,
    progress_start: f64,
    progress_end: f64,
    expected_duration: f64,
) -> Result<(), String> {
    command.args(["-progress", "pipe:1", "-nostats"]);
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|error| format!("{action}失败：{error}"))?;
    let stdout = child
        .stdout
        .take()
        .ok_or_else(|| format!("{action}失败：无法读取 FFmpeg 进度。"))?;
    let stderr = child
        .stderr
        .take()
        .ok_or_else(|| format!("{action}失败：无法读取 FFmpeg 日志。"))?;
    let (progress_sender, progress_receiver) = mpsc::channel::<String>();
    let progress_reader = thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if progress_sender.send(line).is_err() {
                break;
            }
        }
    });
    let stderr_reader = thread::spawn(move || {
        BufReader::new(stderr)
            .lines()
            .map_while(Result::ok)
            .collect::<Vec<_>>()
    });

    let status = loop {
        if task.is_cancel_requested() {
            let _ = child.kill();
            let _ = child.wait();
            let _ = progress_reader.join();
            let _ = stderr_reader.join();
            task.mark_cancelled();
            return Err("任务已取消。".to_string());
        }

        while let Ok(line) = progress_receiver.try_recv() {
            if let Some(out_time_us) = line
                .strip_prefix("out_time_us=")
                .and_then(|value| value.parse::<f64>().ok())
            {
                let ratio = if expected_duration > 0.0 {
                    (out_time_us / 1_000_000.0 / expected_duration).clamp(0.0, 1.0)
                } else {
                    0.0
                };
                task.set_progress(
                    progress_start + (progress_end - progress_start) * ratio,
                    action,
                );
            }
        }

        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => thread::sleep(Duration::from_millis(80)),
            Err(error) => return Err(format!("{action}失败：{error}")),
        }
    };
    let _ = progress_reader.join();
    let stderr = stderr_reader.join().unwrap_or_default();

    if status.success() {
        task.set_progress(progress_end, action);
        return Ok(());
    }

    let summary = stderr
        .iter()
        .map(String::as_str)
        .rev()
        .filter(|line| !line.trim().is_empty())
        .take(8)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n");

    if summary.is_empty() {
        Err(format!("{action}失败：FFmpeg 未返回错误详情。"))
    } else {
        Err(format!("{action}失败：\n{summary}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{unix_millis, TaskKind, TaskManager, TaskState};
    use std::fs;

    #[test]
    fn ffmpeg_runner_reports_completion_when_ffmpeg_is_available() {
        let Some(ffmpeg) = discover_ffmpeg_binary("ffmpeg") else {
            return;
        };
        let root = std::env::temp_dir().join(format!("linglux-ffmpeg-test-{}", unix_millis()));
        fs::create_dir_all(&root).expect("create test directory");
        let output_path = root.join("progress.mp4");
        let task = TaskManager::new().create(TaskKind::Proxy, None, "test", None);
        task.start("test");
        let mut command = Command::new(ffmpeg);
        command.args(["-v", "error", "-y", "-f", "lavfi", "-i"]);
        command.arg("color=c=black:s=64x64:r=24:d=0.15");
        command.args(["-c:v", "libx264", "-pix_fmt", "yuv420p"]);
        command.arg(&output_path);

        run_ffmpeg_process(command, "test", &task, 0.0, 100.0, 0.15).expect("ffmpeg succeeds");
        task.succeed(serde_json::json!({ "path": output_path }));
        assert_eq!(task.snapshot().state, TaskState::Succeeded);
        assert!(output_path.exists());
        let _ = fs::remove_dir_all(root);
    }
}
