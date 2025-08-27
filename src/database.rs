use mongodb::{
    bson,
    options::{ClientOptions, ServerApi, ServerApiVersion},
};
use tracing::info;

use crate::{Result, models};

pub const APPS_COLL: &str = "apps";
pub const DISCORD_COLL: &str = "discord";
pub const JUNCTION_COLL: &str = "junction";

#[derive(Clone)]
pub struct Database {
    /// Name of the MongoDB database.
    name: String,
    /// Handle to the MongoDB client.
    client: mongodb::Client,
}

impl Database {
    /// Connects to a MongoDB database named `name`.
    pub async fn new(uri: &str, name: impl Into<String>) -> Result<Self> {
        let mut options = {
            #[cfg(windows)]
            {
                // https://github.com/mongodb/mongo-rust-driver/blob/2cf619c4f3484d1d7c42c3407bf9794c9e33d7ac/README.md#windows-dns-note
                use mongodb::options::ResolverConfig;
                ClientOptions::parse(uri)
                    .resolver_config(ResolverConfig::cloudflare())
                    .await?
            }
            #[cfg(not(windows))]
            {
                ClientOptions::parse(uri).await?
            }
        };
        options.server_api = Some(ServerApi::builder().version(ServerApiVersion::V1).build());
        let client = mongodb::Client::with_options(options)?;

        let name = name.into();
        let db = client.database(&name);
        info!("Pinging database...");
        db.run_command(bson::doc! {"ping": 1}).await?;
        info!("Pong received from database");

        Ok(Self { name, client })
    }

    pub async fn start_session(&self) -> mongodb::error::Result<mongodb::ClientSession> {
        self.client.start_session().await
    }

    pub fn discord(&self) -> mongodb::Collection<models::Discord> {
        self.db().collection(DISCORD_COLL)
    }

    pub fn junction(&self) -> mongodb::Collection<models::Junction> {
        self.db().collection(JUNCTION_COLL)
    }

    pub fn apps(&self) -> mongodb::Collection<models::App> {
        self.db().collection(APPS_COLL)
    }

    fn db(&self) -> mongodb::Database {
        self.client.database(&self.name)
    }
}

#[cfg(test)]
pub mod test {
    use std::ops::Deref;

    use super::*;
    use crate::{
        Result,
        util::{self, ResLog},
    };

    pub struct TestDatabase(Database);

    impl TestDatabase {
        /// Connects to the test database and automatically calls [`Self::drop_collections`].
        pub async fn new() -> Result<Self> {
            dotenvy::dotenv().twarn()?;
            let uri: String = util::env_var("MONGODB_URI")?;
            let name: String = util::env_var("MONGODB_TESTDBNAME")?;
            let inner = Database::new(&uri, name).await?;

            let db = TestDatabase(inner);
            db.drop_collections().await?;

            Ok(db)
        }

        /// Drops all collections from the test database.
        pub async fn drop_collections(&self) -> Result<()> {
            for name in self.db().list_collection_names().await? {
                self.db().collection::<bson::Document>(&name).drop().await?;
            }
            Ok(())
        }
    }

    impl Deref for TestDatabase {
        type Target = Database;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }
}
