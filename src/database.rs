use mongodb::{
    bson::doc,
    options::{ClientOptions, ServerApi, ServerApiVersion},
};

use crate::Error;

const DATABASE_NAME: &str = "RUST_DEV";

pub struct Database {
    client: mongodb::Client,
}

impl Database {
    pub async fn new(uri: &str) -> Result<Self, Error> {
        let mut options = ClientOptions::parse(uri).await?;
        options.server_api = Some(ServerApi::builder().version(ServerApiVersion::V1).build());
        let client = mongodb::Client::with_options(options)?;
        client
            .database(DATABASE_NAME)
            .run_command(doc! {"ping": 1})
            .await?;

        Ok(Self { client })
    }

    fn db(&self) -> mongodb::Database {
        self.client.database(DATABASE_NAME)
    }
}
