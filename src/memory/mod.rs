use tokio::sync::mpsc;

use crate::{
    events::AppEvent, memory::mongodb::MongodbMemory, services::qdrant::QdrantService,
    types::message::Message,
};

pub mod mongodb;

pub struct MemoryManager {
    qdrant: QdrantService,
    pub mongodb: MongodbMemory,
}

impl MemoryManager {
    pub async fn new(_event_sender: mpsc::Sender<AppEvent>) -> Result<Self, anyhow::Error> {
        Ok(Self {
            qdrant: QdrantService::new(),
            mongodb: MongodbMemory::new().await?,
        })
    }

    pub async fn process_exchange(&self, messages: &[Message]) -> Result<(), anyhow::Error> {
        for message in messages {
            self.mongodb.messages.insert_one(message).await?;
            self.qdrant.insert_message(message).await?;
        }

        Ok(())
    }

    pub async fn gather_memory(&self) -> Result<Vec<Message>, anyhow::Error> {
        let recent_messages = self.mongodb.messages.get_recent_messages().await?;
        Ok(recent_messages)
    }
}
