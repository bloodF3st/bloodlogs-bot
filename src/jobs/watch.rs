use std::sync::Arc;
use std::time::Duration;

use chrono::{DateTime, FixedOffset, Utc};
use sqlx::SqlitePool;
use teloxide::prelude::*;
use teloxide::types::{ChatId, ChatKind, ParseMode, UserId};
use tracing::{info, warn};

use crate::messages::{chat_link_html, escape_html, format_duration, user_link_html};
use crate::state::AppState;

fn send_ntfy(title: &str, body: &str) {
    let url = match std::env::var("NTFY_URL") {
        Ok(u) if !u.is_empty() => u,
        _ => return,
    };
    let title = title.to_string();
    let body = body.to_string();
    tokio::spawn(async move {
        tokio::task::spawn_blocking(move || {
            let _ = std::process::Command::new("curl")
                .args([
                    "-s", "-m", "5",
                    "-H", &format!("Title: {title}"),
                    "-H", "Priority: default",
                    "-H", "Tags: bell",
                    "-d", &body,
                    &url,
                ])
                .output();
        })
        .await
        .ok();
    });
}

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
    target_display: Option<String>,
    chat_display: Option<String>,
}

fn fmt_datetime_msk(dt: DateTime<Utc>) -> String {
    let msk = FixedOffset::east_opt(3 * 3600).expect("MSK +3");
    dt.with_timezone(&msk).format("%Y-%m-%d %H:%M MSK").to_string()
}

/// `<a href="tg://user?id=X">Name</a> <code>X</code>`
/// или просто `<a href="...">X</a>` если имя неизвестно.
fn user_html(id: i64, display: Option<&str>) -> String {
    match display.map(str::trim).filter(|s| !s.is_empty()) {
        Some(name) => format!(
            "<a href=\"tg://user?id={id}\">{}</a> <code>{id}</code>",
            escape_html(name)
        ),
        None => user_link_html(id),
    }
}

/// `<a href="...">Chat Name</a> <code>chat_id</code>`
/// или стандартный chat_link_html если имя неизвестно.
fn chat_html(chat_id: i64, display: Option<&str>) -> String {
    match display.map(str::trim).filter(|s| !s.is_empty()) {
        Some(title) => {
            let s = chat_id.to_string();
            let url = if let Some(rest) = s.strip_prefix("-100") {
                if let Ok(internal) = rest.parse::<i64>() {
                    format!("{HTTPS_TME_C}{internal}/1")
                } else {
                    let abs = (chat_id as i64).unsigned_abs();
                    format!("tg://openmessage?chat_id={abs}")
                }
            } else {
                let abs = (chat_id as i64).unsigned_abs();
                format!("tg://openmessage?chat_id={abs}")
            };
            format!(
                "<a href=\"{url}\">{}</a> <code>{chat_id}</code>",
                escape_html(title)
            )
        }
        None => chat_link_html(chat_id),
    }
}

/// Возвращает (target_display, chat_display).
/// Если поля уже заполнены в БД — возвращает их.
/// Иначе пробует getChatMember + getChat, сохраняет результат.
async fn resolve_display_names(
    bot: &Bot,
    pool: &SqlitePool,
    row: &WatchRow,
) -> (Option<String>, Option<String>) {
    let target = row.target_display.clone();
    let chat = row.chat_display.clone();

    let need_target = target.as_deref().map(str::trim).unwrap_or("").is_empty();
    let need_chat = chat.as_deref().map(str::trim).unwrap_or("").is_empty();

    if !need_target && !need_chat {
        return (target, chat);
    }

    let fetched_target = if need_target {
        bot.get_chat_member(ChatId(row.chat_id), UserId(row.target_user_id as u64))
            .await
            .ok()
            .map(|m| {
                let u = &m.user;
                let mut s = u.first_name.trim().to_string();
                if let Some(ln) = &u.last_name {
                    let ln = ln.trim();
                    if !ln.is_empty() {
                        s.push(' ');
                        s.push_str(ln);
                    }
                }
                s
            })
            .filter(|s| !s.is_empty())
    } else {
        target.clone()
    };

    let fetched_chat = if need_chat {
        bot.get_chat(ChatId(row.chat_id))
            .await
            .ok()
            .and_then(|c| c.title().map(str::to_string))
            .filter(|s| !s.is_empty())
    } else {
        chat.clone()
    };

    // Сохраняем что удалось получить
    if fetched_target.is_some() || fetched_chat.is_some() {
        let _ = sqlx::query(
            "UPDATE watch_timers SET target_display = COALESCE(?, target_display), \
             chat_display = COALESCE(?, chat_display) WHERE id = ?",
        )
        .bind(fetched_target.as_deref())
        .bind(fetched_chat.as_deref())
        .bind(row.id)
        .execute(pool)
        .await;
    }

    (fetched_target.or(target), fetched_chat.or(chat))
}

async fn tick(bot: &Bot, pool: &SqlitePool, admin_ids: &[i64], notify_chat_id: Option<i64>) -> anyhow::Result<()> {
    let rows: Vec<WatchRow> = tokio::time::timeout(
        Duration::from_secs(25),
        sqlx::query_as::<_, WatchRow>(
            r#"
            SELECT id, owner_user_id, target_user_id, chat_id, timeout_seconds,
                   last_message_at, last_notified_at, last_message_id, created_at,
                   target_display, chat_display
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

        // Уведомляем только один раз. Сброс происходит когда цель пишет
        // (on_message обнуляет last_notified_at).
        if row.last_notified_at.is_some() {
            continue;
        }

        // Если имена неизвестны — пробуем получить через Bot API и сохранить.
        let (target_display, chat_display) =
            resolve_display_names(bot, pool, &row).await;

        let user_h = user_html(row.target_user_id, target_display.as_deref());
        let chat_h = chat_html(row.chat_id, chat_display.as_deref());
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
            "ᴛɪᴍᴇʀ #{id}: {user_h} | {chat_h} | ɪɴᴀᴄᴛɪᴠᴇ ≥ {elapsed_fmt} (ᴛʜʀᴇsʜᴏʟᴅ {threshold_fmt}).{last_line}",
            id = row.id,
        );

        let mut notified = false;
        for &aid in admin_ids {
            match bot
                .send_message(ChatId(aid), &html)
                .parse_mode(ParseMode::Html)
                .await
            {
                Ok(_) => { notified = true; }
                Err(e) => {
                    warn!("watch #{}: failed to notify admin {aid}: {e}", row.id);
                }
            }
        }
        if notified {
            send_ntfy(
                "⏰ Таймер неактивности",
                &format!(
                    "#{}: {} ({}) | {} | inactive ≥ {}",
                    row.id,
                    target_display.as_deref().unwrap_or("?"),
                    row.target_user_id,
                    chat_display.as_deref().unwrap_or(&row.chat_id.to_string()),
                    threshold_fmt,
                ),
            );
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

        // Отправляем уведомление в notify_chat и закрепляем
        if let Some(nchat) = notify_chat_id {
            match bot
                .send_message(ChatId(nchat), &html)
                .parse_mode(ParseMode::Html)
                .await
            {
                Ok(sent) => {
                    if let Err(e) = bot.pin_chat_message(ChatId(nchat), sent.id).await {
                        warn!("watch #{}: failed to pin in notify_chat {nchat}: {e}", row.id);
                    }
                }
                Err(e) => {
                    warn!("watch #{}: failed to send to notify_chat {nchat}: {e}", row.id);
                }
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

    // Имя пользователя: first_name + last_name
    let mut user_display = u.first_name.trim().to_string();
    if let Some(ln) = &u.last_name {
        let ln = ln.trim();
        if !ln.is_empty() {
            user_display.push(' ');
            user_display.push_str(ln);
        }
    }

    // Название чата
    let chat_display = match &msg.chat.kind {
        ChatKind::Public(p) => p.title.as_deref().unwrap_or("").to_string(),
        ChatKind::Private(_) => user_display.clone(),
    };

    sqlx::query(
        r#"
        UPDATE watch_timers
        SET last_message_at  = datetime('now'),
            last_notified_at = NULL,
            updated_at       = datetime('now'),
            last_message_id  = ?,
            target_display   = ?,
            chat_display     = ?
        WHERE target_user_id = ?
          AND chat_id = ?
        "#,
    )
    .bind(msg.id.0 as i64)
    .bind(&user_display)
    .bind(&chat_display)
    .bind(target_id)
    .bind(chat_id)
    .execute(pool)
    .await
    .map_err(|e| anyhow::anyhow!("watch_timers UPDATE activity: {e}"))?;

    Ok(())
}

async fn tick_loop(bot: Bot, pool: Arc<SqlitePool>, admin_ids: Vec<i64>, notify_chat_id: Option<i64>) {
    let mut intv = tokio::time::interval(Duration::from_secs(30));
    intv.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        intv.tick().await;
        if let Err(e) = tick(&bot, pool.as_ref(), &admin_ids, notify_chat_id).await {
            warn!("watch tick: {e:#}");
        }
    }
}

async fn watch_supervised(bot: Bot, pool: Arc<SqlitePool>, admin_ids: Vec<i64>, notify_chat_id: Option<i64>) {
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
            let ids = admin_ids.clone();
            checker = Some(tokio::spawn(async move { tick_loop(b, p, ids, notify_chat_id).await }));
        }
    }
}

pub fn spawn_watch_supervisor(bot: Bot, state: &AppState) -> tokio::task::JoinHandle<()> {
    let pool = state.db.clone();
    let admin_ids = state.admin_ids().to_vec();
    let notify_chat_id = state.cfg.notify_chat_id;
    tokio::spawn(async move { watch_supervised(bot, pool, admin_ids, notify_chat_id).await })
}
