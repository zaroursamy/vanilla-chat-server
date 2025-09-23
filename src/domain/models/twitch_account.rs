// domain/models/user.rs
use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TwitchAccount {
    pub id: String,
    pub username: String,
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    pub access_token_expires_at: Option<NaiveDateTime>,
    pub account_id: Option<String>,
}
