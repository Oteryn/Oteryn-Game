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
                super::appearance_change_postgres::observer_visibility_tests::admit(&h).await?;
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
            let room = qualify_native_entry_room(world).map_err(debug)?;
            let (bytes, pins) = portal_fixture(destination)?;
            let map = boot(&bytes, &pins, world, channel, |key| {
                let mut definition = fixture::floor_items("oteryn:item.tibia.i37001")?;
                match key {
                    "oteryn:item.tibia.i37000" => Some(definition),
                    "oteryn:item.tibia.i49362" => {
                        definition.reference = std::num::NonZeroU32::new(5)?;
                        Some(definition)
                    }
                    _ => None,
                }
            })
            .map_err(debug)?;
            assert_eq!(map.floors().teleport(PORTAL), Some(destination));
            let cells = map.movement_cells(room.movement_cells()).map_err(debug)?;
            let pin = ChannelContentPin::from_activation(
                world,
                1,
                room.compiled().server_digest(),
                room.compiled().client_digest(),
                map.frame_binding_digest(world),
                map.map_revision_digest(),
                (i32::from(START.x), i32::from(START.y), 14),
            );
            let deps = Dependencies::with_content_and_capacity(
                pin,
                room,
                crate::spell::SpellBook::new(vec![]).map_err(debug)?,
                if hard_win { 16 } else { 2 },
            )?;
            let mut owner = deps.owner(&h, &authority);
            owner.movement_cells = (&cells).into();
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
                two_player_exit(&h, &owner, deps.actor, session, &cells).await?;
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

fn portal_fixture(destination: TilePos) -> TestResult<(Vec<u8>, crate::map::boot::BootPins)> {
    let (bytes, mut pins) = fixture::teleport_bundle();
    let mut manifest = bundle::read(&bytes)?.manifest;
    manifest.world = bundle::Extent {
        min_x: START.x - 3,
        min_y: START.y,
        max_x: destination.x + 1,
        max_y: destination.y + 1,
        floors: vec![START.floor],
    };
    let mut portal = manifest.palette.pop().ok_or("portal palette")?;
    portal.key = "oteryn:item.tibia.i37000".into();
    let mut fire = portal.clone();
    fire.key = "oteryn:item.tibia.i49362".into();
    fire.id = portal.id + 1;
    manifest.palette.truncate(1);
    manifest.palette.push(portal);
    manifest.palette.push(fire);
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
    let sectors = [
        bundle::Sector {
            floor: START.floor,
            sx: START.x / 32,
            sy: START.y / 32,
            tiles: vec![
                tile(TilePos {
                    x: START.x - 3,
                    ..START
                }),
                tile(TilePos {
                    x: START.x - 2,
                    ..START
                }),
                tile(TilePos {
                    x: START.x - 1,
                    ..START
                }),
                tile(START),
                entrance,
            ],
        },
        bundle::Sector {
            floor: FORGE.floor,
            sx: destination.x / 32,
            sy: destination.y / 32,
            tiles: vec![tile(destination)],
        },
    ];
    let bytes = writer::write(&manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let mut pins = fixture::floor_pins(pins);
    pins.start = START;
    Ok((bytes, pins))
}

async fn two_player_exit(
    h: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    first: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
    cells: &crate::content::NativeEntryMovementCells,
) -> TestResult {
    use super::fixture::id;
    use crate::foundation::{CommandId, CommandRef, GameSessionId, OwnerDamageCommand};
    use crate::gameplay_transport::{
        encounter_phases, grizzly_kills,
        kill_reward::{DrainOutcome, KillSink, record_creature_kill},
    };
    use crate::map::floor::{BundleDoors, prepare_quest_encounter_member};
    use oteryn_gameplay_encounters::bosses::phase_controller::arbaziloth::{BOSS, FORGEMASTER};
    use oteryn_gameplay_quests::quest::world_callbacks::{
        Encounter, EncounterMonster, EncounterPhase, EncounterPlayer, Position,
    };
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
    let (controller, _) =
        super::death_ml_loss_composed::production_generation_with_pin(owner.world_id, "content-1")?;
    let native = controller
        .active()
        .and_then(|g| g.native_gameplay())
        .ok_or("native policies")?;
    let original = native
        .policies()
        .policies()
        .iter()
        .find(|p| p.definition_key == BOSS)
        .ok_or("Arbaziloth policy")?;
    let mut runtime = owner.runtime.lock().await;
    let digest = runtime.content_pin().server_artifact_digest();
    let policies = [BOSS, FORGEMASTER, ENRAGED].map(|key| {
        let mut policy = (**original).clone();
        policy.definition_key = key.into();
        policy.maximum_health = 100;
        policy
    });
    runtime
        .install_companion_policies(
            crate::content::creature_policy::CompiledCreaturePolicies::from_active_artifact(
                digest,
                Vec::from(policies),
            )
            .map_err(debug)?,
        )
        .map_err(debug)?;
    let reservation = runtime
        .reserve_fresh_session(other_session)
        .map_err(debug)?;
    let other = runtime.commit_fresh_session(reservation).map_err(debug)?;
    let start = MovementLocalPosition {
        x: i32::from(START.x),
        y: i32::from(START.y),
        floor: 14,
    };
    runtime
        .initialize_pinned_test_position(
            other,
            MovementLocalPosition {
                x: start.x - 1,
                ..start
            },
        )
        .map_err(debug)?;
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
    let queue = owner.states.attack.lock().await.kills().clone();
    let mut runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    use oteryn_gameplay_world::map::boot::BundleCollisionIndexExt;
    let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
        cells.index()
    else {
        return Err("bundle arena cells".into());
    };
    let doors = BundleDoors(&index.map().facts);
    let proofs = players
        .iter()
        .map(|&(actor, session)| {
            let original = runtime.read_actor_position(actor).map_err(debug)?;
            prepare_quest_encounter_member(
                &runtime, cells, actor, session, original, start, false, &doors, true,
            )
            .map_err(debug)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let position = |x| Position {
        x,
        y: START.y,
        floor: START.floor,
    };
    let encounter = Encounter {
        minimum_level: 0,
        players: vec![
            EncounterPlayer {
                from: position(START.x),
                to: position(START.x),
            },
            EncounterPlayer {
                from: position(START.x - 1),
                to: position(START.x),
            },
        ],
        monsters: vec![
            EncounterMonster {
                creature: BOSS.into(),
                position: position(START.x - 2),
            },
            EncounterMonster {
                creature: FORGEMASTER.into(),
                position: position(START.x - 3),
            },
        ],
        room_min: position(START.x - 3),
        room_max: position(PORTAL.x),
        timing: None,
        phase: Some(EncounterPhase::Arbaziloth),
    };
    let credits =
        grizzly_kills::capture_encounter_admission(owner, &runtime, &players).map_err(debug)?;
    let mut active = states
        .encounter_phases
        .reserve(&mut runtime, 1)
        .map_err(debug)?;
    let (entered, (phase, boss)) = runtime
        .commit_quest_encounter_with_phase(
            proofs,
            &[
                (
                    BOSS,
                    100,
                    MovementLocalPosition {
                        x: start.x - 2,
                        ..start
                    },
                ),
                (
                    FORGEMASTER,
                    100,
                    MovementLocalPosition {
                        x: start.x - 3,
                        ..start
                    },
                ),
            ],
            |runtime, actors| {
                Ok((
                    encounter_phases::bind(
                        runtime,
                        &encounter,
                        1,
                        actors,
                        &players,
                        credits,
                        cells.clone(),
                    )?,
                    actors[0],
                ))
            },
            || true,
        )
        .map_err(debug)?;
    active.push(phase);
    drop(active);
    assert_eq!(entered.len(), 2);
    assert!(entered.iter().all(|snapshot| snapshot.position() == start));
    for (phase, health) in [100, 80, 60, 40].into_iter().enumerate() {
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
        let shields = runtime
            .arbaziloth_shields_mut(boss)
            .ok_or("bound shields")?;
        assert!(shields.step_on_fire());
        assert!(shields.use_rod(100, 100));
    }
    encounter_phases::tick(&mut runtime, &states, 3100);
    let enraged = runtime
        .positioned_actor_census()
        .map_err(debug)?
        .into_iter()
        .map(|(actor, _, _)| actor)
        .find(|actor| runtime.matches_live_creature_identity(*actor, ENRAGED.as_bytes()))
        .ok_or("enraged replacement")?;
    let observed = owner.owner_now();
    let now = observed.get() / 1000;
    for (actor, session, damage) in [(other, other_session, 25), (first, session, 75)] {
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
    for lease in [first_lease, second_lease] {
        let character = oteryn_gameplay_domain::domain::CharacterId::from_bytes(
            *lease.character_id().as_bytes(),
        )
        .map_err(debug)?;
        let rows = owner
            .root
            .read_character_quest_state(owner.character, character)
            .await
            .map_err(debug)?;
        let track = "oteryn:quest-progress/crystalserver/quest/u14_10/no_rest_for_the_wicked";
        assert_eq!(rows.tracks().get(&format!("{track}/hard_kills")), Some(&1));
        assert_eq!(
            rows.tracks()
                .get(&format!("{track}/kill_arbaziloth/questline")),
            Some(&2)
        );
        assert_eq!(rows.tracks().get(&format!("{track}/questlog")), Some(&1));
        assert_eq!(
            rows.tracks().get(&format!("{track}/forgemaster_door/questline")),
            Some(&1)
        );
    }
    for (actor, session) in players {
        let (outcome, _) = owner.paced_step(actor, session, StepDirection::East).await;
        assert_eq!(outcome.disposition, StepDisposition::Moved);
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
