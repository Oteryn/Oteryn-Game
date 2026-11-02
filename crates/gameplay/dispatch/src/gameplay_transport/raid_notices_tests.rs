use super::*;
use crate::foundation::{ChannelId, ScopeOwnershipGeneration};

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn id(value: u16) -> [u8; 16] {
    let mut bytes = [1; 16];
    bytes[0..2].copy_from_slice(&value.to_be_bytes());
    bytes[6] = 0x70;
    bytes[8] = 0x80;
    bytes
}

fn session(value: u16) -> TestResult<GameSessionId> {
    GameSessionId::decode(&id(value)).map_err(|error| format!("{error:?}").into())
}

fn step(ordinal: u32) -> TestResult<RaidStepIdentity> {
    Ok(RaidStepIdentity {
        firing_id: id(500),
        channel: ChannelId::decode(&id(501)).map_err(|error| format!("{error:?}"))?,
        generation: ScopeOwnershipGeneration::new(1).map_err(|error| format!("{error:?}"))?,
        ordinal,
    })
}

fn warning() -> ServerMessage {
    ServerMessage::Warning("Rats are invading!".into())
}

fn connected(tag: u16, generation: u64) -> TestResult<AdmittedSession> {
    use crate::foundation::{AuthenticatedTransportRefV1, ExactActorRef, WorldId};
    use crate::gameplay_transport::capabilities::{
        PRODUCTION_OFFERED_CAPABILITIES, SelectedCapabilities,
    };
    use crate::gameplay_transport::connection::{
        ControllerBinding, FirstEntryOutcome, SessionContinuity,
    };
    let world_id = WorldId::decode(&id(600))?;
    let channel_id = ChannelId::decode(&id(601))?;
    let mut continuity = SessionContinuity::FRESH;
    continuity.connection_generation = generation;
    continuity.selected_capabilities = SelectedCapabilities::select(
        PRODUCTION_OFFERED_CAPABILITIES,
        &[21, CAPABILITY_ZONES_NOLOGOUT_V1],
    )
    .ok_or("capability selection")?;
    Ok(AdmittedSession {
        game_session_id: session(tag)?,
        world_id,
        channel_id,
        runtime_actor: Some(ExactActorRef::transport_fixture(world_id, channel_id)),
        first_entry: FirstEntryOutcome::Positioned,
        controller: Some(ControllerBinding {
            transport: AuthenticatedTransportRefV1::decode(&id(602))
                .map_err(|error| format!("{error:?}"))?,
            account_id: id(603),
        }),
        continuity,
        item_fence: None,
    })
}

fn publish(hub: &ServerMessageHub, ordinal: u32) -> TestResult {
    hub.publish(step(ordinal)?, warning())
        .map_err(|()| "publication refused".into())
}

#[test]
fn raid_notice_snapshots_current_recipients_and_replay_never_reaches_late_joiners() -> TestResult {
    let hub = ServerMessageHub::default();
    let (a, b, late) = (session(1)?, session(2)?, session(3)?);
    assert!(hub.join(a, 1));
    assert!(hub.join(b, 1));
    publish(&hub, 0)?;
    assert_eq!(hub.take(a, 1), Some(warning()));
    assert_eq!(hub.take(b, 1), Some(warning()));
    assert!(hub.join(late, 1));
    publish(&hub, 0)?;
    assert_eq!(hub.take(a, 1), None);
    assert_eq!(hub.take(b, 1), None);
    assert_eq!(hub.take(late, 1), None);
    assert!(
        hub.publish(step(0)?, ServerMessage::Advance("changed kind".into()))
            .is_err()
    );
    publish(&hub, 1)?;
    assert_eq!(hub.take(late, 1), Some(warning()));
    Ok(())
}

#[test]
fn raid_notice_activity_owner_shares_origin_channel_and_other_channels_stay_isolated() -> TestResult
{
    let channel = crate::gameplay_transport::channel_states::ChannelSessionStates::default();
    let instance = channel.for_activity();
    let other = crate::gameplay_transport::channel_states::ChannelSessionStates::default();
    let (outside, inside, elsewhere) = (session(1)?, session(2)?, session(3)?);
    assert!(channel.server_messages.join(outside, 1));
    assert!(instance.server_messages.join(inside, 1));
    assert!(other.server_messages.join(elsewhere, 1));
    publish(&channel.server_messages, 0)?;
    assert_eq!(channel.server_messages.take(outside, 1), Some(warning()));
    assert_eq!(instance.server_messages.take(inside, 1), Some(warning()));
    assert_eq!(other.server_messages.take(elsewhere, 1), None);
    instance.server_messages.leave(inside, 1);
    drop(instance);
    publish(&channel.server_messages, 1)?;
    assert_eq!(channel.server_messages.take(outside, 1), Some(warning()));
    Ok(())
}

#[test]
fn raid_notice_connection_replacement_drops_backlog_without_old_leave_erasing_successor()
-> TestResult {
    let hub = ServerMessageHub::default();
    let a = session(1)?;
    assert!(hub.join(a, 1));
    publish(&hub, 0)?;
    assert!(hub.join(a, 2));
    hub.leave(a, 1);
    assert_eq!(hub.take(a, 2), None);
    publish(&hub, 0)?;
    assert_eq!(hub.take(a, 2), None);
    publish(&hub, 1)?;
    assert_eq!(hub.take(a, 1), None);
    assert_eq!(hub.take(a, 2), Some(warning()));
    hub.leave(a, 2);
    publish(&hub, 2)?;
    assert!(hub.join(a, 3));
    assert_eq!(hub.take(a, 3), None);
    Ok(())
}

#[test]
fn raid_notice_delayed_older_join_preserves_current_connection_and_backlog() -> TestResult {
    let hub = ServerMessageHub::default();
    let a = session(1)?;
    assert!(hub.join(a, 3));
    publish(&hub, 0)?;
    assert!(!hub.join(a, 2));
    hub.leave(a, 2);
    assert_eq!(hub.take(a, 2), None);
    assert_eq!(hub.take(a, 3), Some(warning()));
    assert!(hub.join(a, 3));
    publish(&hub, 1)?;
    assert_eq!(hub.take(a, 3), Some(warning()));
    Ok(())
}

#[test]
fn raid_notice_connected_handoff_preserves_pending_delivery_for_the_physical_connection()
-> TestResult {
    let hub = ServerMessageHub::default();
    let (source, next) = (connected(1, 3)?, connected(2, 1)?);
    assert!(hub.join(source.game_session_id, 3));
    publish(&hub, 0)?;
    hub.transfer_connection(source, next)
        .map_err(|()| "handoff refused")?;
    hub.leave(source.game_session_id, 2);
    assert!(hub.join(next.game_session_id, 1));
    assert_eq!(hub.take(source.game_session_id, 3), None);
    assert_eq!(hub.take(next.game_session_id, 1), Some(warning()));
    hub.transfer_connection(source, next)
        .map_err(|()| "handoff replay refused")?;
    publish(&hub, 0)?;
    assert_eq!(hub.take(next.game_session_id, 1), None);
    publish(&hub, 1)?;
    assert_eq!(hub.take(next.game_session_id, 1), Some(warning()));
    hub.leave(source.game_session_id, 3);
    assert!(
        hub.state
            .lock()
            .map_err(|_| "hub lock")?
            .recipients
            .is_empty()
    );
    Ok(())
}

#[test]
fn raid_notice_handoff_never_replays_to_disconnected_or_changed_controller() -> TestResult {
    use crate::foundation::AuthenticatedTransportRefV1;
    let hub = ServerMessageHub::default();
    let (source, next) = (connected(1, 3)?, connected(2, 1)?);
    assert!(hub.join(source.game_session_id, 3));
    publish(&hub, 0)?;
    let mut stale = source;
    stale.continuity.connection_generation = 2;
    assert!(hub.transfer_connection(stale, next).is_err());
    let mut changed = next;
    changed.controller.as_mut().ok_or("controller")?.transport =
        AuthenticatedTransportRefV1::decode(&id(604)).map_err(|error| format!("{error:?}"))?;
    hub.transfer_connection(source, changed)
        .map_err(|()| "changed controller refusal")?;
    assert_eq!(hub.take(next.game_session_id, 1), None);
    assert_eq!(hub.take(source.game_session_id, 3), Some(warning()));
    hub.leave(source.game_session_id, 3);
    hub.transfer_connection(source, next)
        .map_err(|()| "absent source refusal")?;
    publish(&hub, 1)?;
    assert!(hub.join(next.game_session_id, 1));
    assert_eq!(hub.take(next.game_session_id, 1), None);
    Ok(())
}

#[test]
fn raid_notice_capacity_is_atomic_and_disconnect_releases_recipient_queue() -> TestResult {
    let hub = ServerMessageHub::default();
    let (slow, fresh) = (session(1)?, session(2)?);
    assert!(hub.join(slow, 1));
    for ordinal in 0..MAX_ANNOUNCEMENTS {
        publish(&hub, u32::try_from(ordinal)?)?;
    }
    assert!(hub.join(fresh, 1));
    let next = u32::try_from(MAX_ANNOUNCEMENTS)?;
    assert!(hub.publish(step(next)?, warning()).is_err());
    assert_eq!(hub.take(fresh, 1), None);
    hub.leave(slow, 1);
    publish(&hub, next)?;
    assert_eq!(hub.take(fresh, 1), Some(warning()));
    assert_eq!(hub.take(fresh, 1), None);
    Ok(())
}

#[test]
fn raid_notice_recipient_bound_matches_listener_and_reuses_departed_entries() -> TestResult {
    let hub = ServerMessageHub::default();
    for index in 0..MAX_RECIPIENTS {
        assert!(hub.join(session(u16::try_from(index)?)?, 1));
    }
    assert!(!hub.join(session(256)?, 1));
    assert!(hub.join(session(0)?, 2));
    hub.leave(session(0)?, 1);
    assert!(!hub.join(session(256)?, 1));
    hub.forget(session(0)?);
    assert!(hub.join(session(256)?, 1));
    assert!(!hub.join(session(257)?, 0));
    Ok(())
}

#[test]
fn raid_notice_receipt_limit_preserves_replay_and_refuses_new_identity() -> TestResult {
    let hub = ServerMessageHub::default();
    for ordinal in 0..MAX_RECEIPTS {
        publish(&hub, u32::try_from(ordinal)?)?;
    }
    publish(&hub, 0)?;
    assert!(
        hub.publish(step(u32::try_from(MAX_RECEIPTS)?)?, warning())
            .is_err()
    );
    assert!(hub.join(session(1)?, 1));
    publish(&hub, 0)?;
    assert_eq!(hub.take(session(1)?, 1), None);
    Ok(())
}

#[test]
fn raid_notice_acknowledgement_reclaims_receipt_capacity_and_retains_queued_message() -> TestResult
{
    let hub = ServerMessageHub::default();
    for ordinal in 0..MAX_RECEIPTS {
        publish(&hub, u32::try_from(ordinal)?)?;
    }
    let next = u32::try_from(MAX_RECEIPTS)?;
    assert!(hub.publish(step(next)?, warning()).is_err());
    hub.acknowledge(step(0)?)
        .map_err(|()| "acknowledgement refused")?;
    let a = session(1)?;
    assert!(hub.join(a, 1));
    publish(&hub, next)?;
    hub.acknowledge(step(next)?)
        .map_err(|()| "acknowledgement refused")?;
    hub.acknowledge(step(next)?)
        .map_err(|()| "replayed acknowledgement refused")?;
    assert_eq!(hub.take(a, 1), Some(warning()));
    assert_eq!(hub.take(a, 1), None);
    publish(&hub, next + 1)?;
    assert_eq!(hub.take(a, 1), Some(warning()));
    Ok(())
}

#[test]
fn raid_notice_terminal_ack_reclaims_uncertain_receipts_across_generations_without_dropping_queue()
-> TestResult {
    let hub = ServerMessageHub::default();
    for ordinal in 0..MAX_RECEIPTS - 2 {
        publish(&hub, u32::try_from(ordinal)?)?;
    }
    let a = session(1)?;
    assert!(hub.join(a, 1));
    publish(&hub, u32::try_from(MAX_RECEIPTS - 2)?)?;
    publish(&hub, u32::try_from(MAX_RECEIPTS - 1)?)?;
    let mut successor = step(0)?;
    successor.firing_id = id(502);
    assert!(hub.publish(successor, warning()).is_err());
    let mut lost = step(u32::try_from(MAX_RECEIPTS)?)?;
    lost.generation = ScopeOwnershipGeneration::new(2).map_err(|error| format!("{error:?}"))?;
    hub.acknowledge_end(lost)
        .map_err(|()| "terminal acknowledgement refused")?;
    assert_eq!(hub.take(a, 1), Some(warning()));
    assert_eq!(hub.take(a, 1), Some(warning()));
    assert_eq!(hub.take(a, 1), None);
    hub.publish(successor, warning())
        .map_err(|()| "successor publication refused")?;
    assert_eq!(hub.take(a, 1), Some(warning()));
    hub.acknowledge_end(lost)
        .map_err(|()| "terminal replay refused")?;
    hub.publish(successor, warning())
        .map_err(|()| "successor replay refused")?;
    assert_eq!(hub.take(a, 1), None);
    Ok(())
}

#[test]
fn raid_notice_wakes_outbound_without_a_timer_and_survives_cancelled_poll() -> TestResult {
    use std::future::Future;
    use std::task::{Context, Poll, Waker};
    let hub = ServerMessageHub::default();
    let a = session(1)?;
    assert!(hub.join(a, 1));
    let mut context = Context::from_waker(Waker::noop());
    {
        let mut waiting = std::pin::pin!(hub.wait(a, 1));
        assert!(matches!(waiting.as_mut().poll(&mut context), Poll::Pending));
    }
    publish(&hub, 0)?;
    let mut waiting = std::pin::pin!(hub.wait(a, 1));
    assert!(matches!(
        waiting.as_mut().poll(&mut context),
        Poll::Ready(())
    ));
    assert_eq!(hub.take(a, 1), Some(warning()));
    let mut waiting = std::pin::pin!(hub.wait(a, 1));
    assert!(matches!(waiting.as_mut().poll(&mut context), Poll::Pending));
    publish(&hub, 1)?;
    assert!(matches!(
        waiting.as_mut().poll(&mut context),
        Poll::Ready(())
    ));
    Ok(())
}

include!("raid_notices_connection_tests.rs");
