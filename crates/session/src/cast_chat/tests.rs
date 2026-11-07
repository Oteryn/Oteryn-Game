use super::*;
#[test]
fn s17c_live_caster_line_decodes_with_revision_and_direction_checks() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                let (client, mut server) = tokio::io::duplex(65536);
                let empty = cast::Message::CasterChatEmpty
                    .encode()
                    .map_err(|e| format!("{e:?}"))?;
                let peer = tokio::spawn(async move {
                    join_peer_with(
                        &mut server,
                        &[290, 291],
                        &[290, 291],
                        &[],
                        (1, spatial(0)),
                        &[(103, 1, 1, empty)],
                    )
                    .await
                });
                let mut session = Session::admit(client, admission_with(&[290, 291])?).await?;
                peer.await??;
                let bytes = cast::Message::CasterLine {
                    handle: [3; 16],
                    text: "Viewer-1: hello".into(),
                }
                .encode()
                .map_err(|e| format!("{e:?}"))?;
                let frame = encode_state_delta(1, 41, 103, 1, 2, 1, &bytes)?;
                let event =
                    session.apply_state_delta(41, decode_wire_envelope(&frame)?.payload())?;
                let SessionEvent::CastCasterLine(line) = event else {
                    return Err("caster line event".into());
                };
                assert_eq!(line.value.handle, [3; 16]);
                assert_eq!(line.value.text, "Viewer-1: hello");
                assert!(
                    session
                        .apply_state_delta(42, decode_wire_envelope(&frame)?.payload())
                        .is_err(),
                    "stale base cannot duplicate a line"
                );
    let roster=cast::Message::CasterRoster{revision:7,subjects:vec![[3;16],[4;16]]}.encode().map_err(|e|format!("{e:?}"))?;
                    let roster_frame=encode_state_delta(1,42,103,2,3,cast::DELTA_TYPE_CAST_ROSTER,&roster)?;
                    let roster_event=session.apply_state_delta(42,decode_wire_envelope(&roster_frame)?.payload())?;
                    assert!(matches!(roster_event,SessionEvent::CastRoster(row) if row.value.revision==7&&row.value.subjects==vec![[3;16],[4;16]]));
                                let wrong = cast::Message::Chat("wrong direction".into())
                    .encode()
                    .map_err(|e| format!("{e:?}"))?;
                let frame = encode_state_delta(1, 43, 103, 3, 4, 1, &wrong)?;
                assert!(
                    session
                        .apply_state_delta(42, decode_wire_envelope(&frame)?.payload())
                        .is_err()
                );
                Ok(())
            })
        })?
        .join()
        .map_err(|_| "cast chat test thread failed")??
}
