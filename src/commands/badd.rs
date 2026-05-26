use teloxide::prelude::*;
use teloxide::types::Message;

use crate::messages::{chat_link_html, send_html};
use crate::state::AppState;

pub async fn handle(bot: Bot, msg: Message, args: String, state: AppState) -> ResponseResult<()> {
    let chat_id = if msg.chat.is_private() {
        let a = args.trim();
        if a.is_empty() {
            send_html(&bot, msg.chat.id, "ᴜsᴀɢᴇ: /badd &lt;chat_id&gt; (from DM) or run in the target chat").await;
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

    match sqlx::query("INSERT OR IGNORE INTO logged_chats (chat_id) VALUES (?)")
        .bind(chat_id)
        .execute(state.db.as_ref())
        .await
    {
        Ok(r) => {
            let link = chat_link_html(chat_id);
            if r.rows_affected() > 0 {
                send_html(&bot, msg.chat.id, &format!("ʟᴏɢɢɪɴɢ ᴇɴᴀʙʟᴇᴅ: {link}")).await;
            } else {
                send_html(&bot, msg.chat.id, &format!("ᴀʟʀᴇᴀᴅʏ ʟᴏɢɢɪɴɢ: {link}")).await;
            }
        }
        Err(e) => {
            tracing::warn!("badd db: {e:#}");
            send_html(&bot, msg.chat.id, "ᴅʙ ᴇʀʀᴏʀ.").await;
        }
    }

    Ok(())
}
