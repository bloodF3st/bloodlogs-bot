use std::sync::Arc;

use sqlx::SqlitePool;
use tokio::sync::mpsc::Sender;

use crate::config::Config;

#[derive(Clone)]
pub struct AppState {
    pub db: Arc<SqlitePool>,
    pub cfg: Arc<Config>,
    pub log_tx: Sender<String>,
}

impl AppState {
    pub fn new(db: Arc<SqlitePool>, cfg: Arc<Config>, log_tx: Sender<String>) -> Self {
        Self {
            db,
            cfg,
            log_tx,
        }
    }

    pub fn admin_ids(&self) -> &[i64] {
        &self.cfg.admin_ids
    }

    pub fn is_admin(&self, uid: i64) -> bool {
        self.cfg.is_admin(uid)
    }
}
