// API client for Educartable endpoints
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::fmt;
use tokio::time::sleep;
use std::time::Duration;
use crate::models::{UserInfo, ActivitiesResponse, Activity};

// Issue #23: Custom error types
#[derive(Debug)]
pub enum ApiError {
    Authentication(String),
    Network(String),
    RateLimit,
    ServerError(String),
    ParseError(String),
}

impl fmt::Display for ApiError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ApiError::Authentication(msg) => write!(f, "Authentication error: {}", msg),
            ApiError::Network(msg) => write!(f, "Network error: {}", msg),
            ApiError::RateLimit => write!(f, "Rate limit exceeded"),
            ApiError::ServerError(msg) => write!(f, "Server error: {}", msg),
            ApiError::ParseError(msg) => write!(f, "Parse error: {}", msg),
        }
    }
}

impl std::error::Error for ApiError {}

pub struct EducartableClient {
    client: Client,
    access_token: String,
}

impl EducartableClient {
    pub fn new(access_token: String) -> Self {
        log::debug!("Creating new EducartableClient");
        Self {
            client: Client::new(),
            access_token,
        }
    }

    // Issue #23: Constructor with timeout
    pub fn new_with_timeout(access_token: String, timeout_secs: u64) -> Self {
        log::debug!("Creating new EducartableClient with {}s timeout", timeout_secs);
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .unwrap();

        Self {
            client,
            access_token,
        }
    }

    // Issue #23: Retry logic with exponential backoff
    async fn get_with_retry<T: for<'de> Deserialize<'de>>(
        &self,
        url: &str,
        max_retries: u32
    ) -> Result<T, ApiError> {
        let mut attempt = 0;
        log::debug!("GET request with retry: {}", url);

        loop {
            attempt += 1;
            log::debug!("Attempt {}/{} for {}", attempt, max_retries, url);

            match self.client
                .get(url)
                .header("Authorization", &self.access_token)
                .send()
                .await
            {
                Ok(response) => {
                    let status = response.status();
                    match status.as_u16() {
                        200..=299 => {
                            log::debug!("Request successful: {} (status: {})", url, status);
                            return response.json().await
                                .map_err(|e| {
                                    log::error!("Failed to parse response JSON: {}", e);
                                    ApiError::ParseError(e.to_string())
                                });
                        }
                        401 | 403 => {
                            log::error!("Authentication failed for {}: {}", url, status);
                            return Err(ApiError::Authentication("Invalid token".to_string()));
                        }
                        429 => {
                            if attempt >= max_retries {
                                log::error!("Rate limit exceeded for {} after {} attempts", url, attempt);
                                return Err(ApiError::RateLimit);
                            }
                            let wait_time = 2_u64.pow(attempt);
                            log::warn!("Rate limited on {}, retrying in {}s", url, wait_time);
                            sleep(Duration::from_secs(wait_time)).await;
                            continue;
                        }
                        500..=599 => {
                            if attempt >= max_retries {
                                log::error!("Server error for {} after {} attempts: {}", url, attempt, status);
                                return Err(ApiError::ServerError(
                                    format!("Server error: {}", status)
                                ));
                            }
                            let wait_time = 2_u64.pow(attempt);
                            log::warn!("Server error {} on {}, retrying in {}s", status, url, wait_time);
                            sleep(Duration::from_secs(wait_time)).await;
                            continue;
                        }
                        _ => {
                            log::error!("Unexpected status {} for {}", status, url);
                            return Err(ApiError::ServerError(
                                format!("Unexpected status: {}", status)
                            ));
                        }
                    }
                }
                Err(e) => {
                    if attempt >= max_retries {
                        log::error!("Network error for {} after {} attempts: {}", url, attempt, e);
                        return Err(ApiError::Network(e.to_string()));
                    }
                    let wait_time = 2_u64.pow(attempt);
                    log::warn!("Network error on {}, retrying in {}s: {}", url, wait_time, e);
                    sleep(Duration::from_secs(wait_time)).await;
                    continue;
                }
            }
        }
    }

    async fn get<T: for<'de> Deserialize<'de>>(
        &self,
        url: &str
    ) -> Result<T, Box<dyn std::error::Error + Send + Sync>> {
        log::debug!("GET request: {}", url);

        let response = self.client
            .get(url)
            .header("Authorization", &self.access_token)  // NO "Bearer"!
            .send()
            .await?;

        let status = response.status();
        log::debug!("Response status: {}", status);

        if !status.is_success() {
            log::warn!("Non-success status {} for {}", status, url);
        }

        let data: T = response.json().await?;
        log::debug!("Response parsed successfully");
        Ok(data)
    }

    async fn post<T: for<'de> Deserialize<'de>, B: Serialize>(
        &self,
        url: &str,
        body: &B
    ) -> Result<T, Box<dyn std::error::Error + Send + Sync>> {
        log::debug!("POST request: {}", url);

        let response = self.client
            .post(url)
            .header("Authorization", &self.access_token)
            .json(body)
            .send()
            .await?;

        let status = response.status();
        log::debug!("Response status: {}", status);

        if !status.is_success() {
            log::warn!("Non-success status {} for {}", status, url);
        }

        let data: T = response.json().await?;
        log::debug!("Response parsed successfully");
        Ok(data)
    }

    // Issue #20: User info endpoint
    pub async fn get_user_info(&self) -> Result<UserInfo, Box<dyn std::error::Error + Send + Sync>> {
        log::info!("Fetching user info");
        let url = "https://app.educartable.com/api/1.0/educore/users/me?light=1";
        let result = self.get::<UserInfo>(url).await;
        match &result {
            Ok(_) => log::info!("User info fetched successfully"),
            Err(e) => log::error!("Failed to fetch user info: {}", e),
        }
        result
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
        page: u32
    ) -> Result<ActivitiesResponse, Box<dyn std::error::Error + Send + Sync>> {
        log::debug!("Fetching activities page {} for parent {}", page, parent_id);
        let url = format!(
            "https://app.educartable.com/api/1.0/educartable/parent/{}/messages?type=activity&sort=date&direction=desc&page={}",
            parent_id, page
        );
        let result = self.get::<ActivitiesResponse>(&url).await;
        match &result {
            Ok(response) => log::debug!("Fetched {} activities from page {}", response.data.len(), page),
            Err(e) => log::error!("Failed to fetch activities page {}: {}", page, e),
        }
        result
    }

    pub async fn fetch_all_activities(
        &self,
        parent_id: i64
    ) -> Result<Vec<Activity>, Box<dyn std::error::Error + Send + Sync>> {
        log::info!("Fetching all activities for parent {}", parent_id);
        let mut all_activities = Vec::new();
        let mut page = 1;

        loop {
            let response = self.get_activities(parent_id, page).await?;
            let count = response.data.len();
            all_activities.extend(response.data);

            log::debug!("Page {}: {} activities (total so far: {})", page, count, all_activities.len());

            if !response.pagination.has_next_page {
                break;
            }
            page += 1;
        }

        log::info!("Fetched {} total activities across {} pages", all_activities.len(), page);
        Ok(all_activities)
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

        // Disable automatic redirect following to capture the Location header
        let response = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()?
            .get(&url)
            .header("Authorization", &self.access_token)
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
