CREATE TABLE IF NOT EXISTS bot_inaccessible_chats (
    chat_id     INTEGER PRIMARY KEY,
    reason      TEXT    NOT NULL DEFAULT 'unknown',
    detected_at DATETIME NOT NULL DEFAULT (datetime('now')),
    notified_at DATETIME
);
