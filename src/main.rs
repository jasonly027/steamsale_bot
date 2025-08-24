use tracing_subscriber::EnvFilter;

use crate::util::ResLog;

mod commands;
mod database;
mod framework;
mod util;

type Error = anyhow::Error;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_new("info,serenity=WARN")?)
        .init();
    dotenvy::dotenv().warn();

    let token: String = util::env_var("DISCORD_TOKEN")?;
    let dev_guild: Option<u64> = match util::env_var("DISCORD_DEVGUILDID") {
        Ok(x) => Some(x),
        Err(util::EnvVarError::InvalidOrMissingKey { .. }) => None,
        Err(err) => Err(err)?,
    };

    framework::run(&token, dev_guild).await?;

    Ok(())
}
