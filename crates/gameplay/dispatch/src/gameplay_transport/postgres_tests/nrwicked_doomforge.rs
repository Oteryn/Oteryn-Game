//! Drive the retained Doomforge teleport through the session's normal paced STEP.
use super::fixture::{Harness, debug};
use super::{Dependencies, TestResult, prepare_current_session};
use crate::content::{channel_content_pin::ChannelContentPin, qualify_native_entry_room};
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use crate::map::boot::{boot, tests as fixture};
use crate::map::overlay::TilePos;
use oteryn_protocol_oteryn::world_spatial::{StepDirection, StepDisposition};
use oteryn_world_bundle::{bundle, sector, writer};

const START: TilePos = TilePos {
    x: 34040,
    y: 32330,
    floor: -14,
};
const LEVER: TilePos = TilePos { x: 34058, y: 32394, floor: -14 };
const ARENA_EXIT: TilePos = TilePos { x: 34058, y: 32403, floor: -14 };
const STAGING: TilePos = TilePos { y: LEVER.y + 1, ..LEVER };
const ARENA_ENTRY: MovementLocalPosition = MovementLocalPosition { x: 34034, y: 32340, floor: 14 };
const WICKED_TRACK: &str =
    "oteryn:quest-progress/crystalserver/quest/u14_10/no_rest_for_the_wicked";

const PORTAL: TilePos = TilePos { x: 34041, ..START };
const FORGE: TilePos = TilePos {
    x: 34062,
    y: 32334,
    ..START
};

#[test]
fn nrwicked_doomforge_step_cannot_use_a_retargeted_destination() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(
        admin,
        "nrwicked_doomforge_retargeted",
        TilePos {
            y: FORGE.y + 1,
            ..FORGE
        },
    )
}

#[test]
fn nrwicked_doomforge_step_cannot_use_the_raw_destination() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run(admin, "nrwicked_doomforge", FORGE)
}

#[test]
fn nrwicked_two_admitted_players_can_leave_after_the_hard_win() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, "nrwicked_party", FORGE, Scenario::HardWin)
}

#[test]
fn nrwicked_abandoned_attempt_releases_actors_but_retains_admission_cooldown() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, "nrwicked_abandonment", FORGE, Scenario::Abandonment)
}

#[test]
fn nrwicked_lost_admission_commit_ack_replays_and_publishes() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, "nrwicked_admission_ack", FORGE, Scenario::AdmissionAckLost)
}

#[test]
fn nrwicked_reconnect_grace_retains_the_fight_until_expiry() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, "nrwicked_reconnect", FORGE, Scenario::ReconnectGrace)
}

#[test]
fn nrwicked_fire_deadline_is_measured_from_shield_raise() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(
        admin,
        "nrwicked_fire_deadline",
        FORGE,
        Scenario::FireDeadline,
    )
}

#[test]
fn nrwicked_successor_keeps_paid_cooldown_before_publication() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, "nrwicked_paid_before_publish", FORGE, Scenario::PaidBeforePublication)
}

#[test]
fn nrwicked_successor_keeps_paid_cooldown_after_publication() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, "nrwicked_paid_after_publish", FORGE, Scenario::PaidAfterPublication)
}

#[test]
fn nrwicked_charge_refuses_stale_party_session_without_payment() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    run_case(admin, "nrwicked_stale_charge", FORGE, Scenario::StaleCharge)
}

#[derive(Clone, Copy)]
enum Scenario {
    Portal,
    HardWin,
    Abandonment,
    ReconnectGrace,
    FireDeadline,
    AdmissionAckLost,
    PaidBeforePublication,
    PaidAfterPublication,
    StaleCharge,
}

fn run(admin: String, fixture_name: &'static str, destination: TilePos) -> TestResult {
    run_case(admin, fixture_name, destination, Scenario::Portal)
}

fn run_case(
    admin: String,
    fixture_name: &'static str,
    destination: TilePos,
    case: Scenario,
) -> TestResult {
    let started = std::time::Instant::now();
    let result = std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            scenario(admin, fixture_name, destination, case).map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Doomforge scenario thread panicked")?;
    eprintln!(
        "POSTGRES_CASE_COMPLETED {fixture_name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result.map_err(Into::into)
}

fn scenario(
    admin: String,
    fixture_name: &'static str,
    destination: TilePos,
    case: Scenario,
) -> TestResult {
    let encounter = !matches!(case, Scenario::Portal);
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            let h = Harness::create_with_admission_seed(admin, fixture_name, true, false).await?;
            prepare_current_session(&h).await?;
            if encounter {
                h.seed_appearance_observer().await?;
                super::appearance_change_postgres_fixtures::observer_visibility_tests::admit(&h).await?;
            }
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let initial = Dependencies::new()?;
            let world = initial.admitted.world_id;
            let channel = initial.admitted.channel_id;
            let production = if encounter {
                Some(super::death_ml_loss_composed_fixtures::production_generation_with_pin(world, "content-1")?)
            } else {
                None
            };
            let generation = production.as_ref().and_then(|(controller, _)| controller.active());
            let native = generation.and_then(|g| g.native_gameplay());
            let room = if let Some((_, pin)) = &production {
                pin.qualified_room().clone()
            } else {
                qualify_native_entry_room(world).map_err(debug)?
            };
            let (bytes, pins) = portal_fixture(destination, native)?;
            let map = boot(&bytes, &pins, world, channel, |key| {
                if let Some(native) = native {
                    return crate::map::boot::native_item_definition(Some(native), key);
                }
                (key == "oteryn:item.tibia.i37000")
                    .then(|| fixture::floor_items("oteryn:item.tibia.i37001"))
                    .flatten()
            }).map_err(debug)?;
            assert_eq!(map.floors().teleport(PORTAL), Some(destination));
            let cells = map.movement_cells(room.movement_cells()).map_err(debug)?;
            let spawn = if encounter { STAGING } else { START };
            let pin = ChannelContentPin::from_activation(
                world,
                1,
                room.compiled().server_digest(),
                room.compiled().client_digest(),
                map.frame_binding_digest(world),
                map.map_revision_digest(),
                (i32::from(spawn.x), i32::from(spawn.y), 14),
            );
            let mut deps = Dependencies::with_content_and_capacity(
                pin.clone(),
                room.clone(),
                crate::spell::SpellBook::new(vec![]).map_err(debug)?,
                if encounter { 16 } else { 2 },
            )?;
            if matches!(case, Scenario::FireDeadline | Scenario::ReconnectGrace) {
                deps.combat_room = Some(room.bundle_view(
                    cells.clone(),
                    pin.frame_binding_digest(),
                    pin.map_revision_digest(),
                    crate::content::NativeEntryStart {
                        x: i32::from(spawn.x),
                        y: i32::from(spawn.y),
                        floor: 14,
                    },
                ).ok_or("qualified Doomforge AI bundle room")?);
            }
            let recovery_source = if matches!(case, Scenario::ReconnectGrace) {
                Some(super::house_transfer_fixtures::RecoverySource::start(&h).await?)
            } else {
                None
            };
            let mut owner = deps.owner(&h, &authority);
            if let Some(source) = &recovery_source {
                owner.evidence = &source.evidence;
            }
            owner.movement_cells = (&cells).into();
            owner.active_generation = generation;
            let session = deps.admitted.game_session_id;
            {
                let mut runtime = deps.runtime.lock().await;
                runtime
                    .initialize_first_entry_position(deps.actor)
                    .map_err(debug)?;
                owner
                    .states
                    .spell_states
                    .lock()
                    .await
                    .initialize(
                        &runtime,
                        deps.actor,
                        session,
                        crate::gameplay_transport::actor_spell::tests::FACTS,
                        (0, 0),
                        owner.owner_now(),
                    )
                    .ok_or("spell state")?;
            }
            if encounter {
                owner.quest_catalogue = Some(std::sync::Arc::new(
                    oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                        .map_err(debug)?
                        .catalogue()
                        .clone(),
                ));
                let lever_key = bundle::placement_key(LEVER.floor, LEVER.x, LEVER.y, 2)
                    .ok_or("lever placement")?;
                let mut lever_placement = map.base().digest().to_vec();
                lever_placement.extend_from_slice(&lever_key.to_be_bytes());
                sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user, encode($1,'hex')::uuid, encode($2,'hex')::uuid, 4)")
                    .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).execute(&h.pool).await?;
                sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid, encode($2,'hex')::uuid, 1, NULL, $3, $4, $5)")
                    .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice())
                    .bind(pin.server_artifact_digest().as_slice()).bind(pin.client_artifact_digest().as_slice())
                    .bind(pin.frame_binding_digest().as_slice()).execute(&h.pool).await?;
                two_player_encounter(
                    &h, &deps, &mut owner, deps.admitted, lever_placement, map.facts(), case,
                )
                    .await?;
                drop(owner);
                drop(authority);
                drop(seal);
                return h.cleanup().await;
            }
            let (outcome, _) = owner
                .paced_step(deps.actor, session, StepDirection::East)
                .await;
            assert_eq!(outcome.disposition, StepDisposition::Blocked);
            assert_eq!(
                deps.runtime
                    .lock()
                    .await
                    .read_actor_position(deps.actor)
                    .map_err(debug)?
                    .position(),
                MovementLocalPosition {
                    x: i32::from(START.x),
                    y: i32::from(START.y),
                    floor: 14
                },
            );
            assert!(crate::map::floor::is_teleport(
                &cells,
                crate::content::LogicalCell {
                    x: i32::from(PORTAL.x),
                    y: i32::from(PORTAL.y),
                    z: -i32::from(PORTAL.floor),
                },
            ));
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
}

fn portal_fixture(
    destination: TilePos,
    native: Option<&crate::content::native_gameplay::NativeGameplayState>,
) -> TestResult<(Vec<u8>, crate::map::boot::BootPins)> {
    let (bytes, mut pins) = fixture::teleport_bundle();
    let mut manifest = bundle::read(&bytes)?.manifest;
    manifest.world = bundle::Extent {
        min_x: if native.is_some() { 34025 } else { START.x - 3 },
        min_y: if native.is_some() { 32322 } else { START.y },
        max_x: if native.is_some() { destination.x.max(STAGING.x) + 1 } else { destination.x + 1 },
        max_y: if native.is_some() { ARENA_EXIT.y + 1 } else { destination.y + 1 },
        floors: vec![START.floor],
    };
    let mut portal = manifest.palette.pop().ok_or("portal palette")?;
    portal.key = "oteryn:item.tibia.i37000".into();
    manifest.palette.truncate(1);
    if let Some(native) = native {
        portal.id = crate::map::boot::native_item_definition(Some(native), &portal.key)
            .ok_or("production portal")?.reference.get() - 1;
    }
    manifest.palette.push(portal);
    if let Some(native) = native {
        for key in ["oteryn:item.tibia.i2342", "oteryn:item.tibia.i8911"] {
            let definition = crate::map::boot::native_item_definition(Some(native), key)
                .ok_or("production lever stack item")?;
            manifest.palette.push(bundle::PaletteEntry {
                key: key.into(), family: bundle::Family::Item, id: definition.reference.get() - 1, terrain: None,
            });
        }
    }
    let tile = |at: TilePos| sector::Tile {
        x: at.x,
        y: at.y,
        flags: 0,
        house: 0,
        zones: vec![],
        items: vec![sector::Item {
            palette: 0,
            depth: 0,
            attrs: sector::Attrs::default(),
        }],
    };
    let mut entrance = tile(PORTAL);
    entrance.items.push(sector::Item {
        palette: 1,
        depth: 0,
        attrs: sector::Attrs {
            teleport: Some((
                destination.x,
                destination.y,
                destination.floor.to_ne_bytes()[0],
            )),
            ..Default::default()
        },
    });
    let mut tiles = std::collections::BTreeMap::<_, Vec<_>>::new();
    let mut add = |tile: sector::Tile| {
        tiles.entry((tile.x / 32, tile.y / 32)).or_default().push(tile);
    };
    if native.is_some() {
        for y in 32322..=32341 {
            for x in 34025..=34043 {
                if (x, y) != (PORTAL.x, PORTAL.y) {
                    add(tile(TilePos { x, y, ..START }));
                }
            }
        }
        for y in STAGING.y..=STAGING.y + 4 {
            add(tile(TilePos { y, ..STAGING }));
        }
        add(tile(ARENA_EXIT));
        let mut lever = tile(LEVER);
        lever.items.extend([2, 3].map(|palette| sector::Item {
            palette, depth: 0, attrs: Default::default(),
        }));
        add(lever);
    } else {
        for x in START.x - 3..=START.x {
            add(tile(TilePos { x, ..START }));
        }
    }
    add(entrance);
    add(tile(destination));
    let mut sectors: Vec<_> = tiles.into_iter().map(|((sx, sy), mut tiles)| {
        tiles.sort_by_key(|tile| (tile.y, tile.x));
        bundle::Sector { floor: START.floor, sx, sy, tiles }
    }).collect();
    sectors.sort_by_key(bundle::order);
    let bytes = writer::write(&manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let mut pins = fixture::floor_pins(pins);
    pins.start = if native.is_some() { STAGING } else { START };
    Ok((bytes, pins))
}

async fn two_player_encounter(
    h: &Harness,
    deps: &Dependencies,
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: super::AdmittedSession,
    lever_placement: Vec<u8>,
    facts: &crate::map::facts::BundleFacts,
    case: Scenario,
) -> TestResult {
    use super::fixture::id;
    use crate::foundation::{CommandId, CommandRef, GameSessionId, OwnerDamageCommand};
    use crate::gameplay_transport::{
        encounter_phases,
        kill_reward::{DrainOutcome, KillSink, record_creature_kill},
    };
    use crate::gameplay_transport::connection::UseCommand;
    use oteryn_gameplay_encounters::bosses::phase_controller::arbaziloth::{BOSS, FORGEMASTER};
    use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
    use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
    const ENRAGED: &str = "oteryn:creature.enraged_arbaziloth";
    let first = admitted.runtime_actor.ok_or("first actor")?;
    let session = admitted.game_session_id;
    let other_session = GameSessionId::decode(&id(110)).map_err(debug)?;
    let admissions =
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone());
    let first_lease = admissions
        .current_session_at(session)
        .await?
        .0
        .current_character_lease();
    for participant in [session, other_session] {
        super::qengine_outfit_stories_fixtures::seed_production_progression_context(
            h, owner, participant,
        ).await?;
    }
    owner.admit_progression_session(&mut admitted).await;
    assert!(owner.progression_binding(session).is_some());
    let mut observer = super::AdmittedSession {
        game_session_id: other_session,
        runtime_actor: None,
        ..admitted
    };
    owner.admit_progression_session(&mut observer).await;
    assert!(owner.progression_binding(other_session).is_some());
    let (seed_fence, revision) =
        super::qengine_outfit_stories_fixtures::seed_quest_revision(owner, other_session, 111)
            .await?;
    assert_eq!(seed_fence.expected_character_revision.get(), 1);
    assert_eq!(revision, 2);
    let mut import = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *import)
        .await?;
    for (track, value) in [
        ("hard_kills", 8_i64),
        ("kill_arbaziloth/questline", 2),
        ("questlog", 1),
        ("forgemaster_door/questline", 1),
    ] {
        let key = format!("{WICKED_TRACK}/{track}");
        let definition = owner
            .quest_catalogue
            .as_ref()
            .ok_or("wicked catalogue")?
            .track(&key)
            .ok_or("canonical wicked track")?;
        assert!((definition.min..=definition.max).contains(&value));
        let inserted = sqlx::query(
            "INSERT INTO game_character_quest_tracks
                (character_id, track_key, quest_key, value, committed_character_revision)
             SELECT character_id, $2, $3, $4, character_revision
             FROM game_character_roots WHERE character_id = encode($1, 'hex')::uuid
                AND character_revision = $5::text::numeric",
        )
        .bind(seed_fence.character_id.as_bytes().as_slice())
        .bind(key)
        .bind(&definition.quest)
        .bind(value)
        .bind(revision.to_string())
        .execute(&mut *import)
        .await?;
        assert_eq!(inserted.rows_affected(), 1);
    }
    import.commit().await?;
    let second_lease = admissions
        .current_session_at(other_session)
        .await?
        .0
        .current_character_lease();
    let native = owner.active_generation.and_then(|g| g.native_gameplay())
        .ok_or("production native policies")?;
    let mut runtime = owner.runtime.lock().await;
    native.install_companion_policies(&mut runtime).map_err(debug)?;
    let reservation = runtime
        .reserve_fresh_session(other_session)
        .map_err(debug)?;
    let other = runtime.commit_fresh_session(reservation).map_err(debug)?;
    runtime.initialize_pinned_test_position(other, MovementLocalPosition {
        x: i32::from(STAGING.x), y: i32::from(STAGING.y) + 1, floor: 14,
    }).map_err(debug)?;
    runtime
        .bind_attacker_lease(first, session, first_lease)
        .map_err(debug)?;
    runtime
        .bind_attacker_lease(other, other_session, second_lease)
        .map_err(debug)?;
    owner
        .states
        .spell_states
        .lock()
        .await
        .initialize(
            &runtime,
            other,
            other_session,
            crate::gameplay_transport::actor_spell::tests::FACTS,
            (0, 0),
            owner.owner_now(),
        )
        .ok_or("second spell state")?;
    drop(runtime);
    let players = [(first, session), (other, other_session)];
    for (actor, session) in players {
        owner.refresh_quest_session(session, true).await;
        assert!(owner.refresh_source_party_presence(actor, session).await);
    }
    let fence = owner.current_quest_fence(session).await
        .map_err(|()| "Arbaziloth lever fence")?.ok_or("missing Arbaziloth lever fence")?;
    use std::sync::atomic::Ordering;
    use super::source_boss_handoff_fixtures::{
        active_cooldowns, assert_paid_handoff, assert_paid_receipt,
    };
    use crate::durability::boss_encounter::SourceBossLever;
    let lever = SourceBossLever::Arbaziloth;
    if matches!(case, Scenario::AdmissionAckLost) {
        owner.states.quest_callbacks.lose_source_boss_commit_ack.store(true, Ordering::Release);
    }
    if matches!(case, Scenario::PaidBeforePublication) {
        owner.states.quest_callbacks.interrupt_source_boss_before_projection.store(true, Ordering::Release);
    }
    let use_lever = owner.use_object(first, UseCommand {
        game_session_id: session, command_id: 1, item_fence: Some(fence.item_fence()),
    }, WorldObjectTarget { placement: lever_placement.clone(), expected_revision: 0 });
    let outcome = if matches!(case, Scenario::StaleCharge) {
        use crate::gameplay_transport::quest_callbacks::PRE_SOURCE_BOSS_CHARGE;
        use std::sync::Arc;
        let barrier = Arc::new((tokio::sync::Notify::new(), tokio::sync::Notify::new()));
        let using_lever = PRE_SOURCE_BOSS_CHARGE.scope(barrier.clone(), use_lever);
        tokio::pin!(using_lever);
        tokio::select! {
            outcome = &mut using_lever => return Err(format!(
                "lever ended before payment: {:?}", outcome.disposition,
            ).into()),
            () = barrier.0.notified() => {}
        }
        assert!(
            owner.runtime.try_lock().is_ok(),
            "world observations continue before payment"
        );
        assert!(
            owner.states.spell_states.try_lock().is_ok(),
            "spell observations continue before payment"
        );
        assert!(
            active_cooldowns(h, lever).await?.is_empty(),
            "staging and the lever reservation must precede payment"
        );
        // Advance the peer's actual durable generation after the party fences
        // were captured and the room was staged, before charging.
        let changed = sqlx::query(
            "UPDATE game_durability_reconnect_sessions SET current_generation=current_generation+1
              WHERE game_session_id=encode($1,'hex')::uuid AND session_state=2",
        )
        .bind(other_session.as_bytes().as_slice())
        .execute(&h.pool)
        .await?;
        assert_eq!(changed.rows_affected(), 1);
        barrier.1.notify_one();
        using_lever.await
    } else {
        use_lever.await
    };
    if matches!(case, Scenario::PaidBeforePublication | Scenario::StaleCharge) {
        assert_eq!(outcome.disposition, UseDisposition::Rejected);
        assert!(!owner.states.quest_callbacks.interrupt_source_boss_before_projection.load(Ordering::Acquire));
        {
            let runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            assert!(states.encounter_phases.active.lock().map_err(|_| "phases")?.is_empty());
            assert!(runtime.positioned_actor_census().map_err(debug)?.iter().all(
                |(_, _, session)| session.is_some(),
            ));
            for ((actor, _), y) in players.into_iter().zip(STAGING.y..=STAGING.y + 1) {
                let position = runtime.read_actor_position(actor).map_err(debug)?.position();
                assert_eq!((position.x, position.y), (i32::from(STAGING.x), i32::from(y)));
            }
        }
        let key = bundle::placement_key(LEVER.floor, LEVER.x, LEVER.y, 2).ok_or("lever key")?;
        assert_eq!(crate::map::view::MapFacts::object_revision(facts, key), 0);
        if matches!(case, Scenario::StaleCharge) {
            assert!(active_cooldowns(h, lever).await?.is_empty());
            let admissions: i64 = sqlx::query_scalar("SELECT count(*) FROM game_source_boss_admissions")
                .fetch_one(&h.pool).await?;
            assert_eq!(admissions, 0, "the stale party cannot be charged");
            return Ok(());
        }
        let peer = owner.current_quest_fence(other_session).await
            .map_err(|()| "peer fence")?.ok_or("missing peer fence")?;
        return assert_paid_handoff(h, deps, owner, lever, vec![fence.item_fence(), peer.item_fence()]).await;
    }
    assert_eq!(outcome.disposition, UseDisposition::Committed, "production Arbaziloth lever");
    if matches!(case, Scenario::AdmissionAckLost) {
        assert!(!owner.states.quest_callbacks.lose_source_boss_commit_ack.load(Ordering::Acquire));
        let cooldowns = active_cooldowns(h, lever).await?;
        assert_eq!(cooldowns.len(), 2);
        assert_eq!(cooldowns[0].1, cooldowns[1].1, "replay retains the same paid admission");
        assert_eq!(cooldowns[0].2, cooldowns[1].2, "replay retains both deadlines");
        let admission = cooldowns[0].1.as_slice().try_into()?;
        assert_paid_receipt(h, &admission).await?;
    }
    if matches!(case, Scenario::HardWin) {
        // Complete the prior win's persisted count after admitting the current
        // fight. The following real lethal settlement must choose the unlock.
        let (_, saved_revision) =
            super::qengine_outfit_stories_fixtures::seed_quest_revision(owner, other_session, 112).await?;
        let mut tx = h.pool.begin().await?;
        sqlx::query("SET LOCAL session_replication_role = replica").execute(&mut *tx).await?;
        let changed = sqlx::query(
            "UPDATE game_character_quest_tracks t SET value=9, committed_character_revision=r.character_revision
               FROM game_character_roots r WHERE t.character_id=r.character_id
                 AND t.character_id=encode($1,'hex')::uuid AND t.track_key=$2
                 AND r.character_revision=$3::text::numeric AND t.value=8",
        )
        .bind(second_lease.character_id().as_bytes().as_slice())
        .bind(format!("{WICKED_TRACK}/hard_kills"))
        .bind(saved_revision.to_string())
        .execute(&mut *tx).await?;
        assert_eq!(changed.rows_affected(), 1);
        tx.commit().await?;
    }
    let rod = oteryn_gameplay_items::fishing::FISHING_ROD;
    if matches!(case, Scenario::HardWin) {
        super::item_look_fixtures::seed_main_backpack(h, &[(rod, 1)]).await?;
    }
    let queue = owner.states.attack.lock().await.kills().clone();
    let mut runtime = owner.runtime.lock().await;
    let mut states = owner.states.spell_states.lock().await;
    for (actor, _) in players {
        assert_eq!(runtime.read_actor_position(actor).map_err(debug)?.position(), ARENA_ENTRY);
    }
    let census = runtime.positioned_actor_census().map_err(debug)?;
    let spawned = |key: &str| {
        census.iter().find_map(|(actor, _, _)| {
            runtime
                .matches_live_creature_identity(*actor, key.as_bytes())
                .then_some(*actor)
        })
    };
    let boss = spawned(BOSS).ok_or("normal Arbaziloth spawned by lever")?;
    let forgemaster = spawned(FORGEMASTER).ok_or("Forgemaster spawned by lever")?;
    assert_eq!(
        runtime.companion_snapshot(forgemaster).map_err(debug)?.health,
        10_500
    );
    if matches!(case, Scenario::PaidAfterPublication) {
        assert_eq!(states.encounter_phases.active.lock().map_err(|_| "phases")?.len(), 1);
        let key = bundle::placement_key(LEVER.floor, LEVER.x, LEVER.y, 2).ok_or("lever key")?;
        assert_eq!(crate::map::view::MapFacts::object_revision(facts, key), 1);
        drop(states);
        drop(runtime);
        let peer = owner.current_quest_fence(other_session).await
            .map_err(|()| "published peer fence")?.ok_or("missing published peer fence")?;
        return assert_paid_handoff(h, deps, owner, lever, vec![fence.item_fence(), peer.item_fence()]).await;
    }
    if matches!(case, Scenario::AdmissionAckLost) {
        assert_eq!(states.encounter_phases.active.lock().map_err(|_| "phases")?.len(), 1);
        return Ok(());
    }
    if matches!(case, Scenario::FireDeadline) {
        let raised_at = owner.owner_ms() * 1_000;
        raise_shield(&mut runtime, boss, raised_at, b"nrwicked-owner-fire")?;
        drop(states);
        drop(runtime);
        let mut states = owner.states.spell_states.lock().await;
        let first_ai_pass = raised_at + 1_000_000;
        states.next_monster_ai_pass_us = first_ai_pass;
        // The encounter's next observation precedes the next melee swing pass.
        states.monster_melee.next_swing_pass_us = first_ai_pass + 1_000_000;
        drop(states);
        use super::srx3_native_source_cast_fixtures::{
            drain_ready_monster_melee_at, settle_monster_melee_at_until,
        };
        assert!(
            settle_monster_melee_at_until(owner, first_ai_pass, async || {
                owner
                    .states
                    .spell_states
                    .lock()
                    .await
                    .next_monster_ai_pass_us
                    > first_ai_pass
            })
            .await?,
            "the first periodic encounter turn must run"
        );
        let fire_at = raised_at + 1_500_000;
        {
            let runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            assert!(states.next_monster_ai_pass_us > fire_at);
            assert_eq!(states.next_monster_ai_due_us(&runtime), fire_at);
        }
        drain_ready_monster_melee_at(owner, fire_at - 1).await?;
        assert_eq!(
            purple_fire_count(facts),
            0,
            "fire waits for the shield deadline"
        );
        drain_ready_monster_melee_at(owner, fire_at).await?;
        assert_eq!(
            purple_fire_count(facts),
            1,
            "the owner publishes fire between AI passes"
        );
        return Ok(());
    }
    if matches!(case, Scenario::ReconnectGrace) {
        let closed = crate::map::floor::BundleDoors(facts);
        let previous = runtime.read_actor_position(other).map_err(debug)?;
        let staging = MovementLocalPosition { x: i32::from(STAGING.x), y: i32::from(STAGING.y) + 1, floor: 14 };
        let proof = crate::map::floor::prepare_quest_callback_teleport(
            &runtime, owner.movement_cells(), other, other_session, previous, staging, false, &closed,
        ).map_err(debug)?;
        runtime.commit_quest_relocation(vec![proof], || true).map_err(debug)?;
        drop(states);
        drop(runtime);
        return exercise_reconnect_grace(h, owner, admitted, boss, forgemaster).await;
    }
    let boss_home = runtime.read_actor_position(boss).map_err(debug)?.position();
    let purple_fire = || {
        facts
            .movement_item_entries()
            .into_iter()
            .filter_map(|(at, entry)| (entry.facts.appearance_id == 49362).then_some(at))
            .collect::<Vec<_>>()
    };
    let mut rod_command_id = 100;
    for phase in 0..4 {
        let raised_at = owner.owner_ms() * 1_000;
        raise_shield(
            &mut runtime,
            boss,
            raised_at,
            format!("nrwicked-party-{phase}").as_bytes(),
        )?;
        encounter_phases::tick(&mut runtime, &states, raised_at);
        assert!(
            purple_fire().is_empty(),
            "the first tick precedes the fire deadline"
        );
        encounter_phases::tick(&mut runtime, &states, raised_at + 1_499_000);
        assert!(purple_fire().is_empty(), "fire waits for the source delay");
        let fire_at = raised_at + 1_500_000;
        encounter_phases::tick(&mut runtime, &states, fire_at);
        let [fire]: [TilePos; 1] = purple_fire()
            .try_into()
            .map_err(|_| "expected exactly one production purple fire per shield")?;
        if matches!(case, Scenario::Abandonment) {
            let expires = owner.owner_now().get() + 20 * 60 * 1_000_000;
            encounter_phases::tick(&mut runtime, &states, expires);
            assert!(!runtime.contains_live_creature(boss));
            assert!(!runtime.contains_live_creature(forgemaster));
            assert!(purple_fire().is_empty());
            assert!(states.encounter_phases.active.lock().map_err(|_| "phases")?.is_empty());
            let exit = MovementLocalPosition { x: i32::from(ARENA_EXIT.x), y: i32::from(ARENA_EXIT.y), floor: 14 };
            for (actor, _) in players {
                assert_eq!(runtime.read_actor_position(actor).map_err(debug)?.position(), exit);
            }
            return_to_staging(owner, &mut runtime, facts, players)?;
            drop(states);
            drop(runtime);

            let cooldowns: Vec<i64> = sqlx::query_scalar(
                "SELECT available_at-floor(extract(epoch FROM clock_timestamp())*1000)::bigint
                   FROM game_character_boss_cooldowns WHERE boss_key='oteryn:boss.arbaziloth'",
            ).fetch_all(&h.pool).await?;
            assert_eq!(cooldowns.len(), 2);
            assert!(cooldowns.iter().all(|ms| (71_990_000..=72_000_000).contains(ms)));
            let key = bundle::placement_key(LEVER.floor, LEVER.x, LEVER.y, 2).ok_or("lever key")?;
            let revision = crate::map::view::MapFacts::object_revision(facts, key);
            let target = WorldObjectTarget { placement: lever_placement.clone(), expected_revision: revision };
            let retry = |nonce| UseCommand {
                game_session_id: session, command_id: nonce, item_fence: Some(fence.item_fence()),
            };
            let action_window = crate::gameplay_transport::map_door::WORLD_ACTION_COOLDOWN;
            owner.clock_origin = owner.clock_origin.checked_sub(action_window)
                .ok_or("Doomforge action clock")?;
            assert_eq!(owner.use_object(first, retry(2), target.clone()).await.disposition,
                UseDisposition::Rejected, "the cooldown applies to a fresh lever use after the room clears");
            sqlx::query("UPDATE game_character_boss_cooldowns SET available_at=1 WHERE boss_key='oteryn:boss.arbaziloth'")
                .execute(&h.pool).await?;
            owner.clock_origin = owner.clock_origin.checked_sub(action_window)
                .ok_or("Doomforge action clock")?;
            assert_eq!(owner.use_object(first, retry(3), target).await.disposition,
                UseDisposition::Committed, "the encounter registry released the expired attempt");
            let mut runtime = owner.runtime.lock().await;
            let states = owner.states.spell_states.lock().await;
            return_to_staging(owner, &mut runtime, facts, players)?;
            encounter_phases::tick(&mut runtime, &states, owner.owner_now().get());
            assert!(states.encounter_phases.active.lock().map_err(|_| "phases")?.is_empty(),
                "leaving the room retires the next attempt before its deadline");
            assert!(!runtime.positioned_actor_census().map_err(debug)?.into_iter().any(
                |(actor, _, _)| runtime.matches_live_creature_identity(actor, BOSS.as_bytes())
                    || runtime.matches_live_creature_identity(actor, FORGEMASTER.as_bytes()),
            ));
            return Ok(());
        }
        let before = runtime.read_actor_position(boss).map_err(debug)?;
        runtime
            .borrow_movement_position()
            .commit_creature_relocation(
                boss,
                before,
                MovementLocalPosition {
                    x: i32::from(fire.x),
                    y: i32::from(fire.y),
                    floor: -i16::from(fire.floor),
                },
            )
            .map_err(debug)?;
        encounter_phases::tick(&mut runtime, &states, fire_at);
        assert!(
            purple_fire().is_empty(),
            "boss crossing consumes purple fire"
        );

        // Keep the boss within far-USE range of the player at the arena entry.
        let before = runtime.read_actor_position(boss).map_err(debug)?;
        runtime
            .borrow_movement_position()
            .commit_creature_relocation(
                boss,
                before,
                MovementLocalPosition {
                    y: ARENA_ENTRY.y - 2,
                    ..ARENA_ENTRY
                },
            )
            .map_err(debug)?;
        let visible = runtime
            .visible_entities()
            .creatures
            .into_iter()
            .find(|entry| entry.actor == boss)
            .ok_or("visible Arbaziloth")?;
        let intent = oteryn_protocol_oteryn::world_object::UseIntent {
            target: oteryn_protocol_oteryn::world_object::UseTarget::Item(
                std::num::NonZeroU64::new(2).ok_or("rod handle")?,
            ),
            use_with: Some(oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
                identity: boss.placement_identity(),
                generation: visible.generation,
            }),
            use_with_map_item: None,
            use_with_position: None,
        };
        drop(states);
        drop(runtime);
        owner.clock_origin -= std::time::Duration::from_millis(1_500);
        let mut cracked = false;
        for _ in 0..16 {
            rod_command_id += 1;
            let outcome = owner
                .use_soul_war_source_items(
                    first,
                    UseCommand {
                        game_session_id: session,
                        command_id: rod_command_id,
                        item_fence: Some(fence.item_fence()),
                    },
                    intent.clone(),
                    Some(crate::gameplay_transport::item_view::ItemKey::Instance(id(
                        71,
                    ))),
                    None,
                )
                .await
                .map_err(|()| "rod use authority")?
                .ok_or("carried fishing rod use")?;
            match outcome.disposition {
                UseDisposition::Committed => {
                    cracked = true;
                    break;
                }
                UseDisposition::NotPossible => {
                    // The source 50% roll may refuse; advance the real item-use gate.
                    owner.clock_origin -= std::time::Duration::from_secs(1);
                }
                other => return Err(format!("rod use returned {other:?}").into()),
            }
        }
        assert!(cracked, "the carried fishing rod lowers the cracked shield");
        runtime = owner.runtime.lock().await;
        states = owner.states.spell_states.lock().await;
        let before = runtime.read_actor_position(boss).map_err(debug)?;
        runtime
            .borrow_movement_position()
            .commit_creature_relocation(boss, before, boss_home)
            .map_err(debug)?;
    }
    encounter_phases::tick(&mut runtime, &states, owner.owner_now().get() + 3_000_000);
    let enraged = runtime
        .positioned_actor_census()
        .map_err(debug)?
        .into_iter()
        .map(|(actor, _, _)| actor)
        .find(|actor| runtime.matches_live_creature_identity(*actor, ENRAGED.as_bytes()))
        .ok_or("enraged replacement")?;
    assert!(!runtime.contains_live_creature(boss));
    assert!(!runtime.contains_live_creature(forgemaster));
    let observed = owner.owner_now();
    let now = observed.get() / 1000;
    let health = runtime.companion_snapshot(enraged).map_err(debug)?.health;
    assert_eq!(health, 360_000);
    for (actor, session, damage) in [(other, other_session, health / 4), (first, session, health - health / 4)] {
        runtime
            .borrow_exact_actor_commit_at(now)
            .commit_damage_for_bound_attacker(
                enraged,
                actor,
                CommandRef::new(session, CommandId::new(1).map_err(debug)?),
                0,
                OwnerDamageCommand {
                    target: ENRAGED.as_bytes(),
                    occurrence: session.as_bytes(),
                    binding: b"nrwicked-admitted-damage",
                    damage,
                },
            )
            .map_err(debug)?;
    }
    for _ in 0..2 {
        record_creature_kill(
            &mut runtime,
            KillSink {
                prey: Some(&states),
                activity: &[],
                queue: &queue,
                rewards: native.reward_table(),
                charms: None,
            },
            enraged,
            crate::combat::CapturedRewardPrincipal {
                character: first_lease.character_id(),
                lease_generation: first_lease.generation(),
                session,
                actor: first,
            },
            now,
            Some(observed),
        );
    }
    drop(states);
    drop(runtime);
    assert_eq!(
        owner.drain_kill_rewards(session).await,
        DrainOutcome::Drained
    );
    let base_xp: i64 = sqlx::query_scalar(
        "SELECT r.base_xp FROM game_party_kill_xp_receipts r
          JOIN game_party_kill_xp_members m USING (death_key)
          JOIN game_character_xp_receipts x USING (reward_occurrence_id)
          WHERE r.creature_key=$1 AND m.character_id=encode($2,'hex')::uuid",
    )
        .bind(ENRAGED)
        .bind(first_lease.character_id().as_bytes().as_slice())
        .fetch_one(&h.pool).await?;
    assert_eq!(base_xp, 500_000);
    let corpse: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_item_mint_receipts r
          JOIN game_item_instances i USING (item_instance_id)
          WHERE i.definition_production_key='oteryn:item.tibia.i50029' AND r.materialized_at IS NOT NULL",
    ).fetch_one(&h.pool).await?;
    assert_eq!(corpse, 1);
    let paid_coin_loot: i64 = sqlx::query_scalar(
        "SELECT count(DISTINCT definition_production_key) FROM game_boss_reward_items
          WHERE definition_production_key IN ('oteryn:item.tibia.i3043','oteryn:item.tibia.i3035')",
    ).fetch_one(&h.pool).await?;
    assert_eq!(paid_coin_loot, 2, "the hard-mode coin rows are settled into private loot");
    for (lease, hard_kills, door_stage) in [(first_lease, 1, 1), (second_lease, 10, 2)] {
        let character = oteryn_gameplay_domain::domain::CharacterId::from_bytes(
            *lease.character_id().as_bytes(),
        )
        .map_err(debug)?;
        let rows = owner
            .root
            .read_character_quest_state(owner.character, character)
            .await
            .map_err(debug)?;
        assert_eq!(
            rows.tracks().get(&format!("{WICKED_TRACK}/hard_kills")),
            Some(&hard_kills)
        );
        assert_eq!(
            rows.tracks()
                .get(&format!("{WICKED_TRACK}/kill_arbaziloth/questline")),
            Some(&2)
        );
        assert_eq!(
            rows.tracks().get(&format!("{WICKED_TRACK}/questlog")),
            Some(&1)
        );
        assert_eq!(
            rows.tracks().get(&format!("{WICKED_TRACK}/forgemaster_door/questline")),
            Some(&door_stage)
        );
    }
    {
        let mut runtime = owner.runtime.lock().await;
        let states = owner.states.spell_states.lock().await;
        encounter_phases::tick(&mut runtime, &states, owner.owner_now().get() + 20 * 60 * 1_000_000);
        for (actor, _) in players {
            assert_eq!(runtime.read_actor_position(actor).map_err(debug)?.position(), ARENA_ENTRY);
        }
    }
    for (actor, session) in players {
        for direction in std::iter::repeat_n(StepDirection::East, 6)
            .chain(std::iter::repeat_n(StepDirection::North, 10))
            .chain(std::iter::once(StepDirection::East))
        {
            owner.clock_origin = owner.clock_origin.checked_sub(std::time::Duration::from_secs(1))
                .ok_or("Doomforge movement clock")?;
            let (outcome, _) = owner.paced_step(actor, session, direction).await;
            assert_eq!(outcome.disposition, StepDisposition::Moved);
        }
        let at = owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position();
        assert_eq!(
            (at.x, at.y, at.floor),
            (i32::from(FORGE.x), i32::from(FORGE.y), 14)
        );
    }
    Ok(())
}

fn return_to_staging(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    runtime: &mut crate::foundation::ChannelRuntimeV1,
    facts: &crate::map::facts::BundleFacts,
    players: [(crate::foundation::ExactActorRef, crate::foundation::GameSessionId); 2],
) -> TestResult {
    let closed = crate::map::floor::BundleDoors(facts);
    let mut proofs = Vec::new();
    for ((actor, session), y) in players.into_iter().zip(STAGING.y..=STAGING.y + 1) {
        let previous = runtime.read_actor_position(actor).map_err(debug)?;
        let staging = MovementLocalPosition { x: i32::from(STAGING.x), y: i32::from(y), floor: 14 };
        proofs.push(crate::map::floor::prepare_quest_callback_teleport(
            runtime, owner.movement_cells(), actor, session, previous, staging, false, &closed,
        ).map_err(debug)?);
    }
    runtime.commit_quest_relocation(proofs, || true).map_err(debug)?;
    Ok(())
}


fn raise_shield(
    runtime: &mut crate::foundation::ChannelRuntimeV1,
    boss: crate::foundation::ExactActorRef,
    raised_at_us: u64,
    occurrence: &[u8],
) -> TestResult {
    let health = runtime.companion_snapshot(boss).map_err(debug)?.health;
    let binding = [occurrence, b"\0damage"].concat();
    runtime
        .borrow_exact_actor_commit_at(raised_at_us / 1_000)
        .commit_damage(
            boss,
            crate::foundation::OwnerDamageCommand {
                target: b"oteryn:creature.arbaziloth",
                occurrence,
                binding: &binding,
                damage: health,
            },
        )
        .map_err(debug)?;
    assert!(
        runtime
            .arbaziloth_shields(boss)
            .is_some_and(|shield| shield.needs_fire())
    );
    Ok(())
}

fn purple_fire_count(facts: &oteryn_gameplay_world::map::facts::BundleFacts) -> usize {
    facts
        .movement_item_entries()
        .into_iter()
        .filter(|(_, entry)| entry.facts.appearance_id == 49362)
        .count()
}

async fn exercise_reconnect_grace(
    h: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    mut admitted: super::AdmittedSession,
    boss: crate::foundation::ExactActorRef,
    forgemaster: crate::foundation::ExactActorRef,
) -> TestResult {
    use crate::foundation::{AuthenticatedTransportRefV1, ControlLossMark, GameSessionState};
    use crate::gameplay_transport::{
        connection::{ControlLossResult, FirstEntryOutcome, GraceExpiryResult, ResumeAttempt},
        resume::PRE_RESTORE_TEST_BARRIER,
    };
    use std::sync::Arc;
    use std::time::Duration;

    let session = admitted.game_session_id;
    let actor = admitted.runtime_actor.ok_or("Arbaziloth player")?;
    let store = super::FreshAdmissionStore::from_root(h.root.clone());
    admitted.first_entry = FirstEntryOutcome::Positioned;
    admitted.item_fence =
        crate::gameplay_transport::item_fence_of(store.current_session_at(session).await?.0);
    owner.states.command_ingress.admit(session);
    assert_eq!(
        owner.lose_control(admitted, Duration::ZERO, None).await,
        ControlLossResult::Recorded
    );
    let mark = owner
        .runtime
        .lock()
        .await
        .player_control_facts(actor, session)
        .map_err(debug)?
        .control_loss
        .ok_or("Arbaziloth retained loss")?;
    assert!(owner.runtime.lock().await.contains_live_creature(boss));

    let token = super::house_transfer_fixtures::RecoverySource::token(93)?;
    let barrier = Arc::new((tokio::sync::Notify::new(), tokio::sync::Notify::new()));
    let resuming = PRE_RESTORE_TEST_BARRIER.scope(
        barrier.clone(),
        owner.resume(ResumeAttempt {
            game_session_id: session,
            recovery_material: token.as_bytes(),
            transport: AuthenticatedTransportRefV1::decode(&[93; 16]).map_err(debug)?,
            last_applied_server_sequence: 0,
            supported_capabilities: &[],
        }),
    );
    tokio::pin!(resuming);
    tokio::select! {
        result = &mut resuming => return Err(format!("reconnect ended before native restore: {result:?}").into()),
        () = barrier.0.notified() => {}
    }
    assert_eq!(
        store.current_session_at(session).await?.0.session_state(),
        GameSessionState::Active
    );
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .player_control_facts(actor, session)
            .map_err(debug)?
            .control_loss,
        Some(mark)
    );
    let facts = &owner.bundle_map().ok_or("Doomforge AI bundle")?.facts;
    let purple_fire = || purple_fire_count(facts);
    let raised_at = owner.owner_ms() * 1_000;
    {
        let mut runtime = owner.runtime.lock().await;
        raise_shield(&mut runtime, boss, raised_at, b"nrwicked-reconnect-shield")?;
    }
    assert_eq!(purple_fire(), 0);
    // Run both owner deadlines across the original grace boundary while the
    // committed resume is paused before its native restore.
    let before = super::HOST_NOW.replace(Some(mark.grace_deadline));
    let ai: TestResult = async {
        use super::srx3_native_source_cast_fixtures::settle_monster_melee_at;
        settle_monster_melee_at(owner, raised_at).await?;
        assert_eq!(purple_fire(), 0);
        settle_monster_melee_at(owner, raised_at + 1_500_000).await?;
        assert_eq!(
            purple_fire(),
            1,
            "the owner projects fire while restore is paused"
        );
        let runtime = owner.runtime.lock().await;
        assert_eq!(
            runtime
                .player_control_facts(actor, session)
                .map_err(debug)?
                .control_loss,
            Some(mark)
        );
        Ok(())
    }
    .await;
    super::HOST_NOW.set(before);
    barrier.1.notify_one();
    let resumed = resuming.await.map_err(debug)?;
    ai?;
    {
        let runtime = owner.runtime.lock().await;
        assert!(runtime.contains_live_creature(boss));
        assert_eq!(
            runtime
                .read_actor_position(actor)
                .map_err(debug)?
                .position(),
            ARENA_ENTRY
        );
        assert_eq!(
            runtime
                .player_control_facts(actor, session)
                .map_err(debug)?
                .control_loss,
            None
        );
    }

    // Import an already-expired, unrecovered loss. The real session owner must
    // commit release and remove the exact participant before the AI retires the room.
    let deadline: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint-1")
            .fetch_one(&h.pool)
            .await?;
    let epoch = mark.epoch + 1;
    let lost = sqlx::query(
        "UPDATE game_durability_reconnect_sessions
            SET session_state=1,current_transport_ref=NULL,predecessor_generation=current_generation,
                control_loss_epoch=$2::text::numeric,original_grace_deadline=$3,attempt_count=0
          WHERE game_session_id=encode($1,'hex')::uuid AND session_state=2",
    ).bind(session.as_bytes().as_slice()).bind(epoch.to_string()).bind(deadline)
        .execute(&h.pool).await?;
    assert_eq!(lost.rows_affected(), 1);
    owner
        .runtime
        .lock()
        .await
        .record_control_loss(
            actor,
            session,
            ControlLossMark {
                epoch,
                grace_deadline: deadline,
            },
        )
        .map_err(debug)?;
    assert_eq!(
        owner.release_after_grace(resumed).await,
        GraceExpiryResult::Released
    );
    drive_encounter_ai_until(owner, async || {
        !owner.runtime.lock().await.contains_live_creature(boss)
    }).await?;
    assert_eq!(purple_fire(), 0, "the post-release AI phase tick cleared the fight");
    let runtime = owner.runtime.lock().await;
    assert!(!runtime.contains_live_creature(boss));
    assert!(!runtime.contains_live_creature(forgemaster));
    assert!(
        owner
            .states
            .spell_states
            .lock()
            .await
            .encounter_phases
            .active
            .lock()
            .map_err(|_| "phases")?
            .is_empty()
    );
    Ok(())
}

async fn drive_encounter_ai_until(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    phase_changed: impl AsyncFnMut() -> bool,
) -> TestResult {
    let room = owner.rooms().gate_monster_ai().ok_or("Doomforge AI gate")?;
    assert!(matches!(room.movement_cells().index(),
        crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(_)));
    let due = owner.states.spell_states.lock().await.next_monster_ai_pass_us;
    assert!(
        super::srx3_native_source_cast_fixtures::settle_monster_melee_at_until(
            owner, owner.owner_now().get().max(due), phase_changed,
        ).await?,
        "the qualified AI turn did not apply the expected encounter phase"
    );
    Ok(())
}
