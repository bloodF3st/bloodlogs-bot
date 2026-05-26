use teloxide::prelude::*;
use teloxide::types::Message;

use crate::messages::send_html;

const HELP_HTML: &str = concat!(
    "<b>ʙʟᴏᴏᴅʟᴏɢs</b> — <a href=\"https://t.me/blxfe\">@blxfe</a>\n\n",
    "/btimer <code>&lt;user_id&gt; &lt;chat_id&gt; &lt;time&gt;</code> — ɪɴᴀᴄᴛɪᴠɪᴛʏ ᴛɪᴍᴇʀ\n",
    "/btimer del <code>&lt;id&gt;</code> — ʀᴇᴍᴏᴠᴇ ᴛɪᴍᴇʀ\n\n",
    "/balltimer <code>&lt;lookback&gt; &lt;threshold&gt;</code> — ᴛɪᴍᴇʀ ғᴏʀ ᴀʟʟ ᴀᴄᴛɪᴠᴇ ᴜsᴇʀs ɪɴ ᴛʜɪs ᴄʜᴀᴛ\n",
    "/btimerclear — ʀᴇᴍᴏᴠᴇ ᴀʟʟ ᴛɪᴍᴇʀs ɪɴ ᴛʜɪs ᴄʜᴀᴛ\n\n",
    "/logs — ʟɪsᴛ ᴀᴄᴛɪᴠᴇ ᴛɪᴍᴇʀs\n\n",
    "/bchannel <code>&lt;chat_id&gt;</code> — sᴇᴛ ʟᴏɢ ᴅᴇsᴛɪɴᴀᴛɪᴏɴ ᴄʜᴀɴɴᴇʟ\n",
    "/bchannel — sʜᴏᴡ ᴄᴜʀʀᴇɴᴛ ᴄʜᴀɴɴᴇʟ",
);

pub async fn handle(bot: Bot, msg: Message) -> ResponseResult<()> {
    send_html(&bot, msg.chat.id, HELP_HTML).await;
    Ok(())
}
