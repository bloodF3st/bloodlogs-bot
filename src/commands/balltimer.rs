use teloxide::prelude::*;
use teloxide::types::Message;

use crate::messages::{chat_link_html, format_duration, parse_duration, send_html};
use crate::state::AppState;

pub async fn handle(bot: Bot, msg: Message, args: String, state: AppState) -> ResponseResult<()> {
    let parts: Vec<&str> = args.split_whitespace().collect();
    if parts.len() < 2 {
        send_html(
            &bot,
            msg.chat.id,
            "ᴜsᴀɢᴇ: /balltimer &lt;lookback&gt; &lt;threshold&gt;\nexample: /balltimer 55m 1h",
        )
        .await;
        return Ok(());
    }

    let lookback_seconds = match parse_duration(parts[0]) {
        Ok(s) => s,
        Err(_) => {
            send_html(
                &bot,
                msg.chat.id,
                "ᴜsᴀɢᴇ: /balltimer &lt;lookback&gt; &lt;threshold&gt;\nexample: /balltimer 55m 1h",
            )
            .await;
            return Ok(());
        }
    };

    let timeout_seconds = match parse_duration(parts[1]) {
        Ok(s) => s,
        Err(_) => {
            send_html(
                &bot,
                msg.chat.id,
                "ᴜsᴀɢᴇ: /balltimer &lt;lookback&gt; &lt;threshold&gt;\nexample: /balltimer 55m 1h",
            )
            .await;
            return Ok(());
        }
    };

    let owner_id = msg.from().map(|u| u.id.0 as i64).unwrap_or(0);
    let chat_id = msg.chat.id.0;

    let active_users: Vec<(i64,)> = match sqlx::query_as(
        r#"
        SELECT user_id FROM chat_activity
        WHERE chat_id = ?
          AND last_seen_at >= datetime('now', ? || ' seconds')
        "#,
    )
    .bind(chat_id)
    .bind(format!("-{lookback_seconds}"))
    .fetch_all(state.db.as_ref())
    .await
    {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!("balltimer chat_activity query: {e:#}");
            send_html(&bot, msg.chat.id, "ᴅʙ ᴇʀʀᴏʀ.").await;
            return Ok(());
        }
    };

    if active_users.is_empty() {
        let lookback_fmt = format_duration(lookback_seconds);
        send_html(
            &bot,
            msg.chat.id,
            &format!("ɴᴏ ᴀᴄᴛɪᴠᴇ ᴜsᴇʀs ɪɴ ʟᴀsᴛ {lookback_fmt}."),
        )
        .await;
        return Ok(());
    }

    let mut count = 0usize;
    for (user_id,) in &active_users {
        let res = sqlx::query(
            r#"
            INSERT INTO watch_timers (owner_user_id, target_user_id, chat_id, timeout_seconds,
                                      last_message_at, last_notified_at)
            VALUES (?, ?, ?, ?, NULL, NULL)
            ON CONFLICT (owner_user_id, target_user_id, chat_id) DO UPDATE SET
                timeout_seconds  = excluded.timeout_seconds,
                last_notified_at = NULL,
                updated_at       = datetime('now')
            "#,
        )
        .bind(owner_id)
        .bind(user_id)
        .bind(chat_id)
        .bind(timeout_seconds)
        .execute(state.db.as_ref())
        .await;

        match res {
            Ok(_) => count += 1,
            Err(e) => tracing::warn!("balltimer upsert user {user_id}: {e:#}"),
        }
    }

    let lookback_fmt = format_duration(lookback_seconds);
    let threshold_fmt = format_duration(timeout_seconds);
    let chat_link = chat_link_html(chat_id);
    send_html(
        &bot,
        msg.chat.id,
        &format!("{count} ᴛɪᴍᴇʀs sᴇᴛ ɪɴ {chat_link}\nʟᴏᴏᴋʙᴀᴄᴋ {lookback_fmt} · ᴛʜʀᴇsʜᴏʟᴅ {threshold_fmt}."),
    )
    .await;

    Ok(())
}
