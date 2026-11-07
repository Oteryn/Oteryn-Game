//! Real reset finalization -> admitted command bytes -> receipt -> result -> restart.
use super::*;
use crate::foundation::{
    ChannelId, MessageType, ScopeOwnershipGeneration, WorldId, decode_wire_envelope,
};
use crate::gameplay_transport::capabilities::SelectedCapabilities;
use crate::gameplay_transport::connection::{
    ConnectionEnd, FreshAdmissionAuthority, IDLE_LIVENESS, serve_admitted,
};
use oteryn_protocol_oteryn::world_event as wire;
use oteryn_protocol_oteryn::{
    ClientCommandValue, CommandStatus, decode_command_result, encode_client_command,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

#[path = "world_event/reset_fixture.rs"]
mod event_fixture;

#[test]
fn world_event_postgres_real_dispatch_reset_restart_and_fences() -> TestResult {
    let started = std::time::Instant::now();
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| {
            for epochs in [1, 2] {
                scenario(epochs).map_err(|e| e.to_string())?;
            }
            Ok::<_, String>(())
        })?
        .join()
        .map_err(|_| "world event e2e panicked")??;
    eprintln!(
        "T94_DISPATCH_PG_EXECUTED reset/dispatch/restart elapsed_ms={}",
        started.elapsed().as_millis()
    );
    Ok(())
}

pub(super) fn scenario(epochs: u64) -> TestResult {
    scenario_with_consumers(epochs, false)
}

pub(super) fn scenario_with_consumers(epochs: u64, consumers: bool) -> TestResult {
    let _clock = oteryn_gameplay_foundation::scheduled_events::test_clock::set(
        oteryn_gameplay_foundation::scheduled_events::content_schedule().clone(),
        1_793_422_800,
    );
    let started = std::time::Instant::now();
    run(async move |admin| {
        reset();
        let h =
            Harness::create_with_admission_seed(admin, "world_event_dispatch", !consumers, false).await?;
        let world = WorldId::decode(&id(42)).map_err(debug)?;
        let channel = ChannelId::decode(&id(43)).map_err(debug)?;
        assert_eq!(h.root.active_world_event_epoch(world).await?, None);
        let (controller, pin) =
            crate::content::native_gameplay::test_support::activated_with_loot_tables(
                world,
                Some(include_bytes!(
                    "../../../../../../tools/content-schema/native-gameplay/canary-thalom-world.json"
                )),
            );
        let room = pin.qualified_room();
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        h.root
            .t94_initialize(&authority, "oteryn:game:primary", &mut T94Bootstrap)
            .await?;
        h.root
            .t94_start_world(
                &authority,
                world,
                &Default::default(),
                "oteryn:game:primary",
            )
            .await?;
        event_fixture::reset_epochs(&h, &authority, room, world, channel, epochs).await?;
        // The reset decision owns the calendar across every Channel and restart.
        let calendar = h.root.world_change_calendar(world, epochs).await?;
        assert_eq!(h.root.world_change_calendar(world, epochs).await?, calendar);
        assert!(h.root.world_change_calendar(world, epochs + 1).await.is_err());
        let changes = oteryn_gameplay_world_change::daily::roll_calendar(world.as_bytes(), epochs, calendar.0, calendar.1);
        for (key, expected) in [("grimvale", changes.grimvale_full_moon), ("their_masters_voice", changes.their_masters_voice), ("fury_gates", changes.fury_gates.is_some())] {
            assert_eq!(h.root.world_change_active(world, key).await?, expected);
        }
        for _ in 0..16 {
            h.root
                .t94_start_world(
                    &authority,
                    world,
                    &Default::default(),
                    "oteryn:game:primary",
                )
                .await?;
            h.root.t94_produce(&authority).await?;
        }
        let source_count:i64=sqlx::query_scalar("SELECT count(*) FROM game_t94_native_receipts WHERE kind IN('WORLD_CHANGE','DAILY_WORLD_GATE')").fetch_one(&h.pool).await?;
        assert_eq!(source_count, 2 * i64::try_from(epochs)?);
        let publication = h
            .root
            .t94_next(&authority, "world_event_outcome")
            .await?
            .ok_or("T94 native reset publication")?;
        let snapshot = crate::native_admission_source::world_event_outcomes::Snapshot::decode(
            &publication.wire,
        )?;
        let crate::native_admission_source::world_event_outcomes::Snapshot::Event(snapshot) =
            snapshot
        else {
            return Err("T94 family".into());
        };
        assert!(matches!(
            snapshot.event[0].state.as_str(),
            "VOID" | "CLOSED"
        ));
        assert!(snapshot.event[0].outcome.is_empty());
        assert!(!publication.audit.is_empty());
        event_fixture::admit(&h, epochs + 1).await?;
        let mut deps = Dependencies::new()?;
        deps.room = room.clone();
        if consumers { deps.combat_room = Some(room.clone()); }

        planned_world_reset::reset_fixture::set_runtime_with_capacity(
            &mut deps,
            world,
            channel,
            &pin,
            epochs + 1,
            if consumers { 3 } else { 2 },
        )?;
        let channel_pin = deps.runtime.lock().await.content_pin().clone();
        let actor = {
            let mut runtime = deps.runtime.lock().await;
            let reservation = runtime
                .reserve_fresh_session(deps.admitted.game_session_id)
                .map_err(debug)?;
            runtime.commit_fresh_session(reservation).map_err(debug)?
        };
        deps.actor = actor;
        deps.admitted.runtime_actor = Some(actor);
        deps.admitted.first_entry = FirstEntryOutcome::Positioned;
        let mut item_fence = fence(1)?.item_fence();
        item_fence.character_lease_generation = 2;
        item_fence.scope_ownership_generation =
            ScopeOwnershipGeneration::new(epochs + 1).map_err(debug)?;
        deps.admitted.item_fence = Some(item_fence);
        if consumers {
            let mut runtime = deps.runtime.lock().await;
            runtime.initialize_pinned_test_position(actor, crate::foundation::MovementLocalPosition { x: 2, y: 11, floor: -7 }).map_err(debug)?;
            runtime.bind_attacker_lease(actor, deps.admitted.game_session_id,
                crate::foundation::CharacterLease::new(crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?, 2).map_err(debug)?).map_err(debug)?;
        }

        let mut admitted = deps.admitted;
        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = controller.active();
        // Announcement queries use the shared owner's closed room gates.
        owner.reset_epoch = h
            .root
            .active_world_event_epoch(world)
            .await?
            .ok_or("active epoch")?;
        deps.initialize(&owner).await;
        if consumers {
            super::qworld::consumers::qualify(&h, &mut owner, &mut admitted, actor, channel_pin, room, epochs).await?;
        }

        owner
            .runtime
            .lock()
            .await
            .initialize_first_entry_position(actor)
            .map_err(debug)?;
        let offered = owner.offered_capabilities();
        assert!(
            offered
                .iter()
                .any(|c| c.id == wire::CAPABILITY_WORLD_EVENT_V1)
        );
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(offered, &[6, 53, 371])
                .ok_or("seasonal capability 371")?;
        let fresh = admitted;

        // A completed, source-valid February occurrence remains observable after cleanup.
        // The existing database-domain test clock matches the calendar observation here.
        sqlx::raw_sql("CREATE OR REPLACE FUNCTION game_daily_effect_now() RETURNS timestamptz LANGUAGE sql VOLATILE AS $$ SELECT to_timestamp(1793422800) $$")
            .execute(&h.pool).await?;
        let mut cake_seed = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role=replica")
            .execute(&mut *cake_seed)
            .await?;
        sqlx::query("INSERT INTO game_seasonal_cake_worlds(world_id,event_start,event_end,state) VALUES(encode($1,'hex')::uuid,1771650000,1772168400,$2)")
            .bind(world.as_bytes().as_slice())
            .bind(serde_json::json!({"stage":0,"substances":200,"golems":200,"layer":4,"layer_progress":0,
                "window_start":null,"window_end":null,"bonus_from":1771736400_i64,"bonus_until":1772341200_i64,"revision":12}))
            .execute(&mut *cake_seed).await?;
        cake_seed.commit().await?;

        // Malformed query never reaches storage. A valid query commits its exact result.
        let results = exchange(&owner, &mut admitted, &[(1, &[0]), (2, &[])]).await?;
        assert_eq!(results.len(), 2);
        assert_eq!(results[0], (CommandStatus::Rejected, Vec::new()));
        assert_eq!(results[1].0, CommandStatus::Accepted);
        let state = wire::decode_world_event_state(&results[1].1).map_err(debug)?;
        assert_eq!(state.reset_epoch, epochs);
        let expected_active = epochs % 2 == 1 || changes.their_masters_voice || changes.grimvale_full_moon || changes.fury_gates.is_some();
        assert_eq!(state.active, expected_active);
        assert_eq!(state.announcement.is_empty(), !expected_active);
        for (name, active) in [("Their Master's Voice", changes.their_masters_voice), ("Grimvale", changes.grimvale_full_moon), ("Fury Gates", changes.fury_gates.is_some())] {
            assert_eq!(state.announcement.contains(name), active, "wire world-state decision for {name}");
        }
        assert_eq!(
            state.seasonal.len(),
            3,
            "SRV7: committed annual gameplay windows reach the wire"
        );
        assert!(
            state
                .seasonal
                .iter()
                .any(|event| event.event == wire::SeasonalEventKind::Halloween && event.active)
        );
        for event in &state.seasonal {
            assert!(event.start_unix < event.end_unix);
        }
        assert_eq!(
            state.cake,
            Some(wire::CakeProgress {
                stage: 0,
                substances: 200,
                golems: 200,
                layer: 4,
                layer_progress: 0,
                layer_goal: 1116,
                occurrence_start: 1771650000,
                occurrence_end: 1772168400,
                window_end: None,
                bonus_until: Some(1772341200),
                observed_unix: 1793422800,
            }),
            "cap371 carries the retained Cake timeline even after cleanup and bonus expiry"
        );

        let stored: Vec<u8> = sqlx::query_scalar(
            "SELECT result FROM game_world_event_query_receipts WHERE command_id=2",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(stored, results[1].1);
        assert_eq!(receipt_count(&h).await?, 1);

        // Capability 53 predates strict field 4. The same durable receipt is
        // projected to its original schema for a peer without extension 371.
        let mut legacy_owner = deps.owner(&h, &authority);
        legacy_owner.active_generation = controller.active();
        legacy_owner.reset_epoch = epochs;
        let mut legacy = fresh;
        legacy.continuity.selected_capabilities =
            SelectedCapabilities::select(offered, &[6, 53]).ok_or("legacy capability 53")?;
        let legacy_reply = exchange(&legacy_owner, &mut legacy, &[(1, &[0]), (2, &[])]).await?;
        assert_eq!(legacy_reply[1].0, CommandStatus::Accepted);
        let legacy_state = wire::decode_world_event_state(&legacy_reply[1].1).map_err(debug)?;
        assert!(legacy_state.seasonal.is_empty());
        assert!(
            legacy_state.cake.is_none(),
            "legacy cap53 projection removes strict field5"
        );
        assert_eq!(legacy_state.reset_epoch, state.reset_epoch);
        assert_eq!(legacy_state.announcement, state.announcement);
        assert_eq!(receipt_count(&h).await?, 1);
        drop(legacy_owner);

        // No negotiated capability => no event bytes or receipt, even for a valid query.
        let mut unsupported = admitted;
        unsupported.continuity.selected_capabilities =
            SelectedCapabilities::select(offered, &[6]).ok_or("base capability")?;
        assert_eq!(
            exchange(&owner, &mut unsupported, &[(3, &[])]).await?,
            [(CommandStatus::Rejected, Vec::new())]
        );
        assert_eq!(receipt_count(&h).await?, 1);

        // Fresh DB reads after reconstructing the runtime owner preserve exact bytes.
        drop(owner);
        let mut restarted = deps.owner(&h, &authority);
        restarted.active_generation = controller.active();
        restarted.reset_epoch = h
            .root
            .active_world_event_epoch(world)
            .await?
            .ok_or("restart epoch")?;
        // The reconstructed fixture owner has no retained ingress. Start with fresh
        // admission continuity and reach the durable receipt IDs in stream order.
        admitted = fresh;
        assert_eq!(
            exchange(&restarted, &mut admitted, &[(1, &[0])]).await?,
            [(CommandStatus::Rejected, Vec::new())]
        );
        let replay = exchange(&restarted, &mut admitted, &[(2, &[]), (3, &[])]).await?;
        assert_eq!(
            replay,
            [
                (CommandStatus::Accepted, stored.clone()),
                (CommandStatus::Accepted, stored.clone())
            ]
        );
        assert_eq!(receipt_count(&h).await?, 2);
        // Same-session retries replay retained terminal truth (SESSION-INGRESS-1).
        assert_eq!(
            exchange(&restarted, &mut admitted, &[(3, &[])]).await?,
            [(CommandStatus::DuplicateReplay, stored)]
        );
        assert_eq!(receipt_count(&h).await?, 2);

        // Independently stale connection/lease/scope, wrong identity and missing fence.
        // Start at CommandId 1 so generic outstanding-Bank reconciliation does
        // not consume the stale fence before this mechanic's dispatch boundary.
        for which in 0..7 {
            // Each independent negative starts a fresh fixture ingress at ID 1.
            let mut negative = deps.owner(&h, &authority);
            negative.active_generation = controller.active();
            negative.reset_epoch = epochs;
            let mut forged = fresh;
            let f = forged.item_fence.as_mut().ok_or("fence")?;
            match which {
                0 => {
                    f.connection_generation =
                        crate::foundation::ConnectionGeneration::new(2).map_err(debug)?
                }
                1 => f.character_lease_generation += 1,
                2 => {
                    f.scope_ownership_generation =
                        ScopeOwnershipGeneration::new(epochs + 2).map_err(debug)?
                }
                3 => {
                    f.character_id =
                        crate::domain::CharacterId::from_bytes(id(99)).map_err(debug)?
                }
                4 => {
                    f.runtime_scope = crate::foundation::RuntimeScopeRefV1::channel(
                        WorldId::decode(&id(99)).map_err(debug)?,
                        channel,
                    )
                }
                5 => {
                    f.runtime_scope = crate::foundation::RuntimeScopeRefV1::channel(
                        world,
                        ChannelId::decode(&id(99)).map_err(debug)?,
                    )
                }
                _ => forged.item_fence = None,
            }
            assert_eq!(
                exchange(&negative, &mut forged, &[(1, &[])]).await?,
                [(CommandStatus::Rejected, Vec::new())],
                "negative fence {which} at epoch {epochs}"
            );
        }
        // Stale boot epoch and zero CommandId fail closed at their owning boundaries.
        let mut stale_boot = deps.owner(&h, &authority);
        stale_boot.active_generation = controller.active();
        stale_boot.reset_epoch = epochs + 1;
        let mut stale_boot_session = fresh;
        assert_eq!(
            exchange(&stale_boot, &mut stale_boot_session, &[(1, &[])]).await?,
            [(CommandStatus::Rejected, Vec::new())]
        );
        assert!(
            h.root
                .query_world_event(&authority, &h.node, item_fence, 0, epochs)
                .await
                .is_err()
        );
        assert_eq!(receipt_count(&h).await?, 2);
        // Receipt lookup on a replay still requires the current session generation.
        let mut stale = item_fence;
        stale.character_lease_generation += 1;
        assert!(
            h.root
                .query_world_event(&authority, &h.node, stale, 2, epochs)
                .await
                .is_err()
        );
        // A failed query commit emits no terminal result and leaves its original
        // ingress reservation unresolved; a same-session retry must fail closed.
        sqlx::query("CREATE FUNCTION reject_event_query() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'injected query failure' USING ERRCODE='23514'; END $$").execute(&h.pool).await?;
        sqlx::query("CREATE TRIGGER reject_event_query BEFORE INSERT ON game_world_event_query_receipts FOR EACH ROW EXECUTE FUNCTION reject_event_query()").execute(&h.pool).await?;
        assert!(
            exchange(&restarted, &mut admitted, &[(4, &[])])
                .await?
                .is_empty()
        );
        assert_eq!(admitted.continuity.next_command_id, 5);
        assert_eq!(receipt_count(&h).await?, 2);
        sqlx::query("DROP TRIGGER reject_event_query ON game_world_event_query_receipts")
            .execute(&h.pool)
            .await?;
        assert!(
            exchange(&restarted, &mut admitted, &[(4, &[])])
                .await?
                .is_empty()
        );
        assert_eq!(admitted.continuity.next_command_id, 5);
        assert_eq!(receipt_count(&h).await?, 2);
        drop(restarted);
        let mut restarted = deps.owner(&h, &authority);
        restarted.active_generation = controller.active();
        restarted.reset_epoch = epochs;
        // The reconstructed fixture owner starts a new ingress from its initial
        // admission. Replay durable receipt IDs before retrying the failed ID.
        admitted = fresh;
        assert_eq!(
            exchange(&restarted, &mut admitted, &[(1, &[0])]).await?,
            [(CommandStatus::Rejected, Vec::new())]
        );
        assert_eq!(
            exchange(&restarted, &mut admitted, &[(2, &[]), (3, &[])]).await?,
            replay
        );
        assert_eq!(receipt_count(&h).await?, 2);
        let recovered = exchange(&restarted, &mut admitted, &[(4, &[])]).await?;
        assert_eq!(recovered.len(), 1);
        assert_eq!(recovered[0].0, CommandStatus::Accepted);
        assert_eq!(
            wire::decode_world_event_state(&recovered[0].1).map_err(debug)?,
            state
        );
        assert_eq!(receipt_count(&h).await?, 3);
        // Corrupt persisted state with just the event binding missing. Neither
        // channel boot nor real dispatch may synthesize a replacement decision. DAILY-1's
        // reset-history trigger needs the injected epoch to activate after the real ones.
        sqlx::query("INSERT INTO game_world_reset_epochs SELECT world_id,99,target_digest,map_revision,server_digest,client_digest,frame_digest,calendar,calendar_revision,clock_timestamp(),reset_month FROM game_world_reset_epochs WHERE reset_epoch=0").execute(&h.pool).await?;
        sqlx::query("UPDATE game_world_reset_active SET reset_epoch=99")
            .execute(&h.pool)
            .await?;
        assert_eq!(h.root.active_world_reset_epoch(world).await?, Some(99));
        assert!(h.root.active_world_event_epoch(world).await.is_err());
        restarted.reset_epoch = 99;
        assert!(
            exchange(&restarted, &mut admitted, &[(5, &[])])
                .await?
                .is_empty()
        );
        assert_eq!(admitted.continuity.next_command_id, 6);
        assert_eq!(receipt_count(&h).await?, 3);
        sqlx::query("UPDATE game_world_reset_active SET reset_epoch=$1::text::numeric")
            .bind(epochs.to_string())
            .execute(&h.pool)
            .await?;
        // The occurrence is live data for new queries; replay retains the exact
        // committed calendar even when the event ends between commands.
        let _closed_clock = oteryn_gameplay_foundation::scheduled_events::test_clock::set(
            oteryn_gameplay_foundation::scheduled_events::content_schedule().clone(),
            1_793_682_000,
        );
        let old = h
            .root
            .query_world_event(&authority, &h.node, item_fence, 2, epochs)
            .await?;
        assert_eq!(old, results[1].1);
        sqlx::raw_sql("CREATE OR REPLACE FUNCTION game_daily_effect_now() RETURNS timestamptz LANGUAGE sql VOLATILE AS $$ SELECT to_timestamp(1793682000) $$")
            .execute(&h.pool).await?;
        let current = h
            .root
            .query_world_event(&authority, &h.node, item_fence, 99, epochs)
            .await?;
        let current = wire::decode_world_event_state(&current).map_err(debug)?;
        assert!(
            current
                .seasonal
                .iter()
                .all(|event| event.event != wire::SeasonalEventKind::Halloween || !event.active)
        );
        assert_ne!(current.seasonal, state.seasonal);
        assert_eq!(
            current
                .cake
                .as_ref()
                .ok_or("retained Cake observation")?
                .observed_unix,
            1793682000
        );
        assert_eq!(
            current.cake.as_ref().and_then(|cake| cake.bonus_until),
            Some(1772341200)
        );

        assert_eq!(receipt_count(&h).await?, 4);
        deps.loss(&h, 60).await?;
        assert!(matches!(h.root.query_world_event(&authority, &h.node, item_fence, 4, epochs).await,
            Err(crate::durability::character_progression::CharacterProgressionError::AuthorityRejected)));
        assert_eq!(receipt_count(&h).await?, 4);
        eprintln!(
            "WORLD-EVENT-PG: real reset epoch={epochs}, wire/receipt/restart/fences/calendar passed elapsed={:?}",
            started.elapsed()
        );
        drop(restarted);
        drop(authority);
        drop(seal);
        h.cleanup().await
    })
}

async fn receipt_count(h: &Harness) -> TestResult<i64> {
    Ok(
        sqlx::query_scalar("SELECT count(*) FROM game_world_event_query_receipts")
            .fetch_one(&h.pool)
            .await?,
    )
}

async fn exchange(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    commands: &[(u64, &[u8])],
) -> TestResult<Vec<(CommandStatus, Vec<u8>)>> {
    let mut input = Vec::new();
    for (command_id, payload) in commands {
        let frame = encode_client_command(
            admitted.continuity.connection_generation,
            &ClientCommandValue {
                command_id: *command_id,
                command_type: wire::COMMAND_TYPE_WORLD_EVENT_QUERY,
                payload,
            },
        )
        .map_err(debug)?;
        input.extend_from_slice(&u32::try_from(frame.len())?.to_be_bytes());
        input.extend_from_slice(&frame);
    }
    let (mut server, mut client) = tokio::io::duplex(1 << 20);
    client.write_all(&input).await?;
    client.shutdown().await?;
    let end = tokio::time::timeout(
        Duration::from_secs(30),
        serve_admitted(&mut server, *admitted, owner, IDLE_LIVENESS),
    )
    .await?;
    match end {
        ConnectionEnd::AdmittedThenDisconnected(current)
        | ConnectionEnd::AdmittedThenClosed(current, _) => *admitted = current,
        other => return Err(format!("world event connection ended {other:?}").into()),
    }
    drop(server);
    let mut output = Vec::new();
    client.read_to_end(&mut output).await?;
    let mut cursor = 0;
    let mut results = Vec::new();
    while cursor < output.len() {
        let length = u32::from_be_bytes(output[cursor..cursor + 4].try_into()?) as usize;
        let envelope =
            decode_wire_envelope(&output[cursor + 4..cursor + 4 + length]).map_err(debug)?;
        if envelope.message_type() == MessageType::CommandResult {
            let result = decode_command_result(envelope.payload()).map_err(debug)?;
            results.push((result.status, result.payload.to_vec()));
        }
        cursor += 4 + length;
    }
    Ok(results)
}

struct T94Bootstrap;
impl crate::native_admission_source::account_characters::EpochFence for T94Bootstrap {
    fn read(
        &mut self,
    ) -> Result<u64, crate::native_admission_source::account_characters::FenceUnusable> {
        Ok(0)
    }
    fn persist(
        &mut self,
        _: u64,
    ) -> Result<(), crate::native_admission_source::account_characters::FenceUnusable> {
        Ok(())
    }
}
