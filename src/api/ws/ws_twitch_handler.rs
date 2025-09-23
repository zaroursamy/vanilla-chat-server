use axum::{
    extract::ws::{Message, WebSocket, WebSocketUpgrade},
    response::IntoResponse,
};

pub async fn twitch_ws_handler(ws: WebSocketUpgrade) -> impl IntoResponse {
    ws.on_upgrade(handle_socket)
}

async fn handle_socket(mut socket: WebSocket) {
    use tokio::time::{Duration, sleep};
    loop {
        sleep(Duration::from_secs(5)).await;
        if socket.send(Message::Text("ping".into())).await.is_err() {
            break;
        }
    }
}
