use std::fmt::Debug;

use mongodb::bson;

use crate::{Result, database, models, util::ResLog};

#[derive(Debug, Clone)]
pub struct DiscordRepo {
    coll: mongodb::Collection<models::Discord>,
}

impl DiscordRepo {
    pub fn new(db: &database::Database) -> Self {
        Self { coll: db.discord() }
    }

    #[tracing::instrument]
    pub async fn update_channel_id(
        &self,
        guild_id: i64,
        channel_id: i64,
        session: Option<&mut mongodb::ClientSession>,
    ) -> Result<()> {
        let query = bson::doc! { "server_id": guild_id };
        let update = bson::doc! { "$set": { "channel_id": channel_id } };

        let op = self.coll.update_one(query, update);
        match session {
            Some(s) => op.session(s),
            None => op,
        }
        .await
        .terror()?;

        Ok(())
    }

    #[tracing::instrument]
    pub async fn set_threshold(
        &self,
        guild_id: i64,
        threshold: i32,
        session: Option<&mut mongodb::ClientSession>,
    ) -> Result<()> {
        let query = bson::doc! { "server_id": guild_id };
        let update = bson::doc! { "$set": { "sale_threshold": threshold } };

        let op = self.coll.update_one(query, update);
        match session {
            Some(s) => op.session(s),
            None => op,
        }
        .await
        .terror()?;

        Ok(())
    }
}
