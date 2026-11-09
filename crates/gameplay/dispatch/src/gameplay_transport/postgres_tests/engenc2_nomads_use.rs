//! Actual native window USE must refuse before a quest receipt when crates are absent.
use super::fixture::{Harness, debug, id, run};
use super::{Dependencies, TestResult, prepare_current_session, reset};
use crate::foundation::{ChannelRuntimeV1, MovementLocalPosition};
use crate::gameplay_transport::connection::{
    FirstEntryOutcome, FreshAdmissionAuthority, UseCommand,
};
use oteryn_gameplay_entities::content_adapters::NativeGameplayRuntimeExt;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use std::sync::Arc;
#[test]
fn engenc2_nomads_native_use_refuses_missing_ground_before_quest_receipt() -> TestResult {
    if std::env::var_os("OTERYN_TEST_POSTGRES_ADMIN_URL").is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    let result = super::kill_reward_e2e::scenario_thread::run("engenc2-nomads-use", || {
        run(async |admin| {
            reset();
            let h = Harness::create_with_admission_seed(admin, "engenc2_nomads_use", true, false)
                .await?;
            prepare_current_session(&h).await?;
            let callback = oteryn_gameplay_quests::quest::world_callbacks::load()?
                .into_iter()
                .find(|c| c.key == "oteryn:quest-callback/engenc2/nomads-land-window")
                .ok_or("production window")?;
            let mut deps = Dependencies::new()?;
            let (controller, activated) =
                super::death_ml_loss_composed::production_generation_with_pin(
                    deps.admitted.world_id,
                    "content-1",
                )?;
            let generation = controller.active().ok_or("active generation")?;
            let native = generation.native_gameplay().ok_or("native profiles")?;
            let world = super::l6_world_pickups::native_crop(
                native,
                super::l6_world_pickups::NativeCrop {
                    bytes: include_bytes!(concat!(
                        env!("CARGO_MANIFEST_DIR"),
                        "/../../../content/world/placements/region-z09-x129-y128.b3"
                    )),
                    region: (9, 129, 128),
                    bounds: (33213, 32852, 33220, 32859),
                    start: (33216, 32856),
                    expected: vec![super::l6_world_pickups::ExpectedTile {
                        position: (33216, 32855),
                        ids: vec![7358, 1744],
                        counted_entry: None,
                    }],
                },
            )?;
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
                        x: 33216,
                        y: 32856,
                        floor: 9,
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
                (33216, 32856, 9),
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
                        x: 33216,
                        y: 32856,
                        floor: 9,
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

            let key = oteryn_world_bundle::bundle::placement_key(-9, 33216, 32855, 1)
                .ok_or("window placement")?;
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
                        placement,
                        expected_revision: 0,
                    },
                )
                .await;
            assert_eq!(
                outcome.disposition,
                UseDisposition::Rejected,
                "the actual window must reach its physical Ground guard"
            );
            let receipts: i64 =
                sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts")
                    .fetch_one(&h.pool)
                    .await?;
            assert_eq!(
                receipts, 0,
                "a refused arrangement must create no quest receipt"
            );

            // Initial physical fixtures follow the existing item harness convention.
            // Re-enable all guards before observing the actual USE/quest transaction.
            let mut seed = h.pool.begin().await?;
            sqlx::query("SET LOCAL session_replication_role=replica")
                .execute(&mut *seed)
                .await?;
            let mut number = 150_u8;
            let content = format!(
                "sha256:{}",
                generation
                    .identity()
                    .server_artifact_digest()
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            );
            for requirement in &callback.world_requirements {
                let oteryn_gameplay_quests::quest::world_callbacks::WorldRequirement::Ground {
                    position,
                    ground,
                } = requirement
                else {
                    return Err("Ground pattern".into());
                };
                let mut spatial = i32::from(position.x).to_be_bytes().to_vec();
                spatial.extend_from_slice(&i32::from(position.y).to_be_bytes());
                spatial.extend_from_slice(&i16::from(position.floor).to_be_bytes());
                for item in &ground.stacks {
                    number += 1;
                    sqlx::query("INSERT INTO game_item_instances(item_instance_id,world_id,definition_family,definition_production_key,definition_revision_ref,quantity,lifecycle,minted_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'Item',$3,$4,1,1,encode($5,'hex')::uuid)")
    .bind(id(number).as_slice()).bind(id(42).as_slice()).bind(&item.item).bind(&item.revision).bind(id(160).as_slice()).execute(&mut *seed).await?;
                    sqlx::query("INSERT INTO game_item_ground_locations(item_instance_id,world_id,channel_id,runtime_scope_ownership_generation,spatial_position,corpse_ref,map_revision,content_revision,native_room_placement_context,stack_ordinal) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,$4,$5,$6,$7,$8,$9)")
    .bind(id(number).as_slice()).bind(id(42).as_slice()).bind(id(43).as_slice()).bind(&spatial).bind(vec![1_u8;16]).bind(world.map_revision()).bind(&content).bind(frame.as_slice()).bind(i64::from(number)).execute(&mut *seed).await?;
                }
            }
            seed.commit().await?;
            // A refused USE still starts the normal owner-clock cooldown.
            owner.clock_origin = owner
                .clock_origin
                .checked_sub(std::time::Duration::from_secs(3))
                .ok_or("owner clock")?;
            let mut placement = world.base().digest().to_vec();
            placement.extend_from_slice(&key.to_be_bytes());
            let accepted = owner
                .use_object(
                    deps.actor,
                    UseCommand {
                        game_session_id: admitted.game_session_id,
                        command_id: 2,
                        item_fence: admitted.item_fence,
                    },
                    WorldObjectTarget {
                        placement,
                        expected_revision: 0,
                    },
                )
                .await;
            assert_eq!(
                accepted.disposition,
                UseDisposition::Committed,
                "the same real USE must commit once physical Ground matches"
            );
            let count: i64 = sqlx::query_scalar(
                "SELECT count(*) FROM game_character_quest_receipts WHERE transition_key=$1",
            )
            .bind(&callback.key)
            .fetch_one(&h.pool)
            .await?;
            assert_eq!(count, 1);
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        })
    });
    eprintln!(
        "ENGENC2_NOMADS_USE_PG_EXECUTED elapsed_s={:.6} success={}",
        started.elapsed().as_secs_f64(),
        result.is_ok()
    );
    result
}
