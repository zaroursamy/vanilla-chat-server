use std::{collections::HashMap, sync::Arc};

use rdkafka::producer::FutureProducer;

use crate::application::ports::user_repository::UserRepository;

pub type WsSender = tokio::sync::broadcast::Sender<String>;

#[derive(Clone)]
pub struct AppState {
    pub user_repo: Arc<dyn UserRepository + Send + Sync + 'static>,
    pub redis_client: redis::Client,
    pub kafka_producer: FutureProducer,
    pub ws_hub: Arc<tokio::sync::RwLock<HashMap<String, WsSender>>>,
}
