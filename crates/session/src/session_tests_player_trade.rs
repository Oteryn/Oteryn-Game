#[test]
fn player_trade_and_container_pushes_are_typed_revision_checked_and_selected()
-> Result<(), BoxError> {
    std::thread::Builder::new().stack_size(64 << 20).spawn(|| block_on(async {
        for selected in [false, true] {
            let capabilities: &[u32] = if selected { &[4, 6, 12, 14, 60] } else { &[13] };
            let (client, mut server) = tokio::io::duplex(64 * 1024);
            let initial = player_trade::encode_state(&Default::default()).map_err(|e| format!("{e:?}"))?;
            let raw = if selected { vec![(26, 1, 1, initial), (14, 1, 1, vec![])] } else { vec![] };
            let peer = tokio::spawn(async move {
                join_peer_with(&mut server, capabilities, capabilities, &[],
                    if selected { item_snapshot(vec![])? } else { (SNAPSHOT_TYPE_WORLD_SPATIAL_V1, spatial(0)) }, &raw).await?;
                read_frame(&mut server).await?;
                let state = player_trade::State { phase: 1, reason: 0, partner: Some(EntityRef { identity: [4; 16], generation: 2 }),
                    message: "A player wants to trade with you.".into(), ..Default::default() };
                let payload = player_trade::encode_state(&state).map_err(|e| format!("{e:?}"))?;
                write_frame(&mut server, &encode_state_delta(1, 41, 26, 1, 2, 1, &payload)?).await?;
                write_frame(&mut server, &encode_state_delta(1, 42, 14, 1, 2, 1, &[])?).await?;
                write_frame(&mut server, &encode_command_result(1, 43, 7, CommandStatus::Accepted,
                    &encode_step_result(StepDisposition::Blocked))?).await?;
                Ok::<(), BoxError>(())
            });
            let admitted = Session::admit(client, admission_with(capabilities)?).await;
            let mut session = match admitted {
                Ok(session) => session,
                Err(error) => { peer.await??; return Err(format!("selected={selected}: admission: {error}").into()); }
            };
            let outcome = session.step(StepDirection::North).await;
            peer.await??;
            if !selected {
                assert!(matches!(outcome, Err(SessionError::UnselectedDomain { domain_id: 26 })));
                continue;
            }
            outcome?;
            assert_eq!(session.gated_snapshot(14).ok_or("container snapshot missing")?.revision, 2);
            assert!(session.container_views().is_some());
            let stale = encode_state_delta(1, 44, 14, 1, 3, 1, &[])?;
            assert!(session
                .apply_state_delta(44, decode_wire_envelope(&stale)?.payload()).is_err());
            let events = session.take_events();
            assert!(matches!(&events[..], [
                SessionEvent::PlayerTrade(update), SessionEvent::ContainerViews(_)
            ]
                if update.base_revision == 1 && update.new_revision == 2 && update.value.phase == 1));
            assert_eq!(session.player_trade_state().ok_or("state missing")?.phase, 1);
            let payload = player_trade::encode_state(&Default::default()).map_err(|e| format!("{e:?}"))?;
            let frame = encode_state_delta(1, 44, 26, 1, 3, 1, &payload)?;
            assert!(session.apply_state_delta(44, decode_wire_envelope(&frame)?.payload()).is_err());
            let frame = encode_state_delta(1, 44, 26, 2, 3, 1, &[255])?;
            assert!(session.apply_state_delta(44, decode_wire_envelope(&frame)?.payload()).is_err());
            assert_eq!(session.gated_snapshot(26).ok_or("snapshot missing")?.revision, 2);
        }
        Ok::<(), BoxError>(())
    })?)?.join().map_err(|_| "trade thread panicked")?
}
