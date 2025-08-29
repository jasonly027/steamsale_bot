use anyhow::Context;
use poise::serenity_prelude as serenity;
use tracing::{error, warn};

use crate::{Result, StdResult};

pub trait ResLog<T, E> {
    fn twarn(self) -> StdResult<T, E>;
    fn terror(self) -> StdResult<T, E>;
}

impl<T, E: std::fmt::Display> ResLog<T, E> for StdResult<T, E> {
    #[track_caller]
    fn twarn(self) -> StdResult<T, E> {
        self.inspect_err(|err| {
            let loc = std::panic::Location::caller();
            warn!(
                %err,
                "Error at {}",
                loc_as_str(loc)
            );
        })
    }

    #[track_caller]
    fn terror(self) -> StdResult<T, E> {
        self.inspect_err(|err| {
            let loc = std::panic::Location::caller();
            error!(
                %err,
                "Error at {}",
                loc_as_str(loc)
            );
        })
    }
}

fn loc_as_str(loc: &std::panic::Location<'_>) -> String {
    format!(
        "{}:{}:{}",
        loc.file().replace("\\", "/"),
        loc.line(),
        loc.column()
    )
}

#[derive(Debug, thiserror::Error)]
pub enum EnvVarError {
    #[error("Invalid/Missing {key} or {key}_FILE")]
    InvalidOrMissingKey { key: String },
    #[error("Invalid value for {key}: {err}")]
    InvalidValue { key: String, err: String },
}

pub fn env_var<T: std::str::FromStr>(key: &str) -> StdResult<T, EnvVarError>
where
    T::Err: std::fmt::Display,
{
    match std::env::var(key) {
        Ok(x) => x.parse().map_err(|err: T::Err| EnvVarError::InvalidValue {
            key: key.to_string(),
            err: err.to_string(),
        }),
        Err(_) => std::env::var(format!("{key}_FILE"))
            .map_err(|_| EnvVarError::InvalidOrMissingKey {
                key: key.to_string(),
            })?
            .parse()
            .map_err(|err: T::Err| EnvVarError::InvalidValue {
                key: format!("{key}_FILE"),
                err: err.to_string(),
            }),
    }
}

pub trait ContextExt {
    async fn permissions_in(
        &self,
        channel: &serenity::GuildChannel,
    ) -> Result<serenity::Permissions>;
}

impl ContextExt for crate::framework::Context<'_> {
    async fn permissions_in(
        &self,
        channel: &serenity::GuildChannel,
    ) -> Result<serenity::Permissions> {
        let guild = self
            .partial_guild()
            .await
            .with_context(|| "Getting partial guild")?;
        let bot_id = self.cache().current_user().id;
        let bot_member = guild.member(&self, bot_id).await?;
        let permissions = guild.user_permissions_in(channel, &bot_member);

        Ok(permissions)
    }
}

/// Provides a convenience method for creating a reply out of self.
pub trait ToReply {
    fn to_reply(self) -> poise::CreateReply;
}

impl ToReply for serenity::CreateEmbed {
    /// Creates a default reply that only contains this embed.
    fn to_reply(self) -> poise::CreateReply {
        poise::CreateReply::default().embed(self)
    }
}

pub fn parse_csv_app_ids(x: &str) -> StdResult<Vec<i32>, std::num::ParseIntError> {
    x.split(",").try_fold(Vec::new(), |mut vec, app| {
        vec.push(app.trim().parse()?);
        Ok(vec)
    })
}

pub const PARSE_APP_IDS_FAIL_MSG: &str = "Failed to parse appids. \
Please make sure its in the format `<appid1>, <appid2>, ...`. \
Ex: `1868140, 413150, 3527290`";
