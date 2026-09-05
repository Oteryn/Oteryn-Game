#[path = "support/authority_matrix.rs"]
mod authority_matrix;

#[test]
fn independently_read_fnd02_membership_and_ids_reach_every_consumer()
-> authority_matrix::TestResult<()> {
    use authority_matrix::{ConsumerBoundary, LiveSource, Seed, exercise, prepared_record};
    use serde_json::json;
    let seed = Seed::fixed();
    let record = prepared_record(seed)?;
    for boundary in [
        ConsumerBoundary::CommitV1,
        ConsumerBoundary::CommitV2,
        ConsumerBoundary::ReconcileV1,
        ConsumerBoundary::ReconcileV2,
    ] {
        let source = LiveSource::read(seed);
        assert!(exercise(boundary, &record, &source, seed)?);
        for (field, value) in [
            ("pending_present", json!(false)),
            ("pending_id", json!(2)),
            ("domain_present", json!(false)),
            ("domain_id", json!(2)),
        ] {
            let mut changed = source.clone();
            changed.0[field] = value;
            assert!(
                !exercise(boundary, &record, &changed, seed)?,
                "independent FND02 source ignored: {boundary:?}/{field}"
            );
        }
    }
    Ok(())
}

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
