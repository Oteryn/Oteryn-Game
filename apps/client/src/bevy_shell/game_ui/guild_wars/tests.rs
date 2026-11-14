use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use bevy::{asset::AssetPlugin, input_focus::InputFocus};
type Error = Box<dyn std::error::Error>;
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    Outbox,
);
fn fixture() -> Result<Fixture, Error> {
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.guild_wars_test_selected();
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
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    open(app.world().non_send::<HostOwner>());
    app.update();
    Ok((app, commands, outbox))
}
fn panel(app: &App) -> Result<Panel, Error> {
    Ok(app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .guild_war_panel())
}
fn click(app: &mut App, kind: Kind) -> Result<(), Error> {
    let entity = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| (control.kind == kind).then_some(entity))
        .ok_or("control")?;
    app.world_mut().trigger(Activate { entity });
    Ok(())
}
fn draft(app: &mut App) {
    for (field, mut text) in app
        .world_mut()
        .query::<(&Field, &mut EditableText)>()
        .iter_mut(app.world_mut())
    {
        text.editor
            .set_text(["01010101010170018001010101010101", "100", "7"][field.0]);
    }
}
#[test]
fn guild_wars_window_shows_server_data_only() -> Result<(), Error> {
    let (mut app, mut commands, outbox) = fixture()?;
    assert!(panel(&app)?.summary.is_empty());
    assert_eq!(
        app.world()
            .resource::<State>()
            .snapshot
            .as_ref()
            .ok_or("snapshot")?
            .1,
        panel(&app)?
    );
    draft(&mut app);
    click(&mut app, Kind::Propose(Intent::Declare))?;
    app.update();
    click(&mut app, Kind::Confirm)?;
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("command".into());
    };
    assert!(panel(&app)?.summary.is_empty());
    outbox.guild_wars_test_reply(&command);
    app.update();
    let expected = oteryn_client_ui::guild_war::panel_text(&panel(&app)?);
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == expected)
    );
    assert!(
        panel(&app)?
            .summary
            .contains("01010101010170018001010101010101")
    );
    Ok(())
}
#[test]
fn guild_wars_actions_are_bound_and_confirmed() -> Result<(), Error> {
    for intent in [Intent::Declare, Intent::Accept] {
        let (mut app, mut commands, outbox) = fixture()?;
        draft(&mut app);
        click(&mut app, Kind::Propose(intent))?;
        assert!(commands.try_recv().is_err());
        app.update();
        click(&mut app, Kind::Confirm)?;
        click(&mut app, Kind::Confirm)?;
        let PlayCommand::Feature(command) = commands.try_recv()? else {
            return Err("command".into());
        };
        assert_eq!((command.capability, command.kind), (103, 119));
        let decoded = oteryn_client_runtime::guild_war::decode_guild_war_intent(&command.payload)
            .map_err(|e| format!("{e:?}"))?;
        assert!(matches!(
            (intent, decoded),
            (
                Intent::Declare,
                oteryn_client_runtime::guild_war::GuildWarIntent::Declare {
                    frag_limit: 100,
                    duration_days: 7,
                    ..
                }
            ) | (
                Intent::Accept,
                oteryn_client_runtime::guild_war::GuildWarIntent::Accept { .. }
            )
        ));
        assert!(commands.try_recv().is_err());
        assert!(panel(&app)?.pending && panel(&app)?.summary.is_empty());
        outbox.guild_wars_test_reply(&command);
        assert!(!panel(&app)?.pending);
    }
    Ok(())
}
#[test]
fn guild_wars_rejected_when_session_replaced_or_target_changed() -> Result<(), Error> {
    let (mut app, mut commands, _) = fixture()?;
    draft(&mut app);
    click(&mut app, Kind::Propose(Intent::Declare))?;
    app.update();
    let before = panel(&app)?;
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .guild_war_edit(0, "changed".into());
    click(&mut app, Kind::Confirm)?;
    assert!(commands.try_recv().is_err());
    assert!(!panel(&app)?.pending);
    assert_eq!(panel(&app)?.summary, before.summary);
    let (mut app, mut commands, _) = fixture()?;
    draft(&mut app);
    click(&mut app, Kind::Propose(Intent::Accept))?;
    app.update();
    let higher = app
        .world_mut()
        .spawn((GameModal, GlobalZIndex(220), Node::default()))
        .id();
    click(&mut app, Kind::Confirm)?;
    assert!(commands.try_recv().is_err());
    app.world_mut().entity_mut(higher).despawn();
    app.world_mut().non_send_mut::<HostOwner>().close();
    click(&mut app, Kind::Confirm)?;
    assert!(commands.try_recv().is_err());
    app.update();
    assert!(
        app.world_mut()
            .query::<&Overlay>()
            .iter(app.world())
            .next()
            .is_none()
    );
    Ok(())
}
#[test]
fn guild_wars_confirmation_cannot_cross_identical_replacement_session() -> Result<(), Error> {
    let (mut app, mut old_commands, _) = fixture()?;
    draft(&mut app);
    click(&mut app, Kind::Propose(Intent::Accept))?;
    app.update();
    let (link, mut new_commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.guild_wars_test_selected();
    link.guild_war_open();
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    app.world_mut().non_send_mut::<HostOwner>().play =
        Some(crate::bevy_shell::owner::ActivePlay { view, link });
    let before = panel(&app)?;
    click(&mut app, Kind::Confirm)?;
    assert!(new_commands.try_recv().is_err());
    assert!(old_commands.try_recv().is_err());
    assert_eq!(panel(&app)?, before);
    Ok(())
}
#[test]
fn guild_wars_window_registered() -> Result<(), Error> {
    assert!(include_str!("../mod.rs").contains("guild_wars::install(app)"));
    assert!(include_str!("../../ui.rs").contains("Control::GuildWars"));
    let (mut app, _, _) = fixture()?;
    click(&mut app, Kind::Close)?;
    assert!(!panel(&app)?.open);
    app.update();
    let toolbar = app
        .world_mut()
        .query::<(Entity, &crate::bevy_shell::ui::Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| (control.id() == "shell.guild_wars").then_some(entity))
        .ok_or("toolbar entry")?;
    app.world_mut().trigger(Activate { entity: toolbar });
    assert!(panel(&app)?.open);
    app.update();
    let key = |key_code, logical_key| bevy::input::keyboard::KeyboardInput {
        key_code,
        logical_key,
        state: bevy::input::ButtonState::Pressed,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    };
    app.world_mut()
        .write_message(key(KeyCode::Escape, bevy::input::keyboard::Key::Escape));
    app.update();
    assert!(!panel(&app)?.open);
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(&[shortcut::owner_chord()?], false);
    app.update();
    assert!(panel(&app)?.open);
    Ok(())
}

#[test]
fn guild_wars_cancel_preserves_draft_and_queued_edits_delay_confirmation() -> Result<(), Error> {
    let (mut app, mut commands, _) = fixture()?;
    draft(&mut app);
    click(&mut app, Kind::Propose(Intent::Declare))?;
    app.update();
    click(&mut app, Kind::Cancel)?;
    app.update();
    assert!(
        app.world_mut()
            .query::<(&Field, &EditableText)>()
            .iter(app.world())
            .any(|(field, text)| field.0 == 0
                && text.value() == "01010101010170018001010101010101")
    );
    for (_, mut text) in app
        .world_mut()
        .query::<(&Field, &mut EditableText)>()
        .iter_mut(app.world_mut())
    {
        text.queue_edit(bevy::text::TextEdit::Insert("x".into()));
    }
    click(&mut app, Kind::Propose(Intent::Accept))?;
    assert!(app.world().resource::<State>().confirmation.is_none());
    assert!(commands.try_recv().is_err());
    app.update();
    assert!(app.world().resource::<State>().confirmation.is_none());
    for (_, mut text) in app
        .world_mut()
        .query::<(&Field, &mut EditableText)>()
        .iter_mut(app.world_mut())
    {
        text.pending_edits.clear();
    }
    app.update();
    assert!(app.world().resource::<State>().confirmation.is_some());
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[path = "tests/shortcut.rs"]
mod shortcut;
