use super::*;
#[test]
fn clock_snapshot_and_updates_validate_before_delivery() -> Result<(), BoxError> {
    use oteryn_protocol_oteryn::world_clock::*;
    block_on(async {
        let (client, mut server) = tokio::io::duplex(64 * 1024);
        let clock = WorldClock {
            tibian_minute: 60,
            light_level: 128,
            light_color: 215,
        };
        let peer = tokio::spawn(async move {
            join_peer_with(
                &mut server,
                &[6, 19],
                &[6, 19],
                &[],
                entity_snapshot(vec![]),
                &[(
                    18,
                    0,
                    1,
                    encode_world_clock(clock).map_err(|e| format!("{e:?}"))?,
                )],
            )
            .await?;
            Ok::<_, BoxError>(())
        });
        let mut session = Session::admit(client, admission_with(&[6, 19])?).await?;
        peer.await??;
        assert_eq!(
            decode_world_clock(
                &session
                    .gated_snapshot(18)
                    .ok_or("missing snapshot")?
                    .payload
            )
            .map_err(|e| format!("{e:?}"))?,
            clock
        );
        let value = WorldClock {
            tibian_minute: 61,
            ..clock
        };
        let frame = encode_state_delta(
            1,
            41,
            18,
            0,
            1,
            1,
            &encode_world_clock(value).map_err(|e| format!("{e:?}"))?,
        )?;
        let event = session.apply_state_delta(41, decode_wire_envelope(&frame)?.payload())?;
        assert!(matches!(event, SessionEvent::WorldClock(delta) if delta.value == value));
        assert!(
            session
                .apply_state_delta(42, decode_wire_envelope(&frame)?.payload())
                .is_err()
        );
        let bad = encode_state_delta(1, 43, 18, 1, 2, 1, &[255])?;
        assert!(
            session
                .apply_state_delta(43, decode_wire_envelope(&bad)?.payload())
                .is_err()
        );
        assert_eq!(
            session
                .gated_revisions
                .iter()
                .find(|(id, _)| *id == 18)
                .map(|(_, revision)| *revision),
            Some(1)
        );
        Ok(())
    })?
}
