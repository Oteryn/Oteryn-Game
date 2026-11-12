//! Staff enrichment failures must not disconnect ordinary admitted players.
use super::*;

#[test]
fn staff_refresh_without_gameplay_fence_keeps_non_staff_session_connected() -> TestResult {
    run_case(false)
}

#[test]
fn staff_refresh_status_error_keeps_non_staff_session_connected() -> TestResult {
    run_case(true)
}

fn run_case(status_error: bool) -> TestResult {
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || scenario(status_error).map_err(|e| e.to_string()))?
        .join()
        .map_err(|_| "staff refresh PostgreSQL thread panicked")??;
    Ok(())
}

fn scenario(status_error: bool) -> TestResult {
    run(async |admin| {
        let h = Harness::create_with_admission_seed(admin, "staff_refresh", true, false).await?;
        eprintln!("POSTGRES_STARTED staff_refresh status_error={status_error}");
        if status_error {
            prepare_current_session(&h).await?;
        }
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let deps = Dependencies::new()?;
        let owner = deps.owner(&h, &authority);
        deps.initialize(&owner).await;
        owner
            .runtime
            .lock()
            .await
            .initialize_first_entry_position(deps.actor)
            .map_err(debug)?;
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        admitted.continuity.selected_capabilities = SelectedCapabilities::NONE;
        if status_error {
            let fence = crate::gameplay_transport::character_gameplay_fence::current(
                &owner,
                admitted.game_session_id,
                false,
            )
            .await
            .ok_or("current gameplay fence")?;
            assert_eq!(
                owner
                    .root
                    .staff_live_status(&authority, owner.holder, fence.item_fence())
                    .await?
                    .group,
                0
            );
            // Break only the enrichment query in this isolated database. Session
            // and Character reads still succeed, so refresh reaches the failed read.
            sqlx::query("ALTER TABLE game_staff_bans RENAME COLUMN starts_at_ms TO unavailable_starts_at_ms")
                .execute(&h.pool).await?;
            assert!(
                crate::gameplay_transport::character_gameplay_fence::current(
                    &owner,
                    admitted.game_session_id,
                    false,
                )
                .await
                .is_some()
            );
            assert!(
                owner
                    .root
                    .staff_live_status(&authority, owner.holder, fence.item_fence())
                    .await
                    .is_err()
            );
            super::super::maintain_ready(&h.root).await?;
        } else {
            assert!(
                crate::gameplay_transport::character_gameplay_fence::current(
                    &owner,
                    admitted.game_session_id,
                    false,
                )
                .await
                .is_none()
            );
        }
        // A wire command result proves serve_admitted passed refresh and
        // dispatched input rather than closing the connection early.
        let reply = exchange_raw(&owner, admitted, u32::MAX, &[]).await?;
        assert!(
            reply.is_some(),
            "staff enrichment failure disconnected an ordinary player"
        );
        assert_eq!(owner.states.staff_world.group(deps.actor), 0);
        assert!(!owner.states.staff_world.invisible(deps.actor));
        if status_error {
            sqlx::query("ALTER TABLE game_staff_bans RENAME COLUMN unavailable_starts_at_ms TO starts_at_ms")
                .execute(&h.pool).await?;
            super::super::maintain_ready(&h.root).await?;
            assert!(
                owner.staff_refresh(admitted).await,
                "next refresh retries successfully"
            );
            let mut stale = admitted;
            stale.continuity.connection_generation += 1;
            assert!(
                !owner.staff_refresh(stale).await,
                "definite stale connection remains rejected"
            );
        }
        owner
            .runtime
            .lock()
            .await
            .record_control_loss(
                deps.actor,
                admitted.game_session_id,
                crate::foundation::ControlLossMark {
                    epoch: 1,
                    grace_deadline: 100,
                },
            )
            .map_err(debug)?;
        assert!(
            !owner.staff_refresh(admitted).await,
            "lost control remains rejected even without enrichment"
        );
        drop(owner);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}
