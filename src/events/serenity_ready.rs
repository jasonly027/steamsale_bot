use std::sync::Arc;

use poise::serenity_prelude as serenity;
use tracing::info;

use crate::{
    Result, database,
    framework::{self, Data},
    repos, steam,
    util::{self, PoiseData},
};

pub struct SerenityReady;

#[serenity::async_trait]
impl serenity::EventHandler for SerenityReady {
    /// Initialize framework data and start check apps loop.
    async fn ready(&self, ctx: serenity::Context, _ready: serenity::Ready) {
        init_data(&ctx).await;
        init_check_apps(ctx.poise_data_unwrap().await);
    }
}

async fn init_data(ctx: &serenity::Context) {
    // We place poise data in the serenity store so it's still accessible
    // in places where we only have the serenity ctx and not poise ctx.
    // For example, serenity::EventHandler's.

    info!("Writing poise data to serenity store");
    let mut store = ctx.data.write().await;
    let data = Arc::new(create_data(ctx.http.clone()).await.unwrap());
    store.insert::<framework::Data>(data);
}

async fn create_data(http: Arc<serenity::Http>) -> Result<Data> {
    let repo = {
        let uri: String = util::env_var("MONGODB_URI")?;
        let name: String = util::env_var("MONGODB_DBNAME")?;
        let db = database::Database::new(&uri, name).await?;

        repos::Repo::new(Arc::new(db))
    };

    let steam = {
        let store = "https://store.steampowered.com";
        let community = "https://steamcommunity.com";

        steam::Client::new(store, community)
    };

    Ok(Data { http, repo, steam })
}

fn init_check_apps(ctx: Arc<framework::Data>) {
    tokio::spawn(check_apps(ctx));
}

async fn check_apps(ctx: Arc<framework::Data>) {}
