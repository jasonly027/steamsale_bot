use std::time::Duration;

use anyhow::Context;
use poise::serenity_prelude::{self as serenity, futures::StreamExt};
use strum::IntoEnumIterator;

use crate::{framework, util::ResLog, Result};

#[derive(strum_macros::Display, strum_macros::EnumIter, strum_macros::EnumString)]
enum ConfirmOptions {
    Yes,
    No,
}

/// Remove all apps from the tracker.
#[poise::command(slash_command, user_cooldown = 3)]
#[tracing::instrument(skip(ctx))]
pub async fn clear_apps(ctx: framework::Context<'_>) -> Result<()> {
    // Ask for confirmation
    let id = ctx.id().to_string();
    let reply = confirmation_dropdown(&id);
    ctx.send(reply).await?;

    // Listen for confirmation
    let mut listener = serenity::ComponentInteractionCollector::new(ctx)
        .author_id(ctx.author().id)
        .channel_id(ctx.channel_id())
        .custom_ids(vec![id])
        .timeout(Duration::from_secs(300))
        .stream();
    let Some(event) = listener.next().await else {
        return Ok(());
    };
    event
        .create_response(&ctx, serenity::CreateInteractionResponse::Acknowledge)
        .await?;

    // Clear apps or abort
    let confirmed = parse_confirmation(&event).terror()?;
    if confirmed {
        let guild_id: i64 = ctx.guild_id().with_context(|| "Getting guild_id")?.into();
        let repo = &ctx.data().repo.junction;
        repo.clear_apps(guild_id).await?;

        let embed = serenity::CreateEmbed::new()
            .title("Clear Tracked Apps?")
            .description("Successfully cleared.");
        let edit = serenity::EditInteractionResponse::new()
            .embed(embed)
            .components(Vec::new());
        event.edit_response(&ctx, edit).await?;
    } else {
        let embed = serenity::CreateEmbed::new()
            .title("Clear Tracked Apps?")
            .description("Aborting operation...");
        let edit = serenity::EditInteractionResponse::new()
            .embed(embed)
            .components(Vec::new());
        event.edit_response(&ctx, edit).await?;
    }

    Ok(())
}

fn confirmation_dropdown(id: &str) -> poise::CreateReply {
    let embed = serenity::CreateEmbed::new()
        .title("Clear Tracked Apps?")
        .description("Are you sure you want to clear tracked apps?");

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

fn parse_confirmation(event: &serenity::ComponentInteraction) -> Result<bool> {
    let serenity::ComponentInteractionDataKind::StringSelect { ref values } = event.data.kind
    else {
        return Err(anyhow::anyhow!("Not StringSelect"));
    };
    if values.len() != 1 {
        return Err(anyhow::anyhow!("Unexpected values: {values:?}"));
    }
    match values[0].parse()? {
        ConfirmOptions::Yes => Ok(true),
        ConfirmOptions::No => Ok(false),
    }
}
