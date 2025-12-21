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
        Self {
            client: Client::new(),
            access_token,
        }
    }

    // Issue #23: Constructor with timeout
    pub fn new_with_timeout(access_token: String, timeout_secs: u64) -> Self {
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

        loop {
            attempt += 1;

            match self.client
                .get(url)
                .header("Authorization", &self.access_token)
                .send()
                .await
            {
                Ok(response) => {
                    match response.status().as_u16() {
                        200..=299 => {
                            return response.json().await
                                .map_err(|e| ApiError::ParseError(e.to_string()));
                        }
                        401 | 403 => {
                            return Err(ApiError::Authentication("Invalid token".to_string()));
                        }
                        429 => {
                            if attempt >= max_retries {
                                return Err(ApiError::RateLimit);
                            }
                            sleep(Duration::from_secs(2_u64.pow(attempt))).await;
                            continue;
                        }
                        500..=599 => {
                            if attempt >= max_retries {
                                return Err(ApiError::ServerError(
                                    format!("Server error: {}", response.status())
                                ));
                            }
                            sleep(Duration::from_secs(2_u64.pow(attempt))).await;
                            continue;
                        }
                        _ => {
                            return Err(ApiError::ServerError(
                                format!("Unexpected status: {}", response.status())
                            ));
                        }
                    }
                }
                Err(e) => {
                    if attempt >= max_retries {
                        return Err(ApiError::Network(e.to_string()));
                    }
                    sleep(Duration::from_secs(2_u64.pow(attempt))).await;
                    continue;
                }
            }
        }
    }

    async fn get<T: for<'de> Deserialize<'de>>(
        &self,
        url: &str
    ) -> Result<T, Box<dyn std::error::Error>> {
        let response = self.client
            .get(url)
            .header("Authorization", &self.access_token)  // NO "Bearer"!
            .send()
            .await?;

        let data: T = response.json().await?;
        Ok(data)
    }

    async fn post<T: for<'de> Deserialize<'de>, B: Serialize>(
        &self,
        url: &str,
        body: &B
    ) -> Result<T, Box<dyn std::error::Error>> {
        let response = self.client
            .post(url)
            .header("Authorization", &self.access_token)
            .json(body)
            .send()
            .await?;

        let data: T = response.json().await?;
        Ok(data)
    }

    // Issue #20: User info endpoint
    pub async fn get_user_info(&self) -> Result<UserInfo, Box<dyn std::error::Error>> {
        let url = "https://app.educartable.com/api/1.0/educore/users/me?light=1";
        self.get::<UserInfo>(url).await
    }

    pub async fn get_parent_id(&self) -> Result<i64, Box<dyn std::error::Error>> {
        let user_info = self.get_user_info().await?;
        Ok(user_info.id)
    }

    // Issue #21: Activities pagination
    pub async fn get_activities(
        &self,
        parent_id: i64,
        page: u32
    ) -> Result<ActivitiesResponse, Box<dyn std::error::Error>> {
        let url = format!(
            "https://app.educartable.com/api/1.0/educartable/parent/{}/messages?type=activity&sort=date&direction=desc&page={}",
            parent_id, page
        );
        self.get::<ActivitiesResponse>(&url).await
    }

    pub async fn fetch_all_activities(
        &self,
        parent_id: i64
    ) -> Result<Vec<Activity>, Box<dyn std::error::Error>> {
        let mut all_activities = Vec::new();
        let mut page = 1;

        loop {
            let response = self.get_activities(parent_id, page).await?;
            all_activities.extend(response.data);

            if !response.pagination.has_next_page {
                break;
            }
            page += 1;
        }

        Ok(all_activities)
    }

    // Issue #22: Signed URL retrieval
    pub async fn get_signed_media_url(
        &self,
        media_id: &str,
        filename: &str
    ) -> Result<String, Box<dyn std::error::Error>> {
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

        // Extract Location header from 302 redirect
        if response.status().is_redirection() {
            let location = response.headers()
                .get("Location")
                .ok_or("No Location header in redirect")?
                .to_str()?
                .to_string();

            Ok(location)
        } else {
            Err(format!("Expected redirect response, got status: {}", response.status()).into())
        }
    }
}
