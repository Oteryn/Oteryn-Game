use super::*;
use oteryn_protocol_oteryn::container_tree::*;

#[test]
fn empty_depot_source_allocates_before_replacing_view_zero() -> Result<(), BoxError> {
    open_exchange(false)
}
#[test]
fn missing_depot_source_delta_expires_at_session_deadline_without_ending_play()
-> Result<(), BoxError> {
    open_exchange(true)
}
fn open_exchange(omit_delta: bool) -> Result<(), BoxError> {
    block_on(async move {
        const SELECTED: &[u32] = &[4, 6, 12, 14];
        let (client, mut server) = tokio::io::duplex(256 * 1024);
        let peer = tokio::spawn(async move {
            let inventory = item_view::encode_character_inventory(&CharacterInventory {
                main_backpack: Some(entry(1, 4, 1)),
                ..Default::default()
            })
            .map_err(|e| format!("{e:?}"))?;
            let corpse = item_view::encode_open_container(&OpenContainer::default())
                .map_err(|e| format!("{e:?}"))?;
            join_peer_with(
                &mut server,
                SELECTED,
                SELECTED,
                &[],
                item_snapshot(vec![])?,
                &[(9, 1, 1, inventory), (11, 1, 1, corpse), (14, 1, 1, vec![])],
            )
            .await?;
            let mut opened = false;
            let mut sequence = 41;
            for id in 7..=8 {
                let frame = read_frame(&mut server).await?;
                let envelope = decode_wire_envelope(&frame)?;
                let command = envelope.client_command(1)?;
                assert_eq!(command.command_type, 21);
                let request =
                    decode_container_view_intent(command.payload).map_err(|e| format!("{e:?}"))?;
                // Match container_view.rs::plan: Some(id) is Stale unless that view exists;
                // None allocates the first free slot. Domain 14 is EMPTY at admission.
                let outcome = match request {
                    ContainerViewIntent::Open {
                        handle: source,
                        replace_view,
                    } if source == handle(1) => {
                        assert_eq!(replace_view, if opened { Some(0) } else { None });
                        if replace_view.is_some() && !opened {
                            ContainerViewOutcome::Stale
                        } else {
                            ContainerViewOutcome::Opened
                        }
                    }
                    _ => ContainerViewOutcome::Stale,
                };
                let payload = encode_container_view_result(outcome);
                write_frame(
                    &mut server,
                    &encode_command_result(1, sequence, id, CommandStatus::Accepted, &payload)?,
                )
                .await?;
                sequence += 1;
                if !omit_delta {
                    let views = ContainerViews {
                        views: vec![ContainerView {
                            view_id: 0,
                            container_handle: handle(1),
                            parent_handle: None,
                            capacity: 20,
                            entries: vec![],
                            browse: None,
                        }],
                    };
                    let payload = encode_container_views(&views).map_err(|e| format!("{e:?}"))?;
                    write_frame(
                        &mut server,
                        &encode_state_delta(1, sequence, 14, id - 6, id - 5, 1, &payload)?,
                    )
                    .await?;
                    sequence += 1;
                    opened = true;
                }
            }
            let mut rest = Vec::new();
            server.read_to_end(&mut rest).await?;
            Ok::<(), BoxError>(())
        });
        let mut admission = item_admission()?;
        admission.supported_capabilities = SELECTED;
        admission.deadline = Duration::from_millis(100);
        let mut session = Session::admit(client, admission).await?;
        for _ in 0..2 {
            let start = tokio::time::Instant::now();
            let result = session.open_depot_container(handle(1)).await?;
            assert_eq!(
                result,
                Some(if omit_delta {
                    ContainerViewOutcome::Stale
                } else {
                    ContainerViewOutcome::Opened
                })
            );
            if omit_delta {
                assert!(start.elapsed() >= Duration::from_millis(100));
                assert!(start.elapsed() < Duration::from_secs(1));
            }
        }
        drop(session);
        peer.await??;
        Ok::<(), BoxError>(())
    })?
}
