use std::time::Duration;

use anyhow::Context;
use futures::StreamExt;
use poise::serenity_prelude as serenity;

use crate::{Result, config, framework, models};

const PAGE_SIZE: usize = 10;

/// List apps being tracked and their discount thresholds.
#[poise::command(slash_command, user_cooldown = 3)]
#[tracing::instrument(level = "error", skip(ctx))]
pub async fn list_apps(ctx: framework::Context<'_>) -> Result<()> {
    ctx.defer().await?;
    let guild_id: i64 = ctx.guild_id().with_context(|| "Getting guild_id")?.into();

    let Some(listings) = get_app_listings(&ctx, guild_id).await? else {
        return Ok(());
    };
    let pages = listings.chunks(PAGE_SIZE).collect::<Vec<_>>();
    let sale_threshold = get_guild_sale_threshold(&ctx, guild_id).await?;

    paginate(&ctx, &pages, sale_threshold).await?;

    Ok(())
}

async fn get_app_listings(
    ctx: &framework::Context<'_>,
    guild_id: i64,
) -> Result<Option<Vec<models::AppListing>>> {
    let repo = &ctx.data().repo;
    let mut listings = repo.junction.get_app_listings(guild_id).await?;

    if listings.is_empty() {
        ctx.say("No apps currently being tracked.").await?;
        Ok(None)
    } else {
        listings.sort_unstable_by(|a, b| a.app_name.cmp(&b.app_name));
        Ok(Some(listings))
    }
}

async fn get_guild_sale_threshold(ctx: &framework::Context<'_>, guild_id: i64) -> Result<i32> {
    let repo = &ctx.data().repo.discord;
    let models::Discord { sale_threshold, .. } = repo
        .find_one_by_guild_id(guild_id)
        .await?
        .with_context(|| anyhow::anyhow!("Missing Discord record for guild_id={guild_id}"))?;

    Ok(sale_threshold)
}

async fn paginate(
    ctx: &framework::Context<'_>,
    pages: &[&[models::AppListing]],
    threshold: i32,
) -> Result<()> {
    let id = ctx.id().to_string();
    let prev_button_id = format!("{}prev", id);
    let next_button_id = format!("{}next", id);

    // Send first page
    let reply = {
        let components = serenity::CreateActionRow::Buttons(vec![
            serenity::CreateButton::new(&prev_button_id).emoji('◀'),
            serenity::CreateButton::new(&next_button_id).emoji('▶'),
        ]);
        poise::CreateReply::default()
            .embed(create_embed(0, pages, threshold))
            .components(vec![components])
    };
    ctx.send(reply).await?;

    // Handle page turns
    let mut current_page = 0;
    let mut listener = serenity::ComponentInteractionCollector::new(ctx)
        .filter(move |ev| ev.data.custom_id.starts_with(&id))
        .timeout(Duration::from_secs(300))
        .stream();
    while let Some(event) = listener.next().await {
        let action = &event.data.custom_id;
        if *action == next_button_id {
            current_page += 1;
            if current_page >= pages.len() {
                current_page = 0;
            }
        } else if *action == prev_button_id {
            current_page = current_page.checked_sub(1).unwrap_or(pages.len() - 1);
        } else {
            continue;
        }

        let update = serenity::CreateInteractionResponse::UpdateMessage(
            serenity::CreateInteractionResponseMessage::new().embed(create_embed(
                current_page,
                pages,
                threshold,
            )),
        );
        event.create_response(&ctx, update).await?;
    }

    Ok(())
}

fn create_embed(
    current_page: usize,
    pages: &[&[models::AppListing]],
    threshold: i32,
) -> serenity::CreateEmbed {
    let description = pages[current_page]
        .iter()
        .map(
            |models::AppListing {
                 app_id,
                 app_name,
                 sale_threshold,
             }| {
                match sale_threshold {
                    Some(threshold) => format!("{app_name} ({app_id}) ({threshold}%)"),
                    None => format!("{app_name} ({app_id})"),
                }
            },
        )
        .collect::<Vec<_>>()
        .join("\n");

    let footer = format!("General Discount Threshold: {threshold}%");

    serenity::CreateEmbed::new()
        .title(format!("Tracked Apps {}/{}", current_page + 1, pages.len()))
        .description(description)
        .footer(serenity::CreateEmbedFooter::new(footer))
        .color(config::BRAND_DARK_COLOR)
}
