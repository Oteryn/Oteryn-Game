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
    run_case(admin, "nrwicked_party", FORGE, true)
}

fn run(admin: String, fixture_name: &'static str, destination: TilePos) -> TestResult {
    run_case(admin, fixture_name, destination, false)
}

fn run_case(
    admin: String,
    fixture_name: &'static str,
    destination: TilePos,
    hard_win: bool,
) -> TestResult {
    let started = std::time::Instant::now();
    let result = std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            scenario(admin, fixture_name, destination, hard_win).map_err(|error| error.to_string())
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
    hard_win: bool,
) -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async move {
            let h = Harness::create_with_admission_seed(admin, fixture_name, true, false).await?;
            prepare_current_session(&h).await?;
            if hard_win {
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
            let production = if hard_win {
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
            let spawn = if hard_win { STAGING } else { START };
            let pin = ChannelContentPin::from_activation(
                world,
                1,
                room.compiled().server_digest(),
                room.compiled().client_digest(),
                map.frame_binding_digest(world),
                map.map_revision_digest(),
                (i32::from(spawn.x), i32::from(spawn.y), 14),
            );
            let deps = Dependencies::with_content_and_capacity(
                pin,
                room,
                crate::spell::SpellBook::new(vec![]).map_err(debug)?,
                if hard_win { 16 } else { 2 },
            )?;
            let mut owner = deps.owner(&h, &authority);
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
            if hard_win {
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
                two_player_exit(&h, &mut owner, deps.actor, session, lever_placement, map.facts())
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
        max_y: if native.is_some() { STAGING.y + 5 } else { destination.y + 1 },
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

async fn two_player_exit(
    h: &Harness,
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    first: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
    lever_placement: Vec<u8>,
    facts: &crate::map::facts::BundleFacts,
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
    let other_session = GameSessionId::decode(&id(110)).map_err(debug)?;
    let admissions =
        crate::durability::fresh_admission::FreshAdmissionStore::from_root(h.root.clone());
    let first_lease = admissions
        .current_session_at(session)
        .await?
        .0
        .current_character_lease();
    let second_lease = admissions
        .current_session_at(other_session)
        .await?
        .0
        .current_character_lease();
    // The second player starts one hard win away from qualifying for the key.
    // Imported progress lands at the character's current committed revision,
    // as the other quest fixtures seed it: bumping the revision without a
    // receipt breaks the progression receipt chain at the next commit.
    let mut import = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role = replica")
        .execute(&mut *import)
        .await?;
    for (track, value) in [
        ("hard_kills", 9_i64),
        ("kill_arbaziloth/questline", 2),
        ("questlog", 1),
        ("forgemaster_door/questline", 1),
    ] {
        // The stored quest owner must match the catalogue's track owner, or
        // the kill commit rejects the row as InvalidStoredState.
        let key = format!("{WICKED_TRACK}/{track}");
        let definition = owner
            .quest_catalogue
            .as_ref()
            .ok_or("wicked catalogue")?
            .track(&key)
            .ok_or("canonical wicked track")?;
        assert!((definition.min..=definition.max).contains(&value));
        let quest = definition.quest.clone();
        let inserted = sqlx::query(
            "INSERT INTO game_character_quest_tracks
                (character_id, track_key, quest_key, value, committed_character_revision)
             SELECT character_id, $2, $3, $4, character_revision
             FROM game_character_roots WHERE character_id = encode($1, 'hex')::uuid",
        )
        .bind(second_lease.character_id().as_bytes().as_slice())
        .bind(key)
        .bind(quest)
        .bind(value)
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
    let outcome = owner.use_object(first, UseCommand {
        game_session_id: session, command_id: 1, item_fence: Some(fence.item_fence()),
    }, WorldObjectTarget { placement: lever_placement, expected_revision: 0 }).await;
    assert_eq!(outcome.disposition, UseDisposition::Committed, "production Arbaziloth lever");
    let queue = owner.states.attack.lock().await.kills().clone();
    let mut runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
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
    let boss_home = runtime.read_actor_position(boss).map_err(debug)?.position();
    let purple_fire = || {
        facts
            .movement_item_entries()
            .into_iter()
            .filter_map(|(at, entry)| (entry.facts.appearance_id == 49362).then_some(at))
            .collect::<Vec<_>>()
    };
    for phase in 0..4 {
        let health = runtime.companion_snapshot(boss).map_err(debug)?.health;
        let occurrence = format!("nrwicked-party-{phase}");
        let binding = format!("{occurrence}\0damage");
        runtime
            .borrow_exact_actor_commit()
            .commit_damage(
                boss,
                OwnerDamageCommand {
                    target: BOSS.as_bytes(),
                    occurrence: occurrence.as_bytes(),
                    binding: binding.as_bytes(),
                    damage: health,
                },
            )
            .map_err(debug)?;
        encounter_phases::tick(&mut runtime, &states, 100_000);
        let [fire]: [TilePos; 1] = purple_fire()
            .try_into()
            .map_err(|_| "expected exactly one production purple fire per shield")?;
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
        encounter_phases::tick(&mut runtime, &states, 100_000);
        assert!(purple_fire().is_empty(), "boss crossing consumes purple fire");
        assert!(
            runtime
                .arbaziloth_shields_mut(boss)
                .ok_or("bound shields")?
                .use_rod(100, 100)
        );
        let before = runtime.read_actor_position(boss).map_err(debug)?;
        runtime
            .borrow_movement_position()
            .commit_creature_relocation(boss, before, boss_home)
            .map_err(debug)?;
    }
    encounter_phases::tick(&mut runtime, &states, 3_100_000);
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
