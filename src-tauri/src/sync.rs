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
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    log::debug!("Downloading file to {:?}", destination);

    let client = Client::new();
    let response = client.get(url).send().await?;

    let status = response.status();
    if !status.is_success() {
        log::error!("Download failed with status: {}", status);
        return Err(format!("Download failed with status: {}", status).into());
    }

    // Create parent directory if it doesn't exist
    if let Some(parent) = destination.parent() {
        log::debug!("Creating directory: {:?}", parent);
        tokio::fs::create_dir_all(parent).await?;
    }

    // Stream response to file
    let mut file = File::create(destination).await?;
    let bytes = response.bytes().await?;
    let size = bytes.len();
    file.write_all(&bytes).await?;

    log::debug!("Downloaded {} bytes to {:?}", size, destination);
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
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let folder = get_activity_folder(sync_path, activity);
    log::debug!("Saving article markdown for activity {} to {:?}", activity.id, folder);

    // Create directory if it doesn't exist
    tokio::fs::create_dir_all(&folder).await?;

    let article_path = folder.join("article.md");

    // Format the markdown content
    let markdown_content = format_article_markdown(activity);

    // Write to file
    let mut file = File::create(&article_path).await?;
    file.write_all(markdown_content.as_bytes()).await?;

    log::debug!("Article saved to {:?}", article_path);
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
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    // Check if file exists
    if !path.exists() {
        log::debug!("File does not exist, will download: {:?}", path);
        return Ok(true);
    }

    // Get existing file size
    let metadata = fs::metadata(path).await?;
    let actual_size = metadata.len();

    // File exists - check if it seems valid (not empty and not suspiciously small)
    if actual_size == 0 {
        log::warn!("File exists but is empty, will re-download: {:?}", path);
        return Ok(true);
    }

    // If file is significantly smaller than expected (less than 50%), it might be corrupted
    let size_ratio = actual_size as f64 / expected_size as f64;
    if size_ratio < 0.5 {
        log::warn!("File exists but is suspiciously small ({} bytes, expected {}), will re-download: {:?}",
            actual_size, expected_size, path);
        return Ok(true);
    }

    // File exists and seems valid - skip download
    // Note: API size is often inaccurate (reports uncompressed size while CDN serves compressed),
    // so we don't do exact size matching
    log::debug!("File already exists ({} bytes, API reports {} bytes), skipping: {:?}",
        actual_size, expected_size, path);
    Ok(false)
}

// Issue #28: Retry logic with exponential backoff
pub async fn download_with_retry(
    url: &str,
    destination: &Path,
    max_retries: u32,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let mut attempt = 0;
    log::debug!("Starting download with retry for {:?}", destination);

    loop {
        attempt += 1;

        match download_file(url, destination).await {
            Ok(_) => {
                log::debug!("Download successful on attempt {} for {:?}", attempt, destination);
                return Ok(());
            }
            Err(e) => {
                if attempt >= max_retries {
                    log::error!("Download failed after {} attempts for {:?}: {}", attempt, destination, e);
                    return Err(format!("Failed after {} attempts: {}", attempt, e).into());
                }

                // Check if error is retryable
                if !is_retryable_error(&e) {
                    log::error!("Non-retryable error for {:?}: {}", destination, e);
                    return Err(e);
                }

                // Exponential backoff: 2^attempt seconds
                let wait_time = 2_u64.pow(attempt);
                log::warn!("Download attempt {} failed for {:?}, retrying in {}s: {}", attempt, destination, wait_time, e);
                sleep(Duration::from_secs(wait_time)).await;
            }
        }
    }
}

fn is_retryable_error(error: &Box<dyn std::error::Error + Send + Sync>) -> bool {
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
        log::debug!("Creating SyncEngine with sync_path: {:?}", sync_path);
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

    pub async fn sync_all(&self) -> Result<SyncStats, Box<dyn std::error::Error + Send + Sync>> {
        log::info!("Starting sync operation");
        let mut stats = SyncStats::default();

        // Emit starting progress
        self.emit_progress(0, 0, "Starting sync...".to_string());

        // Get parent ID
        log::info!("Fetching parent ID");
        let parent_id = self.api_client.get_parent_id().await
            .map_err(|e| {
                log::error!("Failed to get user info: {}", e);
                format!("Failed to get user info: {}", e)
            })?;

        // Fetch all activities
        self.emit_progress(0, 0, "Fetching activities...".to_string());
        log::info!("Fetching all activities");
        let activities = self.api_client.fetch_all_activities(parent_id).await
            .map_err(|e| {
                log::error!("Failed to fetch activities: {}", e);
                format!("Failed to fetch activities: {}", e)
            })?;

        stats.total_activities = activities.len() as u32;
        log::info!("Found {} activities to process", stats.total_activities);

        // Count total media files
        let total_media: u32 = activities.iter()
            .map(|a| a.medias.len() as u32)
            .sum();
        stats.total_media = total_media;
        log::info!("Found {} total media files to sync", total_media);

        let mut processed_media = 0u32;

        // Process each activity
        for activity in &activities {
            log::debug!("Processing activity {}: {}", activity.id, activity.title);

            // Save article as markdown
            if let Err(e) = save_article_markdown(&self.sync_path, activity).await {
                log::error!("Failed to save article {}: {}", activity.id, e);
            }

            // Process each media file
            for media in &activity.medias {
                processed_media += 1;

                // Skip videos if not configured to include them
                // Note: config.include_videos check would need to be passed to sync_all
                // For now, we'll download everything

                // Prepare filename for progress display
                let filename = format!("{}{}", media.name, media.extension);
                log::debug!("Processing media {}/{}: {}", processed_media, total_media, filename);
                self.emit_progress(processed_media, total_media, filename.clone());

                // Get destination path
                let destination = get_media_path(&self.sync_path, activity, media);

                // Check if file needs downloading
                log::debug!("Checking if file needs download: {} (expected size: {} bytes)", filename, media.size);
                match should_download_file(&destination, media.size).await {
                    Ok(true) => {
                        // File needs downloading
                        log::info!("Downloading: {}", filename);
                        match self.api_client.get_signed_media_url(&media.id, &filename).await {
                            Ok(signed_url) => {
                                // Download with retry
                                match download_with_retry(&signed_url, &destination, 3).await {
                                    Ok(_) => {
                                        // Verify downloaded file size
                                        if let Ok(metadata) = tokio::fs::metadata(&destination).await {
                                            log::info!("Successfully downloaded: {} ({} bytes, expected {} bytes)",
                                                filename, metadata.len(), media.size);
                                        } else {
                                            log::info!("Successfully downloaded: {}", filename);
                                        }
                                        stats.downloaded += 1;
                                    }
                                    Err(e) => {
                                        log::error!("Failed to download {}: {}", filename, e);
                                        stats.failed += 1;
                                    }
                                }
                            }
                            Err(e) => {
                                log::error!("Failed to get signed URL for {}: {}", filename, e);
                                stats.failed += 1;
                            }
                        }
                    }
                    Ok(false) => {
                        // File already exists and is complete
                        log::debug!("Skipping existing file: {}", filename);
                        stats.skipped += 1;
                    }
                    Err(e) => {
                        log::error!("Failed to check file status for {}: {}", filename, e);
                        stats.failed += 1;
                    }
                }
            }
        }

        // Emit completion
        self.emit_progress(total_media, total_media, "Sync complete!".to_string());

        log::info!("Sync completed. Downloaded: {}, Skipped: {}, Failed: {}", stats.downloaded, stats.skipped, stats.failed);
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
    log::info!("Sync command invoked");

    // Load authentication tokens
    log::debug!("Loading authentication tokens");
    let tokens = crate::auth::load_tokens()
        .map_err(|e| {
            log::error!("Not authenticated: {}", e);
            format!("Not authenticated: {}", e)
        })?;

    // Validate sync path
    if config.sync_path.as_os_str().is_empty() {
        log::error!("Sync directory not configured");
        return Err("Sync directory not configured".to_string());
    }

    log::info!("Sync directory: {:?}", config.sync_path);

    // Create API client
    let api_client = EducartableClient::new(tokens.access_token);

    // Create sync engine
    let sync_engine = SyncEngine::new(app_handle, api_client, config.sync_path);

    // Run synchronization
    log::info!("Starting synchronization");
    let result = sync_engine.sync_all().await
        .map_err(|e| {
            log::error!("Sync failed: {}", e);
            format!("Sync failed: {}", e)
        });

    match &result {
        Ok(stats) => log::info!("Sync completed successfully: {:?}", stats),
        Err(e) => log::error!("Sync failed: {}", e),
    }

    result
}
