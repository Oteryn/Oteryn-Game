// Opt-in local shard of the `postgres_tests/` mounts. Unset (CI, default) mounts every file;
// `OTERYN_DISPATCH_PG_TESTS=a,b` mounts only files whose stem contains `a` or `b`, plus every
// sibling they reach by path (`::name` or `::{.., name}`), so a filtered local test build
// compiles a fraction of the crate. A reference the scan misses fails the build; it never
// drops a test silently from an unfiltered build.
use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::path::{Path, PathBuf};

pub const ENV: &str = "OTERYN_DISPATCH_PG_TESTS";

/// Rewrites the mount list `mounts` (pairs of `#[path = ..]` and `mod stem;` lines) to the
/// selected closure when `ENV` is set and non-empty. Siblings that `parent` (the module that
/// includes the list) or its explicit `#[path]` children reach are always kept.
pub fn filter(mounts: &Path, parent: &Path) -> Result<(), Box<dyn Error>> {
    println!("cargo:rerun-if-env-changed={ENV}");
    let selector = std::env::var(ENV).unwrap_or_default();
    let patterns: Vec<&str> = selector
        .split(',')
        .map(str::trim)
        .filter(|p| !p.is_empty())
        .collect();
    if patterns.is_empty() {
        return Ok(());
    }
    let text = std::fs::read_to_string(mounts)?;
    let lines: Vec<&str> = text.lines().collect();
    let mut modules = BTreeMap::new();
    for pair in lines.chunks(2) {
        let [path, module] = pair else {
            return Err("unpaired mount line".into());
        };
        let stem = module
            .strip_prefix("mod ")
            .and_then(|m| m.strip_suffix(';'))
            .ok_or("unexpected mount line")?;
        let file = path
            .strip_prefix("#[path = \"")
            .and_then(|p| p.strip_suffix("\"]"))
            .ok_or("unexpected path line")?;
        modules.insert(stem.to_owned(), (PathBuf::from(file), *path, *module));
    }
    let mut keep = BTreeSet::new();
    let mut pending: Vec<String> = modules
        .keys()
        .filter(|stem| patterns.iter().any(|p| stem.contains(p)))
        .cloned()
        .collect();
    let mut start = Some(parent.to_path_buf());
    loop {
        let root = match start.take() {
            Some(parent) => parent,
            None => match pending.pop() {
                Some(stem) if keep.insert(stem.clone()) => modules[&stem].0.clone(),
                Some(_) => continue,
                None => break,
            },
        };
        let mut files = BTreeSet::new();
        module_tree(&root, &mut files);
        for file in &files {
            println!("cargo:rerun-if-changed={}", file.display());
            for name in path_segments(&std::fs::read_to_string(file)?) {
                if modules.contains_key(&name) && !keep.contains(&name) {
                    pending.push(name);
                }
            }
        }
    }
    let mut out = String::new();
    for (stem, (_, path, module)) in &modules {
        if keep.contains(stem) {
            out.push_str(&format!("{path}\n{module}\n"));
        }
    }
    std::fs::write(mounts, out)?;
    Ok(())
}

/// The file, every `.rs` file under its stem directory, and every `.rs` string literal it names
/// (`#[path]`, `include!`), resolved against both its directory and its stem directory.
fn module_tree(file: &Path, files: &mut BTreeSet<PathBuf>) {
    if !file.is_file() || !files.insert(file.to_path_buf()) {
        return;
    }
    let stem_dir = file.with_extension("");
    if let Ok(entries) = std::fs::read_dir(&stem_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                module_tree(&path.with_extension("rs"), files);
                for nested in walk(&path) {
                    module_tree(&nested, files);
                }
            } else if path.extension().is_some_and(|e| e == "rs") {
                module_tree(&path, files);
            }
        }
    }
    let text = std::fs::read_to_string(file).unwrap_or_default();
    let dir = file.parent().unwrap_or(Path::new("."));
    for literal in text.split('"').skip(1).step_by(2) {
        if literal.ends_with(".rs") {
            module_tree(&dir.join(literal), files);
            module_tree(&stem_dir.join(literal), files);
        }
    }
}

fn walk(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                found.extend(walk(&path));
            } else if path.extension().is_some_and(|e| e == "rs") {
                found.push(path);
            }
        }
    }
    found
}

/// Identifiers in path position: right after `::`, or anywhere in a `::{ .. }` group.
fn path_segments(text: &str) -> Vec<String> {
    let mut names = Vec::new();
    let mut rest = text;
    while let Some(at) = rest.find("::") {
        rest = rest[at + 2..].trim_start();
        let end = if rest.starts_with('{') {
            let mut depth = 0;
            rest.char_indices()
                .find_map(|(i, c)| {
                    match c {
                        '{' => depth += 1,
                        '}' => depth -= 1,
                        _ => {}
                    }
                    (depth == 0).then_some(i + 1)
                })
                .unwrap_or(rest.len())
        } else {
            rest.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                .unwrap_or(rest.len())
        };
        names.extend(
            rest[..end]
                .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
                .filter(|w| !w.is_empty())
                .map(str::to_owned),
        );
    }
    names
}
