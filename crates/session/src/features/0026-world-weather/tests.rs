use super::*;
#[test]
fn weather_snapshot_and_updates_validate_before_delivery() -> Result<(), BoxError> {
    use oteryn_protocol_oteryn::world_weather::*;
    weather_block_on(async {
        let (client, mut server) = tokio::io::duplex(64 * 1024);
        let weather = WorldWeather::Active {
            binding_token: [1; 32],
            absolute_slot: 0,
            kind: 3,
            intensity: 2,
        };
        let peer = tokio::spawn(async move {
            join_peer_with(
                &mut server,
                &[6, 26],
                &[6, 26],
                &[],
                entity_snapshot(vec![]),
                &[(
                    22,
                    0,
                    1,
                    encode_world_weather(weather).map_err(|e| format!("{e:?}"))?,
                )],
            )
            .await?;
            Ok::<_, BoxError>(())
        });
        let mut session = Session::admit(client, admission_with(&[6, 26])?).await?;
        peer.await??;
        assert_eq!(
            decode_world_weather(
                &session
                    .gated_snapshot(22)
                    .ok_or("missing snapshot")?
                    .payload
            )
            .map_err(|e| format!("{e:?}"))?,
            weather
        );
        let value = WorldWeather::Unavailable;
        let frame = encode_state_delta(
            1,
            41,
            22,
            0,
            1,
            1,
            &encode_world_weather(value).map_err(|e| format!("{e:?}"))?,
        )?;
        let event = session.apply_state_delta(41, decode_wire_envelope(&frame)?.payload())?;
        assert!(matches!(event, SessionEvent::WorldWeather(delta) if delta.value == value));
        assert!(
            session
                .apply_state_delta(42, decode_wire_envelope(&frame)?.payload())
                .is_err()
        );
        let bad = encode_state_delta(1, 43, 22, 1, 2, 1, &[255])?;
        assert!(
            session
                .apply_state_delta(43, decode_wire_envelope(&bad)?.payload())
                .is_err()
        );
        assert_eq!(
            session
                .gated_revisions
                .iter()
                .find(|(id, _)| *id == 22)
                .map(|(_, revision)| *revision),
            Some(1)
        );
        Ok(())
    })?
}

#[test]
fn weather_requires_valid_snapshot_and_negotiated_capability() -> Result<(), BoxError> {
    assert!(CLIENT_SUPPORTED_CAPABILITIES.contains(&26));
    assert!(
        crate::capability_support::gated_routes()
            .any(|(id, _, domains)| id == 26 && domains.contains(&22))
    );
    assert!(crate::weather::validate_join(&[6, 26], &[]).is_err());
    assert!(crate::weather::validate_join(&[6], &[]).is_ok());
    for (snapshot_type, payload) in [(2, vec![8, 1]), (1, vec![255])] {
        assert!(
            crate::weather::validate_join(
                &[6, 26],
                &[GatedSnapshot {
                    domain_id: 22,
                    revision: 0,
                    snapshot_type,
                    payload
                }]
            )
            .is_err()
        );
    }
    weather_block_on(async {
        let (client, mut server) = tokio::io::duplex(64 * 1024);
        let peer = tokio::spawn(async move {
            join_peer_with(&mut server, &[6], &[6], &[], entity_snapshot(vec![]), &[]).await
        });
        let mut session = Session::admit(client, admission_with(&[6])?).await?;
        peer.await??;
        let frame = encode_state_delta(1, 41, 22, 0, 1, 1, &[8, 1])?;
        assert!(matches!(
            session.apply_state_delta(41, decode_wire_envelope(&frame)?.payload()),
            Err(SessionError::UnselectedDomain { domain_id: 22 })
        ));
        Ok(())
    })?
}

fn weather_block_on<F>(future: F) -> Result<F::Output, BoxError>
where
    F: Future + Send + 'static,
    F::Output: Send + 'static,
{
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || block_on(future))?
        .join()
        .map_err(|_| "weather session test thread failed")?
}
