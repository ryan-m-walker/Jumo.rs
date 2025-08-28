use std::env;

use qdrant_client::{
    Payload, Qdrant,
    qdrant::{
        CreateCollectionBuilder, Distance, PointStruct, ScalarQuantizationBuilder,
        UpsertPointsBuilder, VectorParamsBuilder,
    },
};
use tokio::sync::mpsc;

use crate::{
    database::models::message::{ContentBlock, Message},
    events::AppEvent,
    services::openai::{EMBEDDINGS_DIMENSIONS, create_embedding},
};

const QDRANT_COLLECTION_NAME: &str = "jumo_messages";

pub struct QdrantService {
    client: Option<Qdrant>,
    event_sender: mpsc::Sender<AppEvent>,
}

impl QdrantService {
    pub fn new(event_sender: mpsc::Sender<AppEvent>) -> Self {
        Self {
            client: None,
            event_sender,
        }
    }

    pub async fn init(&mut self) -> Result<(), anyhow::Error> {
        let qdrant_url = env::var("QDRANT_URL").unwrap_or("http://localhost:6334".to_string());
        let client = Qdrant::from_url(&qdrant_url).build()?;

        let create_collection_request = CreateCollectionBuilder::new(QDRANT_COLLECTION_NAME)
            .vectors_config(VectorParamsBuilder::new(
                EMBEDDINGS_DIMENSIONS as u64,
                Distance::Cosine,
            ))
            .quantization_config(ScalarQuantizationBuilder::default());

        let collections_result = client.list_collections().await?;
        let collection_exists = collections_result
            .collections
            .iter()
            .any(|c| c.name == QDRANT_COLLECTION_NAME);

        if !collection_exists {
            client.create_collection(create_collection_request).await?;
        }

        self.client = Some(client);

        Ok(())
    }

    pub fn insert_message(&self, message: &Message) -> Result<(), anyhow::Error> {
        let message = message.clone();

        tokio::spawn(async move {
            let qdrant_url = env::var("QDRANT_URL").unwrap_or("http://localhost:6334".to_string());

            for content in &message.content {
                if let ContentBlock::Text { text } = content {
                    let client = match Qdrant::from_url(&qdrant_url).build() {
                        Ok(client) => client,
                        Err(err) => {
                            // TODO: log error
                            return;
                        }
                    };

                    let embedding = match create_embedding(text).await {
                        Ok(embedding) => embedding,
                        Err(err) => {
                            // TODO: log error
                            return;
                        }
                    };

                    let payload = serde_json::json!({
                        "message_id": message.id,
                        "role": message.role,
                        "created_at": message.created_at,
                        "text": text,
                    });

                    let payload: Payload = match payload.try_into() {
                        Ok(payload) => payload,
                        Err(err) => {
                            // TODO: log error
                            return;
                        }
                    };

                    let points = vec![PointStruct::new(message.id.clone(), embedding, payload)];

                    let res = client
                        .upsert_points(UpsertPointsBuilder::new(QDRANT_COLLECTION_NAME, points))
                        .await;

                    if let Err(err) = res {
                        // let _ = self.event_sender.send(AppEvent::Log(format!(
                        //     "Failed to insert message into Qdrant: {err}".to_string()
                        // ));
                    }
                }
            }
        });

        Ok(())
    }
}
