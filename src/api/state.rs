use std::sync::Arc;

use rdkafka::producer::FutureProducer;

use crate::application::ports::user_repository::UserRepository;

#[derive(Clone)]
pub struct AppState {
    pub user_repo: Arc<dyn UserRepository + Send + Sync + 'static>,
    pub redis_client: redis::Client,
    pub kafka_producer: FutureProducer,
}
