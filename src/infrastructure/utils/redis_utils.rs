use anyhow::Result;
use redis;
use redis::AsyncCommands;

pub async fn try_acquire_lock(
    redis_client: &redis::Client,
    user_id: &str,
    ttl_secs: usize,
) -> Result<bool> {
    let mut conn = redis_client.get_multiplexed_tokio_connection().await?;
    let key = format!("twitch_listener:{}", user_id);

    // SET key "locked" NX EX ttl_secs
    let res: Option<String> = redis::cmd("SET")
        .arg(&key)
        .arg("locked")
        .arg("NX")
        .arg("EX")
        .arg(ttl_secs)
        .query_async(&mut conn)
        .await?;

    Ok(res.is_some()) // Some -> acquired, None -> already exists
}

// helper: release lock (guarded, best-effort)
pub async fn release_lock(redis_client: &redis::Client, user_id: &str) -> Result<()> {
    let mut conn = redis_client.get_multiplexed_tokio_connection().await?;
    let key = format!("twitch_listener:{}", user_id);
    let _: () = conn.del(key).await?;
    Ok(())
}

// helper: publish stop control
pub async fn publish_stop(redis_client: &redis::Client, user_id: &str) -> Result<()> {
    let mut conn = redis_client.get_multiplexed_tokio_connection().await?;
    let channel = format!("twitch:control:{}", user_id);
    let payload = "stop";
    let _: () = redis::cmd("PUBLISH")
        .arg(&channel)
        .arg(payload)
        .query_async(&mut conn)
        .await?;
    Ok(())
}
