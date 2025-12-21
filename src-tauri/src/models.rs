use serde::{Deserialize, Serialize};

// Data models

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuthTokens {
    pub access_token: String,
    pub refresh_token: String,
    pub id_token: String,
    pub expires_at: i64,
    pub session_state: String,
}
