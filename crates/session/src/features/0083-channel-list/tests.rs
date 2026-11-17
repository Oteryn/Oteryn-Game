use super::*;
#[test]
fn gap_world_channel_list() -> Result<(), BoxError> {
    assert!(
        crate::capability_support::gated_routes()
            .any(|(id, _, domains)| id == 83 && domains == [48])
    );
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                for premium in [false, true] {
                    let (client, mut server) = tokio::io::duplex(64 * 1024);
                    let payload = oteryn_protocol_oteryn::channel_list::encode_channel_list(
                        &[1; 16], &[2; 16], premium,
                    );
                    let expected = payload.clone();
                    let peer = tokio::spawn(async move {
                        join_peer_with(
                            &mut server,
                            &[83],
                            &[83],
                            &[],
                            (1, spatial(0)),
                            &[(48, 0, 1, payload)],
                        )
                        .await?;
                        Ok::<_, BoxError>(server)
                    });
                    let mut session = Session::admit(client, admission_with(&[83])?).await?;
                    let mut server = peer.await??;
                    assert_eq!(
                        session
                            .gated_snapshot(48)
                            .ok_or("missing served channel")?
                            .payload,
                        expected
                    );
                    let value = crate::channel_list::decode(
                        &session.gated_snapshot(48).ok_or("typed channel")?.payload,
                    )?;
                    assert_eq!(
                        (value.world_id, value.channel_id, value.premium_only),
                        ([1; 16], [2; 16], premium)
                    );
                    let replacement = oteryn_protocol_oteryn::channel_list::encode_channel_list(
                        &[3; 16], &[4; 16], !premium,
                    );
                    let frames = encode_single_chunk_snapshot(
                        1,
                        2,
                        41,
                        &[DomainSnapshot {
                            domain_id: 48,
                            revision: 1,
                            snapshot_type: 1,
                            payload: &replacement,
                        }],
                    )?;
                    for frame in &frames[1..] {
                        write_frame(&mut server, frame).await?;
                    }
                    session
                        .replace_snapshot(decode_wire_envelope(&frames[0])?.payload())
                        .await?;
                    assert_eq!(
                        crate::channel_list::decode(
                            &session.gated_snapshot(48).ok_or("replacement")?.payload
                        )?
                        .world_id,
                        [3; 16]
                    );
                    assert_eq!(
                        crate::channel_list::decode(
                            &session.gated_snapshot(48).ok_or("replacement")?.payload
                        )?
                        .premium_only,
                        !premium
                    );
                }
                Ok::<(), BoxError>(())
            })
        })?
        .join()
        .map_err(|_| "channel test thread panicked")??
}

#[test]
fn gap_world_channel_list_malformed_admission() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                let valid = oteryn_protocol_oteryn::channel_list::encode_channel_list(
                    &[1; 16], &[2; 16], false,
                );
                let mut duplicate = valid.clone();
                duplicate[20] = 10;
                let mut bad_bool = valid.clone();
                bad_bool[39] = 2;
                let mut bad_length = valid.clone();
                bad_length[3] = 15;
                for (selected, rows) in [
                    (&[83][..], vec![]),
                    (&[83][..], vec![(48, 0, 2, valid.clone())]),
                    (&[][..], vec![(48, 0, 1, valid.clone())]),
                    (&[83][..], vec![(48, 0, 1, duplicate)]),
                    (&[83][..], vec![(48, 0, 1, bad_bool)]),
                    (&[83][..], vec![(48, 0, 1, bad_length)]),
                    (&[83][..], vec![(48, 0, 1, valid[..38].to_vec())]),
                ] {
                    let (client, mut server) = tokio::io::duplex(64 * 1024);
                    let selected = selected.to_vec();
                    let peer = tokio::spawn(async move {
                        join_peer_with(&mut server, &[83], &selected, &[], (1, spatial(0)), &rows)
                            .await
                    });
                    assert!(
                        Session::admit(client, admission_with(&[83])?)
                            .await
                            .is_err()
                    );
                    peer.await??;
                }
                Ok::<(), BoxError>(())
            })
        })?
        .join()
        .map_err(|_| "channel test thread panicked")??
}
