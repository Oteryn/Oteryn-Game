//! Genuine independently admitted Party member and forced Player movement through Ahau's exit.
use super::TestResult;
use super::fixture::{Harness, debug, id};
use crate::foundation::{MovementLocalPosition, RuntimeScopeRefV1};
use crate::gameplay_transport::{
    ComposedFreshAdmission,
    connection::{AdmittedSession, ControllerBinding, FreshAdmissionAuthority, SessionContinuity},
};

#[test]
fn m25_push_peer_uses_genuine_fresh_admission_postgres() -> TestResult {
    if crate::bestiary_postgres_harness::configured_admin().is_none() {
        return Ok(());
    }
    let started = std::time::Instant::now();
    super::kill_reward_e2e::scenario_thread::run("m25-push-peer-fresh", || {
        super::fixture::run(async |admin| {
            let h = Harness::create_with_admission_seed(admin, "m25_push_peer", false, false)
                .await?;
            let _source = super::house_transfer::RecoverySource::start(&h).await?;
            super::house_transfer::prepare_owned_session(&h)
                .await
                .map_err(|error| format!("M25 first Fresh admission: {error}"))?;
            let now = sqlx::query_scalar::<_, i64>(
                "SELECT floor(extract(epoch FROM clock_timestamp()))::bigint",
            )
            .fetch_one(&h.pool)
            .await?;
            crate::world_party_runtime_postgres_cases::committed_fresh_player_from_source_owned(
                &crate::world_party_runtime_postgres_cases::FreshPlayerOwners {
                    root: h.root.clone(),
                    node: h.node.clone(),
                    recovery: &h.recovery,
                },
                60,
                63,
                171,
                now,
                Some("platform-test"),
            )
            .await
            .map_err(|error| format!("M25 second Fresh admission: {error}"))?;
            h.cleanup().await
        })
    })?;
    eprintln!("M25_PUSH_PEER_FRESH_COMPLETED duration={:?}", started.elapsed());
    Ok(())
}

pub(super) async fn prepare_peer_and_party(
    h: &Harness,
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    leader: &mut AdmittedSession,
) -> TestResult<AdmittedSession> {
    let now: i64 =
        sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp()))::bigint")
            .fetch_one(&h.pool)
            .await?;
    let (current, operation) =
        crate::world_party_runtime_postgres_cases::committed_fresh_player_from_source_owned(
            &crate::world_party_runtime_postgres_cases::FreshPlayerOwners {
                root: h.root.clone(),
                node: h.node.clone(),
                recovery: &h.recovery,
            },
            60,
            63,
            171,
            now,
            Some("platform-test"),
        )
        .await
        .map_err(|error| format!("M25 peer Fresh admission: {error}"))?;
    let fence = crate::gameplay_transport::item_fence_of(current).ok_or("actual peer fence")?;
    let actor = {
        let _permit = owner
            .spell_lane_permit()
            .await
            .ok_or("actual source lane")?;
        let mut runtime = owner.runtime.lock().await;
        let reservation = runtime
            .reserve_fresh_session(fence.game_session_id)
            .map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        runtime
            .bind_attacker_lease(
                actor,
                fence.game_session_id,
                current.current_character_lease(),
            )
            .map_err(debug)?;
        actor
    };
    let mut continuity = SessionContinuity::FRESH;
    continuity.selected_capabilities = leader.continuity.selected_capabilities;
    let mut peer = AdmittedSession {
        game_session_id: fence.game_session_id,
        world_id: leader.world_id,
        channel_id: leader.channel_id,
        runtime_actor: Some(actor),
        first_entry: crate::gameplay_transport::connection::FirstEntryOutcome::Positioned,
        controller: Some(ControllerBinding {
            transport: current.current_transport().ok_or("actual peer transport")?,
            account_id: id(63),
        }),
        continuity,
        item_fence: Some(fence),
    };
    super::srx3_activity_source_fixture::prepare_player_with_operation(
        h, owner, &mut peer, 1999, "knight", &operation,
    )
    .await
    .map_err(|error| format!("M25 peer progression preparation: {error}"))?;
    // The second genuine waiting pad, reached through the same physical movement owner.
    let (step, _) = owner
        .paced_step(
            actor,
            peer.game_session_id,
            oteryn_protocol_oteryn::world_spatial::StepDirection::West,
        )
        .await;
    assert_eq!(
        step.disposition,
        oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved
    );
    // Use the actual Party ingress; it owns the physical proof's borrow and SQL
    // transaction together. The fixture holds no Runtime mutex across an await.
    use oteryn_protocol_oteryn::party::*;
    let mut supported = leader.continuity.selected_capabilities.as_slice().to_vec();
    supported.push(CAPABILITY_PARTY_V1);
    supported.sort_unstable();
    supported.dedup();
    let selected =
        crate::gameplay_transport::connection::fresh_capability_selection(owner, &supported)
            .await
            .ok_or("actual Party capability negotiation")?;
    owner
        .required_capabilities
        .authenticate_resume_support(&supported, selected)
        .map_err(debug)?;
    leader.continuity.selected_capabilities = selected;
    peer.continuity.selected_capabilities = selected;
    let target = {
        let runtime = owner.runtime.lock().await;
        let entry = runtime
            .visible_entities()
            .players
            .into_iter()
            .find(|entry| entry.actor == actor)
            .ok_or("actual visible Party target")?;
        oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
            identity: actor.placement_identity(),
            generation: entry.generation,
        }
    };
    let mut leader_view = crate::gameplay_transport::party_command::SessionPartyView::default();
    let invite = encode_party_intent(PartyIntent::Invite(target)).map_err(debug)?;
    let result = owner
        .party_wire_command(
            *leader,
            leader.continuity.next_command_id,
            COMMAND_TYPE_PARTY_INTENT,
            &invite,
            &mut leader_view,
        )
        .await
        .ok_or("actual Party invite reply")?;
    assert_eq!(
        decode_party_result(&result).map_err(debug)?.disposition,
        PartyDisposition::Ok
    );
    leader.continuity.next_command_id += 1;
    let mut peer_view = crate::gameplay_transport::party_command::SessionPartyView::default();
    let query = owner
        .party_wire_command(
            peer,
            peer.continuity.next_command_id,
            COMMAND_TYPE_PARTY_QUERY,
            &[],
            &mut peer_view,
        )
        .await
        .ok_or("actual Party invitation query")?;
    let query = decode_party_result(&query).map_err(debug)?;
    assert_eq!(query.disposition, PartyDisposition::Ok);
    let [invitation] = query.invitations.as_slice() else {
        return Err("one actual Party invitation".into());
    };
    peer.continuity.next_command_id += 1;
    let accept = encode_party_intent(PartyIntent::Accept(invitation.handle)).map_err(debug)?;
    let result = owner
        .party_wire_command(
            peer,
            peer.continuity.next_command_id,
            COMMAND_TYPE_PARTY_INTENT,
            &accept,
            &mut peer_view,
        )
        .await
        .ok_or("actual Party accept reply")?;
    assert_eq!(
        decode_party_result(&result).map_err(debug)?.disposition,
        PartyDisposition::Ok
    );
    peer.continuity.next_command_id += 1;
    owner
        .activity_runtimes
        .remember(*leader)
        .map_err(|_| "remember Party leader")?;
    owner
        .activity_runtimes
        .remember(peer)
        .map_err(|_| "remember Party peer")?;
    Ok(peer)
}

pub(super) async fn exercise(
    h: &Harness,
    origin: &ComposedFreshAdmission<'_, '_, '_>,
    view: &mut ComposedFreshAdmission<'_, '_, '_>,
    pusher: AdmittedSession,
    source_peer: AdmittedSession,
) -> TestResult {
    let peer = origin.activity_runtimes.current_connection(source_peer);
    let scope = pusher.item_fence.ok_or("pusher Instance")?.runtime_scope;
    assert!(matches!(scope, RuntimeScopeRefV1::Instance { .. }));
    assert_ne!(peer.game_session_id, source_peer.game_session_id);
    assert_eq!(peer.item_fence.ok_or("peer Instance")?.runtime_scope, scope);
    walk(
        view,
        peer,
        MovementLocalPosition {
            x: 34002,
            y: 31705,
            floor: 10,
        },
    )
    .await?;
    walk(
        view,
        pusher,
        MovementLocalPosition {
            x: 34002,
            y: 31704,
            floor: 10,
        },
    )
    .await?;
    let target = peer.runtime_actor.ok_or("actual pushed actor")?;
    let source_controller = peer.controller.ok_or("actual peer controller")?;
    assert_eq!(
        view.commit_control_loss(peer.game_session_id, target, source_controller)
            .await,
        crate::gameplay_transport::connection::ControlLossResult::Recorded
    );
    let (entity, retained_loss) = {
        let runtime = view.runtime.lock().await;
        assert!(
            runtime
                .bound_attacker_lease(target, peer.game_session_id)
                .is_ok(),
            "ordinary grace does not install an explicit write fence"
        );
        let facts = runtime
            .player_control_facts(target, peer.game_session_id)
            .map_err(debug)?;
        let loss = facts.control_loss.ok_or("actual retained control loss")?;
        let entity = runtime
            .visible_entities()
            .players
            .into_iter()
            .find(|entry| entry.actor == target)
            .ok_or("actual visible grace avatar")?;
        (
            oteryn_protocol_oteryn::world_spatial_entities::EntityRef {
                identity: target.placement_identity(),
                generation: entity.generation,
            },
            loss,
        )
    };
    let intent = oteryn_protocol_oteryn::creature_push::CreaturePushIntent {
        target: entity,
        destination: oteryn_protocol_oteryn::world_spatial::ActorPosition {
            x: 34002,
            y: 31706,
            floor: 10,
        },
    };
    let disposition = tokio::time::timeout(
        std::time::Duration::from_secs(10),
        view.push_creature_now(
            pusher.runtime_actor.ok_or("pusher actor")?,
            pusher.game_session_id,
            intent,
        ),
    )
    .await
    .map_err(|_| "PUSH/shared return deadlocked")?;
    assert_eq!(
        disposition,
        oteryn_protocol_oteryn::creature_push::CreaturePushDisposition::Moved
    );
    let returned = origin.activity_runtimes.current_connection(peer);
    let returned_fence = returned.item_fence.ok_or("returned peer fence")?;
    assert!(matches!(
        returned_fence.runtime_scope,
        RuntimeScopeRefV1::Channel { .. }
    ));
    assert_ne!(returned.game_session_id, peer.game_session_id);
    assert!(
        returned.controller.is_none(),
        "physical STEP return must not restore input authority"
    );
    {
        let runtime = origin.runtime.lock().await;
        let actor = returned.runtime_actor.ok_or("returned grace actor")?;
        let at = runtime
            .read_actor_position(actor)
            .map_err(debug)?
            .position();
        assert_eq!((at.x, at.y, at.floor), (34036, 31717, 10));
        let mut runtime = runtime;
        let at = crate::gameplay_transport::bundle_map_position(
            &mut runtime,
            origin.movement_cells(),
            actor,
        )
        .ok_or("actual destination FX map")?;
        let expected = crate::map::overlay::TilePos {
            x: u16::try_from(at.at.x)?,
            y: u16::try_from(at.at.y)?,
            floor: i8::try_from(at.at.floor)?,
        };
        assert_eq!(
            at.map
                .facts
                .movement_effects_since(0)
                .iter()
                .filter(|row| row.1 == expected && row.2 == 11)
                .count(),
            1,
            "one authentic destination teleport effect"
        );
        assert_eq!(
            runtime
                .player_control_facts(actor, returned.game_session_id)
                .map_err(debug)?
                .control_loss,
            Some(retained_loss)
        );
    }
    {
        let runtime = view.runtime.lock().await;
        assert_eq!(
            runtime
                .read_actor_position(pusher.runtime_actor.ok_or("pusher stays")?)
                .map_err(debug)?
                .position(),
            MovementLocalPosition {
                x: 34002,
                y: 31704,
                floor: 10
            }
        );
        assert_eq!(runtime.binding().runtime_scope(), scope);
        assert!(
            runtime.read_actor_position(target).is_err(),
            "source target retired once"
        );
    }
    let row:(i64,i64)=sqlx::query_as("SELECT count(*), count(*) FILTER(WHERE reason=1 AND command_id IS NULL AND native_step_exit IS NOT NULL) FROM game_activity_runtime_returns WHERE source_game_session_id=encode($1,'hex')::uuid").bind(peer.game_session_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(row, (1, 1));
    let movement = view
        .source_exit_movement_for_test(peer.game_session_id)
        .await
        .ok_or("actual PUSH receipt retained")?;
    assert_eq!(movement.mover(), target);
    view.source_boss_exit_movement(target, peer.game_session_id, movement)
        .await;
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_activity_runtime_returns")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        count, 1,
        "exact old physical event cannot return twice or return pusher"
    );
    super::m25_step_exit_matrix::assert_native_step_exit_matrix(h, peer.game_session_id).await?;
    eprintln!("M25_AHAU_ACTUAL_PLAYER_PUSH_GRACE_EXIT_EXECUTED");
    Ok(())
}

async fn walk(
    view: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    destination: MovementLocalPosition,
) -> TestResult {
    let actor = admitted.runtime_actor.ok_or("actual walking actor")?;
    let wire = {
        let runtime = view.runtime.lock().await;
        view.wire_observation(&runtime, destination)
            .ok_or("real route projection")?
    };
    let route = view
        .item_walk_route(
            actor,
            admitted.game_session_id,
            crate::gameplay_transport::paced_movement::AutoWalkTarget::Cell(wire.actor_position),
        )
        .await
        .map_err(|_| "actual source route")?
        .ok_or("actual walkable source route")?;
    for direction in route.steps {
        view.clock_origin = view
            .clock_origin
            .checked_sub(std::time::Duration::from_secs(1))
            .ok_or("real cadence")?;
        let (result, _) = view
            .paced_step(actor, admitted.game_session_id, direction)
            .await;
        assert_eq!(
            result.disposition,
            oteryn_protocol_oteryn::world_spatial::StepDisposition::Moved
        );
    }
    Ok(())
}
