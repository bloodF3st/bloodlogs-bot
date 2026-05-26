use teloxide::prelude::*;
use teloxide::types::Message;

use crate::messages::{chat_link_html, send_html};
use crate::state::AppState;

pub async fn handle(bot: Bot, msg: Message, state: AppState) -> ResponseResult<()> {
    let owner_id = msg.from().map(|u| u.id.0 as i64).unwrap_or(0);
    let chat_id = msg.chat.id.0;

    let n = match sqlx::query(
        "DELETE FROM watch_timers WHERE chat_id = ? AND owner_user_id = ?",
    )
    .bind(chat_id)
    .bind(owner_id)
    .execute(state.db.as_ref())
    .await
    {
        Ok(r) => r.rows_affected(),
        Err(e) => {
            tracing::warn!("btimerclear db: {e:#}");
            send_html(&bot, msg.chat.id, "ᴅʙ ᴇʀʀᴏʀ.").await;
            return Ok(());
        }
    };

    let chat_link = chat_link_html(chat_id);
    send_html(
        &bot,
        msg.chat.id,
        &format!("ᴀʟʟ ᴛɪᴍᴇʀs ɪɴ {chat_link} ᴄʟᴇᴀʀᴇᴅ ({n} ᴅᴇʟᴇᴛᴇᴅ)."),
    )
    .await;

    Ok(())
}
