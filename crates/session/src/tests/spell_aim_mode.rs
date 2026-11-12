//! Capability 480: each target option reaches the wire as command 3 field 5, and a session
//! without 480 refuses the aimed send.
use super::*;
use oteryn_protocol_oteryn::actor_spell::SpellAimMode;

fn position() -> SpellTargetPosition {
    SpellTargetPosition {
        x: 11,
        y: 22,
        floor: 7,
    }
}

#[test]
fn every_aim_option_is_encoded_with_the_negotiated_aim_mode() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                let spell = NonZeroU32::new(5).ok_or("nonzero")?;
                let cases = [
                    (AimedTarget::Facing, SpellAimMode::Facing, SpellTarget::None),
                    (
                        AimedTarget::Cursor(position()),
                        SpellAimMode::Cursor,
                        SpellTarget::Position(position()),
                    ),
                    (AimedTarget::Target, SpellAimMode::Target, SpellTarget::None),
                ];
                let (client, mut server) = tokio::io::duplex(65536);
                let peer = tokio::spawn(async move {
                    join_peer_with(&mut server, &[13, 480], &[480], &[], (1, spatial(0)), &[])
                        .await?;
                    for (index, (_, mode, target)) in cases.into_iter().enumerate() {
                        let frame = read_frame(&mut server).await?;
                        let command = decode_wire_envelope(&frame)?.client_command(1)?;
                        assert_eq!(command.command_type, 3);
                        let aimed = actor_spell::decode_aimed_spell_cast_intent(command.payload)
                            .map_err(|error| format!("{error:?}"))?;
                        assert_eq!(aimed.aim_mode, Some(mode));
                        assert_eq!(aimed.intent.target, target);
                        assert!(!aimed.intent.aim_at_target);
                        let sequence = 41 + index as u64;
                        write_frame(
                            &mut server,
                            &encode_command_result(
                                1,
                                sequence,
                                7 + index as u64,
                                CommandStatus::Rejected,
                                &actor_spell::encode_spell_cast_result(
                                    SpellCastDisposition::Rejected,
                                ),
                            )?,
                        )
                        .await?;
                    }
                    Ok::<(), BoxError>(())
                });
                let mut a = admission()?;
                a.supported_capabilities = &[13, 480];
                let mut session = Session::admit(client, a).await?;
                for aim in [
                    AimedTarget::Facing,
                    AimedTarget::Cursor(position()),
                    AimedTarget::Target,
                ] {
                    let outcome = session.cast_spell_aimed(spell, aim).await?;
                    assert_eq!(outcome.disposition, SpellCastDisposition::Rejected);
                }
                peer.await??;
                Ok::<(), BoxError>(())
            })?
        })?
        .join()
        .map_err(|_| "spell aim session thread panicked")?
}

#[test]
fn without_capability_480_the_aimed_send_is_refused_and_the_old_encoding_is_unchanged()
-> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                let spell = NonZeroU32::new(5).ok_or("nonzero")?;
                let (client, mut server) = tokio::io::duplex(65536);
                let peer = tokio::spawn(async move {
                    join_peer_with(&mut server, &[13, 480], &[], &[], (1, spatial(0)), &[]).await?;
                    let frame = read_frame(&mut server).await?;
                    let command = decode_wire_envelope(&frame)?.client_command(1)?;
                    // Only the old command bytes are sent: no field 5 byte (tag 0x28).
                    let intent = actor_spell::decode_spell_cast_intent(command.payload)
                        .map_err(|error| format!("{error:?}"))?;
                    assert_eq!(intent.target, SpellTarget::None);
                    assert_eq!(
                        command.payload,
                        actor_spell::encode_spell_cast_intent(&intent)
                    );
                    write_frame(
                        &mut server,
                        &encode_command_result(
                            1,
                            41,
                            7,
                            CommandStatus::Rejected,
                            &actor_spell::encode_spell_cast_result(SpellCastDisposition::Rejected),
                        )?,
                    )
                    .await?;
                    Ok::<(), BoxError>(())
                });
                let mut a = admission()?;
                a.supported_capabilities = &[13, 480];
                let mut session = Session::admit(client, a).await?;
                let refused = session.cast_spell_aimed(spell, AimedTarget::Facing).await;
                assert!(matches!(
                    refused,
                    Err(SessionError::CapabilityNotSelected { capability: 480 })
                ));
                // The refusal sent nothing and the session stays usable on the old path.
                assert_eq!(session.next_command_id(), 7);
                session.cast_spell(spell, SpellTarget::None, false).await?;
                peer.await??;
                Ok::<(), BoxError>(())
            })?
        })?
        .join()
        .map_err(|_| "spell aim session thread panicked")?
}
