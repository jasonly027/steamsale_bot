use anyhow::Context;

use crate::{Result, framework, util};

/// Remove apps from the tracker.
#[poise::command(slash_command, user_cooldown = 3)]
#[tracing::instrument(skip(ctx))]
pub async fn remove_apps(
    ctx: framework::Context<'_>,
    #[max = 150]
    #[rename = "appids"]
    app_ids: String,
) -> Result<()> {
    let Ok(app_ids) = util::parse_csv_app_ids(&app_ids) else {
        ctx.say(util::PARSE_APP_IDS_FAIL_MSG).await?;
        return Ok(());
    };
    ctx.defer().await?;

    let guild_id: i64 = ctx.guild_id().with_context(|| "Getting guild_id")?.into();
    let repo = &ctx.data().repo.junction;
    repo.remove_junctions(guild_id, &app_ids).await?;

    ctx.say("Successfully removed apps").await?;

    Ok(())
}
