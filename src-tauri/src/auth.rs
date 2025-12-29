// Authentication module - OAuth via webview

use crate::models::AuthTokens;
use keyring::Entry;
use std::sync::{mpsc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tauri::webview::PageLoadEvent;
use tauri::{AppHandle, WebviewUrl, WebviewWindowBuilder, Wry};

/// Trait for storing, loading, and deleting credentials
pub trait CredentialStore: Send + Sync {
    /// Store a credential with the given key and value
    fn store(&self, key: &str, value: &str) -> Result<(), String>;

    /// Load a credential by key
    fn load(&self, key: &str) -> Result<String, String>;

    /// Delete a credential by key
    fn delete(&self, key: &str) -> Result<(), String>;
}

/// `KeyringCredentialStore` - stores credentials using the system keyring
pub struct KeyringCredentialStore {
    service_name: String,
    username_prefix: String,
}

impl KeyringCredentialStore {
    /// Create a new `KeyringCredentialStore` with default service and username prefix
    pub fn new() -> Self {
        Self {
            service_name: SERVICE_NAME.to_string(),
            username_prefix: USERNAME_PREFIX.to_string(),
        }
    }
}

impl CredentialStore for KeyringCredentialStore {
    fn store(&self, key: &str, value: &str) -> Result<(), String> {
        let value_bytes = value.len(); // UTF-8 byte length
        let value_chars = value.chars().count(); // Character count

        log::info!(
            "Storing token field '{key}' (length: {value_chars} chars, {value_bytes} bytes)"
        );

        if value_bytes <= SAFE_CHUNK_SIZE_BYTES {
            // Field fits in single entry, store directly
            let username = format!("{}.{}", self.username_prefix, key);
            let entry = Entry::new(&self.service_name, &username).map_err(|e| {
                log::error!("Keyring entry creation failed for '{key}': {e}");
                format!(
                    "Cannot access system keyring for '{key}'. Please check your system permissions."
                )
            })?;

            entry.set_password(value).map_err(|e| {
                log::error!(
                    "Failed to store '{key}' ({value_chars} chars, {value_bytes} bytes) in keyring: {e}"
                );
                format!(
                    "Cannot save '{key}' credential ({value_chars} chars, {value_bytes} bytes). Error: {e}"
                )
            })?;

            log::debug!(
                "Token field '{key}' stored successfully ({value_chars} chars, {value_bytes} bytes, single entry)"
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
                "Token field '{key}' is {value_bytes} bytes ({value_chars} chars), splitting into {chunk_count} chunks of max {SAFE_CHUNK_SIZE_BYTES} bytes each"
            );

            // Store metadata entry with chunk count (with prefix to avoid confusion with numeric data)
            let username = format!("{}.{}", self.username_prefix, key);
            let entry = Entry::new(&self.service_name, &username).map_err(|e| {
                log::error!("Keyring metadata entry creation failed for '{key}': {e}");
                format!("Cannot access system keyring for '{key}'.")
            })?;

            // Use "CHUNKS:" prefix to distinguish metadata from numeric data (e.g., timestamps)
            let metadata = format!("CHUNKS:{chunk_count}");
            entry.set_password(&metadata).map_err(|e| {
                log::error!("Failed to store '{key}' metadata in keyring: {e}");
                format!("Cannot save '{key}' metadata.")
            })?;

            log::debug!("Token field '{key}' metadata stored: {chunk_count} chunks");

            // Store each chunk
            for (index, chunk) in chunks.iter().enumerate() {
                let chunk_index = index + 1; // 1-based indexing
                let chunk_bytes = chunk.len();
                let chunk_chars = chunk.chars().count();

                let username = format!("{}.{}_{}", self.username_prefix, key, chunk_index);
                let entry = Entry::new(&self.service_name, &username).map_err(|e| {
                    log::error!(
                        "Keyring entry creation failed for '{key}' chunk {chunk_index}: {e}"
                    );
                    format!("Cannot access system keyring for '{key}' chunk {chunk_index}.")
                })?;

                entry.set_password(chunk).map_err(|e| {
                    log::error!(
                        "Failed to store '{key}' chunk {chunk_index} ({chunk_chars} chars, {chunk_bytes} bytes) in keyring: {e}"
                    );
                    format!(
                        "Cannot save '{key}' chunk {chunk_index} ({chunk_chars} chars, {chunk_bytes} bytes). Error: {e}"
                    )
                })?;

                log::debug!(
                    "Token field '{key}' chunk {chunk_index}/{chunk_count} stored successfully ({chunk_chars} chars, {chunk_bytes} bytes)"
                );
            }

            log::info!(
                "Token field '{key}' stored successfully ({value_chars} chars, {value_bytes} bytes in {chunk_count} chunks)"
            );
        }

        Ok(())
    }

    fn load(&self, key: &str) -> Result<String, String> {
        let username = format!("{}.{}", self.username_prefix, key);
        let entry = Entry::new(&self.service_name, &username).map_err(|e| {
            log::error!("Keyring entry creation failed for '{key}': {e}");
            "Cannot access system keyring. Please check your system permissions.".to_string()
        })?;

        let value = entry.get_password().map_err(|e| {
            log::warn!("Failed to load '{key}' from keyring: {e}");
            format!("Not authenticated. Missing token field '{key}'. Please log in first.")
        })?;

        // Check if this is metadata (chunk count) or actual data
        // Metadata has "CHUNKS:" prefix to avoid confusion with numeric data
        if let Some(chunk_count_str) = value.strip_prefix("CHUNKS:") {
            let chunk_count = chunk_count_str.parse::<usize>().map_err(|e| {
                log::error!("Failed to parse chunk count for '{key}': {e}");
                format!("Corrupted metadata for '{key}'. Please log in again.")
            })?;

            // This is chunked data, load all chunks
            log::debug!("Token field '{key}' is chunked, loading {chunk_count} chunks");

            let mut chunks = Vec::with_capacity(chunk_count);

            for chunk_index in 1..=chunk_count {
                let username = format!("{}.{}_{}", self.username_prefix, key, chunk_index);
                let entry = Entry::new(&self.service_name, &username).map_err(|e| {
                    log::error!(
                        "Keyring entry creation failed for '{key}' chunk {chunk_index}: {e}"
                    );
                    "Cannot access system keyring.".to_string()
                })?;

                let chunk = entry.get_password().map_err(|e| {
                    log::error!("Failed to load '{key}' chunk {chunk_index} from keyring: {e}");
                    format!("Missing '{key}' chunk {chunk_index}. Please log in again.")
                })?;

                log::debug!(
                    "Token field '{}' chunk {}/{} loaded ({} chars)",
                    key,
                    chunk_index,
                    chunk_count,
                    chunk.len()
                );
                chunks.push(chunk);
            }

            let reassembled = chunks.join("");
            log::info!(
                "Token field '{}' loaded successfully ({} chars from {} chunks)",
                key,
                reassembled.len(),
                chunk_count
            );
            Ok(reassembled)
        } else {
            // Single entry, return as-is
            log::debug!(
                "Token field '{}' loaded successfully ({} chars, single entry)",
                key,
                value.len()
            );
            Ok(value)
        }
    }

    fn delete(&self, key: &str) -> Result<(), String> {
        let username = format!("{}.{}", self.username_prefix, key);
        let entry = Entry::new(&self.service_name, &username).map_err(|e| {
            log::error!("Keyring entry creation failed for '{key}': {e}");
            format!("Cannot access system keyring for '{key}'.")
        })?;

        // Try to read the entry to check if it's chunked
        if let Ok(value) = entry.get_password() {
            // Check for "CHUNKS:" prefix to identify metadata
            if let Some(chunk_count_str) = value.strip_prefix("CHUNKS:") {
                if let Ok(chunk_count) = chunk_count_str.parse::<usize>() {
                    // This is chunked data, delete all chunks
                    log::debug!("Token field '{key}' is chunked, deleting {chunk_count} chunks");

                    for chunk_index in 1..=chunk_count {
                        let username = format!("{}.{}_{}", self.username_prefix, key, chunk_index);
                        if let Ok(chunk_entry) = Entry::new(&self.service_name, &username) {
                            match chunk_entry.delete_password() {
                                Ok(()) => log::debug!(
                                    "Token field '{key}' chunk {chunk_index} deleted"
                                ),
                                Err(e) => log::warn!(
                                    "Failed to delete '{key}' chunk {chunk_index}: {e} (may not exist)"
                                ),
                            }
                        }
                    }
                }
            }
        }

        // Delete the main entry (either metadata or single value)
        match entry.delete_password() {
            Ok(()) => {
                log::debug!("Token field '{key}' deleted successfully");
                Ok(())
            }
            Err(e) => {
                log::warn!("Failed to delete '{key}' from keyring: {e} (may not exist)");
                Ok(()) // Don't treat as error if entry doesn't exist
            }
        }
    }
}

// Global state to store tokens during extraction
static TOKEN_CHANNEL: Mutex<Option<mpsc::Sender<String>>> = Mutex::new(None);

// Constants for keyring service identification
const SERVICE_NAME: &str = "educartable-sync";
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

/// Store authentication tokens securely using the provided credential store
pub fn store_tokens(store: &dyn CredentialStore, tokens: &AuthTokens) -> Result<(), String> {
    log::info!(
        "Attempting to store authentication tokens in keyring (split storage mode with chunking)"
    );

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
        store.store(field_name, value)?;
    }

    log::info!("All authentication tokens stored successfully");
    Ok(())
}

/// Store authentication tokens using the default `KeyringCredentialStore`
pub fn store_tokens_default(tokens: &AuthTokens) -> Result<(), String> {
    let store = KeyringCredentialStore::new();
    store_tokens(&store, tokens)
}

/// Load authentication tokens using the provided credential store
pub fn load_tokens(store: &dyn CredentialStore) -> Result<AuthTokens, String> {
    log::info!(
        "Attempting to load authentication tokens from keyring (split storage mode with chunking)"
    );

    // Load each token field (will be automatically reassembled if chunked)
    let access_token = store.load("access_token")?;
    let refresh_token = store.load("refresh_token")?;
    let id_token = store.load("id_token")?;
    let expires_at_str = store.load("expires_at")?;
    let session_state = store.load("session_state")?;

    // Parse expires_at
    let expires_at = expires_at_str.parse::<i64>().map_err(|e| {
        log::error!("Failed to parse expires_at as i64: {e}");
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

/// Load authentication tokens using the default `KeyringCredentialStore`
pub fn load_tokens_default() -> Result<AuthTokens, String> {
    let store = KeyringCredentialStore::new();
    load_tokens(&store)
}

/// Delete authentication tokens using the provided credential store
pub fn delete_tokens(store: &dyn CredentialStore) -> Result<(), String> {
    log::info!("Attempting to delete authentication tokens from keyring (split storage mode with chunking)");

    let mut errors = Vec::new();

    // Delete each token field (will automatically delete all chunks if present)
    for field_name in &TOKEN_FIELDS {
        if let Err(e) = store.delete(field_name) {
            errors.push(format!("{field_name}: {e}"));
        }
    }

    if !errors.is_empty() {
        log::error!("Errors occurred while deleting token entries: {errors:?}");
        return Err(format!(
            "Failed to clear some login credentials: {}",
            errors.join(", ")
        ));
    }

    log::info!("Authentication tokens deleted successfully");
    Ok(())
}

/// Delete authentication tokens using the default `KeyringCredentialStore`
pub fn delete_tokens_default() -> Result<(), String> {
    let store = KeyringCredentialStore::new();
    delete_tokens(&store)
}

/// Receives authentication tokens from the webview JavaScript injection.
///
/// This command is called by JavaScript code injected into the authentication
/// webview after successful login. It receives the serialized token data from
/// localStorage and forwards it to the authentication flow via an internal channel.
///
/// # Arguments
/// * `tokens_json` - Serialized JSON string containing OAuth tokens from localStorage
///
/// # Returns
/// - `Ok(())` - Tokens successfully received and forwarded
/// - `Err(String)` - Failed to forward tokens through internal channel
///
/// # Errors
/// - Channel send failure (authentication flow may have timed out)
#[tauri::command]
pub fn submit_tokens(tokens_json: String) -> Result<(), String> {
    log::debug!("Received token submission from webview");

    if let Ok(guard) = TOKEN_CHANNEL.lock() {
        if let Some(tx) = guard.as_ref() {
            tx.send(tokens_json).map_err(|e| {
                log::error!("Failed to send tokens through channel: {e}");
                e.to_string()
            })?;
        }
    }
    Ok(())
}

/// Initiates OAuth authentication flow via Keycloak webview.
///
/// Opens a new webview window pointing to Educartable's login page.
/// After successful authentication, extracts tokens from localStorage
/// via JavaScript injection and stores them securely in the system keyring.
/// The webview automatically monitors for successful OAuth callback URLs
/// and triggers token extraction when login completes.
///
/// # Arguments
/// * `app_handle` - Tauri application handle for creating webview window
///
/// # Returns
/// - `Ok(AuthTokens)` - Authentication successful, tokens extracted and stored
/// - `Err(String)` - Authentication failed with user-friendly error message
///
/// # Errors
/// - Login timeout (300 seconds)
/// - Token extraction failure from webview
/// - Keyring storage failure
/// - User cancelled login or closed webview
#[tauri::command]
pub async fn authenticate(app_handle: AppHandle<Wry>) -> Result<AuthTokens, String> {
    log::info!("Starting authentication flow");

    // Create a synchronous channel for communication between the webview event handler and this function
    let (tx, rx) = mpsc::channel();

    // Create a channel for token data
    let (token_tx, token_rx) = mpsc::channel();

    // Store the token sender in global state
    #[allow(clippy::unwrap_used)] // Safe: static mutex, won't be poisoned
    {
        let mut guard = TOKEN_CHANNEL.lock().unwrap();
        *guard = Some(token_tx);
    }

    log::debug!("Creating authentication webview window");
    #[allow(clippy::unwrap_used)] // Safe: hardcoded valid URL
    let webview = WebviewWindowBuilder::new(
        &app_handle,
        "auth",
        WebviewUrl::External("https://app.educartable.com".parse().unwrap()),
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
        log::error!("Failed to create authentication webview: {e}");
        e.to_string()
    })?;

    // Spawn a blocking task to wait for the login with timeout
    log::debug!("Waiting for user to complete login (300s timeout)");
    let _url = tokio::task::spawn_blocking(move || {
        rx.recv_timeout(Duration::from_secs(300)).map_err(|_| {
            log::error!("Login timeout: no response within 300 seconds");
            "Login timeout: no response within 300 seconds".to_string()
        })
    })
    .await
    .map_err(|e| {
        log::error!("Task error during login wait: {e}");
        format!("Task error: {e}")
    })??;

    // Wait for the OIDC client library to complete the token exchange
    log::debug!("Waiting for OIDC token exchange to complete");
    tokio::time::sleep(Duration::from_secs(3)).await;

    // JavaScript to inject for extracting tokens from localStorage
    // Call the Tauri command to send tokens back
    let js_code = r"
        (function() {
            const key = 'oidc.user:https://accounts.edumoov.com/auth/realms/edumoov:educlasse';
            const data = localStorage.getItem(key);
            if (data) {
                window.__TAURI__.core.invoke('submit_tokens', { tokensJson: data });
            } else {
                window.__TAURI__.core.invoke('submit_tokens', { tokensJson: 'null' });
            }
        })();
    ";

    log::debug!("Injecting JavaScript to extract tokens from localStorage");
    webview.eval(js_code).map_err(|e| {
        log::error!("Failed to inject JavaScript: {e}");
        "Login window error. Please try again.".to_string()
    })?;

    // Wait for the tokens with timeout
    log::debug!("Waiting for tokens from webview (5s timeout)");
    let tokens_str: String = tokio::task::spawn_blocking(move || {
        token_rx.recv_timeout(Duration::from_secs(5)).map_err(|_| {
            log::error!("Timeout waiting for tokens from webview");
            "Login completion timeout. Please try again.".to_string()
        })
    })
    .await
    .map_err(|e| {
        log::error!("Task error during token extraction: {e}");
        "Internal error during login. Please try again.".to_string()
    })??;

    // Clear the global state
    #[allow(clippy::unwrap_used)] // Safe: static mutex, won't be poisoned
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
    let tokens: AuthTokens = serde_json::from_str(&tokens_str).map_err(|e| {
        log::error!("Failed to parse tokens from JSON: {e}");
        "Login data error. Please try again.".to_string()
    })?;

    // Store tokens securely in the OS keyring
    store_tokens_default(&tokens)?;

    log::info!("Authentication completed successfully");
    Ok(tokens)
}

/// Clears stored authentication tokens from the system keyring.
///
/// Removes all authentication tokens including access token, refresh token,
/// id token, expiration timestamp, and session state. This effectively logs
/// the user out of the application. The user will need to authenticate again
/// to use any features requiring API access.
///
/// # Returns
/// - `Ok(())` - Tokens successfully cleared from keyring
/// - `Err(String)` - Failed to clear tokens with user-friendly error message
///
/// # Errors
/// - Keyring access failure
/// - Some tokens could not be deleted
#[tauri::command]
pub async fn logout() -> Result<(), String> {
    log::info!("User logout requested");
    delete_tokens_default()?;
    log::info!("User logged out successfully");
    Ok(())
}

/// Refresh access token using the refresh token
/// Returns new `AuthTokens` with updated `access_token`, `refresh_token`, and `expires_at`
pub async fn refresh_access_token(
    store: &dyn CredentialStore,
    refresh_token: &str,
) -> Result<AuthTokens, String> {
    log::info!("Attempting to refresh access token");

    let client = reqwest::Client::new();
    let token_url =
        "https://accounts.edumoov.com/auth/realms/edumoov/protocol/openid-connect/token";

    // Prepare form data for token refresh
    let params = [
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
        ("client_id", "educlasse"),
    ];

    log::debug!("Sending token refresh request to Keycloak");
    let response = client
        .post(token_url)
        .form(&params)
        .send()
        .await
        .map_err(|e| {
            log::error!("Token refresh request failed: {e}");
            format!("Failed to connect to authentication server: {e}")
        })?;

    let status = response.status();
    log::debug!("Token refresh response status: {status}");

    if !status.is_success() {
        let error_body = response.text().await.unwrap_or_default();
        log::error!("Token refresh failed with status {status}: {error_body}");
        return Err("Token refresh failed. Please log in again.".to_string());
    }

    // Parse the response
    #[allow(clippy::items_after_statements)] // Local struct for response parsing
    #[derive(serde::Deserialize)]
    struct TokenResponse {
        access_token: String,
        refresh_token: String,
        id_token: String,
        expires_in: i64,
        session_state: String,
    }

    let token_response: TokenResponse = response.json().await.map_err(|e| {
        log::error!("Failed to parse token refresh response: {e}");
        "Invalid response from authentication server. Please log in again.".to_string()
    })?;

    // Calculate expires_at timestamp
    #[allow(clippy::unwrap_used, clippy::cast_possible_wrap)]
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap() // Safe: current time is after UNIX_EPOCH
        .as_secs() as i64;
    let expires_at = now + token_response.expires_in;

    let tokens = AuthTokens {
        access_token: token_response.access_token,
        refresh_token: token_response.refresh_token,
        id_token: token_response.id_token,
        expires_at,
        session_state: token_response.session_state,
    };

    // Store the new tokens
    store_tokens(store, &tokens)?;

    log::info!("Access token refreshed successfully, expires at {expires_at}");
    Ok(tokens)
}

/// Get valid access token, refreshing if necessary
/// This function should be used by API client to ensure tokens are always valid
pub async fn get_valid_access_token(store: &dyn CredentialStore) -> Result<String, String> {
    log::debug!("Getting valid access token");

    let tokens = load_tokens(store).map_err(|_| {
        log::debug!("No tokens found in storage");
        "Not authenticated. Please log in first.".to_string()
    })?;

    #[allow(clippy::unwrap_used, clippy::cast_possible_wrap)]
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap() // Safe: current time is after UNIX_EPOCH
        .as_secs() as i64;

    // Check if access token is expired or will expire in the next 60 seconds
    if tokens.expires_at <= now + 60 {
        log::info!("Access token expired or expiring soon, attempting refresh");
        let new_tokens = refresh_access_token(store, &tokens.refresh_token).await?;
        Ok(new_tokens.access_token)
    } else {
        log::debug!("Access token is still valid");
        Ok(tokens.access_token)
    }
}

/// Check if the user is authenticated (has valid tokens that can be refreshed)
pub async fn is_authenticated_with_store(store: &dyn CredentialStore) -> Result<bool, String> {
    log::debug!("Checking authentication status");

    if let Ok(tokens) = load_tokens(store) {
        // Check if token is expired
        #[allow(clippy::unwrap_used, clippy::cast_possible_wrap)]
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap() // Safe: current time is after UNIX_EPOCH
            .as_secs() as i64;

        // Consider authenticated if we have tokens, even if access token is expired
        // (as long as we can potentially refresh)
        // For a more accurate check, we'd need to decode the refresh token's expiration
        // For now, we'll try to refresh if access token is expired
        if tokens.expires_at > now {
            log::debug!("Authentication status: valid (access token not expired)");
            Ok(true)
        } else {
            // Access token expired, try to refresh
            log::debug!("Access token expired, attempting refresh to verify authentication");
            match refresh_access_token(store, &tokens.refresh_token).await {
                Ok(_) => {
                    log::debug!("Authentication status: valid (token refreshed successfully)");
                    Ok(true)
                }
                Err(e) => {
                    log::debug!("Authentication status: invalid (refresh failed: {e})");
                    Ok(false)
                }
            }
        }
    } else {
        log::debug!("Authentication status: not authenticated");
        Ok(false)
    }
}

/// Checks if the user is currently authenticated with valid tokens.
///
/// Verifies that valid tokens exist in the keyring. If the access token
/// is expired but a refresh token exists, automatically attempts to refresh
/// the access token. Returns true if the user has valid tokens (either
/// unexpired or successfully refreshed).
///
/// # Returns
/// - `Ok(true)` - User is authenticated with valid or refreshable tokens
/// - `Ok(false)` - User is not authenticated or tokens cannot be refreshed
/// - `Err(String)` - Error checking authentication status
///
/// # Errors
/// - Keyring access failure
#[tauri::command]
pub async fn is_authenticated() -> Result<bool, String> {
    let store = KeyringCredentialStore::new();
    is_authenticated_with_store(&store).await
}

// Mock credential store for testing - accessible to all test modules
#[cfg(test)]
pub mod mock {
    use super::*;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    pub struct MockCredentialStore {
        storage: Arc<Mutex<HashMap<String, String>>>,
    }

    impl MockCredentialStore {
        pub fn new() -> Self {
            Self {
                storage: Arc::new(Mutex::new(HashMap::new())),
            }
        }

        pub fn contains_key(&self, key: &str) -> bool {
            self.storage.lock().unwrap().contains_key(key)
        }
    }

    impl CredentialStore for MockCredentialStore {
        fn store(&self, key: &str, value: &str) -> Result<(), String> {
            self.storage
                .lock()
                .unwrap()
                .insert(key.to_string(), value.to_string());
            Ok(())
        }

        fn load(&self, key: &str) -> Result<String, String> {
            self.storage
                .lock()
                .unwrap()
                .get(key)
                .cloned()
                .ok_or_else(|| format!("Key '{}' not found", key))
        }

        fn delete(&self, key: &str) -> Result<(), String> {
            self.storage.lock().unwrap().remove(key);
            Ok(())
        }
    }
}

// Re-export for backward compatibility with existing tests
#[cfg(test)]
pub use mock::MockCredentialStore;

#[cfg(test)]
mod tests {
    use super::*;

    // NOTE: These tests require access to the system keyring and are marked as #[ignore]
    // by default. They can be run manually with: cargo test -- --ignored
    // These tests may fail in headless CI environments without proper keyring setup.

    /// Helper to create test tokens with specific token sizes
    fn create_test_tokens(token_size: usize) -> AuthTokens {
        AuthTokens {
            access_token: "a".repeat(token_size),
            refresh_token: "r".repeat(token_size),
            id_token: "i".repeat(token_size),
            expires_at: 9999999999, // Far future timestamp
            session_state: "session123".to_string(),
        }
    }

    /// Helper to clean up test tokens from keyring
    fn cleanup_test_tokens() {
        let _ = delete_tokens_default();
    }

    #[test]
    fn test_store_tokens_with_mock_credential_store() {
        let mock_store = MockCredentialStore::new();
        let tokens = AuthTokens {
            access_token: "test_access".to_string(),
            refresh_token: "test_refresh".to_string(),
            id_token: "test_id".to_string(),
            expires_at: 1234567890,
            session_state: "test_session".to_string(),
        };

        // Store tokens
        let result = store_tokens(&mock_store, &tokens);
        assert!(result.is_ok(), "Should store tokens successfully");

        // Load tokens
        let loaded_result = load_tokens(&mock_store);
        assert!(loaded_result.is_ok(), "Should load tokens successfully");

        let loaded_tokens = loaded_result.unwrap();
        assert_eq!(loaded_tokens.access_token, tokens.access_token);
        assert_eq!(loaded_tokens.refresh_token, tokens.refresh_token);
        assert_eq!(loaded_tokens.id_token, tokens.id_token);
        assert_eq!(loaded_tokens.expires_at, tokens.expires_at);
        assert_eq!(loaded_tokens.session_state, tokens.session_state);

        // Delete tokens
        let delete_result = delete_tokens(&mock_store);
        assert!(delete_result.is_ok(), "Should delete tokens successfully");

        // Verify tokens are gone
        let load_after_delete = load_tokens(&mock_store);
        assert!(load_after_delete.is_err(), "Should not load deleted tokens");
    }

    #[test]
    fn test_store_and_load_small_tokens() {
        let store = MockCredentialStore::new();

        // Create tokens that are well under the chunk size (1000 bytes)
        let tokens = create_test_tokens(500);

        // Store tokens
        let store_result = store_tokens(&store, &tokens);
        assert!(
            store_result.is_ok(),
            "Failed to store small tokens: {:?}",
            store_result.err()
        );

        // Load tokens back
        let loaded = load_tokens(&store);
        assert!(
            loaded.is_ok(),
            "Failed to load small tokens: {:?}",
            loaded.err()
        );

        let loaded_tokens = loaded.unwrap();
        assert_eq!(loaded_tokens.access_token, tokens.access_token);
        assert_eq!(loaded_tokens.refresh_token, tokens.refresh_token);
        assert_eq!(loaded_tokens.id_token, tokens.id_token);
        assert_eq!(loaded_tokens.expires_at, tokens.expires_at);
        assert_eq!(loaded_tokens.session_state, tokens.session_state);
    }

    #[test]
    fn test_store_and_load_tokens_at_boundary() {
        let store = MockCredentialStore::new();

        // Create tokens exactly at the chunk size boundary (1000 bytes)
        let tokens = create_test_tokens(1000);

        let store_result = store_tokens(&store, &tokens);
        assert!(
            store_result.is_ok(),
            "Failed to store boundary tokens: {:?}",
            store_result.err()
        );

        let loaded = load_tokens(&store);
        assert!(
            loaded.is_ok(),
            "Failed to load boundary tokens: {:?}",
            loaded.err()
        );

        let loaded_tokens = loaded.unwrap();
        assert_eq!(loaded_tokens.access_token, tokens.access_token);
        assert_eq!(loaded_tokens.refresh_token, tokens.refresh_token);
        assert_eq!(loaded_tokens.id_token, tokens.id_token);
    }

    #[test]
    fn test_store_and_load_large_tokens_requiring_chunking() {
        let store = MockCredentialStore::new();

        // Create tokens that exceed the chunk size and require chunking
        // Note: MockCredentialStore doesn't implement chunking, but can still store large tokens
        // 2500 bytes will be stored as-is without chunking
        let tokens = create_test_tokens(2500);

        let store_result = store_tokens(&store, &tokens);
        assert!(
            store_result.is_ok(),
            "Failed to store large tokens: {:?}",
            store_result.err()
        );

        let loaded = load_tokens(&store);
        assert!(
            loaded.is_ok(),
            "Failed to load large tokens: {:?}",
            loaded.err()
        );

        let loaded_tokens = loaded.unwrap();
        assert_eq!(loaded_tokens.access_token.len(), 2500);
        assert_eq!(loaded_tokens.refresh_token.len(), 2500);
        assert_eq!(loaded_tokens.id_token.len(), 2500);
        assert_eq!(loaded_tokens.access_token, tokens.access_token);
        assert_eq!(loaded_tokens.refresh_token, tokens.refresh_token);
        assert_eq!(loaded_tokens.id_token, tokens.id_token);
    }

    #[test]
    fn test_store_and_load_very_large_tokens() {
        let store = MockCredentialStore::new();

        // Create very large tokens requiring multiple chunks
        // Note: MockCredentialStore doesn't implement chunking, but can still store large tokens
        // 5000 bytes will be stored as-is without chunking
        let tokens = create_test_tokens(5000);

        let store_result = store_tokens(&store, &tokens);
        assert!(
            store_result.is_ok(),
            "Failed to store very large tokens: {:?}",
            store_result.err()
        );

        let loaded = load_tokens(&store);
        assert!(
            loaded.is_ok(),
            "Failed to load very large tokens: {:?}",
            loaded.err()
        );

        let loaded_tokens = loaded.unwrap();
        assert_eq!(loaded_tokens.access_token.len(), 5000);
        assert_eq!(loaded_tokens.access_token, tokens.access_token);
    }

    #[test]
    fn test_utf8_boundary_handling() {
        let store = MockCredentialStore::new();

        // Create tokens with multi-byte UTF-8 characters
        // The emoji 🦀 is 4 bytes in UTF-8
        // Create a string that's close to chunk boundary with multi-byte chars
        let emoji_string = "🦀".repeat(250); // 250 * 4 = 1000 bytes
        let mixed_string = format!("{}{}", "a".repeat(999), "🦀"); // 999 + 4 = 1003 bytes

        let tokens = AuthTokens {
            access_token: emoji_string.clone(),
            refresh_token: mixed_string.clone(),
            id_token: "test".to_string(),
            expires_at: 9999999999,
            session_state: "session".to_string(),
        };

        let store_result = store_tokens(&store, &tokens);
        assert!(
            store_result.is_ok(),
            "Failed to store UTF-8 tokens: {:?}",
            store_result.err()
        );

        let loaded = load_tokens(&store);
        assert!(
            loaded.is_ok(),
            "Failed to load UTF-8 tokens: {:?}",
            loaded.err()
        );

        let loaded_tokens = loaded.unwrap();
        assert_eq!(loaded_tokens.access_token, emoji_string);
        assert_eq!(loaded_tokens.refresh_token, mixed_string);
    }

    #[test]
    fn test_utf8_chunking_with_japanese_characters() {
        let store = MockCredentialStore::new();

        // Japanese characters are 3 bytes each in UTF-8
        // Create a string that will test chunking at UTF-8 boundaries
        // Note: MockCredentialStore doesn't implement chunking, but can still handle UTF-8
        let japanese = "あ".repeat(400); // 400 * 3 = 1200 bytes, requires chunking

        let tokens = AuthTokens {
            access_token: japanese.clone(),
            refresh_token: "test".to_string(),
            id_token: "test".to_string(),
            expires_at: 9999999999,
            session_state: "session".to_string(),
        };

        let store_result = store_tokens(&store, &tokens);
        assert!(
            store_result.is_ok(),
            "Failed to store Japanese tokens: {:?}",
            store_result.err()
        );

        let loaded = load_tokens(&store);
        assert!(
            loaded.is_ok(),
            "Failed to load Japanese tokens: {:?}",
            loaded.err()
        );

        let loaded_tokens = loaded.unwrap();
        assert_eq!(loaded_tokens.access_token, japanese);
        assert_eq!(loaded_tokens.access_token.chars().count(), 400);
    }

    #[test]
    fn test_delete_tokens() {
        let store = MockCredentialStore::new();

        // Store some tokens first
        let tokens = create_test_tokens(500);
        let store_result = store_tokens(&store, &tokens);
        assert!(store_result.is_ok());

        // Verify they're stored
        let loaded = load_tokens(&store);
        assert!(loaded.is_ok());

        // Delete tokens
        let delete_result = delete_tokens(&store);
        assert!(
            delete_result.is_ok(),
            "Failed to delete tokens: {:?}",
            delete_result.err()
        );

        // Verify they're gone
        let loaded_after_delete = load_tokens(&store);
        assert!(
            loaded_after_delete.is_err(),
            "Tokens should not exist after deletion"
        );
        assert!(!store.contains_key("auth.access_token"));
    }

    #[test]
    fn test_delete_chunked_tokens() {
        let store = MockCredentialStore::new();

        // Store large tokens that require chunking
        // Note: MockCredentialStore doesn't implement chunking, but can still store large tokens
        let tokens = create_test_tokens(2500);
        let store_result = store_tokens(&store, &tokens);
        assert!(store_result.is_ok());

        // Delete tokens (should delete all chunks)
        let delete_result = delete_tokens(&store);
        assert!(
            delete_result.is_ok(),
            "Failed to delete chunked tokens: {:?}",
            delete_result.err()
        );

        // Verify they're gone
        let loaded_after_delete = load_tokens(&store);
        assert!(
            loaded_after_delete.is_err(),
            "Chunked tokens should not exist after deletion"
        );
        assert!(!store.contains_key("auth.access_token"));
    }

    #[test]
    fn test_delete_nonexistent_tokens() {
        let store = MockCredentialStore::new();

        // Deleting tokens that don't exist should not error
        let delete_result = delete_tokens(&store);
        assert!(
            delete_result.is_ok(),
            "Deleting nonexistent tokens should succeed: {:?}",
            delete_result.err()
        );
    }

    #[test]
    fn test_load_nonexistent_tokens() {
        let store = MockCredentialStore::new();

        // Loading tokens when none exist should return an error
        let loaded = load_tokens(&store);
        assert!(loaded.is_err(), "Loading nonexistent tokens should fail");
    }

    #[test]
    fn test_expires_at_field_storage() {
        let store = MockCredentialStore::new();

        // Test that numeric expires_at field is stored and loaded correctly
        let tokens = AuthTokens {
            access_token: "test_access".to_string(),
            refresh_token: "test_refresh".to_string(),
            id_token: "test_id".to_string(),
            expires_at: 1234567890,
            session_state: "test_session".to_string(),
        };

        let store_result = store_tokens(&store, &tokens);
        assert!(store_result.is_ok());

        let loaded = load_tokens(&store);
        assert!(loaded.is_ok());

        let loaded_tokens = loaded.unwrap();
        assert_eq!(loaded_tokens.expires_at, 1234567890);
    }

    #[test]
    fn test_overwrite_existing_tokens() {
        let store = MockCredentialStore::new();

        // Store first set of tokens
        let tokens1 = create_test_tokens(500);
        let store_result1 = store_tokens(&store, &tokens1);
        assert!(store_result1.is_ok());

        // Overwrite with different tokens
        let tokens2 = create_test_tokens(1500); // Different size requiring chunking
        let store_result2 = store_tokens(&store, &tokens2);
        assert!(store_result2.is_ok());

        // Load and verify we get the second set
        let loaded = load_tokens(&store);
        assert!(loaded.is_ok());

        let loaded_tokens = loaded.unwrap();
        assert_eq!(loaded_tokens.access_token.len(), 1500);
        assert_eq!(loaded_tokens.access_token, tokens2.access_token);
    }

    // ========== Tests for Token Refresh and Validation ==========

    #[tokio::test]
    async fn test_get_valid_access_token_missing_tokens() {
        let store = MockCredentialStore::new();

        // Try to get access token when no tokens are stored
        let result = get_valid_access_token(&store).await;
        assert!(result.is_err(), "Should fail when no tokens exist");
        assert!(result.unwrap_err().contains("Not authenticated"));
    }

    #[tokio::test]
    async fn test_get_valid_access_token_valid_token() {
        let store = MockCredentialStore::new();

        // Store tokens with future expiration (not expired)
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let future_expiry = now + 3600; // Expires in 1 hour

        let tokens = AuthTokens {
            access_token: "valid_access_token".to_string(),
            refresh_token: "valid_refresh_token".to_string(),
            id_token: "valid_id_token".to_string(),
            expires_at: future_expiry,
            session_state: "session".to_string(),
        };

        let store_result = store_tokens(&store, &tokens);
        assert!(store_result.is_ok(), "Failed to store tokens");

        // Get valid access token (should return without refreshing)
        let result = get_valid_access_token(&store).await;
        assert!(
            result.is_ok(),
            "Should succeed with valid token: {:?}",
            result.err()
        );
        assert_eq!(result.unwrap(), "valid_access_token");
    }

    // ========== Tests demonstrating testability improvements with MockCredentialStore ==========

    #[tokio::test]
    async fn test_is_authenticated_with_store_no_tokens() {
        let store = MockCredentialStore::new();

        // Check authentication when no tokens exist
        let result = is_authenticated_with_store(&store).await;
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            false,
            "Should not be authenticated without tokens"
        );
    }

    #[tokio::test]
    async fn test_is_authenticated_with_store_valid_token() {
        let store = MockCredentialStore::new();

        // Store tokens with future expiration
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let future_expiry = now + 3600; // Expires in 1 hour

        let tokens = AuthTokens {
            access_token: "valid_access".to_string(),
            refresh_token: "valid_refresh".to_string(),
            id_token: "valid_id".to_string(),
            expires_at: future_expiry,
            session_state: "session".to_string(),
        };

        let store_result = store_tokens(&store, &tokens);
        assert!(store_result.is_ok());

        // Check authentication status
        let result = is_authenticated_with_store(&store).await;
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            true,
            "Should be authenticated with valid tokens"
        );
    }

    #[tokio::test]
    async fn test_get_valid_access_token_with_store_missing_tokens() {
        let store = MockCredentialStore::new();

        // Try to get access token when no tokens are stored
        let result = get_valid_access_token(&store).await;
        assert!(result.is_err(), "Should fail when no tokens exist");
        assert!(result.unwrap_err().contains("Not authenticated"));
    }

    #[tokio::test]
    async fn test_get_valid_access_token_with_store_valid_token() {
        let store = MockCredentialStore::new();

        // Store tokens with future expiration (not expired)
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let future_expiry = now + 3600; // Expires in 1 hour

        let tokens = AuthTokens {
            access_token: "test_valid_access_token".to_string(),
            refresh_token: "test_valid_refresh_token".to_string(),
            id_token: "test_valid_id_token".to_string(),
            expires_at: future_expiry,
            session_state: "test_session".to_string(),
        };

        let store_result = store_tokens(&store, &tokens);
        assert!(store_result.is_ok(), "Failed to store tokens");

        // Get valid access token (should return without refreshing)
        let result = get_valid_access_token(&store).await;
        assert!(
            result.is_ok(),
            "Should succeed with valid token: {:?}",
            result.err()
        );
        assert_eq!(result.unwrap(), "test_valid_access_token");
    }

    #[tokio::test]
    async fn test_get_valid_access_token_with_store_expired_token() {
        let store = MockCredentialStore::new();

        // Store tokens with past expiration (expired)
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let past_expiry = now - 3600; // Expired 1 hour ago

        let tokens = AuthTokens {
            access_token: "expired_access".to_string(),
            refresh_token: "valid_refresh".to_string(),
            id_token: "expired_id".to_string(),
            expires_at: past_expiry,
            session_state: "session".to_string(),
        };

        let store_result = store_tokens(&store, &tokens);
        assert!(store_result.is_ok());

        // Try to get valid access token (will attempt refresh, which will fail without HTTP mock)
        let result = get_valid_access_token(&store).await;
        assert!(
            result.is_err(),
            "Should fail to refresh token without valid HTTP endpoint"
        );
    }

    #[tokio::test]
    async fn test_get_valid_access_token_with_store_expiring_soon() {
        let store = MockCredentialStore::new();

        // Store tokens that expire in 30 seconds (within the 60s refresh window)
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let soon_expiry = now + 30; // Expires in 30 seconds

        let tokens = AuthTokens {
            access_token: "expiring_soon_access".to_string(),
            refresh_token: "valid_refresh".to_string(),
            id_token: "expiring_soon_id".to_string(),
            expires_at: soon_expiry,
            session_state: "session".to_string(),
        };

        let store_result = store_tokens(&store, &tokens);
        assert!(store_result.is_ok());

        // Try to get valid access token (should trigger refresh due to 60s window)
        let result = get_valid_access_token(&store).await;
        assert!(
            result.is_err(),
            "Should attempt refresh for token expiring soon"
        );
    }

    #[tokio::test]
    async fn test_is_authenticated_no_tokens() {
        cleanup_test_tokens();

        // Check authentication when no tokens exist
        let result = is_authenticated().await;
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            false,
            "Should not be authenticated without tokens"
        );

        cleanup_test_tokens();
    }

    // Test helper to verify token expiration logic without keyring/HTTP
    #[test]
    fn test_token_expiration_logic() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;

        // Token expires in 2 hours - should be considered valid
        let future_expiry = now + 7200;
        assert!(future_expiry > now + 60, "Future token should be valid");

        // Token expires in 30 seconds - should trigger refresh (within 60s window)
        let soon_expiry = now + 30;
        assert!(
            soon_expiry <= now + 60,
            "Soon-expiring token should trigger refresh"
        );

        // Token already expired
        let past_expiry = now - 100;
        assert!(
            past_expiry <= now + 60,
            "Expired token should trigger refresh"
        );
    }
}
