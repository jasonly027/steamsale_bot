use crate::util::ResLog;

mod commands;
mod database;
mod framework;
mod models;
mod repos;
mod util;

type StdResult<T, E> = std::result::Result<T, E>;
type Result<T> = StdResult<T, Error>;
type Error = anyhow::Error;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::try_new(
            "info,serenity=WARN",
        )?)
        .init();
    dotenvy::dotenv().warn().ok();

    let token: String = util::env_var("DISCORD_TOKEN")?;
    let dev_guild: Option<u64> = match util::env_var("DISCORD_DEVGUILDID") {
        Ok(x) => Some(x),
        Err(util::EnvVarError::InvalidOrMissingKey { .. }) => None,
        Err(err) => Err(err)?,
    };

    framework::run(&token, dev_guild).await?;

    Ok(())
}
