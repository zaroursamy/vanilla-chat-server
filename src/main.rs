use rdkafka::{
    ClientConfig,
    producer::{FutureProducer, FutureRecord, future_producer},
};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

use axum::{
    Router,
    http::HeaderValue,
    routing::{any, get, post},
};
use dotenv::dotenv;
use reqwest::{
    Method,
    header::{ACCEPT, AUTHORIZATION, CONTENT_TYPE},
};
use sqlx::PgPool;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::info;
use vanilla_chat_server::{
    self,
    api::{
        health_checker_handler,
        state::{AppState, WsSender},
        ws::ws_twitch_handler::ws_handler,
    },
    application::commands::start_twitch::start_twitch,
    infrastructure::db::postgres_user_repository::PostgresUserRepository,
};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_target(true)
        .with_level(true)
        //.with_thread_ids(true)
        .with_line_number(true)
        //.with_thread_names(true)
        .init();
    let trace_layer = TraceLayer::new_for_http()
        .make_span_with(tower_http::trace::DefaultMakeSpan::new().level(tracing::Level::INFO))
        .on_response(tower_http::trace::DefaultOnResponse::new().level(tracing::Level::INFO))
        .on_request(tower_http::trace::DefaultOnRequest::new().level(tracing::Level::INFO));

    dotenv().ok();

    let cors = CorsLayer::new()
        .allow_origin("http://localhost:3000".parse::<HeaderValue>().unwrap())
        .allow_methods([Method::GET, Method::POST, Method::PATCH, Method::DELETE])
        .allow_credentials(true)
        .allow_headers([AUTHORIZATION, ACCEPT, CONTENT_TYPE]);

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPool::connect(&database_url).await?;

    let redis_client = redis::Client::open("redis://127.0.0.1/")?;

    let brokers = std::env::var("KAFKA_BROKERS").expect("DATABASE_URL must be set");
    let kafka_producer = ClientConfig::new()
        .set("bootstrap.servers", &*brokers)
        .set("message.timeout.ms", "5000")
        .create()
        .expect("Producer creation error");

    let app_state = AppState {
        user_repo: Arc::new(PostgresUserRepository { pool }),
        redis_client,
        kafka_producer,
        ws_hub: Arc::new(tokio::sync::RwLock::new(HashMap::<String, WsSender>::new())),
    };

    let router = Router::new()
        .route("/healthchecker", get(health_checker_handler))
        .route("/start-twitch", post(start_twitch))
        .route("/ws", any(ws_handler))
        .with_state(app_state);

    let app = router.layer(cors).layer(trace_layer);

    info!("🚀 Server started successfully");

    let listener = tokio::net::TcpListener::bind("0.0.0.0:8000").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}
