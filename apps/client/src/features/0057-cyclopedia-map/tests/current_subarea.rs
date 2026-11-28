use super::*;
use oteryn_client_runtime::cyclopedia_map as map;
use oteryn_client_ui::cyclopedia_map::{Intent, panel_text};

#[test]
fn gap_0093_current_subarea_follows_confirmed_location() -> Result<(), Error> {
    scenario(async {
        let caps = &[
            map::CAPABILITY_CYCLOPEDIA_MAP_V1,
            map::CAPABILITY_SUBAREA_V1,
        ];
        let (client, mut peer) = tokio::io::duplex(64 * 1024);
        let (session, joined) = tokio::join!(admit(client, caps), join(&mut peer, caps));
        joined?;
        let (link, commands, outbox) = play_channel();
        let task = tokio::spawn(run_session(session?, commands, outbox));
        while !link.cyclopedia_map_panel().selected {
            tokio::task::yield_now().await;
        }
        link.cyclopedia_atlas_open();
        let regions =
            map::geometry::encode_reply(&map::geometry::Reply::Areas(map::geometry::AreaPage {
                parent: 0,
                after: 0,
                next: None,
                areas: vec![map::geometry::Area {
                    id: 23,
                    parent: 0,
                    name: "Thais".into(),
                }],
            }))
            .map_err(|error| format!("{error:?}"))?;
        reply(&mut peer, 1, 72, &[255, 1, 0, 0, 0, 0, 127, 0, 0], &regions).await?;
        reply(
            &mut peer,
            2,
            109,
            &[],
            &map::encode_current_subarea(Some((23, 109))),
        )
        .await?;
        let membership = "Current region: Thais | Current subarea: #109";
        while !panel_text(&link.cyclopedia_map_panel()).contains(membership) {
            tokio::task::yield_now().await;
        }

        #[cfg(bevy_shell)]
        {
            let presenter = crate::play::PlayLink {
                owns_session: false,
                commands: link.commands.clone(),
                mailbox: link.mailbox.clone(),
            };
            tokio::task::spawn_blocking(move || {
                crate::bevy_shell::check_current_subarea_and_query(presenter, membership)
            })
            .await??;
        }
        #[cfg(not(bevy_shell))]
        link.cyclopedia_map_request(Intent::Query);
        assert_discovery_requests(&mut peer, 3).await?;

        write_frame(
            &mut peer,
            &wire::encode_state_delta(1, 5, 1, 1, 2, 1, &spatial(101))?,
        )
        .await?;
        reply(&mut peer, 6, 109, &[], &map::encode_current_subarea(None)).await?;
        while !panel_text(&link.cyclopedia_map_panel()).contains("Outside mapped subareas") {
            tokio::task::yield_now().await;
        }
        assert!(!panel_text(&link.cyclopedia_map_panel()).contains("#109"));

        // The next native action still uses the discovery route. A repeated
        // command 109 at the same position would fail the wire assertion.
        link.cyclopedia_map_request(Intent::Query);
        assert_discovery_requests(&mut peer, 7).await?;
        drop(link);
        task.await?;
        Ok(())
    })
}

#[test]
fn gap_0093_legacy_discovery_keeps_old_bytes() -> Result<(), Error> {
    scenario(async {
        let caps = &[map::CAPABILITY_CYCLOPEDIA_MAP_V1];
        let (client, mut peer) = tokio::io::duplex(64 * 1024);
        let (session, joined) = tokio::join!(admit(client, caps), join(&mut peer, caps));
        joined?;
        let (link, commands, outbox) = play_channel();
        let task = tokio::spawn(run_session(session?, commands, outbox));
        while !link.cyclopedia_map_panel().selected {
            tokio::task::yield_now().await;
        }
        link.cyclopedia_map_open();
        write_frame(&mut peer, &wire::encode_liveness_probe(1, 1)?).await?;
        let frame = read_frame(&mut peer).await?;
        assert_eq!(
            wire::decode_wire_envelope(&frame)?
                .liveness_ack(1)?
                .probe_id,
            1
        );
        link.cyclopedia_map_request(Intent::Query);
        assert_discovery_requests(&mut peer, 1).await?;
        assert!(!panel_text(&link.cyclopedia_map_panel()).contains("Current subarea"));
        drop(link);
        task.await?;
        Ok(())
    })
}

async fn assert_discovery_requests(peer: &mut DuplexStream, sequence: u64) -> Result<(), Error> {
    let status = map::encode_result(&map::MapResult {
        code: 0,
        revision: 1,
        discovered: false,
        found: 0,
        area_percent: 0,
        pool: 0,
        epoch: 0,
        improved: false,
        visible: Vec::new(),
    })
    .map_err(|error| format!("{error:?}"))?;
    reply(peer, sequence, 72, &[], &status).await?;
    reply(peer, sequence + 1, 6178, &[], &[0, 0]).await
}

async fn reply(
    peer: &mut DuplexStream,
    sequence: u64,
    kind: u32,
    request: &[u8],
    payload: &[u8],
) -> Result<(), Error> {
    let received = read_frame(peer).await?;
    let command = wire::decode_wire_envelope(&received)?.client_command(1)?;
    assert_eq!(command.command_type, kind);
    assert_eq!(command.payload, request);
    write_frame(
        peer,
        &wire::encode_command_result(
            1,
            sequence,
            command.command_id,
            wire::CommandStatus::Accepted,
            payload,
        )?,
    )
    .await?;
    Ok(())
}
