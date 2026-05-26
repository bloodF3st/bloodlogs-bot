CREATE TABLE IF NOT EXISTS chat_activity (
    chat_id  INTEGER NOT NULL,
    user_id  INTEGER NOT NULL,
    last_seen_at DATETIME NOT NULL DEFAULT (datetime('now')),
    PRIMARY KEY (chat_id, user_id)
);
