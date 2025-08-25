use mongodb::{
    bson,
    options::{ClientOptions, ServerApi, ServerApiVersion},
};
use tracing::info;

use crate::{Result, models};

const DATABASE_NAME: &str = "RUST_DEV";
const APPS_COLL: &str = "apps";
const DISCORD_COLL: &str = "discord";
const JUNCTION_COLL: &str = "junction";

#[derive(Debug, Clone)]
pub struct Database {
    client: mongodb::Client,
}

impl Database {
    pub async fn new(uri: &str) -> Result<Self> {
        let mut options = ClientOptions::parse(uri).await?;
        options.server_api = Some(ServerApi::builder().version(ServerApiVersion::V1).build());
        let client = mongodb::Client::with_options(options)?;

        let db = client.database(DATABASE_NAME);
        info!("Pinging database...");
        db.run_command(bson::doc! {"ping": 1}).await?;
        info!("Pong received from database");

        Ok(Self { client })
    }

    fn db(&self) -> mongodb::Database {
        self.client.database(DATABASE_NAME)
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
}
