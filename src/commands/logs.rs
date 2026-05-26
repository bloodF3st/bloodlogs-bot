use teloxide::prelude::*;
use teloxide::types::Message;

use crate::messages::{chat_link_html, format_duration, send_html, user_link_html};
use crate::state::AppState;

pub async fn handle(bot: Bot, msg: Message, state: AppState) -> ResponseResult<()> {
    let owner_id = msg.from().map(|u| u.id.0 as i64).unwrap_or(0);

    #[derive(sqlx::FromRow)]
    struct TimerRow {
        id: i64,
        target_user_id: i64,
        chat_id: i64,
        timeout_seconds: i64,
    }

    let rows: Vec<TimerRow> = match sqlx::query_as(
        "SELECT id, target_user_id, chat_id, timeout_seconds
         FROM watch_timers
         WHERE owner_user_id = ?
         ORDER BY id",
    )
    .bind(owner_id)
    .fetch_all(state.db.as_ref())
    .await
    {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("logs timer db: {e:#}");
            send_html(&bot, msg.chat.id, "ᴅʙ ᴇʀʀᴏʀ.").await;
            return Ok(());
        }
    };

    if rows.is_empty() {
        send_html(&bot, msg.chat.id, "ɴᴏ ᴀᴄᴛɪᴠᴇ ᴛɪᴍᴇʀs.").await;
        return Ok(());
    }

    let mut lines = vec!["ᴛɪᴍᴇʀs".to_string(), String::new()];
    for r in &rows {
        let chat = chat_link_html(r.chat_id);
        let user = user_link_html(r.target_user_id);
        let thr = format_duration(r.timeout_seconds);
        lines.push(format!("#{} [{chat}] [{user}] [{thr}]", r.id));
    }
    send_html(&bot, msg.chat.id, &lines.join("\n")).await;
    Ok(())
}
