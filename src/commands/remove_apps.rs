use anyhow::Context;

use crate::{Result, framework};

/// Remove apps from the tracker.
#[poise::command(slash_command, user_cooldown = 3)]
#[tracing::instrument(skip(ctx))]
pub async fn remove_apps(
    ctx: framework::Context<'_>,
    #[max = 150]
    #[rename = "appids"]
    app_ids: String,
) -> Result<()> {
    let Ok(app_ids) = parse_app_ids(&app_ids) else {
        ctx.say(
            "Failed to parse appids. \
                Please make sure its in the format `<appid1>, <appid2>, ...`. \
                Ex: `1868140, 413150, 3527290`",
        )
        .await?;
        return Ok(());
    };
    ctx.defer().await?;

    let guild_id: i64 = ctx.guild_id().with_context(|| "Getting guild_id")?.into();
    let repo = &ctx.data().repo.junction;
    repo.remove_apps(guild_id, &app_ids).await?;

    ctx.say("Successfully removed apps").await?;

    Ok(())
}

fn parse_app_ids(x: &str) -> Result<Vec<i32>> {
    x.split(",").try_fold(Vec::new(), |mut vec, app| {
        vec.push(app.trim().parse()?);
        Ok(vec)
    })
}
