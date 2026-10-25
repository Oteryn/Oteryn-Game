/// USE-WIRE-V1 (#162 5868482467): the join snapshot carries `WORLD_OBJECT_OVERLAY` (domain
/// 2, snapshot type 1) alongside `WORLD_SPATIAL_VISIBILITY`, and a COMMITTED `USE_INTENT`
/// gets a `CommandResult` followed by a sequenced `WORLD_OBJECT_OVERLAY` delta (domain 2,
/// delta type 1) whose `from`/`to` revision matches the committed entry's own revision.
#[test]
fn admitted_use_commits_and_the_join_snapshot_carries_the_door_overlay()
-> Result<(), Box<dyn Error>> {
    run(async {
        let overlay = WorldObjectOverlayEntry {
            content_generation: [0x11; 32],
            placement: b"oteryn:cell/entry-door".to_vec(),
            state: b"oteryn:reference.state.closed".to_vec(),
            revision: 0,
        };
        let committed = WorldObjectOverlayEntry {
            content_generation: [0x11; 32],
            placement: b"oteryn:cell/entry-door".to_vec(),
            state: b"oteryn:reference.state.open".to_vec(),
            revision: 1,
        };
        let authority = UseAuthority {
            uses: RefCell::new(Vec::new()),
            commands: RefCell::new(Vec::new()),
            overlay: Some(overlay.clone()),
            watermark: None,
            outcome: UseOutcome {
                disposition: UseDisposition::Committed,
                committed: Some(committed.clone()),
                earned: EarnedNotice::NoneEarned,
                gate_message_id: None, refusal_detail: None,
            },
        };
        let use_type = u64::from(COMMAND_TYPE_USE_INTENT);
        let (end, frames) = drive_admitted(
            &authority,
            &[use_command(1, 1, use_type, b"oteryn:cell/entry-door", 0)],
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
                DomainSnapshot {
                    domain_id: STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                    revision: 0,
                    snapshot_type: SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1,
                    payload: &encode_world_object_overlay_snapshot(&[overlay])
                        .expect("encode overlay snapshot"),
                },
            ],
        )?
        .into();
        expected.extend([
            encode_command_result(
                1,
                1,
                1,
                CommandStatus::Accepted,
                &encode_use_result(UseDisposition::Committed).unwrap_or_default(),
            )?,
            encode_state_delta(
                1,
                2,
                STATE_DOMAIN_WORLD_OBJECT_OVERLAY,
                0,
                1,
                DELTA_TYPE_WORLD_OBJECT_OVERLAY_V1,
                &encode_world_object_overlay_delta(&committed).expect("encode delta"),
            )?,
        ]);
        assert_eq!(frames, expected);
        assert_eq!(authority.uses.borrow().len(), 1);
        assert_eq!(
            authority.uses.borrow()[0],
            oteryn_protocol_oteryn::world_object::WorldObjectTarget {
                placement: b"oteryn:cell/entry-door".to_vec(),
                expected_revision: 0,
            }
        );
        // r4122215795: the ended session's continuity records the last
        // WORLD_OBJECT_OVERLAY revision emitted (here, the committed delta's own
        // revision 1) — the same field a reconnect's FND-02 fence must carry next to
        // `spatial_revision` (`resume.rs`, out of this seam's owned paths).
        let ConnectionEnd::AdmittedThenDisconnected(ended) = end else {
            return Err(format!("unexpected end {end:?}").into());
        };
        assert_eq!(ended.continuity.overlay_revision, 1);
        Ok(())
    })
}

/// r4122215795: the join snapshot alone (before any `USE_INTENT`) already sets the ended
/// session's continuity to the current overlay revision it was actually sent, not left at
/// its prior/default value.
#[test]
fn admitted_join_snapshot_alone_records_the_overlay_revision_it_sent()
-> Result<(), Box<dyn Error>> {
    run(async {
        let overlay = WorldObjectOverlayEntry {
            content_generation: [0x22; 32],
            placement: b"oteryn:cell/entry-door".to_vec(),
            state: b"oteryn:reference.state.closed".to_vec(),
            revision: 5,
        };
        let authority = UseAuthority {
            uses: RefCell::new(Vec::new()),
            commands: RefCell::new(Vec::new()),
            overlay: Some(overlay),
            watermark: None,
            outcome: UseOutcome::rejected(),
        };
        let (end, _frames) = drive_admitted(&authority, &[]).await?;
        let ConnectionEnd::AdmittedThenDisconnected(ended) = end else {
            return Err(format!("unexpected end {end:?}").into());
        };
        assert_eq!(ended.continuity.overlay_revision, 5);
        Ok(())
    })
}

/// Wraps a stream so its `poll_write` deterministically fails once `ok_writes` successful
/// calls have gone through the inner stream, then fails every call after. Reads are
/// unaffected. Used to prove a specific frame's transmission fails, without racing a real
/// duplex's buffering/timing.
#[test]
fn admitted_empty_overlay_is_present_and_records_its_delivered_revision()
-> Result<(), Box<dyn Error>> {
    run(async {
        let authority = UseAuthority {
            uses: RefCell::new(Vec::new()),
            commands: RefCell::new(Vec::new()),
            overlay: None,
            watermark: None,
            outcome: UseOutcome::rejected(),
        };
        let world_id = WorldId::decode(&WORLD)?;
        let channel_id = ChannelId::decode(&CHANNEL)?;
        let mut admitted = AdmittedSession {
            game_session_id: GameSessionId::decode(&SESSION)?,
            world_id,
            channel_id,
            runtime_actor: Some(ExactActorRef::transport_fixture(world_id, channel_id)),
            first_entry: FirstEntryOutcome::Positioned,
            controller: None,
            continuity: SessionContinuity::FRESH,
            item_fence: None,
        };
        let (end, frames) = drive_session(&authority, admitted, &[]).await?;
        let chunk = oteryn_protocol_oteryn::decode_wire_envelope(
            frames.get(1).ok_or("missing snapshot chunk")?,
        )?;
        let (_, domains) = oteryn_protocol_oteryn::decode_snapshot_chunk(chunk.payload())?;
        let overlay = domains
            .iter()
            .find(|domain| domain.domain_id == STATE_DOMAIN_WORLD_OBJECT_OVERLAY)
            .ok_or("empty overlay domain was omitted")?;
        assert_eq!(overlay.revision, 0);
        assert_eq!(overlay.snapshot_type, SNAPSHOT_TYPE_WORLD_OBJECT_OVERLAY_V1);
        assert!(
            oteryn_protocol_oteryn::world_object::decode_world_object_overlay_snapshot(
                overlay.payload
            )
            .map_err(|error| format!("empty overlay decode: {error:?}"))?
            .is_empty()
        );
        let ConnectionEnd::AdmittedThenDisconnected(ended) = end else {
            return Err(format!("unexpected end {end:?}").into());
        };
        assert_eq!(ended.continuity.overlay_revision, 0);
        admitted.continuity.overlay_revision = 7;
        let (end, frames) = drive_session(&authority, admitted, &[]).await?;
        assert!(
            frames.is_empty(),
            "an unavailable previous overlay must not publish empty"
        );
        let ConnectionEnd::AdmittedThenDisconnected(ended) = end else {
            return Err(format!("unexpected end {end:?}").into());
        };
        assert_eq!(ended.continuity.overlay_revision, 7);
        Ok(())
    })
}
