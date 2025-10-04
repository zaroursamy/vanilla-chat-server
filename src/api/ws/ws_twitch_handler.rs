// Dans ton fichier de routes (ex: handlers/ws.rs)
use axum::{
    extract::{
        Query, State,
        ws::{WebSocket, WebSocketUpgrade},
    },
    response::IntoResponse,
};
use serde::Deserialize;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use tracing::{error, info};

use crate::api::state::AppState;
use axum::extract::ws::Message;
use futures_util::{SinkExt, StreamExt};

async fn handle_socket(socket: WebSocket, broadcaster_id: String, state: AppState) {
    info!(
        "We are in handle socket with broadcaster_id {}",
        broadcaster_id
    );
    // Étape 1 : récupérer Receiver depuis le hub
    let mut receiver = {
        let hub = state.ws_hub.clone();
        let mut hub_guard = hub.write().await;

        let sender = hub_guard
            .entry(broadcaster_id.clone())
            .or_insert_with(|| broadcast::channel(32).0)
            .clone();

        sender.subscribe()
    };

    // Étape 2 : split WebSocket en sender/receiver
    let (mut ws_sender, mut ws_receiver) = socket.split();

    // Étape 3 : tâche qui ENVOIE vers le frontend
    let send_task = tokio::spawn(async move {
        while let Ok(msg) = receiver.recv().await {
            info!("Try to send message to front-end {:?}", msg);

            if ws_sender
                .send(Message::text(msg.to_string()))
                .await
                .is_err()
            {
                error!("Fail to send chat message to the client which is potentially disconnected");
                break; // client déconnecté
            }
        }
    });

    // Étape 4 : tâche qui LIT depuis le frontend (et ignore le reste)
    let recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = ws_receiver.next().await {
            match msg {
                Message::Close(_) => break,
                Message::Text(_) => {
                    // éventuellement traiter les messages entrants
                }
                _ => {}
            }
        }

        /* // Nettoyage éventuel
        let mut hub = state.ws_hub.write().await;
        hub.remove(&broadcaster_id); */
        // Nettoyage éventuel
        let mut hub = state.ws_hub.write().await;
        if let Some(sender) = hub.get(&broadcaster_id) {
            if sender.receiver_count() == 0 {
                hub.remove(&broadcaster_id);
                tracing::info!(
                    "Removed broadcaster {} from hub (no more receivers)",
                    broadcaster_id
                );
            }
        }
    });

    // Étape 5 : attendre qu’une des deux tâches termine
    tokio::select! {
        _ = send_task => {}
        _ = recv_task => {}
    }
}

#[derive(Deserialize)]
pub struct WsQuery {
    pub broadcaster_id: String,
}

// Route : GET /ws?broadcaster_id=123
pub async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<WsQuery>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    info!(
        "We are in ws_handler with broadcaster_id {}",
        query.broadcaster_id
    );

    // Upgrade la connexion HTTP → WebSocket
    ws.on_upgrade(move |socket| handle_socket(socket, query.broadcaster_id, state))
}
