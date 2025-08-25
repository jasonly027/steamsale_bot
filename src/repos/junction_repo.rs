use mongodb::bson;

use crate::{Result, database, models, util::ResLog};

#[derive(Debug, Clone)]
pub struct JunctionRepo {
    coll: mongodb::Collection<models::Junction>,
}

impl JunctionRepo {
    pub fn new(db: &database::Database) -> Self {
        Self {
            coll: db.junction(),
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
        .error()?;

        Ok(())
    }
}
