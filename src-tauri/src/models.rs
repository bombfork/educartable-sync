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

#[cfg(test)]
mod tests {
    use super::*;

    // ========== Tests for AuthTokens ==========

    #[test]
    fn test_auth_tokens_serialization() {
        let tokens = AuthTokens {
            access_token: "access123".to_string(),
            refresh_token: "refresh456".to_string(),
            id_token: "id789".to_string(),
            expires_at: 1234567890,
            session_state: "session".to_string(),
        };

        let json = serde_json::to_string(&tokens).unwrap();
        assert!(json.contains("access123"));
        assert!(json.contains("refresh456"));
        assert!(json.contains("1234567890"));
    }

    #[test]
    fn test_auth_tokens_deserialization() {
        let json = r#"{
            "access_token": "test_access",
            "refresh_token": "test_refresh",
            "id_token": "test_id",
            "expires_at": 9999999999,
            "session_state": "test_session"
        }"#;

        let tokens: AuthTokens = serde_json::from_str(json).unwrap();
        assert_eq!(tokens.access_token, "test_access");
        assert_eq!(tokens.refresh_token, "test_refresh");
        assert_eq!(tokens.expires_at, 9999999999);
    }

    // ========== Tests for UserInfo ==========

    #[test]
    fn test_user_info_with_mail_field() {
        // Test that "mail" field is renamed to "email"
        let json = r#"{
            "id": 123,
            "mail": "user@example.com",
            "name": "Test User",
            "firstname": "Test",
            "lastname": "User"
        }"#;

        let user_info: UserInfo = serde_json::from_str(json).unwrap();
        assert_eq!(user_info.id, 123);
        assert_eq!(user_info.email, "user@example.com");
        assert_eq!(user_info.name, Some("Test User".to_string()));
    }

    #[test]
    fn test_user_info_optional_fields_missing() {
        // Test that optional fields can be missing
        let json = r#"{
            "id": 456,
            "mail": "test@example.com"
        }"#;

        let user_info: UserInfo = serde_json::from_str(json).unwrap();
        assert_eq!(user_info.id, 456);
        assert_eq!(user_info.email, "test@example.com");
        assert_eq!(user_info.name, None);
        assert_eq!(user_info.firstname, None);
        assert_eq!(user_info.lastname, None);
    }

    #[test]
    fn test_user_info_response_wrapper() {
        let json = r#"{
            "data": {
                "id": 789,
                "mail": "wrapper@example.com"
            }
        }"#;

        let response: UserInfoResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.data.id, 789);
        assert_eq!(response.data.email, "wrapper@example.com");
    }

    // ========== Tests for Activity ==========

    #[test]
    fn test_activity_serialization() {
        let activity = Activity {
            id: "act123".to_string(),
            title: "Test Activity".to_string(),
            body: "Activity content".to_string(),
            date: "2024-01-01T12:00:00Z".to_string(),
            medias: vec![],
            pupils: vec![1, 2, 3],
        };

        let json = serde_json::to_string(&activity).unwrap();
        assert!(json.contains("act123"));
        assert!(json.contains("Test Activity"));
        assert!(json.contains("2024-01-01"));
    }

    #[test]
    fn test_activity_deserialization() {
        let json = r#"{
            "id": "act456",
            "title": "Test",
            "body": "Content",
            "date": "2024-06-15T10:30:00Z",
            "medias": [],
            "pupils": [10, 20]
        }"#;

        let activity: Activity = serde_json::from_str(json).unwrap();
        assert_eq!(activity.id, "act456");
        assert_eq!(activity.title, "Test");
        assert_eq!(activity.pupils, vec![10, 20]);
    }

    #[test]
    fn test_activity_with_medias() {
        let json = r#"{
            "id": "act789",
            "title": "Activity with Media",
            "body": "Content",
            "date": "2024-12-25T00:00:00Z",
            "medias": [
                {
                    "id": "media1",
                    "name": "photo",
                    "extension": ".jpg",
                    "size": 1024,
                    "type": "image"
                }
            ],
            "pupils": []
        }"#;

        let activity: Activity = serde_json::from_str(json).unwrap();
        assert_eq!(activity.medias.len(), 1);
        assert_eq!(activity.medias[0].name, "photo");
    }

    // ========== Tests for Media ==========

    #[test]
    fn test_media_with_type_field() {
        // Test that "type" field is renamed to "media_type"
        let json = r#"{
            "id": "m123",
            "name": "document",
            "extension": ".pdf",
            "size": 2048,
            "type": "document"
        }"#;

        let media: Media = serde_json::from_str(json).unwrap();
        assert_eq!(media.id, "m123");
        assert_eq!(media.name, "document");
        assert_eq!(media.extension, ".pdf");
        assert_eq!(media.size, 2048);
        assert_eq!(media.media_type, "document");
    }

    #[test]
    fn test_media_serialization() {
        let media = Media {
            id: "m456".to_string(),
            name: "photo".to_string(),
            extension: ".png".to_string(),
            size: 4096,
            media_type: "image".to_string(),
        };

        let json = serde_json::to_string(&media).unwrap();
        let deserialized: Media = serde_json::from_str(&json).unwrap();
        assert_eq!(media.id, deserialized.id);
        assert_eq!(media.media_type, deserialized.media_type);
    }

    // ========== Tests for Pagination ==========

    #[test]
    fn test_pagination_deserialization() {
        let json = r#"{
            "page_count": 10,
            "current_page": 1,
            "has_next_page": true,
            "has_prev_page": false,
            "count": 95,
            "limit": 10
        }"#;

        let pagination: Pagination = serde_json::from_str(json).unwrap();
        assert_eq!(pagination.page_count, 10);
        assert_eq!(pagination.current_page, 1);
        assert_eq!(pagination.has_next_page, true);
        assert_eq!(pagination.has_prev_page, false);
    }

    #[test]
    fn test_activities_response() {
        let json = r#"{
            "success": true,
            "data": [
                {
                    "id": "act1",
                    "title": "Activity 1",
                    "body": "Content 1",
                    "date": "2024-01-01T00:00:00Z",
                    "medias": [],
                    "pupils": [1]
                }
            ],
            "pagination": {
                "page_count": 1,
                "current_page": 1,
                "has_next_page": false,
                "has_prev_page": false,
                "count": 1,
                "limit": 10
            }
        }"#;

        let response: ActivitiesResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.success, true);
        assert_eq!(response.data.len(), 1);
        assert_eq!(response.pagination.page_count, 1);
    }

    // ========== Tests for SyncProgress ==========

    #[test]
    fn test_sync_progress_serialization() {
        let progress = SyncProgress {
            current: 50,
            total: 100,
            current_file: "photo.jpg".to_string(),
            percentage: 50.0,
        };

        let json = serde_json::to_string(&progress).unwrap();
        assert!(json.contains("50"));
        assert!(json.contains("100"));
        assert!(json.contains("photo.jpg"));
    }

    // ========== Tests for SyncStats ==========

    #[test]
    fn test_sync_stats_default() {
        let stats = SyncStats::default();
        assert_eq!(stats.total_activities, 0);
        assert_eq!(stats.total_media, 0);
        assert_eq!(stats.downloaded, 0);
        assert_eq!(stats.skipped, 0);
        assert_eq!(stats.failed, 0);
    }

    #[test]
    fn test_sync_stats_serialization() {
        let stats = SyncStats {
            total_activities: 10,
            total_media: 50,
            downloaded: 45,
            skipped: 3,
            failed: 2,
        };

        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("\"total_activities\":10"));
        assert!(json.contains("\"downloaded\":45"));
    }

    // ========== Edge Cases and Error Handling ==========

    #[test]
    fn test_activity_missing_required_field() {
        let json = r#"{
            "id": "act1",
            "title": "Missing body field"
        }"#;

        let result: Result<Activity, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_media_invalid_size_type() {
        let json = r#"{
            "id": "m1",
            "name": "test",
            "extension": ".jpg",
            "size": "not a number",
            "type": "image"
        }"#;

        let result: Result<Media, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    #[test]
    fn test_pagination_boolean_fields() {
        let json = r#"{
            "page_count": 5,
            "current_page": 2,
            "has_next_page": true,
            "has_prev_page": true,
            "count": 50,
            "limit": 10
        }"#;

        let pagination: Pagination = serde_json::from_str(json).unwrap();
        assert_eq!(pagination.has_next_page, true);
        assert_eq!(pagination.has_prev_page, true);
    }

    #[test]
    fn test_large_numbers() {
        let json = r#"{
            "id": 999999999999,
            "mail": "test@example.com"
        }"#;

        let user_info: UserInfo = serde_json::from_str(json).unwrap();
        assert_eq!(user_info.id, 999999999999);
    }
}
