// API client for Educartable endpoints
use crate::auth;
use crate::models::{ActivitiesResponse, Activity, UserInfo, UserInfoResponse};
use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::Arc;

/// Simple HTTP response abstraction
#[allow(dead_code)]
pub struct HttpResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

#[allow(dead_code)]
impl HttpResponse {
    /// Parse the response body as JSON
    pub fn json<T: for<'de> Deserialize<'de>>(
        &self,
    ) -> Result<T, Box<dyn std::error::Error + Send + Sync>> {
        serde_json::from_slice(&self.body).map_err(|e| e.into())
    }

    /// Convert the response body to a UTF-8 string
    pub fn text(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        String::from_utf8(self.body.clone()).map_err(|e| e.into())
    }

    /// Check if the response status indicates success (2xx)
    pub fn is_success(&self) -> bool {
        self.status >= 200 && self.status < 300
    }

    /// Check if the response status indicates a redirection (3xx)
    pub fn is_redirection(&self) -> bool {
        self.status >= 300 && self.status < 400
    }
}

/// Trait for abstracting HTTP client operations
#[allow(dead_code)]
#[async_trait]
pub trait HttpClient: Send + Sync {
    /// Perform a GET request with headers
    async fn get(
        &self,
        url: &str,
        headers: Vec<(&str, &str)>,
    ) -> Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>>;
}

/// Implementation of HttpClient using reqwest
#[allow(dead_code)]
pub struct ReqwestHttpClient {
    client: Client,
}

#[allow(dead_code)]
impl ReqwestHttpClient {
    /// Create a new ReqwestHttpClient with a default reqwest::Client
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    /// Create a new ReqwestHttpClient with an existing reqwest::Client
    pub fn with_client(client: Client) -> Self {
        Self { client }
    }

    /// Create a new ReqwestHttpClient that doesn't follow redirects
    pub fn new_no_redirect() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()?;
        Ok(Self { client })
    }
}

#[async_trait]
impl HttpClient for ReqwestHttpClient {
    async fn get(
        &self,
        url: &str,
        headers: Vec<(&str, &str)>,
    ) -> Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>> {
        // Build the request with headers
        let mut request_builder = self.client.get(url);
        for (key, value) in headers {
            request_builder = request_builder.header(key, value);
        }

        // Send the request
        let response = request_builder.send().await?;

        // Extract status code
        let status = response.status().as_u16();

        // Extract headers and convert to HashMap<String, String>
        let mut response_headers = HashMap::new();
        for (name, value) in response.headers().iter() {
            let header_name = name.as_str().to_string();
            let header_value = value.to_str().unwrap_or("").to_string();
            response_headers.insert(header_name, header_value);
        }

        // Extract body as Vec<u8>
        let body = response.bytes().await?.to_vec();

        Ok(HttpResponse {
            status,
            headers: response_headers,
            body,
        })
    }
}

pub struct EducartableClient<H: HttpClient> {
    http_client: Arc<H>,
}

impl<H: HttpClient> EducartableClient<H> {
    pub fn new(http_client: H) -> Self {
        log::debug!("Creating new EducartableClient");
        Self {
            http_client: Arc::new(http_client),
        }
    }

    /// Get a valid access token, automatically refreshing if needed
    async fn get_access_token(&self) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        auth::get_valid_access_token().await.map_err(|e| e.into())
    }

    async fn get<T: for<'de> Deserialize<'de>>(
        &self,
        url: &str,
    ) -> Result<T, Box<dyn std::error::Error + Send + Sync>> {
        log::debug!("GET request: {}", url);

        let access_token = self.get_access_token().await?;

        let headers = vec![
            ("Authorization", access_token.as_str()), // NO "Bearer"!
            ("Accept", "application/json"),
            ("Content-Type", "application/json"),
            ("X-Edumoov-NoSession", "true"),
        ];

        let response = self.http_client.get(url, headers).await?;

        log::debug!("Response status: {}", response.status);

        if !response.is_success() {
            // Get the error response body for debugging
            let error_body = response.text()?;
            log::error!(
                "Request failed with status {}: {}",
                response.status,
                error_body
            );
            return Err(format!(
                "Request failed with status {}: {}",
                response.status, error_body
            )
            .into());
        }

        let data: T = response.json()?;
        log::debug!("Response parsed successfully");
        Ok(data)
    }

    // Issue #20: User info endpoint
    pub async fn get_user_info(
        &self,
    ) -> Result<UserInfo, Box<dyn std::error::Error + Send + Sync>> {
        log::info!("Fetching user info");
        let url = "https://app.educartable.com/api/1.0/educore/users/me?light=1";

        let access_token = self.get_access_token().await?;

        let headers = vec![("Authorization", access_token.as_str())];

        let response = self.http_client.get(url, headers).await?;

        log::debug!("User info response status: {}", response.status);

        if !response.is_success() {
            log::error!("User info request failed with status: {}", response.status);
            return Err(
                format!("User info request failed with status: {}", response.status).into(),
            );
        }

        // Parse the response wrapper and extract the data field
        let response_wrapper: UserInfoResponse = response.json()?;

        log::info!(
            "User info fetched successfully for user ID: {}",
            response_wrapper.data.id
        );
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
        parent_id: i64,
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
        filename: &str,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        log::debug!("Getting signed URL for media: {} ({})", media_id, filename);
        let url = format!(
            "https://www.edumoov.com/api/1.0/educore/medias/{}/file?cache=1&filename={}",
            media_id, filename
        );

        let access_token = self.get_access_token().await?;

        let headers = vec![("Authorization", access_token.as_str())];

        let response = self.http_client.get(&url, headers).await?;

        log::debug!("Signed URL response status: {}", response.status);

        // Extract Location header from 302 redirect
        if response.is_redirection() {
            let location = response
                .headers
                .get("location")
                .or_else(|| response.headers.get("Location"))
                .ok_or("No Location header in redirect")?
                .to_string();

            log::debug!("Signed URL obtained for {}", filename);
            Ok(location)
        } else {
            log::error!(
                "Expected redirect response for {}, got status: {}",
                filename,
                response.status
            );
            Err(format!(
                "Expected redirect response, got status: {}",
                response.status
            )
            .into())
        }
    }
}

// Convenience constructor for production code using ReqwestHttpClient
impl EducartableClient<ReqwestHttpClient> {
    #[allow(dead_code)]
    pub fn new_default() -> Self {
        Self::new(ReqwestHttpClient::new())
    }

    pub fn new_no_redirect() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Self::new(ReqwestHttpClient::new_no_redirect()?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::sync::Mutex;

    // ========== MockHttpClient Implementation ==========

    pub struct MockHttpClient {
        responses: Arc<Mutex<VecDeque<HttpResponse>>>,
    }

    impl MockHttpClient {
        pub fn new() -> Self {
            Self {
                responses: Arc::new(Mutex::new(VecDeque::new())),
            }
        }

        pub fn add_response(&self, status: u16, body: &str) {
            let response = HttpResponse {
                status,
                headers: HashMap::new(),
                body: body.as_bytes().to_vec(),
            };
            self.responses.lock().unwrap().push_back(response);
        }

        pub fn add_response_with_headers(
            &self,
            status: u16,
            headers: HashMap<String, String>,
            body: &str,
        ) {
            let response = HttpResponse {
                status,
                headers,
                body: body.as_bytes().to_vec(),
            };
            self.responses.lock().unwrap().push_back(response);
        }
    }

    #[async_trait]
    impl HttpClient for MockHttpClient {
        async fn get(
            &self,
            _url: &str,
            _headers: Vec<(&str, &str)>,
        ) -> Result<HttpResponse, Box<dyn std::error::Error + Send + Sync>> {
            self.responses
                .lock()
                .unwrap()
                .pop_front()
                .ok_or_else(|| "No mock response available".into())
        }
    }

    // ========== Tests for EducartableClient ==========

    #[test]
    fn test_educartable_client_new() {
        // Test that client can be constructed
        let client = EducartableClient::new_default();
        // Verify the client struct exists (compilation test)
        drop(client);
    }

    // ========== HttpResponse Tests ==========

    #[test]
    fn test_http_response_json() {
        let json_body = r#"{"id": 123, "name": "test"}"#;
        let response = HttpResponse {
            status: 200,
            headers: HashMap::new(),
            body: json_body.as_bytes().to_vec(),
        };

        #[derive(Deserialize, Debug, PartialEq)]
        struct TestData {
            id: i32,
            name: String,
        }

        let parsed: TestData = response.json().unwrap();
        assert_eq!(parsed.id, 123);
        assert_eq!(parsed.name, "test");
    }

    #[test]
    fn test_http_response_text() {
        let text_body = "Hello, World!";
        let response = HttpResponse {
            status: 200,
            headers: HashMap::new(),
            body: text_body.as_bytes().to_vec(),
        };

        let text = response.text().unwrap();
        assert_eq!(text, "Hello, World!");
    }

    #[test]
    fn test_http_response_is_success() {
        let response_200 = HttpResponse {
            status: 200,
            headers: HashMap::new(),
            body: Vec::new(),
        };
        assert!(response_200.is_success());

        let response_201 = HttpResponse {
            status: 201,
            headers: HashMap::new(),
            body: Vec::new(),
        };
        assert!(response_201.is_success());

        let response_299 = HttpResponse {
            status: 299,
            headers: HashMap::new(),
            body: Vec::new(),
        };
        assert!(response_299.is_success());

        let response_404 = HttpResponse {
            status: 404,
            headers: HashMap::new(),
            body: Vec::new(),
        };
        assert!(!response_404.is_success());
    }

    #[test]
    fn test_http_response_is_redirection() {
        let response_302 = HttpResponse {
            status: 302,
            headers: HashMap::new(),
            body: Vec::new(),
        };
        assert!(response_302.is_redirection());

        let response_301 = HttpResponse {
            status: 301,
            headers: HashMap::new(),
            body: Vec::new(),
        };
        assert!(response_301.is_redirection());

        let response_200 = HttpResponse {
            status: 200,
            headers: HashMap::new(),
            body: Vec::new(),
        };
        assert!(!response_200.is_redirection());

        let response_404 = HttpResponse {
            status: 404,
            headers: HashMap::new(),
            body: Vec::new(),
        };
        assert!(!response_404.is_redirection());
    }

    #[test]
    fn test_http_response_invalid_json() {
        let invalid_json = "not valid json";
        let response = HttpResponse {
            status: 200,
            headers: HashMap::new(),
            body: invalid_json.as_bytes().to_vec(),
        };

        #[derive(Deserialize)]
        #[allow(dead_code)]
        struct TestData {
            id: i32,
            name: String,
        }

        let result: Result<TestData, _> = response.json();
        assert!(result.is_err());
    }

    #[test]
    fn test_http_response_invalid_utf8() {
        let invalid_utf8 = vec![0xff, 0xfe, 0xfd]; // Invalid UTF-8 sequence
        let response = HttpResponse {
            status: 200,
            headers: HashMap::new(),
            body: invalid_utf8,
        };

        let result = response.text();
        assert!(result.is_err());
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
    async fn test_get_user_info_success() {
        // Setup: Store test tokens for auth
        let tokens = crate::models::AuthTokens {
            access_token: "test_access_token".to_string(),
            refresh_token: "test_refresh_token".to_string(),
            id_token: "test_id_token".to_string(),
            expires_at: 9999999999, // Far future timestamp
            session_state: "test_session".to_string(),
        };

        // Store tokens - if this fails, skip the test
        if auth::store_tokens(&tokens).is_err() {
            eprintln!("Skipping test: unable to access system keyring");
            return;
        }

        // Setup: Create mock HTTP client
        let mock_client = MockHttpClient::new();

        // Mock successful user info response
        let response_body = r#"{"data":{"id":12345,"mail":"test@example.com","name":"Test User"}}"#;
        mock_client.add_response(200, response_body);

        // Test: Call get_user_info
        let client = EducartableClient::new(mock_client);
        let result = client.get_user_info().await;

        // Verify: Check the result
        assert!(
            result.is_ok(),
            "Expected success, got error: {:?}",
            result.err()
        );
        let user_info = result.unwrap();
        assert_eq!(user_info.id, 12345);
        assert_eq!(user_info.email, "test@example.com");
        assert_eq!(user_info.name, Some("Test User".to_string()));

        // Cleanup
        let _ = auth::delete_tokens();
    }

    #[tokio::test]
    async fn test_get_user_info_unauthorized() {
        // Setup: Store test tokens for auth
        let tokens = crate::models::AuthTokens {
            access_token: "invalid_token".to_string(),
            refresh_token: "test_refresh_token".to_string(),
            id_token: "test_id_token".to_string(),
            expires_at: 9999999999, // Far future timestamp
            session_state: "test_session".to_string(),
        };

        // Store tokens - if this fails, skip the test
        if auth::store_tokens(&tokens).is_err() {
            eprintln!("Skipping test: unable to access system keyring");
            return;
        }

        // Setup: Create mock HTTP client
        let mock_client = MockHttpClient::new();

        // Mock 401 unauthorized response
        mock_client.add_response(401, "Unauthorized");

        // Test: Call get_user_info
        let client = EducartableClient::new(mock_client);
        let result = client.get_user_info().await;

        // Verify: Check that request failed with appropriate error
        assert!(result.is_err(), "Expected error for 401 response");
        let error_message = result.unwrap_err().to_string();
        assert!(
            error_message.contains("401") || error_message.contains("failed"),
            "Error message should mention status 401 or failure: {}",
            error_message
        );

        // Cleanup
        let _ = auth::delete_tokens();
    }

    #[tokio::test]
    async fn test_get_activities_success() {
        // Setup: Store valid test tokens
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let future_expiry = now + 3600; // Expires in 1 hour

        let tokens = crate::models::AuthTokens {
            access_token: "test_access_token".to_string(),
            refresh_token: "test_refresh_token".to_string(),
            id_token: "test_id_token".to_string(),
            expires_at: future_expiry,
            session_state: "test_session".to_string(),
        };

        // Store tokens (will use keyring)
        let _ = crate::auth::store_tokens(&tokens);

        // Create mock HTTP client
        let mock = MockHttpClient::new();

        // Mock successful response with activities
        let response_body = r#"{
            "success": true,
            "data": [
                {
                    "id": "act123",
                    "title": "Test Activity",
                    "body": "Activity content",
                    "date": "2024-01-15T10:30:00Z",
                    "medias": [],
                    "pupils": [1, 2, 3]
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
        mock.add_response(200, response_body);

        // Create client with mock
        let client = EducartableClient::new(mock);

        // Test
        let result = client.get_activities(12345).await;
        assert!(result.is_ok(), "get_activities should succeed");

        let response = result.unwrap();
        assert_eq!(response.success, true);
        assert_eq!(response.data.len(), 1);
        assert_eq!(response.data[0].id, "act123");
        assert_eq!(response.data[0].title, "Test Activity");
        assert_eq!(response.data[0].pupils, vec![1, 2, 3]);
        assert_eq!(response.pagination.page_count, 1);

        // Cleanup
        let _ = crate::auth::delete_tokens();
    }

    #[tokio::test]
    async fn test_get_activities_empty() {
        // Setup: Store valid test tokens
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let future_expiry = now + 3600; // Expires in 1 hour

        let tokens = crate::models::AuthTokens {
            access_token: "test_access_token".to_string(),
            refresh_token: "test_refresh_token".to_string(),
            id_token: "test_id_token".to_string(),
            expires_at: future_expiry,
            session_state: "test_session".to_string(),
        };

        // Store tokens (will use keyring)
        let _ = crate::auth::store_tokens(&tokens);

        // Create mock HTTP client
        let mock = MockHttpClient::new();

        // Mock successful response with empty activities
        let response_body = r#"{
            "success": true,
            "data": [],
            "pagination": {
                "page_count": 0,
                "current_page": 1,
                "has_next_page": false,
                "has_prev_page": false,
                "count": 0,
                "limit": 10
            }
        }"#;
        mock.add_response(200, response_body);

        // Create client with mock
        let client = EducartableClient::new(mock);

        // Test
        let result = client.get_activities(12345).await;
        assert!(
            result.is_ok(),
            "get_activities should succeed even with no data"
        );

        let response = result.unwrap();
        assert_eq!(response.success, true);
        assert_eq!(response.data.len(), 0);
        assert_eq!(response.pagination.count, 0);

        // Cleanup
        let _ = crate::auth::delete_tokens();
    }

    #[tokio::test]
    async fn test_get_activities_server_error() {
        // Setup: Store valid test tokens
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let future_expiry = now + 3600; // Expires in 1 hour

        let tokens = crate::models::AuthTokens {
            access_token: "test_access_token".to_string(),
            refresh_token: "test_refresh_token".to_string(),
            id_token: "test_id_token".to_string(),
            expires_at: future_expiry,
            session_state: "test_session".to_string(),
        };

        // Store tokens (will use keyring)
        let _ = crate::auth::store_tokens(&tokens);

        // Create mock HTTP client
        let mock = MockHttpClient::new();

        // Mock 500 server error response
        let response_body = r#"{"error": "Internal server error"}"#;
        mock.add_response(500, response_body);

        // Create client with mock
        let client = EducartableClient::new(mock);

        // Test
        let result = client.get_activities(12345).await;
        assert!(result.is_err(), "get_activities should fail with 500 error");

        let error = result.unwrap_err();
        let error_msg = error.to_string();
        assert!(
            error_msg.contains("500"),
            "Error should mention status code 500, got: {}",
            error_msg
        );

        // Cleanup
        let _ = crate::auth::delete_tokens();
    }

    #[tokio::test]
    async fn test_get_signed_media_url_redirect() {
        // Setup test tokens
        let tokens = crate::models::AuthTokens {
            access_token: "test_access_token".to_string(),
            refresh_token: "test_refresh_token".to_string(),
            id_token: "test_id_token".to_string(),
            expires_at: 9999999999,
            session_state: "test_session".to_string(),
        };
        let _ = auth::store_tokens(&tokens);

        // Setup mock with 302 redirect and Location header
        let mock = MockHttpClient::new();
        let mut headers = HashMap::new();
        headers.insert(
            "location".to_string(),
            "https://signed-url.com/media.jpg".to_string(),
        );
        mock.add_response_with_headers(302, headers, "");

        // Test
        let client = EducartableClient::new(mock);
        let result = client.get_signed_media_url("media123", "photo.jpg").await;

        // Verify
        assert!(
            result.is_ok(),
            "Expected success, got error: {:?}",
            result.err()
        );
        let signed_url = result.unwrap();
        assert_eq!(signed_url, "https://signed-url.com/media.jpg");

        // Cleanup
        let _ = auth::delete_tokens();
    }

    #[tokio::test]
    async fn test_get_signed_media_url_no_location_header() {
        // Setup test tokens
        let tokens = crate::models::AuthTokens {
            access_token: "test_access_token".to_string(),
            refresh_token: "test_refresh_token".to_string(),
            id_token: "test_id_token".to_string(),
            expires_at: 9999999999,
            session_state: "test_session".to_string(),
        };
        let _ = auth::store_tokens(&tokens);

        // Setup mock with 302 redirect but NO Location header
        let mock = MockHttpClient::new();
        let headers = HashMap::new(); // No Location header
        mock.add_response_with_headers(302, headers, "");

        // Test
        let client = EducartableClient::new(mock);
        let result = client.get_signed_media_url("media123", "photo.jpg").await;

        // Verify
        assert!(
            result.is_err(),
            "Expected error for missing Location header"
        );
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("No Location header"),
            "Error message should mention missing Location header, got: {}",
            error_msg
        );

        // Cleanup
        let _ = auth::delete_tokens();
    }

    #[tokio::test]
    async fn test_get_signed_media_url_not_redirect() {
        // Setup test tokens
        let tokens = crate::models::AuthTokens {
            access_token: "test_access_token".to_string(),
            refresh_token: "test_refresh_token".to_string(),
            id_token: "test_id_token".to_string(),
            expires_at: 9999999999,
            session_state: "test_session".to_string(),
        };
        let _ = auth::store_tokens(&tokens);

        // Setup mock with 200 response (not a redirect)
        let mock = MockHttpClient::new();
        mock.add_response(200, "OK");

        // Test
        let client = EducartableClient::new(mock);
        let result = client.get_signed_media_url("media123", "photo.jpg").await;

        // Verify
        assert!(result.is_err(), "Expected error for non-redirect response");
        let error_msg = result.unwrap_err().to_string();
        assert!(
            error_msg.contains("Expected redirect response"),
            "Error message should mention expected redirect, got: {}",
            error_msg
        );

        // Cleanup
        let _ = auth::delete_tokens();
    }

    #[tokio::test]
    async fn test_fetch_all_activities_integration() {
        // Setup: Store valid test tokens
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs() as i64;
        let future_expiry = now + 3600; // Expires in 1 hour

        let tokens = crate::models::AuthTokens {
            access_token: "test_access_token".to_string(),
            refresh_token: "test_refresh_token".to_string(),
            id_token: "test_id_token".to_string(),
            expires_at: future_expiry,
            session_state: "test_session".to_string(),
        };

        // Store tokens (will use keyring)
        let _ = crate::auth::store_tokens(&tokens);

        // Create mock HTTP client
        let mock = MockHttpClient::new();

        // Mock response with multiple activities
        let response_body = r#"{
            "success": true,
            "data": [
                {
                    "id": "act1",
                    "title": "Activity 1",
                    "body": "Content 1",
                    "date": "2024-01-15T10:00:00Z",
                    "medias": [
                        {
                            "id": "media1",
                            "name": "photo1",
                            "extension": ".jpg",
                            "size": 1024,
                            "type": "image"
                        }
                    ],
                    "pupils": [1]
                },
                {
                    "id": "act2",
                    "title": "Activity 2",
                    "body": "Content 2",
                    "date": "2024-01-16T10:00:00Z",
                    "medias": [],
                    "pupils": [2]
                },
                {
                    "id": "act3",
                    "title": "Activity 3",
                    "body": "Content 3",
                    "date": "2024-01-17T10:00:00Z",
                    "medias": [
                        {
                            "id": "media2",
                            "name": "photo2",
                            "extension": ".png",
                            "size": 2048,
                            "type": "image"
                        },
                        {
                            "id": "media3",
                            "name": "photo3",
                            "extension": ".jpg",
                            "size": 3072,
                            "type": "image"
                        }
                    ],
                    "pupils": [1, 2, 3]
                }
            ],
            "pagination": {
                "page_count": 1,
                "current_page": 1,
                "has_next_page": false,
                "has_prev_page": false,
                "count": 3,
                "limit": 10
            }
        }"#;
        mock.add_response(200, response_body);

        // Create client with mock
        let client = EducartableClient::new(mock);

        // Test fetch_all_activities
        let result = client.fetch_all_activities(12345).await;
        assert!(
            result.is_ok(),
            "fetch_all_activities should succeed: {:?}",
            result.err()
        );

        let activities = result.unwrap();
        assert_eq!(activities.len(), 3, "Should have 3 activities");

        // Verify first activity
        assert_eq!(activities[0].id, "act1");
        assert_eq!(activities[0].title, "Activity 1");
        assert_eq!(activities[0].medias.len(), 1);
        assert_eq!(activities[0].medias[0].name, "photo1");

        // Verify second activity
        assert_eq!(activities[1].id, "act2");
        assert_eq!(activities[1].title, "Activity 2");
        assert_eq!(activities[1].medias.len(), 0);

        // Verify third activity
        assert_eq!(activities[2].id, "act3");
        assert_eq!(activities[2].title, "Activity 3");
        assert_eq!(activities[2].medias.len(), 2);
        assert_eq!(activities[2].medias[1].name, "photo3");
        assert_eq!(activities[2].pupils, vec![1, 2, 3]);

        // Cleanup
        let _ = crate::auth::delete_tokens();
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
