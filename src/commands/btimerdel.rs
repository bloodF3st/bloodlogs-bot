use teloxide::prelude::*;
use teloxide::types::Message;

use crate::messages::send_html;
use crate::state::AppState;

pub async fn handle(bot: Bot, msg: Message, args: String, state: AppState) -> ResponseResult<()> {
    let owner_id = msg.from().map(|u| u.id.0 as i64).unwrap_or(0);
    let a = args.trim();

    if a.is_empty() || !a.chars().all(|c| c.is_ascii_digit()) {
        send_html(&bot, msg.chat.id, "ᴜsᴀɢᴇ: /btimerdel &lt;id&gt;").await;
        return Ok(());
    }

    let id: i64 = match a.parse() {
        Ok(v) => v,
        Err(_) => {
            send_html(&bot, msg.chat.id, "ᴜsᴀɢᴇ: /btimerdel &lt;id&gt;").await;
            return Ok(());
        }
    };

    match sqlx::query("DELETE FROM watch_timers WHERE id = ? AND owner_user_id = ?")
        .bind(id)
        .bind(owner_id)
        .execute(state.db.as_ref())
        .await
    {
        Ok(r) if r.rows_affected() > 0 => {
            send_html(&bot, msg.chat.id, &format!("ᴛɪᴍᴇʀ #{id} ᴅᴇʟᴇᴛᴇᴅ.")).await;
        }
        Ok(_) => {
            send_html(&bot, msg.chat.id, "ᴛɪᴍᴇʀ ɴᴏᴛ ғᴏᴜɴᴅ.").await;
        }
        Err(e) => {
            tracing::warn!("btimerdel db: {e:#}");
            send_html(&bot, msg.chat.id, "ᴅʙ ᴇʀʀᴏʀ.").await;
        }
    }

    Ok(())
}
