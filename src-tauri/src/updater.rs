use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_plugin_updater::UpdaterExt;

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateInfo {
    pub available: bool,
    pub current_version: String,
    pub latest_version: Option<String>,
    pub body: Option<String>,
    pub date: Option<String>,
}

/// Check for available updates
#[tauri::command]
pub async fn check_for_updates(app: AppHandle) -> Result<UpdateInfo, String> {
    log::info!("Checking for updates...");

    let current_version = app.package_info().version.to_string();
    log::info!("Current version: {}", current_version);

    match app.updater_builder().build() {
        Ok(updater) => match updater.check().await {
            Ok(update_response) => {
                if let Some(update) = update_response {
                    log::info!("Update available: version {}", update.version);
                    Ok(UpdateInfo {
                        available: true,
                        current_version: current_version.clone(),
                        latest_version: Some(update.version.clone()),
                        body: update.body.clone(),
                        date: update.date.map(|d| d.to_string()),
                    })
                } else {
                    log::info!("No update available");
                    Ok(UpdateInfo {
                        available: false,
                        current_version,
                        latest_version: None,
                        body: None,
                        date: None,
                    })
                }
            }
            Err(e) => {
                log::error!("Failed to check for updates: {:?}", e);
                Err(format!("Failed to check for updates: {}", e))
            }
        },
        Err(e) => {
            log::error!("Failed to build updater: {:?}", e);
            Err(format!("Failed to build updater: {}", e))
        }
    }
}

/// Download and install an update
#[tauri::command]
pub async fn download_and_install_update(app: AppHandle) -> Result<(), String> {
    log::info!("Starting update download and installation...");

    match app.updater_builder().build() {
        Ok(updater) => {
            match updater.check().await {
                Ok(update_response) => {
                    if let Some(update) = update_response {
                        log::info!("Downloading update version {}", update.version);

                        match update
                            .download_and_install(
                                |_chunk_length, _content_length| {
                                    // Progress callback - could emit events here if needed
                                },
                                || {
                                    // Download finished callback
                                    log::info!("Update download completed");
                                },
                            )
                            .await
                        {
                            Ok(_) => {
                                log::info!("Update installed successfully");
                                Ok(())
                            }
                            Err(e) => {
                                log::error!("Failed to download/install update: {:?}", e);
                                Err(format!("Failed to download/install update: {}", e))
                            }
                        }
                    } else {
                        log::warn!("No update available to install");
                        Err("No update available".to_string())
                    }
                }
                Err(e) => {
                    log::error!("Failed to check for updates during install: {:?}", e);
                    Err(format!("Failed to check for updates: {}", e))
                }
            }
        }
        Err(e) => {
            log::error!("Failed to build updater: {:?}", e);
            Err(format!("Failed to build updater: {}", e))
        }
    }
}
