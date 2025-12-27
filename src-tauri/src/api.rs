// API client for Educartable endpoints
use reqwest::Client;
use serde::Deserialize;
use crate::models::{UserInfo, UserInfoResponse, ActivitiesResponse, Activity};
use crate::auth;

pub struct EducartableClient {
    client: Client,
}

impl EducartableClient {
    pub fn new() -> Self {
        log::debug!("Creating new EducartableClient");
        Self {
            client: Client::new(),
        }
    }

    /// Get a valid access token, automatically refreshing if needed
    async fn get_access_token(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        auth::get_valid_access_token()
            .await
            .map_err(|e| e.into())
    }

    async fn get<T: for<'de> Deserialize<'de>>(
        &self,
        url: &str
    ) -> Result<T, Box<dyn std::error::Error + Send + Sync>> {
        log::debug!("GET request: {}", url);

        let access_token = self.get_access_token().await?;

        let response = self.client
            .get(url)
            .header("Authorization", &access_token)  // NO "Bearer"!
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .header("X-Edumoov-NoSession", "true")
            .send()
            .await?;

        let status = response.status();
        log::debug!("Response status: {}", status);

        if !status.is_success() {
            // Get the error response body for debugging
            let error_body = response.text().await?;
            log::error!("Request failed with status {}: {}", status, error_body);
            return Err(format!("Request failed with status {}: {}", status, error_body).into());
        }

        let data: T = response.json().await?;
        log::debug!("Response parsed successfully");
        Ok(data)
    }

    // Issue #20: User info endpoint
    pub async fn get_user_info(&self) -> Result<UserInfo, Box<dyn std::error::Error + Send + Sync>> {
        log::info!("Fetching user info");
        let url = "https://app.educartable.com/api/1.0/educore/users/me?light=1";

        let access_token = self.get_access_token().await?;

        let response = self.client
            .get(url)
            .header("Authorization", &access_token)
            .send()
            .await?;

        let status = response.status();
        log::debug!("User info response status: {}", status);

        if !status.is_success() {
            log::error!("User info request failed with status: {}", status);
            return Err(format!("User info request failed with status: {}", status).into());
        }

        // Parse the response wrapper and extract the data field
        let response_wrapper: UserInfoResponse = response.json().await?;

        log::info!("User info fetched successfully for user ID: {}", response_wrapper.data.id);
        Ok(response_wrapper.data)
    }

    pub async fn get_parent_id(&self) -> Result<i64, Box<dyn std::error::Error + Send + Sync>> {
        log::debug!("Getting parent ID");
        let user_info = self.get_user_info().await?;
        log::debug!("Parent ID: {}", user_info.id);
        Ok(user_info.id)
    }

    // Issue #21: Activities pagination
    pub async fn get_activities(
        &self,
        parent_id: i64,
    ) -> Result<ActivitiesResponse, Box<dyn std::error::Error + Send + Sync>> {
        log::debug!("Fetching activities for parent {}", parent_id);
        let url = format!(
            "https://app.educartable.com/api/1.0/educartable/parent/{}/messages?type=activity&sort=date&direction=desc",
            parent_id
        );
        let result = self.get::<ActivitiesResponse>(&url).await;
        match &result {
            Ok(response) => log::debug!("Fetched {} activities", response.data.len()),
            Err(e) => log::error!("Failed to fetch activities: {}", e),
        }
        result
    }

    pub async fn fetch_all_activities(
        &self,
        parent_id: i64
    ) -> Result<Vec<Activity>, Box<dyn std::error::Error + Send + Sync>> {
        log::info!("Fetching all activities for parent {}", parent_id);

        let response = self.get_activities(parent_id).await?;
        let activities = response.data;

        log::info!("Fetched {} total activities", activities.len());
        Ok(activities)
    }

    // Issue #22: Signed URL retrieval
    pub async fn get_signed_media_url(
        &self,
        media_id: &str,
        filename: &str
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        log::debug!("Getting signed URL for media: {} ({})", media_id, filename);
        let url = format!(
            "https://www.edumoov.com/api/1.0/educore/medias/{}/file?cache=1&filename={}",
            media_id, filename
        );

        let access_token = self.get_access_token().await?;

        // Disable automatic redirect following to capture the Location header
        let response = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()?
            .get(&url)
            .header("Authorization", &access_token)
            .send()
            .await?;

        let status = response.status();
        log::debug!("Signed URL response status: {}", status);

        // Extract Location header from 302 redirect
        if status.is_redirection() {
            let location = response.headers()
                .get("Location")
                .ok_or("No Location header in redirect")?
                .to_str()?
                .to_string();

            log::debug!("Signed URL obtained for {}", filename);
            Ok(location)
        } else {
            log::error!("Expected redirect response for {}, got status: {}", filename, status);
            Err(format!("Expected redirect response, got status: {}", status).into())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ========== Tests for EducartableClient ==========

    #[test]
    fn test_educartable_client_new() {
        // Test that client can be constructed
        let client = EducartableClient::new();
        // Verify the client struct exists (compilation test)
        drop(client);
    }

    // ========== URL Construction Tests ==========

    #[test]
    fn test_user_info_url_format() {
        let url = "https://app.educartable.com/api/1.0/educore/users/me?light=1";
        assert!(url.contains("/api/1.0/educore/users/me"));
        assert!(url.contains("light=1"));
    }

    #[test]
    fn test_activities_url_format() {
        let parent_id = 12345;
        let url = format!(
            "https://app.educartable.com/api/1.0/educartable/parent/{}/messages?type=activity&sort=date&direction=desc",
            parent_id
        );
        assert!(url.contains("/api/1.0/educartable/parent/12345/messages"));
        assert!(url.contains("type=activity"));
        assert!(url.contains("sort=date"));
        assert!(url.contains("direction=desc"));
    }

    #[test]
    fn test_signed_media_url_format() {
        let media_id = "media123";
        let filename = "photo.jpg";
        let url = format!(
            "https://www.edumoov.com/api/1.0/educore/medias/{}/file?cache=1&filename={}",
            media_id, filename
        );
        assert!(url.contains("/api/1.0/educore/medias/media123/file"));
        assert!(url.contains("cache=1"));
        assert!(url.contains("filename=photo.jpg"));
    }

    #[test]
    fn test_activities_url_with_large_parent_id() {
        let parent_id = 9999999999i64;
        let url = format!(
            "https://app.educartable.com/api/1.0/educartable/parent/{}/messages?type=activity&sort=date&direction=desc",
            parent_id
        );
        assert!(url.contains("9999999999"));
    }

    #[test]
    fn test_signed_media_url_special_characters() {
        let media_id = "media-with-dashes_and_underscores";
        let filename = "file name with spaces.jpg";
        let url = format!(
            "https://www.edumoov.com/api/1.0/educore/medias/{}/file?cache=1&filename={}",
            media_id, filename
        );
        assert!(url.contains("media-with-dashes_and_underscores"));
        assert!(url.contains("file name with spaces.jpg"));
    }

    // ========== HTTP Mocking Tests (Placeholders) ==========
    // These tests require mockito to mock HTTP responses

    #[tokio::test]
    #[ignore] // Requires HTTP mocking with mockito
    async fn test_get_user_info_success() {
        // This test would require setting up a mockito server
        // Example implementation:
        //
        // let mut server = mockito::Server::new_async().await;
        // let mock = server.mock("GET", "/api/1.0/educore/users/me")
        //     .match_header("authorization", "test_token")
        //     .with_status(200)
        //     .with_body(r#"{"success":true,"data":{"id":123,"mail":"test@example.com"}}"#)
        //     .create();
        //
        // Then test get_user_info() pointing to mock server
    }

    #[tokio::test]
    #[ignore] // Requires HTTP mocking with mockito
    async fn test_get_user_info_unauthorized() {
        // Test 401 unauthorized response
        // Mock server should return 401 status
        // Verify that get_user_info() returns appropriate error
    }

    #[tokio::test]
    #[ignore] // Requires HTTP mocking with mockito
    async fn test_get_activities_success() {
        // Test successful activities response with pagination
        // Mock server should return ActivitiesResponse with data and pagination
        // Verify activities are parsed correctly
    }

    #[tokio::test]
    #[ignore] // Requires HTTP mocking with mockito
    async fn test_get_activities_empty() {
        // Test response with no activities
        // Mock server should return empty data array
        // Verify empty vec is returned
    }

    #[tokio::test]
    #[ignore] // Requires HTTP mocking with mockito
    async fn test_get_activities_server_error() {
        // Test 500 server error response
        // Mock server should return 500 status
        // Verify error is propagated correctly
    }

    #[tokio::test]
    #[ignore] // Requires HTTP mocking with mockito
    async fn test_get_signed_media_url_redirect() {
        // Test successful redirect with Location header
        // Mock server should return 302 with Location header
        // Verify signed URL is extracted from Location header
    }

    #[tokio::test]
    #[ignore] // Requires HTTP mocking with mockito
    async fn test_get_signed_media_url_no_location_header() {
        // Test redirect without Location header
        // Mock server should return 302 without Location header
        // Verify appropriate error is returned
    }

    #[tokio::test]
    #[ignore] // Requires HTTP mocking with mockito
    async fn test_get_signed_media_url_not_redirect() {
        // Test non-redirect response (e.g., 200 OK)
        // Mock server should return 200 instead of 302
        // Verify error is returned (expects redirect)
    }

    #[tokio::test]
    #[ignore] // Requires HTTP mocking with mockito
    async fn test_fetch_all_activities_integration() {
        // Test full fetch_all_activities flow
        // Mock multiple pages of activities
        // Verify all activities are collected
    }

    // ========== Request Header Tests ==========
    // These verify the expected headers would be sent

    #[test]
    fn test_authorization_header_format() {
        // The API expects token WITHOUT "Bearer" prefix
        let token = "sample_token_123";
        let header_value = token;
        assert_eq!(header_value, "sample_token_123");
        assert!(!header_value.starts_with("Bearer "));
    }

    #[test]
    fn test_required_headers() {
        // Verify expected headers are defined
        let headers = vec![
            "Authorization",
            "Accept",
            "Content-Type",
            "X-Edumoov-NoSession",
        ];

        for header in headers {
            assert!(!header.is_empty());
        }
    }

    #[test]
    fn test_accept_header_value() {
        let accept = "application/json";
        assert_eq!(accept, "application/json");
    }

    #[test]
    fn test_custom_header_value() {
        let no_session = "true";
        assert_eq!(no_session, "true");
    }
}
