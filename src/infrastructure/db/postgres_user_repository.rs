use crate::application::ports::user_repository::UserRepository;
use crate::domain::models::twitch_account::TwitchAccount;
use sqlx::{Pool, Postgres};

pub struct PostgresUserRepository {
    pub pool: Pool<Postgres>,
}

impl PostgresUserRepository {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl UserRepository for PostgresUserRepository {
    async fn find_by_id(&self, id: &str) -> anyhow::Result<Option<TwitchAccount>> {
        let row = sqlx::query_as!(
            TwitchAccount,
            r#"
    SELECT u.name as username,
           a."accessToken" as access_token,
           a."refreshToken" as refresh_token,
           a."accessTokenExpiresAt" as access_token_expires_at,
           a."userId" as id,
           a."accountId" as account_id
    FROM "user" u
    JOIN "account" a ON u.id = a."userId"
    WHERE a."providerId" = 'twitch'
      AND u.id = $1
    "#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row)
    }
}
