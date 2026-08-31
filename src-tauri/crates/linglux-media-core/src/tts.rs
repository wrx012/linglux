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
use std::time::{Duration, Instant};

const REQUIRED_BYTES: u64 = 2_700_000_000;
const MINIFORGE_URL: &str = "https://github.com/conda-forge/miniforge/releases/download/25.3.1-0/Miniforge3-25.3.1-0-MacOSX-arm64.sh";
const MINIFORGE_SHA256: &str = "d9eabd1868030589a1d74017b8723b01cf81b5fec1b9da8021b6fa44be7bbeae";
const MODEL_REPOSITORY: &str = "hexgrad/Kokoro-82M-v1.1-zh";
const MODEL_REVISION: &str = "01e7505bd6a7a2ac4975463114c3a7650a9f7218";
const INSTALL_SCHEMA: u32 = 3;
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
    pub legacy_bytes: u64,
    pub voices: Vec<TtsVoiceOption>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct InstallManifest {
    schema_version: u32,
    model_revision: String,
}

#[derive(Clone)]
pub struct TtsManager {
    root: Arc<PathBuf>,
    runtime_dir: Arc<PathBuf>,
}

impl TtsManager {
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, String> {
        let root = root.into();
        fs::create_dir_all(root.join("downloads")).map_err(|error| error.to_string())?;
        fs::create_dir_all(root.join("tmp")).map_err(|error| error.to_string())?;
        let runtime_dir = prepare_runtime_install_path(&root)?;
        Ok(Self {
            root: Arc::new(root),
            runtime_dir: Arc::new(runtime_dir),
        })
    }

    pub fn status(&self) -> TtsStatus {
        let supported = cfg!(all(target_os = "macos", target_arch = "aarch64"));
        let runtime_installed = self.python_path().is_file();
        let model_installed = self.required_model_files().iter().all(|path| {
            fs::metadata(path)
                .map(|metadata| metadata.is_file() && metadata.len() > 0)
                .unwrap_or(false)
        });
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
            legacy_bytes: self.legacy_install_bytes(),
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
            task.set_progress(20.0, "创建轻量语音环境");
            let mut conda = Command::new(self.runtime_dir().join("bin/python"));
            conda.arg(self.conda_path());
            run_cancellable(
                conda
                    .args(["create", "-y", "-p"])
                    .arg(self.environment_dir())
                    .args(["python=3.10", "pip"]),
                task,
                "创建轻量语音环境",
            )?;
        }

        task.set_progress(30.0, "安装轻量语音依赖");
        run_cancellable(
            Command::new(self.python_path()).args([
                "-m",
                "pip",
                "install",
                "--no-cache-dir",
                "--no-compile",
                "kokoro==0.9.4",
                "misaki[zh]==0.9.4",
                "soundfile==0.13.1",
            ]),
            task,
            "安装轻量语音依赖",
        )?;

        task.set_progress(62.0, "下载中文语音模型（约 400 MB）");
        let download_script = format!(
            "from huggingface_hub import snapshot_download; snapshot_download(repo_id={MODEL_REPOSITORY:?}, revision={MODEL_REVISION:?}, local_dir={:?}, allow_patterns=['config.json','kokoro-v1_1-zh.pth','voices/zf_001.pt','voices/zm_010.pt'])",
            self.model_dir().to_string_lossy()
        );
        run_cancellable(
            Command::new(self.python_path()).args(["-c", &download_script]),
            task,
            "下载中文语音模型",
        )?;

        task.set_progress(92.0, "清理安装缓存");
        let mut conda = Command::new(self.runtime_dir().join("bin/python"));
        conda.arg(self.conda_path());
        run_cancellable(conda.args(["clean", "-a", "-y"]), task, "清理安装缓存")?;
        remove_python_bytecode_caches(&self.environment_dir())?;
        let _ = fs::remove_dir_all(self.runtime_dir().join("pkgs"));
        let _ = fs::remove_file(&installer);
        self.remove_legacy_install()?;

        fs::write(self.worker_path(), WORKER_SOURCE).map_err(|error| error.to_string())?;
        let manifest = InstallManifest {
            schema_version: INSTALL_SCHEMA,
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
        self.runtime_dir.as_ref().clone()
    }
    fn conda_path(&self) -> PathBuf {
        self.runtime_dir().join("bin/conda")
    }
    fn environment_dir(&self) -> PathBuf {
        self.runtime_dir().join("envs/kokoro")
    }
    fn python_path(&self) -> PathBuf {
        self.environment_dir().join("bin/python")
    }
    fn model_dir(&self) -> PathBuf {
        self.root.join("kokoro-82m-zh")
    }
    fn required_model_files(&self) -> [PathBuf; 4] {
        [
            self.model_dir().join("config.json"),
            self.model_dir().join("kokoro-v1_1-zh.pth"),
            self.model_dir().join("voices/zf_001.pt"),
            self.model_dir().join("voices/zm_010.pt"),
        ]
    }
    fn legacy_install_paths(&self) -> [PathBuf; 3] {
        [
            self.runtime_dir().join("envs/cosyvoice"),
            self.root.join("cosyvoice-source"),
            self.root.join("cosyvoice-300m-instruct"),
        ]
    }
    fn legacy_install_bytes(&self) -> u64 {
        self.legacy_install_paths()
            .iter()
            .map(|path| directory_size(path).unwrap_or(0))
            .sum()
    }
    fn remove_legacy_install(&self) -> Result<(), String> {
        for path in self.legacy_install_paths() {
            if path.exists() {
                fs::remove_dir_all(&path).map_err(|error| {
                    format!("清理旧版语音模型失败（{}）：{error}", path.display())
                })?;
            }
        }
        Ok(())
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
                    && manifest.model_revision == MODEL_REVISION
            })
    }
}

fn prepare_runtime_install_path(root: &Path) -> Result<PathBuf, String> {
    let runtime = root.join("runtime");
    if !runtime.to_string_lossy().contains(char::is_whitespace) {
        return Ok(runtime);
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        let home = std::env::var_os("HOME")
            .map(PathBuf::from)
            .ok_or_else(|| "无法确定用户目录，不能安装本地语音运行环境。".to_string())?;
        let digest = Sha256::digest(root.to_string_lossy().as_bytes());
        let digest_hex = format!("{digest:x}");
        let alias_parent = home.join(".linglux/runtime-links");
        let alias = alias_parent.join(format!("tts-{}", &digest_hex[..16]));
        if alias.to_string_lossy().contains(char::is_whitespace) {
            return Err("用户目录路径包含空格，当前本地语音运行环境无法安装。".to_string());
        }
        fs::create_dir_all(&alias_parent).map_err(|error| error.to_string())?;
        match fs::symlink_metadata(&alias) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                let target = fs::read_link(&alias).map_err(|error| error.to_string())?;
                if target != root {
                    fs::remove_file(&alias).map_err(|error| error.to_string())?;
                    symlink(root, &alias).map_err(|error| error.to_string())?;
                }
            }
            Ok(_) => return Err(format!("本地语音运行环境别名已被占用：{}", alias.display())),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                symlink(root, &alias).map_err(|error| error.to_string())?;
            }
            Err(error) => return Err(error.to_string()),
        }
        return Ok(alias.join("runtime"));
    }

    #[cfg(not(unix))]
    Ok(runtime)
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

fn remove_python_bytecode_caches(root: &Path) -> Result<(), String> {
    if !root.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(root).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let path = entry.path();
        if path.is_dir() && entry.file_name() == "__pycache__" {
            fs::remove_dir_all(path).map_err(|error| error.to_string())?;
        } else if path.is_dir() {
            remove_python_bytecode_caches(&path)?;
        } else if path.extension().and_then(|value| value.to_str()) == Some("pyc") {
            fs::remove_file(path).map_err(|error| error.to_string())?;
        }
    }
    Ok(())
}

fn directory_size(path: &Path) -> Result<u64, String> {
    let metadata = fs::symlink_metadata(path).map_err(|error| error.to_string())?;
    if metadata.file_type().is_symlink() {
        return Ok(0);
    }
    if metadata.is_file() {
        return Ok(metadata.len());
    }
    if !metadata.is_dir() {
        return Ok(0);
    }
    fs::read_dir(path)
        .map_err(|error| error.to_string())?
        .map(|entry| entry.map_err(|error| error.to_string()))
        .try_fold(0_u64, |total, entry| {
            let entry = entry?;
            directory_size(&entry.path()).map(|size| total.saturating_add(size))
        })
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
    let log_path = std::env::temp_dir().join(format!(
        "linglux-command-{}-{}.log",
        std::process::id(),
        crate::unix_millis()
    ));
    let log = File::create(&log_path).map_err(|error| format!("{status}失败：{error}"))?;
    command
        .stdout(Stdio::from(
            log.try_clone().map_err(|error| error.to_string())?,
        ))
        .stderr(Stdio::from(log));
    let mut child = command
        .spawn()
        .map_err(|error| format!("{status}失败：{error}"))?;
    loop {
        if task.is_cancel_requested() {
            let _ = child.kill();
            let _ = child.wait();
            let _ = fs::remove_file(&log_path);
            task.mark_cancelled();
            return Err("任务已取消。".to_string());
        }
        if let Some(exit) = child.try_wait().map_err(|error| error.to_string())? {
            if exit.success() {
                let _ = fs::remove_file(&log_path);
                return Ok(());
            }
            let output = fs::read_to_string(&log_path).unwrap_or_default();
            let _ = fs::remove_file(&log_path);
            let details = output
                .lines()
                .rev()
                .filter(|line| !line.trim().is_empty())
                .take(8)
                .collect::<Vec<_>>()
                .into_iter()
                .rev()
                .collect::<Vec<_>>()
                .join("\n");
            return Err(if details.is_empty() {
                format!("{status}失败（退出码 {:?}）。", exit.code())
            } else {
                format!("{status}失败：{}", details.trim())
            });
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
    let started_at = Instant::now();
    let mut last_progress_second = u64::MAX;
    let mut child = Command::new(python)
        .arg(worker)
        .env("PYTHONDONTWRITEBYTECODE", "1")
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
        let elapsed_seconds = started_at.elapsed().as_secs();
        if elapsed_seconds != last_progress_second {
            let (progress, status) = synthesis_progress(elapsed_seconds);
            task.set_progress(progress, status);
            last_progress_second = elapsed_seconds;
        }
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

fn synthesis_progress(elapsed_seconds: u64) -> (f64, &'static str) {
    let progress = (55.0 + elapsed_seconds as f64 * 1.7).min(89.0);
    let status = if elapsed_seconds < 4 {
        "正在加载轻量语音模型"
    } else if elapsed_seconds < 8 {
        "正在解析中文台词"
    } else {
        "正在生成配音"
    };
    (progress, status)
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

    #[test]
    fn synthesis_progress_keeps_moving_while_inference_runs() {
        let samples = [0, 4, 8, 12, 20, 60].map(synthesis_progress);
        assert_eq!(samples[0].1, "正在加载轻量语音模型");
        assert_eq!(samples[1].1, "正在解析中文台词");
        assert_eq!(samples[2].1, "正在生成配音");
        assert!(samples[..5].windows(2).all(|pair| pair[0].0 < pair[1].0));
        assert_eq!(samples.last().expect("last progress").0, 89.0);
    }

    #[cfg(unix)]
    #[test]
    fn creates_a_stable_space_free_runtime_alias() {
        let root = std::env::temp_dir().join(format!("linglux tts alias {}", crate::unix_millis()));
        let manager = TtsManager::new(&root).expect("create TTS manager");
        let runtime = manager.runtime_dir();
        let alias = runtime.parent().expect("runtime alias").to_path_buf();

        assert!(!runtime.to_string_lossy().contains(char::is_whitespace));
        assert_eq!(fs::read_link(&alias).expect("read runtime alias"), root);

        let _ = fs::remove_file(alias);
        let _ = fs::remove_dir_all(root);
    }

    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    #[test]
    fn ready_requires_runtime_manifest_model_config_and_both_voices() {
        let root = std::env::temp_dir().join(format!("linglux tts ready {}", crate::unix_millis()));
        let manager = TtsManager::new(&root).expect("create TTS manager");
        fs::create_dir_all(manager.python_path().parent().expect("python parent"))
            .expect("create runtime");
        fs::write(manager.python_path(), b"python").expect("write python marker");
        for path in manager.required_model_files() {
            fs::create_dir_all(path.parent().expect("model parent")).expect("create model dir");
            fs::write(path, b"model").expect("write model marker");
        }
        fs::write(
            root.join("install.json"),
            serde_json::to_vec(&InstallManifest {
                schema_version: INSTALL_SCHEMA,
                model_revision: MODEL_REVISION.to_string(),
            })
            .expect("serialize manifest"),
        )
        .expect("write manifest");

        assert_eq!(manager.status().state, "ready");
        fs::remove_file(manager.model_dir().join("voices/zm_010.pt")).expect("remove voice");
        assert_eq!(manager.status().state, "notInstalled");

        let alias = manager
            .runtime_dir()
            .parent()
            .expect("runtime alias")
            .to_path_buf();
        let _ = fs::remove_file(alias);
        let _ = fs::remove_dir_all(root);
    }
}
