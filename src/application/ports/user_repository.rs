use crate::domain::models::twitch_account::TwitchAccount;

#[async_trait::async_trait]
pub trait UserRepository: Send + Sync + 'static {
    async fn find_by_id(&self, id: &str) -> anyhow::Result<Option<TwitchAccount>>;
}
