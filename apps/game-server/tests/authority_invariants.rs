#[path = "support/authority_matrix.rs"]
mod authority_matrix;

#[test]
fn authority_registry_executes_every_declared_boundary() -> authority_matrix::TestResult<()> {
    let executions = authority_matrix::run_matrix()?;
    for boundary in [
        "terminal-prepare",
        "commit-v1",
        "commit-v2",
        "reconcile-v1",
        "reconcile-v2",
    ] {
        assert!(
            executions.iter().any(|case| case.starts_with(boundary)),
            "missing executed authority boundary: {boundary}"
        );
    }
    Ok(())
}
