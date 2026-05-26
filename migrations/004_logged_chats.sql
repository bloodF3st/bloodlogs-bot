CREATE TABLE IF NOT EXISTS logged_chats (
    chat_id  INTEGER PRIMARY KEY,
    added_at DATETIME NOT NULL DEFAULT (datetime('now'))
);
