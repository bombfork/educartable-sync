use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// Data models

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub id_token: String,
    pub expires_at: i64,
    pub session_state: String,
}

// Issue #20: User Info
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserInfo {
    pub id: i64,
    #[serde(rename = "mail")]
    pub email: String,
    #[serde(default)]
    pub name: Option<String>,
    #[serde(default)]
    pub firstname: Option<String>,
    #[serde(default)]
    pub lastname: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UserInfoResponse {
    pub data: UserInfo,
}

// Issue #21: Activities and Pagination
#[derive(Debug, Serialize, Deserialize)]
pub struct ActivitiesResponse {
    pub success: bool,
    pub data: Vec<Activity>,
    pub pagination: Pagination,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Activity {
    pub id: String,
    pub title: String,
    pub body: String,
    pub date: String,
    pub medias: Vec<Media>,
    pub pupils: Vec<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Media {
    pub id: String,
    pub name: String,
    pub extension: String,
    pub size: u64,
    #[serde(rename = "type")]
    pub media_type: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Pagination {
    pub page_count: u32,
    pub current_page: u32,
    pub has_next_page: bool,
    pub has_prev_page: bool,
    pub count: u32,
    pub limit: u32,
}

// Issue #26: Progress tracking structures
#[derive(Debug, Serialize, Clone)]
pub struct SyncProgress {
    pub current: u32,
    pub total: u32,
    pub current_file: String,
    pub percentage: f32,
}

#[derive(Debug, Serialize, Default)]
pub struct SyncStats {
    pub total_activities: u32,
    pub total_media: u32,
    pub downloaded: u32,
    pub skipped: u32,
    pub failed: u32,
}

// App Configuration
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AppConfig {
    pub sync_path: PathBuf,
}
