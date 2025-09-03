use std::{sync::Arc, time::Duration};

use anyhow::Context;
use futures::StreamExt;
use once_map::OnceMap;
use poise::serenity_prelude as serenity;
use tracing::{error, info, warn};

use crate::{
    Result, StdResult, config, database,
    framework::{self, Data},
    models, repos, steam,
    util::{self, PoiseData},
};

pub struct SerenityReady;

#[serenity::async_trait]
impl serenity::EventHandler for SerenityReady {
    /// Initialize framework data and start check apps loop.
    async fn ready(&self, ctx: serenity::Context, _ready: serenity::Ready) {
        init_data(&ctx).await;
        init_check_apps(ctx.poise_data_unwrap().await);
    }
}

async fn init_data(ctx: &serenity::Context) {
    // We place poise data in the serenity store so it's still accessible
    // in places where we only have the serenity ctx and not poise ctx.
    // For example, serenity::EventHandler's.

    info!("Writing poise data to serenity store");
    let mut store = ctx.data.write().await;
    let data = Arc::new(create_data(ctx.http.clone()).await.unwrap());
    store.insert::<framework::Data>(data);
}

async fn create_data(http: Arc<serenity::Http>) -> Result<Data> {
    let repo = {
        let uri: String = util::env_var("MONGODB_URI")?;
        let name: String = util::env_var("MONGODB_DBNAME")?;
        let db = database::Database::new(&uri, name).await?;

        repos::Repo::new(Arc::new(db))
    };

    let steam = {
        let store = "https://store.steampowered.com";
        let community = "https://steamcommunity.com";

        steam::Client::new(store, community)
    };

    Ok(Data { http, repo, steam })
}

fn init_check_apps(ctx: Arc<framework::Data>) {
    tokio::spawn(async move {
        loop {
            tokio::time::sleep_until(checking_time()).await;

            info!("Checking apps...");
            if let Err(err) = check_apps(&ctx).await {
                error!(?err, "Failed to check apps");
            }
        }
    });
}

fn checking_time() -> tokio::time::Instant {
    let now = chrono::Utc::now().naive_utc();

    let today_17 = now
        .date()
        .and_hms_opt(17, 0, 0)
        .expect("should be valid hms");

    let next_17 = if now < today_17 {
        today_17
    } else {
        today_17 + chrono::Duration::days(1)
    };

    let duration = (next_17 - now)
        .to_std()
        .expect("should be positive duration");

    tokio::time::Instant::now() + duration
}

#[tracing::instrument(level = "error", skip(ctx))]
async fn check_apps(ctx: &framework::Data) -> Result<()> {
    let apps_repo = &ctx.repo.apps;
    let junc_repo = &ctx.repo.junction;

    apps_repo
        .remove_orphans()
        .await
        .inspect_err(|err| error!(?err, "Failed to remove orphans"))
        .ok();

    let discord_cache = OnceMap::new();
    for app_id in apps_repo.get_app_ids().await? {
        let app = match get_app(&ctx.steam, app_id).await {
            Ok(Some(app)) => app,
            Ok(None) => {
                error!(app_id, "App not found");
                continue;
            }
            Err(err) => {
                error!(?err, app_id, "Failed to fetch app");
                continue;
            }
        };

        junc_repo
            .get_junctions(app_id)
            .await?
            .for_each_concurrent(None, |junction| async {
                match junction {
                    Ok(j) => {
                        let guild_id = j.server_id;
                        let app_id = j.app_id;

                        if let Err(err) = notify_guild(ctx, j, &discord_cache, &app).await {
                            error!(?err, "Failed to notify guild");
                            return;
                        }
                        if app.is_free && !app.release_date.coming_soon {
                            if let Err(err) = junc_repo.remove_junction(guild_id, app_id).await {
                                error!(?err, "Failed to remove free and released app");
                            }
                        }
                    }
                    Err(err) => error!(?err, "Failed to get junction"),
                };
            })
            .await;
    }

    Ok(())
}

async fn get_app(
    steam: &steam::Client,
    app_id: i32,
) -> StdResult<Option<steam::App>, steam::FetchError> {
    let mut tries = 0;
    const MAX_TRIES: u32 = 5;
    const RETRY_TIMEOUT: u64 = 300;

    let mut app_res = steam.app_details(app_id).await;
    while matches!(&app_res, Err(err) if err.is_rate_limited()) {
        info!("Steam rate-limit hit. Temporarily backing off...");
        tokio::time::sleep(Duration::from_secs(RETRY_TIMEOUT)).await;
        app_res = steam.app_details(app_id).await;

        if tries >= MAX_TRIES {
            warn!("Rate limited too many times. No longer retrying app {app_id}");
            break;
        }
        tries += 1;
    }

    app_res
}

async fn notify_guild(
    ctx: &framework::Data,
    mut junction: models::Junction,
    discord_cache: &OnceMap<i64, Arc<models::Discord>>,
    app: &steam::App,
) -> Result<()> {
    let discord = get_discord(&ctx.repo, discord_cache, junction.server_id).await?;
    let channel = serenity::ChannelId::new(discord.channel_id.try_into()?);

    if junction.coming_soon && !app.release_date.coming_soon {
        channel
            .send_message(
                &ctx.http,
                serenity::CreateMessage::new().embed(released_embed(app)),
            )
            .await?;
    }

    let threshold = junction.sale_threshold.unwrap_or(discord.sale_threshold);
    let is_significant_discount = app
        .price_overview
        .as_ref()
        .is_some_and(|p| p.discount_percent >= threshold);

    if is_significant_discount && !junction.is_trailing_sale_day {
        channel
            .send_message(
                &ctx.http,
                serenity::CreateMessage::new().embed(sale_embed(app)),
            )
            .await?;
    }

    junction.coming_soon = app.release_date.coming_soon;
    junction.is_trailing_sale_day = is_significant_discount;
    ctx.repo.junction.update_junction(&junction).await?;

    Ok(())
}

async fn get_discord<'a>(
    repo: &repos::Repo,
    discord_cache: &'a OnceMap<i64, Arc<models::Discord>>,
    guild_id: i64,
) -> Result<&'a models::Discord> {
    if let Some(d) = discord_cache.get(&guild_id) {
        return Ok(d);
    }

    // Separate tasks could race to try initializing, but since the concurrent tasks stem from
    // junction records with unique guild_ids, it shouldn't be a problem.
    let d = repo
        .discord
        .get_guild(guild_id)
        .await?
        .with_context(|| "Discord record doesn't exist but it's junction record does")?;
    Ok(discord_cache.insert(guild_id, |_| Arc::new(d)))
}

fn released_embed(app: &steam::App) -> serenity::CreateEmbed {
    let title = format!("{} has released on Steam!", app.name);
    let url = format!("https://store.steampowered.com/app/{}", app.app_id);

    let price = app
        .price_overview
        .as_ref()
        .map(|p| p.final_formatted.clone())
        .unwrap_or("Free".to_string());

    let mut fields = vec![("Price", price, false)];
    if !app.description.is_empty() {
        fields.push(("Description", app.description.clone(), false));
    }

    serenity::CreateEmbed::new()
        .title(title)
        .url(url)
        .image(app.header_image.clone())
        .fields(fields)
        .color(config::BRAND_DARK_COLOR)
}

fn sale_embed(app: &steam::App) -> serenity::CreateEmbed {
    let price = app
        .price_overview
        .as_ref()
        .expect("should have checked before called this fn");

    let title = format!("{} is {}% off!", app.name, price.discount_percent);
    let url = format!("https://store.steampowered.com/app/{}", app.app_id);

    let mut fields = vec![
        ("Original Price", price.initial_formatted.clone(), true),
        ("Sale Price", price.final_formatted.clone(), true),
    ];
    if let Some(recs) = &app.recommendations {
        fields.push(("Reviews", recs.total.to_string(), true));
    }
    if !app.description.is_empty() {
        fields.push(("Description", app.description.clone(), false));
    }

    serenity::CreateEmbed::new()
        .title(title)
        .url(url)
        .image(&app.header_image)
        .fields(fields)
        .color(sale_color(price.discount_percent))
}

fn sale_color(discount_percent: i32) -> u32 {
    if discount_percent <= 5 {
        0x0bff33
    } else if discount_percent <= 10 {
        0x44fdd2
    } else if discount_percent <= 15 {
        0x44fdfd
    } else if discount_percent <= 20 {
        0x44dbfd
    } else if discount_percent <= 25 {
        0x44b6fd
    } else if discount_percent <= 30 {
        0x448bfd
    } else if discount_percent <= 35 {
        0x445afd
    } else if discount_percent <= 40 {
        0x8544fd
    } else if discount_percent <= 45 {
        0xb044fd
    } else if discount_percent <= 50 {
        0xe144fd
    } else if discount_percent <= 55 {
        0xfd44de
    } else if discount_percent <= 60 {
        0xff23a7
    } else if discount_percent <= 99 {
        0xff0000
    } else {
        0xFFFFFF
    }
}
