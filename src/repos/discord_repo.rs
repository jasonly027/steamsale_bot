use mongodb::bson;

use crate::{database, models};

#[derive(Clone)]
pub struct DiscordRepo {
    coll: mongodb::Collection<models::Discord>,
}

impl DiscordRepo {
    pub fn new(db: &database::Database) -> Self {
        Self { coll: db.discord() }
    }

    #[tracing::instrument(skip(self))]
    pub fn set_channel_id(&self, guild_id: i64, channel_id: i64) -> mongodb::action::Update<'_> {
        let query = bson::doc! { "server_id": guild_id };
        let update = bson::doc! { "$set": { "channel_id": channel_id } };

        self.coll.update_one(query, update)
    }

    #[tracing::instrument(skip(self))]
    pub fn set_threshold(&self, guild_id: i64, threshold: i32) -> mongodb::action::Update<'_> {
        let query = bson::doc! { "server_id": guild_id };
        let update = bson::doc! { "$set": { "sale_threshold": threshold } };

        self.coll.update_one(query, update)
    }

    #[tracing::instrument(skip(self))]
    pub fn find_one_by_guild_id(
        &self,
        guild_id: i64,
    ) -> mongodb::action::FindOne<'_, models::Discord> {
        let filter = bson::doc! { "server_id": guild_id };
        self.coll.find_one(filter)
    }
}

#[cfg(test)]
// #[serial_test::serial] must be defined fn-level UNDER #[rstest] or
// strange things happen with futures. See Also: rstest Issue #302
mod tests {
    use pretty_assertions::assert_eq;

    use super::*;
    use crate::{
        Result, database::test::TestDatabase, models::Discord, util::CollectionCollectAll,
    };

    #[tokio::test]
    #[serial_test::serial(database)]
    #[rustfmt::skip]
    async fn set_channel_id_only_updates_specified_guild() -> Result<()> {
        let db = TestDatabase::new().await?;
        let repo = DiscordRepo::new(&db);
        
        const OLD_CHANNEL_ID: i64 = 0;
        let target = Discord { server_id: 0, channel_id: OLD_CHANNEL_ID, ..Default::default() };
        let dummy = Discord { server_id: 1, channel_id: OLD_CHANNEL_ID, ..Default::default() };
        let mut expected = [target, dummy];
        db.discord().insert_many(&expected).await?;

        const NEW_CHANNEL_ID: i64 = 1;
        repo.set_channel_id(expected[0].server_id, NEW_CHANNEL_ID).await?;

        // Update target's expected channel_id
        expected[0].channel_id = NEW_CHANNEL_ID;

        let actual = db.discord().collect().await?;
        assert_eq!(expected[..], actual[..]);

        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial(database)]
    #[rustfmt::skip]
    async fn set_threshold_only_updates_specified_guild() -> Result<()> {
        let db = TestDatabase::new().await?;
        let repo = DiscordRepo::new(&db);

        const OLD_THRESHOLD: i32 = 0;
        let target = Discord { server_id: 0, sale_threshold: OLD_THRESHOLD, ..Default::default() };
        let dummy = Discord { server_id: 1, sale_threshold: OLD_THRESHOLD, ..Default::default() };
        let mut expected = [target, dummy];
        db.discord().insert_many(&expected).await?;

        const NEW_THRESHOLD: i32 = 1;
        repo.set_threshold(expected[0].server_id, NEW_THRESHOLD).await?;

        // Update target's expected sale_threshold
        expected[0].sale_threshold = NEW_THRESHOLD;

        let actual = db.discord().collect().await?;
        assert_eq!(expected[..], actual[..]);

        Ok(())
    }
}
