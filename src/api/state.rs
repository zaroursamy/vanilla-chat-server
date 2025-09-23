use std::sync::Arc;

use crate::application::ports::user_repository::UserRepository;

#[derive(Clone)]
pub struct AppState {
    pub user_repo: Arc<dyn UserRepository + Send + Sync + 'static>,
}
