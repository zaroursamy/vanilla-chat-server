use std::time::Duration;

use crate::{
    api::{dto::start_twitch_request::StartTwitchRequest, state::AppState},
    domain::models::twitch_account::{self, TwitchAccount},
    infrastructure::utils::redis_utils::{release_lock, try_acquire_lock},
};
use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
};
use redis::AsyncCommands;

use futures_util::{SinkExt, StreamExt};
use rdkafka::producer::{FutureProducer, FutureRecord};
use serde_json::json;
use tokio_tungstenite::connect_async;
use tracing::{error, info};
/*
pub async fn start_listening_chat(twitch_account: &TwitchAccount) -> anyhow::Result<()> {
    let oauth_token = twitch_account
        .access_token
        .as_ref()
        .expect("OAuth token should exist");

    let user_id = &twitch_account.account_id; // broadcaster_user_id
    let client_id = std::env::var("TWITCH_CLIENT_ID").expect("TWITCH_CLIENT_ID should be set");

    // 1️⃣ Connexion WS à EventSub
    let ws_url = std::env::var("TWITCH_EVENTSUB_WEBSOCKET_URL")
        .expect("TWITCH_EVENTSUB_WEBSOCKET_URL should be set");
    info!("Connection to to EventSub WS with ws_url {}", ws_url);

    let (mut ws_stream, _) = connect_async(ws_url).await?;
    info!("Connected to to EventSub WS ");

    // 2️⃣ Lire le premier message "session_welcome"
    let session_id = loop {
        let msg = ws_stream
            .next()
            .await
            .ok_or_else(|| anyhow::anyhow!("No message received"))??;

        match msg {
            tokio_tungstenite::tungstenite::Message::Text(msg_text) => {
                let welcome: serde_json::Value = serde_json::from_str(&msg_text)?;
                if let Some(session_id) = welcome["payload"]["session"]["id"].as_str() {
                    info!("Received session ID: {}", session_id);
                    break session_id.to_string();
                }
            }
            tokio_tungstenite::tungstenite::Message::Ping(payload) => {
                info!("Received ping, sending pong");
                ws_stream
                    .send(tokio_tungstenite::tungstenite::Message::Pong(payload))
                    .await?;
            }
            tokio_tungstenite::tungstenite::Message::Pong(_) => {
                info!("Received pong");
            }
            tokio_tungstenite::tungstenite::Message::Close(_) => {
                return Err(anyhow::anyhow!("Connection closed"));
            }
            other => {
                info!("Other message received during handshake: {:?}", other);
            }
        }
    };

    // 3️⃣ Créer la souscription "channel.chat.message"
    let client = reqwest::Client::new();
    let body = json!({
        "type": "channel.chat.message",
        "version": "1",
        "condition": { "broadcaster_user_id": user_id, "user_id": user_id },
        "transport": {
            "method": "websocket",
            "session_id": session_id
        }
    });

    let res = client
        .post("https://api.twitch.tv/helix/eventsub/subscriptions")
        .header("Authorization", format!("Bearer {}", oauth_token))
        .header("Client-Id", client_id)
        .json(&body)
        .send()
        .await?;

    if !res.status().is_success() {
        let text = res.text().await?;
        anyhow::bail!("Failed to subscribe: {}", text);
    }
    info!("Subscribed to channel.chat.message");

    // 4️⃣ Lire les messages du chat
    while let Some(msg_result) = ws_stream.next().await {
        let msg = msg_result?;

        match msg {
            tokio_tungstenite::tungstenite::Message::Text(txt) => {
                let v: serde_json::Value = serde_json::from_str(&txt)?;

                match v["metadata"]["message_type"].as_str() {
                    Some("notification") => {
                        info!("Chat message received: {:?}", v["payload"]["event"]);
                        // Ici vous pouvez traiter le message de chat
                    }
                    Some("session_keepalive") => {
                        info!("Received keepalive message");
                    }
                    Some("session_reconnect") => {
                        info!("Received reconnect message - should reconnect");
                        // Vous devriez implémenter la logique de reconnexion ici
                        break;
                    }
                    Some("revocation") => {
                        info!("Subscription revoked");
                        break;
                    }
                    other => {
                        info!("Unknown message type: {:?}", other);
                    }
                }
            }
            tokio_tungstenite::tungstenite::Message::Ping(payload) => {
                info!("Received ping, sending pong");
                ws_stream
                    .send(tokio_tungstenite::tungstenite::Message::Pong(payload))
                    .await?;
            }
            tokio_tungstenite::tungstenite::Message::Pong(_) => {
                info!("Received pong");
            }
            tokio_tungstenite::tungstenite::Message::Close(close_frame) => {
                info!("Connection closed: {:?}", close_frame);
                break;
            }
            tokio_tungstenite::tungstenite::Message::Binary(data) => {
                info!("Received binary data: {} bytes", data.len());
            }
            tokio_tungstenite::tungstenite::Message::Frame(f) => {
                info!("Received frame: {:?}", f)
            }
        }
    }

    Ok(())
}

#[axum_macros::debug_handler]
pub async fn start_twitch(
    State(state): State<AppState>,
    Json(payload): Json<StartTwitchRequest>,
) -> impl IntoResponse {
    info!(
        "Received start-twitch request for user: {}",
        payload.user_id
    );

    let twitch_account = state.user_repo.find_by_id(&payload.user_id).await;

    match twitch_account {
        Ok(Some(account)) => {
            let account_cloned = account.clone();
            tokio::spawn(async move {
                if let Err(e) = start_listening_chat(&account_cloned).await {
                    eprintln!("Twitch listener failed: {:?}", e);
                }
            });
            let response = json!({
                "status": "success",
                "message": format!("Started listening Twitch chat for user_id {}", payload.user_id),
                "data": &account.clone()
            });

            return (StatusCode::OK, axum::Json(response));
        }
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                axum::Json(json!({
                    "status": "error",
                    "message": format!("Twitch account not found for user{} ", &payload.user_id)
                })),
            );
        }
        Err(error) => {
            log::error!(
                "Unexpected error while retreiving twitch account: {} ",
                error.root_cause().to_string()
            );
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(json!({
                    "status": "error",
                    "message": "Unexpected error while retreiving twitch account"
                })),
            );
        }
    }
} */
// The long-running listener task
async fn run_listener_for_user(
    redis_client: redis::Client,
    kafka_producer: FutureProducer,
    twitch_account: TwitchAccount,
) -> anyhow::Result<()> {
    let oauth_token = twitch_account.access_token.as_ref().expect("no token");
    let user_id = twitch_account.account_id.clone().unwrap_or_default();
    let client_id = std::env::var("TWITCH_CLIENT_ID").expect("TWITCH_CLIENT_ID should be set");

    // Connect to EventSub WS
    let ws_url = std::env::var("TWITCH_EVENTSUB_WEBSOCKET_URL")
        .unwrap_or_else(|_| "wss://eventsub.wss.twitch.tv/ws".to_string());
    let (mut ws_stream, _) = connect_async(&ws_url).await?;
    info!("Connected EventSub WS for {}", user_id);

    // Read session welcome and extract session_id
    let session_id = loop {
        let msg = ws_stream
            .next()
            .await
            .ok_or_else(|| anyhow::anyhow!("No message received"))??;

        match msg {
            tokio_tungstenite::tungstenite::Message::Text(msg_text) => {
                let welcome: serde_json::Value = serde_json::from_str(&msg_text)?;
                if let Some(session_id) = welcome["payload"]["session"]["id"].as_str() {
                    info!("Received session ID: {}", session_id);
                    break session_id.to_string();
                }
            }
            tokio_tungstenite::tungstenite::Message::Ping(payload) => {
                info!("Received ping, sending pong");
                ws_stream
                    .send(tokio_tungstenite::tungstenite::Message::Pong(payload))
                    .await?;
            }
            tokio_tungstenite::tungstenite::Message::Pong(_) => {
                info!("Received pong");
            }
            tokio_tungstenite::tungstenite::Message::Close(_) => {
                return Err(anyhow::anyhow!("Connection closed"));
            }
            other => {
                info!("Other message received during handshake: {:?}", other);
            }
        }
    };

    // Create subscription via Helix API
    let client = reqwest::Client::new();
    let body = json!({
        "type": "channel.chat.message",
        "version": "1",
        "condition": { "broadcaster_user_id": user_id, "user_id": user_id },
        "transport": {
            "method": "websocket",
            "session_id": session_id
        }
    });

    let res = client
        .post("https://api.twitch.tv/helix/eventsub/subscriptions")
        .header("Authorization", format!("Bearer {}", oauth_token))
        .header("Client-Id", client_id)
        .json(&body)
        .send()
        .await?;

    if !res.status().is_success() {
        let text = res.text().await.unwrap_or_default();
        error!("Failed subscribe: {}", text);
        let _ = release_lock(&redis_client, &user_id).await;
        anyhow::bail!("subscribe failed");
    } else {
        info!(
            "Successfully subscribed to channel.chat.message: {:?}",
            res.text().await
        );
    }

    // Prepare Redis pubsub receiver for control channel
    let channel = format!("twitch:control:{}", user_id);
    let mut pubsub = redis_client.get_async_pubsub().await?;
    pubsub.subscribe(&channel).await?;

    // Convert pubsub into a stream
    let mut pubsub_stream = pubsub.into_on_message();

    // Main loop: select! between ws_stream and pubsub messages
    loop {
        tokio::select! {

            // Message from Twitch
            msg = ws_stream.next() => {
                match msg {
                    Some(Ok(tokio_tungstenite::tungstenite::Message::Text(txt))) => {
                        let v: serde_json::Value = serde_json::from_str(&txt)?;

                        match v["metadata"]["message_type"].as_str() {
                            Some("notification") => {
                                info!("Chat message received: {:?}", v["payload"]["event"]);
                                // Ici vous pouvez traiter le message de chat
                                // Publish into Kafka
                                let payload = serde_json::to_string(&v["payload"]["event"])?;
                                let topic = "twitch.raw.chat";
                                let record = FutureRecord::to(topic)
                                    .payload(&payload)
                                    .key(&user_id);

                                let produce_future = kafka_producer.send(record, Duration::from_secs(0));
                                if let Err((e, _)) = produce_future.await {
                                    error!("Kafka publish error: {:?}", e);
                                }
                            }
                            Some("session_keepalive") => {
                                info!("Received keepalive message");
                            }
                            Some("session_reconnect") => {
                                info!("Received reconnect message - should reconnect");
                                // Vous devriez implémenter la logique de reconnexion ici
                                break;
                            }
                            Some("revocation") => {
                                info!("Subscription revoked");
                                break;
                            }
                            other => {
                                info!("Unknown message type: {:?}", other);
                            }
                        }

                    }
                    Some(Ok(tokio_tungstenite::tungstenite::Message::Ping(p))) => {
                        ws_stream.send(tokio_tungstenite::tungstenite::Message::Pong(p)).await?;
                    }
                    Some(Ok(tokio_tungstenite::tungstenite::Message::Close(_))) => {
                        info!("EventSub closed for {}", user_id);
                        break;
                    }
                    Some(Err(e)) => {
                        error!("WS read error: {:?}", e);
                        break;
                    }
                    None => {
                        info!("WS stream ended");
                        break;
                    }
                    _ => {}
                }
            }

            // Redis pubsub message (control) - now using next() on the stream
            maybe_msg = pubsub_stream.next() => {
                match maybe_msg {
                    Some(msg) => {
                        let payload: String = msg.get_payload()?;
                        info!("control message for {} -> {}", user_id, payload);
                        if payload == "stop" {
                            info!("Stop requested via Redis for {}", user_id);
                            break;
                        }
                        // you can implement other commands: refresh_subscriptions, etc.
                    }
                    None => {
                        info!("Redis pubsub stream ended");
                        break;
                    }
                }
            }
        }
    }

    // Cleanup: release lock
    let _ = release_lock(&redis_client, &user_id).await;
    info!("Listener stopped and lock released for {:?}", user_id);

    Ok(())
}

// Handler: POST /start-twitch
pub async fn start_twitch(
    State(state): State<AppState>,
    Json(payload): Json<StartTwitchRequest>,
) -> impl IntoResponse {
    let user_id = payload.user_id.clone();

    // try acquire lock (ttl 60s)
    match try_acquire_lock(&state.redis_client, &user_id, 60).await {
        Ok(true) => {
            // we got the lock: spawn the task that will keep running until it receives stop
            let twitch_account = state.user_repo.find_by_id(&payload.user_id).await;

            match twitch_account {
                Ok(Some(account)) => {
                    info!("Twitch account found {:?}", account);
                    let redis_client = state.redis_client.clone();
                    let producer = state.kafka_producer.clone();

                    tokio::spawn(async move {
                        // Clone before move inside run_listener_for_user
                        let redis_client_for_cleanup = redis_client.clone();
                        let user_id = account.account_id.clone().unwrap_or_default();

                        if let Err(e) = run_listener_for_user(redis_client, producer, account).await
                        {
                            error!("listener error: {:?}", e);
                            // ensure lock removal in case of fatal error
                            let _ = release_lock(&redis_client_for_cleanup, &user_id).await;
                        }
                    });

                    (
                        axum::http::StatusCode::OK,
                        Json(json!({"status":"started"})),
                    )
                }
                _ => (
                    axum::http::StatusCode::NOT_FOUND,
                    Json(json!({"message":"account not found"})),
                ),
            }
        }
        Ok(false) => {
            // already someone listening
            (
                axum::http::StatusCode::OK,
                Json(json!({"status":"already_listening"})),
            )
        }
        Err(e) => {
            error!("redis error: {:?}", e);
            (
                axum::http::StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({"error":"redis error"})),
            )
        }
        _ => {
            error!("Impossible to acquire lock");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                axum::Json(json!({"message": "Internal error retriveing user from Redis"})),
            )
        }
    }
}
