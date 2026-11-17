mod build_mechanics;
#[path = "../../../apps/game-server/build_mounts.rs"]
mod build_mounts;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR")?);
    build_mechanics::generate(&root, &out)?;
    build_mounts::mount_directory(
        "src/gameplay_transport/connection/routing",
        "mechanic_routing_modules.rs",
        "",
    )?;
    // The shared mount helper emits private test modules; routing bodies are visible to
    // the sibling serve phases, retaining their previous pub(super) module visibility.
    let routing = out.join("mechanic_routing_modules.rs");
    let mounts = std::fs::read_to_string(&routing)?.replace("\nmod ", "\npub(super) mod ");
    std::fs::write(routing, mounts)?;
    // A new mechanic drops its PostgreSQL test file into this directory instead of appending a
    // `#[path] mod` line to `monk_save_postgres_tests.rs`, which every mechanic also edits.
    build_mounts::mount_directory(
        "src/gameplay_transport/postgres_tests",
        "postgres_tests_mounts.rs",
        "",
    )?;
    focus_postgres_mounts(&root, &out)
}

// A focused runtime filter still makes rustc compile every mounted PG case. Allow
// memory-constrained local runs to compile a group and its helper dependency closure.
// An unset selector retains the complete, sorted mount produced above.
fn focus_postgres_mounts(
    root: &std::path::Path,
    out: &std::path::Path,
) -> Result<(), Box<dyn std::error::Error>> {
    use std::collections::{BTreeMap, BTreeSet};
    println!("cargo:rerun-if-env-changed=OTERYN_POSTGRES_TEST_GROUPS");
    let Ok(groups) = std::env::var("OTERYN_POSTGRES_TEST_GROUPS") else {
        return Ok(());
    };
    let directory = root.join("src/gameplay_transport/postgres_tests");
    let mut modules = BTreeMap::new();
    for entry in std::fs::read_dir(&directory)? {
        let path = entry?.path();
        if path.extension().is_some_and(|extension| extension == "rs") {
            let name = path
                .file_stem()
                .ok_or("PG module name")?
                .to_str()
                .ok_or("PG module UTF-8")?;
            modules.insert(name.to_owned(), path);
        }
    }
    let mut selected = BTreeSet::new();
    let mut pending = vec![root.join("src/gameplay_transport/monk_save_postgres_tests.rs")];
    for name in groups.split(',') {
        let path = modules
            .get(name)
            .ok_or_else(|| format!("unknown PostgreSQL test group {name:?}"))?;
        selected.insert(name.to_owned());
        pending.push(path.clone());
    }
    let mut visited = BTreeSet::new();
    while let Some(path) = pending.pop() {
        let path = path.canonicalize()?;
        if !visited.insert(path.clone()) {
            continue;
        }
        println!("cargo:rerun-if-changed={}", path.display());
        let source = std::fs::read_to_string(&path)?;
        // Conservative token matching also retains dependencies mentioned in
        // helper submodules; extra helpers are safe, a missing helper won't compile.
        for token in source.split(|c: char| !c.is_ascii_alphanumeric() && c != '_') {
            if let Some(dependency) = modules.get(token) {
                if selected.insert(token.to_owned()) {
                    pending.push(dependency.clone());
                }
            }
        }
        let parent = path.parent().ok_or("PG source parent")?;
        for suffix in source.split("#[path = \"").skip(1) {
            if let Some((relative, _)) = suffix.split_once('"') {
                let dependency = parent.join(relative);
                if dependency.is_file() {
                    pending.push(dependency);
                }
            }
        }
        let children = path.with_extension("");
        if children.is_dir() {
            pending.extend(rust_sources(&children)?);
        }
    }
    let mut mounts = String::new();
    for name in &selected {
        let path = modules.get(name).ok_or("selected PG module")?;
        mounts.push_str(&format!(
            "#[path = {:?}]\nmod {name};\n",
            path.to_str().ok_or("PG path UTF-8")?
        ));
    }
    println!(
        "cargo:warning=Focused PostgreSQL mounts: {} of {} groups",
        selected.len(),
        modules.len()
    );
    std::fs::write(out.join("postgres_tests_mounts.rs"), mounts)?;
    Ok(())
}

fn rust_sources(directory: &std::path::Path) -> Result<Vec<std::path::PathBuf>, std::io::Error> {
    let mut sources = Vec::new();
    for entry in std::fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            sources.extend(rust_sources(&path)?);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            sources.push(path);
        }
    }
    Ok(sources)
}
