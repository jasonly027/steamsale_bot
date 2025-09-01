//! This module provides a repository for the apps collection.
//!
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
}

#[cfg(test)]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::{
        Result,
        database::{CollectionCollectAll, TestDatabase},
        models::App,
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
}
