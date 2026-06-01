use anyhow::{Context, Result};

#[derive(Clone, Debug)]
pub struct Config {
    pub bot_token: String,
    pub admin_ids: Vec<i64>,
    pub database_url: String,
    pub notify_chat_id: Option<i64>,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();
        let bot_token = std::env::var("BOT_TOKEN").context("BOT_TOKEN missing in .env")?;
        let admin_ids: Vec<i64> = std::env::var("ADMIN_ID")
            .context("ADMIN_ID missing in .env")?
            .split(',')
            .map(|s| s.trim().parse::<i64>().context("ADMIN_ID must be comma-separated numeric Telegram user ids"))
            .collect::<Result<Vec<_>>>()?;
        let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL missing in .env")?;
        let notify_chat_id = std::env::var("NOTIFY_CHAT_ID")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .map(|s| s.trim().parse::<i64>().context("NOTIFY_CHAT_ID must be a numeric chat id"))
            .transpose()?;
        Ok(Self {
            bot_token,
            admin_ids,
            database_url,
            notify_chat_id,
        })
    }

    pub fn is_admin(&self, uid: i64) -> bool {
        self.admin_ids.contains(&uid)
    }
}
