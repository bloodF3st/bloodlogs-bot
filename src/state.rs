use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;

use sqlx::SqlitePool;
use tokio::sync::mpsc::UnboundedSender;
use tokio::sync::Mutex;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<SqlitePool>,
    pub cfg: Arc<Config>,
    pub chat_admin_cache: Arc<Mutex<HashMap<i64, (bool, Instant)>>>,
    pub log_tx: UnboundedSender<String>,
}

impl AppState {
    pub fn new(db: Arc<SqlitePool>, cfg: Arc<Config>, log_tx: UnboundedSender<String>) -> Self {
        Self {
            db,
            cfg,
            chat_admin_cache: Arc::new(Mutex::new(HashMap::new())),
            log_tx,
        }
    }

    pub fn admin_id(&self) -> i64 {
        self.cfg.admin_id
    }
}
