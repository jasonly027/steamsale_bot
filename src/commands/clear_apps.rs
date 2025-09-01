use std::time::Duration;

use anyhow::{Context, bail};
use poise::serenity_prelude::{self as serenity, futures::StreamExt};
use strum::IntoEnumIterator;

use crate::{Result, config, framework};

/// Remove all apps from the tracker.
#[poise::command(slash_command, user_cooldown = 3)]
#[tracing::instrument(skip(ctx))]
pub async fn clear_apps(ctx: framework::Context<'_>) -> Result<()> {
    let id = ctx.id().to_string();

    ctx.send(create_confirmation_dropdown(&id)).await?;

    let Some((event, confirmed)) = get_response(&ctx, id).await? else {
        return Ok(());
    };

    let description = if confirmed {
        let guild_id: i64 = ctx.guild_id().with_context(|| "Getting guild_id")?.into();
        let repo = &ctx.data().repo.junction;
        repo.clear_junctions(guild_id).await?;

        "Successfully cleared.".to_string()
    } else {
        "Aborting operation...".to_string()
    };

    event.edit_response(&ctx, create_edit(description)).await?;

    Ok(())
}

#[derive(strum_macros::Display, strum_macros::EnumIter, strum_macros::EnumString)]
enum ConfirmOptions {
    Yes,
    No,
}

fn create_confirmation_dropdown(id: impl Into<String>) -> poise::CreateReply {
    let embed = serenity::CreateEmbed::new()
        .title("Clear Tracked Apps?")
        .description("Are you sure you want to clear tracked apps?")
        .color(config::BRAND_DARK_COLOR);

    let button = serenity::CreateSelectMenu::new(
        id,
        serenity::CreateSelectMenuKind::String {
            options: ConfirmOptions::iter()
                .map(|x| serenity::CreateSelectMenuOption::new(x.to_string(), x.to_string()))
                .collect(),
        },
    )
    .min_values(1)
    .max_values(1);

    poise::CreateReply::default()
        .embed(embed)
        .components(vec![serenity::CreateActionRow::SelectMenu(button)])
        .ephemeral(true)
}

async fn get_response(
    ctx: &framework::Context<'_>,
    id: impl Into<String>,
) -> Result<Option<(serenity::ComponentInteraction, bool)>> {
    let Some(event) = listen_for_response(ctx, id).await else {
        return Ok(None);
    };
    event
        .create_response(&ctx, serenity::CreateInteractionResponse::Acknowledge)
        .await?;
    let confirmed = parse_response(&event)?;

    Ok(Some((event, confirmed)))
}

async fn listen_for_response(
    ctx: &framework::Context<'_>,
    id: impl Into<String>,
) -> Option<serenity::ComponentInteraction> {
    let mut listener = serenity::ComponentInteractionCollector::new(ctx)
        .author_id(ctx.author().id)
        .channel_id(ctx.channel_id())
        .custom_ids(vec![id.into()])
        .timeout(Duration::from_secs(300))
        .stream();

    listener.next().await
}

fn parse_response(event: &serenity::ComponentInteraction) -> Result<bool> {
    let serenity::ComponentInteractionDataKind::StringSelect { values } = &event.data.kind else {
        bail!("Not StringSelect. event: {event:?}");
    };
    if values.len() != 1 {
        bail!("Unexpected values: event: {event:?}");
    }
    match values[0].parse()? {
        ConfirmOptions::Yes => Ok(true),
        ConfirmOptions::No => Ok(false),
    }
}

fn create_edit(description: impl Into<String>) -> serenity::EditInteractionResponse {
    let embed = serenity::CreateEmbed::new()
        .title("Clear Tracked Apps?")
        .description(description.into())
        .color(config::BRAND_DARK_COLOR);

    serenity::EditInteractionResponse::new()
        .embed(embed)
        .components(Vec::new())
}
