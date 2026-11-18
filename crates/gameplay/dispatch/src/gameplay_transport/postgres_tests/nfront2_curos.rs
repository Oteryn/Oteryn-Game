//! Drive Curos through the normal movement owner with a routed PostgreSQL character.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::connection::FirstEntryOutcome;
use oteryn_gameplay_quests::quest::world_callbacks::Position;
use oteryn_protocol_oteryn::world_spatial::StepDirection;
use std::time::Duration;

const NAME: &str = "nfront2-curos";
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
const QUEST: &str = "oteryn:quest.the_new_frontier_quest";
const QUESTLINE: &str = "oteryn:quest-progress/quest/u8_54/the_new_frontier/questline";
const MISSION: &str = "oteryn:quest-progress/quest/u8_54/the_new_frontier/mission06";

#[test]
fn nfront2_curos_step_starts_survival_and_awards_progress_only_at_deadline() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run(NAME, move || {
        run(async move |_| {
            reset();
            let h =
                Harness::create_with_admission_seed(admin, "nfront2_curos", true, false).await?;
            prepare_current_session(&h).await?;
            let world = world()?;
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
                        x: START.x - 1,
                        y: START.y,
                        floor: 6,
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
                (START.x - 1, START.y, 6),
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
                        x: i32::from(START.x - 1),
                        y: i32::from(START.y),
                        floor: 6,
                    },
                )
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
                    .clone(),
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

            let actor = deps.actor;
            let session = admitted.game_session_id;
            let origin = local(Position {
                x: START.x - 1,
                ..START
            });
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
            sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT character_id,$2,$3,17,character_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
                .bind(id(41).as_slice()).bind(QUESTLINE).bind(QUEST).execute(&h.pool).await?;
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
            assert_eq!(progress(&h).await?, vec![(QUESTLINE.to_owned(), 17)]);
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
            assert_eq!(progress(&h).await?, completed);
            owner.drain_quest_encounter_deadlines().await;
            assert_eq!(progress(&h).await?, completed);
            Ok(())
        })
    })
}

fn local(p: Position) -> MovementLocalPosition {
    MovementLocalPosition {
        x: i32::from(p.x),
        y: i32::from(p.y),
        floor: -i16::from(p.floor),
    }
}

fn world() -> TestResult<crate::map::boot::BundleWorld> {
    use oteryn_world_bundle::{bundle, sector};
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette.truncate(1);
    data.manifest.world = bundle::Extent {
        min_x: START.x - 1,
        max_x: ARENA.x + 1,
        min_y: START.y,
        max_y: EXIT.y + 1,
        floors: vec![-7, -6],
    };
    let mut grouped = std::collections::BTreeMap::<_, Vec<_>>::new();
    for p in [
        Position {
            x: START.x - 1,
            ..START
        },
        START,
        ARENA,
        EXIT,
    ] {
        grouped
            .entry((p.floor, p.x / 32, p.y / 32))
            .or_default()
            .push(sector::Tile {
                x: p.x,
                y: p.y,
                flags: 0,
                house: 0,
                zones: vec![],
                items: vec![sector::Item {
                    palette: 0,
                    depth: 0,
                    attrs: Default::default(),
                }],
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
                x: START.x - 1,
                y: START.y,
                floor: START.floor,
            },
        },
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |_| None,
    )
    .map_err(|error| debug(error).into())
}

async fn progress(harness: &Harness) -> TestResult<Vec<(String, i64)>> {
    Ok(sqlx::query_as("SELECT track_key,value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND quest_key=$2 ORDER BY track_key")
        .bind(id(41).as_slice()).bind(QUEST).fetch_all(&harness.pool).await?)
}
