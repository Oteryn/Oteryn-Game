//! SEAM-2: required join observations retry contention without inventing optional state.
use super::*;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;

#[test]
fn seam_join_tracker_snapshot_retries_busy_holder_without_progression() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            run(async |admin| {
                let h =
                    Harness::create_with_admission_seed(admin, "seam_join_snapshot", false, false)
                        .await?;
                prepare_current_session(&h).await?;
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h
                    .root
                    .open_character_authority(&seal)
                    .await
                    .map_err(debug)?;
                let deps = Dependencies::new()?;
                let owner = deps.owner(&h, &authority);
                let mut admitted = deps.admitted;
                admitted.item_fence = crate::gameplay_transport::item_fence_of(
                    FreshAdmissionStore::from_root(h.root.clone())
                        .current_session(admitted.game_session_id)
                        .await?,
                );
                let expected = owner
                    .cyclopedia_tracker_snapshot(admitted)
                    .await
                    .ok_or("initial empty tracker snapshot")?;
                let observed = {
                    let held = h.root.try_issue_semantic_pass()?;
                    let mut snapshot = std::pin::pin!(owner.cyclopedia_tracker_snapshot(admitted));
                    std::future::poll_fn(|cx| {
                        use std::future::Future;
                        assert!(
                            snapshot.as_mut().poll(cx).is_pending(),
                            "busy holder must not become a missing required snapshot"
                        );
                        std::task::Poll::Ready(())
                    })
                    .await;
                    held.run(|_, _| Box::pin(async { Ok(()) })).await?;
                    snapshot.await
                };
                assert_eq!(observed, Some(expected));
                drop(owner);
                drop(seal);
                h.cleanup().await
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "join snapshot test thread panicked")??;
    Ok(())
}
