use std::fmt::Debug;

use mongodb::{bson, options::ReadConcern};
use poise::serenity_prelude::futures::{StreamExt, TryStreamExt};

use crate::{Result, StdResult, database, models, util::ResLog};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
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
    #[tracing::instrument]
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

    #[tracing::instrument]
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
}
