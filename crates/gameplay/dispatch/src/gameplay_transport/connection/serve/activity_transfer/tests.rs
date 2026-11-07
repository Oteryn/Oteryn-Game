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
