//! This module provides a repository for the discord collection.

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

    pub fn set_channel_id(&self, guild_id: i64, channel_id: i64) -> mongodb::action::Update<'_> {
        let query = bson::doc! { "server_id": guild_id };
        let update = bson::doc! { "$set": { "channel_id": channel_id } };

        self.coll.update_one(query, update)
    }

    pub fn set_threshold(&self, guild_id: i64, threshold: i32) -> mongodb::action::Update<'_> {
        let query = bson::doc! { "server_id": guild_id };
        let update = bson::doc! { "$set": { "sale_threshold": threshold } };

        self.coll.update_one(query, update)
    }

    pub fn get_guild(
        &self,
        guild_id: i64,
    ) -> mongodb::action::FindOne<'_, models::Discord> {
        let filter = bson::doc! { "server_id": guild_id };
        self.coll.find_one(filter)
    }

    pub fn remove_guild(&self, guild_id: i64) -> mongodb::action::Delete<'_> {
        let query = bson::doc! { "server_id": guild_id };
        self.coll.delete_one(query)
    }

    pub fn add_guild(&self, guild_id: i64, channel_id: i64) -> mongodb::action::Update<'_> {
        const DEFAULT_SALE_THRESHOLD: i32 = 1;

        let query = bson::doc! { "server_id": guild_id };
        let discord = models::Discord {
            id: Default::default(),
            server_id: guild_id,
            channel_id,
            sale_threshold: DEFAULT_SALE_THRESHOLD,
        };
        let ddoc = bson::to_document(&discord).expect("discord should be serializable");
        let update = bson::doc! { "$setOnInsert" : ddoc };

        self.coll.update_one(query, update).upsert(true)
    }
}

#[cfg(test)]
// #[serial_test::serial] must be defined fn-level UNDER #[rstest] or
// strange things happen with futures. See Also: rstest Issue #302
mod tests {
    use pretty_assertions::assert_eq;

    use crate::{
        Result,
        database::{CollectionCollectAll, TestDatabase},
        models::Discord,
        repos::discord_repo::DiscordRepo,
    };

    #[tokio::test]
    #[serial_test::serial(database)]
    #[rustfmt::skip]
    async fn set_channel_id_only_updates_channel_id_of_target_guild() -> Result<()> {
        let db = TestDatabase::new().await?;
        let repo = DiscordRepo::new(&db);
        
        const OLD_CHANNEL_ID: i64 = 0;
        let mut target = Discord { server_id: 0, channel_id: OLD_CHANNEL_ID, ..Default::default() };
        let other =      Discord { server_id: 1, channel_id: OLD_CHANNEL_ID, ..Default::default() };
        db.discord().insert_many([&target, &other]).await?;

        const NEW_CHANNEL_ID: i64 = 1;
        repo.set_channel_id(target.server_id, NEW_CHANNEL_ID).await?;

        // Update target's expected channel_id
        target.channel_id = NEW_CHANNEL_ID;

        let actual = db.discord().collect().await?;
        assert_eq!([target, other], actual[..]);

        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial(database)]
    #[rustfmt::skip]
    async fn set_threshold_only_updates_threshold_of_target_guild() -> Result<()> {
        let db = TestDatabase::new().await?;
        let repo = DiscordRepo::new(&db);

        const OLD_THRESHOLD: i32 = 0;
        let mut target = Discord { server_id: 0, sale_threshold: OLD_THRESHOLD, ..Default::default() };
        let other      = Discord { server_id: 1, sale_threshold: OLD_THRESHOLD, ..Default::default() };
        db.discord().insert_many([&target, &other]).await?;

        const NEW_THRESHOLD: i32 = 1;
        repo.set_threshold(target.server_id, NEW_THRESHOLD).await?;

        // Update target's expected sale_threshold
        target.sale_threshold = NEW_THRESHOLD;

        let actual = db.discord().collect().await?;
        assert_eq!([target, other], actual[..]);

        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial(database)]
    #[rustfmt::skip]
    async fn get_guild_gets_correct_guild() -> Result<()> {
        let db = TestDatabase::new().await?;
        let repo = DiscordRepo::new(&db);

        let expected = Discord { server_id: 0, ..Default::default() };
        db.discord().insert_one(&expected).await?;

        let actual = repo.get_guild(expected.server_id).await?;
        assert_eq!(Some(expected), actual);

        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial(database)]
    #[rustfmt::skip]
    async fn remove_guild_only_deletes_target_guild() -> Result<()> {
        let db = TestDatabase::new().await?;
        let repo = DiscordRepo::new(&db);

        let target = Discord { server_id: 0, ..Default::default() };
        let other  = Discord { server_id: 1, ..Default::default() };
        db.discord().insert_many([&target, &other]).await?;

        repo.remove_guild(target.server_id).await?;

        let actual = db.discord().collect().await?;
        assert_eq!([other], actual[..]);

        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial(database)]
    #[rustfmt::skip]
    async fn add_guild_inserts_guild() -> Result<()> {
        let db = TestDatabase::new().await?;
        let repo = DiscordRepo::new(&db);

        let expected = Discord {
            server_id: 0,
            channel_id: 0,
            ..Default::default()
        };

        repo.add_guild(expected.server_id, expected.channel_id).await?;

        let actual = db.discord().collect().await?;
        assert_eq!([expected], actual[..]);

        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial(database)]
    async fn add_guild_does_nothing_if_inserting_duplicate_variants() -> Result<()> {
        let db = TestDatabase::new().await?;
        let repo = DiscordRepo::new(&db);

        let expected = Discord {
            server_id: 0,
            channel_id: 0,
            ..Default::default()
        };
        repo.add_guild(expected.server_id, expected.channel_id)
            .await?;

        // Duplicate variant
        let duplicate = expected.clone();
        repo.add_guild(duplicate.server_id, duplicate.channel_id)
            .await?;

        // Another duplicate variant
        let mut modified = expected.clone();
        modified.channel_id = 2;
        repo.add_guild(modified.server_id, modified.channel_id)
            .await?;

        let actual = db.discord().collect().await?;
        assert_eq!([expected], actual[..]);

        Ok(())
    }
}
