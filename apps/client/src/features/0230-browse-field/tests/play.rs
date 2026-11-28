use crate::play::play_channel;
#[test]
fn browse_field_panel_opens_and_sends() {
    let (link, mut rx, outbox) = play_channel();
    outbox.browse_field_initialize(true);
    link.browse_field_request(vec![]);
    assert!(matches!(
        rx.try_recv(),
        Ok(crate::combat_input::PlayCommand::ServerMechanic(_))
    ));
}

#[test]
fn browse_field_native_moves_use_session_item_commands() {
    use oteryn_session::{ItemHandle, ItemMoveDestination};
    let (link, mut rx, outbox) = play_channel();
    outbox.browse_field_initialize(true);
    let Some(source) = ItemHandle::new(7) else {
        return;
    };
    link.browse_field_move(source, ItemMoveDestination::MainBackpack);
    assert!(
        matches!(rx.try_recv(), Ok(crate::combat_input::PlayCommand::BrowseFieldMove(intent)) if intent.source == source)
    );
}

#[test]
fn browse_field_up_keeps_the_ground_container_view_id() -> Result<(), Box<dyn std::error::Error>> {
    let (link, mut rx, outbox) = play_channel();
    outbox.browse_field_initialize(true);
    link.browse_field_up(3);
    let command = rx.try_recv()?;
    let crate::combat_input::PlayCommand::ServerMechanic(command) = command else {
        return Err("missing container command".into());
    };
    assert_eq!((command.capability, command.command_type), (14, 21));
    assert_eq!(
        oteryn_session::container_tree::decode_container_view_intent(&command.payload)
            .map_err(|e| format!("{e:?}"))?,
        oteryn_session::container_tree::ContainerViewIntent::Up { view_id: 3 }
    );
    Ok(())
}

#[test]
fn browse_field_selects_the_first_visible_server_window() {
    let (link, _, outbox) = play_channel();
    outbox.browse_field_initialize(true);
    outbox.push(vec![oteryn_session::SessionEvent::ContainerViews(
        oteryn_session::AppliedDelta {
            server_sequence: 41,
            base_revision: 1,
            new_revision: 2,
            value: oteryn_session::container_tree::ContainerViews {
                views: vec![oteryn_session::container_tree::ContainerView {
                    view_id: 3,
                    container_handle: oteryn_session::ItemHandle::MIN,
                    parent_handle: None,
                    capacity: 30,
                    entries: vec![],
                    browse: Some(oteryn_session::browse_field::BrowseFieldPage {
                        position: oteryn_session::item_view::WorldTilePosition {
                            x: 100,
                            y: 200,
                            floor: 0,
                        },
                        index: 0,
                        total: 0,
                    }),
                }],
            },
        },
    )]);
    assert_eq!(link.browse_field_panel().view_id, 3);
}

#[path = "../native.rs"]
mod native;

#[test]
fn browse_field_native_controls_and_results() {
    use oteryn_client_runtime::cui3::browse_field::{
        BrowseFieldIntent, decode_browse_field_intent,
    };
    use oteryn_client_runtime::cui3::item_view::WorldTilePosition;
    use oteryn_client_runtime::server_mechanic::{CommandStatus, ServerMechanicResponse};
    use oteryn_client_ui::browse_field::{BrowseFieldPanel, apply_result};
    let mut p = BrowseFieldPanel {
        selected: true,
        ..Default::default()
    };
    assert!(native::submit(&mut p, false, ["bad", "200", "7", "0", "0"]).is_none());
    assert!(!p.pending);
    let command = native::submit(&mut p, false, ["100", "200", "7", "0", "0"]);
    assert!(
        matches!(command, Some(c) if c.capability == 230 && c.command_type == 440 && decode_browse_field_intent(&c.payload) == Ok(BrowseFieldIntent::Open { position: WorldTilePosition { x: 100, y: 200, floor: 7 } }))
    );
    assert!(!native::enabled(&p));
    assert!(native::result_text(&p).contains("Waiting"));
    assert!(native::submit(&mut p, true, ["", "", "", "1", "30"]).is_none());
    apply_result(
        &mut p,
        &ServerMechanicResponse {
            command_id: 1,
            status: CommandStatus::Accepted,
            server_sequence: 1,
            payload: vec![8, 1],
        },
    );
    assert!(native::enabled(&p));
    assert!(native::result_text(&p).contains("Opened"));
    let command = native::submit(&mut p, true, ["", "", "", "1", "30"]);
    assert!(
        matches!(command, Some(c) if decode_browse_field_intent(&c.payload) == Ok(BrowseFieldIntent::Seek { view_id: 1, index: 30 }))
    );
    apply_result(
        &mut p,
        &ServerMechanicResponse {
            command_id: 2,
            status: CommandStatus::Rejected,
            server_sequence: 2,
            payload: vec![],
        },
    );
    assert!(native::enabled(&p));
    assert!(native::result_text(&p).contains("refused"));
    p.selected = false;
    assert!(!native::enabled(&p));
}

#[test]
fn browse_field_native_item_controls_preserve_move_and_up_routes()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_session::{ItemEntry, ItemHandle, ItemMoveDestination};
    let (link, _, outbox) = play_channel();
    outbox.browse_field_initialize(true);
    let position = oteryn_session::item_view::WorldTilePosition {
        x: 100,
        y: 200,
        floor: 0,
    };
    let item = ItemEntry {
        appearance_id: 0,
        handle: ItemHandle::MIN,
        item_definition_ref: 42.try_into()?,
        count: 1.try_into()?,
        sub_type: 0,
    };
    let mut panel = link.browse_field_panel();
    panel
        .views
        .views
        .push(oteryn_session::container_tree::ContainerView {
            view_id: 3,
            container_handle: 50.try_into()?,
            parent_handle: None,
            capacity: 30,
            entries: vec![item],
            browse: Some(oteryn_session::browse_field::BrowseFieldPage {
                position,
                index: 0,
                total: 1,
            }),
        });
    panel.view_id = 3;
    panel.inventory.entries.push(ItemEntry {
        handle: 2.try_into()?,
        ..item
    });
    let command = native::item_command(&mut panel, 105, "0", "0");
    assert!(
        matches!(command,Some(crate::combat_input::PlayCommand::BrowseFieldMove(intent)) if intent.source==item.handle && intent.destination==ItemMoveDestination::MainBackpack)
    );
    assert!(panel.pending);
    panel.pending = false;
    let command = native::item_command(&mut panel, 106, "0", "0");
    assert!(
        matches!(command,Some(crate::combat_input::PlayCommand::BrowseFieldMove(intent)) if intent.source.get()==2 && intent.destination==ItemMoveDestination::Ground(position))
    );
    panel.pending = false;
    let command = native::item_command(&mut panel, 108, "invalid", "invalid");
    let Some(crate::combat_input::PlayCommand::ServerMechanic(command)) = command else {
        return Err("missing Up command".into());
    };
    assert_eq!((command.capability, command.command_type), (14, 21));
    assert_eq!(
        oteryn_session::container_tree::decode_container_view_intent(&command.payload),
        Ok(oteryn_session::container_tree::ContainerViewIntent::Up { view_id: 3 })
    );
    panel.pending = false;
    let command = native::item_command(&mut panel, 109, "0", "0");
    assert!(
        matches!(command, Some(crate::combat_input::PlayCommand::WorldContext(command)) if command.target == oteryn_client_runtime::world_interaction::Target::Item(item.handle) && command.action == oteryn_client_runtime::world_interaction::Action::Use)
    );
    assert!(!panel.pending);
    assert!(native::item_command(&mut panel, 105, "invalid", "0").is_none());
    assert!(!panel.pending);
    Ok(())
}

#[test]
fn browse_panel_accepts_negotiated_house_views_and_keeps_legacy_bound()
-> Result<(), Box<dyn std::error::Error>> {
    use oteryn_session::container_tree::{
        ContainerView, ContainerViews, encode_container_views_with_appearances,
    };
    let views = ContainerViews {
        views: vec![ContainerView {
            view_id: 0,
            container_handle: 1_u64.try_into()?,
            parent_handle: None,
            capacity: 25,
            entries: (2_u64..=26)
                .map(|id| {
                    Ok(oteryn_session::ItemEntry {
                        appearance_id: 2854,
                        handle: id.try_into()?,
                        item_definition_ref: 2854_u32.try_into()?,
                        count: 1_u32.try_into()?,
                        sub_type: 0,
                    })
                })
                .collect::<Result<_, Box<dyn std::error::Error>>>()?,
            browse: None,
        }],
    };
    let bytes = encode_container_views_with_appearances(&views, true, true, true)
        .map_err(|e| format!("{e:?}"))?;
    for selected in [false, true] {
        let (link, _, outbox) = play_channel();
        outbox.container_initialize_with_house_capacity(None, selected);
        outbox.browse_field_initialize(true);
        outbox.browse_field_views(&bytes, true);
        let panel = link.browse_field_panel();
        if selected {
            assert_eq!(panel.views, views);
            assert!(panel.feedback.is_none());
        } else {
            assert!(panel.views.views.is_empty());
            assert_eq!(panel.feedback.as_deref(), Some("Invalid container view"));
        }
    }
    Ok(())
}

#[cfg(bevy_shell)]
impl crate::play::Outbox {
    pub(crate) fn bevy_browse_field_supported(&self) {
        crate::play::mailbox::lock(&self.0).browse_field.selected = true;
    }
    /// Applies a server reply through the production browse-field reducer.
    pub(crate) fn bevy_browse_field_result(
        &self,
        response: &oteryn_client_runtime::server_mechanic::ServerMechanicResponse,
    ) {
        self.browse_field_result(response);
    }
}

type ReplayError = Box<dyn std::error::Error>;

#[derive(Clone, Copy)]
enum ReplayFault {
    None,
    MissingUpdate,
    MissingPickup,
}

struct BrowseReplayPeer {
    events: tokio::sync::mpsc::UnboundedReceiver<oteryn_session::SessionEvent>,
    moves: tokio::sync::mpsc::UnboundedSender<oteryn_client_runtime::inventory::MoveRequest>,
    pending: Vec<oteryn_session::SessionEvent>,
    fault: ReplayFault,
}

impl crate::play::Stepper for BrowseReplayPeer {
    fn server_mechanic_selected(&self, capability: u32) -> bool {
        capability == 230
    }

    fn drain(&mut self) -> Vec<oteryn_session::SessionEvent> {
        std::mem::take(&mut self.pending)
    }

    async fn serve(
        &mut self,
        duration: std::time::Duration,
    ) -> Result<(), oteryn_session::SessionError> {
        match tokio::time::timeout(duration, self.events.recv()).await {
            Ok(Some(event)) if !matches!(self.fault, ReplayFault::MissingUpdate) => {
                self.pending.push(event)
            }
            Ok(None) => return Err(oteryn_session::SessionError::SessionUnusable),
            _ => {}
        }
        Ok(())
    }

    async fn move_item(
        &mut self,
        request: oteryn_client_runtime::inventory::MoveRequest,
    ) -> Result<Option<oteryn_client_runtime::inventory::MoveResponse>, oteryn_session::SessionError>
    {
        if matches!(self.fault, ReplayFault::MissingPickup) {
            return Ok(None);
        }
        self.moves
            .send(request)
            .map_err(|_| oteryn_session::SessionError::SessionUnusable)?;
        Ok(None)
    }

    async fn combat(
        &mut self,
        _: crate::combat_input::CombatCommand,
    ) -> Result<crate::combat_input::CombatReport, oteryn_session::SessionError> {
        Err(oteryn_session::SessionError::SessionUnusable)
    }

    async fn step(
        &mut self,
        _: oteryn_session::StepDirection,
    ) -> Result<oteryn_session::StepOutcome, oteryn_session::SessionError> {
        Err(oteryn_session::SessionError::SessionUnusable)
    }

    async fn logout(
        &mut self,
    ) -> Result<Option<oteryn_session::LogoutResult>, oteryn_session::SessionError> {
        Err(oteryn_session::SessionError::SessionUnusable)
    }
}

async fn delivered_browse_panel(
    link: &crate::play::PlayLink,
    expected: &oteryn_session::container_tree::ContainerViews,
    timeout: std::time::Duration,
) -> Result<oteryn_client_ui::browse_field::BrowseFieldPanel, ReplayError> {
    tokio::time::timeout(timeout, async {
        loop {
            let panel = link.browse_field_panel();
            if panel.selected && &panel.views == expected {
                return panel;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    })
    .await
    .map_err(|_| "I18b captured delta missing from the production mailbox".into())
}

async fn dispatched_browse_pickup(
    moves: &mut tokio::sync::mpsc::UnboundedReceiver<oteryn_client_runtime::inventory::MoveRequest>,
    timeout: std::time::Duration,
) -> Result<oteryn_client_runtime::inventory::MoveRequest, ReplayError> {
    tokio::time::timeout(timeout, moves.recv())
        .await
        .map_err(|_| "I18b pickup missing from production session dispatch")?
        .ok_or_else(|| "I18b pickup missing from production session dispatch".into())
}

#[test]
#[ignore = "requires fresh I18b.full capture from a live qualification run"]
fn i18b_live_browse_field_replay() -> Result<(), ReplayError> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(replay_live_browse_field(
            ReplayFault::None,
            std::time::Duration::from_secs(2),
        ))
}

#[test]
#[ignore = "requires a live I18b capture to test replay rejection"]
fn i18b_replay_rejects_missing_delivery_and_dispatch() -> Result<(), ReplayError> {
    use std::time::Duration;

    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(async {
            for (fault, message) in [
                (
                    ReplayFault::MissingUpdate,
                    "I18b captured delta missing from the production mailbox",
                ),
                (
                    ReplayFault::MissingPickup,
                    "I18b pickup missing from production session dispatch",
                ),
            ] {
                let result = replay_live_browse_field(fault, Duration::from_secs(2)).await;
                assert_eq!(
                    result.err().map(|error| error.to_string()).as_deref(),
                    Some(message)
                );
            }
            Ok(())
        })
}

async fn replay_live_browse_field(
    fault: ReplayFault,
    timeout: std::time::Duration,
) -> Result<(), ReplayError> {
    use oteryn_session::{AppliedDelta, ItemMoveDestination, SessionEvent, container_tree};

    let root = std::path::PathBuf::from(
        std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("capture directory required")?,
    );
    let run = root
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("capture run name")?;
    let (link, commands, outbox) = play_channel();
    let (events, incoming) = tokio::sync::mpsc::unbounded_channel();
    let (sent, mut moves) = tokio::sync::mpsc::unbounded_channel();
    let runner = tokio::spawn(crate::play::run_session(
        BrowseReplayPeer {
            events: incoming,
            moves: sent,
            pending: Vec::new(),
            fault,
        },
        commands,
        outbox,
    ));
    let mut prior = None;
    let mut tile = None;
    for (phase, expected) in [("before-drop", 0), ("after-drop", 1), ("after-pickup", 0)] {
        let bytes = std::fs::read(root.join(format!("i18b-{phase}.bin")))?;
        let end = bytes
            .iter()
            .position(|&byte| byte == b'\n')
            .ok_or("capture header missing")?;
        let fields: Vec<_> = std::str::from_utf8(&bytes[..end])?
            .split_ascii_whitespace()
            .collect();
        let &[
            captured_run,
            session,
            sequence,
            base,
            revision,
            view,
            rope,
            house,
            appearances,
        ] = fields.as_slice()
        else {
            return Err("invalid capture header".into());
        };
        assert_eq!(captured_run, run);
        let sequence = sequence.parse::<u64>()?;
        let base = base.parse::<u64>()?;
        let revision = revision.parse::<u64>()?;
        let view = view.parse::<u8>()?;
        let rope = rope.parse::<u32>()?;
        assert!(matches!(house, "0" | "1") && matches!(appearances, "0" | "1"));
        assert!(revision > base && sequence > 0 && !session.is_empty());
        let identity = (
            session.to_owned(),
            view,
            rope,
            house == "1",
            appearances == "1",
        );
        if let Some((old_identity, old_sequence, old_revision)) = &prior {
            assert_eq!(&identity, old_identity);
            assert!(sequence > *old_sequence);
            assert_eq!(base, *old_revision);
        }
        let value = container_tree::decode_container_views_with_appearances(
            &bytes[end + 1..],
            true,
            identity.3,
            identity.4,
        )
        .map_err(|error| format!("captured container delta: {error:?}"))?;
        events.send(SessionEvent::ContainerViews(AppliedDelta {
            server_sequence: sequence,
            base_revision: base,
            new_revision: revision,
            value: value.clone(),
        }))?;
        let mut panel = delivered_browse_panel(&link, &value, timeout).await?;
        assert_eq!(panel.view_id, view);
        let field = panel
            .views
            .views
            .iter()
            .find(|field| field.view_id == view)
            .ok_or("captured browse view missing from the client panel")?;
        let position = field.browse.ok_or("captured tile absent")?.position;
        if let Some(previous) = tile.replace(position) {
            assert_eq!(position, previous);
        }
        let row = field
            .entries
            .iter()
            .position(|entry| entry.item_definition_ref.get() == rope);
        let count: u32 = field
            .entries
            .iter()
            .filter(|entry| entry.item_definition_ref.get() == rope)
            .map(|entry| entry.count.get())
            .sum();
        assert_eq!(count, expected);
        if let Some(row) = row {
            let handle = field.entries[row].handle;
            let Some(crate::combat_input::PlayCommand::BrowseFieldMove(intent)) =
                native::item_command(&mut panel, 105, &row.to_string(), "")
            else {
                return Err("native pickup action did not produce a move".into());
            };
            link.browse_field_move(intent.source, intent.destination);
            assert_eq!(
                dispatched_browse_pickup(&mut moves, timeout).await?,
                oteryn_client_runtime::inventory::MoveRequest {
                    source: handle,
                    destination: ItemMoveDestination::MainBackpack,
                    count: None,
                }
            );
        }
        prior = Some((identity, sequence, revision));
    }
    assert!(moves.try_recv().is_err());
    runner.abort();
    assert!(runner.await.is_err_and(|error| error.is_cancelled()));
    println!("I18B_CLIENT_REPLAY PASS run={run} phases=3");
    Ok(())
}
