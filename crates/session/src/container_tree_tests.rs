use super::*;
use oteryn_protocol_oteryn::container_tree::*;

fn tree(container: u64, parent: Option<u64>, child: u64) -> ContainerViews {
    ContainerViews {
        views: vec![ContainerView {
            view_id: 3,
            container_handle: handle(container),
            parent_handle: parent.map(handle),
            capacity: 20,
            entries: vec![entry(child, 2854, 1)],
            browse: None,
        }],
    }
}

#[test]
fn container_tree_session_admits_commands_and_applies_live_updates() -> Result<(), BoxError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            block_on(async {
                let selected = &[4, 6, 12, 14];
                let initial = tree(10, None, 11);
                let (client, mut server) = tokio::io::duplex(256 * 1024);
                let encoded = encode_container_views(&initial).map_err(|e| format!("{e:?}"))?;
                let peer = tokio::spawn(async move {
                    join_peer_with(
                        &mut server,
                        selected,
                        selected,
                        &[],
                        item_snapshot(vec![])?,
                        &[(14, 1, 1, encoded)],
                    )
                    .await?;
                    let intents = [
                        ContainerViewIntent::Open {
                            handle: handle(11),
                            replace_view: Some(3),
                        },
                        ContainerViewIntent::Up { view_id: 3 },
                        ContainerViewIntent::Close { view_id: 3 },
                    ];
                    let values = [
                        tree(11, Some(10), 12),
                        tree(10, None, 11),
                        ContainerViews::default(),
                    ];
                    for (i, (intent, value)) in intents.iter().zip(values).enumerate() {
                        let frame = read_frame(&mut server).await?;
                        let envelope = decode_wire_envelope(&frame)?;
                        let command = envelope.client_command(1)?;
                        assert_eq!(command.command_type, 21);
                        assert_eq!(
                            decode_container_view_intent(command.payload)
                                .map_err(|e| format!("{e:?}"))?,
                            *intent
                        );
                        let sequence = 41 + 2 * i as u64;
                        let delta = encode_state_delta(
                            1,
                            sequence + u64::from(i == 1),
                            14,
                            i as u64 + 1,
                            i as u64 + 2,
                            1,
                            &encode_container_views(&value).map_err(|e| format!("{e:?}"))?,
                        )?;
                        let result = encode_command_result(
                            1,
                            sequence + u64::from(i != 1),
                            command.command_id,
                            CommandStatus::Accepted,
                            &encode_container_view_result(if i == 2 {
                                ContainerViewOutcome::Closed
                            } else {
                                ContainerViewOutcome::Opened
                            }),
                        )?;
                        for frame in if i == 1 {
                            [&result, &delta]
                        } else {
                            [&delta, &result]
                        } {
                            write_frame(&mut server, frame).await?;
                        }
                    }
                    let mut rest = Vec::new();
                    server.read_to_end(&mut rest).await?;
                    Ok::<(), BoxError>(())
                });
                let mut admission = item_admission()?;
                admission.supported_capabilities = selected;
                let mut session = Session::admit(client, admission).await?;
                assert_eq!(session.container_views(), Some(&initial));
                // Invalid deltas leave both the typed state and its revision unchanged.
                for (base, payload) in [(0, vec![]), (1, vec![255])] {
                    let frame = encode_state_delta(1, 41, 14, base, 2, 1, &payload)?;
                    let envelope = decode_wire_envelope(&frame)?;
                    assert!(session.apply_state_delta(41, envelope.payload()).is_err());
                    assert_eq!(session.container_views(), Some(&initial));
                }
                let intents = [
                    ContainerViewIntent::Open {
                        handle: handle(11),
                        replace_view: Some(3),
                    },
                    ContainerViewIntent::Up { view_id: 3 },
                    ContainerViewIntent::Close { view_id: 3 },
                ];
                for (i, intent) in intents.into_iter().enumerate() {
                    let result = session.container_view(intent).await?;
                    assert_eq!(
                        result,
                        Some(if i == 2 {
                            ContainerViewOutcome::Closed
                        } else {
                            ContainerViewOutcome::Opened
                        })
                    );
                    let deadline = tokio::time::Instant::now() + DEADLINE;
                    let event = loop {
                        if let Some(event) = session
                            .take_events()
                            .into_iter()
                            .find(|event| matches!(event, SessionEvent::ContainerViews(_)))
                        {
                            break event;
                        }
                        if tokio::time::Instant::now() >= deadline {
                            return Err("container delta timeout".into());
                        }
                        session.service_liveness(Duration::from_millis(20)).await?;
                    };
                    let SessionEvent::ContainerViews(delta) = event else {
                        return Err("wrong event".into());
                    };
                    assert_eq!(session.container_views(), Some(&delta.value));
                    assert_eq!(delta.base_revision, i as u64 + 1);
                }
                assert_eq!(session.container_views(), Some(&ContainerViews::default()));
                drop(session);
                peer.await??;
                Ok::<(), BoxError>(())
            })?
        })?
        .join()
        .map_err(|_| "container session thread failed")?
}

#[test]
fn container_tree_join_refuses_missing_malformed_and_unknown_snapshots() -> Result<(), BoxError> {
    assert!(crate::container_tree::initial(&[], &[])?.is_none());
    assert!(matches!(
        crate::container_tree::initial(&[14], &[]),
        Err(SessionError::MissingDomain(14))
    ));
    let mut snapshot = GatedSnapshot {
        domain_id: 14,
        snapshot_type: 2,
        revision: 1,
        payload: vec![],
    };
    assert!(matches!(
        crate::container_tree::initial(&[14], &[snapshot.clone()]),
        Err(SessionError::UnregisteredSnapshotType { domain_id: 14, .. })
    ));
    snapshot.snapshot_type = 1;
    snapshot.payload = vec![255];
    assert!(crate::container_tree::initial(&[14], &[snapshot]).is_err());
    Ok(())
}

#[test]
fn house_capacity_maximum_crosses_real_session_snapshot_and_delta_frames() -> Result<(), BoxError> {
    std::thread::Builder::new().stack_size(64 << 20).spawn(|| block_on(async {
        let selected: &[u32] = &[4, 6, 12, 14, 49, 446];
        let mut views = ContainerViews::default();
        for id in 0..16_u8 {
            let base = u64::from(id) * 100;
            let entries = (0..25_u64).map(|n| Ok(ItemEntry {
                handle: ItemHandle::new(u64::MAX-base-n).ok_or("maximum entry handle")?,
                item_definition_ref:std::num::NonZeroU32::MAX,
                count:std::num::NonZeroU32::MAX,sub_type:u32::MAX,
            })).collect::<Result<_,BoxError>>()?;
            views.views.push(ContainerView {
                view_id:id,container_handle:ItemHandle::new(u64::MAX-base-98).ok_or("container handle")?,
                parent_handle:Some(ItemHandle::new(u64::MAX-base-99).ok_or("parent handle")?),
                capacity:25,entries,browse:None,
            });
        }
        let payload = encode_container_views_with_house_capacity(&views,false,true).map_err(|e|format!("{e:?}"))?;
        assert_eq!(payload.len(),MAX_HOUSE_CONTAINER_VIEWS_BYTES);
        assert_eq!(payload.len(),12862);
        assert!(decode_container_views_for(&payload,false).is_err());
        let (client,mut server)=tokio::io::duplex(256*1024);
        let peer_payload=payload.clone();
        let peer=tokio::spawn(async move {
            join_peer_with(&mut server,selected,selected,&[],item_snapshot(vec![])?,&[(14,1,1,peer_payload.clone())]).await?;
            let frame=read_frame(&mut server).await?;
            let envelope=decode_wire_envelope(&frame)?;
            let command=envelope.client_command(1)?;
            assert_eq!(command.command_type,21);
            write_frame(&mut server,&encode_state_delta(1,41,14,1,2,1,&peer_payload)?).await?;
            write_frame(&mut server,&encode_command_result(1,42,command.command_id,CommandStatus::Accepted,
                &encode_container_view_result(ContainerViewOutcome::Opened))?).await?;
            Ok::<(),BoxError>(())
        });
        let mut admission=item_admission()?;
        admission.supported_capabilities=selected;
        let mut session=Session::admit(client,admission).await?;
        assert_eq!(session.container_views(),Some(&views));
        assert_eq!(session.container_view(ContainerViewIntent::Up {view_id:0}).await?,Some(ContainerViewOutcome::Opened));
        assert_eq!(session.container_views(),Some(&views));
        assert!(session.take_events().iter().any(|event| matches!(event,SessionEvent::ContainerViews(delta) if delta.value==views)));
        peer.await??;
        Ok(())
    }))?.join().map_err(|_|"House capacity session test panicked")??
}
