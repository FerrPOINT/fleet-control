use std::{env, fs, path::PathBuf};

fn main() {
    let source =
        PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap()).join("../live/src/main.rs");
    let original = fs::read_to_string(&source).unwrap();
    let header = "//! QA-only actual Docker/Hermes driver. No producer receipts or admission are fabricated.\n";
    // include! cannot accept the file's inner doc comment. No executable code changes.
    let body = original
        .strip_prefix(header)
        .expect("Frozen driver header drift");
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("original.rs"),
        body,
    )
    .unwrap();
    println!("cargo:rerun-if-changed={}", source.display());
}
