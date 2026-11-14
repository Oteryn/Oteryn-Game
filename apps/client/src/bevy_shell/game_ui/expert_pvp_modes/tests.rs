use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use bevy::asset::AssetPlugin;
use oteryn_client_runtime::pvp_expert as wire;
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    Outbox,
);
fn fixture() -> std::result::Result<Fixture, Box<dyn std::error::Error>> {
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.expert_pvp_modes_test_state(Panel {
        selected: true,
        open: true,
        ..Default::default()
    });
    outbox.expert_pvp_modes_test_push(wire::ExpertFightModes::default());
    let mut owner = HostOwner::new(None)?;
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
    owner.phase = crate::bevy_shell::owner::Phase::Playing;
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Font>()
        .init_asset::<Image>()
        .init_resource::<bevy::input_focus::InputFocus>()
        .init_resource::<ButtonInput<KeyCode>>()
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    Ok((app, commands, outbox))
}
fn panel(app: &App) -> std::result::Result<Panel, Box<dyn std::error::Error>> {
    Ok(app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .pvp_expert_panel())
}
fn click(app: &mut App, kind: Kind) -> Result {
    let entity = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| (control.kind == kind).then_some(entity))
        .ok_or("control")?;
    app.world_mut().trigger(Activate { entity });
    Ok(())
}
#[test]
fn expert_pvp_modes_window_shows_server_data_only() -> Result {
    let (mut app, mut commands, outbox) = fixture()?;
    let initial = panel(&app)?;
    assert_eq!(initial.confirmed, Some(wire::ExpertFightModes::default()));
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == oteryn_client_ui::pvp_expert::panel_text(&initial))
    );
    let modes = wire::ExpertFightModes {
        expert: wire::ExpertMode::YellowHand,
        ..wire::ExpertFightModes::default()
    };
    outbox.expert_pvp_modes_test_push(modes);
    app.update();
    assert_eq!(panel(&app)?.confirmed, Some(modes));
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0.contains("YellowHand"))
    );
    outbox.expert_pvp_modes_test_state(Panel {
        open: true,
        ..Default::default()
    });
    toggle(app.world().non_send::<HostOwner>());
    assert!(!panel(&app)?.open);
    toggle(app.world().non_send::<HostOwner>());
    assert!(panel(&app)?.open);
    assert!(commands.try_recv().is_err());
    app.update();
    assert!(panel(&app)?.confirmed.is_none());
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0.contains("Unavailable in this session"))
    );
    Ok(())
}
#[test]
fn expert_pvp_modes_actions_are_bound_and_confirmed() -> Result {
    let (mut app, mut commands, outbox) = fixture()?;
    let initial = panel(&app)?.confirmed;
    for (field, value) in [(0, 1), (1, 2), (2, 0), (3, 3)] {
        click(&mut app, Kind::Field(field, value))?;
        app.update();
    }
    click(&mut app, Kind::Apply)?;
    assert!(commands.try_recv().is_err());
    assert_eq!(panel(&app)?.confirmed, initial);
    app.update();
    click(&mut app, Kind::Confirm)?;
    click(&mut app, Kind::Confirm)?;
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("feature command".into());
    };
    assert_eq!((command.capability, command.kind), (87, 103));
    let requested =
        wire::decode_expert_fight_modes(&command.payload).map_err(|e| format!("{e:?}"))?;
    assert_eq!(requested.expert, wire::ExpertMode::RedFist);
    assert_eq!(
        requested.modes.fight_mode,
        wire::attack::FightMode::Offensive
    );
    assert_eq!(requested.modes.chase, wire::attack::ChaseMode::Chase);
    assert!(!requested.modes.secure);
    assert!(commands.try_recv().is_err());
    assert!(panel(&app)?.pending);
    assert_eq!(panel(&app)?.confirmed, initial);
    outbox.expert_pvp_modes_test_reply(&command, oteryn_session::CommandStatus::Rejected, vec![]);
    app.update();
    assert!(!panel(&app)?.pending);
    assert_eq!(panel(&app)?.confirmed, initial);
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0.contains("refused"))
    );
    click(&mut app, Kind::Apply)?;
    app.update();
    click(&mut app, Kind::Confirm)?;
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("feature command".into());
    };
    outbox.expert_pvp_modes_test_reply(
        &command,
        oteryn_session::CommandStatus::Accepted,
        wire::attack::encode_attack_intent_result(wire::attack::AttackIntentDisposition::Ok),
    );
    assert_eq!(panel(&app)?.confirmed, initial);
    outbox.expert_pvp_modes_test_push(requested);
    assert_eq!(panel(&app)?.confirmed, Some(requested));
    Ok(())
}
#[test]
fn expert_pvp_modes_rejected_when_session_replaced_or_target_changed() -> Result {
    let (mut app, mut commands, outbox) = fixture()?;
    click(&mut app, Kind::Apply)?;
    app.update();
    let changed = wire::ExpertFightModes {
        expert: wire::ExpertMode::WhiteHand,
        ..wire::ExpertFightModes::default()
    };
    outbox.expert_pvp_modes_test_push(changed);
    click(&mut app, Kind::Confirm)?;
    assert!(commands.try_recv().is_err());
    app.update();
    click(&mut app, Kind::Apply)?;
    app.update();
    app.world_mut().non_send_mut::<HostOwner>().open_graphics();
    let before = panel(&app)?;
    click(&mut app, Kind::Confirm)?;
    assert_eq!(panel(&app)?, before);
    assert!(commands.try_recv().is_err());
    app.world_mut().non_send_mut::<HostOwner>().cancel();
    let blocker = app
        .world_mut()
        .spawn((super::super::GameModal, Node::default(), GlobalZIndex(999)))
        .id();
    click(&mut app, Kind::Confirm)?;
    assert_eq!(panel(&app)?, before);
    assert!(commands.try_recv().is_err());
    app.world_mut().despawn(blocker);
    app.update();
    // A control from one connection cannot target an identical panel on another connection.
    let old = app
        .world()
        .resource::<State>()
        .snapshot
        .as_ref()
        .ok_or("snapshot")?
        .1
        .clone();
    let (replacement, mut incoming, replacement_outbox) = play_channel();
    replacement_outbox.expert_pvp_modes_test_state(panel(&app)?);
    assert!(!replacement.expert_pvp_modes_dispatch_bound(&old, Action::Apply));
    assert!(incoming.try_recv().is_err());
    click(&mut app, Kind::Close)?;
    click(&mut app, Kind::Confirm)?;
    assert!(commands.try_recv().is_err());
    assert!(!panel(&app)?.open);
    Ok(())
}
#[test]
fn expert_pvp_modes_window_registered() -> Result {
    let (mut app, mut commands, _) = fixture()?;
    click(&mut app, Kind::Close)?;
    app.update();
    let entity = app
        .world_mut()
        .query::<(Entity, &crate::bevy_shell::ui::Control)>()
        .iter(app.world())
        .find_map(|(e, c)| matches!(c, crate::bevy_shell::ui::Control::ExpertPvpModes).then_some(e))
        .ok_or("toolbar entry")?;
    app.world_mut().trigger(Activate { entity });
    app.update();
    assert!(panel(&app)?.open);
    app.world_mut().trigger(Activate { entity });
    app.update();
    assert!(!panel(&app)?.open);
    // Exercise the installed keyboard system using its actual unique chord.
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ControlLeft);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::AltLeft);
    app.world_mut()
        .write_message(bevy::input::keyboard::KeyboardInput {
            key_code: KeyCode::F8,
            logical_key: bevy::input::keyboard::Key::Unidentified(
                bevy::input::keyboard::NativeKey::Unidentified,
            ),
            state: bevy::input::ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
    app.update();
    assert!(panel(&app)?.open);
    click(&mut app, Kind::Close)?;
    assert!(!panel(&app)?.open);
    assert!(commands.try_recv().is_err());
    let registrations = include_str!("../mod.rs");
    assert!(registrations.contains("expert_pvp_modes::install(app)"));
    Ok(())
}
