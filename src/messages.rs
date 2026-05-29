const MSG_TRUNCATE_CHARS: usize = 135;

fn truncate_msg(s: &str) -> String {
    let mut chars = s.chars();
    let truncated: String = chars.by_ref().take(MSG_TRUNCATE_CHARS).collect();
    if chars.next().is_some() {
        truncated + "…"
    } else {
        truncated
    }
}

const HTTPS_TME_C: &str = concat!("https", "://t.me/c/");
const TG_USER: &str = concat!("tg", "://user?id=");
const TG_OPENMESSAGE: &str = concat!("tg", "://openmessage?chat_id=");

pub fn chat_link_html(id: i64) -> String {
    if id <= -1_000_000_000_000_i64 {
        let s = id.to_string();
        if let Some(rest) = s.strip_prefix("-100") {
            if let Ok(internal) = rest.parse::<i64>() {
                return format!("<a href=\"{HTTPS_TME_C}{internal}/1\">{id}</a>");
            }
        }
    }
    let abs = id.unsigned_abs();
    format!("<a href=\"{TG_OPENMESSAGE}{abs}\">{id}</a>")
}

pub fn user_link_html(id: i64) -> String {
    format!("<a href=\"{TG_USER}{id}\">{id}</a>")
}

pub fn format_duration(secs: i64) -> String {
    if secs >= 86_400 {
        let d = secs / 86_400;
        let h = (secs % 86_400) / 3600;
        if h > 0 { format!("{d}d {h}h") } else { format!("{d}d") }
    } else if secs >= 3600 {
        let h = secs / 3600;
        let m = (secs % 3600) / 60;
        if m > 0 { format!("{h}h {m}m") } else { format!("{h}h") }
    } else if secs >= 60 {
        format!("{}m", secs / 60)
    } else {
        format!("{secs}s")
    }
}

pub fn parse_duration(s: &str) -> anyhow::Result<i64> {
    let s = s.trim().to_lowercase();
    if s.is_empty() {
        anyhow::bail!("empty duration");
    }
    let mut total: i64 = 0;
    let mut cur = String::new();
    for c in s.chars() {
        if c.is_ascii_digit() {
            cur.push(c);
        } else if !cur.is_empty() {
            let n: i64 = cur.parse().map_err(|_| anyhow::anyhow!("invalid number in duration"))?;
            cur.clear();
            total += match c {
                'h' => n.saturating_mul(3600),
                'm' => n.saturating_mul(60),
                'd' => n.saturating_mul(86_400),
                's' => n,
                _ => anyhow::bail!("unknown unit '{}': use h, m, d, s", c),
            };
        }
    }
    if !cur.is_empty() {
        anyhow::bail!("trailing number with no unit: use h, m, d, s");
    }
    if total <= 0 {
        anyhow::bail!("duration must be > 0");
    }
    Ok(total)
}

fn msg_link(chat_id: i64, msg_id: i32) -> Option<String> {
    let s = chat_id.to_string();
    let rest = s.strip_prefix("-100")?;
    let internal: i64 = rest.parse().ok()?;
    Some(format!("{HTTPS_TME_C}{internal}/{msg_id}"))
}

pub fn escape_html(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn fmt_timestamp_msk(dt: chrono::DateTime<chrono::Utc>) -> String {
    let msk = chrono::FixedOffset::east_opt(3 * 3600).unwrap();
    dt.with_timezone(&msk).format("%d.%m.%Y %H:%M:%S").to_string()
}

fn user_name(u: &teloxide::types::User) -> String {
    let mut s = u.first_name.trim().to_string();
    if let Some(ln) = &u.last_name {
        let ln = ln.trim();
        if !ln.is_empty() {
            s.push(' ');
            s.push_str(ln);
        }
    }
    s
}

fn media_type_label(msg: &teloxide::types::Message) -> Option<&'static str> {
    use teloxide::types::{MediaKind, MessageKind};
    if let MessageKind::Common(ref c) = msg.kind {
        return match &c.media_kind {
            MediaKind::Photo(_) => Some("PHOTO"),
            MediaKind::Video(_) => Some("VIDEO"),
            MediaKind::Document(_) => Some("DOCUMENT"),
            MediaKind::Audio(_) => Some("AUDIO"),
            MediaKind::Voice(_) => Some("VOICE"),
            MediaKind::VideoNote(_) => Some("VIDEO NOTE"),
            MediaKind::Sticker(_) => Some("STICKER"),
            MediaKind::Animation(_) => Some("GIF"),
            MediaKind::Contact(_) => Some("CONTACT"),
            MediaKind::Location(_) => Some("LOCATION"),
            MediaKind::Poll(_) => Some("POLL"),
            _ => None,
        };
    }
    None
}

pub fn format_log_html(msg: &teloxide::types::Message) -> Option<String> {
    use teloxide::types::{ChatKind, MessageKind};

    let ts = fmt_timestamp_msk(msg.date);
    let chat_id = msg.chat.id.0;
    let msg_id = msg.id.0;

    let chat_title = match &msg.chat.kind {
        ChatKind::Public(p) => p.title.as_deref().unwrap_or("").to_string(),
        ChatKind::Private(p) => {
            let mut s = p.first_name.as_deref().unwrap_or("").to_string();
            if let Some(ln) = &p.last_name {
                if !ln.is_empty() { s.push(' '); s.push_str(ln); }
            }
            s
        }
    };

    let chat_html = if let Some(link) = msg_link(chat_id, msg_id) {
        if chat_title.is_empty() {
            format!("<a href=\"{link}\"><code>{chat_id}</code></a>")
        } else {
            format!("<a href=\"{link}\">{}</a> <code>{chat_id}</code>", escape_html(&chat_title))
        }
    } else {
        let abs = (chat_id as i64).unsigned_abs();
        if chat_title.is_empty() {
            format!("<a href=\"{TG_OPENMESSAGE}{abs}\"><code>{chat_id}</code></a>")
        } else {
            format!("<a href=\"{TG_OPENMESSAGE}{abs}\">{}</a> <code>{chat_id}</code>", escape_html(&chat_title))
        }
    };

    match &msg.kind {
        MessageKind::NewChatMembers(m) => {
            let names: Vec<String> = m.new_chat_members.iter().map(|u| {
                format!(
                    "<a href=\"{TG_USER}{id}\">{name}</a> <code>{id}</code>",
                    id = u.id.0,
                    name = escape_html(&user_name(u))
                )
            }).collect();
            Some(format!("ᴊᴏɪɴᴇᴅ {chat_html}\n{}\n<i>{ts}</i>", names.join(", ")))
        }
        MessageKind::LeftChatMember(m) => {
            let u = &m.left_chat_member;
            let sender_html = format!(
                "<a href=\"{TG_USER}{id}\">{name}</a> <code>{id}</code>",
                id = u.id.0,
                name = escape_html(&user_name(u))
            );
            Some(format!("ʟᴇғᴛ {chat_html}\n{sender_html}\n<i>{ts}</i>"))
        }
        MessageKind::Common(_) => {
            let sender_html = match msg.from() {
                Some(u) => format!(
                    "<a href=\"{TG_USER}{id}\">{name}</a> <code>{id}</code>",
                    id = u.id.0,
                    name = escape_html(&user_name(u))
                ),
                None => {
                    let abs = (chat_id as i64).unsigned_abs();
                    format!("<a href=\"{TG_OPENMESSAGE}{abs}\">ᴀɴᴏɴ</a>")
                }
            };

            let text = truncate_msg(msg.text().unwrap_or(""));
            let caption = truncate_msg(msg.caption().unwrap_or(""));
            let media_label = media_type_label(msg);

            let body = if !text.is_empty() {
                format!("<blockquote>{}</blockquote>", escape_html(&text))
            } else if !caption.is_empty() {
                let label = media_label.unwrap_or("ᴍᴇᴅɪᴀ");
                format!("[{label}]\n<blockquote>{}</blockquote>", escape_html(&caption))
            } else if let Some(label) = media_label {
                format!("[{label}]")
            } else {
                return None;
            };

            Some(format!("{sender_html} → {chat_html}\n{body}\n<i>{ts}</i>"))
        }
        _ => None,
    }
}

pub async fn send_html(bot: &teloxide::Bot, chat_id: teloxide::types::ChatId, html: &str) {
    use teloxide::prelude::*;
    use teloxide::types::ParseMode;
    if let Err(e) = bot
        .send_message(chat_id, html)
        .parse_mode(ParseMode::Html)
        .await
    {
        tracing::warn!("send_html to {}: {e}", chat_id.0);
    }
}
