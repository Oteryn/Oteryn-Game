use super::*;
use crate::gameplay_transport::capabilities::PRODUCTION_OFFERED_CAPABILITIES;
use oteryn_protocol_oteryn::zones_nologout::{ServerMessage, decode_server_message};
use oteryn_protocol_oteryn::{
    decode_snapshot_body, decode_snapshot_chunk_framing, decode_state_delta,
};
use std::cell::Cell;
use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::Duration;
use tokio::io::AsyncWriteExt;

type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

fn id(n: u8) -> [u8; 16] {
    [n, 2, 3, 4, 5, 6, 0x70, 8, 0x80, 10, 11, 12, 13, 14, 15, n]
}

struct HouseNoticeAuthority {
    now: Cell<tokio::time::Instant>,
    joined: tokio::sync::Notify,
    join_count: Cell<u8>,
    changed: tokio::sync::Notify,
    messages: Mutex<VecDeque<ServerMessage>>,
    observed: Cell<Option<GameplayIdleState>>,
    left: Cell<bool>,
}

impl FreshAdmissionAuthority for HouseNoticeAuthority {
    async fn admit(
        &self,
        _attempt: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Rejected)
    }

    async fn house_scope_ack(
        &self,
        _admitted: AdmittedSession,
    ) -> Result<Option<(u64, Vec<u8>)>, ()> {
        // Only the authoritative House projection is replaced. The production
        // House serve loop, snapshot, Notify wake and domain encoder all run.
        Ok(Some((1, vec![1])))
    }

    fn gameplay_idle_now(&self) -> tokio::time::Instant {
        self.now.get()
    }

    fn observe_gameplay_idle(&self, admitted: AdmittedSession) {
        self.observed.set(Some(admitted.continuity.gameplay_idle));
    }

    fn server_messages_joined(&self, _admitted: AdmittedSession) -> bool {
        self.join_count.set(self.join_count.get() + 1);
        self.joined.notify_one();
        true
    }

    fn server_messages_left(&self, _admitted: AdmittedSession) {
        self.left.set(true);
    }

    fn take_server_notice(&self, admitted: AdmittedSession) -> Option<ServerMessage> {
        if !admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_ZONES_NOLOGOUT_V1)
        {
            return None;
        }
        self.messages.lock().ok()?.pop_front()
    }

    async fn server_messages_changed(&self, admitted: AdmittedSession) {
        if !admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_ZONES_NOLOGOUT_V1)
        {
            std::future::pending::<()>().await;
        }
        loop {
            let changed = self.changed.notified();
            if self
                .messages
                .lock()
                .is_ok_and(|messages| !messages.is_empty())
            {
                return;
            }
            changed.await;
        }
    }
}

async fn house_connection(selected: bool, fresh: bool, already_joined: bool) -> TestResult {
    let now = tokio::time::Instant::now();
    let authority = HouseNoticeAuthority {
        now: Cell::new(now),
        joined: tokio::sync::Notify::new(),
        join_count: Cell::new(0),
        changed: tokio::sync::Notify::new(),
        messages: Mutex::new(VecDeque::new()),
        observed: Cell::new(None),
        left: Cell::new(false),
    };
    let world_id = WorldId::decode(&id(1))?;
    let channel_id = ChannelId::decode(&id(2))?;
    let mut admitted = AdmittedSession {
        game_session_id: GameSessionId::decode(&id(3))?,
        world_id,
        channel_id,
        runtime_actor: Some(ExactActorRef::transport_fixture(world_id, channel_id)),
        first_entry: FirstEntryOutcome::Positioned,
        controller: None,
        item_fence: None,
        continuity: SessionContinuity::FRESH,
    };
    admitted.continuity.selected_capabilities = SelectedCapabilities::select(
        PRODUCTION_OFFERED_CAPABILITIES,
        if selected {
            &[86, CAPABILITY_ZONES_NOLOGOUT_V1]
        } else {
            &[86]
        },
    )
    .ok_or("House capability selection")?;
    admitted.continuity.server_message_revision = 4;
    if !fresh {
        admitted
            .continuity
            .gameplay_idle
            .activity(now - Duration::from_secs(600));
    }
    let expected_idle = if fresh {
        now
    } else {
        now - Duration::from_secs(600)
    };
    let (mut server, mut client) = tokio::io::duplex(1 << 16);
    let serving = async {
        let mut integrity = integrity::Integrity::default();
        let mut server_messages = if already_joined {
            ServerMessageConnection::join(&authority, admitted)
        } else {
            None
        };
        let end = serve(
            &mut server,
            admitted,
            &authority,
            &authority,
            &mut server_messages,
            LivenessPolicy {
                interval: Duration::from_secs(3_600),
                missed_limit: 3,
            },
            &mut integrity,
        )
        .await;
        drop(server_messages);
        drop(server);
        end
    };
    let reading = async {
        authority.joined.notified().await;
        let mut has_message_snapshot = false;
        loop {
            let frame = crate::gameplay_transport::tcp_tls::read_frame(&mut client).await?;
            let envelope = decode_wire_envelope(&frame)?;
            if envelope.message_type() == MessageType::SnapshotChunk {
                let (_, body) = decode_snapshot_chunk_framing(envelope.payload())?;
                for domain in decode_snapshot_body(body)? {
                    if domain.domain_id == STATE_DOMAIN_SERVER_MESSAGES {
                        has_message_snapshot = true;
                        assert_eq!(domain.revision, 4);
                        assert_eq!(domain.payload, SERVER_MESSAGES_EMPTY_SNAPSHOT);
                    }
                }
            }
            if envelope.message_type() == MessageType::SnapshotCommit {
                break;
            }
        }
        assert_eq!(has_message_snapshot, selected);
        authority.now.set(now + Duration::from_secs(60));
        let messages = [
            ServerMessage::Warning("Raid warning".into()),
            ServerMessage::Advance("Raid arrival".into()),
        ];
        authority
            .messages
            .lock()
            .map_err(|_| "notice queue")?
            .extend(messages.iter().cloned());
        authority.changed.notify_one();
        if selected {
            for (index, expected) in messages.into_iter().enumerate() {
                let frame = crate::gameplay_transport::tcp_tls::read_frame(&mut client).await?;
                let envelope = decode_wire_envelope(&frame)?;
                assert_eq!(envelope.message_type(), MessageType::StateDelta);
                let delta = decode_state_delta(envelope.payload())?;
                assert_eq!(delta.domain_id, STATE_DOMAIN_SERVER_MESSAGES);
                assert_eq!(delta.base_revision, 4 + u64::try_from(index)?);
                assert_eq!(
                    decode_server_message(delta.payload).map_err(|error| format!("{error:?}"))?,
                    expected
                );
            }
        }
        client.shutdown().await?;
        Ok::<(), Box<dyn std::error::Error>>(())
    };
    let (mut serving, mut reading) = (std::pin::pin!(serving), std::pin::pin!(reading));
    let (mut ended, mut read) = (None, None);
    std::future::poll_fn(|context| {
        if ended.is_none()
            && let std::task::Poll::Ready(result) = serving.as_mut().poll(context)
        {
            ended = Some(result);
        }
        if read.is_none()
            && let std::task::Poll::Ready(result) = reading.as_mut().poll(context)
        {
            read = Some(result);
        }
        if ended.is_some() && read.is_some() {
            std::task::Poll::Ready(())
        } else {
            std::task::Poll::Pending
        }
    })
    .await;
    read.ok_or("House reader unfinished")??;
    let Err(end) = ended.ok_or("House server unfinished")? else {
        return Err("unexpected House transfer".into());
    };
    let ConnectionEnd::AdmittedThenDisconnected(ended) = *end else {
        return Err("unexpected House connection end".into());
    };
    assert_eq!(
        ended.continuity.gameplay_idle.last_activity(),
        Some(expected_idle)
    );
    assert_eq!(
        authority
            .observed
            .get()
            .and_then(|idle| idle.last_activity()),
        Some(expected_idle)
    );
    assert_eq!(
        ended.continuity.server_message_revision,
        if selected { 6 } else { 4 }
    );
    assert_eq!(authority.join_count.get(), 1);
    assert!(authority.left.get());
    Ok(())
}

#[test]
fn raids_global_house_connection_wakes_and_keeps_idle_without_input() -> TestResult {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            for selected in [false, true] {
                for fresh in [false, true] {
                    for already_joined in [false, true] {
                        tokio::time::timeout(
                            Duration::from_secs(2),
                            house_connection(selected, fresh, already_joined),
                        )
                        .await??;
                    }
                }
            }
            Ok(())
        })
}

#[test]
fn raids_global_house_handoff_preserves_idle_and_message_continuity() -> TestResult {
    let mut previous = SessionContinuity::FRESH;
    previous
        .gameplay_idle
        .activity(tokio::time::Instant::now() - Duration::from_secs(600));
    previous.server_message_revision = 9;
    previous.server_motd_sent = true;
    previous.server_sequence = 80;
    previous.next_command_id = 20;
    previous.selected_capabilities = SelectedCapabilities::select(
        PRODUCTION_OFFERED_CAPABILITIES,
        &[86, CAPABILITY_ZONES_NOLOGOUT_V1],
    )
    .ok_or("House capability selection")?;
    let next = crate::gameplay_transport::house_transfer::handoff_continuity(
        previous,
        previous.selected_capabilities,
        1,
    );
    assert_eq!(next.gameplay_idle, previous.gameplay_idle);
    assert_eq!(next.server_message_revision, 9);
    assert!(next.server_motd_sent);
    assert_eq!(next.server_sequence, 0);
    assert_eq!(next.next_command_id, 1);
    assert_eq!(next.selected_capabilities, previous.selected_capabilities);
    Ok(())
}
