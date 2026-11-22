use super::{
    DomainSnapshot, DuplexStream, Error, admit, id, join, lock, play_channel, read_frame,
    run_session, scenario, snapshot, spatial, wire, write_frame,
};

const HOUSE_KEY: &str = "oteryn:content.house.test";

async fn enter_house(
    source_message: bool,
    inspect_publication: bool,
) -> Result<
    (
        crate::play::PlayLink,
        DuplexStream,
        tokio::task::JoinHandle<()>,
    ),
    Error,
> {
    use oteryn_session::property_services::house_transfer as house;
    use wire::zones_nologout as messages;
    let mut caps = vec![86];
    if source_message {
        caps.push(messages::CAPABILITY_ZONES_NOLOGOUT_V1);
        caps.sort_unstable();
    }
    let (client, mut peer) = tokio::io::duplex(256 * 1024);
    let (session, joined) = tokio::join!(admit(client, &caps), join(&mut peer, &caps));
    joined?;
    let (link, commands, outbox) = play_channel();
    let task = tokio::spawn(run_session(session?, commands, outbox));
    while !link.house_transfer_panel().selected {
        tokio::task::yield_now().await;
    }
    link.house_transfer_edit(0, HOUSE_KEY.into());
    link.house_transfer_request(oteryn_client_ui::house_transfer::Intent::Enter);
    let frame = read_frame(&mut peer).await?;
    let command = wire::decode_wire_envelope(&frame)?.client_command(1)?;
    assert_eq!(command.command_type, house::COMMAND_TYPE_HOUSE_TRANSFER);
    if inspect_publication {
        let shared = link.mailbox.clone();
        crate::play::activity_scope::BEFORE_SCOPE_MARKER.with(|hook| {
            hook.replace(Some(Box::new(move || {
                if let Ok(state) = shared.try_lock() {
                    assert!(
                        state.pushed.own.is_none() || state.pushed.scope_session == Some(id(5)),
                        "observer saw the destination snapshot without its presentation marker"
                    );
                }
            })));
        });
    }
    if source_message {
        let payload = messages::encode_server_message(&messages::ServerMessage::Warning(
            "source raid".into(),
        ))
        .map_err(|error| format!("{error:?}"))?;
        write_frame(
            &mut peer,
            &wire::encode_state_delta(1, 1, 57, 1, 2, 1, &payload)?,
        )
        .await?;
    }
    let ack = house::encode_house_transfer_ack(&house::HouseTransferAck {
        transfer: id(9),
        session: id(5),
        world: id(2),
        channel: id(3),
        house: Some(HOUSE_KEY.into()),
        scope_generation: 2,
        connection_generation: 1,
        lease_generation: 2,
        selection: caps,
        position: [200, 0, 0],
    })
    .map_err(|error| format!("{error:?}"))?;
    write_frame(
        &mut peer,
        &wire::encode_command_result(
            1,
            if source_message { 2 } else { 1 },
            command.command_id,
            wire::CommandStatus::Accepted,
            &ack,
        )?,
    )
    .await?;
    let spatial = spatial(200);
    let mut domains = vec![
        DomainSnapshot {
            domain_id: 1,
            revision: 1,
            snapshot_type: 1,
            payload: &spatial,
        },
        DomainSnapshot {
            domain_id: 51,
            revision: 1,
            snapshot_type: 1,
            payload: &ack,
        },
    ];
    let motd = messages::encode_server_message(&messages::ServerMessage::StatusText(
        "destination welcome".into(),
    ))
    .map_err(|error| format!("{error:?}"))?;
    if source_message {
        domains.push(DomainSnapshot {
            domain_id: 57,
            revision: 1,
            snapshot_type: 1,
            payload: &motd,
        });
    }
    snapshot(&mut peer, 1, &domains).await?;
    while lock(&link.mailbox).pushed.scope_session != Some(id(5)) && !task.is_finished() {
        tokio::task::yield_now().await;
    }
    assert_eq!(lock(&link.mailbox).pushed.scope_session, Some(id(5)));
    if inspect_publication {
        crate::play::activity_scope::BEFORE_SCOPE_MARKER
            .with(|hook| assert!(hook.borrow().is_none()));
    }
    Ok((link, peer, task))
}

#[test]
fn house_snapshot_and_presentation_marker_are_published_together() -> Result<(), Error> {
    scenario(async {
        let (link, _peer, task) = enter_house(false, true).await?;
        drop(link);
        task.await?;
        Ok(())
    })
}

#[test]
fn house_transfer_drops_inflight_source_events_and_keeps_destination_motd() -> Result<(), Error> {
    scenario(async {
        let (link, _peer, task) = enter_house(true, false).await?;
        let mut view = super::super::view()?;
        view.tick(&link).map_err(|error| format!("{error:?}"))?;
        let messages: Vec<_> = view
            .server_messages()
            .map(|message| &message.value)
            .collect();
        assert_eq!(
            messages,
            [&wire::zones_nologout::ServerMessage::StatusText(
                "destination welcome".into()
            )],
            "source event queued before the House ACK leaked into the destination"
        );
        drop(link);
        task.await?;
        Ok(())
    })
}

#[test]
fn house_transfer_receipt_and_exit_key_survive_destination_initialization() -> Result<(), Error> {
    use oteryn_session::property_services::house_transfer as house;
    scenario(async {
        let (link, mut peer, task) = enter_house(false, false).await?;
        let mut view = super::super::view()?;
        view.tick(&link).map_err(|error| format!("{error:?}"))?;
        while !link.house_transfer_panel().selected {
            tokio::task::yield_now().await;
        }
        link.house_transfer_open();
        let panel = link.house_transfer_panel();
        assert!(
            panel.summary.contains(HOUSE_KEY) && !panel.feedback.is_empty(),
            "destination initialization erased the committed House receipt"
        );
        link.house_transfer_request(oteryn_client_ui::house_transfer::Intent::Exit);
        let frame = read_frame(&mut peer).await?;
        let command = wire::decode_wire_envelope(&frame)?.client_command(1)?;
        let intent = house::decode_house_transfer_intent(command.payload)
            .map_err(|error| format!("{error:?}"))?;
        assert!(intent.exit);
        assert_eq!(intent.house, HOUSE_KEY);
        drop(peer);
        drop(link);
        task.await?;
        Ok(())
    })
}
