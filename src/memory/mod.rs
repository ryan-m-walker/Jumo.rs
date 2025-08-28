use crate::services::qdrant::QdrantService;

pub mod postgres;

pub struct Memory {}

impl Memory {
    pub fn new() -> Self {
        Self {}
    }
}
