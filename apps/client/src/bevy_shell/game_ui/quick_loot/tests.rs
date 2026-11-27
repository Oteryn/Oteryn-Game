use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use bevy::{asset::AssetPlugin, input_focus::InputFocus};
use oteryn_client_runtime::server_mechanic::ServerMechanicResponse;
use oteryn_client_ui::quick_loot::{
    QuickLootAction, QuickLootItemResult, QuickLootResult, decode_quick_loot_request,
    encode_quick_loot_result,
};
use oteryn_session::CommandStatus;
type TestResult = Result<(), Box<dyn std::error::Error>>;
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    Outbox,
);
pub(crate) fn fixture(selected: bool) -> Result<Fixture, Box<dyn std::error::Error>> {
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_quick_loot_selected(selected);
    let mut owner = HostOwner::new(None)?;
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
    owner.phase = crate::bevy_shell::owner::Phase::Playing;
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Font>()
        .init_asset::<Image>()
        .init_resource::<InputFocus>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    Ok((app, commands, outbox))
}
pub(crate) fn open(app: &mut App) -> TestResult {
    let event = oteryn_input_actions::NormalizedInputEvent::Key {
        code: oteryn_input_actions::KeyCode::new(20)?,
        state: oteryn_input_actions::ButtonState::Pressed,
        modifiers: oteryn_input_actions::Modifiers::one(oteryn_input_actions::Modifier::Control)
            .with(oteryn_input_actions::Modifier::Alt),
        repeat: false,
    };
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(&[event], false);
    app.update();
    Ok(())
}
pub(crate) fn click_corpse_tile(
    app: &mut App,
    actor: oteryn_renderer::TileCoord,
    tile: oteryn_renderer::TileCoord,
    floor: i16,
) -> TestResult {
    use oteryn_input_actions::{
        ButtonState, Modifiers, MouseButton, NormalizedInputEvent, PointerCoordinate, PointerDelta,
        PointerMotion, PointerPosition,
    };
    let mut owner = app.world_mut().non_send_mut::<HostOwner>();
    owner.settings.preferences.gameplay.loot_click = 1;
    let preferences = owner.settings.preferences.clone();
    let play = owner.play.as_mut().ok_or("play")?;
    play.view = PlayView::new(actor, floor, &[])?;
    play.view.use_shared_frontend();
    play.view.set_preferences(preferences)?;
    play.view
        .tick(&play.link)
        .map_err(|error| format!("I07 pointer scene: {error:?}"))?;
    let view = play.view.scene().view();
    let size = i32::try_from(view.tile_px())?;
    let px = (tile.x - view.origin().x) * size + size / 2;
    let py = (tile.y - view.origin().y) * size + size / 2;
    owner.route_input(
        &[
            NormalizedInputEvent::PointerMoved {
                position: PointerPosition::new(
                    PointerCoordinate::new(px)?,
                    PointerCoordinate::new(py)?,
                ),
                motion: PointerMotion::new(PointerDelta::new(0)?, PointerDelta::new(0)?),
            },
            NormalizedInputEvent::MouseButton {
                button: MouseButton::SECONDARY,
                state: ButtonState::Pressed,
                modifiers: Modifiers::NONE,
            },
        ],
        false,
    );
    owner.route_input(
        &[NormalizedInputEvent::MouseButton {
            button: MouseButton::SECONDARY,
            state: ButtonState::Released,
            modifiers: Modifiers::NONE,
        }],
        false,
    );
    Ok(())
}
pub(crate) fn prepare_filter_picker(
    app: &mut App,
    outbox: &Outbox,
    definitions: [u32; 3],
) -> TestResult {
    let entries = definitions
        .into_iter()
        .enumerate()
        .map(|(index, definition)| {
            Ok(oteryn_session::ItemEntry {
                appearance_id: 0,
                handle: u64::try_from(index + 1)?.try_into()?,
                item_definition_ref: definition.try_into()?,
                count: 1.try_into()?,
                sub_type: 0,
            })
        })
        .collect::<Result<Vec<_>, Box<dyn std::error::Error>>>()?;
    outbox.push(vec![oteryn_session::SessionEvent::Inventory(
        oteryn_session::AppliedDelta {
            server_sequence: 1,
            base_revision: 0,
            new_revision: 1,
            value: oteryn_session::CharacterInventory {
                main_backpack: None,
                equipment: vec![],
                entries,
            },
        },
    )]);
    super::super::install::install_scroll_memory(app);
    Ok(())
}

pub(crate) fn add_filter_item(app: &mut App, skip: bool, handle: usize) -> TestResult {
    let mode = if skip { "skip" } else { "accept" };
    let add = named(app, &format!("quick_loot.add.{mode}"))?;
    click(app, add);
    let pick = named(app, &format!("quick_loot.pick.{handle}"))?;
    click(app, pick);
    Ok(())
}
fn control(app: &mut App, kind: Kind) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| (control.kind == kind).then_some(entity))
        .ok_or("quick loot control absent".into())
}
fn named(app: &mut App, name: &str) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &Name)>()
        .iter(app.world())
        .find_map(|(entity, label)| (label.as_str() == name).then_some(entity))
        .ok_or_else(|| format!("missing {name}").into())
}
fn panel(app: &App) -> Result<Panel, Box<dyn std::error::Error>> {
    Ok(app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .discovery_panel()
        .loot)
}
fn destination(category: Option<Category>, fallback: bool) -> Action {
    Action::Destination {
        category,
        recipient: Recipient::MainContainer,
        id: None,
        fallback,
    }
}
fn response(outbox: &Outbox, status: CommandStatus, accepted: bool) -> TestResult {
    outbox.bevy_quick_loot_response(&ServerMechanicResponse {
        command_id: 1,
        server_sequence: 1,
        status,
        payload: encode_quick_loot_result(&QuickLootResult {
            items: vec![QuickLootItemResult {
                item: [7; 16],
                accepted,
            }],
        })
        .map_err(|error| format!("{error:?}"))?,
    });
    Ok(())
}
fn config_response(outbox: &Outbox, status: CommandStatus) -> TestResult {
    outbox.bevy_quick_loot_response(&ServerMechanicResponse {
        command_id: 1,
        server_sequence: 1,
        status,
        payload: encode_quick_loot_result(&QuickLootResult { items: vec![] })
            .map_err(|error| format!("{error:?}"))?,
    });
    Ok(())
}
fn request(
    commands: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
) -> Result<oteryn_client_ui::quick_loot::QuickLootRequest, Box<dyn std::error::Error>> {
    let PlayCommand::ServerMechanic(command) = commands.try_recv()? else {
        return Err("wrong command".into());
    };
    assert_eq!((command.capability, command.command_type), (98, 114));
    decode_quick_loot_request(&command.payload).map_err(|error| format!("{error:?}").into())
}

fn click(app: &mut App, entity: Entity) {
    use bevy::{
        camera::NormalizedRenderTarget,
        picking::{
            backend::HitData,
            events::{Pointer, PointerClick, PointerPress, PointerRelease},
            pointer::{Location, PointerButton, PointerId},
        },
    };
    let pointer = Pointer::new(
        PointerId::Mouse,
        Location {
            target: NormalizedRenderTarget::None {
                width: 800,
                height: 600,
            },
            position: Vec2::ZERO,
        },
    );
    let button = PointerButton::Primary;
    let hit = HitData::new(Entity::PLACEHOLDER, 0., None, None);
    let world = app.world_mut();
    world.trigger(PointerPress {
        entity,
        pointer: pointer.clone(),
        button,
        hit: hit.clone(),
        count: 1,
    });
    world.flush();
    world.trigger(PointerClick {
        entity,
        pointer: pointer.clone(),
        button,
        hit: hit.clone(),
        duration: std::time::Duration::ZERO,
        count: 1,
    });
    world.flush();
    world.trigger(PointerRelease {
        entity,
        pointer,
        button,
        hit,
    });
    app.update();
}

pub(crate) fn link(app: &App) -> Result<&crate::play::PlayLink, Box<dyn std::error::Error>> {
    Ok(&app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link)
}

pub(crate) fn click_action(app: &mut App, action: Action) -> TestResult {
    app.update();
    let entity = control(app, Kind::Request(action))?;
    click(app, entity);
    Ok(())
}

#[test]
fn quick_loot_panel_clicks_dispatch_commands() -> TestResult {
    let (mut app, mut commands, outbox) = fixture(true)?;
    app.add_plugins(bevy::ui_widgets::ButtonPlugin);
    let pouch = [9; 16];
    let link = &app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link;
    assert!(link.quick_loot_action(Action::Destination {
        category: None,
        recipient: Recipient::GoldPouch,
        id: Some(pouch),
        fallback: true,
    }));
    let _ = request(&mut commands)?;
    config_response(&outbox, CommandStatus::Accepted)?;
    outbox.push(vec![oteryn_session::SessionEvent::Inventory(
        oteryn_session::AppliedDelta {
            server_sequence: 1,
            base_revision: 0,
            new_revision: 1,
            value: oteryn_session::CharacterInventory {
                main_backpack: None,
                equipment: vec![],
                entries: vec![oteryn_session::ItemEntry {
                    appearance_id: 0,
                    handle: 7.try_into()?,
                    item_definition_ref: 42.try_into()?,
                    count: 1.try_into()?,
                    sub_type: 0,
                }],
            },
        },
    )]);
    open(&mut app)?;

    let toggle = control(&mut app, Kind::Request(Action::Select(Mode::Skip)))?;
    click(&mut app, toggle);
    let wire = request(&mut commands).map_err(|error| format!("filter mode click: {error}"))?;
    assert_eq!(
        (wire.action, wire.filter_mode),
        (QuickLootAction::FilterSelect, Some(Mode::Skip))
    );
    config_response(&outbox, CommandStatus::Accepted)?;
    app.update();

    let route = control(
        &mut app,
        Kind::Request(destination(Some(Category::Gold), true)),
    )?;
    click(&mut app, route);
    let wire = request(&mut commands).map_err(|error| format!("main backpack click: {error}"))?;
    assert_eq!(wire.action, QuickLootAction::Assign);
    assert_eq!(wire.category, Some(Category::Gold));
    assert_eq!(wire.recipient, Some(Recipient::MainContainer));
    config_response(&outbox, CommandStatus::Accepted)?;
    app.update();

    for (mode, name) in [(Mode::Skip, "skip"), (Mode::Accept, "accept")] {
        let add = named(&mut app, &format!("quick_loot.add.{name}"))?;
        click(&mut app, add);
        assert!(commands.try_recv().is_err());
        let pick = named(&mut app, "quick_loot.pick.7")?;
        click(&mut app, pick);
        let wire = request(&mut commands).map_err(|error| format!("add {name} click: {error}"))?;
        assert_eq!(wire.action, QuickLootAction::FilterAdd);
        assert_eq!(wire.filter_mode, Some(mode));
        assert_eq!(wire.filter_definition_ref, Some(42));
        config_response(&outbox, CommandStatus::Accepted)?;
        app.update();

        let remove = named(&mut app, &format!("quick_loot.remove.{name}.42"))?;
        click(&mut app, remove);
        let wire =
            request(&mut commands).map_err(|error| format!("remove {name} click: {error}"))?;
        assert_eq!(wire.action, QuickLootAction::FilterRemove);
        assert_eq!(wire.filter_mode, Some(mode));
        assert_eq!(wire.filter_definition_ref, Some(42));
        config_response(&outbox, CommandStatus::Accepted)?;
        app.update();
    }

    let assign = control(
        &mut app,
        Kind::Request(Action::Destination {
            category: Some(Category::Gold),
            recipient: Recipient::GoldPouch,
            id: Some(pouch),
            fallback: true,
        }),
    )
    .map_err(|error| format!("gold pouch assignment: {error}"))?;
    click(&mut app, assign);
    let wire = request(&mut commands)?;
    assert_eq!(wire.action, QuickLootAction::Assign);
    assert_eq!(wire.category, Some(Category::Gold));
    assert_eq!(wire.recipient, Some(Recipient::GoldPouch));
    assert_eq!(wire.recipient_id, Some(pouch));
    assert!(wire.fallback_to_main);
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn admitted_hotkey_opens_one_source_dialog_and_loot_all_uses_real_owner_once() -> TestResult {
    let (mut app, mut commands, outbox) = fixture(true)?;
    open(&mut app)?;
    assert!(
        panel(&app)?.open,
        "CtrlAltQ must reach canonical discovery owner"
    );
    assert_eq!(
        app.world_mut()
            .query::<&Overlay>()
            .iter(app.world())
            .count(),
        1
    );
    let loot = control(&mut app, Kind::Request(Action::LootAll))?;
    app.world_mut().trigger(Activate { entity: loot });
    app.world_mut().trigger(Activate { entity: loot });
    assert_eq!(request(&mut commands)?.action, QuickLootAction::LootAll);
    assert!(commands.try_recv().is_err());
    assert_eq!(panel(&app)?.pending, Some(Action::LootAll));
    assert!(panel(&app)?.result.is_none());
    response(&outbox, CommandStatus::Accepted, true)?;
    app.update();
    assert!(panel(&app)?.pending.is_none());
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0.contains("Looted 1 of 1"))
    );
    Ok(())
}
#[test]
fn destination_remains_unconfirmed_until_accepted_and_refusal_preserves_it() -> TestResult {
    let (mut app, mut commands, outbox) = fixture(true)?;
    open(&mut app)?;
    let assignment = destination(Some(Category::Gold), false);
    let button = control(&mut app, Kind::Request(assignment.clone()))?;
    app.world_mut().trigger(Activate { entity: button });
    assert!(panel(&app)?.destinations.is_empty());
    let wire = request(&mut commands)?;
    assert_eq!(
        (
            wire.action,
            wire.category,
            wire.recipient,
            wire.recipient_id,
            wire.fallback_to_main
        ),
        (
            QuickLootAction::Assign,
            Some(Category::Gold),
            Some(Recipient::MainContainer),
            None,
            false
        )
    );
    config_response(&outbox, CommandStatus::Accepted)?;
    app.update();
    assert_eq!(panel(&app)?.destinations, vec![assignment.clone()]);
    let backpack = oteryn_session::ItemEntry {
        handle: 10.try_into()?,
        item_definition_ref: 42.try_into()?,
        appearance_id: 0,
        count: 1.try_into()?,
        sub_type: 0,
    };
    crate::play::PlayLink::bevy_items_fixture(
        &outbox,
        Some(oteryn_session::CharacterInventory {
            main_backpack: Some(backpack),
            ..default()
        }),
        Some(Default::default()),
        true,
        true,
    );
    let open = control(&mut app, Kind::OpenDestination(Some(Category::Gold)))?;
    app.world_mut().trigger(Activate { entity: open });
    assert_eq!(
        commands.try_recv()?,
        PlayCommand::ContainerView(oteryn_session::container_tree::ContainerViewIntent::Open {
            handle: backpack.handle,
            replace_view: None
        })
    );
    let toggle = control(
        &mut app,
        Kind::Request(destination(Some(Category::Gold), true)),
    )?;
    app.world_mut().trigger(Activate { entity: toggle });
    let _ = request(&mut commands)?;
    config_response(&outbox, CommandStatus::Rejected)?;
    app.update();
    assert_eq!(panel(&app)?.destinations, vec![assignment]);
    Ok(())
}
#[test]
fn stale_snapshot_higher_modal_and_session_end_cannot_dispatch_configuration() -> TestResult {
    let (mut app, mut commands, outbox) = fixture(true)?;
    open(&mut app)?;
    let stale = control(&mut app, Kind::Request(Action::LootAll))?;
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .quick_loot_action(Action::Select(Mode::Skip))
    );
    let _ = request(&mut commands)?;
    config_response(&outbox, CommandStatus::Accepted)?;
    // Session receipt changes the canonical snapshot before Bevy redraws.
    app.world_mut().trigger(Activate { entity: stale });
    assert!(commands.try_recv().is_err());
    app.update();
    let loot = control(&mut app, Kind::Request(Action::LootAll))?;
    let higher = app
        .world_mut()
        .spawn((GameModal, GlobalZIndex(220), Node::default()))
        .id();
    app.world_mut().trigger(Activate { entity: loot });
    assert!(commands.try_recv().is_err());
    app.world_mut().entity_mut(higher).despawn();
    app.world_mut().non_send_mut::<HostOwner>().close();
    app.world_mut().trigger(Activate { entity: loot });
    app.update();
    assert!(commands.try_recv().is_err());
    assert_eq!(
        app.world_mut()
            .query::<&Overlay>()
            .iter(app.world())
            .count(),
        0
    );
    Ok(())
}
#[test]
fn unsupported_session_does_not_queue_and_close_preserves_inflight_request() -> TestResult {
    let (mut app, mut commands, _) = fixture(false)?;
    open(&mut app)?;
    let loot = control(&mut app, Kind::Request(Action::LootAll))?;
    app.world_mut().trigger(Activate { entity: loot });
    assert!(commands.try_recv().is_err());
    let close = control(&mut app, Kind::Close)?;
    app.world_mut().trigger(Activate { entity: close });
    assert!(!panel(&app)?.open);
    let (mut app, mut commands, outbox) = fixture(true)?;
    open(&mut app)?;
    let loot = control(&mut app, Kind::Request(Action::LootAll))?;
    app.world_mut().trigger(Activate { entity: loot });
    let _ = request(&mut commands)?;
    app.update();
    let close = control(&mut app, Kind::Close)?;
    app.world_mut().trigger(Activate { entity: close });
    assert!(!panel(&app)?.open);
    assert!(panel(&app)?.pending.is_some());
    response(&outbox, CommandStatus::Accepted, true)?;
    app.update();
    assert!(!panel(&app)?.open);
    Ok(())
}

#[test]
fn fallback_keeps_the_exact_confirmed_recipient_without_exposing_its_identity() -> TestResult {
    let (mut app, mut commands, outbox) = fixture(true)?;
    open(&mut app)?;
    let assignment = Action::Destination {
        category: Some(Category::CreatureProduct),
        recipient: Recipient::Container,
        id: Some([9; 16]),
        fallback: false,
    };
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .quick_loot_action(assignment)
    );
    let _ = request(&mut commands)?;
    config_response(&outbox, CommandStatus::Accepted)?;
    app.update();
    let toggle = control(
        &mut app,
        Kind::Request(Action::Destination {
            category: Some(Category::CreatureProduct),
            recipient: Recipient::Container,
            id: Some([9; 16]),
            fallback: true,
        }),
    )?;
    app.world_mut().trigger(Activate { entity: toggle });
    let wire = request(&mut commands)?;
    assert_eq!(wire.recipient, Some(Recipient::Container));
    assert_eq!(wire.recipient_id, Some([9; 16]));
    assert!(wire.fallback_to_main);
    assert!(
        !app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0.contains("09090909"))
    );
    Ok(())
}

#[test]
fn mode_and_clear_buttons_confirm_through_existing_typed_requests() -> TestResult {
    let (mut app, mut commands, outbox) = fixture(true)?;
    open(&mut app)?;
    let select = control(&mut app, Kind::Request(Action::Select(Mode::Accept)))?;
    app.world_mut().trigger(Activate { entity: select });
    let wire = request(&mut commands)?;
    assert_eq!(
        (wire.action, wire.filter_mode),
        (QuickLootAction::FilterSelect, Some(Mode::Accept))
    );
    assert!(panel(&app)?.confirmed.is_empty());
    config_response(&outbox, CommandStatus::Accepted)?;
    app.update();
    assert_eq!(panel(&app)?.confirmed, vec![Action::Select(Mode::Accept)]);
    let clear = control(&mut app, Kind::Request(Action::Clear(Mode::Accept)))?;
    app.world_mut().trigger(Activate { entity: clear });
    let wire = request(&mut commands)?;
    assert_eq!(
        (wire.action, wire.filter_mode),
        (QuickLootAction::FilterClear, Some(Mode::Accept))
    );
    config_response(&outbox, CommandStatus::Accepted)?;
    app.update();
    assert_eq!(
        panel(&app)?.confirmed,
        vec![Action::Select(Mode::Accept), Action::Clear(Mode::Accept)]
    );
    Ok(())
}

#[test]
fn previous_session_callback_cannot_loot_in_a_replacement_play_owner() -> TestResult {
    let (mut app, _commands, previous_outbox) = fixture(true)?;
    open(&mut app)?;
    let old = control(&mut app, Kind::Request(Action::LootAll))?;
    let (link, mut replacement_commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_quick_loot_selected(true);
    link.discovery_open(98);
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    previous_outbox.bevy_quick_loot_end();
    {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        let previous = owner.generation();
        owner.poll([960, 640]);
        assert!(owner.generation() > previous);
        owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
        owner.phase = crate::bevy_shell::owner::Phase::Playing;
    }
    // The new canonical panel is otherwise identical; only the admission generation differs.
    app.world_mut().trigger(Activate { entity: old });
    assert!(replacement_commands.try_recv().is_err());
    app.update();
    let current = control(&mut app, Kind::Request(Action::LootAll))?;
    app.world_mut().trigger(Activate { entity: current });
    assert_eq!(
        request(&mut replacement_commands)?.action,
        QuickLootAction::LootAll
    );
    Ok(())
}

#[test]
fn source_return_does_nothing_and_escape_closes_only_the_top_manage_dialog() -> TestResult {
    use bevy::input::keyboard::{Key, KeyboardInput};
    let (mut app, mut commands, _outbox) = fixture(true)?;
    open(&mut app)?;
    let key = |key_code, logical_key| KeyboardInput {
        key_code,
        logical_key,
        state: bevy::input::ButtonState::Pressed,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    };
    app.world_mut()
        .write_message(key(KeyCode::Enter, Key::Enter));
    app.update();
    assert!(panel(&app)?.open);
    let higher = app
        .world_mut()
        .spawn((GameModal, GlobalZIndex(220), Node::default()))
        .id();
    app.world_mut()
        .write_message(key(KeyCode::Escape, Key::Escape));
    app.update();
    assert!(panel(&app)?.open);
    app.world_mut().entity_mut(higher).despawn();
    // Complete the higher modal's close frame; its snapshot intentionally fences
    // the same frame so one Escape cannot also close this underlying dialog.
    app.update();
    assert!(panel(&app)?.open);
    app.world_mut()
        .write_message(key(KeyCode::Escape, Key::Escape));
    app.update();
    assert!(!panel(&app)?.open);
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn quick_loot_can_add_carried_item_and_remove_confirmed_filter_entry() -> TestResult {
    let (mut app, mut commands, outbox) = fixture(true)?;
    outbox.push(vec![oteryn_session::SessionEvent::Inventory(
        oteryn_session::AppliedDelta {
            server_sequence: 1,
            base_revision: 0,
            new_revision: 1,
            value: oteryn_session::CharacterInventory {
                main_backpack: None,
                equipment: vec![],
                entries: vec![oteryn_session::ItemEntry {
                    appearance_id: 0,
                    handle: 7.try_into()?,
                    item_definition_ref: 42.try_into()?,
                    count: 1.try_into()?,
                    sub_type: 0,
                }],
            },
        },
    )]);
    super::super::install::install_scroll_memory(&mut app);
    open(&mut app)?;
    let add = named(&mut app, "quick_loot.add.accept")?;
    app.world_mut().trigger(Activate { entity: add });
    app.update();
    app.world_mut().trigger(Activate { entity: add });
    app.update();
    assert_eq!(
        app.world_mut()
            .query::<&Name>()
            .iter(app.world())
            .filter(|name| name.as_str() == "quick_loot.pick.7")
            .count(),
        1
    );
    let pick = named(&mut app, "quick_loot.pick.7")?;
    let column = app
        .world()
        .get::<ChildOf>(pick)
        .ok_or("choices column")?
        .parent();
    assert_eq!(
        app.world().get::<ChildOf>(column),
        app.world().get::<ChildOf>(add)
    );
    app.world_mut().trigger(Activate { entity: pick });
    let wire = request(&mut commands)?;
    assert_eq!(wire.action, QuickLootAction::FilterAdd);
    assert_eq!(wire.filter_definition_ref, Some(42));
    assert_eq!(wire.filter_mode, Some(Mode::Accept));
    response(&outbox, CommandStatus::Accepted, true)?;
    app.update();
    assert!(panel(&app)?.accepted.contains(&42));
    let remove = named(&mut app, "quick_loot.remove.accept.42")?;
    let filters = |app: &mut App| -> Result<Entity, Box<dyn std::error::Error>> {
        let title = app
            .world_mut()
            .query::<(Entity, &Text)>()
            .iter(app.world())
            .find_map(|(e, t)| (t.0 == "QuickLoot filter").then_some(e))
            .ok_or("filter title")?;
        Ok(app
            .world()
            .get::<ChildOf>(title)
            .ok_or("filter pane")?
            .parent())
    };
    let viewport = filters(&mut app)?;
    app.world_mut()
        .entity_mut(viewport)
        .insert(ScrollPosition(Vec2::new(0., 120.)));
    app.world_mut().trigger(Activate { entity: remove });
    assert_eq!(
        request(&mut commands)?.action,
        QuickLootAction::FilterRemove
    );
    app.update();
    let pending = filters(&mut app)?;
    assert_eq!(
        app.world().get::<ScrollPosition>(pending).map(|p| p.0.y),
        Some(120.)
    );
    config_response(&outbox, CommandStatus::Accepted)?;
    app.update();
    assert!(!panel(&app)?.accepted.contains(&42));
    let confirmed = filters(&mut app)?;
    assert_eq!(
        app.world().get::<ScrollPosition>(confirmed).map(|p| p.0.y),
        Some(120.)
    );
    Ok(())
}

#[test]
fn confirmed_filter_mutations_update_the_displayed_mode() -> TestResult {
    let (mut app, mut commands, outbox) = fixture(true)?;
    open(&mut app)?;
    let has_label = |app: &mut App, label: &str| {
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == label)
    };
    for (action, label) in [
        (Action::Select(Mode::Skip), "Confirmed: skip listed items"),
        (
            Action::Add {
                mode: Mode::Accept,
                definition: 42,
            },
            "Confirmed: only listed items",
        ),
        (
            Action::Remove {
                mode: Mode::Skip,
                definition: 42,
            },
            "Confirmed: skip listed items",
        ),
        (Action::Clear(Mode::Accept), "Confirmed: only listed items"),
    ] {
        let link = &app
            .world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link;
        assert!(link.quick_loot_action(action));
        let _ = request(&mut commands)?;
        config_response(&outbox, CommandStatus::Accepted)?;
        app.update();
        assert!(has_label(&mut app, label));
    }
    Ok(())
}

#[test]
fn loot_navigation_opens_catalogue_and_store_from_panel() -> TestResult {
    for (label, catalogue) in [("Item Catalogue", true), ("Get Premium", false)] {
        let (mut app, _commands, _) = fixture(true)?;
        open(&mut app)?;
        let entity = app
            .world_mut()
            .query::<(&Text, &ChildOf)>()
            .iter(app.world())
            .find_map(|(text, parent)| (text.0 == label).then_some(parent.parent()))
            .ok_or(label)?;
        app.world_mut().trigger(Activate { entity });
        let link = &app
            .world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link;
        assert!(!link.quick_loot_panel().open);
        if catalogue {
            assert!(link.discovery_panel().inspection.catalogue_open);
        } else {
            assert!(link.store_panel().open);
        }
    }
    Ok(())
}
