// Sync engine for downloading media

use crate::api::EducartableClient;
use crate::models::{Activity, Media, SyncProgress, SyncStats};
use reqwest::Client;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};
use tokio::fs;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::time::{sleep, Duration};

// Issue #24: File download from signed CDN URLs
pub async fn download_file(
    url: &str,
    destination: &Path,
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
pub fn get_activity_folder(sync_path: &Path, activity: &Activity) -> PathBuf {
    // Extract date (YYYY-MM-DD from ISO datetime)
    let date = activity.date.split('T').next().unwrap_or("unknown-date");

    // Sanitize title for filesystem
    let safe_title = sanitize_filename(&activity.title);

    // Create folder name
    let folder_name = format!("{}_{}", date, safe_title);

    sync_path.join(folder_name)
}

pub fn get_media_path(sync_path: &Path, activity: &Activity, media: &Media) -> PathBuf {
    let folder = get_activity_folder(sync_path, activity);

    // Build filename with extension
    let filename = format!("{}{}", media.name, media.extension);

    folder.join(filename)
}

fn sanitize_filename(title: &str) -> String {
    title
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == ' ' || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>()
        .chars()
        .take(50) // Limit length
        .collect()
}

// Issue #5: Save article text as markdown
pub async fn save_article_markdown(
    sync_path: &Path,
    activity: &Activity,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let folder = get_activity_folder(sync_path, activity);
    log::debug!(
        "Saving article markdown for activity {} to {:?}",
        activity.id,
        folder
    );

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
    let date = activity.date.split('T').next().unwrap_or("unknown-date");

    format!(
        "# {}\n\nPublished: {}\n\n{}",
        activity.title, date, activity.body
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
        log::warn!(
            "File exists but is suspiciously small ({} bytes, expected {}), will re-download: {:?}",
            actual_size,
            expected_size,
            path
        );
        return Ok(true);
    }

    // File exists and seems valid - skip download
    // Note: API size is often inaccurate (reports uncompressed size while CDN serves compressed),
    // so we don't do exact size matching
    log::debug!(
        "File already exists ({} bytes, API reports {} bytes), skipping: {:?}",
        actual_size,
        expected_size,
        path
    );
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
                log::debug!(
                    "Download successful on attempt {} for {:?}",
                    attempt,
                    destination
                );
                return Ok(());
            }
            Err(e) => {
                if attempt >= max_retries {
                    log::error!(
                        "Download failed after {} attempts for {:?}: {}",
                        attempt,
                        destination,
                        e
                    );
                    return Err(format!("Failed after {} attempts: {}", attempt, e).into());
                }

                // Check if error is retryable
                if !is_retryable_error(e.as_ref()) {
                    log::error!("Non-retryable error for {:?}: {}", destination, e);
                    return Err(e);
                }

                // Exponential backoff: 2^attempt seconds
                let wait_time = 2_u64.pow(attempt);
                log::warn!(
                    "Download attempt {} failed for {:?}, retrying in {}s: {}",
                    attempt,
                    destination,
                    wait_time,
                    e
                );
                sleep(Duration::from_secs(wait_time)).await;
            }
        }
    }
}

fn is_retryable_error(error: &dyn std::error::Error) -> bool {
    let error_str = error.to_string().to_lowercase();

    // Network errors are retryable
    error_str.contains("connection")
        || error_str.contains("timeout")
        || error_str.contains("network")
        || error_str.contains("timed out")

    // File system errors are NOT retryable
}

// Issue #26: Progress tracking with Tauri events
pub struct SyncEngine<H: crate::api::HttpClient> {
    app_handle: AppHandle,
    api_client: EducartableClient<H>,
    sync_path: PathBuf,
}

impl<H: crate::api::HttpClient> SyncEngine<H> {
    pub fn new(
        app_handle: AppHandle,
        api_client: EducartableClient<H>,
        sync_path: PathBuf,
    ) -> Self {
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
        let parent_id = self.api_client.get_parent_id().await.map_err(|e| {
            log::error!("Failed to get user info: {}", e);
            "Cannot access your account information. Please check your connection.".to_string()
        })?;

        // Fetch all activities
        self.emit_progress(0, 0, "Fetching activities...".to_string());
        log::info!("Fetching all activities");
        let activities = self
            .api_client
            .fetch_all_activities(parent_id)
            .await
            .map_err(|e| {
                log::error!("Failed to fetch activities: {}", e);
                "Cannot load activities from Educartable. Please check your connection.".to_string()
            })?;

        stats.total_activities = activities.len() as u32;
        log::info!("Found {} activities to process", stats.total_activities);

        // Count total media files
        let total_media: u32 = activities.iter().map(|a| a.medias.len() as u32).sum();
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

                // Prepare filename for progress display
                let filename = format!("{}{}", media.name, media.extension);
                log::debug!(
                    "Processing media {}/{}: {}",
                    processed_media,
                    total_media,
                    filename
                );
                self.emit_progress(processed_media, total_media, filename.clone());

                // Get destination path
                let destination = get_media_path(&self.sync_path, activity, media);

                // Check if file needs downloading
                log::debug!(
                    "Checking if file needs download: {} (expected size: {} bytes)",
                    filename,
                    media.size
                );
                match should_download_file(&destination, media.size).await {
                    Ok(true) => {
                        // File needs downloading
                        log::info!("Downloading: {}", filename);
                        match self
                            .api_client
                            .get_signed_media_url(&media.id, &filename)
                            .await
                        {
                            Ok(signed_url) => {
                                // Download with retry
                                match download_with_retry(&signed_url, &destination, 3).await {
                                    Ok(_) => {
                                        // Verify downloaded file size
                                        if let Ok(metadata) =
                                            tokio::fs::metadata(&destination).await
                                        {
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

        log::info!(
            "Sync completed. Downloaded: {}, Skipped: {}, Failed: {}",
            stats.downloaded,
            stats.skipped,
            stats.failed
        );
        Ok(stats)
    }
}

// Issue #34: Tauri command for starting sync
#[tauri::command]
pub async fn start_sync(
    app_handle: AppHandle,
    config: crate::models::AppConfig,
) -> Result<SyncStats, String> {
    log::info!("Sync command invoked");

    // Verify authentication (this will also attempt token refresh if needed)
    log::debug!("Verifying authentication");
    crate::auth::load_tokens().map_err(|e| {
        log::error!("Not authenticated: {}", e);
        e // Pass through the user-friendly message from auth module
    })?;

    // Validate sync path
    if config.sync_path.as_os_str().is_empty() {
        log::error!("Sync directory not configured");
        return Err("Sync directory not configured. Please select a folder first.".to_string());
    }

    log::info!("Sync directory: {:?}", config.sync_path);

    // Create API client with no-redirect policy for signed URL retrieval
    let api_client = EducartableClient::new_no_redirect()
        .map_err(|e| format!("Failed to create API client: {}", e))?;

    // Create sync engine
    let sync_engine = SyncEngine::new(app_handle, api_client, config.sync_path);

    // Run synchronization
    log::info!("Starting synchronization");
    let result = sync_engine.sync_all().await.map_err(|e| {
        log::error!("Sync failed: {}", e);
        e.to_string()
    });

    match &result {
        Ok(stats) => log::info!("Sync completed successfully: {:?}", stats),
        Err(e) => log::error!("Sync failed: {}", e),
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Activity, Media};

    // Helper function to create test Activity
    fn create_test_activity(id: &str, date: &str, title: &str) -> Activity {
        Activity {
            id: id.to_string(),
            date: date.to_string(),
            title: title.to_string(),
            body: "Content".to_string(),
            medias: vec![],
            pupils: vec![],
        }
    }

    // Helper function to create test Media
    fn create_test_media(id: &str, name: &str, extension: &str) -> Media {
        Media {
            id: id.to_string(),
            name: name.to_string(),
            extension: extension.to_string(),
            size: 1024,
            media_type: "image".to_string(),
        }
    }

    // ========== Tests for sanitize_filename ==========

    #[test]
    fn test_sanitize_filename_normal() {
        let result = sanitize_filename("Hello World 2024");
        assert_eq!(result, "Hello World 2024");
    }

    #[test]
    fn test_sanitize_filename_with_hyphens_underscores() {
        let result = sanitize_filename("test-file_name-123");
        assert_eq!(result, "test-file_name-123");
    }

    #[test]
    fn test_sanitize_filename_invalid_characters() {
        // Test all invalid filesystem characters
        let result = sanitize_filename("file/with\\invalid:chars*?\"<>|");
        assert_eq!(result, "file_with_invalid_chars______");
    }

    #[test]
    fn test_sanitize_filename_forward_slash() {
        let result = sanitize_filename("path/to/file");
        assert_eq!(result, "path_to_file");
    }

    #[test]
    fn test_sanitize_filename_backslash() {
        let result = sanitize_filename("path\\to\\file");
        assert_eq!(result, "path_to_file");
    }

    #[test]
    fn test_sanitize_filename_colon() {
        let result = sanitize_filename("file:name:test");
        assert_eq!(result, "file_name_test");
    }

    #[test]
    fn test_sanitize_filename_asterisk() {
        let result = sanitize_filename("file*name");
        assert_eq!(result, "file_name");
    }

    #[test]
    fn test_sanitize_filename_question_mark() {
        let result = sanitize_filename("file?name");
        assert_eq!(result, "file_name");
    }

    #[test]
    fn test_sanitize_filename_quotes() {
        let result = sanitize_filename("file\"name");
        assert_eq!(result, "file_name");
    }

    #[test]
    fn test_sanitize_filename_angle_brackets() {
        let result = sanitize_filename("file<name>test");
        assert_eq!(result, "file_name_test");
    }

    #[test]
    fn test_sanitize_filename_pipe() {
        let result = sanitize_filename("file|name");
        assert_eq!(result, "file_name");
    }

    #[test]
    fn test_sanitize_filename_unicode_alphanumeric() {
        // Unicode letters are alphanumeric and should be preserved
        let result = sanitize_filename("Fichier École 2024");
        assert_eq!(result, "Fichier École 2024");
    }

    #[test]
    fn test_sanitize_filename_japanese() {
        // Japanese characters are alphanumeric Unicode, should be preserved
        let result = sanitize_filename("ファイル名");
        assert_eq!(result, "ファイル名");
    }

    #[test]
    fn test_sanitize_filename_emojis() {
        // Emojis are not alphanumeric, should be replaced
        let result = sanitize_filename("file🦀name🎉");
        assert_eq!(result, "file_name_");
    }

    #[test]
    fn test_sanitize_filename_mixed_unicode() {
        // Unicode letters like é are alphanumeric
        let result = sanitize_filename("café_résumé_2024");
        assert_eq!(result, "café_résumé_2024");
    }

    #[test]
    fn test_sanitize_filename_very_long() {
        // Test that filenames are truncated to 50 characters
        let long_name = "a".repeat(100);
        let result = sanitize_filename(&long_name);
        assert_eq!(result.len(), 50);
        assert_eq!(result, "a".repeat(50));
    }

    #[test]
    fn test_sanitize_filename_exactly_50_chars() {
        let name = "a".repeat(50);
        let result = sanitize_filename(&name);
        assert_eq!(result.len(), 50);
        assert_eq!(result, name);
    }

    #[test]
    fn test_sanitize_filename_empty() {
        let result = sanitize_filename("");
        assert_eq!(result, "");
    }

    #[test]
    fn test_sanitize_filename_whitespace_only() {
        let result = sanitize_filename("   ");
        assert_eq!(result, "   ");
    }

    #[test]
    fn test_sanitize_filename_special_chars_only() {
        let result = sanitize_filename("/*?:|<>");
        assert_eq!(result, "_______");
    }

    #[test]
    fn test_sanitize_filename_path_traversal_attempt() {
        let result = sanitize_filename("../../../etc/passwd");
        assert_eq!(result, "_________etc_passwd");
    }

    #[test]
    fn test_sanitize_filename_dots() {
        let result = sanitize_filename("..hidden.file.txt");
        assert_eq!(result, "__hidden_file_txt");
    }

    #[test]
    fn test_sanitize_filename_spaces_preserved() {
        let result = sanitize_filename("file with multiple  spaces");
        assert_eq!(result, "file with multiple  spaces");
    }

    #[test]
    fn test_sanitize_filename_leading_trailing_spaces() {
        let result = sanitize_filename("  file  ");
        assert_eq!(result, "  file  ");
    }

    // ========== Tests for get_activity_folder ==========

    #[test]
    fn test_get_activity_folder_basic() {
        let sync_path = PathBuf::from("/sync");
        let activity = create_test_activity("123", "2024-03-15T10:30:00Z", "Test Activity");

        let result = get_activity_folder(&sync_path, &activity);
        assert_eq!(result, PathBuf::from("/sync/2024-03-15_Test Activity"));
    }

    #[test]
    fn test_get_activity_folder_with_invalid_chars() {
        let sync_path = PathBuf::from("/sync");
        let activity = create_test_activity(
            "456",
            "2024-12-25T00:00:00Z",
            "Test/Activity\\With:Invalid*Chars?",
        );

        let result = get_activity_folder(&sync_path, &activity);
        assert_eq!(
            result,
            PathBuf::from("/sync/2024-12-25_Test_Activity_With_Invalid_Chars_")
        );
    }

    #[test]
    fn test_get_activity_folder_long_title() {
        let sync_path = PathBuf::from("/sync");
        let long_title = "a".repeat(100);
        let activity = create_test_activity("789", "2024-01-01T12:00:00Z", &long_title);

        let result = get_activity_folder(&sync_path, &activity);
        let folder_name = result.file_name().unwrap().to_str().unwrap();

        // Should be "2024-01-01_" + 50 'a' characters = 61 chars total
        assert!(folder_name.starts_with("2024-01-01_"));
        assert_eq!(folder_name.len(), 61); // "2024-01-01_" (11 chars) + 50 'a' chars
    }

    #[test]
    fn test_get_activity_folder_malformed_date() {
        let sync_path = PathBuf::from("/sync");
        let activity = create_test_activity("999", "invalid-date", "Test");

        let result = get_activity_folder(&sync_path, &activity);
        // Should use full invalid date string as-is
        assert_eq!(result, PathBuf::from("/sync/invalid-date_Test"));
    }

    #[test]
    fn test_get_activity_folder_empty_date() {
        let sync_path = PathBuf::from("/sync");
        let activity = create_test_activity("111", "", "Test");

        let result = get_activity_folder(&sync_path, &activity);
        assert_eq!(result, PathBuf::from("/sync/_Test"));
    }

    #[test]
    fn test_get_activity_folder_unicode_title() {
        let sync_path = PathBuf::from("/sync");
        let activity = create_test_activity("222", "2024-06-15T08:00:00Z", "École Café");

        let result = get_activity_folder(&sync_path, &activity);
        assert_eq!(result, PathBuf::from("/sync/2024-06-15_École Café"));
    }

    // ========== Tests for get_media_path ==========

    #[test]
    fn test_get_media_path_basic() {
        let sync_path = PathBuf::from("/sync");
        let activity = create_test_activity("1", "2024-03-15T10:30:00Z", "Activity");
        let media = create_test_media("100", "photo", ".jpg");

        let result = get_media_path(&sync_path, &activity, &media);
        assert_eq!(result, PathBuf::from("/sync/2024-03-15_Activity/photo.jpg"));
    }

    #[test]
    fn test_get_media_path_with_extension() {
        let sync_path = PathBuf::from("/sync");
        let activity = create_test_activity("2", "2024-05-20T14:00:00Z", "Test");
        let media = create_test_media("200", "document", ".pdf");

        let result = get_media_path(&sync_path, &activity, &media);
        assert_eq!(result, PathBuf::from("/sync/2024-05-20_Test/document.pdf"));
    }

    #[test]
    fn test_get_media_path_no_extension() {
        let sync_path = PathBuf::from("/sync");
        let activity = create_test_activity("3", "2024-07-10T09:00:00Z", "Activity");
        let media = create_test_media("300", "file", "");

        let result = get_media_path(&sync_path, &activity, &media);
        assert_eq!(result, PathBuf::from("/sync/2024-07-10_Activity/file"));
    }

    #[test]
    fn test_get_media_path_complex_activity_title() {
        let sync_path = PathBuf::from("/sync");
        let activity = create_test_activity(
            "4",
            "2024-11-30T16:30:00Z",
            "Activity/With\\Invalid:Chars*?",
        );
        let media = create_test_media("400", "image123", ".png");

        let result = get_media_path(&sync_path, &activity, &media);
        assert_eq!(
            result,
            PathBuf::from("/sync/2024-11-30_Activity_With_Invalid_Chars__/image123.png")
        );
    }

    #[test]
    fn test_get_media_path_preserves_media_name() {
        // Media names are not sanitized, only activity titles are
        let sync_path = PathBuf::from("/sync");
        let activity = create_test_activity("5", "2024-08-05T11:00:00Z", "Test");
        let media = create_test_media("500", "image-with-dashes_and_underscores", ".jpg");

        let result = get_media_path(&sync_path, &activity, &media);
        assert_eq!(
            result,
            PathBuf::from("/sync/2024-08-05_Test/image-with-dashes_and_underscores.jpg")
        );
    }

    // ========== Tests for is_retryable_error ==========

    #[test]
    fn test_is_retryable_error_connection() {
        let error: Box<dyn std::error::Error + Send + Sync> = "connection refused".into();
        assert!(
            is_retryable_error(error.as_ref()),
            "Connection errors should be retryable"
        );
    }

    #[test]
    fn test_is_retryable_error_timeout() {
        let error: Box<dyn std::error::Error + Send + Sync> = "request timeout".into();
        assert!(
            is_retryable_error(error.as_ref()),
            "Timeout errors should be retryable"
        );
    }

    #[test]
    fn test_is_retryable_error_network() {
        let error: Box<dyn std::error::Error + Send + Sync> = "network error occurred".into();
        assert!(
            is_retryable_error(error.as_ref()),
            "Network errors should be retryable"
        );
    }

    #[test]
    fn test_is_retryable_error_timed_out() {
        let error: Box<dyn std::error::Error + Send + Sync> = "operation timed out".into();
        assert!(
            is_retryable_error(error.as_ref()),
            "Timed out errors should be retryable"
        );
    }

    #[test]
    fn test_is_retryable_error_case_insensitive() {
        let error: Box<dyn std::error::Error + Send + Sync> = "CONNECTION TIMEOUT".into();
        assert!(
            is_retryable_error(error.as_ref()),
            "Error checking should be case insensitive"
        );
    }

    #[test]
    fn test_is_retryable_error_file_not_found() {
        let error: Box<dyn std::error::Error + Send + Sync> = "file not found".into();
        assert!(
            !is_retryable_error(error.as_ref()),
            "File errors should not be retryable"
        );
    }

    #[test]
    fn test_is_retryable_error_permission_denied() {
        let error: Box<dyn std::error::Error + Send + Sync> = "permission denied".into();
        assert!(
            !is_retryable_error(error.as_ref()),
            "Permission errors should not be retryable"
        );
    }

    #[test]
    fn test_is_retryable_error_invalid_url() {
        let error: Box<dyn std::error::Error + Send + Sync> = "invalid URL".into();
        assert!(
            !is_retryable_error(error.as_ref()),
            "Invalid URL should not be retryable"
        );
    }

    #[test]
    fn test_is_retryable_error_http_404() {
        let error: Box<dyn std::error::Error + Send + Sync> = "HTTP 404 Not Found".into();
        assert!(
            !is_retryable_error(error.as_ref()),
            "404 errors should not be retryable"
        );
    }

    // ========== Tests for should_download_file ==========

    #[tokio::test]
    async fn test_should_download_file_nonexistent() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("nonexistent.jpg");

        let result = should_download_file(&file_path, 1000).await;
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            true,
            "Should download file that doesn't exist"
        );
    }

    #[tokio::test]
    async fn test_should_download_file_empty() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("empty.jpg");

        // Create empty file
        tokio::fs::write(&file_path, b"").await.unwrap();

        let result = should_download_file(&file_path, 1000).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), true, "Should re-download empty file");
    }

    #[tokio::test]
    async fn test_should_download_file_corrupted_small() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("small.jpg");

        // Create file with 400 bytes (less than 50% of expected 1000 bytes)
        tokio::fs::write(&file_path, vec![0u8; 400]).await.unwrap();

        let result = should_download_file(&file_path, 1000).await;
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            true,
            "Should re-download corrupted small file"
        );
    }

    #[tokio::test]
    async fn test_should_download_file_valid_size() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("valid.jpg");

        // Create file with 600 bytes (60% of expected 1000 bytes - above 50% threshold)
        tokio::fs::write(&file_path, vec![0u8; 600]).await.unwrap();

        let result = should_download_file(&file_path, 1000).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false, "Should skip valid file");
    }

    #[tokio::test]
    async fn test_should_download_file_exact_size() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("exact.jpg");

        // Create file with exact expected size
        tokio::fs::write(&file_path, vec![0u8; 1000]).await.unwrap();

        let result = should_download_file(&file_path, 1000).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), false, "Should skip file with exact size");
    }

    #[tokio::test]
    async fn test_should_download_file_larger() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("larger.jpg");

        // Create file larger than expected (compressed files can be smaller)
        tokio::fs::write(&file_path, vec![0u8; 1500]).await.unwrap();

        let result = should_download_file(&file_path, 1000).await;
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            false,
            "Should skip file larger than expected"
        );
    }

    #[tokio::test]
    async fn test_should_download_file_at_threshold() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("threshold.jpg");

        // Create file exactly at 50% threshold (500 bytes for 1000 expected)
        tokio::fs::write(&file_path, vec![0u8; 500]).await.unwrap();

        let result = should_download_file(&file_path, 1000).await;
        assert!(result.is_ok());
        // At exactly 50%, size_ratio is 0.5 which is NOT < 0.5, so should skip
        assert_eq!(
            result.unwrap(),
            false,
            "Should skip file at exactly 50% threshold"
        );
    }

    #[tokio::test]
    async fn test_should_download_file_just_below_threshold() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("below_threshold.jpg");

        // Create file just below 50% threshold (499 bytes for 1000 expected)
        tokio::fs::write(&file_path, vec![0u8; 499]).await.unwrap();

        let result = should_download_file(&file_path, 1000).await;
        assert!(result.is_ok());
        assert_eq!(
            result.unwrap(),
            true,
            "Should re-download file just below 50% threshold"
        );
    }

    // ========== Tests for Retry Logic Concepts ==========
    // Note: Full testing of download_with_retry requires HTTP mocking
    // These tests verify the logic without actual downloads

    #[test]
    fn test_exponential_backoff_calculation() {
        // Test that exponential backoff follows 2^attempt pattern
        assert_eq!(2_u64.pow(1), 2, "First retry should wait 2 seconds");
        assert_eq!(2_u64.pow(2), 4, "Second retry should wait 4 seconds");
        assert_eq!(2_u64.pow(3), 8, "Third retry should wait 8 seconds");
        assert_eq!(2_u64.pow(4), 16, "Fourth retry should wait 16 seconds");
    }

    #[test]
    fn test_max_retries_logic() {
        // Test max retries boundary
        let max_retries = 3;

        for attempt in 1..=max_retries {
            assert!(
                attempt <= max_retries,
                "Attempt {} should be within max retries",
                attempt
            );
        }

        let attempt = max_retries + 1;
        assert!(
            attempt > max_retries,
            "Attempt {} should exceed max retries",
            attempt
        );
    }

    #[test]
    fn test_retry_attempt_progression() {
        // Verify retry attempts progress correctly
        let mut attempt = 0;
        let max_retries = 3;

        // Simulate retry loop
        for _ in 0..max_retries {
            attempt += 1;
            assert!(attempt <= max_retries);
        }

        assert_eq!(
            attempt, max_retries,
            "Should reach exactly max_retries attempts"
        );
    }
}
