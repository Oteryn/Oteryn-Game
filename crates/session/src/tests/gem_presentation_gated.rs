//! Real admitted Session gate and framed command exchange, including absent selection.
use super::*;
#[test]
fn gem_presentation_real_session_gate_reaches_wire_and_refuses_unselected() -> Result<(), BoxError>
{
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                for selected in [false, true] {
                    let (client, mut server) = tokio::io::duplex(65536);
                    let peer = tokio::spawn(async move {
                        join_peer_with(
                            &mut server,
                            &[148],
                            if selected { &[148] } else { &[] },
                            &[],
                            (1, spatial(0)),
                            &[],
                        )
                        .await?;
                        if selected {
                            let frame = read_frame(&mut server).await?;
                            let envelope = decode_wire_envelope(&frame)?;
                            let command = envelope.client_command(1)?;
                            assert_eq!(command.command_type, 6121);
                            assert_eq!(command.payload, b"owner-read");
                            write_frame(
                                &mut server,
                                &encode_command_result(
                                    1,
                                    41,
                                    7,
                                    CommandStatus::Accepted,
                                    b"owner-reply",
                                )?,
                            )
                            .await?;
                        }
                        Ok::<(), BoxError>(())
                    });
                    let mut a = admission()?;
                    a.supported_capabilities = &[148];
                    let mut session = Session::admit(client, a).await?;
                    let result = session.gated_command(148, 6121, b"owner-read").await;
                    if selected {
                        assert_eq!(result?.payload, b"owner-reply");
                    } else {
                        assert!(matches!(
                            result,
                            Err(SessionError::CapabilityNotSelected { capability: 148 })
                        ));
                    }
                    peer.await??;
                }
                Ok::<(), BoxError>(())
            })?
        })?
        .join()
        .map_err(|_| "metadata gated session thread panicked")?
}
