//! This module provides a repository for the apps collection.

use futures::TryStreamExt;
use mongodb::bson;

use crate::{database, models};

#[derive(Debug, Clone)]
pub struct AppsRepo {
    coll: mongodb::Collection<models::App>,
}

impl AppsRepo {
    pub fn new(db: &database::Database) -> Self {
        Self { coll: db.apps() }
    }

    /// Inserts the app if it's not present in the collection. Otherwise,
    /// replaces it in the collection (as a way to update the app name).
    pub fn upsert_app(&self, app: &models::App) -> mongodb::action::ReplaceOne<'_> {
        let query = bson::doc! { "app_id": app.app_id };
        self.coll.replace_one(query, app).upsert(true)
    }

    pub async fn remove_orphans(&self) -> mongodb::error::Result<()> {
        let pipeline = [
            bson::doc! {
                "$lookup": {
                    "from": database::JUNCTION_COLL,
                    "localField": "app_id",
                    "foreignField": "app_id",
                    "as": "trackers",
                }
            },
            bson::doc! { "$match": { "trackers": { "$size": 0 } } },
            bson::doc! { "$project": { "_id": true } },
        ];

        let ids = self
            .coll
            .aggregate(pipeline)
            .await?
            .try_filter_map(|x| async move { Ok(x.get_object_id("_id").ok()) })
            .try_collect::<Vec<bson::oid::ObjectId>>()
            .await?;

        let query = bson::doc! { "_id": { "$in": ids } };
        self.coll.delete_many(query).await?;

        Ok(())
    }

    pub async fn get_app_ids(&self) -> mongodb::error::Result<Vec<i32>> {
        self.coll
            .find(bson::doc! {})
            .await?
            .map_ok(|app| app.app_id)
            .try_collect()
            .await
    }
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::{
        Result,
        database::{CollectionCollectAll, TestDatabase},
        models::{App, Junction},
        repos::apps_repo::AppsRepo,
    };

    #[tokio::test]
    #[serial_test::serial(database)]
    async fn upsert_app_inserts_app_into_collection() -> Result<()> {
        let db = TestDatabase::new().await?;
        let repo = AppsRepo::new(&db);

        let expected = App::default();
        repo.upsert_app(&expected).await?;

        let actual = db.apps().collect().await?;
        assert_eq!([expected], actual[..]);

        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial(database)]
    async fn upsert_app_updates_previously_inserted_app() -> Result<()> {
        let db = TestDatabase::new().await?;
        let repo = AppsRepo::new(&db);

        let mut expected = App {
            app_name: "name".to_string(),
            ..Default::default()
        };
        repo.upsert_app(&expected).await?;

        expected.app_name = "changed".to_string();
        repo.upsert_app(&expected).await?;

        let actual = db.apps().collect().await?;
        assert_eq!([expected], actual[..]);

        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial(database)]
    #[rustfmt::skip]
    async fn remove_orphans_only_deletes_orphans() -> Result<()> {
        let db = TestDatabase::new().await?;
        let repo = AppsRepo::new(&db);

        let orphan = App { app_id: 0, ..Default::default() };
        let other = App { app_id: 1, ..Default::default() };
        db.apps().insert_many([&orphan, &other]).await?;

        let tracker_of_other = Junction { app_id: other.app_id, ..Default::default() };
        db.junction().insert_one(&tracker_of_other).await?;

        repo.remove_orphans().await?;

        let actual = db.apps().collect().await?;
        assert_eq!([other], actual[..]);

        Ok(())
    }

    #[tokio::test]
    #[serial_test::serial(database)]
    #[rustfmt::skip]
    async fn get_app_ids_collects_all_app_ids() -> Result<()> {
        let db = TestDatabase::new().await?;
        let repo = AppsRepo::new(&db);

        let app_id = 0;
        db.apps().insert_one(App { app_id, ..Default::default() }).await?;

        let actual = repo.get_app_ids().await?;
        assert_eq!([app_id], actual[..]);

        Ok(())
    }
}
