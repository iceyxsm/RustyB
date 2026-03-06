//! RAG (Retrieval-Augmented Generation) placeholder

use crate::embeddings::EmbeddingModel;
use std::sync::Arc;

/// RAG system for browser history
pub struct BrowserRag {
    _embedding_model: Arc<dyn EmbeddingModel>,
}

impl BrowserRag {
    pub fn new(embedding_model: Arc<dyn EmbeddingModel>) -> Self {
        Self {
            _embedding_model: embedding_model,
        }
    }

    pub async fn index_page(&self, _url: &str, _content: &str) -> anyhow::Result<()> {
        Ok(())
    }

    pub async fn search_history(&self, _query: &str, _limit: usize) -> anyhow::Result<Vec<String>> {
        Ok(vec![])
    }
}
