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

// Windows Credential Manager limit per entry: 2560 BYTES (in UTF-16)
//
// CRITICAL: Windows stores credentials as UTF-16, while we measure in UTF-8 bytes.
// For ASCII/base64 tokens (typical for JWTs):
//   - 1 UTF-8 byte = 1 character = 2 UTF-16 bytes
//   - Example: 1849 UTF-8 bytes → 3698 UTF-16 bytes > 2560 limit ❌
//
// Safe chunk size calculation:
//   - Windows limit: 2560 UTF-16 bytes
//   - Divide by 2: 1280 UTF-8 bytes (for pure ASCII)
//   - Safety margin (20%): 1024 UTF-8 bytes
//   - Final value: 1000 UTF-8 bytes → 2000 UTF-16 bytes < 2560 ✓
const SAFE_CHUNK_SIZE_BYTES: usize = 1000;

// Token field names for separate keyring entries
const TOKEN_FIELDS: [&str; 5] = [
    "access_token",
    "refresh_token",
    "id_token",
    "expires_at",
    "session_state",
];

/// Store a token field in keyring, splitting into chunks if necessary
fn store_token_field(field_name: &str, value: &str) -> Result<(), String> {
    let value_bytes = value.len(); // UTF-8 byte length
    let value_chars = value.chars().count(); // Character count

    log::info!(
        "Storing token field '{}' (length: {} chars, {} bytes)",
        field_name, value_chars, value_bytes
    );

    if value_bytes <= SAFE_CHUNK_SIZE_BYTES {
        // Field fits in single entry, store directly
        let username = format!("{}.{}", USERNAME_PREFIX, field_name);
        let entry = Entry::new(SERVICE_NAME, &username)
            .map_err(|e| {
                log::error!("Keyring entry creation failed for '{}': {}", field_name, e);
                format!("Cannot access system keyring for '{}'. Please check your system permissions.", field_name)
            })?;

        entry.set_password(value)
            .map_err(|e| {
                log::error!(
                    "Failed to store '{}' ({} chars, {} bytes) in keyring: {}",
                    field_name, value_chars, value_bytes, e
                );
                format!(
                    "Cannot save '{}' credential ({} chars, {} bytes). Error: {}",
                    field_name, value_chars, value_bytes, e
                )
            })?;

        log::debug!(
            "Token field '{}' stored successfully ({} chars, {} bytes, single entry)",
            field_name, value_chars, value_bytes
        );
    } else {
        // Field needs to be chunked based on byte size
        let mut chunks = Vec::new();
        let mut current_byte_pos = 0;

        while current_byte_pos < value_bytes {
            let remaining = &value[current_byte_pos..];
            let remaining_bytes = remaining.len();

            let chunk_end_bytes = if remaining_bytes <= SAFE_CHUNK_SIZE_BYTES {
                remaining_bytes
            } else {
                // Find a valid UTF-8 character boundary at or before SAFE_CHUNK_SIZE_BYTES
                let mut byte_pos = SAFE_CHUNK_SIZE_BYTES.min(remaining_bytes);

                // Walk backwards to find a valid UTF-8 boundary
                while byte_pos > 0 && !remaining.is_char_boundary(byte_pos) {
                    byte_pos -= 1;
                }

                if byte_pos == 0 {
                    // Shouldn't happen with reasonable input, but fall back to a safe value
                    log::warn!("Could not find UTF-8 boundary, using fallback");
                    SAFE_CHUNK_SIZE_BYTES / 2
                } else {
                    byte_pos
                }
            };

            chunks.push(&remaining[..chunk_end_bytes]);
            current_byte_pos += chunk_end_bytes;
        }

        let chunk_count = chunks.len();

        log::debug!(
            "Token field '{}' is {} bytes ({} chars), splitting into {} chunks of max {} bytes each",
            field_name, value_bytes, value_chars, chunk_count, SAFE_CHUNK_SIZE_BYTES
        );

        // Store metadata entry with chunk count
        let username = format!("{}.{}", USERNAME_PREFIX, field_name);
        let entry = Entry::new(SERVICE_NAME, &username)
            .map_err(|e| {
                log::error!("Keyring metadata entry creation failed for '{}': {}", field_name, e);
                format!("Cannot access system keyring for '{}'.", field_name)
            })?;

        entry.set_password(&chunk_count.to_string())
            .map_err(|e| {
                log::error!("Failed to store '{}' metadata in keyring: {}", field_name, e);
                format!("Cannot save '{}' metadata.", field_name)
            })?;

        log::debug!("Token field '{}' metadata stored: {} chunks", field_name, chunk_count);

        // Store each chunk
        for (index, chunk) in chunks.iter().enumerate() {
            let chunk_index = index + 1; // 1-based indexing
            let chunk_bytes = chunk.len();
            let chunk_chars = chunk.chars().count();

            let username = format!("{}.{}_{}", USERNAME_PREFIX, field_name, chunk_index);
            let entry = Entry::new(SERVICE_NAME, &username)
                .map_err(|e| {
                    log::error!("Keyring entry creation failed for '{}' chunk {}: {}", field_name, chunk_index, e);
                    format!("Cannot access system keyring for '{}' chunk {}.", field_name, chunk_index)
                })?;

            entry.set_password(chunk)
                .map_err(|e| {
                    log::error!(
                        "Failed to store '{}' chunk {} ({} chars, {} bytes) in keyring: {}",
                        field_name, chunk_index, chunk_chars, chunk_bytes, e
                    );
                    format!(
                        "Cannot save '{}' chunk {} ({} chars, {} bytes). Error: {}",
                        field_name, chunk_index, chunk_chars, chunk_bytes, e
                    )
                })?;

            log::debug!(
                "Token field '{}' chunk {}/{} stored successfully ({} chars, {} bytes)",
                field_name, chunk_index, chunk_count, chunk_chars, chunk_bytes
            );
        }

        log::info!(
            "Token field '{}' stored successfully ({} chars, {} bytes in {} chunks)",
            field_name, value_chars, value_bytes, chunk_count
        );
    }

    Ok(())
}

/// Load a token field from keyring, reassembling chunks if necessary
fn load_token_field(field_name: &str) -> Result<String, String> {
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

    // Check if this is metadata (chunk count) or actual data
    if let Ok(chunk_count) = value.parse::<usize>() {
        // This is chunked data, load all chunks
        log::debug!("Token field '{}' is chunked, loading {} chunks", field_name, chunk_count);

        let mut chunks = Vec::with_capacity(chunk_count);

        for chunk_index in 1..=chunk_count {
            let username = format!("{}.{}_{}", USERNAME_PREFIX, field_name, chunk_index);
            let entry = Entry::new(SERVICE_NAME, &username)
                .map_err(|e| {
                    log::error!("Keyring entry creation failed for '{}' chunk {}: {}", field_name, chunk_index, e);
                    "Cannot access system keyring.".to_string()
                })?;

            let chunk = entry.get_password()
                .map_err(|e| {
                    log::error!("Failed to load '{}' chunk {} from keyring: {}", field_name, chunk_index, e);
                    format!("Missing '{}' chunk {}. Please log in again.", field_name, chunk_index)
                })?;

            log::debug!("Token field '{}' chunk {}/{} loaded ({} chars)",
                       field_name, chunk_index, chunk_count, chunk.len());
            chunks.push(chunk);
        }

        let reassembled = chunks.join("");
        log::info!("Token field '{}' loaded successfully ({} chars from {} chunks)",
                   field_name, reassembled.len(), chunk_count);
        Ok(reassembled)
    } else {
        // Single entry, return as-is
        log::debug!("Token field '{}' loaded successfully ({} chars, single entry)", field_name, value.len());
        Ok(value)
    }
}

/// Delete a token field from keyring, including all chunks if present
fn delete_token_field(field_name: &str) -> Result<(), String> {
    let username = format!("{}.{}", USERNAME_PREFIX, field_name);
    let entry = Entry::new(SERVICE_NAME, &username)
        .map_err(|e| {
            log::error!("Keyring entry creation failed for '{}': {}", field_name, e);
            format!("Cannot access system keyring for '{}'.", field_name)
        })?;

    // Try to read the entry to check if it's chunked
    if let Ok(value) = entry.get_password() {
        if let Ok(chunk_count) = value.parse::<usize>() {
            // This is chunked data, delete all chunks
            log::debug!("Token field '{}' is chunked, deleting {} chunks", field_name, chunk_count);

            for chunk_index in 1..=chunk_count {
                let username = format!("{}.{}_{}", USERNAME_PREFIX, field_name, chunk_index);
                if let Ok(chunk_entry) = Entry::new(SERVICE_NAME, &username) {
                    match chunk_entry.delete_password() {
                        Ok(_) => log::debug!("Token field '{}' chunk {} deleted", field_name, chunk_index),
                        Err(e) => log::warn!("Failed to delete '{}' chunk {}: {} (may not exist)",
                                            field_name, chunk_index, e),
                    }
                }
            }
        }
    }

    // Delete the main entry (either metadata or single value)
    match entry.delete_password() {
        Ok(_) => {
            log::debug!("Token field '{}' deleted successfully", field_name);
            Ok(())
        }
        Err(e) => {
            log::warn!("Failed to delete '{}' from keyring: {} (may not exist)", field_name, e);
            Ok(()) // Don't treat as error if entry doesn't exist
        }
    }
}

/// Store authentication tokens securely in the OS keyring (split into separate entries, chunked if needed)
pub fn store_tokens(tokens: &AuthTokens) -> Result<(), String> {
    log::info!("Attempting to store authentication tokens in keyring (split storage mode with chunking)");

    // Prepare token fields as string values
    let token_values = [
        ("access_token", tokens.access_token.as_str()),
        ("refresh_token", tokens.refresh_token.as_str()),
        ("id_token", tokens.id_token.as_str()),
        ("expires_at", &tokens.expires_at.to_string()),
        ("session_state", tokens.session_state.as_str()),
    ];

    // Store each token field (will be automatically chunked if needed)
    for (field_name, value) in &token_values {
        store_token_field(field_name, value)?;
    }

    log::info!("All authentication tokens stored successfully");
    Ok(())
}

/// Load authentication tokens from the OS keyring (from separate entries, reassembling chunks if needed)
pub fn load_tokens() -> Result<AuthTokens, String> {
    log::info!("Attempting to load authentication tokens from keyring (split storage mode with chunking)");

    // Load each token field (will be automatically reassembled if chunked)
    let access_token = load_token_field("access_token")?;
    let refresh_token = load_token_field("refresh_token")?;
    let id_token = load_token_field("id_token")?;
    let expires_at_str = load_token_field("expires_at")?;
    let session_state = load_token_field("session_state")?;

    // Parse expires_at
    let expires_at = expires_at_str.parse::<i64>()
        .map_err(|e| {
            log::error!("Failed to parse expires_at as i64: {}", e);
            "Login credentials are corrupted. Please log in again.".to_string()
        })?;

    let tokens = AuthTokens {
        access_token,
        refresh_token,
        id_token,
        expires_at,
        session_state,
    };

    log::info!("All authentication tokens loaded successfully");
    Ok(tokens)
}

/// Delete authentication tokens from the OS keyring (all separate entries, including chunks)
pub fn delete_tokens() -> Result<(), String> {
    log::info!("Attempting to delete authentication tokens from keyring (split storage mode with chunking)");

    let mut errors = Vec::new();

    // Delete each token field (will automatically delete all chunks if present)
    for field_name in &TOKEN_FIELDS {
        if let Err(e) = delete_token_field(field_name) {
            errors.push(format!("{}: {}", field_name, e));
        }
    }

    if !errors.is_empty() {
        log::error!("Errors occurred while deleting token entries: {:?}", errors);
        return Err(format!("Failed to clear some login credentials: {}", errors.join(", ")));
    }

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
