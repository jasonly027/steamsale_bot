use std::time::Duration;

use anyhow::Context;
use poise::serenity_prelude::{self as serenity, futures::StreamExt};
use tracing::error;

use crate::{Result, framework, models};

const PAGE_SIZE: usize = 20;

struct Paginator<'a> {
    page_ids: Vec<String>,
    listings: Vec<&'a [models::AppListing]>,
    threshold: i32,
}

impl<'a> Paginator<'a> {
    fn new(id: u64, listings: &'a [models::AppListing], threshold: i32) -> Self {
        let chunked_listings = listings.chunks(PAGE_SIZE).collect::<Vec<_>>();
        let page_ids = (0..chunked_listings.len())
            .map(|i| format!("{id},{i}"))
            .collect();

        Self {
            page_ids,
            listings: chunked_listings,
            threshold,
        }
    }

    fn parse_page_number(str: &str) -> Option<usize> {
        str.rsplit(",").next().map(|s| s.parse().ok())?
    }

    fn get(
        &self,
        page: usize,
    ) -> Option<(
        serenity::CreateEmbed,
        Option<Vec<serenity::CreateActionRow>>,
    )> {
        if page >= self.listings.len() {
            return None;
        }

        let embed = listing_embed(self.listings[page], self.threshold);

        let mut btns = Vec::new();
        if page > 0 {
            let left_id = self.page_ids[page - 1].clone();
            let button = serenity::CreateButton::new(left_id).label("⬅️");
            btns.push(button);
        }
        if page + 1 < self.listings.len() {
            let right_id = self.page_ids[page + 1].clone();
            let btn = serenity::CreateButton::new(right_id).label("➡️");
            btns.push(btn);
        }
        let components = if !btns.is_empty() {
            Some(vec![serenity::CreateActionRow::Buttons(btns)])
        } else {
            None
        };

        Some((embed, components))
    }

    fn ids(&self) -> &[String] {
        &self.page_ids
    }

    fn len(&self) -> usize {
        self.listings.len()
    }
}

/// List apps being tracked and their discount thresholds.
#[poise::command(slash_command, user_cooldown = 3)]
#[tracing::instrument(skip(ctx))]
pub async fn list_apps(ctx: framework::Context<'_>) -> Result<()> {
    ctx.defer().await?;

    // Get listings
    let guild_id: i64 = ctx.guild_id().with_context(|| "Getting guild_id")?.into();
    let repo = &ctx.data().repo;
    let mut listings = repo.junction.get_app_listings(guild_id).await?;
    if listings.is_empty() {
        ctx.say("No apps currently being tracked.").await?;
        return Ok(());
    }
    listings.sort_unstable_by(|a, b| a.app_name.cmp(&b.app_name));

    // Paginate listings
    let models::Discord { sale_threshold, .. } = repo
        .discord
        .find_one_by_guild_id(guild_id)
        .await?
        .with_context(|| anyhow::anyhow!("Missing Discord record for guild_id={guild_id}"))?;
    let paginator = Paginator::new(ctx.id(), &listings, sale_threshold);

    // Send first page
    let (embed, components) = paginator
        .get(0)
        .expect("there should be at least one listing");
    let mut reply = poise::CreateReply::default().embed(embed);
    if let Some(components) = components {
        reply = reply.components(components);
    }
    ctx.send(reply).await?;
    if paginator.len() == 1 {
        return Ok(());
    }

    // Listen for previous/next page requests
    let mut listener = serenity::ComponentInteractionCollector::new(ctx)
        .channel_id(ctx.channel_id())
        .custom_ids(paginator.ids().to_vec())
        .timeout(Duration::from_secs(300))
        .stream();
    while let Some(event) = listener.next().await {
        let id = &event.data.custom_id;
        let Some(requested) = Paginator::parse_page_number(id) else {
            error!(id, "Couldn't parse page number");
            continue;
        };
        let Some((embed, components)) = paginator.get(requested) else {
            error!(
                requested,
                paginator_len = paginator.len(),
                "Request a page out of bounds"
            );
            continue;
        };

        let mut edit = serenity::EditInteractionResponse::new().embed(embed);
        if let Some(components) = components {
            edit = edit.components(components);
        }
        event.edit_response(&ctx, edit).await?;
    }

    Ok(())
}

fn listing_embed(listings: &[models::AppListing], threshold: i32) -> serenity::CreateEmbed {
    let description = listings
        .iter()
        .map(
            |models::AppListing {
                 app_id,
                 app_name,
                 sale_threshold,
             }| {
                match sale_threshold {
                    Some(threshold) => format!("{app_name} {app_id} {threshold}%"),
                    None => format!("{app_name} {app_id}"),
                }
            },
        )
        .collect::<Vec<_>>()
        .join("\n");

    let footer = format!("General Discount Threshold: {threshold}");

    serenity::CreateEmbed::new()
        .title("Tracked Apps")
        .description(description)
        .footer(serenity::CreateEmbedFooter::new(footer))
}
