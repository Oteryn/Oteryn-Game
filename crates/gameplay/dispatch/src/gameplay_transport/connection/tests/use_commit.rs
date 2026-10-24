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
                gate_message_id: None,
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
                gate_message_id: None,
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
                gate_message_id: None,
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
