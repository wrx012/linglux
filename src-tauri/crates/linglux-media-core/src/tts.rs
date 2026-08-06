use crate::{import_media_paths, ImportedMediaFile, ProjectStore, TaskHandle};
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

const REQUIRED_BYTES: u64 = 3_400_000_000;
const MINIFORGE_URL: &str = "https://github.com/conda-forge/miniforge/releases/download/25.3.1-0/Miniforge3-25.3.1-0-MacOSX-arm64.sh";
const MINIFORGE_SHA256: &str = "d9eabd1868030589a1d74017b8723b01cf81b5fec1b9da8021b6fa44be7bbeae";
const COSYVOICE_REPOSITORY: &str = "https://github.com/QwenAudio/CosyVoice.git";
const COSYVOICE_REVISION: &str = "074ca6dc9e80a2f424f1f74b48bdd7d3fea531cc";
const MODEL_REPOSITORY: &str = "FunAudioLLM/CosyVoice-300M-Instruct";
const MODEL_REVISION: &str = "706bee1915e9fd1f1214929e2a0509c874cff433";
const INSTALL_SCHEMA: u32 = 1;
const WORKER_SOURCE: &str = include_str!("tts_worker.py");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TtsVoice {
    ZhMale,
    ZhFemale,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TtsEmotion {
    Natural,
    Gentle,
    Cheerful,
    Serious,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpeechSynthesisRequest {
    pub project_id: String,
    pub text: String,
    pub voice: TtsVoice,
    pub emotion: TtsEmotion,
    pub speed: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TtsVoiceOption {
    pub id: TtsVoice,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TtsStatus {
    pub supported: bool,
    pub state: String,
    pub runtime_installed: bool,
    pub model_installed: bool,
    pub required_bytes: u64,
    pub voices: Vec<TtsVoiceOption>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstallManifest {
    schema_version: u32,
    cosyvoice_revision: String,
    model_revision: String,
}

#[derive(Clone)]
pub struct TtsManager {
    root: Arc<PathBuf>,
}

impl TtsManager {
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, String> {
        let root = root.into();
        fs::create_dir_all(root.join("downloads")).map_err(|error| error.to_string())?;
        fs::create_dir_all(root.join("tmp")).map_err(|error| error.to_string())?;
        Ok(Self {
            root: Arc::new(root),
        })
    }

    pub fn status(&self) -> TtsStatus {
        let supported = cfg!(all(target_os = "macos", target_arch = "aarch64"));
        let runtime_installed = self.python_path().is_file();
        let model_installed = self.model_dir().join("cosyvoice.yaml").is_file();
        let manifest_valid = self.install_manifest_valid();
        TtsStatus {
            supported,
            state: if supported && runtime_installed && model_installed && manifest_valid {
                "ready".to_string()
            } else {
                "notInstalled".to_string()
            },
            runtime_installed,
            model_installed,
            required_bytes: REQUIRED_BYTES,
            voices: vec![
                TtsVoiceOption {
                    id: TtsVoice::ZhMale,
                    label: "中文男声".to_string(),
                },
                TtsVoiceOption {
                    id: TtsVoice::ZhFemale,
                    label: "中文女声".to_string(),
                },
            ],
            error: (!supported).then(|| "首版仅支持 macOS Apple Silicon。".to_string()),
        }
    }

    pub fn setup(&self, task: &TaskHandle) -> Result<TtsStatus, String> {
        if !cfg!(all(target_os = "macos", target_arch = "aarch64")) {
            return Err("首版仅支持 macOS Apple Silicon。".to_string());
        }
        if self.status().state == "ready" {
            return Ok(self.status());
        }

        fs::create_dir_all(self.root.as_path()).map_err(|error| error.to_string())?;
        let installer = self.root.join("downloads/miniforge.sh");
        if !self.conda_path().is_file() {
            task.set_progress(2.0, "下载本地语音运行环境");
            download_verified(MINIFORGE_URL, MINIFORGE_SHA256, &installer, task, 2.0, 18.0)?;
            run_cancellable(
                Command::new("/bin/bash")
                    .arg(&installer)
                    .arg("-b")
                    .arg("-p")
                    .arg(self.runtime_dir()),
                task,
                "安装本地语音运行环境",
            )?;
        }

        if !self.python_path().is_file() {
            task.set_progress(20.0, "创建 CosyVoice 环境");
            run_cancellable(
                Command::new(self.conda_path())
                    .args(["create", "-y", "-p"])
                    .arg(self.environment_dir())
                    .args(["python=3.10", "pip", "git"]),
                task,
                "创建 CosyVoice 环境",
            )?;
        }

        let source_dir = self.source_dir();
        if !source_dir.join("requirements.txt").is_file() {
            task.set_progress(28.0, "下载 CosyVoice 源码");
            let _ = fs::remove_dir_all(&source_dir);
            run_cancellable(
                Command::new(self.environment_dir().join("bin/git"))
                    .args(["clone", "--recursive", COSYVOICE_REPOSITORY])
                    .arg(&source_dir),
                task,
                "下载 CosyVoice 源码",
            )?;
            run_cancellable(
                Command::new(self.environment_dir().join("bin/git"))
                    .arg("-C")
                    .arg(&source_dir)
                    .args(["checkout", COSYVOICE_REVISION]),
                task,
                "固定 CosyVoice 版本",
            )?;
            run_cancellable(
                Command::new(self.environment_dir().join("bin/git"))
                    .arg("-C")
                    .arg(&source_dir)
                    .args(["submodule", "update", "--init", "--recursive"]),
                task,
                "同步语音依赖",
            )?;
        }

        task.set_progress(38.0, "安装 CosyVoice 依赖");
        run_cancellable(
            Command::new(self.python_path())
                .args(["-m", "pip", "install", "-r"])
                .arg(source_dir.join("requirements.txt")),
            task,
            "安装 CosyVoice 依赖",
        )?;

        task.set_progress(58.0, "下载中文语音模型（约 2.3 GB）");
        let download_script = format!(
            "from huggingface_hub import snapshot_download; snapshot_download(repo_id={MODEL_REPOSITORY:?}, revision={MODEL_REVISION:?}, local_dir={:?})",
            self.model_dir().to_string_lossy()
        );
        run_cancellable(
            Command::new(self.python_path()).args(["-c", &download_script]),
            task,
            "下载中文语音模型",
        )?;

        fs::write(self.worker_path(), WORKER_SOURCE).map_err(|error| error.to_string())?;
        let manifest = InstallManifest {
            schema_version: INSTALL_SCHEMA,
            cosyvoice_revision: COSYVOICE_REVISION.to_string(),
            model_revision: MODEL_REVISION.to_string(),
        };
        fs::write(
            self.root.join("install.json"),
            serde_json::to_vec_pretty(&manifest).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
        task.set_progress(100.0, "本地语音模型已就绪");
        Ok(self.status())
    }

    pub fn synthesize(
        &self,
        store: &ProjectStore,
        request: &SpeechSynthesisRequest,
        task: &TaskHandle,
    ) -> Result<ImportedMediaFile, String> {
        validate_request(request)?;
        if self.status().state != "ready" {
            return Err("本地语音模型尚未安装。".to_string());
        }

        fs::create_dir_all(self.root.join("tmp")).map_err(|error| error.to_string())?;
        let output = self
            .root
            .join("tmp")
            .join(format!("linglux-voice-{}.wav", crate::unix_millis()));
        let payload = serde_json::json!({
            "text": request.text.trim(),
            "voice": request.voice,
            "emotion": request.emotion,
            "speed": request.speed,
            "modelDir": self.model_dir(),
            "sourceDir": self.source_dir(),
            "outputPath": output,
        });
        task.set_progress(5.0, "加载本地语音模型");
        let worker_result = run_worker(&self.python_path(), &self.worker_path(), &payload, task)?;
        if worker_result.get("ok").and_then(|value| value.as_bool()) != Some(true) {
            let error = worker_result
                .get("error")
                .and_then(|value| value.as_str())
                .unwrap_or("语音生成失败。");
            let _ = fs::remove_file(&output);
            return Err(error.to_string());
        }
        validate_wav(&output)?;
        task.set_progress(92.0, "保存配音到工程");
        let imported = import_media_paths(store, &request.project_id, &[output.clone()], task)
            .and_then(|mut files| {
                files
                    .pop()
                    .ok_or_else(|| "生成的配音未能导入工程。".to_string())
            });
        let _ = fs::remove_file(output);
        imported
    }

    fn runtime_dir(&self) -> PathBuf {
        self.root.join("runtime")
    }
    fn conda_path(&self) -> PathBuf {
        self.runtime_dir().join("bin/conda")
    }
    fn environment_dir(&self) -> PathBuf {
        self.runtime_dir().join("envs/cosyvoice")
    }
    fn python_path(&self) -> PathBuf {
        self.environment_dir().join("bin/python")
    }
    fn source_dir(&self) -> PathBuf {
        self.root.join("cosyvoice-source")
    }
    fn model_dir(&self) -> PathBuf {
        self.root.join("cosyvoice-300m-instruct")
    }
    fn worker_path(&self) -> PathBuf {
        self.root.join("worker.py")
    }

    fn install_manifest_valid(&self) -> bool {
        fs::read(self.root.join("install.json"))
            .ok()
            .and_then(|bytes| serde_json::from_slice::<InstallManifest>(&bytes).ok())
            .is_some_and(|manifest| {
                manifest.schema_version == INSTALL_SCHEMA
                    && manifest.cosyvoice_revision == COSYVOICE_REVISION
                    && manifest.model_revision == MODEL_REVISION
            })
    }
}

fn validate_request(request: &SpeechSynthesisRequest) -> Result<(), String> {
    let text = request.text.trim();
    if text.is_empty() {
        return Err("请输入需要生成的台词。".to_string());
    }
    if text.chars().count() > 1000 {
        return Err("台词不能超过 1000 个字符。".to_string());
    }
    if !(0.75..=1.5).contains(&request.speed) {
        return Err("语速必须在 0.75× 到 1.50× 之间。".to_string());
    }
    if request.project_id.trim().is_empty() {
        return Err("缺少工程 ID。".to_string());
    }
    Ok(())
}

fn download_verified(
    url: &str,
    expected_sha: &str,
    target: &Path,
    task: &TaskHandle,
    start: f64,
    end: f64,
) -> Result<(), String> {
    let partial = target.with_extension("partial");
    let _ = fs::remove_file(&partial);
    let mut response = Client::new()
        .get(url)
        .send()
        .map_err(|error| error.to_string())?
        .error_for_status()
        .map_err(|error| error.to_string())?;
    let total = response.content_length().unwrap_or(1).max(1);
    let mut file = File::create(&partial).map_err(|error| error.to_string())?;
    let mut hasher = Sha256::new();
    let mut downloaded = 0_u64;
    let mut buffer = [0_u8; 256 * 1024];
    loop {
        if task.is_cancel_requested() {
            let _ = fs::remove_file(&partial);
            task.mark_cancelled();
            return Err("任务已取消。".to_string());
        }
        let read = response
            .read(&mut buffer)
            .map_err(|error| error.to_string())?;
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read])
            .map_err(|error| error.to_string())?;
        hasher.update(&buffer[..read]);
        downloaded += read as u64;
        task.set_progress(
            start + (end - start) * downloaded as f64 / total as f64,
            "下载本地语音运行环境",
        );
    }
    file.sync_all().map_err(|error| error.to_string())?;
    let actual = format!("{:x}", hasher.finalize());
    if actual != expected_sha {
        let _ = fs::remove_file(&partial);
        return Err("运行环境校验失败，请重试。".to_string());
    }
    fs::rename(&partial, target).map_err(|error| error.to_string())
}

fn run_cancellable(command: &mut Command, task: &TaskHandle, status: &str) -> Result<(), String> {
    command.stdout(Stdio::null()).stderr(Stdio::null());
    let mut child = command
        .spawn()
        .map_err(|error| format!("{status}失败：{error}"))?;
    loop {
        if task.is_cancel_requested() {
            let _ = child.kill();
            let _ = child.wait();
            task.mark_cancelled();
            return Err("任务已取消。".to_string());
        }
        if let Some(exit) = child.try_wait().map_err(|error| error.to_string())? {
            if exit.success() {
                return Ok(());
            }
            return Err(format!("{status}失败（退出码 {:?}）。", exit.code()));
        }
        thread::sleep(Duration::from_millis(120));
    }
}

fn run_worker(
    python: &Path,
    worker: &Path,
    payload: &serde_json::Value,
    task: &TaskHandle,
) -> Result<serde_json::Value, String> {
    let mut child = Command::new(python)
        .arg(worker)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| error.to_string())?;
    let mut stdout = child
        .stdout
        .take()
        .ok_or_else(|| "无法读取语音生成结果。".to_string())?;
    let mut stderr = child
        .stderr
        .take()
        .ok_or_else(|| "无法读取语音生成日志。".to_string())?;
    let stdout_reader = thread::spawn(move || {
        let mut value = String::new();
        let _ = stdout.read_to_string(&mut value);
        value
    });
    let stderr_reader = thread::spawn(move || {
        let mut value = String::new();
        let _ = stderr.read_to_string(&mut value);
        value
    });
    if let Some(mut stdin) = child.stdin.take() {
        writeln!(stdin, "{}", payload).map_err(|error| error.to_string())?;
    }
    loop {
        if task.is_cancel_requested() {
            let _ = child.kill();
            let _ = child.wait();
            task.mark_cancelled();
            return Err("任务已取消。".to_string());
        }
        if child
            .try_wait()
            .map_err(|error| error.to_string())?
            .is_some()
        {
            break;
        }
        task.set_progress(65.0, "正在生成配音");
        thread::sleep(Duration::from_millis(150));
    }
    let status = child.wait().map_err(|error| error.to_string())?;
    let stdout = stdout_reader.join().unwrap_or_default();
    let stderr = stderr_reader.join().unwrap_or_default();
    if !status.success() {
        return Err(stderr
            .lines()
            .last()
            .unwrap_or("语音生成进程失败。")
            .to_string());
    }
    let line = stdout.lines().last().unwrap_or_default().to_string();
    serde_json::from_str(&line).map_err(|_| "语音生成进程返回了无效结果。".to_string())
}

fn validate_wav(path: &Path) -> Result<(), String> {
    let bytes = fs::read(path).map_err(|error| error.to_string())?;
    if bytes.len() < 44 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        return Err("生成的音频不是有效的 WAV 文件。".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validates_text_and_speed() {
        let mut request = SpeechSynthesisRequest {
            project_id: "p".into(),
            text: "今天天气很好".into(),
            voice: TtsVoice::ZhFemale,
            emotion: TtsEmotion::Natural,
            speed: 1.0,
        };
        assert!(validate_request(&request).is_ok());
        request.speed = 2.0;
        assert!(validate_request(&request).is_err());
        request.speed = 1.0;
        request.text = " ".into();
        assert!(validate_request(&request).is_err());
    }
}
