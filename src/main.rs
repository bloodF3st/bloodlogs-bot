mod commands;
mod config;
mod jobs;
mod messages;
mod state;

use std::sync::Arc;
use std::time::{Duration, Instant};

use teloxide::{
    dispatching::{Dispatcher, UpdateFilterExt, UpdateHandler},
    prelude::*,
    types::{AllowedUpdate, CallbackQuery, Me, Message, ParseMode, Update},
    update_listeners,
    utils::command::BotCommands,
    RequestError,
};

use config::Config;
use state::AppState;

#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "lowercase")]
enum Command {
    #[command(description = "inactivity timer")]
    Btimer(String),
    #[command(description = "set timers for all recent users in this chat")]
    Balltimer(String),
    #[command(description = "clear all timers in this chat")]
    Btimerclear,
    #[command(description = "list active timers")]
    Btimers,
    #[command(description = "set log destination channel")]
    Bchannel(String),
    #[command(description = "add chat to logging whitelist")]
    Badd(String),
    #[command(description = "remove chat from logging whitelist")]
    Bdell(String),
    #[command(description = "delete watch timer by id")]
    Btimerdel(String),
    #[command(description = "command list")]
    Bhelp,
}

async fn on_command(bot: Bot, msg: Message, cmd: Command, state: AppState) -> ResponseResult<()> {
    match cmd {
        Command::Btimer(args) => commands::btimer::handle(bot, msg, args, state).await,
        Command::Balltimer(args) => commands::balltimer::handle(bot, msg, args, state).await,
        Command::Btimerclear => commands::btimerclear::handle(bot, msg, state).await,
        Command::Btimers => commands::btimers::handle(bot, msg, state).await,
        Command::Bchannel(args) => commands::bchannel::handle(bot, msg, args, state).await,
        Command::Badd(args) => commands::badd::handle(bot, msg, args, state).await,
        Command::Bdell(args) => commands::bdell::handle(bot, msg, args, state).await,
        Command::Btimerdel(args) => commands::btimerdel::handle(bot, msg, args, state).await,
        Command::Bhelp => commands::bhelp::handle(bot, msg).await,
    }
}

async fn on_message(_bot: Bot, msg: Message, state: AppState) -> ResponseResult<()> {
    let chat_id = msg.chat.id.0;

    // Watch-таймеры обновляются всегда — независимо от whitelist логирования.
    // Бот получает сообщение только если он в чате, этого достаточно для трекинга.
    if let Err(e) = jobs::watch::on_message(state.db.as_ref(), &msg).await {
        tracing::warn!("watch on_message: {e:#}");
    }

    let in_whitelist = sqlx::query_scalar::<_, i64>(
        "SELECT 1 FROM logged_chats WHERE chat_id = ?",
    )
    .bind(chat_id)
    .fetch_optional(state.db.as_ref())
    .await
    .unwrap_or(None)
    .is_some();

    if !in_whitelist {
        return Ok(());
    }

    if let Some(user) = msg.from() {
        let uid = user.id.0 as i64;
        if let Err(e) = sqlx::query(
            r#"
            INSERT INTO chat_activity (chat_id, user_id, last_seen_at)
            VALUES (?, ?, datetime('now'))
            ON CONFLICT (chat_id, user_id) DO UPDATE SET last_seen_at = datetime('now')
            "#,
        )
        .bind(chat_id)
        .bind(uid)
        .execute(state.db.as_ref())
        .await
        {
            tracing::warn!("chat_activity upsert: {e:#}");
        }
    }

    if let Some(html) = messages::format_log_html(&msg) {
        let _ = state.log_tx.send(html);
    }

    Ok(())
}

fn schema() -> UpdateHandler<RequestError> {
    let cmd_handler = Update::filter_message()
        .filter(|msg: Message, cfg: Arc<Config>| {
            msg.from()
                .map_or(false, |u| cfg.is_admin(u.id.0 as i64))
        })
        .filter_map(|msg: Message, me: Me| {
            let text = msg.text()?;
            let bot_name = me.user.username.as_deref().unwrap_or("");
            Command::parse(text, bot_name).ok()
        })
        .endpoint(on_command);

    let callback_handler = Update::filter_callback_query()
        .filter(|q: CallbackQuery, cfg: Arc<Config>| {
            cfg.is_admin(q.from.id.0 as i64)
                && q.data.as_deref().map_or(false, |d| d.starts_with("btimers:"))
        })
        .endpoint(commands::btimers::handle_callback);

    let msg_handler = Update::filter_message().endpoint(on_message);

    dptree::entry()
        .branch(callback_handler)
        .branch(cmd_handler)
        .branch(msg_handler)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cfg = Config::from_env()?;
    let cfg = Arc::new(cfg);

    if let Some(path) = cfg.database_url.strip_prefix("sqlite:") {
        if let Some(parent) = std::path::Path::new(path).parent() {
            if !parent.as_os_str().is_empty() {
                tokio::fs::create_dir_all(parent).await.ok();
            }
        }
    }

    let pool = sqlx::sqlite::SqlitePoolOptions::new()
        .max_connections(4)
        .connect_with(
            cfg.database_url
                .parse::<sqlx::sqlite::SqliteConnectOptions>()?
                .create_if_missing(true),
        )
        .await
        .map_err(|e| anyhow::anyhow!("DB connect: {e}"))?;

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .map_err(|e| anyhow::anyhow!("DB migrate: {e}"))?;

    let pool = Arc::new(pool);

    let (log_tx, log_rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    let state = AppState::new(pool, cfg.clone(), log_tx);

    let bot = Bot::new(cfg.bot_token.clone());

    jobs::watch::spawn_watch_supervisor(bot.clone(), &state);
    tokio::spawn(jobs::relay::run(
        bot.clone(),
        state.db.clone(),
        state.admin_ids().to_vec(),
        log_rx,
    ));

    let admin_chat = ChatId(*cfg.admin_ids.first().expect("at least one admin_id required"));

    let flag_path = {
        let url = cfg.database_url.strip_prefix("sqlite:").unwrap_or("data/bloodlogs.db");
        std::path::Path::new(url)
            .parent()
            .unwrap_or(std::path::Path::new("."))
            .join(".started")
    };
    let is_restart = flag_path.exists();
    if !is_restart {
        tokio::fs::write(&flag_path, b"1").await.ok();
    }

    let startup_msg = if is_restart { "ʙʟᴏᴏᴅʟᴏɢs ʀᴇsᴛᴀʀᴛᴇᴅ" } else { "ʙʟᴏᴏᴅʟᴏɢs ᴏɴʟɪɴᴇ" };
    if let Err(e) = bot
        .send_message(admin_chat, startup_msg)
        .parse_mode(ParseMode::Html)
        .await
    {
        tracing::warn!("startup notify: {e}");
    }

    tracing::info!("bloodLogs: long poll, admin_ids={:?}", cfg.admin_ids);

    const MAX_RECONNECTS: usize = 5;
    const RECONNECT_WINDOW: Duration = Duration::from_secs(60);
    let mut reconnect_times: std::collections::VecDeque<Instant> = std::collections::VecDeque::new();

    loop {
        tokio::select! {
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("shutdown on Ctrl+C");
                break;
            }
            _ = async {
                let listener = update_listeners::Polling::builder(bot.clone())
                    .timeout(Duration::from_secs(10))
                    .allowed_updates(vec![
                        AllowedUpdate::Message,
                        AllowedUpdate::CallbackQuery,
                    ])
                    .build();
                let mut dispatcher = Dispatcher::builder(bot.clone(), schema())
                    .dependencies(dptree::deps![state.clone(), cfg.clone()])
                    .default_handler(|_upd: std::sync::Arc<Update>| async move {})
                    .error_handler(LoggingErrorHandler::with_custom_text(
                        "bloodLogs: handler error",
                    ))
                    .build();
                dispatcher
                    .dispatch_with_listener(
                        listener,
                        Arc::new(|err: RequestError| async move {
                            match err {
                                RequestError::RetryAfter(d) => {
                                    tracing::warn!("RetryAfter: sleeping {}s", d.as_secs());
                                    tokio::time::sleep(d).await;
                                }
                                RequestError::Api(ref e) => {
                                    let msg = e.to_string();
                                    if msg.contains("Unauthorized") || msg.contains("Invalid bot token") {
                                        tracing::error!("INVALID TOKEN ({msg}) — exiting");
                                        std::process::exit(1);
                                    }
                                    tracing::error!("api error: {msg}");
                                }
                                e => tracing::error!("listener: {e}"),
                            }
                        }),
                    )
                    .await
            } => {
                let now = Instant::now();
                reconnect_times.retain(|t| now.duration_since(*t) < RECONNECT_WINDOW);
                reconnect_times.push_back(now);

                if reconnect_times.len() > MAX_RECONNECTS {
                    tracing::error!(
                        "bloodLogs: {} reconnects in {}s — persistent failure, exiting for systemd restart",
                        reconnect_times.len(),
                        RECONNECT_WINDOW.as_secs()
                    );
                    let _ = bot
                        .send_message(admin_chat, "ʙʟᴏᴏᴅʟᴏɢs: ᴘᴇʀsɪsᴛᴇɴᴛ ᴀᴘɪ ᴇʀʀᴏʀ — ʀᴇsᴛᴀʀᴛɪɴɢ")
                        .await;
                    std::process::exit(1);
                }

                tracing::error!("long poll stopped, reconnecting in 5s");
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        }
    }

    Ok(())
}
