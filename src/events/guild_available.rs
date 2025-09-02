use poise::serenity_prelude as serenity;
use tracing::error;

use crate::util::PoiseData;

pub struct GuildAvailable;

#[serenity::async_trait]
impl serenity::EventHandler for GuildAvailable {
    /// Adds guild to database.
    async fn guild_create(
        &self,
        ctx: serenity::Context,
        guild: serenity::Guild,
        _is_new: Option<bool>,
    ) {
        let guild_id: i64 = guild.id.into();
        let channel_id: i64 = default_text_channel(&ctx, &guild)
            .await
            .map(|channel| channel.id.into())
            .unwrap_or(0); // User will have to fix channel with the /bind cmd.

        let data = ctx.poise_data_unwrap().await;
        data.repo
            .discord
            .add_guild(guild_id, channel_id)
            .await
            .inspect_err(|err| error!(?err, "Failed to add new guild"))
            .ok();
    }
}

async fn default_text_channel<'a>(
    ctx: &serenity::Context,
    guild: &'a serenity::Guild,
) -> Option<&'a serenity::GuildChannel> {
    let id = ctx.cache.current_user().id;
    let member = guild.member(&ctx, id).await.ok()?;

    guild.channels.values().find(|channel| {
        channel.kind == serenity::ChannelType::Text
            && guild.user_permissions_in(channel, &member).contains(
                serenity::Permissions::SEND_MESSAGES | serenity::Permissions::VIEW_CHANNEL,
            )
    })
}
