//! Setup-time acquisition of the embedding model's assets: the quantized
//! all-MiniLM-L6-v2 ONNX weights and its tokenizer. Per the BOTSPY-bb60bb
//! decision, the model is never committed to the repository and the
//! runtime library makes no network calls — a setup step downloads each
//! asset exactly once (idempotent, content-digest verified) and caches it
//! under the models dir (`BOTSPY_MODELS` or `$HOME/.botspy/models`).
//! Everything after setup is fully offline; nothing leaves the machine.

use crate::store::embed_minilm::models_dir;
use sha2::{Digest, Sha256};
use std::fmt;
use std::io::Read;
use std::path::{Path, PathBuf};

/// The embedding model's identity and cache directory name.
pub const MODEL_ID: &str = "all-MiniLM-L6-v2";

/// One cacheable asset: where setup fetches it from and the exact bytes
/// that must arrive (digest-verified, so a truncated or tampered download
/// never lands in the cache).
struct Asset {
    file: &'static str,
    url: &'static str,
    sha256: &'static str,
}

/// The assets of the shipped model. Digests are pinned to the files the
/// model was validated with.
const ASSETS: [Asset; 2] = [
    Asset {
        file: "model_quantized.onnx",
        url:
            "https://huggingface.co/Xenova/all-MiniLM-L6-v2/resolve/main/onnx/model_quantized.onnx",
        sha256: "afdb6f1a0e45b715d0bb9b11772f032c399babd23bfc31fed1c170afc848bdb1",
    },
    Asset {
        file: "tokenizer.json",
        url: "https://huggingface.co/Xenova/all-MiniLM-L6-v2/resolve/main/tokenizer.json",
        sha256: "da0e79933b9ed51798a3ae27893d3c5fa4a201126cef75586296df9b4d2c62a0",
    },
];

/// Errors setup reports instead of panicking.
#[derive(Debug)]
pub enum AssetsError {
    /// A cache path could not be created or written.
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    /// The download failed (offline, unreachable, HTTP error).
    Network {
        url: &'static str,
        source: ureq::Error,
    },
    /// The downloaded bytes did not match the pinned digest.
    DigestMismatch {
        file: &'static str,
        expected: &'static str,
        actual: String,
    },
}

impl fmt::Display for AssetsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AssetsError::Io { path, source } => {
                write!(
                    f,
                    "model cache path {} is unusable: {source}",
                    path.display()
                )
            }
            AssetsError::Network { url, source } => {
                write!(f, "download of {url} failed: {source}")
            }
            AssetsError::DigestMismatch {
                file,
                expected,
                actual,
            } => write!(
                f,
                "downloaded {file} has digest {actual}, expected {expected}"
            ),
        }
    }
}

impl std::error::Error for AssetsError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            AssetsError::Io { source, .. } => Some(source),
            AssetsError::Network { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Fetch the model assets into the default models cache
/// (`BOTSPY_MODELS` or `$HOME/.botspy/models`). Download-once and
/// idempotent: an asset already cached with the pinned digest is left
/// untouched; a corrupted or partial one is replaced.
pub fn fetch_assets() -> Result<PathBuf, AssetsError> {
    let models_root = models_dir().map_err(|err| AssetsError::Io {
        path: PathBuf::from("models cache"),
        source: std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("the models cache has no root: {err}"),
        ),
    })?;
    fetch_assets_at(&models_root)?;
    Ok(models_root.join(MODEL_ID))
}

/// Fetch the model assets into an explicit cache root (the test seam).
pub fn fetch_assets_at(models_root: &Path) -> Result<(), AssetsError> {
    let model_dir = models_root.join(MODEL_ID);
    std::fs::create_dir_all(&model_dir).map_err(|source| AssetsError::Io {
        path: model_dir.clone(),
        source,
    })?;
    for asset in &ASSETS {
        let path = model_dir.join(asset.file);
        if let Ok(existing) = std::fs::File::open(&path) {
            let digest = file_sha256(existing).map_err(|source| AssetsError::Io {
                path: path.clone(),
                source,
            })?;
            if digest == asset.sha256 {
                continue;
            }
        }
        download_verified(asset, &path)?;
    }
    Ok(())
}

fn file_sha256(mut file: std::fs::File) -> Result<String, std::io::Error> {
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(hex_digest(hasher))
}

fn download_verified(asset: &Asset, path: &Path) -> Result<(), AssetsError> {
    let response = ureq::get(asset.url)
        .call()
        .map_err(|source| AssetsError::Network {
            url: asset.url,
            source,
        })?;
    let temp_path = path.with_extension("part");
    let file = std::fs::File::create(&temp_path).map_err(|source| AssetsError::Io {
        path: temp_path.clone(),
        source,
    })?;
    let mut writer = std::io::BufWriter::new(file);
    let mut reader = response.into_body().into_reader();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = reader
            .read(&mut buffer)
            .map_err(|err| AssetsError::Network {
                url: asset.url,
                source: ureq::Error::Io(err),
            })?;
        if read == 0 {
            break;
        }
        std::io::Write::write_all(&mut writer, &buffer[..read]).map_err(|source| {
            AssetsError::Io {
                path: temp_path.clone(),
                source,
            }
        })?;
    }
    std::io::Write::flush(&mut writer).map_err(|source| AssetsError::Io {
        path: temp_path.clone(),
        source,
    })?;
    install_verified(asset, &temp_path, path)
}

/// Digest-check a fully downloaded temp file and promote it into place:
/// the only way bytes ever enter the cache is through a matching digest.
fn install_verified(asset: &Asset, temp_path: &Path, path: &Path) -> Result<(), AssetsError> {
    let actual = file_sha256(
        std::fs::File::open(temp_path).map_err(|source| AssetsError::Io {
            path: temp_path.to_path_buf(),
            source,
        })?,
    )
    .map_err(|source| AssetsError::Io {
        path: temp_path.to_path_buf(),
        source,
    })?;
    if actual != asset.sha256 {
        let _ = std::fs::remove_file(temp_path);
        return Err(AssetsError::DigestMismatch {
            file: asset.file,
            expected: asset.sha256,
            actual,
        });
    }
    std::fs::rename(temp_path, path).map_err(|source| AssetsError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    Ok(())
}

fn hex_digest(hasher: Sha256) -> String {
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mismatched_digest_never_reaches_the_cache() {
        let dir = std::env::temp_dir().join(format!("botspy-assets-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let temp = dir.join("model_quantized.part");
        let dest = dir.join("model_quantized.onnx");
        std::fs::write(&temp, b"not the model").unwrap();
        let err = install_verified(&ASSETS[0], &temp, &dest).unwrap_err();
        assert!(
            matches!(err, AssetsError::DigestMismatch { .. }),
            "expected a digest mismatch, got {err:?}"
        );
        assert!(!temp.exists(), "the rejected download must be removed");
        assert!(!dest.exists(), "wrong bytes must never land in the cache");
    }

    #[test]
    fn matching_digest_promotes_the_temp_file() {
        let dir = std::env::temp_dir().join(format!("botspy-assets-ok-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let temp = dir.join("tokenizer.part");
        let dest = dir.join("tokenizer.json");
        std::fs::write(&temp, b"exact tokenizer bytes").unwrap();
        // A pinned asset whose digest matches these exact bytes.
        let asset = Asset {
            file: "tokenizer.json",
            url: "unused",
            sha256: "39f16a07484b4b3acb282eb0da22710dfd4c7df65284eb00e5bf9270f6235376",
        };
        install_verified(&asset, &temp, &dest).unwrap();
        assert!(dest.exists());
        assert!(!temp.exists());
    }
}
