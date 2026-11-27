struct FailNthWrite<S> {
    inner: S,
    ok_writes: usize,
}

impl<S: tokio::io::AsyncRead + Unpin> tokio::io::AsyncRead for FailNthWrite<S> {
    fn poll_read(
        self: std::pin::Pin<&mut Self>,
        context: &mut std::task::Context<'_>,
        buffer: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let this = self.get_mut();
        std::pin::Pin::new(&mut this.inner).poll_read(context, buffer)
    }
}

impl<S: AsyncWrite + Unpin> AsyncWrite for FailNthWrite<S> {
    fn poll_write(
        self: std::pin::Pin<&mut Self>,
        context: &mut std::task::Context<'_>,
        buffer: &[u8],
    ) -> std::task::Poll<std::io::Result<usize>> {
        let this = self.get_mut();
        if this.ok_writes == 0 {
            return std::task::Poll::Ready(Err(std::io::Error::other(
                "injected write failure",
            )));
        }
        this.ok_writes -= 1;
        std::pin::Pin::new(&mut this.inner).poll_write(context, buffer)
    }

    fn poll_flush(
        self: std::pin::Pin<&mut Self>,
        context: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let this = self.get_mut();
        std::pin::Pin::new(&mut this.inner).poll_flush(context)
    }

    fn poll_shutdown(
        self: std::pin::Pin<&mut Self>,
        context: &mut std::task::Context<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        let this = self.get_mut();
        std::pin::Pin::new(&mut this.inner).poll_shutdown(context)
    }
}

/// r4122508665: a write failing *after* the committed overlay entry has already been
/// encoded must not update `continuity.overlay_revision`. `write_frame` issues exactly two
/// low-level writes per frame (the length prefix, then the body); the join snapshot (3
/// frames) plus the `USE_INTENT`'s own `CommandResult` (1 frame) are let through in full (8
/// writes), and the 5th frame — the `WORLD_OBJECT_OVERLAY` `StateDelta` — is made to fail on
/// its very first write. `continuity.overlay_revision` must still hold the value from before
/// this connection (the FRESH baseline, 0), never the committed entry's revision (7) that
/// was never confirmed delivered.
#[test]
fn admitted_use_commit_write_failure_does_not_record_the_overlay_revision()
-> Result<(), Box<dyn Error>> {
    run(async {
        let committed = WorldObjectOverlayEntry {
            content_generation: [0x44; 32],
            placement: b"oteryn:cell/entry-door".to_vec(),
            state: b"oteryn:reference.state.open".to_vec(),
            revision: 7,
        };
        let authority = UseAuthority {
            uses: RefCell::new(Vec::new()),
            commands: RefCell::new(Vec::new()),
            overlay: None,
            watermark: None,
            outcome: UseOutcome {
                disposition: UseDisposition::Committed,
                committed: Some(committed),
                earned: EarnedNotice::NoneEarned,
                reward_granted: None,
                gate_message_id: None, potion_utterance: None, refusal_detail: None,
            },
        };
        let use_type = u64::from(COMMAND_TYPE_USE_INTENT);
        let world_id = WorldId::decode(&WORLD)?;
        let channel_id = ChannelId::decode(&CHANNEL)?;
        let admitted = AdmittedSession {
            game_session_id: GameSessionId::decode(&SESSION)?,
            world_id,
            channel_id,
            runtime_actor: Some(ExactActorRef::transport_fixture(world_id, channel_id)),
            first_entry: FirstEntryOutcome::Positioned,
            controller: None,
            continuity: SessionContinuity::FRESH,
            item_fence: None,
        };
        let (server, mut client): (DuplexStream, DuplexStream) = tokio::io::duplex(1 << 21);
        client
            .write_all(&framed(&use_command(
                1,
                1,
                use_type,
                b"oteryn:cell/entry-door",
                0,
            )))
            .await?;
        let mut server = FailNthWrite {
            inner: server,
            ok_writes: 8,
        };
        let end = serve_admitted(&mut server, admitted, &authority, IDLE_LIVENESS).await;
        let ConnectionEnd::AdmittedThenDisconnected(ended) = end else {
            return Err(format!("unexpected end {end:?}").into());
        };
        assert_eq!(ended.continuity.overlay_revision, 0);
        Ok(())
    })
}

/// C2: the dispatch hands `use_object` the `USE`'s own CommandId and GameSession and the
/// admitted session's item fence, and a COMMITTED `USE` without an overlay entry (a chest's
/// MINT) gets its `CommandResult` and no `WORLD_OBJECT_OVERLAY` delta.
#[test]
fn admitted_use_passes_command_identity_and_fence_and_chest_commit_emits_no_delta()
-> Result<(), Box<dyn Error>> {
    use crate::foundation::{
        ConnectionGeneration, RuntimeScopeRefV1, ScopeOwnershipGeneration,
    };
    run(async {
        let authority = UseAuthority {
            uses: RefCell::new(Vec::new()),
            commands: RefCell::new(Vec::new()),
            overlay: None,
            watermark: None,
            outcome: UseOutcome {
                disposition: UseDisposition::Committed,
                committed: None,
                earned: EarnedNotice::NoneEarned,
                reward_granted: None,
                gate_message_id: None, potion_utterance: None, refusal_detail: None,
            },
        };
        let world_id = WorldId::decode(&WORLD)?;
        let channel_id = ChannelId::decode(&CHANNEL)?;
        let game_session_id = GameSessionId::decode(&SESSION)?;
        let fence = CurrentCharacterItemFence {
            character_id: crate::domain::CharacterId::from_bytes(uuid_v7(0x55))
                .map_err(|error| format!("{error:?}"))?,
            game_session_id,
            connection_generation: ConnectionGeneration::new(1)
                .map_err(|error| format!("{error:?}"))?,
            character_lease_generation: 3,
            runtime_scope: RuntimeScopeRefV1::channel(world_id, channel_id),
            scope_ownership_generation: ScopeOwnershipGeneration::new(4)
                .map_err(|error| format!("{error:?}"))?,
        };
        let admitted = AdmittedSession {
            game_session_id,
            world_id,
            channel_id,
            runtime_actor: Some(ExactActorRef::transport_fixture(world_id, channel_id)),
            first_entry: FirstEntryOutcome::Positioned,
            controller: None,
            continuity: SessionContinuity::FRESH,
            item_fence: Some(fence),
        };
        let use_type = u64::from(COMMAND_TYPE_USE_INTENT);
        let (_end, frames) = drive_session(
            &authority,
            admitted,
            &[use_command(
                1,
                1,
                use_type,
                b"oteryn:placement/entry-chest",
                0,
            )],
        )
        .await?;
        let mut expected = baseline();
        expected.push(encode_command_result(
            1,
            1,
            1,
            CommandStatus::Accepted,
            &encode_use_result(UseDisposition::Committed).unwrap_or_default(),
        )?);
        assert_eq!(frames, expected);
        assert_eq!(
            authority.commands.borrow().as_slice(),
            &[UseCommand {
                game_session_id,
                command_id: 1,
                item_fence: Some(fence),
            }]
        );
        Ok(())
    })
}

/// A non-committing `USE_INTENT` disposition (here NOTHING_TO_USE) gets its own encoded
/// `CommandResult` and never a `WORLD_OBJECT_OVERLAY` delta, and an unregistered command
/// type still gets an empty result payload exactly as it does for STEP.
#[test]
fn admitted_use_non_committing_disposition_emits_no_delta_and_unregistered_type_is_empty()
-> Result<(), Box<dyn Error>> {
    run(async {
        let authority = UseAuthority {
            uses: RefCell::new(Vec::new()),
            commands: RefCell::new(Vec::new()),
            overlay: None,
            watermark: None,
            outcome: UseOutcome {
                disposition: UseDisposition::NothingToUse,
                committed: None,
                earned: EarnedNotice::NoneEarned,
                reward_granted: None,
                gate_message_id: None, potion_utterance: None, refusal_detail: None,
            },
        };
        let use_type = u64::from(COMMAND_TYPE_USE_INTENT);
        let (_end, frames) = drive_admitted(
            &authority,
            &[
                use_command(1, 1, use_type, b"oteryn:cell/unknown", 0),
                use_command(1, 2, 0x7fff, b"oteryn:cell/entry-door", 0),
            ],
        )
        .await?;
        let mut expected: Vec<Vec<u8>> = encode_single_chunk_snapshot(
            ADMITTED_GENERATION,
            1,
            0,
            &[
                DomainSnapshot {
                    domain_id: STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY,
                    revision: 1,
                    snapshot_type: SNAPSHOT_TYPE_WORLD_SPATIAL_V1,
                    payload: &encode_world_spatial(&at(0)),
                },
                empty_overlay_snapshot(),
            ],
        )?
        .into();
        expected.extend([
            encode_command_result(
                1,
                1,
                1,
                CommandStatus::Accepted,
                &encode_use_result(UseDisposition::NothingToUse).unwrap_or_default(),
            )?,
            encode_command_result(1, 2, 2, CommandStatus::Rejected, &[])?,
        ]);
        assert_eq!(frames, expected);
        // The unregistered type never reached `use_object`.
        assert_eq!(authority.uses.borrow().len(), 1);
        Ok(())
    })
}

#[derive(Default)]
struct LadderUseAuthority {
    moved: std::cell::Cell<bool>,
    commands: crate::gameplay_transport::session_ingress::SessionCommands,
}

impl FreshAdmissionAuthority for LadderUseAuthority {
    async fn admit(
        &self,
        _attempt: FreshAdmissionAttempt<'_>,
    ) -> Result<AdmittedSession, AdmissionRefusal> {
        Err(AdmissionRefusal::Rejected)
    }

    fn session_commands(
        &self,
    ) -> Option<&crate::gameplay_transport::session_ingress::SessionCommands> {
        Some(&self.commands)
    }

    async fn observe(&self, _: ExactActorRef) -> Option<WorldSpatialObservation> {
        let mut observation = at(10);
        observation.actor_position.floor = if self.moved.get() { 6 } else { 7 };
        Some(observation)
    }

    async fn use_object(
        &self,
        _: ExactActorRef,
        _: UseCommand,
        target: oteryn_protocol_oteryn::world_object::WorldObjectTarget,
    ) -> UseOutcome {
        assert_eq!(target.placement, b"oteryn:fixture/ladder");
        self.moved.set(true);
        UseOutcome {
            disposition: UseDisposition::Committed,
            ..UseOutcome::rejected()
        }
    }
}

fn ladder_use_frame() -> Vec<u8> {
    use_command(
        1,
        1,
        u64::from(COMMAND_TYPE_USE_INTENT),
        b"oteryn:fixture/ladder",
        0,
    )
}

#[test]
fn ladder_use_publishes_landing_before_command_result() -> Result<(), Box<dyn Error>> {
    run(async {
        let authority = LadderUseAuthority::default();
        let (end, frames) = drive_session(&authority, positioned()?, &[ladder_use_frame()]).await?;
        let mut landing = None;
        let mut saw_result = false;
        for bytes in frames {
            let frame = decode_wire_envelope(&bytes)?;
            match frame.message_type() {
                MessageType::StateDelta => {
                    let delta = oteryn_protocol_oteryn::decode_state_delta(frame.payload())?;
                    if delta.domain_id == STATE_DOMAIN_WORLD_SPATIAL_VISIBILITY {
                        let actor = oteryn_protocol_oteryn::world_spatial::decode_world_spatial(
                            delta.payload,
                        )
                        .map_err(|error| format!("{error:?}"))?;
                        assert_eq!(actor.actor_position.floor, 6);
                        landing = Some(frame.server_sequence());
                    }
                }
                MessageType::CommandResult => {
                    let result = oteryn_protocol_oteryn::decode_command_result(frame.payload())?;
                    assert_eq!(result.command_id, 1);
                    assert_eq!(result.status, CommandStatus::Accepted);
                    assert_eq!(
                        oteryn_protocol_oteryn::world_object::decode_use_result(result.payload)
                            .map_err(|error| format!("{error:?}"))?,
                        UseDisposition::Committed
                    );
                    assert_eq!(
                        landing.and_then(|sequence| sequence.checked_add(1)),
                        Some(frame.server_sequence()),
                        "the landing must already be visible when USE completes"
                    );
                    saw_result = true;
                }
                _ => {}
            }
        }
        assert!(saw_result);
        assert!(matches!(end, ConnectionEnd::AdmittedThenDisconnected(_)));
        Ok(())
    })
}

#[test]
fn failed_ladder_relocation_write_keeps_committed_use_resumable() -> Result<(), Box<dyn Error>> {
    run(async {
        let authority = LadderUseAuthority::default();
        let (server, mut client) = tokio::io::duplex(1 << 20);
        client.write_all(&framed(&ladder_use_frame())).await?;
        let mut server = FailNthWrite {
            inner: server,
            // The three join frames pass. Fail the first relocation write.
            ok_writes: 6,
        };
        let end = serve_admitted(&mut server, positioned()?, &authority, IDLE_LIVENESS).await;
        let ConnectionEnd::AdmittedThenDisconnected(ended) = end else {
            return Err(format!("unexpected end {end:?}").into());
        };
        assert!(authority.moved.get());
        assert_eq!(ended.continuity.next_command_id, 2);
        assert!(authority.commands.resumable(&ended));
        let mut resumed = crate::gameplay_transport::session_ingress::SessionCommandStream::open(
            Some(&authority.commands),
            &ended,
        )
        .ok_or("resume stream")?;
        let replay = resumed
            .begin(1, 2, ended.continuity.server_sequence, false)
            .map_err(|error| format!("{error:?}"))?;
        let crate::gameplay_transport::session_ingress::CommandAction::Replay(frame) = replay
        else {
            return Err("committed ladder USE must replay without running again".into());
        };
        let envelope = decode_wire_envelope(&frame)?;
        let result = oteryn_protocol_oteryn::decode_command_result(envelope.payload())?;
        assert_eq!(result.status, CommandStatus::DuplicateReplay);
        assert_eq!(
            oteryn_protocol_oteryn::world_object::decode_use_result(result.payload)
                .map_err(|error| format!("{error:?}"))?,
            UseDisposition::Committed,
        );
        Ok(())
    })
}
