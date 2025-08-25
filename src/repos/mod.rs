use crate::database;

mod junction_repo;

#[derive(Debug, Clone)]
pub struct Repo {
    db: std::sync::Arc<database::Database>,
    pub junction: junction_repo::JunctionRepo,
}

impl Repo {
    pub fn new(db: std::sync::Arc<database::Database>) -> Self {
        let junction = junction_repo::JunctionRepo::new(&db);

        Self { db, junction }
    }

    pub async fn start_session(&self) -> mongodb::error::Result<mongodb::ClientSession> {
        self.db.start_session().await
    }
}
