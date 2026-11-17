//! Two real Character sessions on independently assigned Channels share one World.
use super::chat_world_fixture::admit_second_node_with_pin;
use super::*;
use crate::durability::content_activation::ContentActivationRequest;
use crate::durability::quest_world_callback::QuestWorldContext;
use oteryn_gameplay_quests::quest::world_callbacks::{self, Position};
use oteryn_gameplay_world_change::mummys_curse::{
    BROKEN_JAR, HORESTIS_SPAWNS, INTACT_JAR, JARS, JarOutcome,
};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use sha2::{Digest, Sha256};

#[test]
fn cbchance_mummys_curse_shares_progress_and_keeps_character_cooldowns_separate() -> TestResult {
    let Ok(admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(256 << 20)
        .spawn(move || {
            run(async move |_| {
                reset();
                let h = Harness::create_with_admission_seed(
                    admin,
                    "cbchance_mummys_curse",
                    true,
                    false,
                )
                .await?;
                prepare_current_session(&h).await?;
                let first = Dependencies::new()?;
                let seal = h.recovery.seal_current().map_err(debug)?;
                let authority = h
                    .root
                    .open_character_authority(&seal)
                    .await
                    .map_err(debug)?;
                let (controller, activated) =
                    super::death_ml_loss_composed::production_generation_with_pin(
                        first.admitted.world_id,
                        "content-1",
                    )?;
                let (_, x, y, floor) = JARS[0];
                let jar = Position { x, y, floor };
                let mut fixture = world_callbacks::load()
                    .map_err(debug)?
                    .into_iter()
                    .find(|callback| world_callbacks::is_chyllfroest_entry(&callback.key))
                    .ok_or("physical callback fixture")?;
                fixture.selector.appearance = Some(INTACT_JAR);
                fixture.selector.action = Some(JARS[0].0);
                let world = super::quest_world_callbacks::world_with_objects(
                    &fixture,
                    jar,
                    &[],
                    &[BROKEN_JAR],
                )?;
                let cells = world
                    .movement_cells(first.room.movement_cells())
                    .map_err(debug)?;
                let (final_action, fx, fy, ff) = JARS[4];
                let final_jar = Position {
                    x: fx,
                    y: fy,
                    floor: ff,
                };
                fixture.selector.action = Some(final_action);
                let spawns = HORESTIS_SPAWNS.map(|(x, y, floor, _)| Position { x, y, floor });
                let final_world = super::quest_world_callbacks::world_with_objects_and_cells(
                    &fixture,
                    final_jar,
                    &[],
                    &[BROKEN_JAR],
                    &spawns,
                )?;
                let final_cells = final_world
                    .movement_cells(activated.qualified_room().movement_cells())
                    .map_err(debug)?;
                let generation = controller.active().ok_or("native generation")?;
                let pin = crate::content::channel_content_pin::ChannelContentPin::from_activation(
                    first.admitted.world_id,
                    1,
                    generation.identity().server_artifact_digest(),
                    generation.identity().client_artifact_digest(),
                    final_world.frame_binding_digest(first.admitted.world_id),
                    final_world.map_revision_digest(),
                    (i32::from(fx) - 1, i32::from(fy), -i16::from(ff)),
                );
                let (second_node, second) = admit_second_node_with_pin(&h, pin).await?;
                let mut left = first.owner(&h, &authority);
                left.active_generation = controller.active();
                left.movement_cells = (&cells).into();
                let mut right = second.owner_from(&h.root, &second_node, &authority);
                right.active_generation = Some(generation);
                right.movement_cells = (&final_cells).into();
                assert_ne!(left.channel_id, right.channel_id);
                let (left_fence, left_context) = activate(&h, &first, &left).await?;
                let (right_fence, right_context) = activate(&h, &second, &right).await?;
                {
                    let mut runtime = first.runtime.lock().await;
                    runtime
                        .initialize_pinned_test_position(
                            first.actor,
                            crate::foundation::MovementLocalPosition {
                                x: i32::from(x) - 1,
                                y: i32::from(y),
                                floor: -i16::from(floor),
                            },
                        )
                        .map_err(debug)?;
                    runtime
                        .bind_attacker_lease(
                            first.actor,
                            first.admitted.game_session_id,
                            crate::foundation::CharacterLease::new(
                                crate::foundation::CharacterId::decode(
                                    left_fence.character_id.as_bytes(),
                                )
                                .map_err(debug)?,
                                left_fence.character_lease_generation,
                            )
                            .map_err(debug)?,
                        )
                        .map_err(debug)?;
                }
                let session = first.admitted.game_session_id;
                let command_id = (1_u64..=32)
                    .find(|id| {
                        Sha256::new()
                            .chain_update(b"oteryn.mummys_curse.use.v1")
                            .chain_update(session.as_bytes())
                            .chain_update(id.to_be_bytes())
                            .chain_update(JARS[0].0.to_be_bytes())
                            .finalize()[0]
                            >= 32
                    })
                    .ok_or("failed command draw")?;
                let mut placement = world.base().digest().to_vec();
                placement.extend_from_slice(
                    &oteryn_world_bundle::bundle::placement_key(floor, x, y, 1)
                        .ok_or("jar key")?
                        .to_be_bytes(),
                );
                let target = WorldObjectTarget {
                    placement,
                    expected_revision: 0,
                };
                let before = super::chat_world_fixture::clock(&h).await?;
                let outcome = left
                    .use_quest_callback(
                        first.actor,
                        crate::gameplay_transport::connection::UseCommand {
                            game_session_id: session,
                            command_id,
                            item_fence: Some(left_fence.item_fence()),
                        },
                        left.bundle_map().ok_or("jar map")?,
                        &target,
                    )
                    .await
                    .ok_or("jar dispatcher")?;
                assert_eq!(outcome.disposition, UseDisposition::Committed);
                let retry_at: i64 =
                    sqlx::query_scalar("SELECT retry_at FROM game_mummys_curse_retries")
                        .fetch_one(&h.pool)
                        .await?;
                let after = super::chat_world_fixture::clock(&h).await?;
                assert!((before + 1800..=after + 1800).contains(&retry_at));

                let left_permit = left.spell_lane_permit().await.ok_or("left permit")?;
                let right_permit = right.spell_lane_permit().await.ok_or("right permit")?;
                let left_use = async |action, roll| {
                    h.root
                        .use_mummys_curse_jar(
                            &authority,
                            &h.node,
                            &left_permit,
                            left_fence,
                            left_context.clone(),
                            action,
                            roll,
                        )
                        .await
                };
                let right_use = async |action, roll| {
                    h.root
                        .use_mummys_curse_jar(
                            &authority,
                            &second_node,
                            &right_permit,
                            right_fence,
                            right_context.clone(),
                            action,
                            roll,
                        )
                        .await
                };

                assert_eq!(left_use(50006, 1).await?, (JarOutcome::CoolingDown, 0));
                assert_eq!(right_use(50007, 1).await?, (JarOutcome::Prerequisite, 0));
                assert_eq!(
                    right_use(50006, 1).await?,
                    (JarOutcome::Broken { progress: 1 }, 1)
                );
                assert_eq!(
                    left_use(50007, 1).await?,
                    (JarOutcome::Broken { progress: 2 }, 2)
                );
                assert_eq!(
                    right_use(50008, 1).await?,
                    (JarOutcome::Broken { progress: 3 }, 3)
                );
                assert_eq!(
                    left_use(50009, 1).await?,
                    (JarOutcome::Broken { progress: 4 }, 4)
                );
                drop(right_permit);
                let mut placement = final_world.base().digest().to_vec();
                placement.extend_from_slice(
                    &oteryn_world_bundle::bundle::placement_key(ff, fx, fy, 1)
                        .ok_or("final jar key")?
                        .to_be_bytes(),
                );
                let command = crate::gameplay_transport::connection::UseCommand {
                    game_session_id: second.admitted.game_session_id,
                    command_id: 1,
                    item_fence: Some(right_fence.item_fence()),
                };
                let final_target = WorldObjectTarget {
                    placement,
                    expected_revision: 0,
                };
                let result = right
                    .use_quest_callback(
                        second.actor,
                        command,
                        right.bundle_map().ok_or("final jar map")?,
                        &final_target,
                    )
                    .await
                    .ok_or("final USE dispatch")?;
                assert_eq!(result.disposition, UseDisposition::Committed);
                assert_eq!(
                    spawn_positions(&right).await?,
                    vec![super::quest_world_callbacks::local(spawns[0])]
                );
                right.clock_origin = right
                    .clock_origin
                    .checked_sub(std::time::Duration::from_secs(45))
                    .ok_or("spawn clock")?;
                right.drain_quest_encounter_deadlines().await;
                let positions = spawn_positions(&right).await?;
                assert_eq!(positions.len(), 2);
                assert!(
                    spawns
                        .into_iter()
                        .all(|pos| positions.contains(&super::quest_world_callbacks::local(pos)))
                );
                right.drain_quest_encounter_deadlines().await;
                assert_eq!(spawn_positions(&right).await?, positions);
                assert_eq!(left_use(50010, 1).await?, (JarOutcome::Refused, 5));
                assert_eq!(
                    sqlx::query_scalar::<_, i64>("SELECT count(*) FROM game_mummys_curse_retries")
                        .fetch_one(&h.pool)
                        .await?,
                    1
                );
                Ok(())
            })
            .map_err(|error| error.to_string())
        })?
        .join()
        .map_err(|_| "Crystal jar test thread panicked")??;
    Ok(())
}

async fn spawn_positions(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
) -> TestResult<Vec<crate::foundation::MovementLocalPosition>> {
    Ok(owner
        .runtime
        .lock()
        .await
        .positioned_melee_census()
        .map_err(debug)?
        .into_iter()
        .filter_map(|(_, position, session)| session.is_none().then_some(position.position()))
        .collect())
}

async fn activate(
    h: &Harness,
    deps: &Dependencies,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
) -> TestResult<(
    crate::durability::character_progression::CurrentCharacterGameplayFence,
    QuestWorldContext,
)> {
    let world_id = deps.admitted.world_id;
    let channel_id = deps.admitted.channel_id;
    sqlx::query("INSERT INTO game_control_scope_grants(control_role,world_id,channel_id,operation) VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice())
        .execute(&h.pool).await?;
    let context = {
        let runtime = deps.runtime.lock().await;
        let pin = runtime.content_pin();
        let server_artifact_digest = pin.server_artifact_digest();
        let client_artifact_digest = pin.client_artifact_digest();
        let frame_binding_digest = pin.frame_binding_digest();
        drop(runtime);
        assert!(
            h.root
                .record_content_activation(&ContentActivationRequest {
                    world_id,
                    channel_id,
                    activation_sequence: 1,
                    previous_sequence: None,
                    server_artifact_digest,
                    client_artifact_digest,
                    frame_binding_digest,
                })
                .await
                .map_err(debug)?
        );
        QuestWorldContext {
            map_revision: deps
                .room
                .movement_cells()
                .scope()
                .map_revision
                .as_str()
                .to_owned(),
            content_revision: format!(
                "sha256:{}",
                server_artifact_digest
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>()
            ),
            placement_context: frame_binding_digest.to_vec(),
            reset_epoch: 0,
            actor_placement: deps.actor.placement_identity(),
            movement_revision: 0,
            party: Vec::new(),
        }
    };
    let fence = owner
        .current_quest_fence(deps.admitted.game_session_id)
        .await
        .map_err(|()| "gameplay fence unavailable")?
        .ok_or("gameplay fence")?;
    Ok((fence, context))
}
