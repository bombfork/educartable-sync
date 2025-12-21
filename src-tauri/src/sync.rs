// Sync engine for downloading media

use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use reqwest::Client;
use std::path::{Path, PathBuf};
use tokio::time::{sleep, Duration};
use tokio::fs;
use tauri::{AppHandle, Emitter};
use crate::api::EducartableClient;
use crate::models::{Activity, Media, SyncProgress, SyncStats};

// Issue #24: File download from signed CDN URLs
pub async fn download_file(
    url: &str,
    destination: &Path
) -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();

    let response = client.get(url).send().await?;

    if !response.status().is_success() {
        return Err(format!("Download failed with status: {}", response.status()).into());
    }

    // Create parent directory if it doesn't exist
    if let Some(parent) = destination.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    // Stream response to file
    let mut file = File::create(destination).await?;
    let bytes = response.bytes().await?;
    file.write_all(&bytes).await?;

    Ok(())
}

// Issue #25: Directory structure creation
pub fn get_activity_folder(
    sync_path: &PathBuf,
    activity: &Activity
) -> PathBuf {
    // Extract date (YYYY-MM-DD from ISO datetime)
    let date = activity.date.split('T')
        .next()
        .unwrap_or("unknown-date");

    // Sanitize title for filesystem
    let safe_title = sanitize_filename(&activity.title);

    // Create folder name
    let folder_name = format!("{}_{}", date, safe_title);

    sync_path.join(folder_name)
}

pub fn get_media_path(
    sync_path: &PathBuf,
    activity: &Activity,
    media: &Media
) -> PathBuf {
    let folder = get_activity_folder(sync_path, activity);

    // Build filename with extension
    let filename = format!("{}{}", media.name, media.extension);

    folder.join(filename)
}

fn sanitize_filename(title: &str) -> String {
    title.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .chars()
        .take(50)  // Limit length
        .collect()
}

// Issue #5: Save article text as markdown
pub async fn save_article_markdown(
    sync_path: &PathBuf,
    activity: &Activity
) -> Result<(), Box<dyn std::error::Error>> {
    let folder = get_activity_folder(sync_path, activity);

    // Create directory if it doesn't exist
    tokio::fs::create_dir_all(&folder).await?;

    let article_path = folder.join("article.md");

    // Format the markdown content
    let markdown_content = format_article_markdown(activity);

    // Write to file
    let mut file = File::create(article_path).await?;
    file.write_all(markdown_content.as_bytes()).await?;

    Ok(())
}

fn format_article_markdown(activity: &Activity) -> String {
    // Extract date (YYYY-MM-DD from ISO datetime)
    let date = activity.date.split('T')
        .next()
        .unwrap_or("unknown-date");

    format!(
        "# {}\n\nPublished: {}\n\n{}",
        activity.title,
        date,
        activity.body
    )
}

// Issue #27: Duplicate detection
pub async fn should_download_file(
    path: &Path,
    expected_size: u64,
) -> Result<bool, Box<dyn std::error::Error>> {
    // Check if file exists
    if !path.exists() {
        return Ok(true);
    }

    // Get existing file size
    let metadata = fs::metadata(path).await?;
    let actual_size = metadata.len();

    // Re-download if size doesn't match (incomplete/corrupted file)
    if actual_size != expected_size {
        Ok(true)
    } else {
        Ok(false) // File is complete, skip download
    }
}

// Issue #28: Retry logic with exponential backoff
pub async fn download_with_retry(
    url: &str,
    destination: &Path,
    max_retries: u32,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut attempt = 0;

    loop {
        attempt += 1;

        match download_file(url, destination).await {
            Ok(_) => return Ok(()),
            Err(e) => {
                if attempt >= max_retries {
                    return Err(format!("Failed after {} attempts: {}", attempt, e).into());
                }

                // Check if error is retryable
                if !is_retryable_error(&e) {
                    return Err(e);
                }

                // Exponential backoff: 2^attempt seconds
                let wait_time = 2_u64.pow(attempt);
                eprintln!(
                    "Download attempt {} failed, retrying in {}s: {}",
                    attempt, wait_time, e
                );
                sleep(Duration::from_secs(wait_time)).await;
            }
        }
    }
}

fn is_retryable_error(error: &Box<dyn std::error::Error>) -> bool {
    let error_str = error.to_string().to_lowercase();

    // Network errors are retryable
    error_str.contains("connection")
        || error_str.contains("timeout")
        || error_str.contains("network")
        || error_str.contains("timed out")

    // File system errors are NOT retryable
}

// Issue #26: Progress tracking with Tauri events
pub struct SyncEngine {
    app_handle: AppHandle,
    api_client: EducartableClient,
    sync_path: PathBuf,
}

impl SyncEngine {
    pub fn new(app_handle: AppHandle, api_client: EducartableClient, sync_path: PathBuf) -> Self {
        Self {
            app_handle,
            api_client,
            sync_path,
        }
    }

    fn emit_progress(&self, current: u32, total: u32, filename: String) {
        let percentage = if total > 0 {
            (current as f32 / total as f32) * 100.0
        } else {
            0.0
        };

        let progress = SyncProgress {
            current,
            total,
            current_file: filename,
            percentage,
        };

        let _ = self.app_handle.emit("sync-progress", &progress);
    }

    pub async fn sync_all(&self) -> Result<SyncStats, Box<dyn std::error::Error>> {
        let stats = SyncStats::default();

        // TODO: Implement full sync logic using all the functions above

        Ok(stats)
    }
}

// Issue #29: Video file detection
pub fn is_video(media: &Media) -> bool {
    // Check by MIME type (most reliable)
    if media.media_type.starts_with("video/") {
        return true;
    }

    // Check by extension (fallback)
    let ext = media.extension.to_lowercase();
    matches!(
        ext.as_str(),
        ".mov" | ".mp4" | ".avi" | ".mkv" | ".webm" |
        ".wmv" | ".flv" | ".m4v" | ".mpg" | ".mpeg" |
        ".3gp" | ".ogv"
    )
}

// Issue #34: Tauri command for starting sync
#[tauri::command]
pub async fn start_sync(
    app_handle: AppHandle,
    config: crate::models::AppConfig,
) -> Result<SyncStats, String> {
    // Load authentication tokens
    let tokens = crate::auth::load_tokens()
        .map_err(|e| format!("Not authenticated: {}", e))?;

    // Validate sync path
    if config.sync_path.as_os_str().is_empty() {
        return Err("Sync directory not configured".to_string());
    }

    // Create API client
    let api_client = EducartableClient::new(tokens.access_token);

    // Create sync engine
    let sync_engine = SyncEngine::new(app_handle, api_client, config.sync_path);

    // Run synchronization
    sync_engine.sync_all().await
        .map_err(|e| format!("Sync failed: {}", e))
}
