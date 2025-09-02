use anyhow::Context;
use poise::serenity_prelude as serenity;
use tracing::info;

use crate::{Result,repos, util::PoiseData};

pub struct RemovedFromGuild;

#[serenity::async_trait]
impl serenity::EventHandler for RemovedFromGuild {
    /// Removes guild from database.
    async fn guild_delete(
        &self,
        ctx: serenity::Context,
        incomplete: serenity::UnavailableGuild,
        _full: Option<serenity::Guild>,
    ) {
        if incomplete.unavailable {
            return;
        }

        let guild_id: i64 = incomplete.id.into();
        info!("Bot was removed from {guild_id}. Removing its records...");
        let data = ctx.poise_data_unwrap().await;
        remove_guild_records(&data.repo, guild_id).await.ok();
    }
}

#[tracing::instrument(level = "error", err, skip(repo))]
async fn remove_guild_records(repo: &repos::Repo, guild_id: i64) -> Result<()> {
    let mut session = repo
        .start_session()
        .await
        .with_context(|| "Creating session")?;
    session
        .start_transaction()
        .await
        .with_context(|| "Starting transaction")?;
    repo.discord
        .remove_guild(guild_id)
        .session(&mut session)
        .await
        .with_context(|| "Removing guild from discord collection")?;
    repo.junction
        .clear_junctions(guild_id)
        .session(&mut session)
        .await
        .with_context(|| "Clearing junctions")?;
    session
        .commit_transaction()
        .await
        .with_context(|| "Committing transaction")?;

    Ok(())
}
