//! The native STEP path admits one player, then the Channel timer expels that player.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::connection::{FirstEntryOutcome, FreshAdmissionAuthority};
use oteryn_gameplay_quests::quest::world_callbacks::{EncounterPlayer, EncounterTiming, Position};
use oteryn_protocol_oteryn::world_spatial::StepDirection;
use std::sync::Arc;

const ENTRY: Position = Position {
    x: 100,
    y: 100,
    floor: -7,
};
const ALIAS: Position = Position {
    x: 100,
    y: 102,
    floor: -7,
};
const INSIDE: Position = Position {
    x: 116,
    y: 117,
    floor: -7,
};
const EXIT: Position = Position {
    x: 98,
    y: 101,
    floor: -7,
};
const KEY: &str = "oteryn:quest-world/m01c-stampede-entry";

#[test]
fn hidden_threats_room_step_admits_one_player_and_expires_postgres() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e::scenario_thread::run("hidden-threats-room", move || {
        run(async move |_| {
            reset();
            let h = Harness::create_with_admission_seed(admin, "hidden_threats_room", true, false)
                .await?;
            prepare_current_session(&h).await?;
            // Retain an installed, durable STEP transition while adapting its map
            // effects to the solo room. Quest bindings ship in the content lane.
            let loaded =
                oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
                    .map_err(debug)?;
            let mut callback = loaded
                .catalogue()
                .authored_world_callbacks()
                .and_then(|callbacks| callbacks.iter().find(|callback| callback.key == KEY))
                .ok_or("authored trial")?
                .clone();
            callback.selector.position = Some(ENTRY);
            callback.selector.position_aliases = vec![ALIAS];
            callback.selector.appearance = Some(100);
            callback.return_on_failure = true;
            callback.failure_teleport = Some(EXIT);
            let encounter = callback
                .effects
                .encounter
                .as_mut()
                .ok_or("trial encounter")?;
            encounter.players = vec![EncounterPlayer {
                from: ENTRY,
                to: INSIDE,
            }];
            encounter.room_min = Position {
                x: 114,
                y: 114,
                ..INSIDE
            };
            encounter.room_max = Position {
                x: 120,
                y: 120,
                ..INSIDE
            };
            encounter.monsters.truncate(4);
            for (monster, (x, y, creature)) in encounter.monsters.iter_mut().zip([
                (118, 117, "oteryn:creature.cave_spider"),
                (116, 115, "oteryn:creature.tarantula"),
                (116, 116, "oteryn:creature.tarantula"),
                (116, 118, "oteryn:creature.tarantula"),
            ]) {
                monster.position = Position { x, y, ..INSIDE };
                monster.creature = creature.into();
            }
            encounter.timing = Some(EncounterTiming {
                spawn_after_ms: 0,
                reset_after_ms: 300_000,
            });
            let world = world()?;
            let world_id = crate::foundation::WorldId::decode(&id(42)).map_err(debug)?;
            let (controller, activated) =
                super::death_ml_loss_composed::production_generation_with_pin(
                    world_id,
                    "content-1",
                )?;
            let generation = controller.active().ok_or("active generation")?;
            let cells = world
                .movement_cells(activated.qualified_room().movement_cells())
                .map_err(debug)?;
            let frame = world.frame_binding_digest(world_id);
            let room = activated
                .qualified_room()
                .bundle_view(
                    cells.clone(),
                    frame,
                    world.map_revision_digest(),
                    crate::content::NativeEntryStart {
                        x: 99,
                        y: 100,
                        floor: 7,
                    },
                )
                .ok_or("bundle room")?;
            let pin = crate::content::channel_content_pin::ChannelContentPin::from_activation(
                world_id,
                1,
                generation.identity().server_artifact_digest(),
                generation.identity().client_artifact_digest(),
                frame,
                world.map_revision_digest(),
                (99, 100, 7),
            );
            let deps = Dependencies::with_content_and_capacity(
                pin,
                room.clone(),
                crate::spell::cast::v1_spell_book().map_err(debug)?,
                16,
            )?;
            deps.runtime
                .lock()
                .await
                .initialize_pinned_test_position(deps.actor, local(Position { x: 99, ..ENTRY }))
                .map_err(debug)?;
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
            owner.quest_catalogue = Some(Arc::new(loaded.catalogue().clone()));
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
            use crate::gameplay_transport::spell_premium_coordinator::{
                SpellPremiumCoordinator, purchase_fixture,
            };
            let (consumer, clock, activation, _) = purchase_fixture::current(id(40));
            h.root
                .latch_premium_activation(
                    &activation.activation_id,
                    activation.switch_over_us,
                    activation.switch_over_us + 60_000_000,
                )
                .await
                .map_err(debug)?;
            owner.premium_coordinator = Some(SpellPremiumCoordinator::new(
                consumer,
                Some(clock),
                Some(activation),
                Some(h.root.clone()),
            ));
            owner.admit_premium(id(40), admitted.game_session_id);
            owner.admit_quest_session(&admitted).await;
            let (other, second) = super::grizzly_boss::vacancy::second_player(
                &owner,
                &deps,
                &h,
                local(Position {
                    y: ALIAS.y - 1,
                    ..ALIAS
                }),
            )
            .await?;

            let (step, _) = owner
                .paced_step(deps.actor, admitted.game_session_id, StepDirection::East)
                .await;
            assert_eq!(
                step.disposition,
                oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved
            );
            assert_eq!(
                position(&owner, deps.actor).await?,
                local(INSIDE),
                "STEP commits the solo transfer"
            );
            let (step, _) = owner
                .paced_step(other, second.game_session_id, StepDirection::South)
                .await;
            assert_eq!(
                step.disposition,
                oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved
            );
            assert_eq!(position(&owner, other).await?, local(EXIT));
            assert_eq!(position(&owner, deps.actor).await?, local(INSIDE));
            assert_eq!(
                owner
                    .runtime
                    .lock()
                    .await
                    .current_live_creature_census()
                    .map_err(debug)?
                    .len(),
                4
            );
            let receipts: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
            )
            .bind(KEY)
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(receipts, 1);

            let due = owner
                .states
                .quest_callbacks
                .next_encounter_due_us()
                .ok_or("room deadline")?;
            owner.drain_quest_encounter_deadlines().await;
            assert_eq!(position(&owner, deps.actor).await?, local(INSIDE));
            owner.clock_origin = std::time::Instant::now()
                .checked_sub(std::time::Duration::from_micros(due))
                .ok_or("deadline clock")?;
            owner.drain_quest_encounter_deadlines().await;
            assert_eq!(position(&owner, deps.actor).await?, local(EXIT));
            assert_eq!(position(&owner, other).await?, local(EXIT));
            assert!(
                owner
                    .runtime
                    .lock()
                    .await
                    .current_live_creature_census()
                    .map_err(debug)?
                    .is_empty()
            );
            Ok(())
        })
    })
}

async fn position(
    owner: &super::ComposedFreshAdmission<'_, '_, '_>,
    actor: crate::foundation::ExactActorRef,
) -> TestResult<MovementLocalPosition> {
    Ok(owner
        .runtime
        .lock()
        .await
        .read_actor_position(actor)
        .map_err(debug)?
        .position())
}

fn local(p: Position) -> MovementLocalPosition {
    MovementLocalPosition {
        x: i32::from(p.x),
        y: i32::from(p.y),
        floor: -i16::from(p.floor),
    }
}

fn world() -> TestResult<crate::map::boot::BundleWorld> {
    use crate::map::overlay::TilePos;
    use oteryn_world_bundle::{bundle, sector};
    let (bytes, mut pins) = crate::map::boot::tests::bundle();
    let mut data = bundle::read(&bytes)?;
    data.manifest.palette.truncate(1);
    data.manifest.palette.push(bundle::PaletteEntry {
        id: 1,
        key: "oteryn:terrain.tibia.i100".into(),
        family: bundle::Family::Terrain,
        terrain: Some(bundle::Terrain {
            kind: bundle::TerrainKind::Common,
            walkable: None,
            ground_speed: None,
            blocks_projectile: None,
        }),
    });
    data.manifest.world = bundle::Extent {
        min_x: 98,
        min_y: 99,
        max_x: 121,
        max_y: 121,
        floors: vec![-7],
    };
    let item = |palette| sector::Item {
        palette,
        depth: 0,
        attrs: Default::default(),
    };
    let mut tiles = Vec::new();
    for y in 99..=120 {
        for x in 98..=120 {
            let mut items = vec![item(0)];
            if [(ENTRY.x, ENTRY.y), (ALIAS.x, ALIAS.y)].contains(&(x, y)) {
                items.push(item(1));
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
    let bytes = oteryn_world_bundle::writer::write(
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
                x: 99,
                y: 100,
                floor: -7,
            },
        },
        crate::foundation::WorldId::decode(&id(42)).map_err(debug)?,
        crate::foundation::ChannelId::decode(&id(43)).map_err(debug)?,
        |_| None,
    )
    .map_err(debug)
    .map_err(Into::into)
}
