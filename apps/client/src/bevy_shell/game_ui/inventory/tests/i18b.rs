use super::*;

type ReplayError = Box<dyn std::error::Error>;

#[derive(Clone, Copy)]
enum ReplayFault {
    None,
    MissingUpdate,
    MissingPickup,
    MissingDragStart,
}

struct BrowseReplayPeer {
    events: tokio::sync::mpsc::UnboundedReceiver<oteryn_session::SessionEvent>,
    moves: tokio::sync::mpsc::UnboundedSender<oteryn_client_runtime::inventory::MoveRequest>,
    pending: Vec<oteryn_session::SessionEvent>,
    fault: ReplayFault,
    inventory: CharacterInventory,
}

impl crate::play::Stepper for BrowseReplayPeer {
    fn server_mechanic_selected(&self, capability: u32) -> bool {
        capability == 230
    }

    fn inventory(&self) -> Option<CharacterInventory> {
        Some(self.inventory.clone())
    }

    fn equipment_selected(&self) -> bool {
        true
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
                    ReplayFault::MissingDragStart,
                    "I18b pickup missing from production session dispatch",
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
    use oteryn_session::container_tree;

    let root = std::path::PathBuf::from(
        std::env::var_os("E2E_ALL_CAPTURE_DIR").ok_or("capture directory required")?,
    );
    let run = root
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("capture run name")?;
    let inventory_capture = std::fs::read(root.join("i18b-drag-inventory.bin"))?;
    let inventory_end = inventory_capture
        .iter()
        .position(|&byte| byte == b'\n')
        .ok_or("captured inventory header missing")?;
    let inventory_binding = std::str::from_utf8(&inventory_capture[..inventory_end])?;
    let inventory =
        oteryn_session::test_support::wire::item_view::decode_character_inventory_with_appearances(
            &inventory_capture[inventory_end + 1..],
            true,
            true,
        )
        .map_err(|error| format!("captured drag inventory: {error:?}"))?;
    let backpack = inventory
        .main_backpack
        .ok_or("captured main backpack absent")?
        .handle;
    let (link, commands, outbox) = play_channel();
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<Font>()
        .init_resource::<InventoryUi>()
        .insert_non_send(host(link)?)
        .add_systems(
            Startup,
            (crate::bevy_shell::ui::setup, super::super::super::setup).chain(),
        )
        .add_systems(
            Update,
            (super::super::super::project, view::project).chain(),
        );
    world_drag::install(&mut app);
    app.init_resource::<bevy::picking::hover::HoverMap>()
        .init_resource::<bevy::picking::pointer::PointerMap>();
    app.world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow));
    let (events, incoming) = tokio::sync::mpsc::unbounded_channel();
    let (sent, mut moves) = tokio::sync::mpsc::unbounded_channel();
    let runner = tokio::spawn(crate::play::run_session(
        BrowseReplayPeer {
            events: incoming,
            moves: sent,
            pending: Vec::new(),
            fault,
            inventory,
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
        if phase == "after-drop" {
            assert_eq!(inventory_binding, format!("{run} {session} {sequence}"));
        }
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
        let panel = {
            let owner = app.world().non_send::<HostOwner>();
            let link = &owner
                .play
                .as_ref()
                .ok_or("Bevy replay owner missing play")?
                .link;
            delivered_browse_panel(link, &value, timeout).await?
        };
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
            drag_to_backpack(&mut app, field, handle, backpack, fault)?;
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

fn drag_to_backpack(
    app: &mut App,
    field: &oteryn_session::container_tree::ContainerView,
    item: oteryn_session::ItemHandle,
    backpack: oteryn_session::ItemHandle,
    fault: ReplayFault,
) -> TestResult {
    use bevy::picking::{
        backend::HitData,
        events::{Pointer, PointerDragDrop, PointerDragStart, PointerPress},
        pointer::{Location as PointerLocation, PointerButton, PointerId},
    };

    // The production projection creates the source and destination and installs the observers.
    app.update();
    let mut slots = app.world_mut().query::<(Entity, &Slot)>();
    let source = slots
        .iter(app.world())
        .find_map(|(entity, slot)| {
            (slot.location
                == Location::Container {
                    view: field.view_id,
                    handle: field.container_handle,
                }
                && slot.item.is_some_and(|entry| entry.handle == item))
            .then_some(entity)
        })
        .ok_or("captured browse rope did not project to a Bevy slot")?;
    let target = slots
        .iter(app.world())
        .find_map(|(entity, slot)| {
            (slot.location == Location::Inventory
                && slot.item.is_some_and(|entry| entry.handle == backpack)
                && slot.destination == Some(ItemMoveDestination::MainBackpack))
            .then_some(entity)
        })
        .ok_or("captured backpack did not project to a Bevy drop target")?;
    let owner = app.world().non_send::<HostOwner>();
    let source_slot = *app
        .world()
        .get::<Slot>(source)
        .ok_or("source Slot absent")?;
    let target_slot = *app
        .world()
        .get::<Slot>(target)
        .ok_or("target Slot absent")?;
    assert!(
        source_slot.drag(owner).is_some(),
        "projected browse item cannot be dragged"
    );
    assert!(
        target_slot.current(owner).is_some(),
        "projected backpack is not a current drop target"
    );
    let window = app
        .world_mut()
        .query_filtered::<Entity, With<bevy::window::PrimaryWindow>>()
        .single(app.world())?;
    let window = bevy::window::WindowRef::Entity(window)
        .normalize(None)
        .ok_or("Bevy replay window is unbound")?;
    let location = |position| PointerLocation {
        target: bevy::camera::NormalizedRenderTarget::Window(window),
        position,
    };
    let start = Vec2::splat(30.);
    let end = Vec2::splat(80.);
    for (entity, center) in [(source, start), (target, end)] {
        app.world_mut().entity_mut(entity).insert((
            ComputedNode {
                size: Vec2::splat(32.),
                ..default()
            },
            UiGlobalTransform::from_translation(center),
        ));
    }
    let mouse = app
        .world_mut()
        .spawn((
            PointerId::Mouse,
            bevy::picking::pointer::PointerLocation::new(location(start)),
        ))
        .id();
    let pointer = |point| Pointer::new(PointerId::Mouse, location(point));
    app.world_mut().trigger(PointerPress {
        entity: source,
        pointer: pointer(start),
        button: PointerButton::Primary,
        hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
        count: 1,
    });
    if !matches!(fault, ReplayFault::MissingDragStart) {
        app.world_mut().trigger(PointerDragStart {
            entity: source,
            pointer: pointer(start),
            button: PointerButton::Primary,
            hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
        });
    }
    if !matches!(fault, ReplayFault::MissingDragStart) {
        assert_eq!(
            app.world().resource::<InventoryUi>().dragged,
            Some(source_slot),
            "Bevy did not begin the browse drag",
        );
    }
    app.world_mut()
        .entity_mut(mouse)
        .insert(bevy::picking::pointer::PointerLocation::new(location(end)));
    app.world_mut().trigger(PointerDragDrop {
        entity: target,
        pointer: pointer(end),
        button: PointerButton::Primary,
        dropped: source,
        hit: HitData::new(Entity::PLACEHOLDER, 0., None, None),
    });
    Ok(())
}
