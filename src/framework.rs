use std::sync::Arc;

use poise::serenity_prelude as serenity;
use tracing::{error, info};

use crate::{Error, Result, commands, database, repos, util};

pub struct Data {
    pub repo: repos::Repo,
}

pub type Context<'a> = poise::Context<'a, Data, Error>;

pub async fn run(token: &str, dev_guild: Option<u64>) -> Result<()> {
    let framework = poise::Framework::<Data, Error>::builder()
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                register_commands(ctx, &framework.options().commands, dev_guild).await?;
                create_data().await
            })
        })
        .options(poise::FrameworkOptions {
            commands: vec![
                commands::test(),
                commands::help(),
                commands::bind(),
                commands::set_discount_threshold(),
                commands::list_apps(),
                commands::clear_apps(),
                commands::remove_apps(),
            ],
            command_check: Some(|ctx| Box::pin(command_check(ctx))),
            on_error: |err| Box::pin(on_error(err)),
            ..Default::default()
        })
        .build();

    let intents = serenity::GatewayIntents::non_privileged();

    let client = serenity::ClientBuilder::new(token, intents)
        .framework(framework)
        .await;

    info!("Starting framework");
    Ok(client?.start().await?)
}

async fn register_commands(
    ctx: &serenity::Context,
    commands: &[poise::Command<Data, Error>],
    dev_guild: Option<u64>,
) -> Result<()> {
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

async fn create_data() -> Result<Data> {
    info!("Connecting to database");
    let uri: String = util::env_var("MONGODB_URI")?;
    let name: String = util::env_var("MONGODB_DBNAME")?;
    let db = database::Database::new(&uri, name).await?;

    let repo = repos::Repo::new(Arc::new(db));
    Ok(Data { repo })
}

async fn command_check(ctx: Context<'_>) -> Result<bool> {
    if ctx.guild_id().is_some() {
        return Ok(true);
    }
    ctx.say("Commands must be used in a server").await?;
    Ok(false)
}

pub async fn on_error(err: poise::FrameworkError<'_, Data, Error>) {
    error!(%err, "Unexpected error");

    if let Some(ctx) = err.ctx() {
        ctx.say("An unexpected error has occured...")
            .await
            .inspect_err(|err| error!(?err, "Failed to send unexpected error message"))
            .ok();
    }
}
