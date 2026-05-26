# bloodLogs

Telegram bot for message logging and inactivity timers.

Logs all messages from chats where you are present. Sends logs to a designated channel. Notifies when a user has been inactive longer than a set threshold.

## How it works

- Bot is added to group chats
- On every message it checks whether your Telegram account (`ADMIN_ID`) is a member of that chat
- If yes — logs the message (sender, chat, text/media label, timestamp MSK) to the log channel
- All commands are accepted only from `ADMIN_ID`; everyone else is completely ignored
- Inactivity timers run in background and notify you when a tracked user goes silent

## BotFather setup

1. Open [@BotFather](https://t.me/BotFather) → `/newbot`
2. Set name and username
3. Copy the token → `BOT_TOKEN` in `.env`
4. `/setprivacy` → select your bot → **Disable**
   This is required so the bot can read all messages in groups, not just commands
5. `/setjoingroups` → **Enable** (optional, lets users add bot via link)

## Configuration

Copy `.env.example` to `.env` and fill in:

```
BOT_TOKEN=123456:ABC-token-from-BotFather
ADMIN_ID=your_telegram_user_id
DATABASE_URL=sqlite:data/bloodlogs.db
```

`ADMIN_ID` — your numeric Telegram user ID. Get it from [@userinfobot](https://t.me/userinfobot).

## Running

### From binary (recommended)

Download the latest binary from [Releases](../../releases), put it next to `.env`:

```
bloodlogs-bot
.env
```

```sh
./bloodlogs-bot
```

### systemd service

```ini
[Unit]
Description=bloodlogs bot
After=network.target

[Service]
WorkingDirectory=/opt/bloodlogs
ExecStart=/opt/bloodlogs/bloodlogs-bot
EnvironmentFile=/opt/bloodlogs/.env
Restart=on-failure
RestartSec=5
MemoryMax=200M

[Install]
WantedBy=multi-user.target
```

```sh
systemctl enable --now bloodlogs-bot
```

### From source

```sh
cargo build --release
./target/release/bloodlogs-bot
```

Requires Rust 1.75+.

## Commands

All commands are admin-only.

| Command | Description |
|---|---|
| `/bchannel <chat_id>` | Set the channel where logs are sent |
| `/bchannel` | Show current log channel |
| `/btimer <user_id> <chat_id> <time>` | Set inactivity timer for a user in a chat |
| `/btimer del <id>` | Delete a timer by ID |
| `/balltimer <lookback> <threshold>` | Set timers for all users active in this chat within `lookback` window |
| `/btimerclear` | Delete all timers in the current chat |
| `/logs` | List all active timers |
| `/bhelp` | Command reference |

Time format: `30s`, `5m`, `2h`, `1d`. Combinations: `1h30m`, `2d12h`.

## Logging

What gets logged:
- Regular text messages → blockquote with text
- Media with caption → `[PHOTO/VIDEO/…]` + blockquote with caption
- Media without caption → `[PHOTO]`, `[VIDEO]`, `[GIF]`, `[STICKER]`, etc.
- New member joined / member left
- Sender name + ID (clickable), chat name + ID (clickable link to message), timestamp MSK

What is NOT logged:
- Chats where `ADMIN_ID` is not a member
- Messages that arrive faster than 500 ms per chat (flood protection)

## Inactivity timers

Timer fires when a tracked user has not sent any message in `chat_id` for longer than `threshold`.

```
/btimer 123456789 -1001234567890 2h
```

Fires every `threshold` interval until the user speaks again. Once the user sends a message, the timer resets and stops firing.

`/balltimer 55m 1h` — sets a 1h timer for every user who wrote something in the last 55 minutes in the current chat. One timer per user per chat; re-running updates the threshold.

## Notifications

On start: `ʙʟᴏᴏᴅʟᴏɢs ᴏɴʟɪɴᴇ` (first launch) or `ʙʟᴏᴏᴅʟᴏɢs ʀᴇsᴛᴀʀᴛᴇᴅ`.

If the log channel becomes unreachable (bot removed, channel deleted): DM to admin with the error and `/bchannel` hint.

If the long-poll connection drops more than 5 times in 60 seconds: bot exits with code 1 so systemd restarts it.
