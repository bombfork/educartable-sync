// API client for Educartable endpoints
use reqwest::Client;
use serde::{Deserialize, Serialize};

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
}