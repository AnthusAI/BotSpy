//! The embedder seam: the one place where session texts become vectors.
//! The real embedder is [`crate::store::embed_minilm::MiniLMEmbedder`]
//! (all-MiniLM-L6-v2 via tract on CPU, weights loaded from the local
//! cache that setup fetched — never from the network at runtime). The
//! store records the embedder's model id in `meta` on first embed and
//! refuses a different one later, so a store never mixes models.

use std::fmt;
use std::path::PathBuf;

/// Errors the embedder reports instead of panicking.
#[derive(Debug)]
pub enum EmbedError {
    /// The model files have not been fetched by setup yet, so the
    /// embedder cannot load. Nothing was downloaded here: setup fetches
    /// once, the runtime never touches the network.
    ModelMissing { path: PathBuf },
    /// The model or tokenizer could not be loaded or run.
    Runtime {
        context: String,
        source: Box<dyn std::error::Error + Send + Sync>,
    },
}

impl fmt::Display for EmbedError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            EmbedError::ModelMissing { path } => write!(
                f,
                "embedding model files not found at {} (run the botspy setup step once to fetch them)",
                path.display()
            ),
            EmbedError::Runtime { context, source } => {
                write!(f, "embedder error ({context}): {source}")
            }
        }
    }
}

impl std::error::Error for EmbedError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            EmbedError::Runtime { source, .. } => Some(source.as_ref()),
            _ => None,
        }
    }
}

/// Turns texts into fixed-dimension vectors. One model, one set of
/// weights, one engine: implementors pin a model id and a dimension so a
/// store's vectors are consistent by construction.
pub trait Embedder: Send + Sync {
    /// The embedding model's identity, recorded in the store's `meta`.
    fn model_id(&self) -> &'static str;
    /// The vector dimension (all-MiniLM-L6-v2: 384).
    fn dim(&self) -> usize;
    /// Embed a batch of texts, one vector per text, L2-normalized.
    fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedError>;
}

#[cfg(test)]
pub(crate) mod stub {
    use super::{EmbedError, Embedder};
    use std::sync::Mutex;

    /// A deterministic embedder kept solely as an internal unit-test
    /// seam: it is never the shipped embedder and never drives vector
    /// search specs (per the BOTSPY-bb60bb decision). Vectors derive
    /// from a cheap hash of the text so equal texts embed equal and
    /// different texts embed differently, always normalized.
    pub(crate) struct StubEmbedder {
        /// Counts embed calls, for tests that assert batch behavior.
        pub calls: Mutex<usize>,
        /// The model id this stub claims (tests exercise model-mismatch
        /// handling with two differently-named stubs).
        model: &'static str,
    }

    impl StubEmbedder {
        pub(crate) fn new() -> Self {
            StubEmbedder::with_model("stub")
        }

        pub(crate) fn with_model(model: &'static str) -> Self {
            StubEmbedder {
                calls: Mutex::new(0),
                model,
            }
        }
    }

    fn hash_vector(text: &str) -> Vec<f32> {
        let mut vector = [0.0f32; 4];
        for byte in text.bytes() {
            vector[byte as usize % 4] += (byte as f32 % 13.0) + 1.0;
        }
        let norm = vector.iter().map(|v| v * v).sum::<f32>().sqrt();
        vector.iter().map(|v| v / norm).collect()
    }

    impl Embedder for StubEmbedder {
        fn model_id(&self) -> &'static str {
            self.model
        }

        fn dim(&self) -> usize {
            4
        }

        fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedError> {
            *self.calls.lock().expect("stub call count") += texts.len();
            Ok(texts.iter().map(|text| hash_vector(text)).collect())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stub_embeds_deterministically_and_normalized() {
        let stub = stub::StubEmbedder::new();
        let vectors = stub.embed(&["alpha beta", "alpha beta", "gamma"]).unwrap();
        assert_eq!(vectors.len(), 3);
        assert_eq!(vectors[0], vectors[1], "equal texts embed equal");
        assert_ne!(vectors[0], vectors[2], "different texts embed differently");
        for vector in &vectors {
            let norm: f32 = vector.iter().map(|v| v * v).sum::<f32>().sqrt();
            assert!((norm - 1.0).abs() < 1e-5, "vectors are L2-normalized");
        }
    }
}
