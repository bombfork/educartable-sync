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
