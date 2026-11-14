use super::*;
#[test]
fn skills_config_skills_window_registered() {
    let registrations = include_str!("../mod.rs");
    let toolbar = include_str!("../../ui.rs");
    assert!(registrations.contains("skills_config::install(app)"));
    assert!(toolbar.contains("(Control::SkillsConfig, \"Skills (F24)\", 100.)"));
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
fn fixture(selected: bool) -> Result<Fixture, Box<dyn std::error::Error>> {
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    let _ = selected;
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
    let (mut app, _commands, _outbox) = fixture(false)?;
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
