use crate::unix_millis;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

type TaskListener = Arc<dyn Fn(TaskEvent) + Send + Sync + 'static>;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TaskKind {
    Import,
    Export,
    Proxy,
    Thumbnail,
    Waveform,
    ProjectSave,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum TaskState {
    Queued,
    Running,
    Cancelling,
    Cancelled,
    Succeeded,
    Failed,
    Interrupted,
}

impl TaskState {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Cancelled | Self::Succeeded | Self::Failed | Self::Interrupted
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskSnapshot {
    pub id: String,
    pub kind: TaskKind,
    pub state: TaskState,
    pub project_id: Option<String>,
    pub label: String,
    pub progress: f64,
    pub status: String,
    pub created_at_ms: u64,
    pub updated_at_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TaskEvent {
    pub task: TaskSnapshot,
}

struct TaskRecord {
    snapshot: Mutex<TaskSnapshot>,
    cancel_requested: AtomicBool,
    listener: Option<TaskListener>,
    journal_dir: Option<Arc<PathBuf>>,
}

#[derive(Clone)]
pub struct TaskHandle {
    record: Arc<TaskRecord>,
}

impl TaskHandle {
    pub fn snapshot(&self) -> TaskSnapshot {
        self.record
            .snapshot
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .clone()
    }

    pub fn is_cancel_requested(&self) -> bool {
        self.record.cancel_requested.load(Ordering::Relaxed)
    }

    pub fn start(&self, status: impl Into<String>) {
        self.update(|snapshot| {
            snapshot.state = TaskState::Running;
            snapshot.status = status.into();
            snapshot.progress = snapshot.progress.max(0.0);
        });
    }

    pub fn set_progress(&self, progress: f64, status: impl Into<String>) {
        let progress = progress.clamp(0.0, 100.0);
        let status = status.into();
        let current = self.snapshot();

        if (current.progress - progress).abs() < 0.1
            && current.status == status
            && current.state == TaskState::Running
        {
            return;
        }

        self.update(|snapshot| {
            if snapshot.state.is_terminal() {
                return;
            }

            snapshot.state = if self.is_cancel_requested() {
                TaskState::Cancelling
            } else {
                TaskState::Running
            };
            snapshot.progress = progress;
            snapshot.status = status;
        });
    }

    pub fn succeed(&self, result: Value) {
        self.update(|snapshot| {
            snapshot.state = TaskState::Succeeded;
            snapshot.progress = 100.0;
            snapshot.status = "完成".to_string();
            snapshot.result = Some(result);
            snapshot.error = None;
        });
    }

    pub fn fail(&self, error: impl Into<String>) {
        self.update(|snapshot| {
            snapshot.state = TaskState::Failed;
            snapshot.status = "失败".to_string();
            snapshot.error = Some(error.into());
        });
    }

    pub fn mark_cancelled(&self) {
        self.update(|snapshot| {
            snapshot.state = TaskState::Cancelled;
            snapshot.status = "已取消".to_string();
            snapshot.error = None;
        });
    }

    fn request_cancel(&self) -> bool {
        if self.snapshot().state.is_terminal() {
            return false;
        }

        self.record.cancel_requested.store(true, Ordering::Relaxed);
        self.update(|snapshot| {
            snapshot.state = TaskState::Cancelling;
            snapshot.status = "正在取消".to_string();
        });
        true
    }

    fn update(&self, update: impl FnOnce(&mut TaskSnapshot)) {
        let snapshot = {
            let mut snapshot = self
                .record
                .snapshot
                .lock()
                .unwrap_or_else(|poisoned| poisoned.into_inner());
            update(&mut snapshot);
            snapshot.updated_at_ms = unix_millis();
            snapshot.clone()
        };

        if let Some(listener) = &self.record.listener {
            listener(TaskEvent { task: snapshot });
        }

        if let Some(journal_dir) = &self.record.journal_dir {
            let _ = persist_task_snapshot(journal_dir.as_path(), &self.snapshot());
        }
    }
}

struct TaskManagerInner {
    next_id: AtomicU64,
    tasks: Mutex<HashMap<String, TaskHandle>>,
    journal_dir: Option<Arc<PathBuf>>,
}

#[derive(Clone)]
pub struct TaskManager {
    inner: Arc<TaskManagerInner>,
}

impl TaskManager {
    pub fn new() -> Self {
        Self::from_journal_dir(None, HashMap::new())
    }

    pub fn with_journal(journal_dir: impl Into<PathBuf>) -> Result<Self, String> {
        let journal_dir = journal_dir.into();
        fs::create_dir_all(&journal_dir).map_err(|error| error.to_string())?;
        let journal_dir = Arc::new(journal_dir);
        let mut restored = HashMap::new();

        for entry in fs::read_dir(journal_dir.as_path())
            .map_err(|error| error.to_string())?
            .flatten()
        {
            let path = entry.path();

            if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
                continue;
            }

            let Ok(bytes) = fs::read(&path) else {
                continue;
            };
            let Ok(mut snapshot) = serde_json::from_slice::<TaskSnapshot>(&bytes) else {
                continue;
            };

            if !snapshot.state.is_terminal() {
                snapshot.state = TaskState::Interrupted;
                snapshot.status = "上次运行被中断".to_string();
                snapshot.error = Some("应用退出时任务尚未完成，可以重新执行。".to_string());
                snapshot.updated_at_ms = unix_millis();
                let _ = persist_task_snapshot(journal_dir.as_path(), &snapshot);
            }

            restored.insert(
                snapshot.id.clone(),
                TaskHandle {
                    record: Arc::new(TaskRecord {
                        snapshot: Mutex::new(snapshot),
                        cancel_requested: AtomicBool::new(false),
                        listener: None,
                        journal_dir: Some(journal_dir.clone()),
                    }),
                },
            );
        }

        Ok(Self::from_journal_dir(Some(journal_dir), restored))
    }

    fn from_journal_dir(
        journal_dir: Option<Arc<PathBuf>>,
        tasks: HashMap<String, TaskHandle>,
    ) -> Self {
        Self {
            inner: Arc::new(TaskManagerInner {
                next_id: AtomicU64::new(tasks.len() as u64 + 1),
                tasks: Mutex::new(tasks),
                journal_dir,
            }),
        }
    }

    pub fn create(
        &self,
        kind: TaskKind,
        project_id: Option<String>,
        label: impl Into<String>,
        listener: Option<Arc<dyn Fn(TaskEvent) + Send + Sync + 'static>>,
    ) -> TaskHandle {
        let created_at_ms = unix_millis();
        let sequence = self.inner.next_id.fetch_add(1, Ordering::Relaxed);
        let id = format!("task-{created_at_ms}-{sequence}");
        let handle = TaskHandle {
            record: Arc::new(TaskRecord {
                snapshot: Mutex::new(TaskSnapshot {
                    id: id.clone(),
                    kind,
                    state: TaskState::Queued,
                    project_id,
                    label: label.into(),
                    progress: 0.0,
                    status: "等待中".to_string(),
                    created_at_ms,
                    updated_at_ms: created_at_ms,
                    result: None,
                    error: None,
                }),
                cancel_requested: AtomicBool::new(false),
                listener,
                journal_dir: self.inner.journal_dir.clone(),
            }),
        };

        self.inner
            .tasks
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .insert(id, handle.clone());
        if let Some(journal_dir) = &self.inner.journal_dir {
            let _ = persist_task_snapshot(journal_dir.as_path(), &handle.snapshot());
        }
        handle
    }

    pub fn get(&self, task_id: &str) -> Option<TaskSnapshot> {
        self.inner
            .tasks
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(task_id)
            .map(TaskHandle::snapshot)
    }

    pub fn list(&self, project_id: Option<&str>) -> Vec<TaskSnapshot> {
        let mut tasks = self
            .inner
            .tasks
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .values()
            .map(TaskHandle::snapshot)
            .filter(|task| {
                project_id
                    .map(|project_id| task.project_id.as_deref() == Some(project_id))
                    .unwrap_or(true)
            })
            .collect::<Vec<_>>();
        tasks.sort_by_key(|task| task.created_at_ms);
        tasks
    }

    pub fn cancel(&self, task_id: &str) -> bool {
        self.inner
            .tasks
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .get(task_id)
            .map(TaskHandle::request_cancel)
            .unwrap_or(false)
    }
}

fn persist_task_snapshot(journal_dir: &Path, snapshot: &TaskSnapshot) -> Result<(), String> {
    fs::create_dir_all(journal_dir).map_err(|error| error.to_string())?;
    let path = journal_dir.join(format!("{}.json", snapshot.id));
    let next = journal_dir.join(format!("{}.json.next", snapshot.id));
    let bytes = serde_json::to_vec_pretty(snapshot).map_err(|error| error.to_string())?;
    fs::write(&next, bytes).map_err(|error| error.to_string())?;

    if path.exists() {
        fs::remove_file(&path).map_err(|error| error.to_string())?;
    }

    fs::rename(next, path).map_err(|error| error.to_string())
}

impl Default for TaskManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_lifecycle_is_observable() {
        let manager = TaskManager::new();
        let task = manager.create(TaskKind::Export, Some("project-1".into()), "导出", None);
        task.start("准备");
        task.set_progress(42.0, "编码");

        let snapshot = manager.get(&task.snapshot().id).expect("task exists");
        assert_eq!(snapshot.state, TaskState::Running);
        assert_eq!(snapshot.progress, 42.0);

        assert!(manager.cancel(&snapshot.id));
        assert!(task.is_cancel_requested());
        task.mark_cancelled();
        assert_eq!(task.snapshot().state, TaskState::Cancelled);
    }

    #[test]
    fn unfinished_journalled_tasks_are_restored_as_interrupted() {
        let journal_dir = std::env::temp_dir().join(format!("linglux-task-test-{}", unix_millis()));
        let task_id = {
            let manager = TaskManager::with_journal(&journal_dir).expect("create journal");
            let task = manager.create(TaskKind::Proxy, Some("project-1".into()), "proxy", None);
            task.start("encoding");
            task.snapshot().id
        };

        let restored = TaskManager::with_journal(&journal_dir).expect("restore journal");
        let snapshot = restored.get(&task_id).expect("restored task");
        assert_eq!(snapshot.state, TaskState::Interrupted);
        assert!(snapshot.error.is_some());
        let _ = fs::remove_dir_all(journal_dir);
    }
}
