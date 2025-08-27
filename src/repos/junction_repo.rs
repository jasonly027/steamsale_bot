use std::fmt::Debug;

use mongodb::{bson, options::ReadConcern};
use poise::serenity_prelude::futures::{StreamExt, TryStreamExt};

use crate::{Result, StdResult, database, models, util::ResLog};

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct AppListingAggregate {
    #[serde(flatten)]
    junction: models::Junction,
    apps: Vec<models::App>,
}

impl TryInto<models::AppListing> for AppListingAggregate {
    type Error = bson::oid::ObjectId;

    /// Fails and returns junction's _id if `self.apps` is empty.
    fn try_into(mut self) -> StdResult<models::AppListing, Self::Error> {
        if self.apps.is_empty() {
            return Err(self.junction.id);
        }
        let models::App {
            app_id, app_name, ..
        } = self.apps.swap_remove(0);

        Ok(models::AppListing {
            app_id,
            app_name,
            sale_threshold: self.junction.sale_threshold,
        })
    }
}

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

    #[must_use]
    pub async fn set_thresholds(
        &self,
        guild_id: i64,
        threshold: i32,
        app_ids: impl Into<Vec<i32>> + Debug,
    ) -> Vec<i32> {
        let app_ids = app_ids.into();

        // Get a snapshot of every junction record in the
        // specified guild and is one of the ids in app_ids
        let filter = bson::doc! {
            "server_id": guild_id,
            "app_id": { "$in": &app_ids },
        };
        let concerns = ReadConcern::snapshot();
        let Ok(mut cursor) = self.coll.find(filter).read_concern(concerns).await.terror() else {
            return app_ids;
        };

        // Update each record individually, keeping track of successfully updated
        let mut updated_apps = Vec::new();
        while let Some(junction) = cursor.next().await {
            let Ok(models::Junction { id, app_id, .. }) = junction.terror() else {
                continue;
            };

            let query = bson::doc! { "_id": id };
            let update = bson::doc! { "$set": { "sale_threshold": threshold } };
            let result = self.coll.update_one(query, update).await;

            if result.terror().is_ok() {
                updated_apps.push(app_id);
            }
        }

        let mut failed_apps = app_ids;
        failed_apps.retain(|x| !updated_apps.contains(x));

        failed_apps
    }

    pub async fn get_app_listings(&self, guild_id: i64) -> Result<Vec<models::AppListing>> {
        let pipeline = vec![
            bson::doc! { "$match": { "server_id": guild_id } },
            bson::doc! {
                "$lookup": {
                    "from": database::APPS_COLL,
                    "localField": "app_id",
                    "foreignField": "app_id",
                    "as": "apps",
                }
            },
        ];

        let stream = self
            .coll
            .aggregate(pipeline)
            .with_type::<AppListingAggregate>()
            .await?
            .into_stream();

        Ok(stream
            .filter_map(|x| async {
                match x.terror() {
                    Ok(x) => x.try_into().terror().ok(),
                    Err(_) => None,
                }
            })
            .collect()
            .await)
    }

    pub fn clear_apps(&self, guild_id: i64) -> mongodb::action::Delete<'_> {
        let query = bson::doc! { "server_id": guild_id };
        self.coll.delete_many(query)
    }

    pub fn remove_apps(&self, guild_id: i64, app_ids: &[i32]) -> mongodb::action::Delete<'_> {
        let query = bson::doc! {
            "server_id": guild_id,
            "app_id": { "$in": &app_ids },
        };
        self.coll.delete_many(query)
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::{
        database::test::TestDatabase,
        models::{App, AppListing, Discord, Junction},
        util::CollectionCollectAll,
    };

    use super::*;

    #[tokio::test]
    #[serial_test::serial(database)]
    #[rustfmt::skip]
    async fn set_thresholds_doesnt_change_unmentioned() -> Result<()> {
        let db = TestDatabase::new().await?;
        let repo = JunctionRepo::new(&db);

        const OLD_THRESHOLD: i32 = 0;
        let mut target = Junction { server_id: 0, app_id: 0, sale_threshold: Some(OLD_THRESHOLD), ..Default::default() };
        // Same server, unmentioned app_id
        let not_target1= Junction { server_id: 0, app_id: 1, sale_threshold: Some(OLD_THRESHOLD), ..Default::default() };
        // Diff server, mentioned app_id
        let not_target2= Junction { server_id: 1, app_id: 1, sale_threshold: Some(OLD_THRESHOLD), ..Default::default() };
        // Diff server, unmentioned app_id
        let not_target3= Junction { server_id: 1, app_id: 1, sale_threshold: Some(OLD_THRESHOLD), ..Default::default() };
        db.junction().insert_many([&target, &not_target1, &not_target2, &not_target3]).await?;

        const NEW_THRESHOLD: i32 = 1;
        let failed = repo.set_thresholds(target.server_id, NEW_THRESHOLD, [target.app_id]).await;

        // Update expected sale_threshold
        target.sale_threshold = Some(NEW_THRESHOLD);

        let actual = db.junction().collect().await?;
        assert!(failed.is_empty(), "{failed:?}");
        assert_eq!([target, not_target1, not_target2, not_target3], actual[..]);

        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial(database)]
    #[rustfmt::skip]
    async fn get_app_listings_joins_correctly() -> Result<()> {
        let db = TestDatabase::new().await?;
        let repo = JunctionRepo::new(&db);

        let server_id = 0;
        let server_threshold = 1;
        let junction_threshold = 2;
        let expected = AppListing {
            app_id: 1,
            app_name: "name".to_string(),
            sale_threshold: Some(junction_threshold),
        };
        db.apps().insert_one(
            App {
                app_id: expected.app_id,
                app_name: expected.app_name.clone(),
                ..Default::default()
            }
        ).await?;
        db.discord().insert_one(
            Discord {
                server_id,
                sale_threshold: server_threshold,
                ..Default::default()
            }
        ).await?;
        db.junction().insert_one(
            Junction {
                server_id,
                app_id: expected.app_id,
                sale_threshold: Some(junction_threshold),
                ..Default::default()
            }
        ).await?;

        let actual = repo.get_app_listings(server_id).await?;
        assert_eq!(1, actual.len(), "{actual:?}");
        assert_eq!(expected, actual[0]);

        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial(database)]
    #[rustfmt::skip]
    async fn clear_apps_deletes_all_apps_of_target_guild() -> Result<()> {
        let db = TestDatabase::new().await?;
        let repo = JunctionRepo::new(&db);

        let server_id = 0;
        db.junction().insert_many([
            Junction { server_id, ..Default::default() },
            Junction { server_id, ..Default::default() },
        ]).await?;

        repo.clear_apps(server_id).await?;

        let records = db.junction().collect().await?;
        assert!(records.is_empty());

        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial(database)]
    #[rustfmt::skip]
    async fn clear_apps_doesnt_delete_apps_of_other_guilds() -> Result<()> {
        let db = TestDatabase::new().await?;
        let repo = JunctionRepo::new(&db);

        let server_id = 0;
        let other = Junction { server_id: 1, ..Default::default() };
        db.junction().insert_one(&other).await?;

        repo.clear_apps(server_id).await?;

        let actual = db.junction().collect().await?;
        assert_eq!([other], actual[..]);

        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial(database)]
    #[rustfmt::skip]
    async fn remove_apps_only_deletes_targeted_apps_in_the_target_guild() -> Result<()> {
        let db = TestDatabase::new().await?;
        let repo = JunctionRepo::new(&db);

        let server_id = 0;
        let target1 = Junction { server_id, app_id: 0, ..Default::default() };
        let target2 = Junction { server_id, app_id: 1, ..Default::default() };
        let other  = Junction { server_id, app_id: 2, ..Default::default() };
        db.junction().insert_many([&target1, &target2, &other]).await?;

        repo.remove_apps(server_id, &[target1.app_id, target2.app_id]).await?;

        let actual = db.junction().collect().await?;
        assert_eq!([other], actual[..]);

        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial(database)]
    #[rustfmt::skip]
    async fn remove_apps_doesnt_delete_apps_of_other_guilds() -> Result<()> {
        let db = TestDatabase::new().await?;
        let repo = JunctionRepo::new(&db);

        let app_id = 0;
        let server_id = 0;
        let other = Junction { server_id, app_id: 1, ..Default::default() };
        db.junction().insert_one(&other).await?;

        repo.remove_apps(server_id, &[app_id]).await?;

        let actual = db.junction().collect().await?;
        assert_eq!([other], actual[..]);

        Ok(())
    }
}
