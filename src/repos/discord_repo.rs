use std::fmt::Debug;

use mongodb::{
    bson::{self, doc},
    options::ReadConcern,
};
use poise::serenity_prelude::futures::StreamExt;

use crate::{Result, database, models, util::ResLog};

#[derive(Debug, Clone)]
pub struct DiscordRepo {
    coll: mongodb::Collection<models::Discord>,
}

impl DiscordRepo {
    pub fn new(db: &database::Database) -> Self {
        Self {
            coll: db.discord(),
        }
    }

    #[tracing::instrument]
    pub async fn update_channel_id(
        &self,
        guild_id: i64,
        channel_id: i64,
        session: Option<&mut mongodb::ClientSession>,
    ) -> Result<()> {
        let query = bson::doc! {"server_id": guild_id};
        let update = bson::doc! {
            "$set": bson::doc! {
                "channel_id": channel_id
            }
        };

        let op = self.coll.update_one(query, update);
        match session {
            Some(s) => op.session(s).await,
            None => op.await,
        }
        .terror()?;

        Ok(())
    }

    #[must_use]
    #[tracing::instrument]
    pub async fn set_thresholds(
        &self,
        threshold: i32,
        guild_id: i64,
        app_ids: impl Into<Vec<i32>> + Debug,
    ) -> Vec<i32> {
        let filter = bson::doc! {"server_id": guild_id};

        let Ok(mut cursor) = self
            .coll
            .find(filter)
            .read_concern(ReadConcern::snapshot())
            .await
            .terror()
        else {
            return app_ids.into();
        };

        // let mut failed = Vec::new();
        // while let Some(junc) = cursor.next().await {
        //     let Ok(models::Junction { id, app_id, .. }) = junc.terror() else {
        //         continue;
        //     };

        //     let query = bson::doc! {"_id": id};
        //     let update = bson::doc! {
        //         "$set": bson::doc! {
        //             "sale_threshold": threshold,
        //         }
        //     };
        //     if self.coll.update_one(query, update).await.is_err() {
        //         failed.push(value);
        //     }
        // }

        todo!()
    }
}
