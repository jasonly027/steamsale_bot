use std::time::Duration;

use anyhow::{Context, bail};
use futures::StreamExt;
use poise::serenity_prelude as serenity;

use crate::{Result, config, framework, models, repos, steam};

/// Search for an app to add to the tracker.
#[poise::command(slash_command, user_cooldown = 3)]
#[tracing::instrument(level = "error", skip(ctx))]
pub async fn search(ctx: framework::Context<'_>, #[max_length = 150] query: String) -> Result<()> {
    ctx.defer().await?;

    let search_results = ctx.data().steam.search_apps(&query).await?;
    if search_results.is_empty() {
        ctx.say("Found no matching apps for query").await?;
        return Ok(());
    }

    let id = ctx.id().to_string();
    ctx.send(search_result_dropdown(&id, &search_results))
        .await?;
    let Some((event, app_id)) = get_response(&ctx, id).await? else {
        return Ok(());
    };
    let Some(app) = get_app(&ctx, &event, app_id).await? else {
        return Ok(());
    };
    let guild_id: i64 = ctx.guild_id().with_context(|| "Getting guild_id")?.into();
    add_app_to_db(&ctx.data().repo, guild_id, &app).await?;

    event
        .edit_response(&ctx, create_edit("Successfully added app."))
        .await?;

    Ok(())
}

const CANCEL_SEARCH_OPTION: &str = "--- Cancel Adding App ---";

fn search_result_dropdown(id: &str, results: &[steam::SearchResult]) -> poise::CreateReply {
    let embed = serenity::CreateEmbed::new()
        .title("Search")
        .description("Select an app below.")
        .footer(serenity::CreateEmbedFooter::new(
            "Results may include non-addable apps. \
Make sure your choice is either priced or yet to be released.",
        ))
        .color(config::BRAND_DARK_COLOR);

    let mut options = results
        .iter()
        .map(|r| {
            let label = format!("{} ({})", r.name, r.app_id);
            let value = r.app_id.to_string();
            serenity::CreateSelectMenuOption::new(label, value)
        })
        .collect::<Vec<_>>();
    options.push(serenity::CreateSelectMenuOption::new(
        CANCEL_SEARCH_OPTION,
        CANCEL_SEARCH_OPTION,
    ));
    let dropdown =
        serenity::CreateSelectMenu::new(id, serenity::CreateSelectMenuKind::String { options })
            .min_values(1)
            .max_values(1)
            .placeholder("Search Results");

    poise::CreateReply::default()
        .embed(embed)
        .components(vec![serenity::CreateActionRow::SelectMenu(dropdown)])
}

async fn get_response(
    ctx: &framework::Context<'_>,
    id: impl Into<String>,
) -> Result<Option<(serenity::ComponentInteraction, i32)>> {
    let Some(event) = listen_for_response(ctx, id).await else {
        return Ok(None);
    };
    event
        .create_response(&ctx, serenity::CreateInteractionResponse::Acknowledge)
        .await?;

    let Some(app_id) = parse_response(&event)? else {
        let edit = create_edit("Cancelled adding app.");
        event.edit_response(&ctx, edit).await?;
        return Ok(None);
    };

    Ok(Some((event, app_id)))
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

fn parse_response(event: &serenity::ComponentInteraction) -> Result<Option<i32>> {
    let serenity::ComponentInteractionDataKind::StringSelect { values } = &event.data.kind else {
        bail!("Not StringSelect. event: {event:?}");
    };
    let Some(choice) = values.first() else {
        bail!("Unexpected values: event: {event:?}");
    };

    if choice == CANCEL_SEARCH_OPTION {
        Ok(None)
    } else {
        Ok(Some(choice.parse()?))
    }
}

async fn get_app(
    ctx: &framework::Context<'_>,
    event: &serenity::ComponentInteraction,
    app_id: i32,
) -> Result<Option<steam::App>> {
    let steam = &ctx.data().steam;

    let Some(app) = steam.app_details(app_id).await? else {
        let edit = create_edit(
            "Couldn't get more details on the app. \
It may not be available in the US.",
        );
        event.edit_response(&ctx, edit).await?;
        return Ok(None);
    };
    if app.is_free && !app.release_date.coming_soon {
        let edit = create_edit("Invalid app. App is neither priced or yet to be released.");
        event.edit_response(&ctx, edit).await?;
        return Ok(None);
    }

    Ok(Some(app))
}

async fn add_app_to_db(
    repo: &repos::Repo,
    guild_id: i64,
    app: &steam::App,
) -> mongodb::error::Result<()> {
    let mut session = repo.start_session().await?;
    session.start_transaction().await?;

    let junction = models::Junction {
        id: Default::default(),
        app_id: app.app_id,
        server_id: guild_id,
        is_trailing_sale_day: false,
        coming_soon: app.release_date.coming_soon,
        sale_threshold: None,
    };
    repo.junction
        .add_junction(&junction)
        .session(&mut session)
        .await?;
    repo.apps
        .upsert_app(&app.clone().into())
        .session(&mut session)
        .await?;

    session.commit_transaction().await?;

    Ok(())
}

fn create_edit(description: impl Into<String>) -> serenity::EditInteractionResponse {
    let embed = serenity::CreateEmbed::new()
        .title("Search App")
        .description(description.into())
        .color(config::BRAND_DARK_COLOR);

    serenity::EditInteractionResponse::new()
        .embed(embed)
        .components(Vec::new())
}
