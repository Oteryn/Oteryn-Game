use crate::bestiary_postgres_harness as fixture;
use crate::durability::{DB_PASS_DEADLINE, DurabilityError};
use crate::durability::runtime_scope_assignment::{
    AssignmentError, BootstrapSecret, LaunchBinding, RegistrationError,
};
use oteryn_gameplay_durability::durability::with_stalled_commit_response;

#[test]
fn assignment_read_gets_full_pass_budget_after_tokio_clock_advance() -> fixture::TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            fixture::runtime()
                .map_err(fixture::debug)?
                .block_on(async {
                    let h = fixture::Harness::create(admin, "advanced_pass_clock", true).await?;
                    let scope = fixture::scope()?;
                    tokio::time::pause();
                    tokio::time::advance(DB_PASS_DEADLINE * 2).await;
                    let started = tokio::time::Instant::now();
                    let pass = h.root.try_issue_semantic_pass();
                    tokio::time::resume();
                    let result = async {
                        let deadline: tokio::time::Instant =
                            pass?.run(|_, deadline| Box::pin(async move { Ok(deadline) }))
                                .await?;
                        let budget = deadline.saturating_duration_since(started);
                        if budget != DB_PASS_DEADLINE {
                            return Err(format!(
                                "semantic pass received {budget:?} of {DB_PASS_DEADLINE:?}"
                            )
                            .into());
                        }
                        let assignment = h
                            .root
                            .read_runtime_scope_assignment(scope)
                            .await
                            .map_err(fixture::debug)?
                            .ok_or("missing assigned scope")?;
                        assert_eq!(assignment.holder, Some(h.node.fact()));
                        Ok::<(), Box<dyn std::error::Error>>(())
                    }
                    .await;
                    h.cleanup().await?;
                    result
                })
                .map_err(fixture::debug)
        })?
        .join()
        .map_err(|_| "advanced semantic pass clock worker panicked")?
        .map_err(Into::into)
}

#[test]
fn read_only_assignment_commit_deadline_is_busy_and_retryable() -> fixture::TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            fixture::runtime()
                .map_err(fixture::debug)?
                .block_on(async {
                    let h =
                        fixture::Harness::create(admin, "semantic_commit_deadline", true).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result
                })
                .map_err(fixture::debug)
        })?
        .join()
        .map_err(|_| "semantic commit deadline worker panicked")?
        .map_err(Into::into)
}

async fn exercise(h: &fixture::Harness) -> fixture::TestResult {
    let scope = fixture::scope()?;
    let (read, stalled) =
        with_stalled_commit_response(h.root.read_runtime_scope_assignment(scope)).await;
    assert!(stalled, "assignment read must reach COMMIT");
    assert!(
        matches!(
            read,
            Err(AssignmentError::Unavailable(DurabilityError::RootUnavailable))
        ),
        "{read:?}"
    );
    h.root.await_ready().await?;
    let assignment = h
        .root
        .read_runtime_scope_assignment(scope)
        .await
        .map_err(fixture::debug)?
        .ok_or("missing assigned scope")?;
    assert_eq!(assignment.holder, Some(h.node.fact()));
    assert_eq!(assignment.ownership_generation, 1);

    let secret = BootstrapSecret::from_bytes([2; 32]);
    let launch = LaunchBinding::new("semantic-commit-deadline").map_err(fixture::debug)?;
    let (write, stalled) = with_stalled_commit_response(
        h.root.issue_node_bootstrap_authorization(&secret, &launch, None),
    )
    .await;
    assert!(stalled, "bootstrap authorization must reach COMMIT");
    assert!(
        matches!(
            write,
            Err(RegistrationError::Unavailable(DurabilityError::CommitOutcomeUnknown))
        ),
        "{write:?}"
    );
    h.root.await_ready().await?;
    h.root.issue_node_bootstrap_authorization(&secret, &launch, None)
        .await
        .map_err(fixture::debug)?;
    assert_eq!(h.count("game_node_bootstrap_authorizations").await?, 2);
    Ok(())
}
