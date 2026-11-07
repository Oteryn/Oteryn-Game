// `character_authority_postgres` also runs in the Rust lanes that have no PostgreSQL, so every case
// mounted from `support/` returns `Ok(())` when the admin URL is absent and asserts only in the
// character-authority leg. TUTORIAL-1 shipped five cases that returned an error instead, which
// passed the routed leg and broke `main` in the route-less lane; this guard fails before that.

use std::path::Path;

/// Source shapes (whitespace removed) that turn a missing admin URL into a test failure.
const ROUTE_REQUIRED: [&str; 2] = [
    concat!("configured_admin", "().ok_or("),
    concat!("\"OTERYN_TEST_POSTGRES_ADMIN_URL\"", ")?"),
];

#[test]
fn support_cases_return_early_without_routed_postgres() -> Result<(), Box<dyn std::error::Error>> {
    let mut offenders = Vec::new();
    scan(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/support"),
        &mut offenders,
    )?;
    assert!(
        offenders.is_empty(),
        "these support cases fail without routed PostgreSQL; return Ok(()) when \
         configured_admin() is None instead: {offenders:?}"
    );
    Ok(())
}

fn scan(dir: &Path, offenders: &mut Vec<String>) -> Result<(), Box<dyn std::error::Error>> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            scan(&path, offenders)?;
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            let source: String = std::fs::read_to_string(&path)?
                .chars()
                .filter(|c| !c.is_whitespace())
                .collect();
            if ROUTE_REQUIRED.iter().any(|shape| source.contains(shape)) {
                offenders.push(path.display().to_string());
            }
        }
    }
    Ok(())
}
