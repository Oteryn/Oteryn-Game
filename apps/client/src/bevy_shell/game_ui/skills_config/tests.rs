use super::*;
#[test]
fn skills_config_skills_window_registered() {
    let registrations = include_str!("../mod.rs");
    let toolbar = include_str!("../../ui.rs");
    assert!(registrations.contains("skills_config::install(app)"));
    assert!(toolbar.contains("(Control::SkillsConfig, \"Skills\", 100.)"));
    assert!(toolbar.contains("skills_config::open(&owner)"));
    assert_eq!(
        crate::bevy_shell::ui::Control::SkillsConfig.id(),
        "shell.skills_config"
    );
}

use crate::combat_input::PlayCommand;
use crate::play::{Outbox, PlayView, play_channel};
use bevy::{asset::AssetPlugin, input_focus::InputFocus};
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    Outbox,
);
fn fixture() -> Result<Fixture, Box<dyn std::error::Error>> {
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
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
#[test]
fn skills_config_skills_window_opens_closes_and_rejects_modal()
-> Result<(), Box<dyn std::error::Error>> {
    let (mut app, _commands, _outbox) = fixture()?;
    open(app.world().non_send::<HostOwner>());
    app.update();
    assert!(app.world().resource::<State>().widgets.is_some());
    let (key, panel) = app
        .world()
        .resource::<State>()
        .snapshot
        .clone()
        .ok_or("snapshot")?;
    assert!(panel.widget.rows.is_empty());
    let close = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| (control.kind == Kind::Close).then_some(entity))
        .ok_or("close")?;
    let modal = app
        .world_mut()
        .spawn((GameModal, GlobalZIndex(999), Node::default()))
        .id();
    app.world_mut().trigger(Activate { entity: close });
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .skills_config_snapshot()
            .open
    );
    app.world_mut().entity_mut(modal).despawn();
    app.update();
    app.world_mut().trigger(Activate { entity: close });
    app.update();
    assert!(app.world().resource::<State>().widgets.is_none());
    // A queued control from the closed window cannot mutate the retained layout.
    perform(
        app.world().non_send::<HostOwner>(),
        app.world().resource::<State>(),
        &Control {
            key,
            kind: Kind::Progress(false),
        },
    );
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .skills_config_snapshot()
            .layout
            .preferences
            .progress
    );
    Ok(())
}

#[test]
fn skills_config_skills_ctrl_k_and_visible_refresh() -> Result<(), Box<dyn std::error::Error>> {
    let (mut app, mut commands, outbox) = fixture()?;
    outbox.skills_config_test_progress();
    let event = oteryn_input_actions::NormalizedInputEvent::Key {
        code: oteryn_input_actions::KeyCode::new(14)?,
        state: oteryn_input_actions::ButtonState::Pressed,
        modifiers: oteryn_input_actions::Modifiers::one(oteryn_input_actions::Modifier::Control),
        repeat: false,
    };
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(&[event], false);
    app.update();
    assert!(
        app.world().resource::<State>().widgets.is_some(),
        "Ctrl+K must open skills"
    );
    assert!(matches!(commands.try_recv()?, PlayCommand::Feature(_)));
    Ok(())
}
#[test]
fn skills_config_skills_visible_refresh() -> Result<(), Box<dyn std::error::Error>> {
    let (mut app, mut commands, outbox) = fixture()?;
    outbox.skills_config_test_progress();
    open(app.world().non_send::<HostOwner>());
    app.update();
    assert!(matches!(commands.try_recv()?, PlayCommand::Feature(_)));
    app.insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
        std::time::Duration::from_secs(1),
    ));
    app.update();
    assert!(
        commands.try_recv().is_err(),
        "in-flight query must not repeat"
    );
    outbox.skills_config_test_progress();
    app.update();
    assert!(
        matches!(commands.try_recv()?, PlayCommand::Feature(_)),
        "visible window refreshes each second"
    );
    let hidden = app
        .world_mut()
        .spawn(Window {
            visible: false,
            ..default()
        })
        .id();
    outbox.skills_config_test_progress();
    app.update();
    assert!(
        commands.try_recv().is_err(),
        "hidden window must not refresh"
    );
    app.world_mut().entity_mut(hidden).despawn();
    app.world_mut().non_send_mut::<HostOwner>().phase = crate::bevy_shell::owner::Phase::Offline;
    app.update();
    assert!(
        commands.try_recv().is_err(),
        "non-playing session must not refresh"
    );
    app.world_mut().non_send_mut::<HostOwner>().phase = crate::bevy_shell::owner::Phase::Playing;
    open(app.world().non_send::<HostOwner>());
    app.update();
    outbox.skills_config_test_progress();
    app.update();
    assert!(
        commands.try_recv().is_err(),
        "closed window must not refresh"
    );
    Ok(())
}
