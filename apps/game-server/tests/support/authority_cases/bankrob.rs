//! The World boot clock, selected city and Channel claim survive owner replay.
use crate::bestiary_postgres_harness as fixture;
use crate::durability::runtime_scope_assignment::{
    AssignmentCommand, AssignmentOutcome, AssignmentRequest, ControlActor, OperationKey,
};
use crate::durability::world_event::bank_robbery;
use crate::foundation::{ChannelId, RuntimeScopeRefV1, ScopeOwnershipGeneration};

#[test]
fn bankrob_hourly_world_occurrence() -> fixture::TestResult {
    let Some(admin) = fixture::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            fixture::runtime()
                .map_err(fixture::debug)?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let h = fixture::Harness::create(admin, "bankrob_hourly", false).await?;
                    eprintln!("bankrob-marker: bankrob_hourly_world_occurrence db ready");
                    let result = hourly(&h).await;
                    h.cleanup().await?;
                    eprintln!(
                        "bankrob_hourly_world_occurrence elapsed={:?}",
                        started.elapsed()
                    );
                    result
                })
                .map_err(fixture::debug)
        })?
        .join()
        .map_err(|_| "bank robbery test thread failed")??;
    Ok(())
}

async fn hourly(h: &fixture::Harness) -> fixture::TestResult {
    let scope = fixture::scope()?;
    let world = scope.world_id();
    sqlx::query("INSERT INTO game_world_reset_epochs(world_id,reset_epoch,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,activated_at,reset_month) VALUES(encode($1,'hex')::uuid,1,$2,'map-1',$2,$2,$2,'TIBIA_SERVER_SAVE_BERLIN_06',1,clock_timestamp(),to_char(clock_timestamp() AT TIME ZONE 'Europe/Berlin','YYYY-MM'))")
        .bind(world.as_bytes().as_slice()).bind([7u8;32].as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_world_reset_active VALUES(encode($1,'hex')::uuid,1)")
        .bind(world.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
    assert!(h.root.world_changes(world, 1).await?.is_some());
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let generation = ScopeOwnershipGeneration::new(1).map_err(fixture::debug)?;
    let sites = [true; 4];
    assert!(
        bank_robbery::with_test_roll(
            "thais",
            h.root
                .poll_bank_robbery(&authority, &h.node, scope, generation, 1, sites)
        )
        .await?
        .is_none()
    );
    sqlx::query(
        "UPDATE game_bank_robbery_occurrences SET boot_ms=boot_ms-3600000 WHERE reset_epoch=1",
    )
    .execute(&h.pool)
    .await?;
    assert!(bank_robbery::with_test_roll(
        "thais",
        h.root.poll_bank_robbery(&authority, &h.node, scope, generation, 1, [false; 4]),
    ).await?.is_none());
    let pending: (i16, i64) = sqlx::query_as(
        "SELECT state,last_checked_hour FROM game_bank_robbery_occurrences WHERE reset_epoch=1",
    ).fetch_one(&h.pool).await?;
    assert_eq!(pending, (0, 0));
    sqlx::query(
        "UPDATE game_bank_robbery_occurrences SET boot_ms=boot_ms-3600000 WHERE reset_epoch=1",
    )
    .execute(&h.pool)
    .await?;
    assert!(
        bank_robbery::with_test_roll(
            "thais",
            h.root.poll_bank_robbery(&authority, &h.node, scope, generation, 1, [false; 4]),
        )
        .await?
        .is_none()
    );
    let pending: (i16, i64) = sqlx::query_as(
        "SELECT state,last_checked_hour FROM game_bank_robbery_occurrences WHERE reset_epoch=1",
    )
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(pending, (0, 1));
    let first = bank_robbery::with_test_roll(
        "thais",
        h.root
            .poll_bank_robbery(&authority, &h.node, scope, generation, 1, sites),
    )
    .await?
    .ok_or("hourly Thais occurrence")?;
    assert_eq!(first.city.key, "thais");
    assert!(h.root.bank_robbery_announcement(&authority, &h.node, scope, generation, 1).await?);

    let second = ChannelId::decode(&fixture::id(84)).map_err(fixture::debug)?;
    let other = RuntimeScopeRefV1::channel(world, second);
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,1)")
        .bind(world.as_bytes().as_slice()).bind(second.as_bytes().as_slice()).execute(&h.pool).await?;
    assert!(matches!(
        h.writer
            .submit(&AssignmentRequest {
                operation_key: OperationKey::from_bytes([84; 32]),
                actor: ControlActor::new("oteryn_test_admin").map_err(fixture::debug)?,
                command: AssignmentCommand::Assign {
                    scope: other,
                    target: h.node.fact()
                },
            })
            .await
            .map_err(fixture::debug)?,
        AssignmentOutcome::Committed(_)
    ));
    fixture::publish_readiness_for_scope(&h.pool, &h.root, &h.node, 1, other).await?;
    assert!(
        bank_robbery::with_test_roll(
            "carlin",
            h.root
                .poll_bank_robbery(&authority, &h.node, other, generation, 1, sites)
        )
        .await?
        .is_none()
    );

    let replacement = fixture::register(&h.root, 85).await?;
    let predecessor = h
        .root
        .read_runtime_scope_predecessor(scope)
        .await
        .map_err(fixture::debug)?
        .ok_or("predecessor")?;
    assert!(matches!(
        h.writer
            .submit(&AssignmentRequest {
                operation_key: OperationKey::from_bytes([85; 32]),
                actor: ControlActor::new("oteryn_test_admin").map_err(fixture::debug)?,
                command: AssignmentCommand::Replace {
                    scope,
                    predecessor,
                    target: replacement.fact()
                },
            })
            .await
            .map_err(fixture::debug)?,
        AssignmentOutcome::Committed(_)
    ));
    let successor = ScopeOwnershipGeneration::new(2).map_err(fixture::debug)?;
    fixture::publish_readiness_for_scope(&h.pool, &h.root, &replacement, 2, scope).await?;
    let restarted = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let replay = bank_robbery::with_test_roll(
        "carlin",
        h.root
            .poll_bank_robbery(&restarted, &replacement, scope, successor, 1, sites),
    )
    .await?
    .ok_or("replayed occurrence")?;
    assert_eq!(replay.firing_id, first.firing_id);
    assert_eq!(replay.city.key, "thais");
    let last_hour: i64 = sqlx::query_scalar("SELECT last_checked_hour FROM game_bank_robbery_occurrences WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=1")
        .bind(world.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(last_hour, 2);
    Ok(())
}
