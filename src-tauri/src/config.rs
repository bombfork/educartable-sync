// Configuration management

use crate::models::AppConfig;
use serde_json;
use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

/// Get the path to the config file
fn get_config_path(app_handle: &AppHandle) -> Result<PathBuf, String> {
    log::debug!("Getting config file path");

    let app_data_dir = app_handle
        .path()
        .app_data_dir()
        .map_err(|e| {
            log::error!("Failed to get app data directory: {}", e);
            "Cannot access application data directory. Please check permissions.".to_string()
        })?;

    // Create directory if it doesn't exist
    log::debug!("Ensuring app data directory exists: {:?}", app_data_dir);
    fs::create_dir_all(&app_data_dir)
        .map_err(|e| {
            log::error!("Failed to create app data directory: {}", e);
            "Cannot create application data directory. Please check disk space and permissions.".to_string()
        })?;

    let config_path = app_data_dir.join("config.json");
    log::debug!("Config file path: {:?}", config_path);
    Ok(config_path)
}

/// Load configuration from disk
#[tauri::command]
pub async fn load_config(app_handle: AppHandle) -> Result<AppConfig, String> {
    log::info!("Loading configuration");
    let config_path = get_config_path(&app_handle)?;

    // If config file doesn't exist, return default config
    if !config_path.exists() {
        log::info!("Config file does not exist, returning default config");
        return Ok(AppConfig {
            sync_path: PathBuf::new(),
        });
    }

    // Read and parse config file
    log::debug!("Reading config from {:?}", config_path);
    let config_json = fs::read_to_string(&config_path)
        .map_err(|e| {
            log::error!("Failed to read config file: {}", e);
            "Cannot read configuration file. Please check permissions.".to_string()
        })?;

    let config: AppConfig = serde_json::from_str(&config_json)
        .map_err(|e| {
            log::error!("Failed to parse config file: {}", e);
            "Configuration file is corrupted. Settings may be reset.".to_string()
        })?;

    log::info!("Configuration loaded successfully");
    Ok(config)
}

/// Save configuration to disk
#[tauri::command]
pub async fn save_config(app_handle: AppHandle, config: AppConfig) -> Result<(), String> {
    log::info!("Saving configuration");
    let config_path = get_config_path(&app_handle)?;

    // Serialize config to JSON
    let config_json = serde_json::to_string_pretty(&config)
        .map_err(|e| {
            log::error!("Failed to serialize config: {}", e);
            "Cannot prepare configuration for saving. Please try again.".to_string()
        })?;

    // Write to file
    log::debug!("Writing config to {:?}", config_path);
    fs::write(&config_path, config_json)
        .map_err(|e| {
            log::error!("Failed to write config file: {}", e);
            "Cannot save configuration. Please check disk space and permissions.".to_string()
        })?;

    log::info!("Configuration saved successfully");
    Ok(())
}

/// Open directory picker and return selected path
#[tauri::command]
pub async fn select_sync_directory(app_handle: AppHandle) -> Result<String, String> {
    log::info!("Opening directory picker");
    let dialog = app_handle.dialog().file();

    // Configure dialog to pick directories
    let file_path = dialog
        .blocking_pick_folder()
        .ok_or_else(|| {
            log::warn!("No folder selected by user");
            "No folder selected".to_string()
        })?;

    let path_string = file_path.to_string();
    log::info!("Selected sync directory: {}", path_string);

    // Convert FilePath to String
    Ok(path_string)
}
