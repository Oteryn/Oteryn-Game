use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{Outbox, PlayView, play_channel},
};
use bevy::{asset::AssetPlugin, input_focus::InputFocus};
use oteryn_session::{
    CommandStatus, GatedCommandResult,
    animus_mastery::{AnimusPage, encode_page},
};
type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    Outbox,
);
fn fixture() -> Result<Fixture> {
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    outbox.bevy_animus_selected();
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
    Ok((app, commands, outbox))
}
fn toggle(app: &mut App) {
    open(app.world().non_send::<HostOwner>());
    app.update();
}
fn panel(app: &App) -> Result<Panel> {
    Ok(app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .animus_panel())
}
fn control(app: &mut App, kind: Kind) -> Result<Entity> {
    app.world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find_map(|(entity, control)| (control.kind == kind).then_some(entity))
        .ok_or("missing Animus control".into())
}
fn query(commands: &mut tokio::sync::mpsc::UnboundedReceiver<PlayCommand>) -> Result<AnimusPage> {
    let PlayCommand::Animus(bytes) = commands.try_recv()? else {
        return Err("wrong dispatcher command".into());
    };
    oteryn_session::animus_mastery::decode_query(&bytes).ok_or("invalid page query".into())
}
fn reply(outbox: &Outbox, total: u16, keys: &[String]) -> Result {
    outbox.bevy_animus_reply(&GatedCommandResult {
        command_id: 1,
        server_sequence: 1,
        status: CommandStatus::Accepted,
        payload: encode_page(total, keys).ok_or("page encoding")?,
    });
    Ok(())
}
#[test]
fn animus_mastery_window_shows_server_data_only() -> Result {
    let (mut app, mut commands, outbox) = fixture()?;
    toggle(&mut app);
    let _ = query(&mut commands)?;
    let keys = vec![
        "oteryn:creature.cave_rat".into(),
        "oteryn:creature.dragon".into(),
    ];
    reply(&outbox, 51, &keys)?;
    app.update();
    let expected = oteryn_client_ui::animus_mastery::panel_text(&panel(&app)?).replace('\r', "");
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == expected)
    );
    assert_eq!(panel(&app)?.keys, keys);
    assert_eq!(
        app.world_mut()
            .query::<&Overlay>()
            .iter(app.world())
            .count(),
        1
    );
    Ok(())
}
#[test]
fn animus_mastery_actions_are_bound_and_confirmed() -> Result {
    let (mut app, mut commands, outbox) = fixture()?;
    toggle(&mut app);
    assert_eq!(query(&mut commands)?.offset, 0);
    let keys = (0..50)
        .map(|n| format!("oteryn:creature.species_{n:02}"))
        .collect::<Vec<_>>();
    reply(&outbox, 51, &keys)?;
    app.update();
    for (intent, offset) in [
        (Intent::Next, 50),
        (Intent::Previous, 0),
        (Intent::Refresh, 0),
    ] {
        let before = panel(&app)?;
        let button = control(&mut app, Kind::Request(intent))?;
        app.world_mut().trigger(Activate { entity: button });
        app.world_mut().trigger(Activate { entity: button });
        assert_eq!(query(&mut commands)?.offset, offset);
        assert!(commands.try_recv().is_err());
        assert_eq!(panel(&app)?.keys, before.keys);
        assert_eq!(panel(&app)?.offset, before.offset);
        let returned = if offset == 50 {
            vec!["oteryn:creature.dragon".into()]
        } else {
            keys.clone()
        };
        reply(&outbox, 51, &returned)?;
        // Even an identical refresh reply cannot re-admit the consumed widget before redraw.
        app.world_mut().trigger(Activate { entity: button });
        assert!(commands.try_recv().is_err());
        assert!(panel(&app)?.pending.is_none());
        app.update();
        assert_eq!(panel(&app)?.offset, offset);
        assert_eq!(panel(&app)?.keys, returned);
    }
    outbox.bevy_animus_end();
    let before = panel(&app)?;
    reply(&outbox, 1, &["oteryn:creature.stale".into()])?;
    assert_eq!(panel(&app)?, before);
    Ok(())
}
#[test]
fn animus_mastery_rejected_when_session_replaced_or_target_changed() -> Result {
    let (mut app, mut commands, outbox) = fixture()?;
    toggle(&mut app);
    let _ = query(&mut commands)?;
    reply(&outbox, 3, &["oteryn:creature.rat".into()])?;
    app.update();
    let old = control(&mut app, Kind::Request(Intent::Next))?;
    // A reply changes the target page before Bevy redraws.
    let link = &app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link;
    link.animus_request(Intent::Next);
    let _ = query(&mut commands)?;
    reply(&outbox, 3, &["oteryn:creature.dragon".into()])?;
    let before = panel(&app)?;
    app.world_mut().trigger(Activate { entity: old });
    assert_eq!(panel(&app)?, before);
    assert!(commands.try_recv().is_err());
    app.update();
    let refresh = control(&mut app, Kind::Request(Intent::Refresh))?;
    let higher = app
        .world_mut()
        .spawn((GameModal, GlobalZIndex(220), Node::default()))
        .id();
    app.world_mut().trigger(Activate { entity: refresh });
    assert_eq!(panel(&app)?, before);
    assert!(commands.try_recv().is_err());
    app.world_mut().entity_mut(higher).despawn();
    outbox.bevy_animus_end();
    {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        let generation = owner.generation();
        owner.poll([960, 640]);
        assert!(owner.generation() > generation);
    }
    // Replace the session before the old widget is redrawn.
    let (link, mut replacement, outbox2) = play_channel();
    link.use_shared_frontend();
    outbox2.bevy_animus_selected();
    let binding = link.animus_binding();
    assert!(link.animus_dispatch_bound(&binding, Kind::Toggle, true));
    let _ = query(&mut replacement)?;
    reply(&outbox2, 3, &["oteryn:creature.dragon".into()])?;
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
        owner.phase = crate::bevy_shell::owner::Phase::Playing;
    }
    app.world_mut().trigger(Activate { entity: refresh });
    assert!(replacement.try_recv().is_err());
    assert!(commands.try_recv().is_err());
    app.update();
    let current = control(&mut app, Kind::Request(Intent::Refresh))?;
    app.world_mut().non_send_mut::<HostOwner>().close();
    app.world_mut().trigger(Activate { entity: current });
    assert!(replacement.try_recv().is_err());
    Ok(())
}
#[test]
fn animus_mastery_window_registered() -> Result {
    use bevy::input::keyboard::{Key, KeyboardInput};
    let mut registered = App::new();
    super::super::install(&mut registered);
    assert!(registered.world().contains_resource::<State>());
    let (mut app, mut commands, outbox) = fixture()?;
    let toolbar = app
        .world_mut()
        .query::<(Entity, &crate::bevy_shell::ui::Control)>()
        .iter(app.world())
        .find_map(|(e, c)| (c.id() == "shell.animus_mastery").then_some(e))
        .ok_or("toolbar registration missing")?;
    // The real toolbar observer invokes the same bound opening path.
    app.world_mut().trigger(Activate { entity: toolbar });
    app.update();
    assert!(panel(&app)?.open);
    assert_eq!(query(&mut commands)?.offset, 0);
    reply(&outbox, 0, &[])?;
    app.update();
    let close = control(&mut app, Kind::Close)?;
    app.world_mut().trigger(Activate { entity: close });
    app.update();
    assert!(!panel(&app)?.open);
    let higher = app
        .world_mut()
        .spawn((GameModal, GlobalZIndex(220), Node::default()))
        .id();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(higher, FocusCause::Navigated);
    let chord = oteryn_input_actions::NormalizedInputEvent::Key {
        code: oteryn_input_actions::KeyCode::new(4)?,
        state: oteryn_input_actions::ButtonState::Pressed,
        modifiers: oteryn_input_actions::Modifiers::one(oteryn_input_actions::Modifier::Control),
        repeat: false,
    };
    assert!(oteryn_client_ui::animus_mastery::open_key(
        std::slice::from_ref(&chord)
    ));
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(std::slice::from_ref(&chord), true);
    app.update();
    assert!(!panel(&app)?.open);
    assert!(commands.try_recv().is_err());
    app.world_mut().entity_mut(higher).despawn();
    app.update();
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .route_input(&[chord], false);
    app.update();
    assert!(
        panel(&app)?.open,
        "the default Ctrl+A owner chord must reach the bound window"
    );
    assert_eq!(query(&mut commands)?.offset, 0);
    app.world_mut().write_message(KeyboardInput {
        key_code: KeyCode::Escape,
        logical_key: Key::Escape,
        state: bevy::input::ButtonState::Pressed,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    });
    app.update();
    assert!(!panel(&app)?.open);
    // F8 belongs to other features and must have no Animus window binding.
    app.world_mut().write_message(KeyboardInput {
        key_code: KeyCode::F8,
        logical_key: Key::F8,
        state: bevy::input::ButtonState::Pressed,
        text: None,
        repeat: false,
        window: Entity::PLACEHOLDER,
    });
    app.update();
    assert!(!panel(&app)?.open);
    assert!(commands.try_recv().is_err());
    assert!(
        app.world_mut()
            .query::<&Text>()
            .iter(app.world())
            .any(|text| text.0 == "Animus Mastery (Ctrl+A)")
    );
    app.add_systems(Update, crate::bevy_shell::ui::project_toolbar);
    app.update();
    assert_eq!(
        app.world()
            .get::<Node>(toolbar)
            .ok_or("toolbar node")?
            .display,
        Display::Flex
    );
    let play = app.world_mut().non_send_mut::<HostOwner>().play.take();
    app.update();
    assert_eq!(
        app.world()
            .get::<Node>(toolbar)
            .ok_or("toolbar node")?
            .display,
        Display::None
    );
    app.world_mut().non_send_mut::<HostOwner>().play = play;
    app.update();
    assert_eq!(
        app.world()
            .get::<Node>(toolbar)
            .ok_or("toolbar node")?
            .display,
        Display::Flex
    );
    assert_eq!(
        app.world_mut()
            .query::<&Overlay>()
            .iter(app.world())
            .count(),
        0
    );
    Ok(())
}
