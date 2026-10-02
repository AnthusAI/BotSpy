//! Guardrail: every Gherkin spec file must be embedded in the rustdoc
//! chapters. The docs are organized around the specs, so a `.feature` file
//! that no chapter embeds means the documentation has drifted from the
//! specification tree.

use std::fs;
use std::path::{Path, PathBuf};

fn collect_feature_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|err| panic!("read_dir {}: {err}", dir.display()))
        .map(|entry| entry.expect("directory entry").path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect_feature_files(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("feature") {
            out.push(path);
        }
    }
}

fn collect_doc_sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<PathBuf> = fs::read_dir(dir)
        .unwrap_or_else(|err| panic!("read_dir {}: {err}", dir.display()))
        .map(|entry| entry.expect("directory entry").path())
        .collect();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect_doc_sources(&path, out);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

#[test]
fn every_feature_file_is_embedded_in_a_doc_chapter() {
    let mut features = Vec::new();
    collect_feature_files(Path::new("features"), &mut features);
    assert!(
        !features.is_empty(),
        "no .feature files found under features/; \
         the spec tree must not be empty"
    );

    let mut doc_sources = String::new();
    doc_sources.push_str(
        &fs::read_to_string("src/lib.rs").unwrap_or_else(|err| panic!("read src/lib.rs: {err}")),
    );
    let mut spec_sources = Vec::new();
    collect_doc_sources(Path::new("src/spec"), &mut spec_sources);
    assert!(
        !spec_sources.is_empty(),
        "no chapter modules found under src/spec/"
    );
    for source in spec_sources {
        doc_sources.push_str(
            &fs::read_to_string(&source)
                .unwrap_or_else(|err| panic!("read {}: {err}", source.display())),
        );
        doc_sources.push('\n');
    }

    for feature in &features {
        let relative = feature
            .strip_prefix("features")
            .expect("path under features/")
            .to_string_lossy()
            .replace('\\', "/");
        assert!(
            doc_sources.contains(relative.as_str()),
            "spec file features{relative} is not embedded in any rustdoc \
             chapter; add it to the chapter that mirrors its folder"
        );
    }
}
