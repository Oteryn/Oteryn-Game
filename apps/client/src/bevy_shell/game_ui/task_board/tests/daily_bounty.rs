use super::*;

type DailyClaimError = Box<dyn std::error::Error + Send + Sync>;

#[test]
fn task_board_daily_bounty_gap_0578() -> Result<(), DailyClaimError> {
    daily_bounty_scenario(true)
}

#[test]
fn task_board_daily_bounty_gap_0578_legacy_bytes() -> Result<(), DailyClaimError> {
    daily_bounty_scenario(false)
}

fn daily_bounty_scenario(negotiated: bool) -> Result<(), DailyClaimError> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || {
            let (link, commands, outbox) = crate::play::play_channel();
            link.use_shared_frontend();
            let mut app = task_board_app(link).map_err(|e| e.to_string())?;
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(async {
                    tokio::time::timeout(
                        std::time::Duration::from_secs(5),
                        daily_bounty_session(negotiated, &mut app, commands, outbox),
                    )
                    .await?
                })
        })?
        .join()
        .map_err(|_| "daily bounty session test panicked")?
}

async fn daily_bounty_session(
    negotiated: bool,
    app: &mut App,
    commands: tokio::sync::mpsc::UnboundedReceiver<crate::combat_input::PlayCommand>,
    outbox: crate::play::Outbox,
) -> Result<(), DailyClaimError> {
    use oteryn_session::test_support::wire;
    use oteryn_session::{Admission, Session};

    let (client, mut peer) = tokio::io::duplex(64 * 1024);
    let caps = if negotiated {
        &[72, CAPABILITY_TASK_BOARD_DAILY_BOUNTY_V1][..]
    } else {
        &[72][..]
    };
    let id = |marker| [marker, 0, 0, 0, 0, 0, 0x70, 0, 0x80, 0, 0, 0, 0, 0, 0, 0];
    let mut reply = panel().weekly.ok_or("Task Board fixture")?;
    reply.weekly_results = None;
    reply.weekly.kills[0].race = "oteryn:creature.rat".into();
    reply.bounty.state = 0;
    reply.bounty.offers.clear();
    reply.bounty.current = 0;
    reply.bounty.selected = 0;
    reply.bounty.daily_reroll_at = u64::from(negotiated);
    let server = tokio::spawn(async move {
        use oteryn_session::{read_frame, write_frame};
        let bootstrap = read_frame(&mut peer).await?;
        assert_eq!(
            wire::decode_wire_envelope(&bootstrap)?
                .client_bootstrap()?
                .supported_capabilities,
            caps
        );
        write_frame(
            &mut peer,
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
        let spatial = wire::world_spatial::encode_world_spatial(
            &wire::world_spatial::WorldSpatialObservation {
                content_generation: [17; 32],
                actor_position: wire::world_spatial::ActorPosition {
                    x: 100,
                    y: 100,
                    floor: 7,
                },
            },
        );
        let overlay = wire::world_object::encode_world_object_overlay_snapshot(&[])
            .map_err(|e| format!("{e:?}"))?;
        for frame in wire::encode_single_chunk_snapshot(
            1,
            1,
            0,
            &[
                wire::DomainSnapshot {
                    domain_id: 1,
                    revision: 1,
                    snapshot_type: 1,
                    payload: &spatial,
                },
                wire::DomainSnapshot {
                    domain_id: 2,
                    revision: 1,
                    snapshot_type: 1,
                    payload: &overlay,
                },
            ],
        )? {
            write_frame(&mut peer, &frame).await?;
        }

        let script: &[(u32, &[u8])] = if negotiated {
            &[
                (89, &[8, 4, 16, 0, 26, 0]),
                (COMMAND_CLAIM_DAILY_BOUNTY_TOKEN, &[8, 20, 16, 0, 26, 0]),
            ]
        } else {
            &[(89, &[8, 4, 16, 0, 26, 0])]
        };
        for (i, &(kind, payload)) in script.iter().enumerate() {
            let frame = read_frame(&mut peer).await?;
            let command = wire::decode_wire_envelope(&frame)?.client_command(1)?;
            assert_eq!((command.command_type, command.payload), (kind, payload));
            assert_eq!(command.command_id, i as u64 + 1);
            if kind == COMMAND_CLAIM_DAILY_BOUNTY_TOKEN {
                reply.bounty.rerolls += 1;
                reply.bounty.daily_reroll_at = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)?
                    .as_secs()
                    + 86_400;
            }
            let payload = encode_reply(&reply).map_err(|e| format!("{e:?}"))?;
            write_frame(
                &mut peer,
                &wire::encode_command_result(
                    1,
                    i as u64 + 1,
                    command.command_id,
                    wire::CommandStatus::Accepted,
                    &payload,
                )?,
            )
            .await?;
        }
        // Keep the admitted session live until the client has projected the last reply.
        let _ = read_frame(&mut peer).await;
        Ok::<(), DailyClaimError>(())
    });

    let mut session = Session::admit(
        client,
        Admission {
            appearance_items: None,
            schema_revision: 1,
            character_id: wire::CharacterId::decode(&id(4))?,
            admission_material: b"task-board-test",
            client_build_id: "task-board-test",
            supported_capabilities: caps,
            deadline: std::time::Duration::from_secs(3),
        },
    )
    .await?;
    if !negotiated {
        assert!(matches!(
            session
                .gated_command(
                    CAPABILITY_TASK_BOARD_DAILY_BOUNTY_V1,
                    COMMAND_CLAIM_DAILY_BOUNTY_TOKEN,
                    &[8, 20, 16, 0, 26, 0],
                )
                .await,
            Err(oteryn_session::SessionError::CapabilityNotSelected {
                capability: CAPABILITY_TASK_BOARD_DAILY_BOUNTY_V1
            })
        ));
    }
    let task = tokio::spawn(crate::play::run_session(session, commands, outbox));
    let owner = app.world().non_send::<HostOwner>();
    let link = &owner.play.as_ref().ok_or("active Task Board session")?.link;
    while !link.task_board_panel().selected {
        tokio::task::yield_now().await;
    }
    link.open_task_board();
    while link.task_board_panel().pending {
        tokio::task::yield_now().await;
    }
    let opened = link.task_board_panel();
    if opened.weekly.is_none() {
        if server.is_finished() {
            server.await??;
        }
        return Err(format!("Task Board query failed: {:?}", opened.feedback).into());
    }

    app.update();
    let claim_button = |app: &mut App| {
        app.world_mut()
            .query::<(&Text, &ChildOf)>()
            .iter(app.world())
            .find_map(|(text, parent)| {
                (text.0 == "Claim daily reroll token"
                    && app.world().get::<Action>(parent.parent()).is_some())
                .then_some(parent.parent())
            })
    };
    if negotiated {
        let entity = claim_button(app).ok_or("Task Board did not offer the daily bounty claim")?;
        app.world_mut().trigger(Activate { entity });
        let owner = app.world().non_send::<HostOwner>();
        let link = &owner.play.as_ref().ok_or("active Task Board session")?.link;
        assert!(link.task_board_panel().pending);
        while link.task_board_panel().pending {
            tokio::task::yield_now().await;
        }
        app.update();
        assert!(
            app.world_mut()
                .query::<&Text>()
                .iter(app.world())
                .any(|text| text.0 == "Reroll Tokens: 3 / 10")
        );
        assert!(claim_button(app).is_none());
    } else {
        assert!(claim_button(app).is_none());
        assert!(
            app.world_mut()
                .query::<&Text>()
                .iter(app.world())
                .any(|text| text.0 == "Daily tokens: claim at the Daily Reward Wall")
        );
    }
    app.world_mut().non_send_mut::<HostOwner>().play = None;
    task.await?;
    server.await??;
    Ok(())
}
