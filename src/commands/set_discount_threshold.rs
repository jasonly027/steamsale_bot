use anyhow::Context;

use poise::serenity_prelude as serenity;

use crate::{config, framework, repos, util::ToReply, Result};

enum SetThresholdResult {
    Success,
    Fail(Vec<i32>),
    InvalidAppIdString,
}

#[poise::command(slash_command, user_cooldown = 3)]
#[tracing::instrument(skip(ctx))]
pub async fn set_discount_threshold(
    ctx: framework::Context<'_>,
    #[min = 1]
    #[max = 99]
    threshold: i32,
    #[max = 150]
    #[rename = "appids"]
    app_ids: Option<String>,
) -> Result<()> {
    ctx.defer().await?;

    let repo = &ctx.data().repo;
    let guild: i64 = ctx.guild_id().with_context(|| "Getting guild_id")?.into();
    let result = match &app_ids {
        Some(ids) => set_apps_thresholds(repo, guild, threshold, ids).await,
        None => set_guild_threshold(repo, guild, threshold).await,
    }?;

    match result {
        SetThresholdResult::Success => {
            ctx.say(format!(
                "Successfully updated threshold{}",
                if app_ids.is_some() { "s for apps" } else { "" }
            ))
            .await?;
        }

        SetThresholdResult::Fail(failed_ids) => {
            let description = failed_ids
                .iter()
                .map(|x| x.to_string())
                .collect::<Vec<_>>()
                .join("\n");
            let footer = "Please try again. Additionally, double check \
                         they are valid, tracked appids.";

            let reply = serenity::CreateEmbed::new()
                .title("Set Discount Threshold Failed On")
                .description(description)
                .footer(serenity::CreateEmbedFooter::new(footer))
                .color(config::BRAND_DARK_COLOR)
                .to_reply();
            ctx.send(reply).await?;
        }

        SetThresholdResult::InvalidAppIdString => {
            ctx.say(
                "Failed to parse appids. \
                Please make sure its in the format `<appid1>, <appid2>, ...`. \
                Ex: `1868140, 413150, 3527290`",
            )
            .await?;
        }
    }

    Ok(())
}

async fn set_apps_thresholds(
    repo: &repos::Repo,
    guild_id: i64,
    threshold: i32,
    app_ids: &str,
) -> Result<SetThresholdResult> {
    let Ok(app_ids) = parse_app_ids(app_ids) else {
        return Ok(SetThresholdResult::InvalidAppIdString);
    };

    let j_repo = &repo.junction;
    let failed_apps = j_repo.set_thresholds(guild_id, threshold, app_ids).await;
    if !failed_apps.is_empty() {
        return Ok(SetThresholdResult::Fail(failed_apps));
    }

    Ok(SetThresholdResult::Success)
}

async fn set_guild_threshold(
    repo: &repos::Repo,
    guild_id: i64,
    threshold: i32,
) -> Result<SetThresholdResult> {
    let d_repo = &repo.discord;
    d_repo.set_threshold(guild_id, threshold, None).await?;

    Ok(SetThresholdResult::Success)
}

fn parse_app_ids(x: &str) -> Result<Vec<i32>> {
    x.split(",").try_fold(Vec::new(), |mut vec, app| {
        vec.push(app.trim().parse()?);
        Ok(vec)
    })
}
