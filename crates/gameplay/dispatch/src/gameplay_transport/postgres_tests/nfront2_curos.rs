//! The New Frontier arenas on the normal STEP, USE and native death owners.
use super::fixture::{Harness, debug, id, run};
use super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session,
    reset,
};
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, GameSessionId, MovementLocalPosition};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use oteryn_gameplay_quests::quest::{
    QuestComparison, QuestEffect, QuestEffectKind, QuestTransition, world_callbacks::Position,
};
use oteryn_protocol_oteryn::world_spatial::StepDirection;
use std::time::Duration;

use oteryn_gameplay_quests::quest::the_new_frontier::{
    MISSION09, MORTAL_REWARD_EXIT, MORTAL_TIMEOUT_EXIT, MORTAL_WAVES,
};

const CUROS_ORIGIN: Position = Position {
    x: 33145,
    y: 31413,
    floor: -6,
};
const MORTAL_ORIGIN: Position = Position {
    x: 33080,
    y: 31014,
    floor: -2,
};
const MORTAL_PARTNER: Position = Position {
    x: 33081,
    ..MORTAL_ORIGIN
};
const LEVER: Position = Position {
    x: 33079,
    ..MORTAL_ORIGIN
};
const FIRST_LANDING: Position = Position {
    x: 33059,
    y: 31032,
    floor: -3,
};
const SECOND_LANDING: Position = Position {
    x: 33057,
    y: 31034,
    floor: -3,
};
const WAVE_SITES: [Position; 4] = [
    Position {
        x: 33065,
        y: 31033,
        floor: -3,
    },
    Position {
        x: 33066,
        y: 31037,
        floor: -3,
    },
    Position {
        x: 33065,
        y: 31035,
        floor: -3,
    },
    Position {
        x: 33068,
        y: 31034,
        floor: -3,
    },
];
const START: Position = Position {
    x: 33146,
    y: 31413,
    floor: -6,
};
const ARENA: Position = Position {
    x: 33154,
    y: 31415,
    floor: -7,
};
const EXIT: Position = Position {
    x: 33145,
    y: 31419,
    floor: -7,
};
const CUROS_SPECTATOR: Position = Position { x: 33160, ..ARENA };
const SETUP: &str = "oteryn:quest-transition/nfront2-fixture-stage";
const QUEST: &str = "oteryn:quest.the_new_frontier_quest";
const QUESTLINE: &str = "oteryn:quest-progress/quest/u8_54/the_new_frontier/questline";
const MISSION: &str = "oteryn:quest-progress/quest/u8_54/the_new_frontier/mission06";

#[test]
fn nfront2_curos_step_starts_survival_and_awards_progress_only_at_deadline() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario(admin, false)
}

#[test]
fn nfront2_mortal_combat_pairs_players_spawns_waves_and_rewards_both_on_tirecz_death() -> TestResult
{
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    scenario(admin, true)
}

fn scenario(admin: String, mortal: bool) -> TestResult {
    let name = if mortal {
        "nfront2_mortal"
    } else {
        "nfront2_curos"
    };
    let origin = if mortal { MORTAL_ORIGIN } else { CUROS_ORIGIN };
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run(name, move || {
        run(async move |_| {
            reset();
            let h = Harness::create_with_admission_seed(admin, name, true, false).await?;
            prepare_current_session(&h).await?;
            let world = world(origin, mortal)?;
            let mut deps = Dependencies::new()?;
            let (controller, activated) =
                super::death_ml_loss_composed::production_generation_with_pin(
                    deps.admitted.world_id,
                    "content-1",
                )?;
            let generation = controller.active().ok_or("active generation")?;
            let native = generation.native_gameplay().ok_or("native profiles")?;
            let cells = world
                .movement_cells(activated.qualified_room().movement_cells())
                .map_err(debug)?;
            let frame = world.frame_binding_digest(deps.admitted.world_id);
            let room = activated
                .qualified_room()
                .bundle_view(
                    cells.clone(),
                    frame,
                    world.map_revision_digest(),
                    crate::content::NativeEntryStart {
                        x: i32::from(origin.x),
                        y: i32::from(origin.y),
                        floor: -i16::from(origin.floor),
                    },
                )
                .ok_or("bundle room")?;
            let pin = crate::content::channel_content_pin::ChannelContentPin::from_activation(
                deps.admitted.world_id,
                1,
                generation.identity().server_artifact_digest(),
                generation.identity().client_artifact_digest(),
                frame,
                world.map_revision_digest(),
                (
                    i32::from(origin.x),
                    i32::from(origin.y),
                    -i16::from(origin.floor),
                ),
            );
            let mut runtime = ChannelRuntimeV1::from_committed_assignment(
                deps.admitted.world_id,
                deps.admitted.channel_id,
                crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
                1,
                1,
                1,
                "runtime-scope-assignment:1",
                64,
                pin,
            )
            .map_err(debug)?;
            let reservation = runtime
                .reserve_fresh_session(deps.admitted.game_session_id)
                .map_err(debug)?;
            deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
            runtime
                .initialize_pinned_test_position(deps.actor, local(origin))
                .map_err(debug)?;
            deps.runtime = tokio::sync::Mutex::new(runtime);
            let seal = h.recovery.seal_current().map_err(debug)?;
            let authority = h
                .root
                .open_character_authority(&seal)
                .await
                .map_err(debug)?;
            let mut owner = deps.owner(&h, &authority);
            owner.active_generation = Some(generation);
            owner.movement_cells = (&cells).into();
            owner.rooms = crate::gameplay_transport::SeamRooms::new(Some(&room), Some(&room));
            owner.quest_catalogue = Some(std::sync::Arc::new(
                oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                    .map_err(debug)?
                    .catalogue()
                    .with_fixture_transition(QuestTransition {
                        key: SETUP.into(),
                        quest: QUEST.into(),
                        effects: vec![QuestEffect {
                            track: QUESTLINE.into(),
                            from: QuestComparison::Any,
                            effect: QuestEffectKind::Set(if mortal { 25 } else { 17 }),
                        }],
                        completes: false,
                        experience: None,
                        world_change: None,
                    })
                    .map_err(debug)?,
            ));
            let mut admitted = deps.admitted;
            admitted.runtime_actor = Some(deps.actor);
            admitted.first_entry = FirstEntryOutcome::Positioned;
            let fence = owner
                .current_quest_fence(admitted.game_session_id)
                .await
                .map_err(|()| "fence")?
                .ok_or("fence")?;
            admitted.item_fence = Some(fence.item_fence());
            sqlx::query("INSERT INTO game_control_scope_grants VALUES (session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
            sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
            .bind(id(42).as_slice()).bind(id(43).as_slice()).bind(generation.identity().server_artifact_digest().as_slice())
            .bind(generation.identity().client_artifact_digest().as_slice()).bind(frame.as_slice()).execute(&h.pool).await?;
            let fresh = super::fresh_fixture::fresh::Source::new(0)?.request()?;
            h.root
                .initialize_admission_equipment(
                    &authority,
                    &h.node,
                    &fence.item_fence(),
                    generation.identity().server_artifact_digest(),
                    fresh.operation(),
                )
                .await
                .map_err(debug)?;
            h.root
                .grant_starter_items(
                    &authority,
                    &h.node,
                    &fence.item_fence(),
                    &crate::gameplay_transport::item_ref_admission::NativeItemWeights(native),
                    &crate::durability::starter_grant::StarterTemplate {
                        definition: crate::durability::item_mint::TypedDefinitionRef {
                            family: "Item".into(),
                            production_key: "oteryn:item.tibia.i2854".into(),
                            revision_ref: "definition-r1".into(),
                        },
                        ..crate::durability::starter_grant::StarterTemplate::main()
                    },
                )
                .await
                .map_err(debug)?;
            // Replacing the fixture runtime does not transfer the admitted Character lease.
            owner
                .runtime
                .lock()
                .await
                .bind_attacker_lease(
                    deps.actor,
                    admitted.game_session_id,
                    crate::foundation::CharacterLease::new(
                        crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                            .map_err(debug)?,
                        fence.character_lease_generation,
                    )
                    .map_err(debug)?,
                )
                .map_err(debug)?;
            owner.admit_progression_session(&mut admitted).await;
            deps.initialize(&owner).await;
            owner.admit_quest_session(&admitted).await;

            let result = if mortal {
                Box::pin(mortal_combat(&mut owner, &deps, &admitted, &h, &world)).await
            } else {
                Box::pin(curos(&mut owner, &admitted, &h)).await
            };
            drop(owner);
            drop(authority);
            drop(seal);
            result?;
            h.cleanup().await
        })
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

async fn curos(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    h: &Harness,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("Curos actor")?;
    let session = admitted.game_session_id;
    let origin = local(CUROS_ORIGIN);
    owner.clock_origin -= Duration::from_secs(2);
    owner
        .ordinary_paced_step(actor, session, StepDirection::East, None, None)
        .await;
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        origin,
        "a player outside questline 17 returns to the origin"
    );
    seed_questline(owner, session).await?;
    let occupant = {
        let mut runtime = owner.runtime.lock().await;
        let session = GameSessionId::decode(&id(110)).map_err(debug)?;
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        let occupant = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(occupant, local(CUROS_SPECTATOR))
            .map_err(debug)?;
        occupant
    };
    owner.clock_origin -= Duration::from_secs(2);
    owner
        .ordinary_paced_step(actor, session, StepDirection::East, None, None)
        .await;
    assert_eq!(
        position(owner, actor).await?,
        origin,
        "a spectator at radius six occupies the room"
    );
    owner
        .runtime
        .lock()
        .await
        .remove_encounter_actor(occupant)
        .map_err(debug)?;
    owner.clock_origin -= Duration::from_secs(2);
    owner
        .ordinary_paced_step(actor, session, StepDirection::East, None, None)
        .await;
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        local(ARENA)
    );
    owner.drain_quest_encounter_deadlines().await;
    assert_eq!(progress(h, 41).await?, vec![(QUESTLINE.to_owned(), 17)]);
    owner.clock_origin -= Duration::from_secs(120);
    owner.drain_quest_encounter_deadlines().await;
    assert_eq!(
        owner
            .runtime
            .lock()
            .await
            .read_actor_position(actor)
            .map_err(debug)?
            .position(),
        local(EXIT)
    );
    let completed = vec![(MISSION.to_owned(), 3), (QUESTLINE.to_owned(), 18)];
    assert_eq!(progress(h, 41).await?, completed);
    owner.drain_quest_encounter_deadlines().await;
    assert_eq!(progress(h, 41).await?, completed);
    Ok(())
}

async fn seed_questline(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    session: GameSessionId,
) -> TestResult {
    use crate::durability::quest_state::{
        QuestCause, QuestTransitionOutcome, QuestTransitionRequest,
    };
    use crate::foundation::{CommandId, CommandRef};
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "seed fence")?
        .ok_or("seed session")?;
    let mut slot = owner.revision_sequencer.acquire(fence.character_id).await;
    let outcome = slot
        .commit_quest_transition(
            owner.root,
            owner.character,
            owner.holder,
            fence,
            QuestTransitionRequest {
                transition_key: SETUP.into(),
                cause: QuestCause::Command(CommandRef::new(
                    session,
                    CommandId::new(1000).map_err(debug)?,
                )),
            },
            owner.quest_catalogue.clone().ok_or("seed catalogue")?,
        )
        .await
        .map_err(debug)?;
    assert!(
        matches!(outcome, QuestTransitionOutcome::Committed(_)),
        "seed stage: {outcome:?}"
    );
    Ok(())
}

fn local(p: Position) -> MovementLocalPosition {
    MovementLocalPosition {
        x: i32::from(p.x),
        y: i32::from(p.y),
        floor: -i16::from(p.floor),
    }
}

fn world(origin: Position, mortal: bool) -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::{bundle, sector};
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette.truncate(1);
    let mut positions = if mortal {
        data.manifest.palette.push(bundle::PaletteEntry {
            id: 1,
            key: "oteryn:terrain.tibia.i2773".into(),
            family: bundle::Family::Terrain,
            terrain: Some(bundle::Terrain {
                kind: bundle::TerrainKind::Common,
                walkable: None,
                ground_speed: None,
                blocks_projectile: None,
            }),
        });
        data.manifest.palette.push(bundle::PaletteEntry {
            id: 2,
            key: "oteryn:terrain.tibia.i2317".into(),
            family: bundle::Family::Terrain,
            terrain: Some(bundle::Terrain {
                kind: bundle::TerrainKind::Common,
                walkable: None,
                ground_speed: None,
                blocks_projectile: None,
            }),
        });
        vec![
            LEVER,
            MORTAL_ORIGIN,
            MORTAL_PARTNER,
            FIRST_LANDING,
            SECOND_LANDING,
            MORTAL_REWARD_EXIT,
            MORTAL_TIMEOUT_EXIT,
        ]
    } else {
        vec![CUROS_ORIGIN, START, ARENA, EXIT, CUROS_SPECTATOR]
    };
    if mortal {
        positions.extend(WAVE_SITES);
    }
    positions.sort_by_key(|p| (p.floor, p.y, p.x));
    let mut floors: Vec<_> = positions.iter().map(|p| p.floor).collect();
    floors.dedup();
    data.manifest.world = bundle::Extent {
        min_x: positions.iter().map(|p| p.x).min().ok_or("minimum x")?,
        max_x: positions.iter().map(|p| p.x).max().ok_or("maximum x")? + 1,
        min_y: positions.iter().map(|p| p.y).min().ok_or("minimum y")?,
        max_y: positions.iter().map(|p| p.y).max().ok_or("maximum y")? + 1,
        floors,
    };
    let mut grouped = std::collections::BTreeMap::<_, Vec<_>>::new();
    for p in positions {
        let mut items = vec![sector::Item {
            palette: 0,
            depth: 0,
            attrs: Default::default(),
        }];
        if p == LEVER {
            items.push(sector::Item {
                palette: 2,
                depth: 0,
                attrs: Default::default(),
            });
            items.push(sector::Item {
                palette: 1,
                depth: 0,
                attrs: Default::default(),
            });
        }
        grouped
            .entry((p.floor, p.x / 32, p.y / 32))
            .or_default()
            .push(sector::Tile {
                x: p.x,
                y: p.y,
                flags: 0,
                house: 0,
                zones: vec![],
                items,
            });
    }
    let mut sectors: Vec<_> = grouped
        .into_iter()
        .map(|((floor, sx, sy), tiles)| bundle::Sector {
            floor,
            sx,
            sy,
            tiles,
        })
        .collect();
    sectors.sort_by_key(bundle::order);
    let bytes = oteryn_world_bundle::writer::write(&data.manifest, &sectors, &Default::default())?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: crate::map::overlay::TilePos {
                x: origin.x,
                y: origin.y,
                floor: origin.floor,
            },
        },
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |_| None,
    )
    .map_err(|error| debug(error).into())
}

async fn progress(harness: &Harness, character: u8) -> TestResult<Vec<(String, i64)>> {
    Ok(sqlx::query_as("SELECT track_key,value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2 ORDER BY track_key")
        .bind(id(character).as_slice()).bind(QUEST).fetch_all(&harness.pool).await?)
}

async fn mortal_combat(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
    admitted: &AdmittedSession,
    h: &Harness,
    world: &crate::map::boot::BundleWorld,
) -> TestResult {
    use crate::gameplay_transport::connection::{FreshAdmissionAuthority, UseCommand};
    use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
    let actor = admitted.runtime_actor.ok_or("staging actor")?;
    let key = oteryn_world_bundle::bundle::placement_key(LEVER.floor, LEVER.x, LEVER.y, 2)
        .ok_or("lever placement key")?;
    let mut placement = world.base().digest().to_vec();
    placement.extend_from_slice(&key.to_be_bytes());
    let pull = |command_id| UseCommand {
        game_session_id: admitted.game_session_id,
        command_id,
        item_fence: admitted.item_fence,
    };
    let target = WorldObjectTarget {
        placement,
        expected_revision: 0,
    };
    owner.clock_origin -= Duration::from_secs(2);
    let single = owner.use_object(actor, pull(1), target.clone()).await;
    assert_ne!(single.disposition, UseDisposition::Committed);
    assert_eq!(position(owner, actor).await?, local(MORTAL_ORIGIN));
    seed_questline(owner, admitted.game_session_id).await?;
    let (peer, peer_session) = second_player(owner, deps, h).await?;
    owner.clock_origin -= Duration::from_secs(2);
    let paired = owner.use_object(actor, pull(2), target).await;
    assert_eq!(
        paired.disposition,
        UseDisposition::Committed,
        "both staging actors enter together"
    );
    assert_eq!(position(owner, actor).await?, local(FIRST_LANDING));
    assert_eq!(position(owner, peer).await?, local(SECOND_LANDING));
    for (index, wave) in MORTAL_WAVES.into_iter().enumerate() {
        if index != 0 {
            owner.clock_origin -= Duration::from_secs(90);
        }
        owner.drain_quest_encounter_deadlines().await;
        for creature in wave {
            assert!(
                live_creature(owner, creature).await?.is_some(),
                "wave {index} spawns {creature}"
            );
        }
    }
    assert!(
        live_creature(owner, MORTAL_WAVES[0][0]).await?.is_some(),
        "later waves run with earlier waves alive"
    );
    let boss = live_creature(owner, MORTAL_WAVES[6][0])
        .await?
        .ok_or("Tirecz")?;
    assert!(
        owner
            .refresh_source_party_presence(actor, admitted.game_session_id)
            .await
    );
    assert!(
        owner
            .refresh_source_party_presence(peer, peer_session)
            .await
    );
    kill_tirecz(owner, admitted, boss).await?;
    owner.drain_kill_rewards(admitted.game_session_id).await;
    owner.drain_quest_encounter_deadlines().await;
    assert_eq!(position(owner, actor).await?, local(MORTAL_REWARD_EXIT));
    assert_eq!(position(owner, peer).await?, local(MORTAL_REWARD_EXIT));
    for character in [41, 101] {
        assert_eq!(
            progress(h, character).await?,
            vec![(MISSION09.into(), 2), (QUESTLINE.into(), 26)]
        );
    }
    let achievements: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_account_achievements WHERE achievement_key='oteryn:achievement/champion_of_chazorai'",
    ).fetch_one(&h.pool).await?;
    assert_eq!(achievements, 2);
    assert!(live_creature(owner, MORTAL_WAVES[0][0]).await?.is_none());
    Ok(())
}

async fn position(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
) -> TestResult<MovementLocalPosition> {
    Ok(owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?
        .position())
}

async fn live_creature(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    creature: &str,
) -> TestResult<Option<ExactActorRef>> {
    let runtime = owner.runtime.lock().await;
    Ok(runtime
        .current_live_creature_census()
        .map_err(debug)?
        .into_iter()
        .find(|actor| runtime.matches_live_creature_identity(*actor, creature.as_bytes())))
}

async fn second_player(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    deps: &Dependencies,
    h: &Harness,
) -> TestResult<(ExactActorRef, GameSessionId)> {
    h.seed_appearance_observer().await?;
    super::appearance_change_postgres::observer_visibility_tests::admit(h).await?;
    let session = GameSessionId::decode(&id(110)).map_err(debug)?;
    seed_questline(owner, session).await?;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(|()| "second fence")?
        .ok_or("second session")?;
    let actor = {
        let mut runtime = owner.runtime.lock().await;
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(actor, local(MORTAL_PARTNER))
            .map_err(debug)?;
        runtime
            .bind_attacker_lease(
                actor,
                session,
                crate::foundation::CharacterLease::new(
                    crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                        .map_err(debug)?,
                    fence.character_lease_generation,
                )
                .map_err(debug)?,
            )
            .map_err(debug)?;
        actor
    };
    let mut admitted = AdmittedSession {
        game_session_id: session,
        world_id: deps.admitted.world_id,
        channel_id: deps.admitted.channel_id,
        runtime_actor: Some(actor),
        first_entry: FirstEntryOutcome::Positioned,
        controller: None,
        continuity: crate::gameplay_transport::connection::SessionContinuity::FRESH,
        item_fence: Some(fence.item_fence()),
    };
    owner.admit_progression_session(&mut admitted).await;
    deps.initialize_player(owner, actor, session).await;
    owner.admit_quest_session(&admitted).await;
    Ok((actor, session))
}

async fn kill_tirecz(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: &AdmittedSession,
    target: ExactActorRef,
) -> TestResult {
    use crate::foundation::{CommandId, CommandRef, OwnerDamageCommand};
    let queue = owner.states.attack.lock().await.kills().clone();
    let mut runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    let actor = admitted.runtime_actor.ok_or("lethal actor")?;
    let session = admitted.game_session_id;
    let lease = runtime
        .bound_attacker_lease(actor, session)
        .map_err(debug)?;
    let health = runtime.companion_snapshot(target).map_err(debug)?.health;
    let observed = owner.owner_now();
    let now = observed.get() / 1000;
    runtime
        .borrow_exact_actor_commit_at(now)
        .commit_damage_for_bound_attacker(
            target,
            actor,
            CommandRef::new(session, CommandId::new(9).map_err(debug)?),
            0,
            OwnerDamageCommand {
                target: MORTAL_WAVES[6][0].as_bytes(),
                occurrence: b"nfront2",
                binding: b"tirecz-death",
                damage: health,
            },
        )
        .map_err(debug)?;
    crate::gameplay_transport::kill_reward::record_creature_kill(
        &mut runtime,
        crate::gameplay_transport::kill_reward::KillSink {
            prey: Some(&states),
            activity: &[],
            queue: &queue,
            rewards: owner.reward_table(),
            charms: None,
        },
        target,
        crate::combat::CapturedRewardPrincipal {
            character: lease.character_id(),
            lease_generation: lease.generation(),
            session,
            actor,
        },
        now,
        Some(observed),
    );
    Ok(())
}
