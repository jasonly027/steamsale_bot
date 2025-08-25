use std::sync::Arc;

use crate::database;

mod discord_repo;

#[derive(Debug, Clone)]
pub struct Repo {
    db: Arc<database::Database>,
    pub discord: discord_repo::DiscordRepo,
}

impl Repo {
    pub fn new(db: Arc<database::Database>) -> Self {
        let discord = discord_repo::DiscordRepo::new(&db);

        Self { db, discord }
    }

    pub async fn start_session(&self) -> mongodb::error::Result<mongodb::ClientSession> {
        self.db.start_session().await
    }
}
