use teloxide::prelude::*;
use teloxide::types::Message;

use crate::messages::{chat_link_html, send_html};
use crate::state::AppState;

pub async fn handle(bot: Bot, msg: Message, args: String, state: AppState) -> ResponseResult<()> {
    let chat_id = if msg.chat.is_private() {
        let a = args.trim();
        if a.is_empty() {
            send_html(&bot, msg.chat.id, "ᴜsᴀɢᴇ: /bdell &lt;chat_id&gt; (from DM) or run in the target chat").await;
            return Ok(());
        }
        match a.parse::<i64>() {
            Ok(v) => v,
            Err(_) => {
                send_html(&bot, msg.chat.id, "ɪɴᴠᴀʟɪᴅ ᴄʜᴀᴛ ɪᴅ.").await;
                return Ok(());
            }
        }
    } else {
        msg.chat.id.0
    };

    match sqlx::query("DELETE FROM logged_chats WHERE chat_id = ?")
        .bind(chat_id)
        .execute(state.db.as_ref())
        .await
    {
        Ok(r) => {
            let link = chat_link_html(chat_id);
            if r.rows_affected() > 0 {
                send_html(&bot, msg.chat.id, &format!("ʟᴏɢɢɪɴɢ ᴅɪsᴀʙʟᴇᴅ: {link}")).await;
            } else {
                send_html(&bot, msg.chat.id, &format!("ɴᴏᴛ ɪɴ ʟᴏɢ ʟɪsᴛ: {link}")).await;
            }
        }
        Err(e) => {
            tracing::warn!("bdell db: {e:#}");
            send_html(&bot, msg.chat.id, "ᴅʙ ᴇʀʀᴏʀ.").await;
        }
    }

    Ok(())
}
