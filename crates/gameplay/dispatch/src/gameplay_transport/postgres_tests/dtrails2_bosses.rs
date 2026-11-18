//! Hall admission through the real world-object USE owner on PostgreSQL.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::connection::{
    FirstEntryOutcome, FreshAdmissionAuthority, UseCommand,
};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_gameplay_quests::quest::world_callbacks::{
    Callback, EncounterMonster, EncounterPhase, EncounterPlayer, Position,
};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use std::sync::Arc;

const RAVAGER: &str = "oteryn:creature.the_ravager";
const JAR: &str = "oteryn:creature.greater_canopic_jar";
const SHARGON: &str = "oteryn:creature.death_priest_shargon";
const LESSER: &str = "oteryn:creature.lesser_death_minion";
const GREATER: &str = "oteryn:creature.greater_death_minion";

#[derive(Clone, Copy)]
enum Case {
    Ravager,
    Shargon,
}

impl Case {
    fn unique(self) -> u16 {
        match self {
            Self::Ravager => 30001,
            Self::Shargon => 30002,
        }
    }
}

fn callback(trigger: Position, case: Case) -> TestResult<Callback> {
    let mut callback = oteryn_gameplay_quests::quest::world_callbacks::load()?
        .into_iter()
        .find(|c| c.key == "oteryn:quest-callback/engenc2/cults-essence")
        .ok_or("base encounter callback")?;
    let (key, phase, monsters) = match case {
        Case::Ravager => (
            "ravager",
            EncounterPhase::Ravager,
            &[(106, RAVAGER), (108, JAR)][..],
        ),
        Case::Shargon => ("shargon", EncounterPhase::Shargon, &[(106, SHARGON)][..]),
    };
    callback.key = format!("oteryn:quest-callback/dtrails2/{key}-test");
    callback.selector.position = Some(trigger);
    callback.selector.unique = Some(case.unique());
    callback.selector.action = None;
    callback.selector.appearance = Some(2772);
    callback.lever = Some([2772, 2773]);
    let encounter = callback.effects.encounter.as_mut().ok_or("encounter")?;
    encounter.phase = Some(phase);
    encounter.players = vec![EncounterPlayer {
        from: trigger,
        to: Position { x: 104, ..trigger },
    }];
    encounter.monsters = monsters
        .iter()
        .map(|&(x, creature)| EncounterMonster {
            position: Position { x, ..trigger },
            creature: creature.into(),
        })
        .collect();
    encounter.room_min = Position {
        x: 103,
        y: 99,
        ..trigger
    };
    encounter.room_max = Position {
        x: 113,
        y: 101,
        ..trigger
    };
    Ok(callback)
}

fn world(trigger: Position, case: Case) -> TestResult<crate::map::boot::BundleWorld> {
    use crate::map::overlay::TilePos;
    use oteryn_world_bundle::{bundle, sector, writer};
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette.truncate(1);
    for (index, appearance) in [2772, 2773, 519, 2129].into_iter().enumerate() {
        let mut terrain = data.manifest.palette[0].clone();
        terrain.key = format!("oteryn:terrain.tibia.i{appearance}");
        terrain.id = u32::try_from(index + 1)?;
        if matches!(appearance, 2772 | 2773) {
            terrain.terrain = Some(bundle::Terrain {
                kind: bundle::TerrainKind::Common,
                walkable: None,
                ground_speed: None,
                blocks_projectile: None,
            });
        }
        data.manifest.palette.push(terrain);
    }
    data.manifest.world = bundle::Extent {
        min_x: 99,
        min_y: 99,
        max_x: 114,
        max_y: 102,
        floors: vec![-7],
    };
    let mut tiles = Vec::new();
    for y in 99..=101 {
        for x in 99..=113 {
            let mut items = vec![sector::Item {
                palette: 0,
                depth: 0,
                attrs: Default::default(),
            }];
            if (x, y) == (trigger.x, trigger.y) {
                items.push(sector::Item {
                    palette: 1,
                    depth: 0,
                    attrs: sector::Attrs {
                        unique: Some(case.unique()),
                        ..Default::default()
                    },
                });
            } else {
                match case {
                    Case::Ravager if (x, y) == (107, 100) => items[0].palette = 3,
                    Case::Shargon if (111..=113).contains(&x) => items[0].palette = 4,
                    _ => {}
                }
            }
            tiles.push(sector::Tile {
                x,
                y,
                flags: 0,
                house: 0,
                zones: vec![],
                items,
            });
        }
    }
    let bytes = writer::write(
        &data.manifest,
        &[bundle::Sector {
            floor: -7,
            sx: 3,
            sy: 3,
            tiles,
        }],
        &Default::default(),
    )?;
    pins.digest = bundle::read(&bytes)?.digest;
    let base = crate::map::load(&bytes, &pins).map_err(debug)?;
    crate::map::boot::boot(
        &bytes,
        &crate::map::boot::BootPins {
            map_revision: crate::map::overlay::map_revision(&base),
            bundle: pins,
            start: TilePos {
                x: trigger.x,
                y: trigger.y,
                floor: -7,
            },
        },
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |_| None,
    )
    .map_err(debug)
}

#[test]
fn dtrails2_ravager_use_admission_then_proximity_and_floor_healing_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(Case::Ravager)
}

#[test]
fn dtrails2_shargon_use_admission_then_minion_growth_and_rune_response_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    drop(admin);
    scenario(Case::Shargon)
}

fn scenario(case: Case) -> TestResult {
    super::kill_reward_e2e::scenario_thread::run("dtrails2", move || {
        run(async |admin| {
            reset();
            let h = Harness::create_with_admission_seed(admin, "dark_trails_ravager", true, false)
                .await?;
            prepare_current_session(&h).await?;
            let trigger = Position {
                x: 100,
                y: 100,
                floor: -7,
            };
            let callback = callback(trigger, case)?;
            let world = world(trigger, case)?;
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
                        x: 100,
                        y: 100,
                        floor: 7,
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
                (100, 100, 7),
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
                .initialize_pinned_test_position(
                    deps.actor,
                    MovementLocalPosition {
                        x: 100,
                        y: 100,
                        floor: 7,
                    },
                )
                .map_err(debug)?;
            native
                .install_companion_policies(&mut runtime)
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
            owner.quest_catalogue = Some(Arc::new(
                oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                    .map_err(debug)?
                    .catalogue()
                    .clone(),
            ));
            assert!(
                owner
                    .states
                    .quest_callbacks
                    .fixture
                    .set(vec![callback.clone()])
                    .is_ok()
            );
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
            assert!(
                owner
                    .refresh_source_party_presence(deps.actor, admitted.game_session_id)
                    .await
            );
            let key =
                oteryn_world_bundle::bundle::placement_key(-7, 100, 100, 1).ok_or("lever key")?;
            let mut placement = world.base().digest().to_vec();
            placement.extend_from_slice(&key.to_be_bytes());
            let outcome = owner
                .use_object(
                    deps.actor,
                    UseCommand {
                        game_session_id: admitted.game_session_id,
                        command_id: 1,
                        item_fence: admitted.item_fence,
                    },
                    WorldObjectTarget {
                        placement: placement.clone(),
                        expected_revision: 0,
                    },
                )
                .await;
            assert_eq!(
                outcome.disposition,
                UseDisposition::Committed,
                "phase admission"
            );
            match case {
                Case::Ravager => verify_ravager(&owner, &cells).await?,
                Case::Shargon => verify_shargon(&owner, &cells).await?,
            }
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    })
}

async fn verify_ravager(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    cells: &crate::content::NativeEntryMovementCells,
) -> TestResult {
    let (boss, jar) = {
        let mut runtime = owner.runtime.lock().await;
        let census = runtime.positioned_melee_census().map_err(debug)?;
        let find = |key: &str| {
            census
                .iter()
                .find(|(actor, _, _)| {
                    runtime.matches_live_creature_identity(*actor, key.as_bytes())
                })
                .map(|(actor, _, _)| *actor)
                .ok_or("admitted creature")
        };
        let (boss, jar) = (find(RAVAGER)?, find(JAR)?);
        runtime
            .set_test_creature_health(boss, 20_000)
            .map_err(debug)?;
        (boss, jar)
    };
    owner
        .drain_monster_melee_at(
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(1_000_000),
        )
        .await;
    {
        let mut runtime = owner.runtime.lock().await;
        assert_eq!(runtime.live_creature_health(boss), Some(30_000));
        runtime.remove_encounter_actor(jar).map_err(debug)?;
        crate::movement::relocate::relocate_creature(
            &mut runtime,
            boss,
            &cells,
            &std::collections::BTreeSet::new(),
            MovementLocalPosition {
                x: 107,
                y: 100,
                floor: 7,
            },
        )
        .map_err(debug)?;
    }
    owner
        .drain_monster_melee_at(
            oteryn_simulation_determinism::SemanticTimeMicros::from_micros(2_000_000),
        )
        .await;
    let health = owner
        .runtime
        .lock()
        .await
        .live_creature_health(boss)
        .ok_or("live Ravager")?;
    assert!((37_500..=39_000).contains(&health), "floor heal: {health}");

    Ok(())
}

async fn verify_shargon(
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    cells: &crate::content::NativeEntryMovementCells,
) -> TestResult {
    use oteryn_gameplay_entities::carrier::NativeSummonAdmissionSpec;
    use oteryn_simulation_determinism::SemanticTimeMicros;
    let (boss, minion) = {
        let mut runtime = owner.runtime.lock().await;
        let boss = runtime
            .positioned_melee_census()
            .map_err(debug)?
            .into_iter()
            .find(|(actor, _, _)| {
                runtime.matches_live_creature_identity(*actor, SHARGON.as_bytes())
            })
            .map(|(actor, _, _)| actor)
            .ok_or("admitted Shargon")?;
        let spec = NativeSummonAdmissionSpec::qualified(
            SHARGON,
            LESSER,
            runtime
                .companion_policy(LESSER)
                .map_err(debug)?
                .maximum_health,
            0,
            2,
            2,
            runtime.content_pin().server_artifact_digest(),
        )
        .map_err(debug)?;
        let stamp = runtime.issue_owner_work().map_err(debug)?;
        let minion = runtime
            .admit_current_native_summon(
                stamp,
                boss,
                &spec,
                MovementLocalPosition {
                    x: 106,
                    y: 101,
                    floor: 7,
                },
            )
            .map_err(debug)?;
        runtime
            .install_creature_policy(minion, LESSER)
            .map_err(debug)?;
        (boss, minion)
    };
    owner
        .drain_monster_melee_at(SemanticTimeMicros::from_micros(1_000_000))
        .await;
    owner
        .drain_monster_melee_at(SemanticTimeMicros::from_micros(7_000_000))
        .await;
    assert!(owner.runtime.lock().await.contains_live_creature(minion));
    owner
        .drain_monster_melee_at(SemanticTimeMicros::from_micros(8_000_000))
        .await;
    {
        let mut runtime = owner.runtime.lock().await;
        assert!(!runtime.contains_live_creature(minion));
        assert!(
            runtime
                .positioned_melee_census()
                .map_err(debug)?
                .into_iter()
                .any(|(actor, _, _)| {
                    runtime.matches_live_creature_identity(actor, GREATER.as_bytes())
                        && runtime.native_summon_master(actor).is_none()
                })
        );
        crate::movement::relocate::relocate_creature(
            &mut runtime,
            boss,
            cells,
            &std::collections::BTreeSet::new(),
            MovementLocalPosition {
                x: 112,
                y: 100,
                floor: 7,
            },
        )
        .map_err(debug)?;
    }
    owner
        .drain_monster_melee_at(SemanticTimeMicros::from_micros(9_000_000))
        .await;
    let runtime = owner.runtime.lock().await;
    assert!(!runtime.contains_live_creature(boss));
    assert!(
        !runtime.retains_corpse_projection(boss),
        "runes refuse the fight without kill credit"
    );
    Ok(())
}
