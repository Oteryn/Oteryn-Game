use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{PlayView, play_channel},
};
use oteryn_client_ui::training::Intent;
use oteryn_renderer::TileCoord;
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
fn fixture() -> std::result::Result<
    (
        HostOwner,
        tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
        crate::play::Outbox,
    ),
    Box<dyn std::error::Error>,
> {
    let mut owner = HostOwner::new(None)?;
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    let mut view = PlayView::new(TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
    owner.phase = crate::bevy_shell::owner::Phase::Playing;
    outbox.training_test_selected(true);
    Ok((owner, commands, outbox))
}
#[test]
fn training_window_shows_server_data_only() -> Result {
    let (owner, mut commands, outbox) = fixture()?;
    open(&owner);
    let link = &owner.play.as_ref().ok_or("play")?.link;
    assert_eq!(link.training_panel().charges, None);
    let expected = link.training_binding();
    assert!(link.training_dispatch_bound(
        &expected,
        ["7".into(), "9".into(), "11".into()],
        Some(Intent::Status),
        false
    ));
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("feature".into());
    };
    assert_eq!(link.training_panel().charges, None);
    outbox.training_test_reply(&command, 1, 87, 5432)?;
    let panel = link.training_panel();
    assert_eq!(panel.charges, Some(87));
    assert_eq!(panel.pool_seconds, Some(5432));
    let mut app = test_app(owner);
    app.update();
    let text = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|text| text.0.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(text.contains(&oteryn_client_ui::training::panel_text(&panel)));
    let fields = app
        .world_mut()
        .query::<&bevy::text::EditableText>()
        .iter(app.world())
        .map(|text| text.value().to_string())
        .collect::<Vec<_>>();
    assert_eq!(fields, panel.fields.to_vec());
    Ok(())
}
#[test]
fn training_actions_are_bound_and_confirmed() -> Result {
    let (owner, mut commands, outbox) = fixture()?;
    open(&owner);
    let link = &owner.play.as_ref().ok_or("play")?.link;
    let fields = ["7".into(), "9".into(), "11".into()];
    for intent in [Intent::Offline, Intent::Start, Intent::Status, Intent::Stop] {
        let expected = link.training_binding();
        let before = link.training_panel();
        if intent != Intent::Status {
            assert!(!link.training_dispatch_bound(&expected, fields.clone(), Some(intent), false));
            assert_eq!(link.training_panel(), before);
            assert!(commands.try_recv().is_err());
        }
        assert!(link.training_dispatch_bound(&expected, fields.clone(), Some(intent), true));
        assert_eq!(link.training_panel().charges, before.charges);
        assert_eq!(link.training_panel().pool_seconds, before.pool_seconds);
        assert!(!link.training_dispatch_bound(&expected, fields.clone(), Some(intent), true));
        let PlayCommand::Feature(command) = commands.try_recv()? else {
            return Err("feature".into());
        };
        assert_eq!(command.capability, 54);
        let (kind, payload) = match intent {
            Intent::Offline => (66, vec![8, 7]),
            Intent::Start => (67, vec![8, 1, 16, 9, 24, 11]),
            Intent::Status => (67, vec![8, 2]),
            Intent::Stop => (67, vec![8, 3]),
        };
        assert_eq!(command.kind, kind);
        assert_eq!(command.payload, payload);
        assert!(commands.try_recv().is_err());
        outbox.training_test_reply(&command, 2, 50, 5000)?;
    }
    confirmation_controls()?;
    Ok(())
}
#[test]
fn training_rejected_when_session_replaced_or_target_changed() -> Result {
    let (owner, mut commands, _) = fixture()?;
    open(&owner);
    let link = &owner.play.as_ref().ok_or("play")?.link;
    let expected = link.training_binding();
    let fields = ["7".into(), "9".into(), "11".into()];
    link.training_edit(1, "99".into());
    let before = link.training_panel();
    assert!(!link.training_dispatch_bound(&expected, fields.clone(), Some(Intent::Start), true));
    assert_eq!(before, link.training_panel());
    let (other, mut other_commands, _) = fixture()?;
    open(&other);
    let other_link = &other.play.as_ref().ok_or("play")?.link;
    assert!(!other_link.training_dispatch_bound(
        &expected,
        fields.clone(),
        Some(Intent::Start),
        true
    ));
    assert!(other_commands.try_recv().is_err());
    let expected = link.training_binding();
    assert!(link.training_dispatch_bound(&expected, fields.clone(), None, false));
    assert!(!link.training_dispatch_bound(&expected, fields.clone(), Some(Intent::Start), true));
    assert!(commands.try_recv().is_err());
    drop(commands);
    link.training_open();
    let expected = link.training_binding();
    let before = link.training_panel();
    assert!(!link.training_dispatch_bound(&expected, fields, Some(Intent::Start), true));
    assert_eq!(before, link.training_panel());
    rejected_controls()?;
    Ok(())
}
#[test]
fn training_window_registered() -> Result {
    let source = include_str!("../mod.rs");
    assert!(source.contains("training::install(app)"));
    let toolbar = include_str!("../../ui.rs");
    assert!(toolbar.contains("Control::Training"));
    let (owner, _commands, _outbox) = fixture()?;
    let mut app = test_app(owner);
    app.update();
    let toolbar = app
        .world_mut()
        .query::<(Entity, &crate::bevy_shell::ui::Control)>()
        .iter(app.world())
        .find(|(_, control)| matches!(control, crate::bevy_shell::ui::Control::Training))
        .map(|(entity, _)| entity)
        .ok_or("toolbar")?;
    app.world_mut().trigger(Activate { entity: toolbar });
    app.update();
    assert_eq!(
        app.world_mut()
            .query::<&Name>()
            .iter(app.world())
            .filter(|name| name.as_str() == "training.window")
            .count(),
        1
    );
    let close = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find(|(_, control)| matches!(control.kind, Kind::Close))
        .map(|(entity, _)| entity)
        .ok_or("close")?;
    app.world_mut()
        .trigger(bevy::ui_widgets::Activate { entity: close });
    app.update();
    app.update();
    assert!(
        !app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .training_panel()
            .open
    );
    shortcut(&mut app)?;
    app.update();
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .training_panel()
            .open
    );
    shortcut(&mut app)?;
    app.update();
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .training_panel()
            .open
    );
    app.world_mut()
        .write_message(bevy::input::keyboard::KeyboardInput {
            key_code: KeyCode::Escape,
            logical_key: bevy::input::keyboard::Key::Escape,
            state: bevy::input::ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
    app.update();
    assert!(
        !app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .training_panel()
            .open
    );
    app.world_mut().non_send_mut::<HostOwner>().play = None;
    app.update();
    assert_eq!(
        app.world()
            .get::<Node>(toolbar)
            .ok_or("toolbar node")?
            .display,
        Display::None
    );
    Ok(())
}
fn test_app(owner: HostOwner) -> App {
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::asset::AssetPlugin::default(),
        oteryn_client_ui_bevy::GlobalUiPlugin,
    ));
    app.init_resource::<ButtonInput<KeyCode>>();
    app.insert_non_send(owner);
    app.init_asset::<Image>()
        .init_asset::<Font>()
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    app.add_systems(Update, crate::bevy_shell::ui::project_toolbar);
    super::super::modal::install(&mut app);
    install(&mut app);
    app
}

fn control(app: &mut App, kind: Kind) -> std::result::Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find(|(_, control)| control.kind == kind)
        .map(|(entity, _)| entity)
        .ok_or_else(|| "training control".into())
}
fn click(app: &mut App, kind: Kind) -> Result {
    let entity = control(app, kind)?;
    app.world_mut().trigger(Activate { entity });
    app.update();
    app.update();
    Ok(())
}
fn confirmation_controls() -> Result {
    let (owner, mut commands, outbox) = fixture()?;
    let link = &owner.play.as_ref().ok_or("play")?.link;
    for (index, value) in ["7", "9", "11"].into_iter().enumerate() {
        link.training_edit(index, value.into());
    }
    open(&owner);
    let mut app = test_app(owner);
    app.update();
    for intent in [Intent::Offline, Intent::Start, Intent::Stop] {
        click(&mut app, Kind::Request(intent))?;
        assert!(commands.try_recv().is_err());
        assert!(app.world().resource::<State>().confirmation.is_some());
        click(&mut app, Kind::Cancel)?;
        assert!(commands.try_recv().is_err());
        click(&mut app, Kind::Request(intent))?;
        click(&mut app, Kind::Confirm)?;
        let PlayCommand::Feature(command) = commands.try_recv()? else {
            return Err("feature".into());
        };
        assert!(commands.try_recv().is_err());
        outbox.training_test_reply(&command, 2, 30, 4000)?;
        app.update();
    }
    click(&mut app, Kind::Request(Intent::Status))?;
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("status".into());
    };
    assert_eq!(command.payload, [8, 2]);
    assert!(commands.try_recv().is_err());
    Ok(())
}
fn shortcut(app: &mut App) -> Result {
    use oteryn_input_actions::{Modifier, Modifiers};
    crate::bevy_shell::controls::tests::shortcut(
        app,
        17,
        Modifiers::one(Modifier::Control).with(Modifier::Alt),
    )?;
    Ok(())
}

fn rejected_controls() -> Result {
    let (owner, mut commands, _) = fixture()?;
    let link = &owner.play.as_ref().ok_or("play")?.link;
    for (index, value) in ["7", "9", "11"].into_iter().enumerate() {
        link.training_edit(index, value.into());
    }
    open(&owner);
    let mut app = test_app(owner);
    app.update();
    click(&mut app, Kind::Request(Intent::Start))?;
    let confirm = control(&mut app, Kind::Confirm)?;
    let before = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .training_panel();
    let modal = app
        .world_mut()
        .spawn((GameModal, GlobalZIndex(220), Node::default()))
        .id();
    app.world_mut().trigger(Activate { entity: confirm });
    app.update();
    assert!(commands.try_recv().is_err());
    assert_eq!(
        before,
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .training_panel()
    );
    app.world_mut().entity_mut(modal).despawn();
    app.update();
    app.world_mut().non_send_mut::<HostOwner>().open_graphics();
    app.world_mut().trigger(Activate { entity: confirm });
    app.update();
    assert!(commands.try_recv().is_err());
    app.world_mut().non_send_mut::<HostOwner>().cancel();
    app.update();
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .training_edit(1, "99".into());
    app.world_mut().trigger(Activate { entity: confirm });
    app.update();
    assert!(commands.try_recv().is_err());
    assert!(app.world().resource::<State>().confirmation.is_none());
    Ok(())
}
