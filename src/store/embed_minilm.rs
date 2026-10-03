//! The real embedder: all-MiniLM-L6-v2 through tract (ONNX) on CPU. One
//! model, one set of weights, one engine — every vector this store
//! computes comes from the same quantized weights, so they are consistent
//! by construction. The weights and tokenizer are loaded from the local
//! cache that the setup step fetched (BOTSPY-c3cf41): the runtime never
//! touches the network, and a cache that setup has not populated is a
//! clean typed error, never a silent download.

use crate::store::embed::{EmbedError, Embedder};
use std::path::{Path, PathBuf};

use tract_onnx::prelude::*;
use tract_onnx::tract_core::runtime::DefaultRuntime;
use tract_onnx::tract_hir::infer::Factoid;

/// The model's directory and file names under the models cache.
const MODEL_DIR: &str = "all-MiniLM-L6-v2";
const ONNX_FILE: &str = "model_quantized.onnx";
const TOKENIZER_FILE: &str = "tokenizer.json";

/// Token sequences are padded/truncated to this length (well above the
/// model's 256-window for typical session summaries, fixed so the tract
/// plan is compiled once).
const SEQ: usize = 128;

/// all-MiniLM-L6-v2's output dimension.
const DIM: usize = 384;

/// The compiled ONNX plan produced by the default CPU runtime.
type Plan = Box<dyn Runnable>;

/// The shipped embedder: quantized all-MiniLM-L6-v2, wordpiece
/// tokenization, mean pooling over the attention mask, L2-normalized
/// output. Model id `all-MiniLM-L6-v2`, dimension 384.
pub struct MiniLMEmbedder {
    tokenizer: tokenizers::Tokenizer,
    input_dt: DatumType,
    runnable: Plan,
}

/// The models cache root: `BOTSPY_MODELS` when set, else
/// `$HOME/.botspy/models` — the same home the store and snapshots use.
pub fn models_dir() -> Result<PathBuf, EmbedError> {
    if let Some(models) = std::env::var_os("BOTSPY_MODELS") {
        return Ok(PathBuf::from(models));
    }
    let home = std::env::var_os("HOME").ok_or_else(|| EmbedError::Runtime {
        context: "locate models cache".to_string(),
        source: "no BOTSPY_MODELS and no $HOME".into(),
    })?;
    Ok(PathBuf::from(home).join(".botspy").join("models"))
}

fn runtime_error(
    context: &'static str,
) -> impl FnOnce(Box<dyn std::error::Error + Send + Sync>) -> EmbedError {
    move |source| EmbedError::Runtime {
        context: context.to_string(),
        source,
    }
}

/// Rewrite symbolic dims that the pinned plan binds, everywhere in the
/// graph: load-time symbolic facts (`batch_size`, `sequence_length`) would
/// otherwise refuse to unify with the concrete facts the plan's inputs
/// propagate. Any symbol not listed stays symbolic.
fn concretize_model_dims(model: &mut InferenceModel, bindings: &[(&str, i64)]) {
    for node in model.nodes_mut() {
        for outlet in node.outputs.iter_mut() {
            let fact = &mut outlet.fact;
            for i in 0..fact.shape.dims().count() {
                if let Some(dim) = fact.shape.dim(i) {
                    if let Some(td) = dim.concretize() {
                        let name = format!("{td}");
                        if let Some((_, value)) = bindings.iter().find(|(bound, _)| *bound == name)
                        {
                            fact.shape.set_dim(i, (*value).to_dim());
                        }
                    }
                }
            }
        }
    }
}

impl MiniLMEmbedder {
    /// Open the embedder from the default models cache. Missing files
    /// mean setup has not run yet: a clean [`EmbedError::ModelMissing`].
    pub fn open() -> Result<Self, EmbedError> {
        Self::open_at(&models_dir()?)
    }

    /// Open the embedder from an explicit cache root (the unit-test and
    /// setup seams).
    pub fn open_at(models_root: &Path) -> Result<Self, EmbedError> {
        let dir = models_root.join(MODEL_DIR);
        let onnx_path = dir.join(ONNX_FILE);
        let tokenizer_path = dir.join(TOKENIZER_FILE);
        if !onnx_path.is_file() {
            return Err(EmbedError::ModelMissing { path: onnx_path });
        }
        if !tokenizer_path.is_file() {
            return Err(EmbedError::ModelMissing {
                path: tokenizer_path,
            });
        }
        let tokenizer = tokenizers::Tokenizer::from_file(&tokenizer_path)
            .map_err(|err| runtime_error("load tokenizer")(err))?;
        let mut tokenizer = tokenizer;
        tokenizer
            .with_truncation(Some(tokenizers::TruncationParams {
                max_length: SEQ,
                ..Default::default()
            }))
            .map_err(|err| runtime_error("configure truncation")(err))?;
        let mut model = tract_onnx::onnx()
            .model_for_path(&onnx_path)
            .map_err(|err| runtime_error("load ONNX model")(err.into()))?;
        // The graph carries load-time symbolic facts (`batch_size`,
        // `sequence_length`) that disagree with the concrete batch-1,
        // SEQ-length facts the plan pins; rewriting the symbols everywhere
        // before typing keeps the two consistent. The store only ever runs
        // this model one text at a time.
        concretize_model_dims(
            &mut model,
            &[("batch_size", 1), ("sequence_length", SEQ as i64)],
        );
        for input in 0..model.inputs.len() {
            model
                .set_input_fact(
                    input,
                    InferenceFact::dt_shape(DatumType::I64, vec![1usize, SEQ]),
                )
                .map_err(|err| runtime_error("set input facts")(err.into()))?;
        }
        let typed_model = model
            .into_typed()
            .map_err(|err| runtime_error("type the model")(err.into()))?;
        let input_dt = typed_model
            .input_fact(0)
            .map_err(|err| runtime_error("read input fact")(err.into()))?
            .datum_type;
        let runnable = DefaultRuntime
            .prepare(typed_model)
            .map_err(|err| runtime_error("compile model plan")(err.into()))?;
        Ok(MiniLMEmbedder {
            tokenizer,
            input_dt,
            runnable,
        })
    }

    /// One int input tensor in the model's declared datum type.
    fn int_tensor(&self, data: Vec<i64>) -> TValue {
        match self.input_dt {
            DatumType::I64 => tract_ndarray::Array2::from_shape_vec((1, SEQ), data)
                .expect("int input shape")
                .into_tvalue(),
            _ => tract_ndarray::Array2::from_shape_vec(
                (1, SEQ),
                data.iter().map(|&value| value as i32).collect(),
            )
            .expect("int input shape")
            .into_tvalue(),
        }
    }

    fn embed_one(&self, text: &str) -> Result<Vec<f32>, EmbedError> {
        let encoding = self
            .tokenizer
            .encode(text, true)
            .map_err(|err| runtime_error("tokenize text")(err))?;
        // The pinned tokenizer pads every encoding to a fixed length, so
        // the id count is NOT the token count: the attention mask is. The
        // model gets the encoding's own mask, and pooling covers only the
        // attended positions — pooling over pad positions would swamp the
        // text signal with a near-constant pad component.
        let mask_ref = encoding.get_attention_mask();
        let attended = mask_ref.iter().filter(|&&m| m == 1).count();
        let mut ids: Vec<i64> = encoding.get_ids().iter().map(|&id| id as i64).collect();
        ids.resize(SEQ, 0);
        let mut mask: Vec<i64> = mask_ref.iter().map(|&m| m as i64).collect();
        mask.resize(SEQ, 0);
        let mut type_ids: Vec<i64> = encoding
            .get_type_ids()
            .iter()
            .take(SEQ)
            .map(|&id| id as i64)
            .collect();
        type_ids.resize(SEQ, 0);
        let outputs = self
            .runnable
            .run(tvec![
                self.int_tensor(ids),
                self.int_tensor(mask),
                self.int_tensor(type_ids),
            ])
            .map_err(|err| runtime_error("run model")(err.into()))?;
        let hidden = outputs[0]
            .to_plain_array_view::<f32>()
            .map_err(|err| runtime_error("read model output")(err.into()))?;
        let mut pooled = vec![0.0f32; DIM];
        for position in 0..attended {
            for dim in 0..DIM {
                pooled[dim] += hidden[[0, position, dim]];
            }
        }
        if attended > 0 {
            let attended = attended as f32;
            for value in pooled.iter_mut() {
                *value /= attended;
            }
        }
        let norm = pooled.iter().map(|v| v * v).sum::<f32>().sqrt();
        if norm > 0.0 {
            for value in pooled.iter_mut() {
                *value /= norm;
            }
        }
        Ok(pooled)
    }
}

impl std::fmt::Debug for MiniLMEmbedder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MiniLMEmbedder")
            .field("model_id", &"all-MiniLM-L6-v2")
            .finish()
    }
}

impl Embedder for MiniLMEmbedder {
    fn model_id(&self) -> &'static str {
        "all-MiniLM-L6-v2"
    }

    fn dim(&self) -> usize {
        DIM
    }

    fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>, EmbedError> {
        texts.iter().map(|text| self.embed_one(text)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_model_files_are_a_clean_error() {
        let err =
            MiniLMEmbedder::open_at(Path::new("/nonexistent-botspy-models-cache")).unwrap_err();
        assert!(matches!(err, EmbedError::ModelMissing { .. }), "{err:?}");
    }

    /// Real-model sanity: the shipped embedder puts a paraphrase nearer
    /// the original than an unrelated text is. Ungated per the
    /// BOTSPY-bb60bb decision — CI fetches the model assets first
    /// (`cargo run --example setup_models`); a missing cache fails loudly
    /// rather than silently skipping.
    #[test]
    fn minilm_embeds_related_texts_closer() {
        let embedder = MiniLMEmbedder::open()
            .expect("model assets missing: run `cargo run --example setup_models` once");
        let vectors = embedder
            .embed(&[
                "the cat sat on the mat",
                "a feline rested on the rug",
                "quarterly tax filing deadlines",
            ])
            .expect("the real embedder runs");
        assert_eq!(vectors.len(), 3);
        for vector in &vectors {
            assert_eq!(vector.len(), DIM);
            let norm: f32 = vector.iter().map(|v| v * v).sum::<f32>().sqrt();
            assert!((norm - 1.0).abs() < 1e-3, "L2-normalized, norm {norm}");
        }
        let dot = |a: &[f32], b: &[f32]| -> f64 {
            a.iter()
                .zip(b)
                .map(|(x, y)| (*x as f64) * (*y as f64))
                .sum()
        };
        let related = dot(&vectors[0], &vectors[1]);
        let unrelated = dot(&vectors[0], &vectors[2]);
        assert!(
            related > unrelated + 0.3,
            "the paraphrase ({related}) must clearly beat the unrelated text ({unrelated}); \
             a compressed spread means pooling or masking went wrong"
        );
    }
}
