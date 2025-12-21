// Authentication module - OAuth via webview

use tauri::{AppHandle, WebviewWindowBuilder, WebviewUrl};
use tauri::webview::PageLoadEvent;
use std::sync::mpsc;
use std::time::Duration;

#[tauri::command]
pub async fn authenticate(app_handle: AppHandle) -> Result<String, String> {
    // Create a synchronous channel for communication between the webview event handler and this function
    let (tx, rx) = mpsc::channel();

    let webview = WebviewWindowBuilder::new(
        &app_handle,
        "auth",
        WebviewUrl::External("https://app.educartable.com".parse().unwrap())
    )
    .title("Login to Educartable")
    .inner_size(800.0, 700.0)
    .on_page_load(move |_window, payload| {
        // Only process when page load finishes (not when it starts)
        if let PageLoadEvent::Finished = payload.event() {
            let url = payload.url().to_string();

            // Check if the URL indicates successful login
            // After OAuth login, Keycloak redirects with 'code' query parameter
            if url.contains("code=") && (url.contains("/home") || url.contains("/activities")) {
                // Send the URL through the channel (synchronous send)
                let _ = tx.send(url);
            }
        }
    })
    .build()
    .map_err(|e| e.to_string())?;

    // Spawn a blocking task to wait for the login with timeout
    let result = tokio::task::spawn_blocking(move || {
        rx.recv_timeout(Duration::from_secs(300))
            .map_err(|_| "Login timeout: no response within 300 seconds".to_string())
    })
    .await
    .map_err(|e| format!("Task error: {}", e))?;

    // Close the webview window
    let _ = webview.close();

    result
}
