CREATE TABLE IF NOT EXISTS log_jobs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    owner_user_id INTEGER NOT NULL,
    chat_id INTEGER NOT NULL,
    chat_display TEXT,
    created_at DATETIME DEFAULT (datetime('now')),
    UNIQUE(owner_user_id, chat_id)
);

CREATE TABLE IF NOT EXISTS watch_timers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    owner_user_id INTEGER NOT NULL,
    target_user_id INTEGER NOT NULL,
    chat_id INTEGER NOT NULL,
    timeout_seconds INTEGER NOT NULL,
    last_message_at DATETIME,
    last_notified_at DATETIME,
    target_display TEXT,
    chat_display TEXT,
    last_message_id INTEGER,
    created_at DATETIME DEFAULT (datetime('now')),
    updated_at DATETIME DEFAULT (datetime('now')),
    UNIQUE(owner_user_id, target_user_id, chat_id)
);
