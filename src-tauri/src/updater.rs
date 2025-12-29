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

/// Checks for available application updates.
///
/// Queries the update server to check if a newer version of the application
/// is available. Returns update information including version number, release
/// notes, and publication date.
///
/// # Arguments
/// * `app` - Tauri application handle
///
/// # Returns
/// - `Ok(UpdateInfo)` - Update check completed with availability status
/// - `Err(String)` - Failed to check for updates
///
/// # Errors
/// - Cannot connect to update server
/// - Update server returned invalid response
#[tauri::command]
pub async fn check_for_updates(app: AppHandle) -> Result<UpdateInfo, String> {
    log::info!("Checking for updates...");

    let current_version = app.package_info().version.to_string();
    log::info!("Current version: {current_version}");

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
                log::error!("Failed to check for updates: {e:?}");
                Err(format!("Failed to check for updates: {e}"))
            }
        },
        Err(e) => {
            log::error!("Failed to build updater: {e:?}");
            Err(format!("Failed to build updater: {e}"))
        }
    }
}

/// Downloads and installs an available application update.
///
/// Checks for updates, and if one is available, downloads and installs it.
/// The application will need to be restarted for the update to take effect.
/// This is a blocking operation that may take several minutes depending on
/// update size and connection speed.
///
/// # Arguments
/// * `app` - Tauri application handle
///
/// # Returns
/// - `Ok(())` - Update downloaded and installed successfully
/// - `Err(String)` - Update failed with error message
///
/// # Errors
/// - No update available
/// - Download or installation failure
/// - Cannot connect to update server
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
                            Ok(()) => {
                                log::info!("Update installed successfully");
                                Ok(())
                            }
                            Err(e) => {
                                log::error!("Failed to download/install update: {e:?}");
                                Err(format!("Failed to download/install update: {e}"))
                            }
                        }
                    } else {
                        log::warn!("No update available to install");
                        Err("No update available".to_string())
                    }
                }
                Err(e) => {
                    log::error!("Failed to check for updates during install: {e:?}");
                    Err(format!("Failed to check for updates: {e}"))
                }
            }
        }
        Err(e) => {
            log::error!("Failed to build updater: {e:?}");
            Err(format!("Failed to build updater: {e}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== Tests for UpdateInfo ==========

    #[test]
    fn test_update_info_serialization_with_update_available() {
        let info = UpdateInfo {
            available: true,
            current_version: "1.0.0".to_string(),
            latest_version: Some("1.1.0".to_string()),
            body: Some("Release notes for version 1.1.0".to_string()),
            date: Some("2024-01-15".to_string()),
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("1.0.0"));
        assert!(json.contains("1.1.0"));
        assert!(json.contains("Release notes"));
        assert!(json.contains("2024-01-15"));
        assert!(json.contains("\"available\":true"));
    }

    #[test]
    fn test_update_info_serialization_no_update() {
        let info = UpdateInfo {
            available: false,
            current_version: "1.0.0".to_string(),
            latest_version: None,
            body: None,
            date: None,
        };

        let json = serde_json::to_string(&info).unwrap();
        assert!(json.contains("\"available\":false"));
        assert!(json.contains("1.0.0"));
        assert!(json.contains("\"latest_version\":null"));
        assert!(json.contains("\"body\":null"));
        assert!(json.contains("\"date\":null"));
    }

    #[test]
    fn test_update_info_deserialization_with_update() {
        let json = r#"{
            "available": true,
            "current_version": "2.0.0",
            "latest_version": "2.1.0",
            "body": "Bug fixes and improvements",
            "date": "2024-06-20"
        }"#;

        let info: UpdateInfo = serde_json::from_str(json).unwrap();
        assert!(info.available);
        assert_eq!(info.current_version, "2.0.0");
        assert_eq!(info.latest_version, Some("2.1.0".to_string()));
        assert_eq!(info.body, Some("Bug fixes and improvements".to_string()));
        assert_eq!(info.date, Some("2024-06-20".to_string()));
    }

    #[test]
    fn test_update_info_deserialization_no_update() {
        let json = r#"{
            "available": false,
            "current_version": "2.0.0",
            "latest_version": null,
            "body": null,
            "date": null
        }"#;

        let info: UpdateInfo = serde_json::from_str(json).unwrap();
        assert!(!info.available);
        assert_eq!(info.current_version, "2.0.0");
        assert!(info.latest_version.is_none());
        assert!(info.body.is_none());
        assert!(info.date.is_none());
    }

    #[test]
    fn test_update_info_partial_fields() {
        // Test with only some optional fields populated
        let info = UpdateInfo {
            available: true,
            current_version: "1.5.0".to_string(),
            latest_version: Some("1.6.0".to_string()),
            body: Some("Minor update".to_string()),
            date: None, // Date might not always be available
        };

        let json = serde_json::to_string(&info).unwrap();
        let deserialized: UpdateInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.available, info.available);
        assert_eq!(deserialized.current_version, info.current_version);
        assert_eq!(deserialized.latest_version, info.latest_version);
        assert_eq!(deserialized.body, info.body);
        assert!(deserialized.date.is_none());
    }

    #[test]
    fn test_update_info_debug_format() {
        let info = UpdateInfo {
            available: true,
            current_version: "3.0.0".to_string(),
            latest_version: Some("3.1.0".to_string()),
            body: Some("New features".to_string()),
            date: Some("2024-12-01".to_string()),
        };

        let debug_output = format!("{:?}", info);
        assert!(debug_output.contains("UpdateInfo"));
        assert!(debug_output.contains("3.0.0"));
        assert!(debug_output.contains("3.1.0"));
    }

    #[test]
    fn test_update_info_version_comparison_logic() {
        // Test that we can distinguish between different update states
        let no_update = UpdateInfo {
            available: false,
            current_version: "1.0.0".to_string(),
            latest_version: None,
            body: None,
            date: None,
        };

        let update_available = UpdateInfo {
            available: true,
            current_version: "1.0.0".to_string(),
            latest_version: Some("1.1.0".to_string()),
            body: Some("Update available".to_string()),
            date: Some("2024-12-29".to_string()),
        };

        assert!(!no_update.available);
        assert!(no_update.latest_version.is_none());

        assert!(update_available.available);
        assert!(update_available.latest_version.is_some());
        assert_ne!(
            &update_available.current_version,
            update_available.latest_version.as_ref().unwrap()
        );
    }

    #[test]
    fn test_update_info_empty_optional_strings() {
        // Test that empty strings are handled correctly (different from None)
        let info = UpdateInfo {
            available: true,
            current_version: "1.0.0".to_string(),
            latest_version: Some("1.1.0".to_string()),
            body: Some("".to_string()), // Empty string, not None
            date: Some("".to_string()),
        };

        let json = serde_json::to_string(&info).unwrap();
        let deserialized: UpdateInfo = serde_json::from_str(&json).unwrap();

        assert!(deserialized.body.is_some());
        assert_eq!(deserialized.body.unwrap(), "");
        assert!(deserialized.date.is_some());
        assert_eq!(deserialized.date.unwrap(), "");
    }

    #[test]
    fn test_update_info_long_release_notes() {
        // Test handling of long release notes
        let long_body = "# Release Notes\n\n".to_string() + &"- Feature ".repeat(100);

        let info = UpdateInfo {
            available: true,
            current_version: "1.0.0".to_string(),
            latest_version: Some("2.0.0".to_string()),
            body: Some(long_body.clone()),
            date: Some("2024-12-29".to_string()),
        };

        let json = serde_json::to_string(&info).unwrap();
        let deserialized: UpdateInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.body, Some(long_body));
    }

    #[test]
    fn test_update_info_special_characters_in_version() {
        // Test version strings with special characters (e.g., semantic versioning)
        let info = UpdateInfo {
            available: true,
            current_version: "1.0.0-beta.1".to_string(),
            latest_version: Some("1.0.0-rc.1".to_string()),
            body: Some("Pre-release update".to_string()),
            date: Some("2024-12-29T10:30:00Z".to_string()),
        };

        let json = serde_json::to_string(&info).unwrap();
        let deserialized: UpdateInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.current_version, "1.0.0-beta.1");
        assert_eq!(deserialized.latest_version, Some("1.0.0-rc.1".to_string()));
    }
}
