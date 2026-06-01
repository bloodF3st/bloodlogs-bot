use std::sync::Arc;
use std::time::Duration;

use sqlx::SqlitePool;
use teloxide::prelude::*;
use teloxide::types::ParseMode;
use teloxide::RequestError;
use tokio::sync::mpsc::Receiver;

use crate::commands::bchannel::get_log_channel;

const BATCH_SIZE: usize = 5;
const BATCH_WINDOW: Duration = Duration::from_millis(800);
const BATCH_SEPARATOR: &str = "\n\n";
const MAX_TG_LEN: usize = 4000;
const INTER_BATCH_DELAY: Duration = Duration::from_secs(3);

pub async fn run(
    bot: Bot,
    pool: Arc<SqlitePool>,
    admin_ids: Vec<i64>,
    mut rx: Receiver<String>,
) {
    while let Some(first) = rx.recv().await {
        let mut batch = vec![first];
        tokio::time::sleep(BATCH_WINDOW).await;
        while batch.len() < BATCH_SIZE {
            match rx.try_recv() {
                Ok(msg) => batch.push(msg),
                Err(_) => break,
            }
        }

        let mut combined = batch.join(BATCH_SEPARATOR);
        if combined.len() > MAX_TG_LEN {
            let cut = (0..=MAX_TG_LEN).rev().find(|&i| combined.is_char_boundary(i)).unwrap_or(0);
            combined.truncate(cut);
            combined.push_str("\n…");
        }

        let dest = match get_log_channel(pool.as_ref()).await {
            Ok(Some(id)) => id,
            _ => continue,
        };

        loop {
            match bot
                .send_message(ChatId(dest), &combined)
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
                        for &aid in &admin_ids {
                            let _ = bot
                                .send_message(
                                    ChatId(aid),
                                    &format!(
                                        "ʟᴏɢ ᴄʜᴀɴɴᴇʟ {dest} ᴜɴʀᴇᴀᴄʜᴀʙʟᴇ: {err_str}\nᴜsᴇ /bchannel ᴛᴏ ʀᴇsᴇᴛ."
                                    ),
                                )
                                .await;
                        }
                    }
                    break;
                }
            }
        }

        tokio::time::sleep(INTER_BATCH_DELAY).await;
    }
}
