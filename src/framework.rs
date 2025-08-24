use anyhow::Ok;
use poise::{FrameworkOptions, serenity_prelude as serenity};
use tracing::info;

use crate::{Error, commands, database, util};

pub struct Data {
    db: database::Database,
}

pub type Context<'a> = poise::Context<'a, Data, Error>;

pub async fn run(token: &str, dev_guild: Option<u64>) -> Result<(), Error> {
    let framework = poise::Framework::<Data, Error>::builder()
        .setup(move |ctx, _ready, framework| {
            Box::pin(async move {
                register_commands(ctx, &framework.options().commands, dev_guild).await?;

                info!("Connecting to database");
                let uri: String = util::env_var("MONGODB_URI")?;
                Ok(Data {
                    db: database::Database::new(&uri).await?,
                })
            })
        })
        .options(FrameworkOptions {
            commands: vec![commands::test()],
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
) -> Result<(), Error> {
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
