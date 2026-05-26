use teloxide::prelude::*;
use teloxide::types::Message;

use crate::messages::{chat_link_html, send_html};
use crate::state::AppState;

pub async fn handle(bot: Bot, msg: Message, args: String, state: AppState) -> ResponseResult<()> {
    let a = args.trim();

    if a.is_empty() {
        match get_log_channel(state.db.as_ref()).await {
            Ok(Some(cid)) => {
                let link = chat_link_html(cid);
                send_html(&bot, msg.chat.id, &format!("ʟᴏɢ ᴄʜᴀɴɴᴇʟ: {link}")).await;
            }
            Ok(None) => {
                send_html(
                    &bot,
                    msg.chat.id,
                    "ʟᴏɢ ᴄʜᴀɴɴᴇʟ ɴᴏᴛ sᴇᴛ.\nᴜsᴀɢᴇ: /bchannel &lt;chat_id&gt;",
                )
                .await;
            }
            Err(e) => {
                tracing::warn!("bchannel get: {e:#}");
                send_html(&bot, msg.chat.id, "ᴅʙ ᴇʀʀᴏʀ.").await;
            }
        }
        return Ok(());
    }

    let chat_id: i64 = match a.parse() {
        Ok(v) => v,
        Err(_) => {
            send_html(
                &bot,
                msg.chat.id,
                "ᴜsᴀɢᴇ: /bchannel &lt;chat_id&gt;",
            )
            .await;
            return Ok(());
        }
    };

    match set_log_channel(state.db.as_ref(), chat_id).await {
        Ok(()) => {
            let link = chat_link_html(chat_id);
            send_html(&bot, msg.chat.id, &format!("ʟᴏɢ ᴄʜᴀɴɴᴇʟ sᴇᴛ: {link}")).await;
        }
        Err(e) => {
            tracing::warn!("bchannel set: {e:#}");
            send_html(&bot, msg.chat.id, "ᴅʙ ᴇʀʀᴏʀ.").await;
        }
    }
    Ok(())
}

pub async fn get_log_channel(pool: &sqlx::SqlitePool) -> anyhow::Result<Option<i64>> {
    let val: Option<String> =
        sqlx::query_scalar("SELECT value FROM bot_settings WHERE key = 'log_channel'")
            .fetch_optional(pool)
            .await?;
    Ok(val.and_then(|v| v.parse().ok()))
}

async fn set_log_channel(pool: &sqlx::SqlitePool, chat_id: i64) -> anyhow::Result<()> {
    sqlx::query(
        "INSERT INTO bot_settings (key, value) VALUES ('log_channel', ?)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
    )
    .bind(chat_id.to_string())
    .execute(pool)
    .await?;
    Ok(())
}
