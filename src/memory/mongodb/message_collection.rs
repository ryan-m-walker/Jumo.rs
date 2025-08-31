use crate::types::message::Message;

use futures::StreamExt;
use mongodb::{Database, bson::doc};

const COLLECTION_NAME: &str = "messages";

pub struct MessageCollection {
    collection: mongodb::Collection<Message>,
}

impl MessageCollection {
    pub fn new(db: &Database) -> Self {
        Self {
            collection: db.collection(COLLECTION_NAME),
        }
    }

    pub async fn insert_one(&self, message: &Message) -> Result<(), anyhow::Error> {
        self.collection.insert_one(message).await?;
        Ok(())
    }

    pub async fn get_recent_messages(&self) -> Result<Vec<Message>, anyhow::Error> {
        let mut cursor = self
            .collection
            .find(doc! {})
            .sort(doc! { "created_at": -1 })
            .limit(20)
            .await?;

        let mut messages = Vec::new();

        while let Some(message) = cursor.next().await {
            let message = message?;
            messages.push(message);
        }

        Ok(messages)
    }
}
