use lazy_static::lazy_static;

#[derive(Debug)]
pub struct Config {
    pub elevenlabs_api_key: String,
    pub anthropic_api_key: String,
    pub openai_api_key: String,
    pub qdrant_url: String,
    pub mongodb_url: String,
    pub mongodb_database: String,
}

impl Config {
    pub fn from_env() -> Result<Self, anyhow::Error> {
        Ok(Self {
            elevenlabs_api_key: std::env::var("ELEVENLABS_API_KEY").unwrap_or("".to_string()),
            anthropic_api_key: std::env::var("ANTHROPIC_API_KEY").unwrap_or("".to_string()),
            openai_api_key: std::env::var("OPENAI_API_KEY").unwrap_or("".to_string()),
            qdrant_url: std::env::var("QDRANT_URL").unwrap_or("http://localhost:6333".to_string()),
            mongodb_url: std::env::var("MONGODB_URL")
                .unwrap_or("mongodb://localhost:27017".to_string()),
            mongodb_database: std::env::var("MONGODB_DATABASE").unwrap_or("jumo_rs".to_string()),
        })
    }
}

lazy_static! {
    pub static ref CONFIG: Config = Config::from_env().expect("Failed to load config");
}
