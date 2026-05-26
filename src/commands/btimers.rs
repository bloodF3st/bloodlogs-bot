use chrono::{DateTime, FixedOffset, Utc};
use teloxide::prelude::*;
use teloxide::types::{CallbackQuery, InlineKeyboardButton, InlineKeyboardMarkup, ParseMode};

use crate::messages::{chat_link_html, format_duration, send_html, user_link_html};
use crate::state::AppState;

const PAGE_SIZE: i64 = 5;
const HTTPS_TME_C: &str = concat!("https", "://t.me/c/");

#[derive(sqlx::FromRow)]
struct TimerRow {
    id: i64,
    target_user_id: i64,
    chat_id: i64,
    timeout_seconds: i64,
    last_message_at: Option<DateTime<Utc>>,
    last_message_id: Option<i64>,
    created_at: DateTime<Utc>,
}

fn fmt_msk(dt: DateTime<Utc>) -> String {
    let msk = FixedOffset::east_opt(3 * 3600).expect("MSK +3");
    dt.with_timezone(&msk).format("%d.%m.%Y %H:%M").to_string()
}

fn last_link(chat_id: i64, msg_id: i64, dt: DateTime<Utc>) -> String {
    let s = chat_id.to_string();
    if let Some(rest) = s.strip_prefix("-100") {
        if let Ok(internal) = rest.parse::<i64>() {
            return format!(
                "<a href=\"{HTTPS_TME_C}{internal}/{msg_id}\">{}</a>",
                fmt_msk(dt)
            );
        }
    }
    fmt_msk(dt)
}

async fn fetch_page(
    state: &AppState,
    owner_id: i64,
    page: i64,
) -> anyhow::Result<(Vec<TimerRow>, i64)> {
    let total: i64 =
        sqlx::query_scalar("SELECT COUNT(*) FROM watch_timers WHERE owner_user_id = ?")
            .bind(owner_id)
            .fetch_one(state.db.as_ref())
            .await?;

    let rows: Vec<TimerRow> = sqlx::query_as(
        r#"
        SELECT id, target_user_id, chat_id, timeout_seconds,
               last_message_at, last_message_id, created_at
        FROM watch_timers
        WHERE owner_user_id = ?
        ORDER BY id
        LIMIT ? OFFSET ?
        "#,
    )
    .bind(owner_id)
    .bind(PAGE_SIZE)
    .bind(page * PAGE_SIZE)
    .fetch_all(state.db.as_ref())
    .await?;

    Ok((rows, total))
}

fn build_html(rows: &[TimerRow], page: i64, total: i64) -> String {
    if rows.is_empty() {
        return "ɴᴏ ᴀᴄᴛɪᴠᴇ ᴛɪᴍᴇʀs.".to_string();
    }

    let total_pages = (total + PAGE_SIZE - 1) / PAGE_SIZE;
    let mut lines = vec![
        format!("ᴛɪᴍᴇʀs  {}/{}", page + 1, total_pages),
        String::new(),
    ];

    for r in rows {
        let user = user_link_html(r.target_user_id);
        let chat = chat_link_html(r.chat_id);
        let thr = format_duration(r.timeout_seconds);
        let dt = r.last_message_at.unwrap_or(r.created_at);
        let last = match r.last_message_id {
            Some(mid) => last_link(r.chat_id, mid, dt),
            None => fmt_msk(dt),
        };
        lines.push(format!(
            "#{id}\nᴜsᴇʀ: {user}\nᴄʜᴀᴛ: {chat}\nᴛʜʀᴇsʜᴏʟᴅ: {thr}\nʟᴀsᴛ: {last}",
            id = r.id,
        ));
        lines.push(String::new());
    }

    lines.join("\n")
}

fn build_keyboard(page: i64, total: i64) -> InlineKeyboardMarkup {
    let total_pages = (total + PAGE_SIZE - 1) / PAGE_SIZE;
    let mut row: Vec<InlineKeyboardButton> = vec![];

    if page > 0 {
        row.push(InlineKeyboardButton::callback(
            "← ɴᴀᴢᴀᴅ",
            format!("btimers:{}", page - 1),
        ));
    }
    if page + 1 < total_pages {
        row.push(InlineKeyboardButton::callback(
            "ᴠᴘᴇʀᴇᴅ →",
            format!("btimers:{}", page + 1),
        ));
    }

    if row.is_empty() {
        InlineKeyboardMarkup::new(Vec::<Vec<InlineKeyboardButton>>::new())
    } else {
        InlineKeyboardMarkup::new(vec![row])
    }
}

pub async fn handle(bot: Bot, msg: teloxide::types::Message, state: AppState) -> ResponseResult<()> {
    let owner_id = msg.from().map(|u| u.id.0 as i64).unwrap_or(0);

    let (rows, total) = match fetch_page(&state, owner_id, 0).await {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("btimers db: {e:#}");
            send_html(&bot, msg.chat.id, "ᴅʙ ᴇʀʀᴏʀ.").await;
            return Ok(());
        }
    };

    let html = build_html(&rows, 0, total);

    if total == 0 {
        send_html(&bot, msg.chat.id, &html).await;
        return Ok(());
    }

    bot.send_message(msg.chat.id, &html)
        .parse_mode(ParseMode::Html)
        .reply_markup(build_keyboard(0, total))
        .await?;

    Ok(())
}

pub async fn handle_callback(bot: Bot, q: CallbackQuery, state: AppState) -> ResponseResult<()> {
    let data = match q.data.as_deref() {
        Some(d) => d,
        None => {
            bot.answer_callback_query(&q.id).await?;
            return Ok(());
        }
    };
    let page: i64 = match data.strip_prefix("btimers:").and_then(|s| s.parse().ok()) {
        Some(p) => p,
        None => {
            bot.answer_callback_query(&q.id).await?;
            return Ok(());
        }
    };

    bot.answer_callback_query(&q.id).await?;

    let Some(msg) = q.message else {
        return Ok(());
    };

    let owner_id = q.from.id.0 as i64;
    let (rows, total) = match fetch_page(&state, owner_id, page).await {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("btimers callback db: {e:#}");
            return Ok(());
        }
    };

    bot.edit_message_text(msg.chat.id, msg.id, build_html(&rows, page, total))
        .parse_mode(ParseMode::Html)
        .reply_markup(build_keyboard(page, total))
        .await?;

    Ok(())
}
