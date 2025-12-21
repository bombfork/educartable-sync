// Configuration management

use crate::models::AppConfig;
use serde_json;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

/// Get the path to the config file
fn get_config_path(app_handle: &AppHandle) -> Result<PathBuf, String> {
    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {}", e))?;

    // Create directory if it doesn't exist
    fs::create_dir_all(&app_data_dir)
        .map_err(|e| format!("Failed to create app data directory: {}", e))?;

    Ok(app_data_dir.join("config.json"))
}

/// Load configuration from disk
#[tauri::command]
pub async fn load_config(app_handle: AppHandle) -> Result<AppConfig, String> {
    let config_path = get_config_path(&app_handle)?;

    // If config file doesn't exist, return default config
    if !config_path.exists() {
        return Ok(AppConfig {
            sync_path: PathBuf::new(),
            include_videos: true,
            organize_by_date: true,
        });
    }

    // Read and parse config file
    let config_json = fs::read_to_string(&config_path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;

    let config: AppConfig = serde_json::from_str(&config_json)
        .map_err(|e| format!("Failed to parse config file: {}", e))?;

    Ok(config)
}

/// Save configuration to disk
#[tauri::command]
pub async fn save_config(app_handle: AppHandle, config: AppConfig) -> Result<(), String> {
    let config_path = get_config_path(&app_handle)?;

    // Serialize config to JSON
    let config_json = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    // Write to file
    fs::write(&config_path, config_json)
        .map_err(|e| format!("Failed to write config file: {}", e))?;

    Ok(())
}

/// Open directory picker and return selected path
#[tauri::command]
pub async fn select_sync_directory(app_handle: AppHandle) -> Result<String, String> {
    let dialog = app_handle.dialog().file();

    // Configure dialog to pick directories
    let file_path = dialog
        .blocking_pick_folder()
        .ok_or("No folder selected")?;

    // Convert FilePath to String
    Ok(file_path.to_string())
}
