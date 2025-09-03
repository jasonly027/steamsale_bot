//! This module provides [`run`] for starting the bot and internally
//! sets the bot's configuration.

use std::sync::Arc;

use derivative::Derivative;
use poise::serenity_prelude as serenity;
use tracing::{error, info};

use crate::{Error, Result, StdResult, commands, events, repos, steam, util::PoiseData};

/// Custom data that is provided to all contexts.
#[derive(Derivative)]
#[derivative(Debug)]
pub struct Data {
    pub http: Arc<serenity::Http>,
    /// A handle to all the repositories.
    #[derivative(Debug = "ignore")]
    pub repo: repos::Repo,
    /// A handle to the Steam client.
    #[derivative(Debug = "ignore")]
    pub steam: steam::Client,
}

impl serenity::prelude::TypeMapKey for Data {
    type Value = Arc<Data>;
}

pub type Context<'a> = poise::Context<'a, Arc<Data>, Error>;

pub async fn run(token: &str, dev_guild: Option<u64>) -> Result<()> {
    let framework = poise::Framework::<Arc<Data>, Error>::builder()
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                info!("Setting up poise");
                register_commands(ctx, &framework.options().commands, dev_guild).await?;

                Ok(ctx.poise_data_unwrap().await)
            })
        })
        .options(poise::FrameworkOptions {
            commands: vec![
                commands::help(),
                commands::bind(),
                commands::set_discount_threshold(),
                commands::list_apps(),
                commands::clear_apps(),
                commands::remove_apps(),
                commands::add_apps(),
                commands::search(),
            ],
            command_check: Some(|ctx| Box::pin(command_check(ctx))),
            on_error: |err| Box::pin(on_error(err)),
            ..Default::default()
        })
        .build();

    let intents = serenity::GatewayIntents::non_privileged();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .event_handler(events::SerenityReady)
        .event_handler(events::GuildAvailable)
        .event_handler(events::RemovedFromGuild)
        .await;

    info!("Starting framework");
    Ok(client?.start().await?)
}

async fn register_commands(
    ctx: &serenity::Context,
    commands: &[poise::Command<Arc<Data>, Error>],
    dev_guild: Option<u64>,
) -> StdResult<(), serenity::Error> {
    match dev_guild {
        Some(guild_id) => {
            info!("Registering commands in development guild {}", guild_id);
            poise::builtins::register_in_guild(ctx, commands, serenity::GuildId::new(guild_id))
                .await?;
        }
        None => {
            info!("Registering commands globally");
            poise::builtins::register_globally(ctx, commands).await?;
        }
    }

    Ok(())
}

async fn command_check(ctx: Context<'_>) -> Result<bool> {
    if ctx.guild_id().is_some() {
        return Ok(true);
    }
    ctx.say("Commands must be used in a server").await?;
    Ok(false)
}

pub async fn on_error(err: poise::FrameworkError<'_, Arc<Data>, Error>) {
    if let poise::FrameworkError::CooldownHit {
        remaining_cooldown,
        ctx,
        ..
    } = err
    {
        ctx.say(format!(
            "Command used too quickly! Please wait {} seconds before retrying.",
            remaining_cooldown.as_secs()
        ))
        .await
        .inspect_err(|err| error!(?err, "Failed to send cooldown hit message"))
        .ok();
        return;
    }

    error!(?err, "Unexpected error");

    if let Some(ctx) = err.ctx() {
        ctx.say("An unexpected error has occured...")
            .await
            .inspect_err(|err| error!(?err, "Failed to send unexpected error message"))
            .ok();
    }
}
