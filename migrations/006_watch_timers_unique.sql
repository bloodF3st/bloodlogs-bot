-- Change UNIQUE constraint on watch_timers from (owner_user_id, target_user_id, chat_id)
-- to (target_user_id, chat_id) so that no two timers can watch the same target in the same chat,
-- regardless of who created them.
--
-- SQLite does not support DROP CONSTRAINT, so we recreate the table.

CREATE TABLE watch_timers_new (
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
    UNIQUE(target_user_id, chat_id)
);

-- Copy rows; if duplicates exist, keep the one with the most recent updated_at
INSERT INTO watch_timers_new
    (id, owner_user_id, target_user_id, chat_id, timeout_seconds,
     last_message_at, last_notified_at, target_display, chat_display,
     last_message_id, created_at, updated_at)
SELECT id, owner_user_id, target_user_id, chat_id, timeout_seconds,
       last_message_at, last_notified_at, target_display, chat_display,
       last_message_id, created_at, updated_at
FROM watch_timers
WHERE rowid IN (
    SELECT rowid FROM watch_timers w1
    WHERE updated_at = (
        SELECT MAX(updated_at) FROM watch_timers w2
        WHERE w2.target_user_id = w1.target_user_id AND w2.chat_id = w1.chat_id
    )
);

DROP TABLE watch_timers;
ALTER TABLE watch_timers_new RENAME TO watch_timers;
