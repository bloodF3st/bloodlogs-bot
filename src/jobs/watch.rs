use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, FixedOffset, Utc};
use sqlx::SqlitePool;
use teloxide::prelude::*;
use teloxide::types::ParseMode;
use tracing::{info, warn};

use crate::messages::{chat_link_html, format_duration, user_link_html};
use crate::state::AppState;

const HTTPS_TME_C: &str = concat!("https", "://t.me/c/");

#[derive(sqlx::FromRow)]
struct WatchRow {
    id: i64,
    #[allow(dead_code)]
    owner_user_id: i64,
    target_user_id: i64,
    chat_id: i64,
    timeout_seconds: i64,
    last_message_at: Option<DateTime<Utc>>,
    last_notified_at: Option<DateTime<Utc>>,
    last_message_id: Option<i64>,
    created_at: DateTime<Utc>,
}

fn fmt_datetime_msk(dt: DateTime<Utc>) -> String {
    let msk = FixedOffset::east_opt(3 * 3600).expect("MSK +3");
    dt.with_timezone(&msk).format("%Y-%m-%d %H:%M MSK").to_string()
}

async fn tick(bot: &Bot, pool: &SqlitePool, admin_id: i64) -> anyhow::Result<()> {
    let rows: Vec<WatchRow> = tokio::time::timeout(
        Duration::from_secs(25),
        sqlx::query_as::<_, WatchRow>(
            r#"
            SELECT id, owner_user_id, target_user_id, chat_id, timeout_seconds,
                   last_message_at, last_notified_at, last_message_id, created_at
            FROM watch_timers
            "#,
        )
        .fetch_all(pool),
    )
    .await
    .map_err(|_| anyhow::anyhow!("watch tick: SQL exceeded 25s"))?
    .map_err(|e| anyhow::anyhow!("watch tick SELECT: {e}"))?;

    let now = Utc::now();
    for row in rows {
        let silence_start = row.last_message_at.unwrap_or(row.created_at);
        let elapsed = now.signed_duration_since(silence_start).num_seconds();

        if elapsed < row.timeout_seconds {
            continue;
        }

        if let Some(notified_at) = row.last_notified_at {
            let since_notified = now.signed_duration_since(notified_at).num_seconds();
            if since_notified < row.timeout_seconds {
                continue;
            }
        }

        let user_link = user_link_html(row.target_user_id);
        let chat_link = chat_link_html(row.chat_id);
        let elapsed_fmt = format_duration(elapsed.max(0));
        let threshold_fmt = format_duration(row.timeout_seconds);

        let last_dt = fmt_datetime_msk(row.last_message_at.unwrap_or(row.created_at));
        let last_line = match row.last_message_id {
            Some(mid) => {
                let s = row.chat_id.to_string();
                if let Some(rest) = s.strip_prefix("-100") {
                    if let Ok(internal) = rest.parse::<i64>() {
                        format!("\nʟᴀsᴛ: <a href=\"{HTTPS_TME_C}{internal}/{mid}\">{last_dt}</a>")
                    } else {
                        format!("\nʟᴀsᴛ: {last_dt}")
                    }
                } else {
                    format!("\nʟᴀsᴛ: {last_dt}")
                }
            }
            None => format!("\nʟᴀsᴛ: {last_dt}"),
        };

        let html = format!(
            "ᴛɪᴍᴇʀ: {user_link} | {chat_link} | ɪɴᴀᴄᴛɪᴠᴇ ≥ {elapsed_fmt} (ᴛʜʀᴇsʜᴏʟᴅ {threshold_fmt}).{last_line}"
        );

        match bot
            .send_message(ChatId(admin_id), &html)
            .parse_mode(ParseMode::Html)
            .await
        {
            Ok(_) => {
                if let Err(e) = sqlx::query(
                    "UPDATE watch_timers SET last_notified_at = datetime('now'), updated_at = datetime('now') WHERE id = ?",
                )
                .bind(row.id)
                .execute(pool)
                .await
                {
                    warn!("watch #{}: failed to set last_notified_at: {e}", row.id);
                }
            }
            Err(e) => {
                warn!("watch #{}: failed to notify: {e}", row.id);
            }
        }
    }
    Ok(())
}

pub async fn on_message(pool: &SqlitePool, msg: &teloxide::types::Message) -> anyhow::Result<()> {
    let Some(u) = msg.from() else {
        return Ok(());
    };
    let target_id = u.id.0 as i64;
    let chat_id = msg.chat.id.0;

    sqlx::query(
        r#"
        UPDATE watch_timers
        SET last_message_at = datetime('now'),
            last_notified_at = NULL,
            updated_at = datetime('now'),
            last_message_id = ?
        WHERE target_user_id = ?
          AND chat_id = ?
        "#,
    )
    .bind(msg.id.0 as i64)
    .bind(target_id)
    .bind(chat_id)
    .execute(pool)
    .await
    .map_err(|e| anyhow::anyhow!("watch_timers UPDATE activity: {e}"))?;

    Ok(())
}

async fn tick_loop(bot: Bot, pool: Arc<SqlitePool>, admin_id: i64) {
    let mut intv = tokio::time::interval(Duration::from_secs(30));
    intv.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        intv.tick().await;
        if let Err(e) = tick(&bot, pool.as_ref(), admin_id).await {
            warn!("watch tick: {e:#}");
        }
    }
}

async fn watch_supervised(bot: Bot, pool: Arc<SqlitePool>, admin_id: i64) {
    info!("watch: supervisor started (interval 30s, watchdog 45s)");
    let mut checker: Option<tokio::task::JoinHandle<()>> = None;
    let mut watchdog = tokio::time::interval(Duration::from_secs(45));
    watchdog.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        watchdog.tick().await;

        let task_dead = checker.as_ref().map(|h| h.is_finished()).unwrap_or(true);

        if task_dead {
            if checker.is_some() {
                warn!("watch: watchdog detected checker stopped — restarting");
            } else {
                info!("watch: watchdog starting checker");
            }
            let b = bot.clone();
            let p = pool.clone();
            checker = Some(tokio::spawn(async move { tick_loop(b, p, admin_id).await }));
        }
    }
}

pub fn spawn_watch_supervisor(bot: Bot, state: &AppState) -> tokio::task::JoinHandle<()> {
    let pool = state.db.clone();
    let admin_id = state.admin_id();
    tokio::spawn(async move { watch_supervised(bot, pool, admin_id).await })
}
