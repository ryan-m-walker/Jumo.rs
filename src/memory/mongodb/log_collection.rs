use futures::StreamExt;
use mongodb::{Database, bson::doc};

use crate::types::logs::Log;

const COLLECTION_NAME: &str = "logs";

pub struct LogCollection {
    collection: mongodb::Collection<Log>,
}

impl LogCollection {
    pub fn new(db: &Database) -> Self {
        Self {
            collection: db.collection(COLLECTION_NAME),
        }
    }

    pub async fn insert_one(&self, message: &Log) -> Result<(), anyhow::Error> {
        self.collection.insert_one(message).await?;
        Ok(())
    }

    pub async fn get_recent_logs(&self) -> Result<Vec<Log>, anyhow::Error> {
        let mut cursor = self
            .collection
            .find(doc! {})
            .sort(doc! { "created_at": -1 })
            .limit(20)
            .await?;

        let mut logs = Vec::new();

        while let Some(log) = cursor.next().await {
            let log = log?;
            logs.push(log);
        }

        Ok(logs)
    }
}
