//! One-time setup: download and digest-verify the embedding model's
//! assets into the local models cache (`BOTSPY_MODELS` or
//! `$HOME/.botspy/models`). Everything after this is fully offline; the
//! runtime library itself never touches the network.
//!
//! ```console
//! $ cargo run --example setup_models
//! ```

fn main() {
    match botspy::store::assets::fetch_assets() {
        Ok(model_dir) => {
            println!(
                "Model assets are cached and verified at {}.",
                model_dir.display()
            );
        }
        Err(err) => {
            eprintln!("Setup failed: {err}");
            std::process::exit(1);
        }
    }
}
