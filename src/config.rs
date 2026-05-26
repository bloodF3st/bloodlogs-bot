use anyhow::{Context, Result};

#[derive(Clone, Debug)]
pub struct Config {
    pub bot_token: String,
    pub admin_id: i64,
    pub database_url: String,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        dotenvy::dotenv().ok();
        let bot_token = std::env::var("BOT_TOKEN").context("BOT_TOKEN missing in .env")?;
        let admin_id: i64 = std::env::var("ADMIN_ID")
            .context("ADMIN_ID missing in .env")?
            .trim()
            .parse()
            .context("ADMIN_ID must be a numeric Telegram user id")?;
        let database_url = std::env::var("DATABASE_URL").context("DATABASE_URL missing in .env")?;
        Ok(Self {
            bot_token,
            admin_id,
            database_url,
        })
    }
}
