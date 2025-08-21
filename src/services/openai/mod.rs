use serde::{Deserialize, Serialize};

pub const EMBEDDINGS_DIMENSIONS: usize = 1536;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingRequest {
    input: String,
    model: String,
    dimensions: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResponseData {
    object: String,
    index: usize,
    embedding: Vec<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmbeddingResponse {
    data: Vec<EmbeddingResponseData>,
}

pub async fn create_embedding(input: &str) -> Result<Vec<f32>, anyhow::Error> {
    let Ok(api_key) = std::env::var("OPENAI_API_KEY") else {
        return Err(anyhow::anyhow!("OPENAI_API_KEY is not set"));
    };

    let client = reqwest::Client::new();

    let input = EmbeddingRequest {
        input: input.to_string(),
        model: String::from("text-embedding-3-small"),
        dimensions: EMBEDDINGS_DIMENSIONS,
    };

    let resp = client
        .post("https://api.openai.com/v1/embeddings")
        .header("Authorization", format!("Bearer {api_key}"))
        .header("Content-Type", "application/json")
        .json(&input)
        .send()
        .await?
        .json::<EmbeddingResponse>()
        .await?;

    let mut data = resp.data;
    Ok(data.remove(0).embedding)
}
