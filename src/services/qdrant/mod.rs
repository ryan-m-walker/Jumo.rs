use std::env;

use qdrant_client::{
    Payload, Qdrant,
    qdrant::{
        CreateCollectionBuilder, Distance, PointStruct, ScalarQuantizationBuilder,
        UpsertPointsBuilder, VectorParamsBuilder,
    },
};

use crate::services::openai::{EMBEDDINGS_DIMENSIONS, create_embedding};

const QDRANT_COLLECTION_NAME: &str = "jumo_messages";

pub struct QdrantService {
    client: Option<Qdrant>,
}

impl QdrantService {
    pub fn new() -> Self {
        Self { client: None }
    }

    pub async fn connect(&mut self) -> Result<(), anyhow::Error> {
        let qdrant_url = env::var("QDRANT_URL").unwrap_or("http://localhost:6333".to_string());
        let client = Qdrant::from_url(&qdrant_url).build()?;

        let create_collection_request = CreateCollectionBuilder::new(QDRANT_COLLECTION_NAME)
            .vectors_config(VectorParamsBuilder::new(
                EMBEDDINGS_DIMENSIONS as u64,
                Distance::Cosine,
            ))
            .quantization_config(ScalarQuantizationBuilder::default());

        client.create_collection(create_collection_request).await?;

        self.client = Some(client);

        Ok(())
    }

    pub async fn insert_message(&self, text: &str) -> Result<(), anyhow::Error> {
        let Some(client) = &self.client else {
            return Err(anyhow::anyhow!("Qdrant client is not initialized"));
        };

        let embedding = create_embedding(text).await?;

        let payload: Payload = serde_json::json!({}).try_into()?;

        let points = vec![PointStruct::new(0, embedding, payload)];

        client
            .upsert_points(UpsertPointsBuilder::new(QDRANT_COLLECTION_NAME, points))
            .await?;

        Ok(())
    }
}
