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
const USERNAME_PREFIX: &str = "auth";

// Windows Credential Manager limit per entry
const MAX_CREDENTIAL_LENGTH: usize = 2560;

// Token field names for separate keyring entries
const TOKEN_FIELDS: [&str; 5] = [
    "access_token",
    "refresh_token",
    "id_token",
    "expires_at",
    "session_state",
];

/// Validate that a token field doesn't exceed Windows Credential Manager limit
fn validate_token_length(field_name: &str, value: &str) -> Result<(), String> {
    let length = value.len();
    if length > MAX_CREDENTIAL_LENGTH {
        log::error!(
            "Token field '{}' exceeds Windows Credential Manager limit: {} chars (limit: {} chars)",
            field_name,
            length,
            MAX_CREDENTIAL_LENGTH
        );
        return Err(format!(
            "Token field '{}' is too long ({} chars). Maximum allowed: {} chars. \
             This is a platform limitation. Please contact support.",
            field_name, length, MAX_CREDENTIAL_LENGTH
        ));
    }
    log::debug!("Token field '{}' length validated: {} chars (within {} limit)",
                field_name, length, MAX_CREDENTIAL_LENGTH);
    Ok(())
}

/// Store authentication tokens securely in the OS keyring (split into separate entries)
pub fn store_tokens(tokens: &AuthTokens) -> Result<(), String> {
    log::debug!("Attempting to store authentication tokens in keyring (split storage mode)");

    // Prepare token fields as string values
    let token_values = [
        ("access_token", tokens.access_token.as_str()),
        ("refresh_token", tokens.refresh_token.as_str()),
        ("id_token", tokens.id_token.as_str()),
        ("expires_at", &tokens.expires_at.to_string()),
        ("session_state", tokens.session_state.as_str()),
    ];

    // First, validate all token lengths before storing anything
    for (field_name, value) in &token_values {
        validate_token_length(field_name, value)?;
    }

    // All validations passed, now store each token in a separate keyring entry
    for (field_name, value) in &token_values {
        let username = format!("{}.{}", USERNAME_PREFIX, field_name);
        let entry = Entry::new(SERVICE_NAME, &username)
            .map_err(|e| {
                log::error!("Keyring entry creation failed for '{}': {}", field_name, e);
                format!("Cannot access system keyring for '{}'. Please check your system permissions.", field_name)
            })?;

        entry.set_password(value)
            .map_err(|e| {
                log::error!("Failed to store '{}' in keyring: {}", field_name, e);
                format!("Cannot save '{}' credential. Please check your system permissions.", field_name)
            })?;

        log::debug!("Token field '{}' stored successfully ({} chars)", field_name, value.len());
    }

    log::info!("All authentication tokens stored successfully in {} separate keyring entries", token_values.len());
    Ok(())
}

/// Load authentication tokens from the OS keyring (from separate entries)
pub fn load_tokens() -> Result<AuthTokens, String> {
    log::debug!("Attempting to load authentication tokens from keyring (split storage mode)");

    // Load each token field from its separate keyring entry
    let mut token_fields = std::collections::HashMap::new();

    for field_name in &TOKEN_FIELDS {
        let username = format!("{}.{}", USERNAME_PREFIX, field_name);
        let entry = Entry::new(SERVICE_NAME, &username)
            .map_err(|e| {
                log::error!("Keyring entry creation failed for '{}': {}", field_name, e);
                "Cannot access system keyring. Please check your system permissions.".to_string()
            })?;

        let value = entry.get_password()
            .map_err(|e| {
                log::warn!("Failed to load '{}' from keyring: {}", field_name, e);
                format!("Not authenticated. Missing token field '{}'. Please log in first.", field_name)
            })?;

        log::debug!("Token field '{}' loaded successfully ({} chars)", field_name, value.len());
        token_fields.insert(*field_name, value);
    }

    // Reconstruct AuthTokens from individual fields
    let expires_at = token_fields.get("expires_at")
        .ok_or_else(|| "Missing expires_at field".to_string())?
        .parse::<i64>()
        .map_err(|e| {
            log::error!("Failed to parse expires_at as i64: {}", e);
            "Login credentials are corrupted. Please log in again.".to_string()
        })?;

    let tokens = AuthTokens {
        access_token: token_fields.remove("access_token")
            .ok_or_else(|| "Missing access_token field".to_string())?,
        refresh_token: token_fields.remove("refresh_token")
            .ok_or_else(|| "Missing refresh_token field".to_string())?,
        id_token: token_fields.remove("id_token")
            .ok_or_else(|| "Missing id_token field".to_string())?,
        expires_at,
        session_state: token_fields.remove("session_state")
            .ok_or_else(|| "Missing session_state field".to_string())?,
    };

    log::info!("All authentication tokens loaded successfully from {} separate keyring entries", TOKEN_FIELDS.len());
    Ok(tokens)
}

/// Delete authentication tokens from the OS keyring (all separate entries)
pub fn delete_tokens() -> Result<(), String> {
    log::debug!("Attempting to delete authentication tokens from keyring (split storage mode)");

    let mut errors = Vec::new();
    let mut deleted_count = 0;

    // Delete each token field from its separate keyring entry
    for field_name in &TOKEN_FIELDS {
        let username = format!("{}.{}", USERNAME_PREFIX, field_name);
        let entry = Entry::new(SERVICE_NAME, &username)
            .map_err(|e| {
                log::error!("Keyring entry creation failed for '{}': {}", field_name, e);
                format!("Cannot access system keyring for '{}': {}", field_name, e)
            });

        match entry {
            Ok(entry) => {
                match entry.delete_password() {
                    Ok(_) => {
                        log::debug!("Token field '{}' deleted successfully", field_name);
                        deleted_count += 1;
                    }
                    Err(e) => {
                        log::warn!("Failed to delete '{}' from keyring: {} (may not exist)", field_name, e);
                        // Don't treat as error if entry doesn't exist
                    }
                }
            }
            Err(e) => {
                errors.push(e);
            }
        }
    }

    if !errors.is_empty() {
        log::error!("Errors occurred while deleting token entries: {:?}", errors);
        return Err(format!("Failed to clear some login credentials: {}", errors.join(", ")));
    }

    log::info!("Authentication tokens deleted successfully ({} entries cleared)", deleted_count);
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
