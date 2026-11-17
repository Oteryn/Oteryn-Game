//! SEASONWIN: the accepted producer API, real save activation, and durable buffs.
use crate::bestiary_postgres_harness::{self as f, Harness, TestResult, debug, id};
use crate::durability::DurabilityRoot;
use crate::durability::seasonwin::SeasonalEvent;
use crate::foundation::{ChannelId, WorldId};

async fn reset_epoch(
    h: &Harness,
    authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    room: &oteryn_gameplay_content::content::QualifiedNativeEntryRoom,
    world: WorldId,
    channel: ChannelId,
) -> TestResult {
    use crate::durability::runtime_scope_assignment::{
        AssignmentCommand, AssignmentOutcome, AssignmentRequest, ControlActor, OperationKey,
    };
    sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).execute(&h.pool).await?;
    assert!(
        h.root
            .record_content_activation(
                &crate::durability::content_activation::ContentActivationRequest {
                    world_id: world,
                    channel_id: channel,
                    activation_sequence: 1,
                    previous_sequence: None,
                    server_artifact_digest: room.compiled().server_digest(),
                    client_artifact_digest: room.compiled().client_digest(),
                    frame_binding_digest: room.frame_binding().digest(),
                }
            )
            .await?
    );
    assert!(sqlx::query_scalar::<_,bool>("SELECT game_world_reset_initialize(encode($1,'hex')::uuid,$2,$3,$2,$4,$5,'TIBIA_SERVER_SAVE_BERLIN_06',1)")
        .bind(world.as_bytes().as_slice()).bind(room.compiled().server_digest().as_slice()).bind(room.movement_cells().scope().map_revision.as_str())
        .bind(room.compiled().client_digest().as_slice()).bind(room.frame_binding().digest().as_slice()).fetch_one(&h.pool).await?);
    let scope = crate::foundation::RuntimeScopeRefV1::channel(world, channel);
    f::publish_readiness_for_scope(&h.pool, &h.root, &h.node, 1, scope).await?;
    assert!(sqlx::query_scalar::<_,bool>("SELECT game_world_reset_publish(encode($1,'hex')::uuid,0::numeric,$2,$3,$2,$4,$5,'[]'::jsonb,1)")
        .bind(world.as_bytes().as_slice()).bind(room.compiled().server_digest().as_slice()).bind(room.movement_cells().scope().map_revision.as_str())
        .bind(room.compiled().client_digest().as_slice()).bind(room.frame_binding().digest().as_slice()).fetch_one(&h.pool).await?);
    let reset = h
        .root
        .retiring_world_reset(world)
        .await?
        .ok_or("retiring reset")?;
    let predecessor = h
        .root
        .read_runtime_scope_predecessor(scope)
        .await
        .map_err(debug)?
        .ok_or("predecessor")?;
    assert!(matches!(
        h.writer
            .submit(&AssignmentRequest {
                operation_key: OperationKey::from_bytes([102; 32]),
                actor: ControlActor::new("oteryn_test_admin").map_err(debug)?,
                command: AssignmentCommand::Replace {
                    scope,
                    predecessor,
                    target: h.node.fact()
                },
            })
            .await
            .map_err(debug)?,
        AssignmentOutcome::Committed(_)
    ));
    for _ in 0..2 {
        assert_eq!(
            h.root
                .finish_world_reset(authority, &h.node, &reset, room, room)
                .await?,
            Some(1)
        );
    }
    Ok(())
}

#[test]
fn seasonwin_calendar_progress_buffs_restart_and_exactly_once_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .map_err(|e| e.to_string())?
                .block_on(Box::pin(exercise(admin)))
                .map_err(|e| e.to_string())
        })?
        .join()
        .map_err(|_| "seasonwin thread")?
        .map_err(Into::into)
}

async fn exercise(admin: String) -> TestResult {
    let started = std::time::Instant::now();
    for (event, open, before, last, expires, goal) in [
        (
            SeasonalEvent::AutumnVintage,
            "2026-10-01 06:00 Europe/Berlin",
            "2026-10-01 05:59:59 Europe/Berlin",
            "2026-10-09 05:59:59 Europe/Berlin",
            "2026-10-09 06:00 Europe/Berlin",
            4000,
        ),
        (
            SeasonalEvent::AutumnVintage,
            "2026-10-17 06:00 Europe/Berlin",
            "2026-10-17 05:59:59 Europe/Berlin",
            "2026-10-25 04:59:59 Europe/Berlin",
            "2026-10-25 05:00 Europe/Berlin",
            4000,
        ),
        (
            SeasonalEvent::DemonsLullaby,
            "2026-05-07 06:00 Europe/Berlin",
            "2026-05-07 05:59:59 Europe/Berlin",
            "2026-05-21 05:59:59 Europe/Berlin",
            "2026-05-21 06:00 Europe/Berlin",
            500,
        ),
    ] {
        let h =
            Harness::create_with_admission_seed(admin.clone(), "seasonwin", true, false).await?;
        let result = cases(&h, event, open, before, last, expires, goal).await;
        h.cleanup().await?;
        result?;
    }
    assert!(started.elapsed().as_secs_f64() > 0.01);
    eprintln!("SEASONWIN_PG_EXECUTED elapsed={:?}", started.elapsed());
    Ok(())
}

async fn clock(h: &Harness, at: &str) -> TestResult {
    // Only the business clock changes; reset publication and all runtime fences remain real.
    sqlx::query("CREATE TABLE IF NOT EXISTS seasonwin_test_clock(at timestamptz NOT NULL)")
        .execute(&h.pool)
        .await?;
    sqlx::query("TRUNCATE seasonwin_test_clock")
        .execute(&h.pool)
        .await?;
    sqlx::query("INSERT INTO seasonwin_test_clock VALUES($1::text::timestamptz)")
        .bind(at)
        .execute(&h.pool)
        .await?;
    sqlx::query("CREATE OR REPLACE FUNCTION game_seasonwin_now() RETURNS timestamptz LANGUAGE sql STABLE AS $$ SELECT at FROM seasonwin_test_clock $$").execute(&h.pool).await?;
    sqlx::query("GRANT SELECT ON seasonwin_test_clock TO oteryn_game_runtime")
        .execute(&h.pool)
        .await?;
    Ok(())
}

async fn cases(
    h: &Harness,
    event: SeasonalEvent,
    open: &str,
    before: &str,
    last: &str,
    expires: &str,
    goal: i64,
) -> TestResult {
    let world = WorldId::decode(&id(42)).map_err(debug)?;
    let channel = ChannelId::decode(&id(43)).map_err(debug)?;
    clock(h, open).await?;
    sqlx::query("CREATE FUNCTION seasonwin_reset_clock() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN NEW.activated_at=game_seasonwin_now(); NEW.reset_month=to_char((NEW.activated_at AT TIME ZONE 'Europe/Berlin')-interval '6 hours','YYYY-MM'); RETURN NEW; END $$").execute(&h.pool).await?;
    sqlx::query("CREATE TRIGGER seasonwin_reset_clock BEFORE INSERT ON game_world_reset_epochs FOR EACH ROW EXECUTE FUNCTION seasonwin_reset_clock()").execute(&h.pool).await?;
    let (_controller, pin) =
        oteryn_gameplay_content::content::native_gameplay::test_support::activated_with_loot_tables(
            world,
            Some(include_bytes!(
                "../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
            )),
        );
    let seal = h.recovery.seal_current().map_err(debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(debug)?;
    reset_epoch(h, &authority, pin.qualified_room(), world, channel).await?;
    h.root.maintain_ready_once().await?;
    clock(h, before).await?;
    assert!(
        h.root
            .record_progress(world, 1, event, 1, id(90))
            .await?
            .is_none()
    );
    clock(h, open).await?;
    assert!(
        h.root
            .record_progress(world, 2, event, 1, id(91))
            .await
            .is_err()
    );
    assert!(
        h.root
            .record_progress(world, 1, event, 0, id(91))
            .await
            .is_err()
    );
    h.root.maintain_ready_once().await?;
    let first = h
        .root
        .record_progress(world, 1, event, goal - 1, id(91))
        .await?
        .ok_or("inside window")?;
    assert_eq!(first.progress, goal - 1);
    assert!(!first.succeeded);
    let inactive = h.root.seasonal_buffs(world).await?;
    assert_eq!(inactive.potion_percent, 100);
    assert_eq!(inactive.experience_percent, 100);
    sqlx::query("UPDATE seasonwin_test_clock SET at=at+interval '1 day'")
        .execute(&h.pool)
        .await?;
    let root = DurabilityRoot::connect_test_runtime(&h.database.url)?;
    root.maintain_ready_once().await?;
    let first_root = h.root.clone();
    let pending =
        tokio::spawn(async move { first_root.record_progress(world, 1, event, 1, id(92)).await });
    let b = root.record_progress(world, 1, event, 1, id(92)).await;
    let a = pending.await?;
    let success = a?.ok_or("world success")?;
    assert_eq!(b?, Some(success.clone()));
    assert!(success.succeeded);
    assert_eq!(success.progress, goal);
    let expected_ms: i64 =
        sqlx::query_scalar("SELECT (extract(epoch FROM $1::text::timestamptz)*1000)::bigint")
            .bind(expires)
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(success.expires_ms, Some(expected_ms));
    assert!(
        h.root
            .record_progress(world, 1, event, 2, id(92))
            .await
            .is_err()
    );
    h.root.maintain_ready_once().await?;
    let buffs = root.seasonal_buffs(world).await?;
    match event {
        SeasonalEvent::AutumnVintage => {
            assert_eq!(buffs.scale_potion(300), 399);
            assert_eq!(buffs.scale_experience(300)?, 300);
            assert_eq!(
                crate::durability::seasonwin::autumn_status(Some(&success)),
                "As of now, 40 loads have been processed. Keep it up!"
            );
        }
        SeasonalEvent::DemonsLullaby => {
            assert_eq!(buffs.scale_experience(300)?, 450);
            assert_eq!(buffs.scale_potion(300), 300);
        }
    }
    drop(root);
    // A newly constructed owner recovers from PostgreSQL alone; there is no shared cache.
    let root = DurabilityRoot::connect_test_runtime(&h.database.url)?;
    root.maintain_ready_once().await?;
    assert_eq!(
        root.seasonal_progress(world, event).await?,
        Some(success.clone())
    );
    assert_eq!(root.seasonal_buffs(world).await?, buffs);
    assert_eq!(
        root.record_progress(world, 1, event, 10, id(93)).await?,
        Some(success.clone())
    );
    assert_eq!(
        root.record_progress(world, 1, event, goal - 1, id(91))
            .await?,
        Some(first)
    );
    sqlx::query(
        "UPDATE seasonwin_test_clock SET at=(SELECT closes_at FROM game_seasonwin_rounds LIMIT 1)",
    )
    .execute(&h.pool)
    .await?;
    assert!(
        root.record_progress(world, 1, event, 1, id(94))
            .await?
            .is_none()
    );
    assert_eq!(root.seasonal_buffs(world).await?, buffs);
    clock(h, last).await?;
    assert_eq!(root.seasonal_buffs(world).await?, buffs);
    clock(h, expires).await?;
    assert_eq!(root.seasonal_buffs(world).await?, inactive);
    assert!(
        root.record_progress(world, 1, event, 1, id(94))
            .await?
            .is_none()
    );
    assert_eq!(root.seasonal_progress(world, event).await?, Some(success));
    let receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_seasonwin_receipts")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(receipts, 3);
    assert_eq!(
        crate::durability::seasonwin::autumn_status(None),
        "As of now, no berries have been processed..."
    );
    Ok(())
}
