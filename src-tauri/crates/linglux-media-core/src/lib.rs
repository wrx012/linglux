mod cache;
mod executor;
mod ffmpeg;
mod import;
mod media;
mod project_store;
mod storyboard;
mod task;
mod tts;

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub use cache::{CacheStats, CacheStore};
pub use executor::TaskExecutor;
pub use ffmpeg::{discover_ffmpeg_binary, run_ffmpeg_process};
pub use import::{import_media_paths, ImportedMediaFile};
pub use media::{generate_media_derivatives, MediaDerivatives, MediaKind, MediaMetadata};
pub use project_store::{ProjectDocument, ProjectSaveResult, ProjectStore, PROJECT_SCHEMA_VERSION};
pub use storyboard::{
    storyboard_to_video, StoryboardFrameRect, StoryboardToVideoRequest, StoryboardToVideoResult,
};
pub use task::{TaskEvent, TaskHandle, TaskKind, TaskManager, TaskSnapshot, TaskState};
pub use tts::{SpeechSynthesisRequest, TtsEmotion, TtsManager, TtsStatus, TtsVoice};

#[derive(Clone)]
pub struct MediaCore {
    root: Arc<PathBuf>,
    projects: ProjectStore,
    cache: CacheStore,
    tasks: TaskManager,
    executor: TaskExecutor,
    tts: TtsManager,
}

impl MediaCore {
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, String> {
        let root = root.into();
        fs::create_dir_all(&root).map_err(|error| error.to_string())?;

        let tasks = TaskManager::with_journal(root.join("tasks"))?;

        let tts = TtsManager::new(root.join("tts"))?;
        Ok(Self {
            projects: ProjectStore::new(root.join("projects"))?,
            cache: CacheStore::new(root.join("cache"))?,
            tasks,
            executor: TaskExecutor::new(1, 2),
            tts,
            root: Arc::new(root),
        })
    }

    pub fn root(&self) -> &Path {
        self.root.as_path()
    }

    pub fn projects(&self) -> &ProjectStore {
        &self.projects
    }

    pub fn cache(&self) -> &CacheStore {
        &self.cache
    }

    pub fn tasks(&self) -> &TaskManager {
        &self.tasks
    }

    pub fn tts(&self) -> &TtsManager {
        &self.tts
    }

    pub fn submit(
        &self,
        kind: TaskKind,
        job: impl FnOnce() + Send + 'static,
    ) -> Result<(), String> {
        self.executor.submit(kind, job)
    }
}

pub(crate) fn sanitize_path_component(value: &str, fallback: &str) -> String {
    let sanitized = value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('.')
        .to_string();

    if sanitized.is_empty() {
        fallback.to_string()
    } else {
        sanitized
    }
}

pub(crate) fn unix_millis() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis().min(u64::MAX as u128) as u64)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_components_are_safe() {
        assert_eq!(
            sanitize_path_component("project:demo/one", "project"),
            "project_demo_one"
        );
        assert_eq!(sanitize_path_component("...", "project"), "project");
    }
}
