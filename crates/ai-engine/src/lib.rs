//! AI integration layer for the browser

pub mod embeddings;
pub mod features;
pub mod llm;
pub mod rag;

pub use embeddings::*;
pub use features::*;
pub use llm::*;
pub use rag::*;
