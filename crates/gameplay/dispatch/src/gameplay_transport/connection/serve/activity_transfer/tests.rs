use super::*;
use oteryn_protocol_oteryn::activity_scope::{
    ActivityScopeAck, COMMAND_TYPE_ACTIVITY_SCOPE_ACK, encode_activity_scope_ack,
};
use oteryn_protocol_oteryn::{ClientCommandValue, encode_client_command};
use tokio::io::AsyncWriteExt;

fn id(seed: u8) -> [u8; 16] {
    let mut value = [seed; 16];
    value[6] = 0x70;
    value[8] = 0x80;
    value
}

#[test]
fn soulpit_scope_ack_discards_same_generation_source_commands_and_keeps_partial_frame()
-> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let expected = ActivityScopeAck {
                handoff: id(3),
                session: id(4),
            };
            let old = encode_client_command(
                1,
                &ClientCommandValue {
                    command_id: 1,
                    command_type: 1,
                    payload: &[],
                },
            )?;
            let payload = encode_activity_scope_ack(&expected).map_err(|e| format!("{e:?}"))?;
            let ack = encode_client_command(
                1,
                &ClientCommandValue {
                    command_id: 1,
                    command_type: COMMAND_TYPE_ACTIVITY_SCOPE_ACK,
                    payload: &payload,
                },
            )?;
            let (mut sender, mut receiver) = tokio::io::duplex(4096);
            let mut frames = FrameReader::default();
            let prefix = u32::try_from(old.len())?.to_be_bytes();
            sender.write_all(&prefix[..2]).await?;
            // A canceled source poll may already own only part of a length prefix.
            {
                let partial = frames.next(&mut receiver);
                tokio::pin!(partial);
                assert!(
                    std::future::poll_fn(|cx| {
                        std::task::Poll::Ready(
                            std::future::Future::poll(partial.as_mut(), cx).is_pending(),
                        )
                    })
                    .await
                );
            }
            sender.write_all(&prefix[2..]).await?;
            sender.write_all(&old).await?;
            write_frame(&mut sender, &ack).await?;
            let actual = read_acknowledgement(&mut receiver, &mut frames, 1, expected)
                .await
                .map_err(|_| "scope acknowledgement failed")?;
            assert_eq!(actual, ack);
            Ok(())
        })
}

#[test]
fn soulpit_scope_ack_requires_this_handoff_and_destination()
-> Result<(), Box<dyn std::error::Error>> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            let expected = ActivityScopeAck {
                handoff: id(3),
                session: id(4),
            };
            let old = ActivityScopeAck {
                handoff: id(3),
                session: id(5),
            };
            let payload = encode_activity_scope_ack(&old).map_err(|e| format!("{e:?}"))?;
            let wrong = encode_client_command(
                1,
                &ClientCommandValue {
                    command_id: 1,
                    command_type: COMMAND_TYPE_ACTIVITY_SCOPE_ACK,
                    payload: &payload,
                },
            )?;
            let (mut sender, mut receiver) = tokio::io::duplex(4096);
            write_frame(&mut sender, &wrong).await?;
            sender.shutdown().await?;
            assert!(
                read_acknowledgement(&mut receiver, &mut FrameReader::default(), 1, expected)
                    .await
                    .is_err()
            );
            Ok(())
        })
}

type RefreshTestResult<T = ()> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReturnWindow {
    Commit,
    NoSuccessor,
    Unknown,
    NoBarrier,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Publication {
    Present,
    Removed,
    Published,
    Delivered,
}

struct ReturnOwner {
    source: AdmittedSession,
    destination: AdmittedSession,
    window: ReturnWindow,
    publication: std::cell::Cell<Publication>,
    settlements: std::cell::Cell<usize>,
    finished: std::cell::Cell<usize>,
}

impl ReturnOwner {
    fn owns_source(&self, admitted: AdmittedSession) -> bool {
        admitted.game_session_id == self.source.game_session_id
            && admitted.runtime_actor == self.source.runtime_actor
            && admitted.item_fence == self.source.item_fence
            && admitted.controller == self.source.controller
            && admitted.continuity.connection_generation
                == self.source.continuity.connection_generation
    }
}

impl FreshAdmissionAuthority for ReturnOwner {
    async fn admit(
        &self,
        _: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Rejected)
    }

    async fn staff_refresh(&self, admitted: AdmittedSession) -> bool {
        if self.owns_source(admitted) {
            if self.publication.get() == Publication::Present {
                // Reproduce remove -> await -> publish after the preceding
                // pending-switch check, without publishing a successor yet.
                self.publication.set(Publication::Removed);
                tokio::task::yield_now().await;
            }
            return false;
        }
        admitted == self.destination && self.publication.get() == Publication::Delivered
    }

    async fn settle_activity_connection(
        &self,
        admitted: AdmittedSession,
    ) -> Result<AdmittedSession, ()> {
        if !self.owns_source(admitted) || self.publication.get() != Publication::Removed {
            return Err(());
        }
        self.settlements.set(self.settlements.get() + 1);
        match self.window {
            ReturnWindow::Commit => {
                self.publication.set(Publication::Published);
                Ok(self.destination)
            }
            ReturnWindow::NoSuccessor => Ok(admitted),
            ReturnWindow::Unknown => Err(()),
            ReturnWindow::NoBarrier => Ok(self.destination),
        }
    }

    async fn pending_activity_switch(
        &self,
        admitted: AdmittedSession,
    ) -> Result<Option<crate::gameplay_transport::activity_runtime::ActivityConnectionSwitch>, ()>
    {
        if !self.owns_source(admitted) || self.publication.get() != Publication::Published {
            return Ok(None);
        }
        use oteryn_protocol_oteryn::activity_scope::{
            ActivityScope, ActivityScopeSnapshot, encode_activity_scope,
        };
        let payload = encode_activity_scope(&ActivityScopeSnapshot {
            handoff: id(3),
            source_session: *admitted.game_session_id.as_bytes(),
            session: *self.destination.game_session_id.as_bytes(),
            scope: ActivityScope::Channel {
                world: *self.destination.world_id.as_bytes(),
                channel: *self.destination.channel_id.as_bytes(),
            },
            scope_generation: 1,
            connection_generation: 1,
            lease_generation: 2,
            requires_ack: true,
            selection: self
                .destination
                .continuity
                .selected_capabilities
                .as_slice()
                .to_vec(),
        })
        .map_err(|_| ())?;
        Ok(Some(
            crate::gameplay_transport::activity_runtime::ActivityConnectionSwitch {
                movement_effect_start: Some(17),
                source: admitted,
                destination: self.destination,
                payload,
            },
        ))
    }

    fn finish_activity_switch(
        &self,
        source: AdmittedSession,
        destination: AdmittedSession,
    ) -> bool {
        if !self.owns_source(source)
            || destination != self.destination
            || self.publication.get() != Publication::Published
        {
            return false;
        }
        self.finished.set(self.finished.get() + 1);
        self.publication.set(Publication::Delivered);
        true
    }
}

fn run_refresh_window(window: ReturnWindow) -> RefreshTestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> RefreshTestResult {
            tokio::runtime::Builder::new_current_thread().enable_all().build()?.block_on(async {
                let world = WorldId::decode(&id(10))?;
                let channel = ChannelId::decode(&id(11))?;
                let selected = SelectedCapabilities::select(
                    crate::gameplay_transport::capabilities::BUNDLE_WORLD_OFFERED_CAPABILITIES,
                    &[oteryn_protocol_oteryn::boss_encounter::CAPABILITY_BOSS_ENCOUNTER_V1],
                ).ok_or("activity transport capabilities")?;
                let source = AdmittedSession {
                    game_session_id: GameSessionId::decode(&id(12))?,
                    world_id: world,
                    channel_id: channel,
                    runtime_actor: Some(ExactActorRef::transport_fixture(world, channel)),
                    first_entry: FirstEntryOutcome::Positioned,
                    controller: None,
                    continuity: SessionContinuity { selected_capabilities: selected, ..SessionContinuity::FRESH },
                    item_fence: None,
                };
                let destination_channel = ChannelId::decode(&id(13))?;
                let destination = AdmittedSession {
                    game_session_id: GameSessionId::decode(&id(14))?,
                    channel_id: destination_channel,
                    runtime_actor: Some(ExactActorRef::transport_fixture(world, destination_channel)),
                    ..source
                };
                let owner = ReturnOwner {
                    source, destination, window,
                    publication: std::cell::Cell::new(Publication::Present),
                    settlements: std::cell::Cell::new(0),
                    finished: std::cell::Cell::new(0),
                };
                let (mut server, mut client) = tokio::io::duplex(1 << 20);
                let mut admitted = source;
                let mut session = setup::prepare(
                    &mut server, &mut admitted, &owner, IDLE_LIVENESS,
                    source.runtime_actor.ok_or("source actor")?,
                    WorldSpatialObservation {
                        content_generation: [7; 32],
                        actor_position: crate::gameplay_transport::world_spatial::ActorPosition { x: 300, y: 300, floor: 7 },
                    },
                    integrity::Integrity::default(),
                ).await.map_err(|_| "prepare real source stream")?;
                loop {
                    let frame = read_frame(&mut client).await?;
                    if decode_wire_envelope(&frame)?.message_type() == MessageType::SnapshotCommit { break; }
                }
                assert!(pending(&mut server, &owner, &mut session).await
                    .map_err(|_| "initial pending check")?.is_none());
                assert_eq!(owner.publication.get(), Publication::Present);
                let result = refresh_staff(&mut server, &owner, &mut session).await;
                assert_eq!(owner.settlements.get(), 1);
                if window != ReturnWindow::Commit {
                    assert!(matches!(result, Err(ServeEnd::Disconnected)), "{window:?}");
                    assert_eq!(session.admitted.game_session_id, source.game_session_id);
                    assert_eq!(owner.finished.get(), 0);
                    drop(server);
                    let mut bytes = Vec::new();
                    tokio::io::AsyncReadExt::read_to_end(&mut client, &mut bytes).await?;
                    assert!(bytes.is_empty(), "no successor or Unknown cannot emit a barrier");
                    return Ok(());
                }
                let (next, cursor) = result.map_err(|_| "retained handoff refresh")?
                    .ok_or("actual source barrier")?;
                assert_eq!(next, destination);
                assert_eq!(cursor, Some(17));
                assert_eq!(session.admitted, destination);
                assert_eq!(owner.finished.get(), 1);
                assert_eq!(session.activity_ack, Some(ActivityScopeAck { handoff: id(3), session: id(14) }));
                let mut barrier = None;
                loop {
                    let frame = read_frame(&mut client).await?;
                    let envelope = decode_wire_envelope(&frame)?;
                    if envelope.message_type() == MessageType::SnapshotChunk {
                        let (_, domains) = oteryn_protocol_oteryn::decode_snapshot_chunk(envelope.payload())
                            .map_err(|error| format!("{error:?}"))?;
                        for domain in domains {
                            if domain.domain_id == STATE_DOMAIN_ACTIVITY_SCOPE {
                                assert!(barrier.is_none(), "one exact activity barrier");
                                barrier = Some(oteryn_protocol_oteryn::activity_scope::decode_activity_scope(domain.payload)
                                    .map_err(|error| format!("{error:?}"))?);
                            }
                        }
                    }
                    if envelope.message_type() == MessageType::SnapshotCommit { break; }
                }
                let barrier = barrier.ok_or("wire activity scope")?;
                assert_eq!(barrier.source_session, *source.game_session_id.as_bytes());
                assert_eq!(barrier.session, *destination.game_session_id.as_bytes());
                assert!(barrier.requires_ack);
                assert!(!owner.staff_refresh(source).await, "the retired source never regains control");
                assert!(owner.staff_refresh(destination).await, "the next owner performs its own refresh");
                Ok(())
            })
        })?
        .join()
        .map_err(|_| "staff refresh handoff test panicked")?
}

#[test]
fn staff_refresh_settles_removed_source_and_publishes_its_activity_barrier() -> RefreshTestResult {
    run_refresh_window(ReturnWindow::Commit)
}

#[test]
fn failed_staff_refresh_without_a_committed_barrier_remains_disconnected() -> RefreshTestResult {
    for window in [
        ReturnWindow::NoSuccessor,
        ReturnWindow::Unknown,
        ReturnWindow::NoBarrier,
    ] {
        run_refresh_window(window)?;
    }
    Ok(())
}
