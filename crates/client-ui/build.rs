//! Step 1 composition seam: validate additive sources and generate private descriptors.
#[path = "../../tools/architecture-check/src/client_fragments.rs"]
mod client_fragments;

use std::path::Path;

fn run() -> Result<(), String> {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").map_err(|e| e.to_string())?;
    let root = Path::new(&manifest).join("../..");
    let out = std::env::var("OUT_DIR").map_err(|e| e.to_string())?;
    for path in [
        "crates/client-ui/src/features",
        "crates/protocol-oteryn/capabilities",
        "docs/contracts/protocol-oteryn/v1/registry/command_types",
        "tools/architecture-check/client-fragment.schema.json",
        "tools/architecture-check/client-unsupported-baseline.json",
        "tools/architecture-check/client-unsupported-baseline.d",
        "tools/architecture-check/src/client_fragments.rs",
        "tools/architecture-check/src/client_fragments",
    ] {
        println!("cargo:rerun-if-changed={}", root.join(path).display());
    }
    let (entries, coverage) = client_fragments::scan(&root)?;
    let (rust, report) = client_fragments::render(&entries, &coverage)?;
    std::fs::write(Path::new(&out).join("client_registrations.rs"), rust)
        .map_err(|e| e.to_string())?;
    std::fs::write(
        Path::new(&out).join("client-mechanic-coverage.json"),
        report,
    )
    .map_err(|e| e.to_string())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("client registrations: {error}");
        std::process::exit(1);
    }
}
