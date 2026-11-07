//! Exercise the shipped admission allowlist and actual gated Session command route.
use super::*;

fn exchange(capability: u32, kind: u32, reply_len: usize) -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            block_on(async move {
                for selected in [false, true] {
                    let (client, mut server) = tokio::io::duplex(64 * 1024);
                    let peer = tokio::spawn(async move {
                        let selected_caps = if selected { vec![capability] } else { vec![] };
                        join_peer_with(
                            &mut server, CLIENT_SUPPORTED_CAPABILITIES, &selected_caps,
                            &[], (1, spatial(0)), &[],
                        ).await?;
                        if selected {
                            let frame = read_frame(&mut server).await?;
                            let envelope = decode_wire_envelope(&frame)?;
                            let command = envelope.client_command(1)?;
                            assert_eq!((command.command_type, command.command_id, command.payload), (kind, 7, &[][..]));
                            write_frame(&mut server, &encode_command_result(
                                1, 41, 7, CommandStatus::Accepted, &vec![0; reply_len],
                            )?).await?;
                        }
                        Ok::<(), BoxError>(())
                    });
                    let mut admission = admission()?;
                    admission.supported_capabilities = CLIENT_SUPPORTED_CAPABILITIES;
                    let mut session = Session::admit(client, admission).await?;
                    let result = session.gated_command(capability, kind, &[]).await;
                    if selected {
                        let result = result?;
                        assert_eq!(result.status, CommandStatus::Accepted);
                        assert_eq!(result.payload, vec![0; reply_len]);
                    } else {
                        assert!(matches!(result, Err(SessionError::CapabilityNotSelected { capability: id }) if id == capability));
                    }
                    peer.await??;
                }
                Ok::<(), BoxError>(())
            })?
        })?
        .join()
        .map_err(|_| "status session thread panicked")?
}

macro_rules! status_exchange {
    ($name:ident, $cap:expr, $command:expr, $len:expr) => {
        #[test]
        fn $name() -> Result<(), BoxError> {
            exchange($cap, $command, $len)
        }
    };
}
status_exchange!(condition_status_shipped_session_routes, 276, 492, 1);
status_exchange!(rest_status_shipped_session_routes, 277, 493, 11);
status_exchange!(battle_instinct_shipped_session_routes, 278, 494, 9);
status_exchange!(skill_progress_shipped_session_routes, 279, 495, 160);
status_exchange!(nx7_skill_display_shipped_session_routes, 370, 640, 224);
status_exchange!(party_effects_shipped_session_routes, 280, 496, 41);
