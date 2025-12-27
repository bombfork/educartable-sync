mod auth;
mod api;
mod sync;
mod config;
mod models;
mod updater;

use tauri::Manager;

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

/// Open the logs directory in the system file explorer
#[tauri::command]
async fn open_logs_directory(app: tauri::AppHandle) -> Result<(), String> {
    let log_dir = app.path().app_log_dir()
        .map_err(|e| format!("Failed to get logs directory: {}", e))?;

    log::info!("Opening logs directory: {:?}", log_dir);

    // Ensure the directory exists
    if !log_dir.exists() {
        std::fs::create_dir_all(&log_dir)
            .map_err(|e| format!("Failed to create logs directory: {}", e))?;
    }

    // Open the directory in the system file explorer
    tauri_plugin_opener::open_path(log_dir.to_str().unwrap(), None::<&str>)
        .map_err(|e| format!("Failed to open logs directory: {}", e))?;

    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Force WebKit to use system graphics libraries
    // This prevents EGL/OpenGL version mismatches in AppImage
    #[cfg(target_os = "linux")]
    {
        std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
    }

    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .targets([
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Stdout),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::LogDir { file_name: None }),
                    tauri_plugin_log::Target::new(tauri_plugin_log::TargetKind::Webview),
                ])
                .level(if cfg!(debug_assertions) {
                    log::LevelFilter::Debug
                } else {
                    log::LevelFilter::Info
                })
                .max_file_size(50_000 /* 50 KB */)
                .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepAll)
                .build(),
        )
        .setup(|app| {
            log::info!("Starting Educartable Sync application");
            log::info!("Version: {}", app.package_info().version);
            log::info!("App identifier: {}", app.config().identifier);
            Ok(())
        })
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![
            greet,
            auth::authenticate,
            auth::submit_tokens,
            auth::logout,
            auth::is_authenticated,
            config::load_config,
            config::save_config,
            config::select_sync_directory,
            sync::start_sync,
            open_logs_directory,
            updater::check_for_updates,
            updater::download_and_install_update
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greet() {
        let result = greet("World");
        assert_eq!(result, "Hello, World! You've been greeted from Rust!");
    }

    #[test]
    fn test_greet_with_empty_string() {
        let result = greet("");
        assert_eq!(result, "Hello, ! You've been greeted from Rust!");
    }

    #[test]
    fn test_greet_with_special_characters() {
        let result = greet("Rust 🦀");
        assert_eq!(result, "Hello, Rust 🦀! You've been greeted from Rust!");
    }
}
