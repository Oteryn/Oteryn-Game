//! Native Ancient Tombs movement and reward claims through PostgreSQL-backed dispatch.
use super::fixture::{Harness, debug, id};
use super::l6_world_pickups::{ExpectedTile, NativeCrop, native_scene};
use super::quest_world_callbacks::{local, wire};
use super::{
    AdmittedSession, ComposedFreshAdmission, Dependencies, TestResult, prepare_current_session,
};
use crate::content::channel_content_pin::ChannelContentPin;
use crate::foundation::ChannelRuntimeV1;
use crate::gameplay_transport::connection::FirstEntryOutcome;
use oteryn_gameplay_quests::quest::{loader, world_callbacks::Position};
use oteryn_protocol_oteryn::world_spatial::{
    COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT, StepDirection, StepDisposition, decode_step_result,
    encode_step_intent,
};
use std::sync::Arc;

const QUEST: &str = "oteryn:quest.the_ancient_tombs_quest";
const PREFIX: &str = "oteryn:quest-progress/quest/u7_4/the_ancient_tombs/";

#[test]
fn morguthis_seven_flames_preserve_higher_imported_progress() -> TestResult {
    run("morguthis_seven_flames", false)
}

#[test]
fn sarcophagi_grant_their_pieces_once() -> TestResult {
    run("sarcophagi", true)
}

#[test]
fn ashmunrah_north_table_assembles_helmet() -> TestResult {
    super::l6_floor_native::run("l6-floor-assembly-native")
}

#[test]
fn ashmunrah_north_table_enchants_helmet() -> TestResult {
    super::l6_floor_native::run("l6-floor-assembly-native-ruby-stack")
}

fn run(name: &'static str, rewards: bool) -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run_async(async move |admin| {
        let h =
            Harness::create_with_admission_seed(admin, "qcx_ancient_tombs", false, false).await?;
        prepare_current_session(&h).await?;
        let seal = h.recovery.seal_current().map_err(debug)?;
        let authority = h
            .root
            .open_character_authority(&seal)
            .await
            .map_err(debug)?;
        let mut deps = Dependencies::new()?;
        let (controller, activated) =
            super::death_ml_loss_composed::production_generation_with_pin(
                deps.admitted.world_id,
                "content-1",
            )?;
        let generation = controller.active().ok_or("content generation")?;
        let native = generation.native_gameplay().ok_or("native content")?;
        let origin = if rewards {
            local(approach(SARCOPHAGI[0].0))
        } else {
            local(Position {
                x: 33163,
                y: 32696,
                floor: -14,
            })
        };
        let world = if rewards {
            reward_world(native)?
        } else {
            super::l6_native_flames::world(native)?
        };
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
                    x: origin.x,
                    y: origin.y,
                    floor: origin.floor,
                },
            )
            .ok_or("tomb bundle room")?;
        let pin = ChannelContentPin::from_activation(
            deps.admitted.world_id,
            1,
            generation.identity().server_artifact_digest(),
            generation.identity().client_artifact_digest(),
            frame,
            world.map_revision_digest(),
            (origin.x, origin.y, origin.floor),
        );
        let mut runtime = ChannelRuntimeV1::from_committed_assignment(
            deps.admitted.world_id,
            deps.admitted.channel_id,
            crate::foundation::NodeId::decode(&id(1)).map_err(debug)?,
            1,
            1,
            1,
            "runtime-scope-assignment:1",
            2,
            pin,
        )
        .map_err(debug)?;
        let reservation = runtime
            .reserve_fresh_session(deps.admitted.game_session_id)
            .map_err(debug)?;
        deps.actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .initialize_pinned_test_position(deps.actor, origin)
            .map_err(debug)?;
        deps.runtime = tokio::sync::Mutex::new(runtime);

        let treasure = format!("{PREFIX}morguthis_treasure");
        if !rewards {
            // A restored character has all seven flames and a value above the door's stage.
            let mut tx = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *tx)
                .await?;
            sqlx::query("INSERT INTO game_character_quest_tracks(character_id,track_key,quest_key,value,committed_character_revision) SELECT encode($1,'hex')::uuid,$2||'morguthis_blue_flame_storage'||n::text,$3,1,2 FROM generate_series(1,7) n")
            .bind(id(41).as_slice()).bind(PREFIX).bind(QUEST).execute(&mut *tx).await?;
            sqlx::query(
                "INSERT INTO game_character_quest_tracks VALUES(encode($1,'hex')::uuid,$2,$3,3,2)",
            )
            .bind(id(41).as_slice())
            .bind(&treasure)
            .bind(QUEST)
            .execute(&mut *tx)
            .await?;
            tx.commit().await?;
        }

        let mut owner = deps.owner(&h, &authority);
        owner.active_generation = Some(generation);
        owner.movement_cells = (&cells).into();
        owner.rooms = crate::gameplay_transport::SeamRooms::new(Some(&room), Some(&room));
        owner.quest_catalogue = Some(Arc::new(
            loader::load_embedded_quest_state("content-1")
                .map_err(debug)?
                .catalogue()
                .clone(),
        ));
        let mut admitted = deps.admitted;
        admitted.first_entry = FirstEntryOutcome::Positioned;
        let fence = owner
            .current_quest_fence(admitted.game_session_id)
            .await
            .map_err(|()| "quest fence")?
            .ok_or("quest fence")?;
        admitted.item_fence = Some(fence.item_fence());
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
        if rewards {
            sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
                .bind(id(42).as_slice()).bind(id(43).as_slice()).execute(&h.pool).await?;
            sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
                .bind(id(42).as_slice()).bind(id(43).as_slice())
                .bind(generation.identity().server_artifact_digest().as_slice())
                .bind(generation.identity().client_artifact_digest().as_slice())
                .bind(frame.as_slice()).execute(&h.pool).await?;
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
        }
        owner.admit_progression_session(&mut admitted).await;
        deps.initialize(&owner).await;
        owner.admit_quest_session(&admitted).await;
        if rewards {
            return claim_parts(&mut owner, &mut admitted, &h, deps.actor).await;
        }
        let goal = local(Position {
            x: 33163,
            y: 32694,
            floor: -14,
        });
        let result = wire(
            &owner,
            &mut admitted,
            1,
            COMMAND_TYPE_WORLD_ACTOR_STEP_INTENT,
            encode_step_intent(StepDirection::North),
        )
        .await?;
        assert_eq!(
            decode_step_result(&result).map_err(debug)?,
            StepDisposition::Moved
        );
        let actual = deps
            .runtime
            .lock()
            .await
            .read_actor_position(deps.actor)
            .map_err(debug)?
            .position();
        assert_eq!(
            actual, goal,
            "the gate admits a character with all seven flags"
        );
        let stored: i64 = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key=$2")
            .bind(id(41).as_slice()).bind(&treasure).fetch_one(&h.pool).await?;
        assert_eq!(stored, 3, "gate must preserve higher treasure progress");
        Ok(())
    });
    eprintln!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?} success={}",
        started.elapsed(),
        result.is_ok()
    );
    result
}

// Spoiler curid 5073 oldid 1174796 links each pharaoh to one distinct part.
macro_rules! region {
    ($name:literal) => {
        include_bytes!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../content/world/placements/region-z",
            $name,
            ".b3"
        ))
    };
}

type Sarcophagus = ((u16, u16, u8), u16, &'static [u8]);
const SARCOPHAGI: [Sarcophagus; 5] = [
    ((33126, 32589, 15), 3222, region!("15-x129-y127")),
    ((33174, 32932, 15), 3223, region!("15-x129-y128")),
    ((33182, 32712, 14), 3224, region!("14-x129-y127")),
    ((33349, 32825, 14), 3226, region!("14-x130-y128")),
    ((33041, 32772, 14), 3227, region!("14-x129-y128")),
];

fn approach((x, y, z): (u16, u16, u8)) -> Position {
    Position {
        x,
        y: y + 1,
        floor: -(z as i8),
    }
}

fn reward_world(
    native: &crate::content::native_gameplay::NativeGameplayState,
) -> TestResult<crate::map::boot::BundleWorld> {
    let crops = SARCOPHAGI
        .iter()
        .map(|&((x, y, z), _, bytes)| NativeCrop {
            bytes,
            region: (z, x / 256, y / 256),
            bounds: (x, y, x + 1, y + 2),
            start: (x, y + 1),
            expected: vec![
                ExpectedTile {
                    position: (x, y),
                    ids: vec![423, 1994],
                    counted_entry: None,
                },
                ExpectedTile {
                    position: (x, y + 1),
                    ids: vec![423],
                    counted_entry: None,
                },
            ],
        })
        .collect();
    native_scene(native, crops, &[])
}

async fn claim_parts(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    actor: crate::foundation::ExactActorRef,
) -> TestResult {
    use oteryn_protocol_oteryn::world_object::{
        UseDisposition, WorldObjectTarget, decode_use_result, encode_use_intent,
    };
    let mut failures = Vec::new();
    for (point @ (x, y, _), id, _) in SARCOPHAGI {
        let position = approach(point);
        let key = oteryn_world_bundle::bundle::placement_key(position.floor, x, y, 1)
            .ok_or("sarcophagus placement")?;
        let map = owner.bundle_map().ok_or("reward room map")?;
        let mut placement = map.overlay.base().digest().to_vec();
        placement.extend_from_slice(&key.to_be_bytes());
        let doors = crate::map::floor::BundleDoors(map.facts.doors());
        {
            let mut runtime = owner.runtime.lock().await;
            let previous = runtime.read_actor_position(actor).map_err(debug)?;
            if previous.position() != local(position) {
                let proof = crate::map::floor::prepare_quest_callback_teleport(
                    &runtime,
                    &owner.movement_cells,
                    actor,
                    admitted.game_session_id,
                    previous,
                    local(position),
                    false,
                    &doors,
                )
                .map_err(debug)?;
                runtime.commit_source_step(proof).map_err(debug)?;
            }
        }
        for expected in [UseDisposition::Committed, UseDisposition::Rejected] {
            owner.clock_origin -= std::time::Duration::from_secs(2);
            let ordinal = admitted.continuity.next_command_id;
            let expected_revision = owner
                .bundle_map()
                .ok_or("reward room map")?
                .facts
                .object_revision(key);
            let payload = encode_use_intent(&WorldObjectTarget {
                placement: placement.clone(),
                expected_revision,
            })
            .map_err(debug)?;
            let result = wire(owner, admitted, ordinal, 2, payload).await?;
            let actual = decode_use_result(&result).map_err(debug)?;
            let quantity: i64 = sqlx::query_scalar("SELECT coalesce(sum(i.quantity),0)::bigint FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) WHERE i.lifecycle=1 AND i.definition_production_key=$1")
                .bind(format!("oteryn:item.tibia.i{id}")).fetch_one(&h.pool).await?;
            if actual != expected || quantity != 1 {
                failures.push((id, expected, actual, quantity));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "claim mismatches (item, expected result, actual result, quantity): {failures:?}"
    );
    Ok(())
}
