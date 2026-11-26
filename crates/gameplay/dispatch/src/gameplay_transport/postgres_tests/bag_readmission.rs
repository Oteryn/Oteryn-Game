//! The first USE on a fresh successor must open the Character's persisted backpack.
use super::TestResult;
use super::full_world_support::world_scenario;
use crate::gameplay_transport::connection::{
    AdmittedSession, ControllerBinding, SessionContinuity, regression_bag_use_with_busy_holder,
};
use oteryn_protocol_oteryn::{self as protocol, container_tree, world_object};

#[test]
fn bag_use_after_readmission_opens_container_postgres() -> TestResult {
    let Ok(_admin) = std::env::var("OTERYN_TEST_POSTGRES_ADMIN_URL") else {
        return Ok(());
    };
    super::kill_reward_e2e_fixtures::scenario_thread::run("bag-readmission", scenario)
}

fn scenario() -> TestResult {
    world_scenario!(
        "bag_readmission",
        |h,
         _request,
         seal,
         authority,
         world_id,
         native,
         world,
         cells,
         room,
         pin,
         deps,
         actor,
         session,
         owner,
         fence,
         admitted| {
            assert_eq!(
                owner.logout_session(admitted).await,
                LogoutReply::Result(LogoutResult::Accepted)
            );
            let successor = relogin(&h).await?;
            assert_ne!(successor, session);
            let current = FreshAdmissionStore::from_root(h.root.clone())
                .current_session(successor)
                .await?;
            let next_fence =
                crate::gameplay_transport::item_fence_of(current).ok_or("readmitted fence")?;
            assert_eq!(next_fence.character_id, fence.character_id);
            assert!(next_fence.character_lease_generation > fence.character_lease_generation);
            let next_actor = {
                let mut runtime = deps.runtime.lock().await;
                let reservation = runtime.reserve_fresh_session(successor).map_err(debug)?;
                let next_actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
                runtime
                    .bind_attacker_lease(next_actor, successor, current.current_character_lease())
                    .map_err(debug)?;
                next_actor
            };
            let gameplay = owner
                .current_quest_fence(successor)
                .await
                .map_err(debug)?
                .ok_or("readmitted gameplay fence")?;
            let position = owner
                .read_admission_position(gameplay)
                .await
                .map_err(debug)?;
            assert!(matches!(position, AdmissionPosition::Stored(_)));
            {
                let mut runtime = deps.runtime.lock().await;
                owner
                    .initialize_admitted_position(&mut runtime, next_actor, successor, &position)
                    .await
                    .map_err(debug)?;
            }
            assert!(owner.track_restored(successor, next_actor, &position, gameplay));
            deps.initialize_player(&owner, next_actor, successor).await;
            let mut readmitted = AdmittedSession {
                game_session_id: successor,
                runtime_actor: Some(next_actor),
                controller: Some(ControllerBinding {
                    transport: current.current_transport().ok_or("readmitted transport")?,
                    account_id: admitted.controller.ok_or("original controller")?.account_id,
                }),
                item_fence: Some(next_fence),
                continuity: SessionContinuity {
                    selected_capabilities: SelectedCapabilities::select(
                        crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
                        &[4, 6, 12, 14, 15],
                    )
                    .ok_or("bag USE capabilities")?,
                    ..SessionContinuity::FRESH
                },
                ..admitted
            };
            assert!(
                readmitted
                    .continuity
                    .selected_capabilities
                    .command_selected(world_object::COMMAND_TYPE_USE_INTENT)
            );
            owner.admit_loop_training(&mut readmitted).await;
            assert_eq!(readmitted.first_entry, FirstEntryOutcome::Positioned);
            let bag: Vec<u8> = sqlx::query_scalar(
                "SELECT uuid_send(item_instance_id) FROM game_item_container_slots
                 WHERE character_id = encode($1, 'hex')::uuid",
            )
            .bind(next_fence.character_id.as_bytes().as_slice())
            .fetch_one(&h.pool)
            .await?;
            let (mut server, mut client) = tokio::io::duplex(1 << 20);
            let handle = tokio::time::timeout(
                Duration::from_secs(20),
                regression_bag_use_with_busy_holder(
                    &owner,
                    readmitted,
                    &mut server,
                    ItemKey::Instance(bag.try_into().map_err(|_| "invalid bag identity")?),
                ),
            )
            .await??;
            drop(server);
            let mut replied = false;
            loop {
                let frame = super::item_look_fixtures::read_frame(&mut client).await?;
                let envelope = protocol::decode_wire_envelope(&frame).map_err(debug)?;
                match envelope.message_type() {
                    protocol::MessageType::CommandResult => {
                        let reply =
                            protocol::decode_command_result(envelope.payload()).map_err(debug)?;
                        assert_eq!(reply.command_id, readmitted.continuity.next_command_id);
                        assert_eq!(reply.status, protocol::CommandStatus::Accepted);
                        assert_eq!(
                            world_object::decode_use_result(reply.payload).map_err(debug)?,
                            world_object::UseDisposition::Committed
                        );
                        replied = true;
                    }
                    protocol::MessageType::StateDelta => {
                        let delta =
                            protocol::decode_state_delta(envelope.payload()).map_err(debug)?;
                        if delta.domain_id == container_tree::STATE_DOMAIN_CONTAINER_VIEWS {
                            let views = container_tree::decode_container_views(delta.payload)
                                .map_err(debug)?;
                            assert!(replied, "the container update follows the typed USE result");
                            assert_eq!(views.views.len(), 1);
                            assert_eq!(views.views[0].container_handle, handle);
                            assert_eq!(views.views[0].entries.len(), 1, "persisted bag contents");
                            break;
                        }
                    }
                    _ => {}
                }
            }
            drop(owner);
            drop(authority);
            drop(seal);
            h.cleanup().await
        }
    )
}
