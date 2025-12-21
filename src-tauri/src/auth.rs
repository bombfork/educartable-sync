// Authentication module - OAuth via webview

use tauri::{AppHandle, Manager, WebviewWindowBuilder, WebviewUrl};

#[tauri::command]
pub async fn authenticate(app_handle: AppHandle) -> Result<String, String> {
    let _webview = WebviewWindowBuilder::new(
        &app_handle,
        "auth",
        WebviewUrl::External("https://app.educartable.com".parse().unwrap())
    )
    .title("Login to Educartable")
    .inner_size(800.0, 700.0)
    .build()
    .map_err(|e| e.to_string())?;

    Ok("Webview opened".to_string())
}
