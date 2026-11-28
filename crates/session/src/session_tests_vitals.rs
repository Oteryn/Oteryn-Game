/// An admitted session whose join carried actor vitals, paired with the peer end.
pub(super) async fn vitals_pair() -> Result<(Session<DuplexStream>, DuplexStream), BoxError> {
    vitals_pair_with_content(CONTENT_GENERATION).await
}

async fn vitals_pair_with_content(
    content_generation: [u8; 32],
) -> Result<(Session<DuplexStream>, DuplexStream), BoxError> {
    let (client, mut peer) = tokio::io::duplex(64 * 1024);
    let joining = tokio::spawn(async move {
        send_vitals_join_with_content(&mut peer, content_generation).await?;
        Ok::<_, BoxError>(peer)
    });
    let session = Session::admit(client, admission()?).await?;
    Ok((session, joining.await??))
}

fn vitals_frame(
    generation: u64,
    sequence: u64,
    domain: u32,
    base: u64,
    new: u64,
    delta_type: u32,
    payload: &[u8],
) -> Result<Vec<u8>, BoxError> {
    Ok(encode_state_delta(
        generation, sequence, domain, base, new, delta_type, payload,
    )?)
}

pub(super) async fn send_vitals_delta(
    peer: &mut DuplexStream,
    sequence: u64,
    base: u64,
    new: u64,
    payload: &[u8],
) -> Result<(), BoxError> {
    write_frame(peer, &vitals_frame(1, sequence, 3, base, new, 1, payload)?).await?;
    Ok(())
}

#[test]
fn idle_applies_sequenced_actor_vitals_before_liveness_ack() -> Result<(), BoxError> {
    block_on(async {
        let (mut session, mut peer) = vitals_pair().await?;
        let payload = actor_spell::encode_actor_vitals(&vitals(140, 55))
            .map_err(|error| format!("vitals: {error:?}"))?;
        write_frame(&mut peer, &vitals_frame(1, 41, 3, 3, 4, 1, &payload)?).await?;
        write_frame(
            &mut peer,
            &oteryn_protocol_oteryn::encode_liveness_probe(1, 9)?,
        )
        .await?;
        session.service_liveness(Duration::from_millis(10)).await?;
        assert_eq!(session.actor_vitals(), Some(&vitals(140, 55)));
        assert_eq!(session.last_server_sequence(), 41);
        assert_eq!(session.vitals_revision, 4);
        let ack = read_frame(&mut peer).await?;
        let ack = decode_wire_envelope(&ack)?.liveness_ack(1)?;
        assert_eq!(ack.last_applied_server_sequence, Some(41));
        Ok::<_, BoxError>(())
    })?
}

#[test]
fn asynchronous_vitals_before_command_result_preserves_mandated_cast_delta() -> Result<(), BoxError>
{
    block_on(async {
        let (mut session, mut peer) = vitals_pair().await?;
        let responder = tokio::spawn(async move {
            let command_frame = read_frame(&mut peer).await?;
            let command = decode_wire_envelope(&command_frame)?.client_command(1)?;
            let payload = actor_spell::encode_actor_vitals(&vitals(140, 55))
                .map_err(|error| format!("vitals: {error:?}"))?;
            write_frame(&mut peer, &vitals_frame(1, 41, 3, 3, 4, 1, &payload)?).await?;
            write_frame(
                &mut peer,
                &encode_command_result(
                    1,
                    42,
                    command.command_id,
                    CommandStatus::Accepted,
                    &actor_spell::encode_spell_cast_result(SpellCastDisposition::Cast),
                )?,
            )
            .await?;
            let paid = actor_spell::encode_actor_vitals(&vitals(145, 30))
                .map_err(|error| format!("vitals: {error:?}"))?;
            write_frame(&mut peer, &vitals_frame(1, 43, 3, 4, 5, 1, &paid)?).await?;
            Ok::<_, BoxError>(())
        });
        let cast = session
            .cast_spell(
                NonZeroU32::new(1).ok_or("spell index")?,
                SpellTarget::None,
                false,
            )
            .await?;
        let applied = cast.actor_vitals_delta.ok_or("mandatory cast delta")?;
        assert_eq!(
            (
                applied.server_sequence,
                applied.base_revision,
                applied.new_revision
            ),
            (43, 4, 5)
        );
        assert_eq!(session.actor_vitals(), Some(&vitals(145, 30)));
        assert_eq!(session.last_server_sequence(), 43);
        responder.await??;
        Ok::<_, BoxError>(())
    })?
}

#[test]
fn invalid_unsolicited_vitals_poison_idle_and_command_without_advancing_state()
-> Result<(), BoxError> {
    block_on(async {
        let payload = actor_spell::encode_actor_vitals(&vitals(140, 55))
            .map_err(|error| format!("vitals: {error:?}"))?;
        let mut nonadvancing = vitals_frame(1, 41, 3, 3, 4, 1, &payload)?;
        let revision = nonadvancing
            .windows(4)
            .position(|bytes| bytes == [24, 4, 32, 1])
            .ok_or("revision field in existing encoded envelope")?;
        nonadvancing[revision + 1] = 3;
        // A frame that fails the sequence check records nothing; one with a valid sequence
        // records its sequence before the domain store refuses it. Either poisons the session
        // and leaves the vitals unchanged.
        let cases = [
            ("generation", 40, vitals_frame(2, 41, 3, 3, 4, 1, &payload)?),
            (
                "stale sequence",
                40,
                vitals_frame(1, 40, 3, 3, 4, 1, &payload)?,
            ),
            (
                "sequence gap",
                40,
                vitals_frame(1, 42, 3, 3, 4, 1, &payload)?,
            ),
            ("domain", 41, vitals_frame(1, 41, 1, 3, 4, 1, &payload)?),
            ("delta type", 41, vitals_frame(1, 41, 3, 3, 4, 2, &payload)?),
            ("stale base", 41, vitals_frame(1, 41, 3, 2, 4, 1, &payload)?),
            ("nonadvancing revision", 41, nonadvancing),
            (
                "malformed vitals",
                41,
                vitals_frame(1, 41, 3, 3, 4, 1, &[8, 255])?,
            ),
        ];
        for command_wait in [false, true] {
            for (name, sequence, frame) in &cases {
                let (mut session, mut peer) = vitals_pair().await?;
                write_frame(&mut peer, frame).await?;
                let failed = if command_wait {
                    session
                        .cast_spell(NonZeroU32::new(1).ok_or("index")?, SpellTarget::None, false)
                        .await
                        .map(|_| ())
                } else {
                    session.service_liveness(Duration::from_millis(10)).await
                };
                assert!(
                    failed.is_err(),
                    "invalid {name}; command_wait={command_wait}"
                );
                assert_eq!(session.last_server_sequence(), *sequence, "{name}");
                assert_eq!(session.vitals_revision, 3, "{name}");
                assert_eq!(session.actor_vitals(), Some(&vitals(150, 55)), "{name}");
                assert!(matches!(
                    session.service_liveness(Duration::ZERO).await,
                    Err(SessionError::SessionUnusable)
                ));
                let next = session.next_command_id();
                assert!(matches!(
                    session.step(StepDirection::East).await,
                    Err(SessionError::SessionUnusable)
                ));
                assert_eq!(session.next_command_id(), next);
            }
        }
        Ok::<_, BoxError>(())
    })?
}

#[test]
fn expired_idle_window_does_not_start_queued_delta_but_finishes_started_frame()
-> Result<(), BoxError> {
    block_on(async {
        let (mut session, mut peer) = vitals_pair().await?;
        let payload = actor_spell::encode_actor_vitals(&vitals(140, 55))
            .map_err(|error| format!("vitals: {error:?}"))?;
        write_frame(&mut peer, &vitals_frame(1, 41, 3, 3, 4, 1, &payload)?).await?;
        session.service_liveness(Duration::ZERO).await?;
        assert_eq!(session.last_server_sequence(), 40);
        session.service_liveness(Duration::from_millis(10)).await?;
        assert_eq!(session.last_server_sequence(), 41);
        let body = vitals_frame(1, 42, 3, 4, 5, 1, &payload)?;
        let mut frame = u32::try_from(body.len())?.to_be_bytes().to_vec();
        frame.extend_from_slice(&body);
        peer.write_all(&frame[..1]).await?;
        let finishing = tokio::spawn(async move {
            tokio::time::sleep(Duration::from_millis(30)).await;
            peer.write_all(&frame[1..]).await?;
            Ok::<_, BoxError>(peer)
        });
        session.service_liveness(Duration::from_millis(10)).await?;
        assert_eq!(session.last_server_sequence(), 42);
        assert_eq!(session.vitals_revision, 5);
        let _peer = finishing.await??;
        Ok::<_, BoxError>(())
    })?
}

async fn gift_pair(capabilities: &'static [u32]) -> Result<(Session<DuplexStream>, DuplexStream), BoxError> {
    let (client, mut peer) = tokio::io::duplex(64 * 1024);
    let joining = tokio::spawn(async move {
        let low = actor_spell::encode_actor_vitals(&vitals(1, 5)).map_err(|e| format!("{e:?}"))?;
        let raw = [(3, 3, 1, low), (19, 0, 1, vec![]), (20, 0, 1, vec![])];
        join_peer_with(
            &mut peer,
            capabilities,
            capabilities,
            &[],
            entity_snapshot(vec![]),
            &raw,
        )
        .await?;
        Ok::<_, BoxError>(peer)
    });
    let session = Session::admit(client, admission_with(capabilities)?).await?;
    Ok((session, joining.await??))
}

fn gift_cooldown_frame(sequence: u64) -> Result<Vec<u8>, BoxError> {
    use oteryn_protocol_oteryn::presentation as wire;
    let payload = wire::encode_actor_cooldowns(&wire::ActorCooldowns {
        server_now_ms: 0,
        rtt_ms: None,
        entries: vec![wire::ActorCooldown {
            kind: wire::ActorCooldownKind::Spell,
            id: 1,
            expires_at_ms: 30_000,
        }],
    })
    .map_err(|e| format!("{e:?}"))?;
    vitals_frame(
        1,
        sequence,
        wire::STATE_DOMAIN_ACTOR_COOLDOWNS,
        0,
        1,
        wire::DELTA_TYPE_ACTOR_COOLDOWNS_WITH_VITALS_V1,
        &payload,
    )
}

#[test]
fn gift_cooldown_and_vitals_cross_an_idle_deadline_together() -> Result<(), BoxError> {
    block_on(async {
        let (mut session, mut peer) = gift_pair(&[6, 23]).await?;
        write_frame(&mut peer, &gift_cooldown_frame(41)?).await?;
        let rescued =
            actor_spell::encode_actor_vitals(&vitals(30, 20)).map_err(|e| format!("{e:?}"))?;
        {
            let mut serving = std::pin::pin!(session.service_liveness(Duration::from_millis(20)));
            assert!(
                tokio::time::timeout(Duration::from_millis(100), &mut serving)
                    .await
                    .is_err(),
                "the idle turn returned with the cooldown while the peer still held the vitals frame",
            );
            send_vitals_delta(&mut peer, 42, 3, 4, &rescued).await?;
            serving.await?;
        }
        assert_eq!(session.actor_vitals(), Some(&vitals(30, 20)));
        assert!(matches!(
            session.take_events().as_slice(),
            [SessionEvent::GiftOfLife { cooldowns, .. }] if cooldowns.value.snapshot
        ));
        Ok(())
    })?
}

#[test]
fn cast_claims_the_vitals_in_its_queued_rescue_pair() -> Result<(), BoxError> {
    use oteryn_protocol_oteryn::presentation::CAPABILITY_ACTOR_COOLDOWN_DELTAS_V1;
    block_on(async {
        let (mut session, mut peer) =
            gift_pair(&[6, 23, CAPABILITY_ACTOR_COOLDOWN_DELTAS_V1]).await?;
        let responder = tokio::spawn(async move {
            let command = read_frame(&mut peer).await?;
            let id = decode_wire_envelope(&command)?
                .client_command(1)?
                .command_id;
            write_frame(
                &mut peer,
                &encode_command_result(
                    1,
                    41,
                    id,
                    CommandStatus::Accepted,
                    &actor_spell::encode_spell_cast_result(SpellCastDisposition::Cast),
                )?,
            )
            .await?;
            write_frame(&mut peer, &gift_cooldown_frame(42)?).await?;
            let rescued =
                actor_spell::encode_actor_vitals(&vitals(30, 20)).map_err(|e| format!("{e:?}"))?;
            send_vitals_delta(&mut peer, 43, 3, 4, &rescued).await?;
            Ok::<_, BoxError>(())
        });
        let cast = session
            .cast_spell(NonZeroU32::MIN, SpellTarget::None, false)
            .await?;
        let claimed = cast
            .actor_vitals_delta
            .ok_or("cast must claim the rescued vitals")?;
        assert_eq!(claimed.server_sequence, 43);
        assert_eq!(claimed.value, vitals(30, 20));
        let events = session.take_events();
        let [
            SessionEvent::GiftOfLife {
                cooldowns,
                vitals: delivered,
            },
        ] = events.as_slice()
        else {
            return Err(format!("expected one queued rescue pair, got {events:?}").into());
        };
        assert_eq!(cooldowns.server_sequence, 42);
        assert!(!cooldowns.value.snapshot);
        assert_eq!(delivered, &claimed);
        assert_eq!(session.actor_vitals(), Some(&claimed.value));
        responder.await??;
        Ok(())
    })?
}

#[test]
fn malformed_rescue_vitals_leave_both_domain_revisions_unchanged() -> Result<(), BoxError> {
    block_on(async {
        let (mut session, mut peer) = gift_pair(&[6, 23]).await?;
        write_frame(&mut peer, &gift_cooldown_frame(41)?).await?;
        send_vitals_delta(&mut peer, 42, 2, 4, &[]).await?;
        assert!(
            session
                .service_liveness(Duration::from_millis(20))
                .await
                .is_err()
        );
        assert!(session.take_events().is_empty());
        assert_eq!(session.actor_vitals(), Some(&vitals(1, 5)));
        assert_eq!(session.vitals_revision, 3);
        assert_eq!(
            session.gated_revisions.iter().find(|(id, _)| *id == 20),
            Some(&(20, 0)),
        );
        Ok(())
    })?
}
