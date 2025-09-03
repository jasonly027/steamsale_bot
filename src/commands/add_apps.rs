use anyhow::Context;
use futures::{StreamExt, stream};
use poise::serenity_prelude as serenity;
use tracing::error;

use crate::{
    Result, config, framework, models, repos,
    steam::{self, FetchError},
    util::{self, ToReply},
};

/// Adds apps to the tracker.
#[poise::command(slash_command, user_cooldown = 3)]
#[tracing::instrument(level = "error", skip(ctx))]
pub async fn add_apps(
    ctx: framework::Context<'_>,
    #[rename = "appids"]
    #[max_length = 75]
    app_ids: String,
    #[min = 1]
    #[max = 99]
    threshold: Option<i32>,
) -> Result<()> {
    let Ok(app_ids) = util::parse_csv_app_ids(&app_ids) else {
        ctx.say(util::PARSE_APP_IDS_FAIL_MSG).await?;
        return Ok(());
    };
    ctx.defer().await?;

    let guild_id: i64 = ctx.guild_id().with_context(|| "Getting guild_id")?.into();

    let (apps, rate_limited) = fetch_apps(&ctx.data().steam, app_ids.clone()).await;
    let added_apps = add_apps_to_db(&ctx.data().repo, guild_id, &apps, threshold).await;
    let failed_apps = app_ids
        .into_iter()
        .filter(|&app_id| !added_apps.iter().any(|app| app.app_id == app_id))
        .collect::<Vec<i32>>();

    let reply = create_reply(added_apps, failed_apps, rate_limited);
    ctx.send(reply).await?;

    Ok(())
}

async fn fetch_apps(steam: &steam::Client, app_ids: Vec<i32>) -> (Vec<steam::App>, bool) {
    const FETCH_BUFFER_SIZE: usize = 5;

    let fetches = stream::iter(app_ids.into_iter().map(|app_id| {
        let steam = steam.clone();
        async move { (app_id, steam.app_details(app_id).await) }
    }));
    let mut fetch_stream = fetches.buffer_unordered(FETCH_BUFFER_SIZE);

    let mut apps = Vec::new();
    let mut rate_limited = false;
    while let Some((app_id, app)) = fetch_stream.next().await {
        match app {
            Ok(Some(app)) => {
                if !app.is_free || app.release_date.coming_soon {
                    apps.push(app);
                }
            }
            Ok(None) => { /* App doesn't exist for given app_id */ }
            Err(FetchError::Http(err))
                if err.status() == Some(reqwest::StatusCode::TOO_MANY_REQUESTS) =>
            {
                rate_limited = true;
                break;
            }
            Err(err) => error!(app_id, ?err, "Failed to fetch app"),
        }
    }

    (apps, rate_limited)
}

async fn add_apps_to_db<'a>(
    repo: &repos::Repo,
    guild_id: i64,
    apps: &'a [steam::App],
    threshold: Option<i32>,
) -> Vec<&'a steam::App> {
    let mut added_apps = Vec::new();
    for app in apps {
        let mut session = match repo.start_session().await {
            Ok(x) => x,
            Err(err) => {
                error!(?err, "Failed to create session");
                continue;
            }
        };
        if let Err(err) = session.start_transaction().await {
            error!(?err, "Failed to start transaction");
            continue;
        };

        let junction = models::Junction {
            id: Default::default(),
            app_id: app.app_id,
            server_id: guild_id,
            is_trailing_sale_day: false,
            coming_soon: app.release_date.coming_soon,
            sale_threshold: threshold,
        };

        if repo
            .junction
            .add_junction_if_not_exists(&junction)
            .session(&mut session)
            .await
            .inspect_err(|err| error!(?err, "Failed to add junction"))
            .is_err()
            || repo
                .apps
                .upsert_app(&app.clone().into())
                .session(&mut session)
                .await
                .inspect_err(|err| error!(?err, "Failed to upsert app"))
                .is_err()
        {
            continue;
        };

        match session.commit_transaction().await {
            Ok(_) => added_apps.push(app),
            Err(err) => error!(?err, "Failed to commit transaction"),
        }
    }

    added_apps
}

fn create_reply(
    added_apps: Vec<&steam::App>,
    failed_apps: Vec<i32>,
    rate_limited: bool,
) -> poise::CreateReply {
    let mut embed = serenity::CreateEmbed::new()
        .title("Add Apps")
        .color(config::BRAND_DARK_COLOR);

    if !added_apps.is_empty() {
        let success_body = added_apps
            .iter()
            .map(|app| format!("{} ({})", app.name, app.app_id))
            .collect::<Vec<String>>()
            .join("\n");
        embed = embed.field("Successfully Added", success_body, false);
    }
    if !failed_apps.is_empty() {
        let fail_body = failed_apps
            .iter()
            .map(|id| id.to_string())
            .collect::<Vec<String>>()
            .join("\n");
        embed = embed.field("Failed to Add", fail_body, false);

        let footer = if rate_limited {
            "Bot was rate-limited by Steam. Please wait a few minutes before trying failed apps again!"
        } else {
            "Make sure failed apps are valid and either priced or yet to be released."
        };

        embed = embed.footer(serenity::CreateEmbedFooter::new(footer));
    }

    embed.to_reply()
}
