// Authentication module - OAuth via webview

use tauri::{AppHandle, WebviewWindowBuilder, WebviewUrl, Wry};
use tauri::webview::PageLoadEvent;
use std::sync::{Mutex, mpsc};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use crate::models::AuthTokens;
use keyring::Entry;
use reqwest::Client;

// Global state to store tokens during extraction
static TOKEN_CHANNEL: Mutex<Option<mpsc::Sender<String>>> = Mutex::new(None);

// Constants for keyring service identification
const SERVICE_NAME: &str = "educartable-downloader";
const USERNAME: &str = "auth_tokens";

/// Store authentication tokens securely in the OS keyring
pub fn store_tokens(tokens: &AuthTokens) -> Result<(), String> {
    let entry = Entry::new(SERVICE_NAME, USERNAME)
        .map_err(|e| format!("Keyring error: {}", e))?;

    let tokens_json = serde_json::to_string(tokens)
        .map_err(|e| format!("Serialization error: {}", e))?;

    entry.set_password(&tokens_json)
        .map_err(|e| format!("Failed to store tokens: {}", e))?;

    Ok(())
}

/// Load authentication tokens from the OS keyring
pub fn load_tokens() -> Result<AuthTokens, String> {
    let entry = Entry::new(SERVICE_NAME, USERNAME)
        .map_err(|e| format!("Keyring error: {}", e))?;

    let tokens_json = entry.get_password()
        .map_err(|e| format!("Failed to load tokens: {}", e))?;

    let tokens: AuthTokens = serde_json::from_str(&tokens_json)
        .map_err(|e| format!("Deserialization error: {}", e))?;

    Ok(tokens)
}

/// Delete authentication tokens from the OS keyring
pub fn delete_tokens() -> Result<(), String> {
    let entry = Entry::new(SERVICE_NAME, USERNAME)
        .map_err(|e| format!("Keyring error: {}", e))?;

    entry.delete_password()
        .map_err(|e| format!("Failed to delete tokens: {}", e))?;

    Ok(())
}

/// Refresh expired access tokens using the refresh_token
pub async fn refresh_tokens(refresh_token: &str) -> Result<AuthTokens, String> {
    let client = Client::new();

    let params = [
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
        ("client_id", "educlasse"),
    ];

    let response = client
        .post("https://accounts.edumoov.com/auth/realms/edumoov/protocol/openid-connect/token")
        .form(&params)
        .send()
        .await
        .map_err(|e| format!("Refresh request failed: {}", e))?;

    if !response.status().is_success() {
        return Err("Token refresh failed".to_string());
    }

    let tokens: AuthTokens = response.json().await
        .map_err(|e| format!("Failed to parse tokens: {}", e))?;

    // Store new tokens
    store_tokens(&tokens)?;

    Ok(tokens)
}

/// Get a valid access token, refreshing if necessary
pub async fn get_valid_token() -> Result<String, String> {
    let tokens = load_tokens()?;

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    if tokens.expires_at > now + 60 {
        // Token is still valid (with 60s buffer)
        Ok(tokens.access_token)
    } else {
        // Token expired, refresh it
        let new_tokens = refresh_tokens(&tokens.refresh_token).await?;
        Ok(new_tokens.access_token)
    }
}

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

    // Store tokens securely in the OS keyring
    store_tokens(&tokens)?;

    Ok(tokens)
}

#[tauri::command]
pub async fn logout() -> Result<(), String> {
    delete_tokens()?;
    Ok(())
}

#[tauri::command]
pub async fn is_authenticated() -> Result<bool, String> {
    match load_tokens() {
        Ok(tokens) => {
            // Check if token is expired
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs() as i64;

            Ok(tokens.expires_at > now)
        }
        Err(_) => Ok(false)
    }
}
