use super::{play_channel, run_session};
use crate::combat_input::PlayCommand;
use crate::play::mailbox::lock;
use oteryn_foundation::CancellationToken;
use oteryn_session::test_support::wire::{self, DomainSnapshot};
use oteryn_session::{Admission, Session, read_frame, write_frame};
use std::time::Duration;
use tokio::io::DuplexStream;

type Error = Box<dyn std::error::Error + Send + Sync>;
const DEADLINE: Duration = Duration::from_secs(3);

fn id(marker: u8) -> [u8; 16] {
    let mut id = [marker; 16];
    id[6] = 0x70;
    id[8] = 0x80;
    id
}

fn spatial(x: i32) -> Vec<u8> {
    wire::world_spatial::encode_world_spatial(&wire::world_spatial::WorldSpatialObservation {
        content_generation: [17; 32],
        actor_position: wire::world_spatial::ActorPosition { x, y: 0, floor: 0 },
    })
}

async fn snapshot(
    peer: &mut DuplexStream,
    sequence: u64,
    domains: &[DomainSnapshot<'_>],
) -> Result<(), Error> {
    let mut domains = domains.to_vec();
    let overlay = wire::world_object::encode_world_object_overlay_snapshot(&[])
        .map_err(|error| format!("{error:?}"))?;
    if domains.iter().any(|domain| domain.domain_id == 1) {
        domains.push(DomainSnapshot {
            domain_id: 2,
            revision: 1,
            snapshot_type: 1,
            payload: &overlay,
        });
        domains.sort_by_key(|domain| domain.domain_id);
    }
    for frame in wire::encode_single_chunk_snapshot(1, 1, sequence, &domains)? {
        write_frame(peer, &frame).await?;
    }
    Ok(())
}

async fn join(peer: &mut DuplexStream, caps: &[u32]) -> Result<(), Error> {
    read_frame(peer).await?;
    write_frame(
        peer,
        &wire::encode_server_accepted(&wire::ServerAcceptedValue {
            game_session_id: wire::GameSessionId::decode(&id(1))?,
            world_id: wire::WorldId::decode(&id(2))?,
            channel_id: wire::ChannelId::decode(&id(3))?,
            connection_generation: 1,
            current_server_sequence: 0,
            next_command_id: 1,
            schema_revision: 1,
            selected_capabilities: caps,
        })?,
    )
    .await?;
    snapshot(
        peer,
        0,
        &[DomainSnapshot {
            domain_id: 1,
            revision: 1,
            snapshot_type: 1,
            payload: &spatial(100),
        }],
    )
    .await
}

async fn admit(client: DuplexStream, caps: &[u32]) -> Result<Session<DuplexStream>, Error> {
    Ok(Session::admit(
        client,
        Admission {
            appearance_items: None,
            schema_revision: 1,
            character_id: wire::CharacterId::decode(&id(4))?,
            admission_material: b"grant",
            client_build_id: "handoff-test",
            supported_capabilities: caps,
            deadline: DEADLINE,
        },
    )
    .await?)
}

fn scenario(
    test: impl std::future::Future<Output = Result<(), Error>> + Send + 'static,
) -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(async { tokio::time::timeout(DEADLINE, test).await? })
        })?
        .join()
        .map_err(|_| "handoff thread failed")?
}

#[test]
fn house_transfer_discards_queued_source_step_and_cancels_probe() -> Result<(), Error> {
    use oteryn_session::property_services::house_transfer as house;
    scenario(async {
        let caps = &[86];
        let (client, mut peer) = tokio::io::duplex(256 * 1024);
        let (session, joined) = tokio::join!(admit(client, caps), join(&mut peer, caps));
        joined?;
        let (link, commands, outbox) = play_channel();
        let probe = CancellationToken::new();
        lock(&link.mailbox).channel_probe_cancel = Some(probe.clone());
        let task = tokio::spawn(run_session(session?, commands, outbox));
        while !link.house_transfer_panel().selected {
            tokio::task::yield_now().await;
        }
        let key = "oteryn:content.house.test";
        link.house_transfer_edit(0, key.into());
        link.house_transfer_request(oteryn_client_ui::house_transfer::Intent::Enter);
        let received = read_frame(&mut peer).await?;
        let command = wire::decode_wire_envelope(&received)?.client_command(1)?;
        assert_eq!(command.command_type, house::COMMAND_TYPE_HOUSE_TRANSFER);
        link.commands
            .send(PlayCommand::Step(crate::input::StepDir::North))?;
        let ack = house::encode_house_transfer_ack(&house::HouseTransferAck {
            transfer: id(9),
            session: id(5),
            world: id(2),
            channel: id(3),
            house: Some(key.into()),
            scope_generation: 2,
            connection_generation: 1,
            lease_generation: 2,
            selection: caps.to_vec(),
            position: [200, 0, 0],
        })
        .map_err(|error| format!("{error:?}"))?;
        write_frame(
            &mut peer,
            &wire::encode_command_result(
                1,
                1,
                command.command_id,
                wire::CommandStatus::Accepted,
                &ack,
            )?,
        )
        .await?;
        snapshot(
            &mut peer,
            1,
            &[
                DomainSnapshot {
                    domain_id: 1,
                    revision: 1,
                    snapshot_type: 1,
                    payload: &spatial(200),
                },
                DomainSnapshot {
                    domain_id: 51,
                    revision: 1,
                    snapshot_type: 1,
                    payload: &ack,
                },
            ],
        )
        .await?;
        assert!(link.house_transfer_panel().pending);
        tokio::select! {
            frame = read_frame(&mut peer) => {
                let frame = frame?;
                let actual = wire::decode_wire_envelope(&frame)?.client_command(1)?.command_type;
                assert_eq!(actual, house::COMMAND_TYPE_HOUSE_TRANSFER, "queued source step reached the House destination");
            }
            () = probe.cancelled() => {}
        }
        drop(link);
        task.await?;
        Ok(())
    })
}

#[test]
fn idle_handoff_discards_queued_source_modal_answer() -> Result<(), Error> {
    use oteryn_client_runtime::server_modal::{Dialog, Reply};
    use oteryn_session::activity_scope::{
        ActivityScope, ActivityScopeSnapshot, encode_activity_scope,
    };
    scenario(async {
        let caps = &[117, 450];
        let (client, mut peer) = tokio::io::duplex(256 * 1024);
        let (session, joined) = tokio::join!(admit(client, caps), join(&mut peer, caps));
        joined?;
        let (link, commands, outbox) = play_channel();
        link.use_shared_frontend();
        let task = tokio::spawn(run_session(session?, commands, outbox));
        let received = read_frame(&mut peer).await?;
        let command = wire::decode_wire_envelope(&received)?.client_command(1)?;
        let dialog = Reply::Dialog(Dialog {
            id: 73,
            title: "Source only".into(),
            message: "Choose".into(),
            choices: vec![],
            buttons: vec![(4, "OK".into())],
            default_button: None,
            cancel_button: None,
            image: None,
        })
        .encode()
        .map_err(|error| format!("{error:?}"))?;
        write_frame(
            &mut peer,
            &wire::encode_command_result(
                1,
                1,
                command.command_id,
                wire::CommandStatus::Accepted,
                &dialog,
            )?,
        )
        .await?;
        let source = loop {
            if let Some(source) = link.server_modal_panel() {
                break source;
            }
            tokio::task::yield_now().await;
        };
        let scope = encode_activity_scope(&ActivityScopeSnapshot {
            handoff: id(9),
            source_session: id(1),
            session: id(5),
            scope: ActivityScope::Instance {
                world: id(2),
                instance: id(6),
            },
            scope_generation: 2,
            connection_generation: 1,
            lease_generation: 2,
            selection: caps.to_vec(),
            requires_ack: false,
        })
        .map_err(|error| format!("{error:?}"))?;
        snapshot(
            &mut peer,
            2,
            &[DomainSnapshot {
                domain_id: 99,
                revision: 1,
                snapshot_type: 1,
                payload: &scope,
            }],
        )
        .await?;
        assert!(link.server_modal_submit(
            source.revision,
            oteryn_client_runtime::server_modal::Answer {
                dialog_id: 73,
                button_id: 4,
                choice_id: None,
            },
            Default::default()
        ));
        snapshot(
            &mut peer,
            1,
            &[
                DomainSnapshot {
                    domain_id: 1,
                    revision: 1,
                    snapshot_type: 1,
                    payload: &spatial(200),
                },
                DomainSnapshot {
                    domain_id: 99,
                    revision: 1,
                    snapshot_type: 1,
                    payload: &scope,
                },
            ],
        )
        .await?;
        while !lock(&link.mailbox).scope_pending {
            tokio::task::yield_now().await;
        }
        // The same current-thread runtime has run the next poll up to its first await.
        assert!(
            link.server_modal_panel().is_none(),
            "source modal was republished in destination"
        );
        let mut view = super::view()?;
        view.tick(&link).map_err(|error| format!("{error:?}"))?;
        link.commands
            .send(PlayCommand::Step(crate::input::StepDir::East))?;
        let received = read_frame(&mut peer).await?;
        let command = wire::decode_wire_envelope(&received)?.client_command(1)?;
        assert_eq!(
            command.command_type, 1,
            "queued source modal answer reached the destination"
        );
        drop(peer);
        drop(link);
        task.await?;
        Ok(())
    })
}
