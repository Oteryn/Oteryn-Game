//! The node's catalogue pass must wait for the committed October 31 save.
use crate::bestiary_postgres_harness as fixture;
use crate::durability::raid_runner::{CommittedRaidAction, RaidPlacementOutcome};
use crate::durability::raid_schedule::RaidWrite;
use crate::durability::runtime_scope_assignment::{
    AssignmentCommand, AssignmentOutcome, AssignmentRequest, ControlActor, OperationKey,
};
use crate::foundation::{ChannelId, ScopeOwnershipGeneration};
use oteryn_gameplay_encounter_rules::raid::RaidCatalogue;
use production_server::node::{RaidScheduleTick, check_raid_tick};

#[test]
fn w05d_hare_fires_once_after_house_bundle_reset_commits() -> fixture::TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            fixture::runtime()
                .map_err(fixture::debug)?
                .block_on(async {
                    let h = fixture::Harness::create_with_admission_seed(
                        admin,
                        "w05d_bundle_reset",
                        true,
                        false,
                    )
                    .await?;
                    let result = exercise(&h).await;
                    let cleanup = h.cleanup().await;
                    result?;
                    cleanup
                })
                .map_err(fixture::debug)
        })?
        .join()
        .map_err(|_| "Hare scheduler thread panicked")?
        .map_err(Into::into)
}

async fn exercise(h: &fixture::Harness) -> fixture::TestResult {
    let world = fixture::scope()?.world_id();
    let channel = ChannelId::decode(&fixture::id(fixture::CHANNEL)).map_err(fixture::debug)?;
    let generation = ScopeOwnershipGeneration::new(2).map_err(fixture::debug)?;
    let (bytes, pins) = oteryn_gameplay_world::map::boot::tests::bundle_with_house(
        "item:box",
        0,
        "item:coin",
        1,
        42,
    );
    let bundle_digest = pins.digest;
    let map = format!("sha256:{}", hex_digest(bundle_digest));
    let bundle = oteryn_gameplay_world::map::boot::check(
        bytes,
        oteryn_gameplay_world::map::boot::BootPins {
            bundle: pins,
            map_revision: map.clone(),
            start: oteryn_gameplay_world::map::overlay::TilePos {
                x: 1,
                y: 0,
                floor: -7,
            },
        },
    )?;
    let room =
        production_server::content::qualify_native_entry_room(world).map_err(fixture::debug)?;
    let key = "oteryn:raid.kazordoon_raids.the_halloween_hare";
    let catalogue: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../content/encounters/raids/catalogue.json"
    ))?;
    let hare = catalogue["records"]
        .as_array()
        .ok_or("catalogue records")?
        .iter()
        .find(|raid| raid["key"] == key)
        .ok_or("retained Hare record")?;
    let catalogue = RaidCatalogue::from_json(
        &serde_json::json!({
            "schema": catalogue["schema"],
            "records": [hare]
        })
        .to_string(),
    )?;
    let seal = h.recovery.seal_current().map_err(fixture::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(fixture::debug)?;
    let server = room.compiled().server_digest();
    let client = room.compiled().client_digest();
    let frame = room.frame_binding().digest();
    assert_ne!(server, bundle_digest);
    sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice())
        .execute(&h.pool).await?;
    assert!(
        h.root
            .record_content_activation(
                &crate::durability::content_activation::ContentActivationRequest {
                    world_id: world,
                    channel_id: channel,
                    activation_sequence: 1,
                    previous_sequence: None,
                    server_artifact_digest: server,
                    client_artifact_digest: client,
                    frame_binding_digest: frame,
                },
            )
            .await?
    );
    // Qualify a deterministic October save by replacing only the clock reads
    // in this disposable database. The actual reset owners write all epochs.
    sqlx::raw_sql(
        "DO $test$ DECLARE f regprocedure; instant text; body text; BEGIN
         FOR f, instant IN VALUES
           ('game_world_reset_initialize(uuid,bytea,text,bytea,bytea,bytea,text,numeric)'::regprocedure, '2026-10-30 06:00:00 Europe/Berlin'),
           ('game_world_reset_activate(uuid,numeric,bytea,bytea,text,bytea,bytea,bytea,uuid,numeric)'::regprocedure, '2026-10-31 06:00:05 Europe/Berlin')
         LOOP
           body := pg_get_functiondef(f);
           IF position('instant:=clock_timestamp();' IN body)=0 THEN RAISE EXCEPTION 'reset clock hook missing'; END IF;
           EXECUTE replace(body,'instant:=clock_timestamp();',format('instant:=%L::timestamptz;',instant));
         END LOOP;
         END $test$;",
    ).execute(&h.pool).await?;
    assert!(sqlx::query_scalar::<_, bool>("SELECT game_world_reset_initialize(encode($1,'hex')::uuid,$2,$3,$2,$4,$5,'TIBIA_SERVER_SAVE_BERLIN_06',1)")
        .bind(world.as_bytes().as_slice()).bind(server.as_slice()).bind(&map)
        .bind(client.as_slice()).bind(frame.as_slice()).fetch_one(&h.pool).await?);
    let mut house_tx = h.pool.begin().await?;
    sqlx::query("INSERT INTO game_house_properties(world_id,house_key,kind,state,rent_gold,content_digest)
                 VALUES(encode($1,'hex')::uuid,'oteryn:content.house.reset_fixture','private_house',1,0,$2)")
        .bind(world.as_bytes().as_slice()).bind(bundle_digest.as_slice())
        .execute(&mut *house_tx).await?;
    sqlx::query("INSERT INTO game_house_ownership(world_id,house_key,owner_character_id,ownership_revision,acquisition_receipt_ref)
                 VALUES(encode($1,'hex')::uuid,'oteryn:content.house.reset_fixture',NULL,1,game_character_uuid_v7())")
        .bind(world.as_bytes().as_slice()).execute(&mut *house_tx).await?;
    house_tx.commit().await?;

    let publish = |target_map: String| {
        sqlx::query_scalar::<_, bool>("SELECT game_world_reset_publish(encode($1,'hex')::uuid,0,$2,$3,$2,$4,$5,'[]'::jsonb,1)")
            .bind(world.as_bytes().as_slice().to_vec()).bind(server.to_vec()).bind(target_map)
            .bind(client.to_vec()).bind(frame.to_vec())
    };
    assert!(
        publish(format!("sha256:{}", hex_digest([99; 32])))
            .fetch_one(&h.pool)
            .await
            .is_err()
    );
    assert!(publish(map).fetch_one(&h.pool).await?);

    let committed_ms: i64 = sqlx::query_scalar(
        "SELECT (extract(epoch FROM TIMESTAMPTZ '2026-10-31 06:00:05 Europe/Berlin') * 1000)::bigint",
    ).fetch_one(&h.pool).await?;
    let mut tick = RaidScheduleTick {
        world,
        channel,
        generation,
        reset_epoch: 1,
        now_ms: committed_ms,
        bundle_digest,
    };
    assert!(
        check_raid_tick(&h.root, &authority, &h.node, &catalogue, &tick, None)
            .await??
            .firings
            .is_empty(),
        "an uncommitted reset cannot open the occurrence"
    );
    let predecessor = h
        .root
        .read_runtime_scope_predecessor(fixture::scope()?)
        .await
        .map_err(fixture::debug)?
        .ok_or("reset predecessor")?;
    assert!(matches!(
        h.writer
            .submit(&AssignmentRequest {
                operation_key: OperationKey::from_bytes([23; 32]),
                actor: ControlActor::new("oteryn_test_admin").map_err(fixture::debug)?,
                command: AssignmentCommand::Replace {
                    scope: fixture::scope()?,
                    predecessor,
                    target: h.node.fact(),
                },
            })
            .await
            .map_err(fixture::debug)?,
        AssignmentOutcome::Committed(_)
    ));
    let reset = h
        .root
        .retiring_world_reset(world)
        .await?
        .ok_or("published reset")?;
    let epoch = production_server::node::serve::finish_bundle_reset(
        &h.root, &authority, &h.node, &reset, &room, &bundle,
    )
    .await?
    .ok_or("committed bundle reset")?;
    assert_eq!(epoch, 1);
    assert_eq!(h.root.active_world_reset_epoch(world).await?, Some(epoch));
    assert_eq!(h.count("game_house_ownership").await?, 1);
    fixture::publish_readiness_for_scope(
        &h.pool,
        &h.root,
        &h.node,
        generation.get(),
        fixture::scope()?,
    )
    .await?;
    tick.reset_epoch = epoch;
    tick.now_ms = committed_ms - 1;
    assert!(
        check_raid_tick(&h.root, &authority, &h.node, &catalogue, &tick, None)
            .await??
            .firings
            .is_empty(),
        "a clock sample before activation cannot fire"
    );
    tick.now_ms = committed_ms;
    tick.bundle_digest = server;
    assert!(
        check_raid_tick(&h.root, &authority, &h.node, &catalogue, &tick, None)
            .await??
            .firings
            .is_empty(),
        "the native artifact digest cannot authorize a bundle raid",
    );
    tick.bundle_digest = bundle_digest;
    let outcome = check_raid_tick(&h.root, &authority, &h.node, &catalogue, &tick, None).await??;
    let [(firing, fired_key)] = outcome.firings.as_slice() else {
        return Err("the first post-commit tick must admit exactly one Hare firing".into());
    };
    assert_eq!(fired_key.as_str(), key);
    // With a zero-delay wave the deadline is already due. A retry before
    // the live Channel starts draining effects must leave the firing runnable.
    assert!(
        check_raid_tick(&h.root, &authority, &h.node, &catalogue, &tick, None)
            .await??
            .firings
            .is_empty()
    );
    assert_eq!(
        h.root.due_raid_firings(channel).await?,
        vec![(*firing, fired_key.clone())],
    );
    let selected = catalogue
        .get(fired_key)
        .ok_or("Hare")?
        .select(bundle_digest, *firing)?;
    let announcement = h
        .root
        .advance_raid_run(
            &h.node,
            *firing,
            channel,
            generation,
            selected.definition.clone(),
            committed_ms,
        )
        .await?
        .ok_or("immediate Hare announcement")?;
    assert_eq!(
        announcement.action,
        CommittedRaidAction::Announcement {
            text: selected.definition.announcements[0].text.clone(),
            advance: true,
        }
    );

    assert_eq!(
        h.root
            .acknowledge_raid_effect(&h.node, announcement.identity, vec![], committed_ms)
            .await?,
        RaidWrite::Applied,
    );
    let wave = h
        .root
        .advance_raid_run(
            &h.node,
            *firing,
            channel,
            generation,
            selected.definition.clone(),
            committed_ms,
        )
        .await?
        .ok_or("immediate Hare wave")?;
    let CommittedRaidAction::Wave { creatures, .. } = &wave.action else {
        return Err("expected Hare wave after announcement".into());
    };
    let [hare] = creatures.as_slice() else {
        return Err("expected one creature request".into());
    };
    assert_eq!(
        hare.creature,
        selected.definition.waves[0].entries[0].creature
    );
    // This scheduler case models the runner's placement acknowledgement. The
    // dispatch case exercises the real map and actor placement separately.
    assert_eq!(
        h.root
            .acknowledge_raid_effect(
                &h.node,
                wave.identity,
                vec![RaidPlacementOutcome {
                    identity: hare.identity,
                    placed: true
                }],
                committed_ms
            )
            .await?,
        RaidWrite::Applied,
    );
    let end = h
        .root
        .advance_raid_run(
            &h.node,
            *firing,
            channel,
            generation,
            selected.definition,
            committed_ms,
        )
        .await?
        .ok_or("completed Hare wave")?;
    assert_eq!(end.action, CommittedRaidAction::Complete { cleanup: false });
    assert_eq!(
        h.root
            .acknowledge_raid_effect(&h.node, end.identity, vec![], committed_ms)
            .await?,
        RaidWrite::Applied,
    );

    for ms in [committed_ms, committed_ms + 60_000] {
        tick.now_ms = ms;
        assert!(
            check_raid_tick(&h.root, &authority, &h.node, &catalogue, &tick, None)
                .await??
                .firings
                .is_empty()
        );
    }
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM game_world_raid_firings WHERE raid_key=$1")
            .bind(key)
            .fetch_one(&h.pool)
            .await?;
    assert_eq!(count, 1);
    eprintln!("POSTGRES_CASE_COMPLETED W05d node tick confirmed reset, announcement and replay");
    Ok(())
}

fn hex_digest(digest: [u8; 32]) -> String {
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}
