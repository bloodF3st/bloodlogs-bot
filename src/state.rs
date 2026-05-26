use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use sqlx::SqlitePool;
use tokio::sync::Mutex;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<SqlitePool>,
    pub cfg: Arc<Config>,
    pub log_cooldown: Arc<Mutex<HashMap<i64, Instant>>>,
    pub chat_admin_cache: Arc<Mutex<HashMap<i64, (bool, Instant)>>>,
}

impl AppState {
    pub fn new(db: Arc<SqlitePool>, cfg: Arc<Config>) -> Self {
        Self {
            db,
            cfg,
            log_cooldown: Arc::new(Mutex::new(HashMap::new())),
            chat_admin_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn admin_id(&self) -> i64 {
        self.cfg.admin_id
    }
}
