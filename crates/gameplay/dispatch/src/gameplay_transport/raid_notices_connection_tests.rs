// Exercise the production serve loop, its Notify poll and the domain encoder.
// The fixture replaces only admission/current-state reads, not outbound routing.
struct WireNoticeAuthority {
    hub: ServerMessageHub,
    joined: tokio::sync::Notify,
}

impl FreshAdmissionAuthority for WireNoticeAuthority {
    async fn admit(
        &self,
        _attempt: crate::gameplay_transport::connection::FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, crate::gameplay_transport::connection::AdmissionRefusal> {
        Err(crate::gameplay_transport::connection::AdmissionRefusal::Rejected)
    }

    async fn observe(&self, _actor: crate::foundation::ExactActorRef) -> Option<crate::gameplay_transport::world_spatial::WorldSpatialObservation> {
        Some(crate::gameplay_transport::world_spatial::WorldSpatialObservation {
            content_generation: [0x5c; 32],
            actor_position: crate::gameplay_transport::world_spatial::ActorPosition { x: 0, y: 0, floor: 0 },
        })
    }

    fn server_messages_joined(&self, admitted: AdmittedSession) -> bool {
        let joined = !selected(admitted) || self.hub.join(admitted.game_session_id, admitted.continuity.connection_generation);
        self.joined.notify_one();
        joined
    }

    fn server_messages_left(&self, admitted: AdmittedSession) {
        self.hub.leave(admitted.game_session_id, admitted.continuity.connection_generation);
    }

    fn take_server_notice(&self, admitted: AdmittedSession) -> Option<ServerMessage> {
        if !selected(admitted) { return None; }
        self.hub.take(admitted.game_session_id, admitted.continuity.connection_generation)
    }

    async fn server_messages_changed(&self, admitted: AdmittedSession) {
        if !selected(admitted) { std::future::pending::<()>().await; }
        self.hub.wait(admitted.game_session_id, admitted.continuity.connection_generation).await;
    }
}

#[test]
fn raid_notice_cancelled_handoff_drops_destination_before_it_can_start_serving() -> TestResult {
    use std::future::Future;
    use std::task::{Context, Poll, Waker};
    let authority = WireNoticeAuthority { hub: ServerMessageHub::default(), joined: tokio::sync::Notify::new() };
    let (source, next) = (connected(1, 3)?, connected(2, 1)?);
    let mut context = Context::from_waker(Waker::noop());
    {
        let serving = async {
            let _guard = ServerMessageConnection::join(&authority, source).ok_or("connection join")?;
            publish(&authority.hub, 0)?;
            authority.hub.transfer_connection(source, next).map_err(|()| "handoff refused")?;
            // The committed owner changed, but its result has not reached the
            // peer and destination serve has not installed another guard.
            std::future::pending::<()>().await;
            Ok::<(), Box<dyn std::error::Error>>(())
        };
        let mut serving = std::pin::pin!(serving);
        assert!(matches!(serving.as_mut().poll(&mut context), Poll::Pending));
        assert!(authority.hub.state.lock().map_err(|_| "hub lock")?.recipients.contains_key(&next.game_session_id));
    }
    assert!(authority.hub.state.lock().map_err(|_| "hub lock")?.recipients.is_empty());
    publish(&authority.hub, 1)?;
    assert!(authority.hub.join(next.game_session_id, 2));
    assert_eq!(authority.hub.take(next.game_session_id, 2), None);
    Ok(())
}

#[test]
fn raid_notice_handoff_guard_cannot_erase_a_reconnected_destination() -> TestResult {
    let authority = WireNoticeAuthority { hub: ServerMessageHub::default(), joined: tokio::sync::Notify::new() };
    let (source, next) = (connected(1, 3)?, connected(2, 1)?);
    let guard = ServerMessageConnection::join(&authority, source).ok_or("source join")?;
    publish(&authority.hub, 0)?;
    authority.hub.transfer_connection(source, next).map_err(|()| "handoff refused")?;
    // A delayed duplicate join cannot establish another recipient with this
    // physical connection's original root after it moved to the destination.
    assert!(!authority.hub.join(source.game_session_id, 3));
    let mut resumed = next;
    resumed.continuity.connection_generation = 2;
    let successor = ServerMessageConnection::join(&authority, resumed).ok_or("resumed join")?;
    publish(&authority.hub, 1)?;
    drop(guard);
    assert_eq!(authority.hub.take(resumed.game_session_id, 2), Some(warning()));
    assert_eq!(authority.hub.take(resumed.game_session_id, 2), None);
    drop(successor);
    assert!(authority.hub.state.lock().map_err(|_| "hub lock")?.recipients.is_empty());
    Ok(())
}

async fn wire_notice_connection(has_capability: bool) -> TestResult {
    use crate::foundation::{ExactActorRef, MessageType, WorldId};
    use crate::gameplay_transport::capabilities::{PRODUCTION_OFFERED_CAPABILITIES, SelectedCapabilities};
    use crate::gameplay_transport::connection::{FirstEntryOutcome, LivenessPolicy, SessionContinuity, serve_admitted};
    use oteryn_protocol_oteryn::{decode_state_delta, decode_wire_envelope};
    use oteryn_protocol_oteryn::zones_nologout::decode_server_message;
    use std::future::Future;
    use std::time::Duration;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    let authority = WireNoticeAuthority { hub: ServerMessageHub::default(), joined: tokio::sync::Notify::new() };
    authority.hub.publish(step(0)?, ServerMessage::Warning("Previous raid".into()))
        .map_err(|()| "old publication")?;
    let world_id = WorldId::decode(&id(600))?;
    let channel_id = ChannelId::decode(&id(601))?;
    let mut admitted = AdmittedSession {
        game_session_id: session(602)?, world_id, channel_id,
        runtime_actor: Some(ExactActorRef::transport_fixture(world_id, channel_id)),
        first_entry: FirstEntryOutcome::Positioned, controller: None,
        continuity: SessionContinuity::FRESH, item_fence: None,
    };
    if has_capability {
        admitted.continuity.selected_capabilities = SelectedCapabilities::select(
            PRODUCTION_OFFERED_CAPABILITIES, &[21, CAPABILITY_ZONES_NOLOGOUT_V1],
        ).ok_or("capability selection")?;
    }
    let (mut server, mut client) = tokio::io::duplex(1 << 16);
    // No movement, command or short periodic refresh is present. A server
    // message must arrive before this one-hour liveness interval fires.
    let policy = LivenessPolicy { interval: Duration::from_secs(3_600), missed_limit: 3 };
    let serving = async {
        let end = serve_admitted(&mut server, admitted, &authority, policy).await;
        drop(server);
        end
    };
    let reading = async {
        authority.joined.notified().await;
        authority.hub.publish(step(0)?, ServerMessage::Warning("Previous raid".into()))
            .map_err(|()| "old replay")?;
        publish(&authority.hub, 1)?;
        let advance = ServerMessage::Advance("The army has arrived!".into());
        authority.hub.publish(step(2)?, advance.clone()).map_err(|()| "advance publication")?;
        let mut received = Vec::new();
        if has_capability {
            while received.len() < 2 {
                let frame = crate::gameplay_transport::tcp_tls::read_frame(&mut client).await?;
                let envelope = decode_wire_envelope(&frame)?;
                if envelope.message_type() == MessageType::StateDelta {
                    let delta = decode_state_delta(envelope.payload())?;
                    if delta.domain_id == 57 {
                        received.push(decode_server_message(delta.payload).map_err(|error| format!("{error:?}"))?);
                    }
                }
            }
            assert_eq!(received, vec![warning(), advance]);
        }
        publish(&authority.hub, 1)?;
        client.shutdown().await?;
        let mut trailing = Vec::new();
        client.read_to_end(&mut trailing).await?;
        let mut bytes = trailing.as_slice();
        while !bytes.is_empty() {
            let header: [u8; 4] = bytes.get(..4).ok_or("trailing header")?.try_into()?;
            let count = usize::try_from(u32::from_be_bytes(header))?;
            let frame = bytes.get(4..4 + count).ok_or("trailing frame")?;
            let envelope = decode_wire_envelope(frame)?;
            if envelope.message_type() == MessageType::StateDelta {
                assert_ne!(decode_state_delta(envelope.payload())?.domain_id, 57);
            }
            bytes = bytes.get(4 + count..).ok_or("trailing remainder")?;
        }
        Ok::<(), Box<dyn std::error::Error>>(())
    };
    let (mut serving, mut reading) = (std::pin::pin!(serving), std::pin::pin!(reading));
    let (mut ended, mut read) = (false, None);
    std::future::poll_fn(|context| {
        if !ended && serving.as_mut().poll(context).is_ready() { ended = true; }
        if read.is_none() && let std::task::Poll::Ready(result) = reading.as_mut().poll(context) { read = Some(result); }
        if ended && read.is_some() { std::task::Poll::Ready(()) } else { std::task::Poll::Pending }
    }).await;
    read.ok_or("connection reader did not finish")??;
    assert!(authority.hub.state.lock().map_err(|_| "hub lock")?.recipients.is_empty());
    Ok(())
}

#[test]
fn raid_production_connection_wakes_without_input_and_suppresses_unselected_or_replayed_notices() -> TestResult {
    tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
        for selected in [false, true] {
            tokio::time::timeout(std::time::Duration::from_secs(2), wire_notice_connection(selected)).await??;
        }
        Ok(())
    })
}
