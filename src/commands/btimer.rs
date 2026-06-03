use teloxide::prelude::*;
use teloxide::types::Message;

use crate::messages::{chat_link_html, format_duration, parse_duration, send_html, user_link_html};
use crate::state::AppState;

pub async fn handle(bot: Bot, msg: Message, args: String, state: AppState) -> ResponseResult<()> {
    let owner_id = msg.from().map(|u| u.id.0 as i64).unwrap_or(0);
    let a = args.trim();

    if a.is_empty() {
        send_html(
            &bot,
            msg.chat.id,
            "ᴜsᴀɢᴇ: /btimer <user_id> <chat_id> <time> · /btimer del <id>",
        )
        .await;
        return Ok(());
    }

    if a.starts_with("del")
        && (a.len() == 3 || a.as_bytes().get(3).map_or(false, |b| b.is_ascii_whitespace()))
    {
        let rest = a[3..].trim();
        if rest.is_empty() || !rest.chars().all(|c| c.is_ascii_digit()) {
            send_html(
                &bot,
                msg.chat.id,
                "ᴜsᴀɢᴇ: /btimer <user_id> <chat_id> <time> · /btimer del <id>",
            )
            .await;
            return Ok(());
        }
        let id: i64 = rest.parse().unwrap_or(0);
        match delete_timer(&state, id).await {
            Ok(Some((target_user_id, chat_id))) => {
                let user_link = user_link_html(target_user_id);
                let chat_link = chat_link_html(chat_id);
                send_html(
                    &bot,
                    msg.chat.id,
                    &format!("ᴛɪᴍᴇʀ #{id} ᴅᴇʟᴇᴛᴇᴅ · {user_link} | {chat_link}"),
                )
                .await;
            }
            Ok(None) => {
                send_html(&bot, msg.chat.id, "ᴛɪᴍᴇʀ ɴᴏᴛ ғᴏᴜɴᴅ.").await;
            }
            Err(e) => {
                tracing::warn!("btimer del db: {e:#}");
                send_html(&bot, msg.chat.id, "ᴅʙ ᴇʀʀᴏʀ.").await;
            }
        }
        return Ok(());
    }

    let parts: Vec<&str> = a.split_whitespace().collect();
    if parts.len() != 3 {
        send_html(
            &bot,
            msg.chat.id,
            "ᴜsᴀɢᴇ: /btimer <user_id> <chat_id> <time> · /btimer del <id>",
        )
        .await;
        return Ok(());
    }

    let target_user_id: i64 = match parts[0].parse() {
        Ok(v) => v,
        Err(_) => {
            send_html(
                &bot,
                msg.chat.id,
                "ᴜsᴀɢᴇ: /btimer <user_id> <chat_id> <time> · /btimer del <id>",
            )
            .await;
            return Ok(());
        }
    };
    let chat_id: i64 = match parts[1].parse() {
        Ok(v) => v,
        Err(_) => {
            send_html(
                &bot,
                msg.chat.id,
                "ᴜsᴀɢᴇ: /btimer <user_id> <chat_id> <time> · /btimer del <id>",
            )
            .await;
            return Ok(());
        }
    };
    let timeout_seconds = match parse_duration(parts[2]) {
        Ok(s) => s,
        Err(_) => {
            send_html(
                &bot,
                msg.chat.id,
                "ᴜsᴀɢᴇ: /btimer <user_id> <chat_id> <time> · /btimer del <id>",
            )
            .await;
            return Ok(());
        }
    };

    match upsert_timer(&state, owner_id, target_user_id, chat_id, timeout_seconds).await {
        Ok((id, is_new)) => {
            let user_link = user_link_html(target_user_id);
            let chat_link = chat_link_html(chat_id);
            let threshold = format_duration(timeout_seconds);
            if is_new {
                send_html(
                    &bot,
                    msg.chat.id,
                    &format!("ᴛɪᴍᴇʀ #{id} · {user_link} | {chat_link} | ᴛʜʀᴇsʜᴏʟᴅ {threshold}."),
                )
                .await;
            } else {
                send_html(
                    &bot,
                    msg.chat.id,
                    &format!("ᴛɪᴍᴇʀ ᴜᴘᴅᴀᴛᴇᴅ: {user_link} | {chat_link} | {threshold}."),
                )
                .await;
            }

            // Сразу проверяем доступ к целевому чату
            if let Err(e) = bot.get_chat(ChatId(chat_id)).await {
                let s = e.to_string();
                if s.contains("Forbidden") || s.contains("bot is not a member")
                    || s.contains("chat not found") || s.contains("not enough rights")
                {
                    send_html(
                        &bot,
                        msg.chat.id,
                        &format!("⚠️ ᴛɪᴍᴇʀ sᴇᴛ, ʙᴜᴛ ʙᴏᴛ ɪs ɴᴏᴛ ᴀ ᴍᴇᴍʙᴇʀ ᴏғ {chat_link}.\nᴀᴅᴅ ᴛʜᴇ ʙᴏᴛ ᴛᴏ ᴛʜᴀᴛ ᴄʜᴀᴛ ᴛᴏ ᴛʀᴀᴄᴋ ᴀᴄᴛɪᴠɪᴛʏ."),
                    )
                    .await;
                }
            }
        }
        Err(e) => {
            tracing::warn!("btimer upsert db: {e:#}");
            send_html(&bot, msg.chat.id, "ᴅʙ ᴇʀʀᴏʀ.").await;
        }
    }
    Ok(())
}

async fn upsert_timer(
    state: &AppState,
    owner_id: i64,
    target_user_id: i64,
    chat_id: i64,
    timeout_seconds: i64,
) -> anyhow::Result<(i64, bool)> {
    let already: bool = sqlx::query_scalar(
        "SELECT EXISTS(SELECT 1 FROM watch_timers WHERE target_user_id = ? AND chat_id = ?)",
    )
    .bind(target_user_id)
    .bind(chat_id)
    .fetch_one(state.db.as_ref())
    .await?;

    let id: i64 = sqlx::query_scalar(
        r#"
        INSERT INTO watch_timers (owner_user_id, target_user_id, chat_id, timeout_seconds,
                                  last_message_at, last_notified_at)
        VALUES (?, ?, ?, ?, NULL, NULL)
        ON CONFLICT (target_user_id, chat_id) DO UPDATE SET
            owner_user_id = excluded.owner_user_id,
            timeout_seconds = excluded.timeout_seconds,
            last_notified_at = NULL,
            updated_at = datetime('now')
        RETURNING id
        "#,
    )
    .bind(owner_id)
    .bind(target_user_id)
    .bind(chat_id)
    .bind(timeout_seconds)
    .fetch_one(state.db.as_ref())
    .await?;

    Ok((id, !already))
}

async fn delete_timer(state: &AppState, id: i64) -> anyhow::Result<Option<(i64, i64)>> {
    let row: Option<(i64, i64)> = sqlx::query_as(
        "DELETE FROM watch_timers WHERE id = ? RETURNING target_user_id, chat_id",
    )
    .bind(id)
    .fetch_optional(state.db.as_ref())
    .await?;
    Ok(row)
}
