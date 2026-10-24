/// An admitted session whose join carried actor vitals, paired with the peer end.
async fn vitals_pair() -> Result<(Session<DuplexStream>, DuplexStream), BoxError> {
    let (client, mut peer) = tokio::io::duplex(64 * 1024);
    let joining = tokio::spawn(async move {
        send_vitals_join(&mut peer).await?;
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
