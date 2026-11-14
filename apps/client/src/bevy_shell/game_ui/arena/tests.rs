use super::*;
use crate::{
    combat_input::PlayCommand,
    play::{PlayView, play_channel},
};
use oteryn_client_runtime::arena::{ArenaAction, DromeAction};
use oteryn_renderer::TileCoord;
use oteryn_session::{CommandStatus, GatedCommandResult};
type Result = std::result::Result<(), Box<dyn std::error::Error>>;
type Fixture = (
    App,
    tokio::sync::mpsc::UnboundedReceiver<PlayCommand>,
    crate::play::Outbox,
);
fn app() -> std::result::Result<Fixture, Box<dyn std::error::Error>> {
    let mut owner = HostOwner::new(None)?;
    let (link, commands, outbox) = play_channel();
    link.use_shared_frontend();
    let mut view = PlayView::new(TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    owner.play = Some(crate::bevy_shell::owner::ActivePlay { view, link });
    owner.phase = crate::bevy_shell::owner::Phase::Playing;
    outbox.bevy_arena_selected();
    let mut app = App::new();
    app.add_plugins((
        MinimalPlugins,
        bevy::asset::AssetPlugin::default(),
        oteryn_client_ui_bevy::GlobalUiPlugin,
    ))
    .init_asset::<Image>()
    .init_asset::<Font>()
    .init_resource::<InputFocus>()
    .init_resource::<ButtonInput<KeyCode>>()
    .insert_non_send(owner)
    .add_systems(Startup, crate::bevy_shell::ui::setup);
    super::super::modal::install(&mut app);
    install(&mut app);
    app.update();
    Ok((app, commands, outbox))
}
fn reply(outbox: &crate::play::Outbox, payload: Vec<u8>) {
    outbox.bevy_arena_result(&GatedCommandResult {
        command_id: 1,
        status: CommandStatus::Accepted,
        server_sequence: 1,
        payload,
    });
}
fn texts(app: &mut App) -> Vec<String> {
    app.world_mut()
        .query::<&Text>()
        .iter(app.world())
        .map(|text| text.0.clone())
        .collect()
}
fn click(app: &mut App, predicate: impl Fn(&Action) -> bool) -> Result {
    let entity = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find(|(_, control)| predicate(&control.action))
        .map(|(entity, _)| entity)
        .ok_or("control")?;
    app.world_mut().trigger(Activate { entity });
    app.update();
    Ok(())
}
fn open(app: &mut App) {
    toggle(app.world().non_send::<HostOwner>());
    app.update();
}
fn catalogue() -> std::result::Result<Vec<u8>, Box<dyn std::error::Error>> {
    oteryn_session::arena::encode_catalogue().map_err(|_| "catalogue".into())
}
#[test]
fn arena_window_shows_server_data_only() -> Result {
    let (mut app, mut commands, outbox) = app()?;
    open(&mut app);
    let _ = commands.try_recv()?;
    let before = texts(&mut app);
    assert!(!before.iter().any(|text| text.contains("1000 gold")));
    reply(&outbox, catalogue()?);
    app.update();
    let panel = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .arena_panel();
    assert!(
        texts(&mut app)
            .contains(&oteryn_client_ui::arena::panel_text(&panel).replace("\r\n", "\n"))
    );
    click(&mut app, |a| {
        matches!(a, Action::Request(Intent::Action(ArenaAction::Status), _))
    })?;
    let _ = commands.try_recv()?;
    reply(&outbox, vec![8, 3, 16, 1]);
    app.update();
    assert!(
        texts(&mut app)
            .iter()
            .any(|text| text.contains("Insufficient gold"))
    );
    click(&mut app, |a| {
        matches!(
            a,
            Action::Request(Intent::Action(ArenaAction::Drome(DromeAction::Status)), _)
        )
    })?;
    let _ = commands.try_recv()?;
    reply(
        &outbox,
        vec![8, 2, 16, 8, 24, 15, 32, 10, 40, 150, 1, 80, 2, 88, 3],
    );
    app.update();
    let text = texts(&mut app).join("\n");
    assert!(text.contains("Drome points: 150"));
    assert!(text.contains("Inventory full"));
    assert!(text.contains("Unclaimed potions: 3"));
    assert!(commands.try_recv().is_err());
    Ok(())
}
#[test]
fn arena_all_controls_confirm_and_dispatch_once() -> Result {
    let (mut app, mut commands, outbox) = app()?;
    open(&mut app);
    let _ = commands.try_recv()?;
    reply(&outbox, catalogue()?);
    app.update();
    let actions = widgets::actions();
    assert_eq!(actions.len(), 46, "all native actions and reward selectors");
    for (index, (intent, _)) in actions.iter().enumerate() {
        assert!(
            !actions[..index]
                .iter()
                .any(|(earlier, _)| earlier == intent)
        );
    }
    for id in 200..=204 {
        let native = oteryn_client_ui::arena::drome_intent(id, -1).ok_or("native action")?;
        assert!(actions.iter().any(|(intent, _)| *intent == native));
    }
    for selector in 0..18 {
        let native = oteryn_client_ui::arena::drome_intent(205, selector).ok_or("native reward")?;
        assert!(actions.iter().any(|(intent, _)| *intent == native));
    }
    for (intent, _) in actions {
        let before = app
            .world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .arena_panel();
        click(
            &mut app,
            |action| matches!(action,Action::Request(current,_) if *current==intent),
        )?;
        if arena_needs_confirmation(intent) {
            assert!(commands.try_recv().is_err());
            assert_eq!(
                app.world()
                    .non_send::<HostOwner>()
                    .play
                    .as_ref()
                    .ok_or("play")?
                    .link
                    .arena_panel(),
                before
            );
            click(&mut app, |action| matches!(action, Action::Confirm))?;
        }
        let PlayCommand::Feature(command) = commands.try_recv()? else {
            return Err("feature command".into());
        };
        assert_eq!(command.capability, 170);
        match intent {
            Intent::Catalogue => {
                assert_eq!(command.kind, 240);
                assert!(command.payload.is_empty());
            }
            Intent::Action(action) => {
                assert_eq!(command.kind, 436);
                assert_eq!(
                    command.payload,
                    oteryn_session::arena::encode_match_request(action).map_err(|_| "wire")?
                );
            }
        }
        assert!(commands.try_recv().is_err());
        let waiting = app
            .world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .arena_panel();
        assert_eq!(waiting.challenges, before.challenges);
        assert_eq!(waiting.result, before.result);
        assert_eq!(waiting.drome, before.drome);
        reply(
            &outbox,
            match intent {
                Intent::Catalogue => catalogue()?,
                Intent::Action(ArenaAction::Drome(_)) => {
                    vec![8, 2, 16, 8, 24, 15, 32, 10, 40, 150, 1, 80, 2, 88, 3]
                }
                Intent::Action(_) => vec![8, 3, 16, 1],
            },
        );
        app.update();
    }
    Ok(())
}
#[test]
fn arena_window_registered() -> Result {
    assert!(include_str!("../mod.rs").contains("arena::install(app);"));
    let (mut app, mut commands, outbox) = app()?;
    let toolbar = app
        .world_mut()
        .query::<(Entity, &crate::bevy_shell::ui::Control)>()
        .iter(app.world())
        .find(|(_, c)| matches!(c, crate::bevy_shell::ui::Control::Arena))
        .map(|(e, _)| e)
        .ok_or("arena toolbar")?;
    app.world_mut().trigger(Activate { entity: toolbar });
    app.update();
    assert!(matches!(commands.try_recv()?, PlayCommand::Feature(_)));
    assert!(app.world().resource::<State>().root.is_some());
    app.world_mut().trigger(Activate { entity: toolbar });
    app.update();
    assert!(app.world().resource::<State>().root.is_none());
    reply(&outbox, catalogue()?);
    app.update();
    assert!(app.world().resource::<State>().root.is_none());
    use bevy::input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    };
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ControlLeft);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ShiftLeft);
    let window = app.world_mut().spawn_empty().id();
    app.world_mut().write_message(KeyboardInput {
        key_code: KeyCode::KeyZ,
        logical_key: Key::Character("Z".into()),
        state: ButtonState::Pressed,
        text: None,
        repeat: false,
        window,
    });
    app.update();
    assert!(app.world().resource::<State>().root.is_some());
    assert!(matches!(commands.try_recv()?, PlayCommand::Feature(_)));
    app.world_mut().write_message(KeyboardInput {
        key_code: KeyCode::KeyZ,
        logical_key: Key::Character("Z".into()),
        state: ButtonState::Pressed,
        text: None,
        repeat: false,
        window,
    });
    app.update();
    assert!(app.world().resource::<State>().root.is_none());
    assert!(commands.try_recv().is_err());
    Ok(())
}

#[test]
fn arena_modal_stale_confirmation_and_duplicate_are_rejected() -> Result {
    let (mut app, mut commands, outbox) = app()?;
    open(&mut app);
    let _ = commands.try_recv()?;
    reply(&outbox, catalogue()?);
    app.update();
    click(&mut app, |action| {
        matches!(
            action,
            Action::Request(Intent::Action(ArenaAction::Enter), _)
        )
    })?;
    click(&mut app, |action| matches!(action, Action::Cancel))?;
    assert!(commands.try_recv().is_err());
    click(&mut app, |action| {
        matches!(
            action,
            Action::Request(Intent::Action(ArenaAction::Enter), _)
        )
    })?;
    let (confirm_entity, stale) = app
        .world_mut()
        .query::<(Entity, &Control)>()
        .iter(app.world())
        .find(|(_, control)| matches!(control.action, Action::Confirm))
        .map(|(entity, control)| (entity, control.clone()))
        .ok_or("confirm")?;
    let modal = app
        .world_mut()
        .spawn((super::super::GameModal, Node::default(), GlobalZIndex(999)))
        .id();
    app.update();
    let before = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .arena_panel();
    app.world_mut().trigger(Activate {
        entity: confirm_entity,
    });
    assert!(commands.try_recv().is_err());
    assert_eq!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .arena_panel(),
        before
    );
    app.world_mut().despawn(modal);
    app.update();
    click(&mut app, |action| matches!(action, Action::Confirm))?;
    let _ = commands.try_recv()?;
    // A second activation with the same captured control cannot submit again.
    app.world_mut()
        .resource_scope(|world, mut state: Mut<State>| {
            perform(world.non_send::<HostOwner>(), &mut state, &stale);
        });
    assert!(commands.try_recv().is_err());
    reply(&outbox, vec![8, 3, 16, 1]);
    app.update();
    let before = app
        .world()
        .non_send::<HostOwner>()
        .play
        .as_ref()
        .ok_or("play")?
        .link
        .arena_panel();
    // The original confirmation is stale after the authoritative target state changed.
    app.world_mut()
        .resource_scope(|world, mut state: Mut<State>| {
            perform(world.non_send::<HostOwner>(), &mut state, &stale);
        });
    assert!(commands.try_recv().is_err());
    assert_eq!(
        app.world()
            .non_send::<HostOwner>()
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .arena_panel(),
        before
    );
    Ok(())
}

#[test]
fn arena_shortcut_is_fenced_by_another_modal() -> Result {
    use bevy::input::{
        ButtonState,
        keyboard::{Key, KeyboardInput},
    };
    let (mut app, mut commands, _) = app()?;
    let modal = app
        .world_mut()
        .spawn((super::super::GameModal, Node::default(), GlobalZIndex(999)))
        .id();
    app.world_mut()
        .resource_mut::<InputFocus>()
        .set(modal, bevy::input_focus::FocusCause::Navigated);
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ControlLeft);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::ShiftLeft);
    app.world_mut().write_message(KeyboardInput {
        key_code: KeyCode::KeyZ,
        logical_key: Key::Character("Z".into()),
        state: ButtonState::Pressed,
        text: None,
        repeat: false,
        window: modal,
    });
    app.update();
    assert!(commands.try_recv().is_err());
    assert!(app.world().resource::<State>().root.is_none());
    Ok(())
}
