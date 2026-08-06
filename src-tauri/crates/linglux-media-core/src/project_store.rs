use crate::{sanitize_path_component, unix_millis};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;

pub const PROJECT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectDocument {
    pub schema_version: u32,
    pub revision: u64,
    pub saved_at_ms: u64,
    pub project: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectSaveResult {
    pub revision: u64,
    pub manifest_path: String,
}

#[derive(Clone)]
pub struct ProjectStore {
    root: Arc<PathBuf>,
}

impl ProjectStore {
    pub fn new(root: impl Into<PathBuf>) -> Result<Self, String> {
        let root = root.into();
        fs::create_dir_all(&root).map_err(|error| error.to_string())?;
        Ok(Self {
            root: Arc::new(root),
        })
    }

    pub fn project_dir(&self, project_id: &str) -> PathBuf {
        self.root.join(format!(
            "{}.linglux",
            sanitize_path_component(project_id, "project")
        ))
    }

    pub fn media_dir(&self, project_id: &str) -> Result<PathBuf, String> {
        let path = self.project_dir(project_id).join("media");
        fs::create_dir_all(&path).map_err(|error| error.to_string())?;
        Ok(path)
    }

    pub fn proxies_dir(&self, project_id: &str) -> Result<PathBuf, String> {
        let path = self.project_dir(project_id).join("proxies");
        fs::create_dir_all(&path).map_err(|error| error.to_string())?;
        Ok(path)
    }

    pub fn save_json(&self, project_id: &str, project: Value) -> Result<ProjectSaveResult, String> {
        let project_dir = self.project_dir(project_id);
        fs::create_dir_all(&project_dir).map_err(|error| error.to_string())?;
        let manifest_path = project_dir.join("manifest.json");
        let backup_path = project_dir.join("manifest.json.bak");
        let next_path = project_dir.join("manifest.json.next");
        let revision = self
            .load(project_id)
            .map(|document| document.revision.saturating_add(1))
            .unwrap_or(1);
        let document = ProjectDocument {
            schema_version: PROJECT_SCHEMA_VERSION,
            revision,
            saved_at_ms: unix_millis(),
            project,
        };
        let bytes = serde_json::to_vec_pretty(&document).map_err(|error| error.to_string())?;

        write_synced(&next_path, &bytes)?;
        replace_recoverably(&next_path, &manifest_path, &backup_path)?;

        Ok(ProjectSaveResult {
            revision,
            manifest_path: manifest_path.to_string_lossy().to_string(),
        })
    }

    pub fn load(&self, project_id: &str) -> Option<ProjectDocument> {
        let project_dir = self.project_dir(project_id);
        let manifest_path = project_dir.join("manifest.json");
        let backup_path = project_dir.join("manifest.json.bak");

        read_document(&manifest_path).or_else(|| read_document(&backup_path))
    }

    pub fn save_agent_conversation(
        &self,
        project_id: &str,
        conversation: &Value,
    ) -> Result<(), String> {
        let project_dir = self.project_dir(project_id);
        fs::create_dir_all(&project_dir).map_err(|error| error.to_string())?;
        let path = project_dir.join("agent-conversation.json");
        let backup_path = project_dir.join("agent-conversation.json.bak");
        let next_path = project_dir.join("agent-conversation.json.next");
        let bytes = serde_json::to_vec_pretty(conversation).map_err(|error| error.to_string())?;

        write_synced(&next_path, &bytes)?;
        replace_recoverably(&next_path, &path, &backup_path)
    }

    pub fn load_agent_conversation(&self, project_id: &str) -> Option<Value> {
        let project_dir = self.project_dir(project_id);
        let path = project_dir.join("agent-conversation.json");
        let backup_path = project_dir.join("agent-conversation.json.bak");

        read_json_value(&path).or_else(|| read_json_value(&backup_path))
    }

    pub fn clear_agent_conversation(&self, project_id: &str) -> Result<(), String> {
        let project_dir = self.project_dir(project_id);

        for file_name in [
            "agent-conversation.json",
            "agent-conversation.json.bak",
            "agent-conversation.json.next",
        ] {
            let path = project_dir.join(file_name);

            match fs::remove_file(path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.to_string()),
            }
        }

        Ok(())
    }
}

fn read_document(path: &Path) -> Option<ProjectDocument> {
    let bytes = fs::read(path).ok()?;
    let document = serde_json::from_slice::<ProjectDocument>(&bytes).ok()?;

    if document.schema_version > PROJECT_SCHEMA_VERSION {
        return None;
    }

    Some(document)
}

fn read_json_value(path: &Path) -> Option<Value> {
    let bytes = fs::read(path).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn write_synced(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = File::create(path).map_err(|error| error.to_string())?;
    file.write_all(bytes).map_err(|error| error.to_string())?;
    file.sync_all().map_err(|error| error.to_string())
}

fn replace_recoverably(next: &Path, current: &Path, backup: &Path) -> Result<(), String> {
    if backup.exists() {
        fs::remove_file(backup).map_err(|error| error.to_string())?;
    }

    if current.exists() {
        fs::rename(current, backup).map_err(|error| error.to_string())?;
    }

    if let Err(error) = fs::rename(next, current) {
        if backup.exists() && !current.exists() {
            let _ = fs::rename(backup, current);
        }

        return Err(error.to_string());
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temporary_root() -> PathBuf {
        std::env::temp_dir().join(format!("linglux-media-core-test-{}", unix_millis()))
    }

    #[test]
    fn saves_and_recovers_project_revisions() {
        let root = temporary_root();
        let store = ProjectStore::new(&root).expect("create store");
        let first = store
            .save_json("project:one", serde_json::json!({ "name": "One" }))
            .expect("save first");
        let second = store
            .save_json("project:one", serde_json::json!({ "name": "Two" }))
            .expect("save second");

        assert_eq!(first.revision, 1);
        assert_eq!(second.revision, 2);
        assert_eq!(
            store.load("project:one").expect("load").project["name"],
            "Two"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn saves_recovers_and_clears_agent_conversation() {
        let temp_dir = std::env::temp_dir().join(format!(
            "linglux-project-store-agent-{}",
            crate::unix_millis()
        ));
        let store = ProjectStore::new(temp_dir.clone()).expect("store");
        let first = serde_json::json!({ "projectId": "project:agent", "messages": [{ "content": "first" }] });
        let second = serde_json::json!({ "projectId": "project:agent", "messages": [{ "content": "second" }] });

        store
            .save_agent_conversation("project:agent", &first)
            .expect("save first conversation");
        store
            .save_agent_conversation("project:agent", &second)
            .expect("save second conversation");
        assert_eq!(
            store
                .load_agent_conversation("project:agent")
                .expect("load conversation")["messages"][0]["content"],
            "second"
        );

        let active = store
            .project_dir("project:agent")
            .join("agent-conversation.json");
        fs::write(&active, b"invalid").expect("corrupt active conversation");
        assert_eq!(
            store
                .load_agent_conversation("project:agent")
                .expect("recover backup")["messages"][0]["content"],
            "first"
        );

        store
            .clear_agent_conversation("project:agent")
            .expect("clear conversation");
        assert!(store.load_agent_conversation("project:agent").is_none());
        let _ = fs::remove_dir_all(temp_dir);
    }
}
