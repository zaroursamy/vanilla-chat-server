use serde::Deserialize;

#[derive(Deserialize)]
pub struct StopTwitchRequest {
    pub user_id: String,
}
