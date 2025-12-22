// Authentication module - OAuth via webview

use tauri::{AppHandle, WebviewWindowBuilder, WebviewUrl, Wry};
use tauri::webview::PageLoadEvent;
use std::sync::{Mutex, mpsc};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use crate::models::AuthTokens;
use keyring::Entry;

// Global state to store tokens during extraction
static TOKEN_CHANNEL: Mutex<Option<mpsc::Sender<String>>> = Mutex::new(None);

// Constants for keyring service identification
const SERVICE_NAME: &str = "educartable-downloader";
const USERNAME: &str = "auth_tokens";

/// Store authentication tokens securely in the OS keyring
pub fn store_tokens(tokens: &AuthTokens) -> Result<(), String> {
    log::debug!("Attempting to store authentication tokens in keyring");

    let entry = Entry::new(SERVICE_NAME, USERNAME)
        .map_err(|e| {
            log::error!("Keyring entry creation failed: {}", e);
            "Cannot access system keyring. Please check your system permissions.".to_string()
        })?;

    let tokens_json = serde_json::to_string(tokens)
        .map_err(|e| {
            log::error!("Token serialization failed: {}", e);
            "Failed to save login information. Please try again.".to_string()
        })?;

    entry.set_password(&tokens_json)
        .map_err(|e| {
            log::error!("Failed to store tokens in keyring: {}", e);
            "Cannot save login credentials. Please check your system permissions.".to_string()
        })?;

    log::info!("Authentication tokens stored successfully");
    Ok(())
}

/// Load authentication tokens from the OS keyring
pub fn load_tokens() -> Result<AuthTokens, String> {
    log::debug!("Attempting to load authentication tokens from keyring");

    let entry = Entry::new(SERVICE_NAME, USERNAME)
        .map_err(|e| {
            log::error!("Keyring entry creation failed: {}", e);
            "Cannot access system keyring. Please check your system permissions.".to_string()
        })?;

    let tokens_json = entry.get_password()
        .map_err(|e| {
            log::warn!("Failed to load tokens from keyring: {}", e);
            "Not authenticated. Please log in first.".to_string()
        })?;

    let tokens: AuthTokens = serde_json::from_str(&tokens_json)
        .map_err(|e| {
            log::error!("Token deserialization failed: {}", e);
            "Login credentials are corrupted. Please log in again.".to_string()
        })?;

    log::debug!("Authentication tokens loaded successfully");
    Ok(tokens)
}

/// Delete authentication tokens from the OS keyring
pub fn delete_tokens() -> Result<(), String> {
    log::debug!("Attempting to delete authentication tokens from keyring");

    let entry = Entry::new(SERVICE_NAME, USERNAME)
        .map_err(|e| {
            log::error!("Keyring entry creation failed: {}", e);
            "Cannot access system keyring. Please check your system permissions.".to_string()
        })?;

    entry.delete_password()
        .map_err(|e| {
            log::error!("Failed to delete tokens from keyring: {}", e);
            "Failed to clear login credentials. You may need to log in again manually.".to_string()
        })?;

    log::info!("Authentication tokens deleted successfully");
    Ok(())
}

#[tauri::command]
pub fn submit_tokens(tokens_json: String) -> Result<(), String> {
    log::debug!("Received token submission from webview");

    if let Ok(guard) = TOKEN_CHANNEL.lock() {
        if let Some(tx) = guard.as_ref() {
            tx.send(tokens_json).map_err(|e| {
                log::error!("Failed to send tokens through channel: {}", e);
                e.to_string()
            })?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn authenticate(app_handle: AppHandle<Wry>) -> Result<AuthTokens, String> {
    log::info!("Starting authentication flow");

    // Create a synchronous channel for communication between the webview event handler and this function
    let (tx, rx) = mpsc::channel();

    // Create a channel for token data
    let (token_tx, token_rx) = mpsc::channel();

    // Store the token sender in global state
    {
        let mut guard = TOKEN_CHANNEL.lock().unwrap();
        *guard = Some(token_tx);
    }

    log::debug!("Creating authentication webview window");
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
                log::info!("OAuth login detected, callback URL received");
                // Send the URL through the channel (synchronous send)
                let _ = tx.send(url);
            }
        }
    })
    .build()
    .map_err(|e| {
        log::error!("Failed to create authentication webview: {}", e);
        e.to_string()
    })?;

    // Spawn a blocking task to wait for the login with timeout
    log::debug!("Waiting for user to complete login (300s timeout)");
    let _url = tokio::task::spawn_blocking(move || {
        rx.recv_timeout(Duration::from_secs(300))
            .map_err(|_| {
                log::error!("Login timeout: no response within 300 seconds");
                "Login timeout: no response within 300 seconds".to_string()
            })
    })
    .await
    .map_err(|e| {
        log::error!("Task error during login wait: {}", e);
        format!("Task error: {}", e)
    })??;

    // Wait for the OIDC client library to complete the token exchange
    log::debug!("Waiting for OIDC token exchange to complete");
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

    log::debug!("Injecting JavaScript to extract tokens from localStorage");
    webview.eval(js_code)
        .map_err(|e| {
            log::error!("Failed to inject JavaScript: {}", e);
            "Login window error. Please try again.".to_string()
        })?;

    // Wait for the tokens with timeout
    log::debug!("Waiting for tokens from webview (5s timeout)");
    let tokens_str: String = tokio::task::spawn_blocking(move || {
        token_rx.recv_timeout(Duration::from_secs(5))
            .map_err(|_| {
                log::error!("Timeout waiting for tokens from webview");
                "Login completion timeout. Please try again.".to_string()
            })
    })
    .await
    .map_err(|e| {
        log::error!("Task error during token extraction: {}", e);
        "Internal error during login. Please try again.".to_string()
    })??;

    // Clear the global state
    {
        let mut guard = TOKEN_CHANNEL.lock().unwrap();
        *guard = None;
    }

    // Close the webview window
    log::debug!("Closing authentication webview");
    let _ = webview.close();

    // Check if tokens were found
    if tokens_str == "null" {
        log::error!("Tokens not found in localStorage");
        return Err("Login failed. Please check your credentials and try again.".to_string());
    }

    // Parse the JSON string into AuthTokens
    log::debug!("Parsing extracted tokens");
    let tokens: AuthTokens = serde_json::from_str(&tokens_str)
        .map_err(|e| {
            log::error!("Failed to parse tokens from JSON: {}", e);
            "Login data error. Please try again.".to_string()
        })?;

    // Store tokens securely in the OS keyring
    store_tokens(&tokens)?;

    log::info!("Authentication completed successfully");
    Ok(tokens)
}

#[tauri::command]
pub async fn logout() -> Result<(), String> {
    log::info!("User logout requested");
    delete_tokens()?;
    log::info!("User logged out successfully");
    Ok(())
}

#[tauri::command]
pub async fn is_authenticated() -> Result<bool, String> {
    log::debug!("Checking authentication status");

    match load_tokens() {
        Ok(tokens) => {
            // Check if token is expired
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;

            let is_valid = tokens.expires_at > now;
            log::debug!("Authentication status: {}", if is_valid { "valid" } else { "expired" });
            Ok(is_valid)
        }
        Err(_) => {
            log::debug!("Authentication status: not authenticated");
            Ok(false)
        }
    }
}
