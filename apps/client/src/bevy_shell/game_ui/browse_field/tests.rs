use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use bevy::{asset::AssetPlugin, input_focus::InputFocus};
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
fn fixture() -> std::result::Result<
    (
        App,
        tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
        Outbox,
    ),
    Box<dyn std::error::Error>,
> {
    let (link, rx, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_browse_field_selected(true);
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
    outbox.bevy_browse_field_data()?;
    app.update();
    Ok((app, rx, outbox))
}
fn click(app: &mut App, kind: Kind) -> Result {
    let entity = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(e, c)| (c.kind == kind).then_some(e))
        .ok_or("control")?;
    app.world_mut().trigger(Activate { entity });
    Ok(())
}
fn panel(app: &App) -> std::result::Result<Panel, Box<dyn std::error::Error>> {
    Ok(app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .browse_field_panel())
}
#[test]
fn browse_field_window_shows_server_data_only() -> Result {
    let (mut app, mut rx, outbox) = fixture()?;
    let text = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|t| t.0.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(text.contains("Item 42 × 3"));
    assert!(text.contains("Item 99 × 1"));
    assert!(text.contains("Backpack rows:"));
    assert_eq!(panel(&app)?.views.views[0].view_id, 3);
    assert!(rx.try_recv().is_err());
    outbox.bevy_browse_field_change_target();
    app.update();
    let text = app
        .world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|t| t.0.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert!(!text.contains("Item 42 × 3"));
    assert!(text.contains("Item 99 × 1"));
    Ok(())
}
#[test]
fn browse_field_actions_are_bound_and_confirmed() -> Result {
    let (mut app, mut rx, outbox) = fixture()?;
    let original = panel(&app)?;
    click(&mut app, Kind::Item(106))?;
    assert!(rx.try_recv().is_err());
    app.update();
    click(&mut app, Kind::Confirm)?;
    assert!(
        matches!(rx.try_recv()?,PlayCommand::BrowseFieldMove(intent) if intent.source.get()==2 && matches!(intent.destination,oteryn_session::ItemMoveDestination::Ground(_)))
    );
    assert_eq!(panel(&app)?.inventory, original.inventory);
    assert_eq!(panel(&app)?.views, original.views);
    click(&mut app, Kind::Confirm)?;
    assert!(rx.try_recv().is_err());
    outbox.bevy_browse_field_reply();
    app.update();
    assert!(!panel(&app)?.pending);
    assert_eq!(panel(&app)?.views, original.views);
    for action in [105, 107, 108, 109, 110] {
        click(&mut app, Kind::Item(action))?;
        let command = rx.try_recv()?;
        assert!(matches!(
            (action, command),
            (105, PlayCommand::BrowseFieldMove(_))
                | (107 | 108, PlayCommand::ServerMechanic(_))
                | (109 | 110, PlayCommand::WorldContext(_))
        ));
        outbox.bevy_browse_field_reply();
        app.update();
    }
    for kind in [Kind::Open, Kind::Seek] {
        click(&mut app, kind)?;
        assert!(
            matches!(rx.try_recv()?,PlayCommand::ServerMechanic(c) if c.capability==230 && c.command_type==440)
        );
        outbox.bevy_browse_field_reply();
        app.update();
    }
    Ok(())
}
#[test]
fn browse_field_rejected_when_session_replaced_or_target_changed() -> Result {
    let (mut app, mut rx, outbox) = fixture()?;
    outbox.bevy_browse_field_selected(false);
    click(&mut app, Kind::Item(105))?;
    assert!(rx.try_recv().is_err());
    outbox.bevy_browse_field_selected(true);
    let modal = app
        .world_mut()
        .spawn((GameModal, Node::default(), GlobalZIndex(999)))
        .id();
    click(&mut app, Kind::Item(105))?;
    assert!(rx.try_recv().is_err());
    app.world_mut().despawn(modal);
    app.update();
    click(&mut app, Kind::Item(106))?;
    app.update();
    outbox.bevy_browse_field_change_target();
    click(&mut app, Kind::Confirm)?;
    assert!(rx.try_recv().is_err());
    let (replacement, mut replacement_rx, replacement_outbox) = play_channel();
    replacement.use_shared_frontend();
    replacement_outbox.bevy_browse_field_selected(true);
    replacement.browse_field_set_open_bound(true);
    replacement_outbox.bevy_browse_field_data()?;
    app.world_mut().non_send_mut::<HostOwner>().play = Some(crate::bevy_shell::owner::ActivePlay {
        view: PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
        link: replacement,
    });
    click(&mut app, Kind::Confirm)?;
    assert!(replacement_rx.try_recv().is_err());
    app.world_mut().non_send_mut::<HostOwner>().play = None;
    click(&mut app, Kind::Confirm)?;
    assert!(rx.try_recv().is_err());
    Ok(())
}
#[test]
fn browse_field_window_registered() -> Result {
    let (mut app, mut rx, _) = fixture()?;
    assert!(
        app.world_mut()
            .query::<&crate::bevy_shell::ui::Control>()
            .iter(app.world())
            .any(|c| c.id() == "shell.browse_field")
    );
    click(&mut app, Kind::Close)?;
    app.update();
    assert!(!panel(&app)?.open);
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let code = KeyCode::new(5)?;
    let event = |modifiers| NormalizedInputEvent::Key {
        code,
        state: ButtonState::Pressed,
        modifiers,
        repeat: false,
    };
    let chord = Modifiers::one(Modifier::Control).with(Modifier::Shift);
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(&[event(chord.with(Modifier::Super))], false);
    app.update();
    assert!(!panel(&app)?.open);
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(&[NormalizedInputEvent::DeviceLost], false);
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(&[event(chord)], false);
    app.update();
    assert!(panel(&app)?.open);
    click(&mut app, Kind::Close)?;
    app.update();
    assert!(!panel(&app)?.open);
    let button = app
        .world_mut()
        .query::<(Entity, &crate::bevy_shell::ui::Control)>()
        .iter(app.world())
        .find_map(|(e, c)| (c.id() == "shell.browse_field").then_some(e))
        .ok_or("toolbar")?;
    app.world_mut().trigger(Activate { entity: button });
    app.update();
    assert!(panel(&app)?.open);
    assert!(rx.try_recv().is_err());
    Ok(())
}
