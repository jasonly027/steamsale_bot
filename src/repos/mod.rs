use std::sync::Arc;

use crate::database;

mod discord_repo;
mod junction_repo;

#[derive(Clone)]
pub struct Repo {
    db: Arc<database::Database>,
    pub discord: discord_repo::DiscordRepo,
    pub junction: junction_repo::JunctionRepo,
}

impl Repo {
    pub fn new(db: Arc<database::Database>) -> Self {
        let discord = discord_repo::DiscordRepo::new(&db);
        let junction = junction_repo::JunctionRepo::new(&db);

        Self {
            db,
            discord,
            junction,
        }
    }

    pub async fn start_session(&self) -> mongodb::error::Result<mongodb::ClientSession> {
        self.db.start_session().await
    }
}
