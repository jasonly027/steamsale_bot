use anyhow::Context;
use poise::serenity_prelude as serenity;

use crate::{
    Error, Result, framework,
    util::{ContextExt, ResLog},
};

const MISSING_PERMISSIONS: &str =
    "Cannot bind to that channel, I am missing `View Channel` and `Send Messages` permissions";

/// Set the channel where alerts are sent. Sends to the server default channel by default.
#[poise::command(slash_command, user_cooldown = 3, on_error=on_error)]
#[tracing::instrument(skip(ctx))]
pub async fn bind(
    ctx: framework::Context<'_>,
    #[channel_types("Text")] channel: serenity::GuildChannel,
) -> Result<()> {
    ctx.defer().await?;

    let perms = ctx
        .permissions_in(&channel)
        .await
        .with_context(|| "Getting bot permissions")?;
    if !perms.contains(serenity::Permissions::VIEW_CHANNEL | serenity::Permissions::SEND_MESSAGES) {
        ctx.say(MISSING_PERMISSIONS).await?;
        return Ok(());
    }

    let repo = &ctx.data().repo.discord;
    repo.set_channel_id(channel.guild_id.into(), channel.id.into())
        .await?;

    ctx.say(format!("Bounded to <#{}>", channel.id)).await?;

    Ok(())
}

async fn on_error(err: poise::FrameworkError<'_, framework::Data, Error>) {
    match err {
        poise::FrameworkError::ArgumentParse { ctx, .. } => {
            ctx.say(MISSING_PERMISSIONS).await.terror().ok();
        }
        _ => framework::on_error(err).await,
    }
}
