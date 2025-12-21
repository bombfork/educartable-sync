// Authentication module - OAuth via webview

use tauri::{AppHandle, WebviewWindowBuilder, WebviewUrl, Wry};
use tauri::webview::PageLoadEvent;
use std::sync::{Mutex, mpsc};
use std::time::Duration;
use crate::models::AuthTokens;

// Global state to store tokens during extraction
static TOKEN_CHANNEL: Mutex<Option<mpsc::Sender<String>>> = Mutex::new(None);

#[tauri::command]
pub fn submit_tokens(tokens_json: String) -> Result<(), String> {
    if let Ok(guard) = TOKEN_CHANNEL.lock() {
        if let Some(tx) = guard.as_ref() {
            tx.send(tokens_json).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn authenticate(app_handle: AppHandle<Wry>) -> Result<AuthTokens, String> {
    // Create a synchronous channel for communication between the webview event handler and this function
    let (tx, rx) = mpsc::channel();

    // Create a channel for token data
    let (token_tx, token_rx) = mpsc::channel();

    // Store the token sender in global state
    {
        let mut guard = TOKEN_CHANNEL.lock().unwrap();
        *guard = Some(token_tx);
    }

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
    let _url = tokio::task::spawn_blocking(move || {
        rx.recv_timeout(Duration::from_secs(300))
            .map_err(|_| "Login timeout: no response within 300 seconds".to_string())
    })
    .await
    .map_err(|e| format!("Task error: {}", e))??;

    // Wait for the OIDC client library to complete the token exchange
    tokio::time::sleep(Duration::from_secs(3)).await;

    // JavaScript to inject for extracting tokens from localStorage
    // Call the Tauri command to send tokens back
    let js_code = r#"
        (function() {
            const key = 'oidc.user:https://accounts.edumoov.com/auth/realms/edumoov:educlasse';
            const data = localStorage.getItem(key);
            if (data) {
                window.__TAURI__.core.invoke('submit_tokens', { tokensJson: data });
            } else {
                window.__TAURI__.core.invoke('submit_tokens', { tokensJson: 'null' });
            }
        })();
    "#;

    webview.eval(js_code)
        .map_err(|e| format!("Failed to inject JavaScript: {}", e))?;

    // Wait for the tokens with timeout
    let tokens_str: String = tokio::task::spawn_blocking(move || {
        token_rx.recv_timeout(Duration::from_secs(5))
            .map_err(|_| "Timeout waiting for tokens".to_string())
    })
    .await
    .map_err(|e| format!("Task error: {}", e))??;

    // Clear the global state
    {
        let mut guard = TOKEN_CHANNEL.lock().unwrap();
        *guard = None;
    }

    // Close the webview window
    let _ = webview.close();

    // Check if tokens were found
    if tokens_str == "null" {
        return Err("Tokens not found in localStorage".to_string());
    }

    // Parse the JSON string into AuthTokens
    let tokens: AuthTokens = serde_json::from_str(&tokens_str)
        .map_err(|e| format!("Failed to parse tokens: {}", e))?;

    Ok(tokens)
}
