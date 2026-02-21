//! Embedding abstraction layer: `Embedder` trait, Vertex AI provider, and mock.

pub mod mock;
pub mod traits;
pub mod vertex;

pub use mock::MockEmbedder;
pub use traits::Embedder;
pub use vertex::VertexAiEmbedder;
