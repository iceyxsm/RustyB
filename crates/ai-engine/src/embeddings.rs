//! Embeddings placeholder

/// Embedding model trait
pub trait EmbeddingModel: Send + Sync {
    fn embed(&self, text: &str) -> anyhow::Result<Vec<f32>>;
}

/// Mock embedding model
pub struct MockEmbeddingModel;

impl EmbeddingModel for MockEmbeddingModel {
    fn embed(&self, _text: &str) -> anyhow::Result<Vec<f32>> {
        Ok(vec![0.0; 384])
    }
}
