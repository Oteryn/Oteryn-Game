// PG-COVERAGE-1 guard. CI runs only the PostgreSQL targets that `.github/workflows/rust.yml`
// names; every other `tests/*_postgres.rs` target is skipped there because the routed database
// is absent. Each such standalone target must therefore be a thin wrapper whose shared
// `support/*_cases.rs` files are all included by a CI-run target. Needs no database.

use std::collections::BTreeSet;
use std::fs;
use std::io;
use std::path::PathBuf;

const CASES_PREFIX: &str = "#[path = \"support/";
const CASES_SUFFIX: &str = "_cases.rs\"]";

fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Target names CI runs: `--test <name>` and `run_registered_target <name> <path>` calls.
fn ci_run_targets() -> io::Result<BTreeSet<String>> {
    let text = fs::read_to_string(manifest_dir().join("../../.github/workflows/rust.yml"))?;
    let mut targets = BTreeSet::new();
    for line in text.lines() {
        let line = line.trim();
        for marker in ["--test ", "run_registered_target "] {
            if let Some(rest) = line.split(marker).nth(1) {
                let name = rest.split_whitespace().next().unwrap_or_default();
                if name.ends_with("_postgres") {
                    targets.insert(name.to_owned());
                }
            }
        }
    }
    Ok(targets)
}

/// The `support/*_cases.rs` files a target includes through `#[path = ...]`.
fn included_cases(source: &str) -> BTreeSet<String> {
    source
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with(CASES_PREFIX) && line.ends_with(CASES_SUFFIX))
        .map(|line| line[CASES_PREFIX.len()..line.len() - 2].to_owned())
        .collect()
}

fn postgres_targets() -> io::Result<Vec<(String, String)>> {
    let mut targets = Vec::new();
    for entry in fs::read_dir(manifest_dir().join("tests"))? {
        let path = entry?.path();
        let Some(name) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        if path.extension().is_some_and(|ext| ext == "rs") && name.ends_with("_postgres") {
            targets.push((name.to_owned(), fs::read_to_string(&path)?));
        }
    }
    targets.sort();
    Ok(targets)
}

#[test]
fn every_postgres_target_runs_its_cases_in_a_ci_run_target() -> io::Result<()> {
    let ci = ci_run_targets()?;
    assert!(
        ci.contains("character_authority_postgres") && ci.contains("durability_postgres"),
        "CI-run PostgreSQL targets not found in rust.yml: {ci:?}"
    );
    let targets = postgres_targets()?;
    let mut aggregated = BTreeSet::new();
    for (name, source) in &targets {
        if ci.contains(name) {
            aggregated.extend(included_cases(source));
        }
    }
    let mut failures = Vec::new();
    for (name, source) in &targets {
        if ci.contains(name) {
            continue;
        }
        if source.contains("#[test]") || source.contains("#[tokio::test") {
            failures.push(format!(
                "{name}: has inline tests; move them into support/*_cases.rs"
            ));
        }
        let cases = included_cases(source);
        if cases.is_empty() {
            failures.push(format!("{name}: includes no support/*_cases.rs file"));
        }
        for case in cases.difference(&aggregated) {
            failures.push(format!("{name}: {case} is not included by a CI-run target"));
        }
    }
    assert!(
        failures.is_empty(),
        "PostgreSQL cases CI never runs (CI-run targets: {ci:?}):\n{}",
        failures.join("\n")
    );
    Ok(())
}
