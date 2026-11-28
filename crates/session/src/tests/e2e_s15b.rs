//! Replay the unmodified result envelopes through Session's logout exchange.
use super::*;
use oteryn_protocol_oteryn::logout::LogoutResult;

#[test]
#[ignore = "requires the current node-boot S15b wire capture"]
fn e2e_s15b_in_fight_logout_refusal_keeps_the_session_playing() -> Result<(), BoxError> {
    let root = std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("S15b requires a live capture")?;
    let root = std::path::Path::new(&root);
    let manifest = std::fs::read_to_string(root.join("s15b-expected.txt"))?;
    let fields: Vec<_> = manifest.split_whitespace().collect();
    assert_eq!(fields.join(","), std::env::var("OTERYN_S15B_CAPTURE_KEY")?);
    let [
        session,
        refused_id,
        refused_sequence,
        accepted_id,
        accepted_sequence,
        retry,
    ] = fields.as_slice()
    else {
        return Err("invalid S15b capture manifest".into());
    };
    assert_eq!(session.len(), 36);
    let refused_id = refused_id.parse::<u64>()?;
    let accepted_id = accepted_id.parse::<u64>()?;
    let refused_sequence = refused_sequence.parse::<u64>()?;
    let accepted_sequence = accepted_sequence.parse::<u64>()?;
    let retry_after_ms = retry.parse::<u32>()?;
    assert!((55_000..=60_000).contains(&retry_after_ms));
    assert!(refused_id < accepted_id && refused_sequence < accepted_sequence);

    let refused = std::fs::read(root.join("s15b-refused.bin"))?;
    let accepted = std::fs::read(root.join("s15b-accepted.bin"))?;
    let generation = decode_wire_envelope(&refused)?.connection_generation();
    assert_eq!(
        decode_wire_envelope(&accepted)?.connection_generation(),
        generation
    );

    block_on(async {
        const SELECTED: &[u32] = &[logout::CAPABILITY_LOGOUT_V1];
        let (transport, mut peer) = tokio::io::duplex(64 * 1024);
        let server = tokio::spawn(async move {
            join_peer_with(&mut peer, SELECTED, SELECTED, &[], (1, spatial(0)), &[]).await?;
            for (id, raw) in [(refused_id, refused), (accepted_id, accepted)] {
                let intent = read_frame(&mut peer).await?;
                let envelope = decode_wire_envelope(&intent)?;
                let command = envelope.client_command(generation)?;
                assert_eq!(
                    (command.command_id, command.command_type),
                    (id, logout::COMMAND_TYPE_LOGOUT_INTENT)
                );
                assert!(command.payload.is_empty());
                write_frame(&mut peer, &raw).await?;
            }
            Ok::<_, BoxError>(())
        });

        let mut session = Session::admit(transport, admission_with(SELECTED)?).await?;
        session.connection_generation = generation;
        session.first_command_id = refused_id;
        session.next_command_id = refused_id;
        session.last_server_sequence = refused_sequence
            .checked_sub(1)
            .ok_or("zero refusal sequence")?;
        let refused = session.logout().await?;
        assert_eq!(
            (
                refused.command_id,
                refused.result_server_sequence,
                refused.status,
                refused.result
            ),
            (
                refused_id,
                refused_sequence,
                CommandStatus::Rejected,
                Some(LogoutResult::InFight { retry_after_ms })
            )
        );
        assert_eq!(session.next_command_id(), refused_id + 1);
        // Movement between the live requests occupies the gap in the command and
        // result streams. Match their captured cursors without resetting usability.
        session.next_command_id = accepted_id;
        session.last_server_sequence = accepted_sequence - 1;
        let accepted = session.logout().await?;
        assert_eq!(
            (
                accepted.command_id,
                accepted.result_server_sequence,
                accepted.status,
                accepted.result
            ),
            (
                accepted_id,
                accepted_sequence,
                CommandStatus::Accepted,
                Some(LogoutResult::Accepted)
            )
        );
        assert!(matches!(
            session.logout().await,
            Err(SessionError::SessionUnusable)
        ));
        assert_eq!(session.next_command_id(), accepted_id + 1);
        server.await??;
        Ok(())
    })?
}
