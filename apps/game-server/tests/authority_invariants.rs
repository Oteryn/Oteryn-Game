#[path = "support/authority_matrix.rs"]
mod authority_matrix;

#[test]
fn historical_terminal_projection_preserves_reason_under_source_mutations()
-> authority_matrix::TestResult<()> {
    use authority_matrix::*;
    use oteryn_game_server::foundation::*;
    let seed = Seed::fixed();
    let record = prepared_record(seed)?;
    for disposition in [
        ReconnectDurableTerminalDispositionV1::TransportRefCollision,
        ReconnectDurableTerminalDispositionV1::ConcurrentPrepared,
        ReconnectDurableTerminalDispositionV1::StaleAuthority,
    ] {
        run_terminal_matrix(
            seed,
            &record,
            &LiveSource::read(seed),
            ReconnectDurableReconciliationSnapshotV1::terminal(record.clone()),
            ReconnectDurableReconciliationSnapshotV2::new(
                record.clone(),
                ReconnectDurableOutcomeV2::Terminal { disposition },
            ),
            disposition,
        )?;
    }
    Ok(())
}

#[test]
fn retry_replay_executes_each_registered_final_revalidation_case()
-> authority_matrix::TestResult<()> {
    use authority_matrix::*;
    use oteryn_game_server::foundation::*;
    let seed = Seed::fixed();
    let cases = run_retry_matrix(
        seed,
        &prepared_record(seed)?,
        &LiveSource::read(seed),
        None,
        Some(ReconnectPrepareDispositionV1::ExistingPrepared),
        ReconnectPrepareDispositionV2::ExistingPrepared,
    )?;
    for boundary in [ConsumerBoundary::CommitV1, ConsumerBoundary::CommitV2] {
        for &invariant in AuthorityInvariant::ALL {
            if boundary.not_applicable(invariant).is_some() {
                continue;
            }
            for &operator in invariant.operators() {
                let label = format!("{}/{invariant:?}/{operator:?}", boundary.label());
                assert!(
                    cases.contains(&label),
                    "missing executed retry revalidation: {label}"
                );
            }
        }
    }
    Ok(())
}

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
            ("pending_extra", json!(true)),
            ("domain_extra", json!(true)),
            ("next_command", json!(2)),
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
