use super::*;
use crate::bevy_shell::owner::{ActivePlay, HostOwner, Phase};
use crate::play::{PlayView, play_channel};
use bevy::ui_widgets::Activate;
#[derive(Resource)]
struct TestCommands {
    commands: tokio::sync::mpsc::UnboundedReceiver<crate::combat_input::PlayCommand>,
}
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
fn fixture() -> std::result::Result<App, Box<dyn std::error::Error>> {
    let (link, commands, _) = play_channel();
    link.use_shared_frontend();
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 200), -7, &[])?;
    view.use_shared_frontend();
    let mut owner = HostOwner::new(None)?;
    owner.phase = Phase::Playing;
    owner.play = Some(ActivePlay { link, view });
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, bevy::asset::AssetPlugin::default()))
        .init_asset::<Font>()
        .init_asset::<Image>()
        .insert_resource(TestCommands { commands })
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    Ok(app)
}
fn press(app: &mut App, kind: Kind) -> Result {
    let entity = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(e, c)| (c.kind == kind).then_some(e))
        .ok_or("control")?;
    app.world_mut().trigger(Activate { entity });
    Ok(())
}
fn snapshot(app: &App) -> std::result::Result<Snapshot, Box<dyn std::error::Error>> {
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .view
        .minimap_editor_snapshot()
        .ok_or_else(|| "snapshot".into())
}
#[test]
fn minimap_markers_window_shows_owner_data_only() -> Result {
    let mut app = fixture()?;
    open(&mut app.world_mut().non_send_mut::<HostOwner>());
    app.update();
    let panel = snapshot(&app)?;
    assert!(panel.markers.is_empty());
    assert_eq!(
        app.world()
            .resource::<State>()
            .snapshot
            .as_ref()
            .map(|(_, p)| p),
        Some(&panel)
    );
    assert_eq!(
        app.world_mut().query::<&Field>().iter(app.world()).count(),
        4
    );
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|t| t.0 == panel_text(&panel))
    );
    {
        let owner = app.world().non_send::<HostOwner>();
        let play = owner.play.as_ref().ok_or("play")?;
        assert!(play.view.minimap_editor_bound(
            &play.link,
            &panel,
            Action::Save(Marker {
                x: 123,
                y: 456,
                floor: -8,
                label: "Owner supplied marker".into(),
                color: 2,
            }),
            true
        ));
    }
    app.update();
    let panel = snapshot(&app)?;
    assert_eq!(
        app.world()
            .resource::<State>()
            .snapshot
            .as_ref()
            .map(|(_, p)| p),
        Some(&panel)
    );
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|t| t.0 == "Owner supplied marker · 123, 456 · floor -8")
    );
    assert_eq!(
        app.world_mut()
            .query::<&Control>()
            .iter(app.world())
            .filter(|c| matches!(c.kind, Kind::Select(_)))
            .count(),
        panel.markers.len()
    );
    Ok(())
}
#[test]
fn minimap_markers_actions_are_bound_and_confirmed() -> Result {
    let mut app = fixture()?;
    open(&mut app.world_mut().non_send_mut::<HostOwner>());
    app.update();
    for (field, mut text) in app
        .world_mut()
        .query::<(&Field, &mut bevy::text::EditableText)>()
        .iter_mut(app.world_mut())
    {
        if field.0 == 3 {
            text.editor_mut().set_text("Home");
        }
    }
    press(&mut app, Kind::Save)?;
    app.update();
    assert_eq!(snapshot(&app)?.markers[0].label, "Home");
    press(&mut app, Kind::Select(0))?;
    app.update();
    press(&mut app, Kind::GoTo)?;
    app.update();
    for (kind, check) in [(Kind::ZoomOut, 2), (Kind::ZoomIn, 1)] {
        press(&mut app, kind)?;
        app.update();
        assert_eq!(snapshot(&app)?.zoom, check);
    }
    let centre = snapshot(&app)?.centre;
    for kind in [Kind::West, Kind::East, Kind::North, Kind::South] {
        press(&mut app, kind)?;
        app.update();
    }
    assert_eq!(snapshot(&app)?.centre, centre);
    press(&mut app, Kind::FloorUp)?;
    app.update();
    assert_eq!(
        snapshot(&app)?.centre.2,
        oteryn_client_ui::minimap::floor_up(centre.2)
    );
    press(&mut app, Kind::FloorDown)?;
    app.update();
    assert_eq!(snapshot(&app)?.centre.2, centre.2);
    press(&mut app, Kind::Recenter)?;
    app.update();
    assert!(snapshot(&app)?.tracking);
    press(&mut app, Kind::Centre)?;
    app.update();
    assert_eq!(
        app.world()
            .resource::<State>()
            .draft
            .as_ref()
            .ok_or("draft")?
            .fields[0],
        snapshot(&app)?.centre.0.to_string()
    );
    press(&mut app, Kind::Select(0))?;
    app.update();
    press(&mut app, Kind::GoTo)?;
    app.update();
    assert_eq!(snapshot(&app)?.centre, (100, 200, -7));
    press(&mut app, Kind::Color(3))?;
    app.update();
    press(&mut app, Kind::Save)?;
    app.update();
    assert_eq!(snapshot(&app)?.markers[0].color, 3);
    assert_eq!(snapshot(&app)?.markers.len(), 1);

    press(&mut app, Kind::Select(0))?;
    app.update();
    press(&mut app, Kind::Delete)?;
    assert!(app.world().resource::<State>().confirm.is_some());
    assert_eq!(snapshot(&app)?.markers.len(), 1);
    app.update();
    // Editing any draft value revokes consent.
    for (field, mut text) in app
        .world_mut()
        .query::<(&Field, &mut EditableText)>()
        .iter_mut(app.world_mut())
    {
        if field.0 == 3 {
            text.editor_mut().set_text("Other label");
        }
    }
    app.update();
    assert!(app.world().resource::<State>().confirm.is_none());
    press(&mut app, Kind::Delete)?;
    assert_eq!(snapshot(&app)?.markers.len(), 1);
    app.update();
    press(&mut app, Kind::ConfirmDelete)?;
    app.update();
    assert!(snapshot(&app)?.markers.is_empty());
    Ok(())
}
#[test]
fn minimap_markers_rejected_when_session_replaced_or_target_changed() -> Result {
    let mut app = fixture()?;
    open(&mut app.world_mut().non_send_mut::<HostOwner>());
    app.update();
    let old = snapshot(&app)?;
    let owner = app.world().non_send::<HostOwner>();
    let play = owner.play.as_ref().ok_or("play")?;
    assert!(
        play.view
            .minimap_editor_bound(&play.link, &old, Action::ZoomIn, true)
    );
    assert!(
        !play
            .view
            .minimap_editor_bound(&play.link, &old, Action::ZoomIn, true)
    );
    let fresh = play.view.minimap_editor_snapshot().ok_or("fresh")?;
    assert!(
        !play
            .view
            .minimap_editor_bound(&play.link, &fresh, Action::ZoomOut, false)
    );
    let mut replacement = PlayView::new(oteryn_renderer::TileCoord::new(100, 200), -7, &[])?;
    replacement.use_shared_frontend();
    assert!(!replacement.minimap_editor_bound(&play.link, &fresh, Action::Close, true));
    // Close ignores draft/snapshot changes, modal admission and gameplay blocking.
    assert!(
        play.view
            .minimap_editor_bound(&play.link, &old, Action::Close, false)
    );
    assert!(play.view.minimap_editor_snapshot().is_none());
    assert!(
        !play
            .view
            .minimap_editor_bound(&play.link, &fresh, Action::ZoomIn, true)
    );
    assert!(play.view.minimap_editor_open(&play.link, true));
    assert!(
        !play
            .view
            .minimap_editor_bound(&play.link, &old, Action::ZoomIn, true)
    );
    let reopened = play.view.minimap_editor_snapshot().ok_or("reopened")?;
    let (other_link, _other_commands, _) = play_channel();
    other_link.use_shared_frontend();
    assert!(
        !play
            .view
            .minimap_editor_bound(&other_link, &reopened, Action::Close, true)
    );
    assert_eq!(
        play.view.minimap_editor_snapshot().as_ref(),
        Some(&reopened)
    );
    // A newer modal blocks ordinary actions, but Close is always admitted for this session.
    let modal = app
        .world_mut()
        .spawn((super::super::GameModal, Node::default(), GlobalZIndex(999)))
        .id();
    app.update();
    let before = snapshot(&app)?;
    press(&mut app, Kind::ZoomOut)?;
    assert_eq!(snapshot(&app)?, before);
    press(&mut app, Kind::Close)?;
    app.update();
    assert!(app.world().resource::<State>().root.is_none());
    app.world_mut().entity_mut(modal).despawn();
    open(&mut app.world_mut().non_send_mut::<HostOwner>());
    app.update();
    app.world_mut()
        .write_message(bevy::input::keyboard::KeyboardInput {
            key_code: bevy::prelude::KeyCode::Escape,
            logical_key: bevy::input::keyboard::Key::Escape,
            state: bevy::input::ButtonState::Pressed,
            text: None,
            repeat: false,
            window: Entity::PLACEHOLDER,
        });
    app.update();
    assert!(app.world().resource::<State>().root.is_none());

    Ok(())
}
#[test]
fn minimap_markers_window_registered() -> Result {
    let mut app = fixture()?;
    let button = app
        .world_mut()
        .query::<(Entity, &crate::bevy_shell::ui::Control)>()
        .iter(app.world())
        .find_map(|(e, c)| matches!(c, crate::bevy_shell::ui::Control::MinimapMarkers).then_some(e))
        .ok_or("toolbar")?;
    app.world_mut().trigger(Activate { entity: button });
    app.update();
    assert!(app.world().resource::<State>().root.is_some());
    press(&mut app, Kind::Close)?;
    app.update();
    assert!(app.world().resource::<State>().root.is_none());
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let key = NormalizedInputEvent::Key {
        code: KeyCode::new(16)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::NONE
            .with(Modifier::Control)
            .with(Modifier::Alt)
            .with(Modifier::Shift),
        repeat: false,
    };
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(&[key], false);
    app.update();
    assert!(app.world().resource::<State>().root.is_some());
    press(&mut app, Kind::Close)?;
    app.update();
    app.world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .open_chat();
    let chord = NormalizedInputEvent::Key {
        code: KeyCode::new(16)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::NONE
            .with(Modifier::Control)
            .with(Modifier::Alt)
            .with(Modifier::Shift),
        repeat: false,
    };
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(&[chord], false);
    app.update();
    assert!(app.world().resource::<State>().root.is_none());
    let arrow = NormalizedInputEvent::Key {
        code: KeyCode::ARROW_UP,
        state: ButtonState::Pressed,
        modifiers: Modifiers::NONE,
        repeat: false,
    };
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(&[arrow], true);
    assert!(
        app.world_mut()
            .resource_mut::<TestCommands>()
            .commands
            .try_recv()
            .is_err()
    );
    Ok(())
}
