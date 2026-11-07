//! Mounted once by the real dispatch owner, reusing its existing admission fixture.
use super::planned_world_reset::reset_fixture;
use super::*;
use crate::foundation::GameSessionState;

#[test]
fn world_reset_retired_holder_release_preserves_live_holders_and_original_grace() -> TestResult {
    std::thread::Builder::new()
        .name("retired-holder-reset".into())
        .stack_size(64 << 20)
        .spawn(|| {
            run(async |admin| {
                for lost in [false, true] {
                    retired_session(admin.clone(), lost).await?;
                }
                Ok(())
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "retired-holder reset scenario panicked")?
        .map_err(Into::into)
}

async fn retired_session(admin: String, lost: bool) -> TestResult {
    reset();
    let harness =
        Harness::create_with_admission_seed(admin, "reset_retired_holder", true, false).await?;
    let (database, version): (String, String) =
        sqlx::query_as("SELECT current_database(),current_setting('server_version_num')")
            .fetch_one(&harness.pool)
            .await?;
    eprintln!("WORLD-RESET-RETIRED-HOLDER-PG: {database} PostgreSQL={version} lost={lost}");
    prepare_current_session(&harness).await?;
    let seal = harness.recovery.seal_current().map_err(debug)?;
    let authority = harness
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    let dependencies = Dependencies::new()?;
    let owner = dependencies.owner(&harness, &authority);
    dependencies.initialize(&owner).await;
    owner
        .runtime
        .lock()
        .await
        .initialize_first_entry_position(dependencies.actor)
        .map_err(debug)?;
    track(&owner, &dependencies)?;
    let world = owner.world_id;
    let channel = owner.channel_id;
    reset_fixture::seed_retiring(&harness.pool, world).await?;
    let retiring = harness
        .root
        .retiring_world_reset(world)
        .await?
        .ok_or("sealed reset")?;
    if lost {
        use crate::gameplay_transport::connection::FreshAdmissionAuthority;
        assert!(matches!(
            owner
                .lose_control(dependencies.admitted, Duration::ZERO, Some(&retiring))
                .await,
            crate::gameplay_transport::connection::ControlLossResult::Recorded
        ));
    }
    // Two current registrations exist. A fresh assignment alone cannot release
    // the original holder's session while that holder is still registered.
    let node = fixture::registered_node(&harness.root, 2).await?;
    reset_fixture::replace(&harness, world, channel, &node).await?;
    assert!(
        !harness
            .root
            .retire_world_reset_ground(&authority, &node, &retiring)
            .await
            .map_err(debug)?
    );
    let store = FreshAdmissionStore::from_root(harness.root.clone());
    let session = dependencies.admitted.game_session_id;
    let before = store.current_session(session).await?;
    assert_eq!(
        before.session_state(),
        if lost {
            GameSessionState::Reconnectable
        } else {
            GameSessionState::Active
        }
    );
    harness
        .root
        .revoke_node_registration(harness.node.fact())
        .await
        .map_err(debug)?;
    if lost {
        // The holder has ended, but the existing grace must expire on the
        // database clock; boot cannot shorten it or synthesize another epoch.
        assert!(
            !harness
                .root
                .retire_world_reset_ground(&authority, &node, &retiring)
                .await
                .map_err(debug)?
        );
        let (current, now) = store.current_session_at(session).await?;
        assert_eq!(
            current.current_control_loss_epoch(),
            before.current_control_loss_epoch()
        );
        let deadline = current.current_original_grace_deadline().ok_or("grace")?;
        let remaining = u64::try_from(deadline.saturating_sub(now))?;
        tokio::time::sleep(Duration::from_secs(remaining + 1)).await;
    }
    drop(owner);
    assert!(
        harness
            .root
            .retire_world_reset_ground(&authority, &node, &retiring)
            .await
            .map_err(debug)?
    );
    let terminal = store.current_session(session).await?;
    assert_eq!(terminal.session_state(), GameSessionState::Terminal);
    assert_eq!(
        terminal.current_control_loss_epoch(),
        before.current_control_loss_epoch()
    );
    let receipts: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_durability_admission_lifecycle_receipts")
            .fetch_one(&harness.pool)
            .await?;
    assert!(
        harness
            .root
            .retire_world_reset_ground(&authority, &node, &retiring)
            .await
            .map_err(debug)?
    );
    let replay_receipts: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_durability_admission_lifecycle_receipts")
            .fetch_one(&harness.pool)
            .await?;
    assert_eq!(receipts, replay_receipts);
    drop(authority);
    drop(seal);
    harness.cleanup().await
}
