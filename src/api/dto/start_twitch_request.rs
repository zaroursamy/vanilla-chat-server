use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct StartTwitchRequest {
    #[serde(rename = "userId")]
    pub user_id: String,
}
