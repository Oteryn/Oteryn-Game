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
fn fixture(selected: bool) -> Result<Fixture, Box<dyn std::error::Error>> {
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
fn open(app: &mut App) -> TestResult {
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
fn control(app: &mut App, kind: Kind) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| (control.kind == kind).then_some(entity))
        .ok_or("quick loot control absent".into())
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
