use std::sync::Arc;
use std::time::Duration;

use sqlx::SqlitePool;
use teloxide::prelude::*;
use teloxide::types::ParseMode;
use teloxide::RequestError;
use tokio::sync::mpsc::UnboundedReceiver;

use crate::commands::bchannel::get_log_channel;

const INTER_MSG_DELAY: Duration = Duration::from_millis(100);

pub async fn run(
    bot: Bot,
    pool: Arc<SqlitePool>,
    admin_id: i64,
    mut rx: UnboundedReceiver<String>,
) {
    while let Some(html) = rx.recv().await {
        let dest = match get_log_channel(pool.as_ref()).await {
            Ok(Some(id)) => id,
            _ => continue,
        };

        loop {
            match bot
                .send_message(ChatId(dest), &html)
                .parse_mode(ParseMode::Html)
                .await
            {
                Ok(_) => break,
                Err(RequestError::RetryAfter(d)) => {
                    tracing::warn!("log relay: flood wait {}s", d.as_secs());
                    tokio::time::sleep(d).await;
                }
                Err(RequestError::InvalidJson { raw, .. }) if raw.contains("\"ok\":true") => {
                    break;
                }
                Err(e) => {
                    let err_str = e.to_string();
                    tracing::warn!("log relay to {dest}: {err_str}");
                    if err_str.contains("Forbidden") || err_str.contains("chat not found") {
                        let _ = bot
                            .send_message(
                                ChatId(admin_id),
                                &format!(
                                    "ʟᴏɢ ᴄʜᴀɴɴᴇʟ {dest} ᴜɴʀᴇᴀᴄʜᴀʙʟᴇ: {err_str}\nᴜsᴇ /bchannel ᴛᴏ ʀᴇsᴇᴛ."
                                ),
                            )
                            .await;
                    }
                    break;
                }
            }
        }

        tokio::time::sleep(INTER_MSG_DELAY).await;
    }
}
