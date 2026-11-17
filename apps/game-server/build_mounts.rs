//! Directory mounts for build scripts (`#[path]`-included by `apps/game-server/build.rs` and
//! `crates/gameplay/dispatch/build.rs`): a shared test mount list every mechanic appended to was
//! the most frequent merge conflict, so those targets mount a directory instead.

use std::{collections::BTreeSet, error::Error, path::Path};

/// Writes `OUT_DIR/<out>` with one `#[path] mod <file stem>;` per `.rs` file of `dir`, sorted, so a
/// target mounts them with `include!(concat!(env!("OUT_DIR"), "/<out>"));`.
pub fn mount_directory(dir: &str, out: &str, attributes: &str) -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-changed={dir}");
    let root = Path::new(&std::env::var("CARGO_MANIFEST_DIR")?).join(dir);
    let mut files = Vec::new();
    for entry in std::fs::read_dir(&root).map_err(|error| format!("{}: {error}", root.display()))? {
        let path = entry?.path();
        if path.extension().is_some_and(|extension| extension == "rs") {
            files.push(path);
        }
    }
    files.sort();
    // Cargo's runtime test filter still compiles every mounted PostgreSQL case. Allow
    // an explicit local module selection on memory-constrained dispatch checkouts.
    // An unset selector retains every test, including in CI; typos must fail the build.
    let selected = if out == "postgres_tests_mounts.rs"
        && std::env::var("CARGO_PKG_NAME").as_deref() == Ok("oteryn-gameplay-dispatch")
    {
        println!("cargo:rerun-if-env-changed=OTERYN_DISPATCH_POSTGRES_MODULES");
        println!("cargo:rustc-check-cfg=cfg(oteryn_dispatch_focused_pg)");
        std::env::var_os("OTERYN_DISPATCH_POSTGRES_MODULES")
            .map(|value| {
                value
                    .into_string()
                    .map_err(|_| "OTERYN_DISPATCH_POSTGRES_MODULES must be UTF-8")
            })
            .transpose()?
            .map(|value| {
                value
                    .split(',')
                    .map(|name| name.trim().to_owned())
                    .collect::<BTreeSet<_>>()
            })
    } else {
        None
    };
    if selected.as_ref().is_some_and(|names| names.contains("")) {
        return Err("OTERYN_DISPATCH_POSTGRES_MODULES must name nonempty modules".into());
    }
    let mut unmatched = selected.clone().unwrap_or_default();
    let mut mounts = String::new();
    for path in files {
        let stem = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or_default();
        let snake_case = stem.starts_with(|c: char| c.is_ascii_lowercase())
            && stem
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
        if !snake_case {
            return Err(format!("{} must be named as a snake_case module", path.display()).into());
        }
        if selected.as_ref().is_some_and(|names| !names.contains(stem)) {
            continue;
        }
        unmatched.remove(stem);
        let path = path.to_str().ok_or("mount path is not UTF-8")?;
        mounts.push_str(&format!("{attributes}#[path = {path:?}]\nmod {stem};\n"));
    }
    if let Some(name) = unmatched.first() {
        return Err(format!("unknown dispatch PostgreSQL test module: {name}").into());
    }
    if selected.is_some() {
        println!("cargo:rustc-cfg=oteryn_dispatch_focused_pg");
    }
    std::fs::write(Path::new(&std::env::var("OUT_DIR")?).join(out), mounts)?;
    Ok(())
}
