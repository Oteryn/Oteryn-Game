//! A busy holder must not terminate a still-current consent refresh or ACK.
use crate::bestiary_postgres_harness::{Harness, TestResult, configured_admin, id, runtime};

#[test]
fn s17_cast_privacy_refresh_and_ack_wait_for_holder_without_extending_lease() -> TestResult {
    let Some(admin) = configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            runtime()
                .map_err(|error| error.to_string())?
                .block_on(holder_case(admin))
                .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "cast privacy holder thread panicked")?
        .map_err(Into::into)
}

async fn holder_case(admin: String) -> TestResult {
    use crate::durability::{DurabilityError, DurabilityRoot};
    use std::{future::Future, task::Poll};

    let h = Harness::create(admin, "s17_privacy_holder", true).await?;
    let result = async {
        let mut worker = h.root.register_cast_consent_worker(&h.node, id(97)).await?;
        let version = worker.version();
        let held = h.root.try_issue_semantic_pass()?;
        let mut refresh = Box::pin(DurabilityRoot::with_holder_owner(
            h.root.refresh_cast_consent_worker(&h.node, &mut worker),
        ));
        std::future::poll_fn(|cx| {
            assert!(
                refresh.as_mut().poll(cx).is_pending(),
                "refresh must wait for the held pass rather than fail immediately"
            );
            Poll::Ready(())
        })
        .await;
        drop(held);
        let batch = refresh.as_mut().await?;
        assert!(batch.is_empty());
        // End the mutable borrow owned by the completed, pinned future.
        drop(refresh);
        assert!(worker.valid());
        assert_eq!(worker.version(), version);

        let held = h.root.try_issue_semantic_pass()?;
        let mut ack = Box::pin(DurabilityRoot::with_holder_owner(
            h.root.acknowledge_cast_consent(&h.node, &mut worker, batch),
        ));
        std::future::poll_fn(|cx| {
            assert!(
                ack.as_mut().poll(cx).is_pending(),
                "ACK must retain its batch while the holder is busy"
            );
            Poll::Ready(())
        })
        .await;
        drop(held);
        ack.as_mut().await?;
        drop(ack);
        assert!(worker.valid());
        assert_eq!(worker.version(), version);

        let until = worker.valid_until();
        let held = h.root.try_issue_semantic_pass()?;
        let error = DurabilityRoot::with_holder_owner(
            h.root.refresh_cast_consent_worker(&h.node, &mut worker),
        )
        .await
        .err()
        .ok_or("an occupied holder must not renew beyond the original lease")?;
        assert!(matches!(
            error,
            DurabilityError::RootPassDeadlineExceeded | DurabilityError::RootUnavailable
        ));
        assert_eq!(worker.valid_until(), until);
        assert!(!worker.valid());
        assert_eq!(worker.version(), version);
        drop(held);
        h.root.close_cast_consent_worker(&h.node, worker).await?;
        Ok(())
    }
    .await;
    h.cleanup().await?;
    result
}
