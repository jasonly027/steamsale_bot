use std::sync::Arc;

use crate::database;

mod apps_repo;
mod discord_repo;
mod junction_repo;

#[derive(Clone)]
pub struct Repo {
    db: Arc<database::Database>,
    pub apps: apps_repo::AppsRepo,
    pub discord: discord_repo::DiscordRepo,
    pub junction: junction_repo::JunctionRepo,
}

impl Repo {
    pub fn new(db: Arc<database::Database>) -> Self {
        let apps = apps_repo::AppsRepo::new(&db);
        let discord = discord_repo::DiscordRepo::new(&db);
        let junction = junction_repo::JunctionRepo::new(&db);

        Self {
            db,
            apps,
            discord,
            junction,
        }
    }

    pub async fn start_session(&self) -> mongodb::error::Result<mongodb::ClientSession> {
        self.db.start_session().await
    }
}
