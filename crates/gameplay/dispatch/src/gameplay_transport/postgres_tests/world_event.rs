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

#[test]
fn w05d_halloween_calendar_reaches_real_reset_query_and_replay() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario_run(2, Some("Halloween"), false, None).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "seasonal reset query thread panicked")??;
    Ok(())
}

#[test]
fn w05d_christmas_real_npc_present_replay_and_permanent_claim() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario_run(2, Some("Christmas"), false, None).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Christmas native NPC thread panicked")??;
    eprintln!(
        "W05D_CHRISTMAS_PG_EXECUTED elapsed_ms={}",
        started.elapsed().as_millis()
    );
    Ok(())
}

#[test]
fn w05d_cake_real_baker_burns_only_remaining_goal_and_replays() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(|| scenario_run(2, Some("A Piece of Cake"), false, None).map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "Cake native NPC thread panicked")??;
    eprintln!(
        "W05D_CAKE_BAKER_PG_EXECUTED elapsed_ms={}",
        started.elapsed().as_millis()
    );
    Ok(())
}

pub(super) fn scenario(epochs: u64) -> TestResult {
    scenario_run(epochs, None, false, None)
}

pub(super) fn scenario_with_consumers(epochs: u64, consumers: bool) -> TestResult {
    scenario_run(epochs, None, consumers, None)
}

pub(super) fn scenario_with_native_deaths(epochs: u64, creature: &'static str, active: bool) -> TestResult {
    scenario_run(epochs, None, false, Some((creature, active)))
}

fn scenario_run(epochs: u64, seasonal: Option<&str>, consumers: bool, native_deaths: Option<(&'static str, bool)>) -> TestResult {
    run(async move |admin| {
        reset();
        let h =
            Harness::create_with_admission_seed(admin, "world_event_dispatch", !(consumers || native_deaths.is_some()), false).await?;
        let clock = seasonal.or_else(|| native_deaths.and_then(|(creature, active)| {
            (creature == "werewolf").then_some(if active { "QworldGrimvaleActive" } else { "QworldGrimvaleInactive" })
        }));
        if clock.is_some() {
            // Fixture clock set before the real reset writer creates its epoch;
            // admission, publication and mutation guards remain enabled.
            if clock == Some("A Piece of Cake") {
                // Business clock matches the historical epoch fixture. T94's
                // actual observation clock remains current: terminal native
                // evidence must not claim a future decision (native.rs:483-484).
                sqlx::query("CREATE OR REPLACE FUNCTION game_cake_now_ms() RETURNS bigint LANGUAGE sql STABLE AS $$ SELECT floor(extract(epoch FROM TIMESTAMPTZ '2026-02-21 06:00 Europe/Berlin')*1000)::bigint $$")
                    .execute(&h.pool).await?;
                sqlx::query("CREATE FUNCTION seasonal_fixture_clock() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN NEW.activated_at=TIMESTAMPTZ '2026-02-21 06:00 Europe/Berlin'; NEW.reset_month=to_char((NEW.activated_at AT TIME ZONE 'Europe/Berlin')-interval '6 hours','YYYY-MM'); RETURN NEW; END $$")
                    .execute(&h.pool).await?;
            } else if matches!(clock, Some("QworldGrimvaleActive" | "QworldGrimvaleInactive")) {
                let clock_sql = if clock == Some("QworldGrimvaleActive") {
                    "CREATE FUNCTION seasonal_fixture_clock() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN NEW.activated_at=TIMESTAMPTZ '2026-10-13 06:00 Europe/Berlin'; NEW.reset_month=to_char((NEW.activated_at AT TIME ZONE 'Europe/Berlin')-interval '6 hours','YYYY-MM'); RETURN NEW; END $$"
                } else {
                    "CREATE FUNCTION seasonal_fixture_clock() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN NEW.activated_at=TIMESTAMPTZ '2026-10-12 06:00 Europe/Berlin'; NEW.reset_month=to_char((NEW.activated_at AT TIME ZONE 'Europe/Berlin')-interval '6 hours','YYYY-MM'); RETURN NEW; END $$"
                };
                sqlx::query(clock_sql).execute(&h.pool).await?;
            } else if clock == Some("Christmas") {
                sqlx::query("CREATE FUNCTION seasonal_fixture_clock() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN NEW.activated_at=TIMESTAMPTZ '2025-12-12 06:00 Europe/Berlin'; NEW.reset_month=to_char((NEW.activated_at AT TIME ZONE 'Europe/Berlin')-interval '6 hours','YYYY-MM'); RETURN NEW; END $$")
                    .execute(&h.pool).await?;
            } else {
                sqlx::query("CREATE FUNCTION seasonal_fixture_clock() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN NEW.activated_at=TIMESTAMPTZ '2026-10-31 06:00 Europe/Berlin'; NEW.reset_month=to_char((NEW.activated_at AT TIME ZONE 'Europe/Berlin')-interval '6 hours','YYYY-MM'); RETURN NEW; END $$")
                    .execute(&h.pool).await?;
            }
            sqlx::query("CREATE TRIGGER seasonal_fixture_clock BEFORE INSERT ON game_world_reset_epochs FOR EACH ROW EXECUTE FUNCTION seasonal_fixture_clock()")
                .execute(&h.pool).await?;
        }
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
        if consumers || native_deaths.is_some() {
            deps.combat_room = Some(room.clone());
            // Native death threshold cases retain twelve distinct dead actors
            // alongside the player; the smaller consumer fixture needs only three slots.
            let actor_capacity = if native_deaths.is_some() { 14 } else { 3 };
            planned_world_reset::reset_fixture::set_runtime_with_capacity(
                &mut deps, world, channel, &pin, epochs + 1, actor_capacity,
            )?;
        } else {
            set_seasonal_runtime(&mut deps, world, channel, &pin, epochs + 1)?;
        }
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
        if consumers || native_deaths.is_some() {
            let mut runtime = deps.runtime.lock().await;
            runtime.initialize_pinned_test_position(actor, crate::foundation::MovementLocalPosition { x: 2, y: 11, floor: -7 }).map_err(debug)?;
            runtime.bind_attacker_lease(actor, deps.admitted.game_session_id,
                crate::foundation::CharacterLease::new(crate::foundation::CharacterId::decode(&id(41)).map_err(debug)?, 2).map_err(debug)?).map_err(debug)?;
        }

        // Match production first-entry admission: inventory services require the
        // runtime actor to carry the same durable Character lease as its fence.
        deps.runtime
            .lock()
            .await
            .bind_attacker_lease(
                actor,
                deps.admitted.game_session_id,
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(item_fence.character_id.as_bytes())
                        .map_err(debug)?,
                    item_fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
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

        if let Some((creature, active)) = native_deaths {
            super::qworld_native_deaths::exercise(&h, &mut owner, &mut admitted, creature, active).await?;
        }
        owner
            .runtime
            .lock()
            .await
            .initialize_first_entry_position(actor)
            .map_err(debug)?;
        if seasonal == Some("Christmas") {
            santa_present_scenario(&h, &owner, actor, admitted.game_session_id, item_fence).await?;
        }
        if seasonal == Some("A Piece of Cake") {
            cake_baker_scenario(&h, &owner, actor, admitted.game_session_id, item_fence).await?;
        }
        let offered = owner.offered_capabilities();
        assert!(
            offered
                .iter()
                .any(|c| c.id == wire::CAPABILITY_WORLD_EVENT_V1)
        );
        admitted.continuity.selected_capabilities =
            SelectedCapabilities::select(offered, &[6, 11, 53]).ok_or("capability 53")?;
        let fresh = admitted;

        // Malformed query never reaches storage. A valid query commits its exact result.
        let results = exchange(&owner, &mut admitted, &[(1, &[0]), (2, &[])]).await?;
        assert_eq!(results.len(), 2);
        assert_eq!(results[0], (CommandStatus::Rejected, Vec::new()));
        assert_eq!(results[1].0, CommandStatus::Accepted);
        let state = wire::decode_world_event_state(&results[1].1).map_err(debug)?;
        assert_eq!(state.reset_epoch, epochs);
        let expected_active = epochs % 2 == 1 || seasonal.is_some() || changes.their_masters_voice || changes.grimvale_full_moon || changes.fury_gates.is_some();
        assert_eq!(state.active, expected_active);
        assert_eq!(state.announcement.is_empty(), !expected_active);
        for (name, active) in [("Their Master's Voice", changes.their_masters_voice), ("Grimvale", changes.grimvale_full_moon), ("Fury Gates", changes.fury_gates.is_some())] {
            assert_eq!(state.announcement.contains(name), active, "wire world-state decision for {name}");
        }
        if let Some(name) = seasonal {
            assert!(state.announcement.contains(name), "the real query must expose its seasonal producer");
        }
        let stored: Vec<u8> = sqlx::query_scalar(
            "SELECT result FROM game_world_event_query_receipts WHERE command_id=2",
        )
        .fetch_one(&h.pool)
        .await?;
        assert_eq!(stored, results[1].1);
        assert_eq!(receipt_count(&h).await?, 1);

        // No negotiated capability => no event bytes or receipt, even for a valid query.
        let mut unsupported = admitted;
        unsupported.continuity.selected_capabilities =
            SelectedCapabilities::select(offered, &[6, 11]).ok_or("base capability")?;
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
        deps.loss(&h, 60).await?;
        assert!(matches!(h.root.query_world_event(&authority, &h.node, item_fence, 4, epochs).await,
            Err(crate::durability::character_progression::CharacterProgressionError::AuthorityRejected)));
        assert_eq!(receipt_count(&h).await?, 3);
        eprintln!("WORLD-EVENT-PG: real reset epoch={epochs}, wire/receipt/restart/fences passed");
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

fn set_seasonal_runtime(
    deps: &mut Dependencies,
    world: WorldId,
    channel: ChannelId,
    pin: &crate::content::NativeEntryContentPin,
    generation: u64,
) -> TestResult {
    let room = pin.qualified_room();
    let start = room.entry_start();
    // Native entry maps use -z; ordinary Channel/NPC movement uses legacy z.
    // Match the production bundle pin boundary (node/serve.rs::bundle_channel_pin),
    // rather than negating an already-native player floor at NPC placement.
    let legacy_floor = start
        .floor
        .checked_neg()
        .ok_or("native fixture floor overflow")?;
    assert!(
        (0..=15).contains(&legacy_floor),
        "source-qualified legacy floor"
    );
    let content = ChannelContentPin::from_activation(
        world,
        generation,
        pin.identity().server_artifact_digest(),
        pin.identity().client_artifact_digest(),
        pin.frame_binding().digest(),
        room.map_revision_digest(),
        (start.x, start.y, legacy_floor),
    );
    deps.runtime = Mutex::new(
        ChannelRuntimeV1::from_committed_assignment(
            world,
            channel,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            generation,
            generation,
            &format!("runtime-scope-assignment:{generation}"),
            2,
            content,
        )
        .map_err(debug)?,
    );
    Ok(())
}

async fn santa_present_scenario(
    h: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
) -> TestResult {
    use crate::foundation::{MovementFacing, NpcPlacement};
    use crate::gameplay_transport::connection::UseCommand;
    use crate::gameplay_transport::npc_talk::{NpcTalkBook, NpcTalkEntry};
    use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
    use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
    let book = NpcTalkBook::default().with(
        "oteryn:npc.santa_claus",
        NpcTalkEntry::new("Santa Claus", Some("Merry Christmas!")),
    );
    owner
        .states
        .chat
        .install_npc_talk(std::sync::Arc::new(book));
    owner
        .states
        .chat
        .admit(session, "Fixture Hero", owner.owner_now());
    owner.states.chat.set_npc_service(session);
    let npc_actor = {
        let mut runtime = owner.runtime.lock().await;
        let position = runtime
            .read_actor_position(actor)
            .map_err(debug)?
            .position();
        let npc = runtime
            .place_dynamic_npc(
                "oteryn:npc.santa_claus",
                NpcPlacement {
                    x: u16::try_from(position.x + 1)?,
                    y: u16::try_from(position.y)?,
                    floor: -i8::try_from(position.floor)?,
                    direction: MovementFacing::South,
                },
            )
            .map_err(debug)?;
        let npc = runtime
            .visible_entities()
            .npcs
            .into_iter()
            .find(|visible| visible.actor == npc)
            .ok_or("Santa native actor")?;
        EntityRef {
            identity: npc.actor.placement_identity(),
            generation: npc.generation,
        }
    };
    super::item_look::seed_main_backpack(h, &[]).await?;
    let command = |command_id| UseCommand {
        game_session_id: session,
        command_id,
        item_fence: Some(fence),
    };
    let intent = |text: &str| NpcTalkIntent {
        npc_actor,
        text: text.into(),
    };
    assert_eq!(
        owner
            .npc_talk(actor, command(100), intent("hi"))
            .await
            .disposition,
        NpcIntentDisposition::Ok
    );
    // Constrain only the fixture input, using the real deterministic decision
    // domain: both source branches must be exercised for every content digest.
    // The production NPC caller still performs its own original draws.
    let root = oteryn_simulation_determinism::GameplayDecisionRoot::from_bytes(
        owner
            .runtime
            .lock()
            .await
            .content_pin()
            .server_artifact_digest(),
    );
    let mut normal_command = None;
    let mut empty_command = None;
    for candidate in 101_u64..10_101 {
        use sha2::{Digest, Sha256};
        let mut hash = Sha256::new();
        hash.update(b"oteryn:santa-present:v1");
        hash.update(session.as_bytes());
        hash.update(candidate.to_be_bytes());
        hash.update(fence.character_id.as_bytes());
        let occurrence = oteryn_simulation_determinism::DecisionOccurrenceId::from_bytes(
            hash.finalize()[..16].try_into().map_err(debug)?,
        );
        let value = oteryn_simulation_determinism::deterministic_decision_u64(
            &root,
            occurrence,
            "santa.present.chance",
            0,
        )
        .map_err(debug)?;
        if value >= u64::MAX - u64::MAX % 100 {
            continue;
        }
        if value % 100 < 90 {
            normal_command.get_or_insert(candidate);
        } else {
            empty_command.get_or_insert(candidate);
        }
        if normal_command.is_some() && empty_command.is_some() {
            break;
        }
    }
    let normal_command = normal_command.ok_or("fixture normal source branch")?;
    let empty_command = empty_command.ok_or("fixture failed source branch")?;
    let failed = owner
        .npc_talk(actor, command(empty_command), intent("present"))
        .await;
    assert!(
        failed.inventory_changed,
        "native Santa present must change inventory: disposition={:?}, unknown={}, conversation={:?}",
        failed.disposition, failed.unknown, failed.conversation
    );
    assert_eq!(failed.disposition, NpcIntentDisposition::Ok);
    assert!(
        failed.conversation.is_none(),
        "the source fails before its Christmas reply"
    );
    let failed_counts: (i64,i64,i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_reward_claims WHERE claim_production_key='oteryn:reward-claim.santa-claus-present'),(SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i6510'),(SELECT count(*) FROM game_reward_claim_mint_receipts)").fetch_one(&h.pool).await?;
    assert_eq!(
        failed_counts,
        (0, 1, 1),
        "the source creates its bag before failure and sets no permanent Santa flag"
    );
    let empty_replay = owner
        .npc_talk(actor, command(empty_command), intent("present"))
        .await;
    assert!(empty_replay.inventory_changed);
    assert!(empty_replay.conversation.is_none());
    let empty_bags:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i6510'").fetch_one(&h.pool).await?;
    assert_eq!(
        empty_bags, 1,
        "the same failed-script command never mints twice"
    );
    let gift = owner
        .npc_talk(actor, command(normal_command), intent("present"))
        .await;
    assert!(gift.inventory_changed, "native gift must reach MINT");
    assert_eq!(
        gift.conversation.as_ref().ok_or("gift conversation")?.lines,
        ["Merry Christmas!"]
    );
    let counts: (i64,i64,i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_reward_claims WHERE claim_production_key='oteryn:reward-claim.santa-claus-present'),(SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i6510'),(SELECT count(*) FROM game_reward_claim_mint_receipts)").fetch_one(&h.pool).await?;
    assert_eq!(counts, (1, 2, 2));
    // Four actual talk intents exhausted this source window. Advance the
    // fixture's existing monotonic clock before replay and a fresh request.
    let mut later = (*owner).clone();
    later.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(2))
        .ok_or("fixture monotonic clock")?;
    let replay = later
        .npc_talk(actor, command(normal_command), intent("present"))
        .await;
    assert!(replay.inventory_changed);
    let repeated = later
        .npc_talk(actor, command(20_000), intent("present"))
        .await;
    assert!(!repeated.inventory_changed);
    assert_eq!(
        repeated
            .conversation
            .as_ref()
            .ok_or("once conversation")?
            .lines,
        ["You can't get other present."]
    );
    let after: (i64,i64,i64) = sqlx::query_as("SELECT (SELECT count(*) FROM game_reward_claims WHERE claim_production_key='oteryn:reward-claim.santa-claus-present'),(SELECT count(*) FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i6510'),(SELECT count(*) FROM game_reward_claim_mint_receipts)").fetch_one(&h.pool).await?;
    assert_eq!(after, counts);
    Ok(())
}

async fn cake_baker_scenario(
    h: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
) -> TestResult {
    use crate::foundation::{MovementFacing, NpcPlacement};
    use crate::gameplay_transport::connection::UseCommand;
    use crate::gameplay_transport::npc_talk::{NpcTalkBook, NpcTalkEntry};
    use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};
    use oteryn_protocol_oteryn::world_spatial_entities::EntityRef;
    let book = NpcTalkBook::default().with(
        "oteryn:npc.biff_the_baker",
        NpcTalkEntry::new("Biff The Baker", Some("Hello!")),
    );
    owner
        .states
        .chat
        .install_npc_talk(std::sync::Arc::new(book));
    owner
        .states
        .chat
        .admit(session, "Fixture Hero", owner.owner_now());
    owner.states.chat.set_npc_service(session);
    let npc_actor = {
        let mut runtime = owner.runtime.lock().await;
        let position = runtime
            .read_actor_position(actor)
            .map_err(debug)?
            .position();
        let npc = runtime
            .place_dynamic_npc(
                "oteryn:npc.biff_the_baker",
                NpcPlacement {
                    x: u16::try_from(position.x + 1)?,
                    y: u16::try_from(position.y)?,
                    floor: -i8::try_from(position.floor)?,
                    direction: MovementFacing::South,
                },
            )
            .map_err(debug)?;
        let npc = runtime
            .visible_entities()
            .npcs
            .into_iter()
            .find(|visible| visible.actor == npc)
            .ok_or("Biff native actor")?;
        EntityRef {
            identity: npc.actor.placement_identity(),
            generation: npc.generation,
        }
    };
    super::item_look::seed_main_backpack(
        h,
        &[
            ("oteryn:item.tibia.i12143", 100),
            ("oteryn:item.tibia.i12143", 100),
            ("oteryn:item.tibia.i12143", 100),
        ],
    )
    .await?;
    let command = |command_id| UseCommand {
        game_session_id: session,
        command_id,
        item_fence: Some(fence),
    };
    let intent = |text: &str| NpcTalkIntent {
        npc_actor,
        text: text.into(),
    };
    assert_eq!(
        owner
            .npc_talk(actor, command(100), intent("hi"))
            .await
            .disposition,
        NpcIntentDisposition::Ok
    );
    let delivered = owner.npc_talk(actor, command(101), intent("deliver")).await;
    assert!(
        delivered.inventory_changed,
        "native Baker topic must reach the physical burn owner: disposition={:?}, unknown={}, conversation={:?}",
        delivered.disposition, delivered.unknown, delivered.conversation
    );
    assert_eq!(
        delivered
            .conversation
            .as_ref()
            .ok_or("Baker delivery conversation")?
            .lines,
        ["Wonderful, thank you! You delivered 200 substance(s). (200/200 total delivered)"]
    );
    let progress = owner
        .root
        .cake_world_progress(owner.world_id, owner.reset_epoch)
        .await?
        .ok_or("committed Cake occurrence")?;
    assert_eq!((progress.substances, progress.golems), (200, 0));
    let remaining:i64=sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i12143' AND lifecycle=1").fetch_one(&h.pool).await?;
    assert_eq!(remaining, 100);
    let replay = owner.npc_talk(actor, command(101), intent("deliver")).await;
    assert!(replay.inventory_changed);
    assert_eq!(
        replay
            .conversation
            .as_ref()
            .ok_or("Baker replay conversation")?
            .lines,
        delivered
            .conversation
            .as_ref()
            .ok_or("delivery reply")?
            .lines
    );
    let goal = owner.npc_talk(actor, command(102), intent("deliver")).await;
    assert!(!goal.inventory_changed);
    assert_eq!(
        goal.conversation
            .as_ref()
            .ok_or("Baker goal conversation")?
            .lines,
        ["We already have all the substances we need, thank you though!"]
    );
    let after:i64=sqlx::query_scalar("SELECT coalesce(sum(quantity),0)::bigint FROM game_item_instances WHERE definition_production_key='oteryn:item.tibia.i12143' AND lifecycle=1").fetch_one(&h.pool).await?;
    let receipts:i64=sqlx::query_scalar("SELECT count(*) FROM game_item_consume_transform_records WHERE recipe='oteryn:cake.baker-substances'").fetch_one(&h.pool).await?;
    assert_eq!((after, receipts), (remaining, 1));
    Ok(())
}
