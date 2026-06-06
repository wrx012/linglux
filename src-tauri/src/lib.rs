use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::{AppHandle, Manager};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiKeySettings {
    provider: String,
    base_url: String,
    api_key: String,
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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            create_video_plan,
            load_api_key_settings,
            save_api_key_settings,
            clear_api_key_settings
        ])
        .run(tauri::generate_context!())
        .expect("error while running Linglux");
}
