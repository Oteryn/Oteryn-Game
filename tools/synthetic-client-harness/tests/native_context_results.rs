//! C12 review: real session exchanges and a held UI frame while live pushes are reduced.
use oteryn_client::play::{PlayView, play_channel, run_session};
use oteryn_client_ui::world_interaction::protocol::{
    self as context, Action, Command, Selection, Target,
};
use oteryn_protocol_oteryn::{
    ChannelId, CharacterId, CommandStatus, DomainSnapshot, GameSessionId, ServerAcceptedValue,
    WorldId, decode_wire_envelope, encode_command_result, encode_server_accepted,
    encode_single_chunk_snapshot, encode_state_delta,
    item_look::{self, ItemLookIntent, ItemLookResult, ItemLookSelection, ItemLookTarget},
    item_view::{self, CharacterInventory, OpenContainer},
    world_map::*,
    world_object::{self, UseDisposition},
    world_spatial::{ActorPosition, StepDirection},
    world_spatial_entities::*,
};
use oteryn_renderer::TileCoord;
use oteryn_session::{Admission, CLIENT_SUPPORTED_CAPABILITIES, Session, read_frame, write_frame};
use std::time::Duration;
use tokio::{
    io::{AsyncRead, DuplexStream, ReadBuf},
    sync::oneshot,
};
type Error = Box<dyn std::error::Error + Send + Sync>;
const WAIT: Duration = Duration::from_secs(5);

fn identity(marker: u8) -> [u8; 16] {
    [
        1, 0x93, 0x4f, 0x10, 0x7c, 0, 0x70, marker, 0x80, 0x5b, 0x3b, 0x11, 0x22, 0x33, 0x44,
        marker,
    ]
}
fn position(x: i32) -> ActorPosition {
    ActorPosition {
        x,
        y: 200,
        floor: 0,
    }
}
fn actor(x: i32, id: u8) -> WorldSpatialEntity {
    WorldSpatialEntity {
        display_name: None,
        entity: EntityRef {
            identity: [id; 16],
            generation: 1,
        },
        kind: if id == 7 {
            EntityKind::Player
        } else {
            EntityKind::Creature
        },
        position: position(x),
        detail: EntityDetail::Actor {
            direction: StepDirection::South,
            appearance_ref: 1,
            health_percent: 100,
        },
    }
}
fn map() -> WorldMapViewSnapshot {
    WorldMapViewSnapshot {
        header: MapViewHeader {
            content_generation: [1; 32],
            bundle_digest: [2; 32],
            reset_epoch: 1,
            origin: position(100),
            first_visible_floor: 0,
        },
        tiles: vec![MapTile {
            position: position(101),
            items: vec![MapItem {
                definition: MapDefinition::Item(std::num::NonZeroU32::MIN),
                count: 1,
                sub_type: 0,
                origin: MapOrigin::Handle(std::num::NonZeroU64::MIN),
                appearance_id: 0,
            }],
            more: false,
            ground_speed: 0,
        }],
    }
}
fn command(action: Action) -> Command {
    let (target, position, binding) = if action == Action::Look {
        (Target::Creature(actor(102, 9).entity), position(102), None)
    } else {
        (
            Target::Item(std::num::NonZeroU64::MIN),
            position(101),
            Some(([1; 32], [2; 32], 1)),
        )
    };
    Command {
        target,
        action,
        selection: Some(Selection { position, binding }),
    }
}
async fn admit(stream: DuplexStream) -> Result<Session<DuplexStream>, Error> {
    Ok(Session::admit(
        stream,
        Admission {
            appearance_items: None,
            schema_revision: 1,
            character_id: CharacterId::decode(&identity(4))?,
            admission_material: b"grant",
            client_build_id: "c12-context-review",
            supported_capabilities: CLIENT_SUPPORTED_CAPABILITIES,
            deadline: WAIT,
        },
    )
    .await?)
}
async fn join(stream: &mut DuplexStream) -> Result<(), Error> {
    read_frame(stream).await?;
    write_frame(
        stream,
        &encode_server_accepted(&ServerAcceptedValue {
            game_session_id: GameSessionId::decode(&identity(1))?,
            world_id: WorldId::decode(&identity(2))?,
            channel_id: ChannelId::decode(&identity(3))?,
            connection_generation: 1,
            current_server_sequence: 0,
            next_command_id: 7,
            schema_revision: 1,
            selected_capabilities: &[4, 6, 11, 15, 18, 22, 28],
        })?,
    )
    .await?;
    let spatial = encode_world_spatial_entities_snapshot(&WorldSpatialEntitiesSnapshot {
        content_generation: [1; 32],
        actor_position: position(100),
        own_identity: [7; 16],
        entities: vec![actor(100, 7), actor(102, 9)],
    })
    .map_err(|e| format!("{e:?}"))?;
    let inventory = item_view::encode_character_inventory(&CharacterInventory {
        main_backpack: Some(item_view::ItemEntry {
            handle: 2.try_into()?,
            item_definition_ref: 1.try_into()?,
            count: 1.try_into()?,
            sub_type: 0,
        }),
        entries: [3]
            .into_iter()
            .map(|handle| {
                Ok(item_view::ItemEntry {
                    handle: handle.try_into()?,
                    item_definition_ref: 1.try_into()?,
                    count: 1.try_into()?,
                    sub_type: 0,
                })
            })
            .collect::<Result<Vec<_>, Error>>()?,
        equipment: vec![],
    })
    .map_err(|e| format!("{e:?}"))?;
    let container = item_view::encode_open_container(&OpenContainer::default())
        .map_err(|e| format!("{e:?}"))?;
    let mut map_view = map();
    map_view.tiles.push(MapTile {
        position: position(103),
        items: vec![MapItem {
            definition: MapDefinition::Terrain(std::num::NonZeroU32::MIN),
            count: 1,
            sub_type: 0,
            origin: MapOrigin::BaseOrdinal {
                ordinal: 0,
                object_revision: 0,
            },
            appearance_id: 0,
        }],
        more: false,
        ground_speed: 0,
    });
    let mut other = map_view.tiles.first().cloned().ok_or("tile")?;
    other.position = position(104);
    other.items.first_mut().ok_or("item")?.origin = MapOrigin::Handle(4.try_into()?);
    map_view.tiles.push(other);
    let map = encode_world_map_snapshot(&map_view).map_err(|e| format!("{e:?}"))?;
    let domains = [
        DomainSnapshot {
            domain_id: 1,
            revision: 5,
            snapshot_type: 2,
            payload: &spatial,
        },
        DomainSnapshot {
            domain_id: 9,
            revision: 1,
            snapshot_type: 1,
            payload: &inventory,
        },
        DomainSnapshot {
            domain_id: 11,
            revision: 1,
            snapshot_type: 1,
            payload: &container,
        },
        DomainSnapshot {
            domain_id: 17,
            revision: 1,
            snapshot_type: 1,
            payload: &map,
        },
    ];
    for frame in encode_single_chunk_snapshot(1, 1, 40, &domains)? {
        write_frame(stream, &frame).await?;
    }
    Ok(())
}
fn on_thread(
    future: impl std::future::Future<Output = Result<(), Error>> + Send + 'static,
) -> Result<(), Error> {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(future)
        })?
        .join()
        .map_err(|_| "context review thread failed")?
}

async fn no_command(
    stream: &mut DuplexStream,
    mut finished: oneshot::Receiver<()>,
) -> Result<(), Error> {
    use std::{future::Future, pin::Pin, task::Poll};
    std::future::poll_fn(|cx| {
        let mut byte = [0];
        let mut buffer = ReadBuf::new(&mut byte);
        if let Poll::Ready(result) = Pin::new(&mut *stream).poll_read(cx, &mut buffer) {
            return Poll::Ready(Err(format!(
                "stale command or closed session on wire: {result:?}"
            )
            .into()));
        }
        match Pin::new(&mut finished).poll(cx) {
            Poll::Ready(result) => Poll::Ready(result.map_err(Into::into)),
            Poll::Pending => Poll::Pending,
        }
    })
    .await
}

#[test]
fn context_runtime_send_decodes_real_action_results_and_survives_unknown_payloads()
-> Result<(), Error> {
    on_thread(async {
        let (client, mut server) = tokio::io::duplex(64 * 1024);
        let (finish, finished) = oneshot::channel();
        let peer = tokio::spawn(async move {
            join(&mut server).await?;
            let text = item_look::encode_item_look_result_for(
                &ItemLookResult::Text("You see a dragon.\n翼".into()),
                ItemLookSelection { extended: true },
            )
            .map_err(|e| format!("{e:?}"))?;
            let use_result = world_object::encode_use_result(UseDisposition::Committed)
                .map_err(|e| format!("{e:?}"))?;
            let bed_result = world_object::encode_use_result(UseDisposition::BedOccupied)
                .map_err(|e| format!("{e:?}"))?;
            let replies = [
                (30, CommandStatus::Accepted, text),
                (2, CommandStatus::Accepted, use_result.clone()),
                (2, CommandStatus::Accepted, use_result.clone()),
                (31, CommandStatus::Accepted, bed_result),
                (30, CommandStatus::Rejected, vec![]),
                (30, CommandStatus::Accepted, vec![255]),
                (2, CommandStatus::Accepted, vec![255]),
                (2, CommandStatus::Accepted, vec![255]),
                (31, CommandStatus::Accepted, vec![255]),
                (2, CommandStatus::Accepted, use_result),
                (2, CommandStatus::Rejected, vec![]),
                (2, CommandStatus::Rejected, vec![]),
                (31, CommandStatus::Rejected, vec![]),
            ];
            for (index, (kind, status, payload)) in replies.into_iter().enumerate() {
                let frame = read_frame(&mut server).await?;
                let envelope = decode_wire_envelope(&frame)?;
                let request = envelope.client_command(1)?;
                let id = 7 + u64::try_from(index)?;
                assert_eq!((request.command_id, request.command_type), (id, kind));
                if kind == 30 {
                    assert_eq!(
                        item_look::decode_item_look_intent_for(
                            request.payload,
                            ItemLookSelection { extended: true }
                        )
                        .map_err(|e| format!("{e:?}"))?,
                        ItemLookIntent {
                            target: ItemLookTarget::Creature(actor(102, 9).entity)
                        }
                    );
                } else if kind == 31 {
                    let bed = oteryn_protocol_oteryn::bed::decode_bed_sleep_intent(request.payload)
                        .map_err(|e| format!("{e:?}"))?;
                    assert_eq!((bed.target.get(), bed.skill), (1, 6));
                } else {
                    assert_eq!(
                        request.payload,
                        world_object::encode_use_item_intent(std::num::NonZeroU64::MIN)
                    );
                }
                write_frame(
                    &mut server,
                    &encode_command_result(1, 41 + u64::try_from(index)?, id, status, &payload)?,
                )
                .await?;
            }
            finished.await?;
            Ok::<_, Error>(())
        });
        let mut session = admit(client).await?;
        for (action, expected) in [
            (Action::Look, "You see a dragon.\n翼"),
            (Action::Use, "Use: completed"),
            (Action::Open, "Open: completed"),
            (Action::Bed(6), "Bed: this bed is occupied"),
            (Action::Look, "Look refused"),
            (Action::Look, "Look: unknown result"),
            (Action::Use, "Use: unknown result"),
            (Action::Open, "Open: unknown result"),
            (Action::Bed(6), "Bed: unknown result"),
            (Action::Use, "Use: completed"),
            (Action::Use, "Use refused"),
            (Action::Open, "Open refused"),
            (Action::Bed(6), "Bed refused"),
        ] {
            assert_eq!(
                context::send(&mut session, &command(action)).await?,
                expected
            );
        }
        finish.send(()).map_err(|_| "peer gone")?;
        peer.await??;
        // Transport loss still fails the flow, unlike a local result decoder.
        assert!(
            context::send(&mut session, &command(Action::Use))
                .await
                .is_err()
        );
        Ok(())
    })
}

#[test]
fn context_held_menu_drops_creature_moves_death_and_old_map_epoch_at_dispatch() -> Result<(), Error>
{
    on_thread(async {
        for leave in [false, true] {
            let (client, mut server) = tokio::io::duplex(64 * 1024);
            let (push, pushed) = oneshot::channel();
            let (finish, finished) = oneshot::channel();
            let peer = tokio::spawn(async move {
                join(&mut server).await?;
                pushed.await?;
                let payload = encode_world_spatial_entities_delta(&WorldSpatialEntitiesDelta {
                    content_generation: [1; 32],
                    actor_position: position(100),
                    enter: vec![],
                    update: if leave { vec![] } else { vec![actor(103, 9)] },
                    leave: if leave {
                        vec![actor(102, 9).entity]
                    } else {
                        vec![]
                    },
                })
                .map_err(|e| format!("{e:?}"))?;
                write_frame(
                    &mut server,
                    &encode_state_delta(1, 41, 1, 5, 6, 2, &payload)?,
                )
                .await?;
                // No command is allowed after this push, even if the UI still displays its old frame.
                no_command(&mut server, finished).await?;
                Ok::<_, Error>(())
            });
            let session = admit(client).await?;
            let mut view = PlayView::from_join(session.join_snapshot())?;
            view.set_live_map(&session.world_map_snapshot().ok_or("map")?)?;
            view = view.with_entities(
                session
                    .world_entities()
                    .ok_or("entities")?
                    .others()
                    .copied()
                    .collect(),
            )?;
            let (link, commands, outbox) = play_channel();
            let task = tokio::spawn(run_session(session, commands, outbox));
            let menu = tokio::time::timeout(WAIT, async {
                loop {
                    if let Some(menu) = view.world_context(TileCoord::new(102, 200), &link) {
                        break menu;
                    }
                    tokio::task::yield_now().await;
                }
            })
            .await
            .map_err(|_| "menu readiness timed out")?;
            push.send(()).map_err(|_| "push peer gone")?;
            tokio::time::timeout(WAIT, async {
                loop {
                    if let Some(oteryn_client::play::PlayEvent::Pushed(pushed)) = link.poll()
                        && ((leave && pushed.own == Some(position(100)))
                            || pushed.entities.as_ref().is_some_and(|entities| {
                                entities
                                    .iter()
                                    .any(|entity| entity.position == position(103))
                            }))
                    {
                        break;
                    }
                    tokio::task::yield_now().await;
                }
            })
            .await
            .map_err(|_| "live entity push timed out")?;
            assert!(view.world_context_choose(&link, &menu, Action::Look));
            tokio::time::timeout(WAIT, async {
                loop {
                    if link.world_context_feedback().as_deref() == Some("Target changed") {
                        break;
                    }
                    tokio::task::yield_now().await;
                }
            })
            .await
            .map_err(|_| {
                format!(
                    "Target changed feedback timed out: {:?}",
                    link.world_context_feedback()
                )
            })?;
            finish.send(()).map_err(|_| "finish peer gone")?;
            peer.await??;
            task.abort();
        }
        // A captured base-item menu from epoch 0 must not send into the current epoch 1.
        let (client, mut server) = tokio::io::duplex(64 * 1024);
        let (finish, finished) = oneshot::channel();
        let peer = tokio::spawn(async move {
            join(&mut server).await?;
            no_command(&mut server, finished).await?;
            Ok::<_, Error>(())
        });
        let mut session = admit(client).await?;
        let mut old = command(Action::Use);
        old.selection.as_mut().ok_or("selection")?.binding = Some(([1; 32], [2; 32], 0));
        assert_eq!(context::send(&mut session, &old).await?, "Target changed");
        finish.send(()).map_err(|_| "peer gone")?;
        peer.await??;
        Ok(())
    })
}

#[test]
fn native_use_with_targeting_cancels_and_projects_server_refusals() -> Result<(), Error> {
    on_thread(async {
        let (client, mut server) = tokio::io::duplex(64 * 1024);
        let (finish, finished) = oneshot::channel();
        let peer = tokio::spawn(async move {
            join(&mut server).await?;
            for (index, expected) in [
                (2, Target::Creature(actor(102, 9).entity)),
                (1, Target::Position(position(101))),
                (1, Target::Item(4.try_into()?)),
                (2, Target::Item(3.try_into()?)),
            ]
            .into_iter()
            .enumerate()
            {
                let frame = read_frame(&mut server).await?;
                let envelope = decode_wire_envelope(&frame)?;
                let request = envelope.client_command(1)?;
                assert_eq!(request.command_type, 2);
                let source = expected.0.try_into()?;
                assert_eq!(
                    request.payload,
                    context::use_with_payload(source, &expected.1)?
                );
                let payload = world_object::encode_use_result(if index == 0 {
                    UseDisposition::TargetIllegal
                } else {
                    UseDisposition::Exhausted
                })
                .map_err(|e| format!("{e:?}"))?;
                write_frame(
                    &mut server,
                    &encode_command_result(
                        1,
                        41 + u64::try_from(index)?,
                        request.command_id,
                        CommandStatus::Accepted,
                        &payload,
                    )?,
                )
                .await?;
            }
            no_command(&mut server, finished).await?;
            Ok::<_, Error>(())
        });
        let session = admit(client).await?;
        let mut view = PlayView::from_join(session.join_snapshot())?;
        view.set_live_map(&session.world_map_snapshot().ok_or("map")?)?;
        view = view.with_entities(
            session
                .world_entities()
                .ok_or("entities")?
                .others()
                .copied()
                .collect(),
        )?;
        let (link, commands, outbox) = play_channel();
        let task = tokio::spawn(run_session(session, commands, outbox));
        tokio::time::timeout(WAIT, async {
            while link.inventory_panel().inventory.is_none() {
                tokio::task::yield_now().await;
            }
        })
        .await?;
        let menu = view
            .world_context(TileCoord::new(101, 200), &link)
            .ok_or("menu")?;
        assert!(view.world_context_choose(&link, &menu, Action::UseWith));
        let escape = oteryn_input_actions::NormalizedInputEvent::Key {
            code: oteryn_input_actions::KeyCode::ESCAPE,
            state: oteryn_input_actions::ButtonState::Pressed,
            modifiers: oteryn_input_actions::Modifiers::NONE,
            repeat: false,
        };
        assert!(link.use_with_cancel_input(&[escape]));
        assert!(!view.use_with_click(TileCoord::new(102, 200), &link));
        assert!(link.inventory_select(2.try_into()?));
        assert!(link.inventory_use_rune());
        assert!(view.use_with_click(TileCoord::new(102, 200), &link));
        for index in 0..4 {
            let expected = if index == 0 {
                "Use with: illegal target or blocked line of sight"
            } else {
                "Use with: exhausted"
            };
            tokio::time::timeout(WAIT, async {
                while link.world_context_feedback().as_deref() != Some(expected) {
                    tokio::task::yield_now().await;
                }
            })
            .await?;
            match index {
                0 | 1 => {
                    let menu = view
                        .world_context(TileCoord::new(101, 200), &link)
                        .ok_or("menu")?;
                    assert!(view.world_context_choose(
                        &link,
                        &menu,
                        if index == 0 {
                            Action::UseRune
                        } else {
                            Action::UseWith
                        }
                    ));
                    assert!(view.use_with_click(
                        TileCoord::new(if index == 0 { 101 } else { 104 }, 200),
                        &link
                    ));
                }
                2 => {
                    assert!(link.inventory_select(2.try_into()?));
                    assert!(link.inventory_use_with());
                    assert!(link.inventory_select(3.try_into()?));
                    assert!(link.inventory_use_with());
                }
                _ => {}
            }
        }
        finish.send(()).map_err(|_| "peer gone")?;
        peer.await??;
        task.abort();
        Ok(())
    })
}
