use mongodb::{
    bson,
    options::{ClientOptions, ServerApi, ServerApiVersion},
};
use tracing::info;

use crate::{Result, models};

pub const APPS_COLL: &str = "apps";
pub const DISCORD_COLL: &str = "discord";
pub const JUNCTION_COLL: &str = "junction";

#[derive(Debug, Clone)]
pub struct Database {
    /// Name of the MongoDB database.
    name: String,
    /// Handle to the MongoDB client.
    client: mongodb::Client,
}

impl Database {
    /// Connects to a MongoDB database named `name`.
    pub async fn new(uri: &str, name: impl Into<String>) -> Result<Self> {
        let name = name.into();
        let mut options = ClientOptions::parse(uri).await?;
        options.server_api = Some(ServerApi::builder().version(ServerApiVersion::V1).build());
        let client = mongodb::Client::with_options(options)?;

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

    fn db(&self) -> mongodb::Database {
        self.client.database(&self.name)
    }
}
