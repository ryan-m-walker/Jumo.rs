use mongodb::Database;

use crate::{
    config::CONFIG,
    memory::mongodb::{log_collection::LogCollection, message_collection::MessageCollection},
};

pub mod log_collection;
pub mod message_collection;

pub struct MongodbMemory {
    db: Database,
    pub messages: MessageCollection,
    pub logs: LogCollection,
}

impl MongodbMemory {
    pub async fn new() -> Result<Self, anyhow::Error> {
        let connection = mongodb::Client::with_uri_str(&CONFIG.mongodb_url).await?;
        let db = connection.database(&CONFIG.mongodb_database);

        let messages = MessageCollection::new(&db);
        let logs = LogCollection::new(&db);

        Ok(Self { db, messages, logs })
    }
}
