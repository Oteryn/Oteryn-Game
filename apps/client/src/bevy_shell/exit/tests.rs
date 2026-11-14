use super::*;
use crate::bevy_shell::owner::Phase;
use bevy::{asset::AssetPlugin, ecs::system::RunSystemOnce};
type TestResult = Result<(), Box<dyn std::error::Error>>;

fn app(confirm: bool) -> Result<(App, Entity), Box<dyn std::error::Error>> {
    let mut owner = HostOwner::new(None)?;
    owner.settings.preferences.miscellaneous.confirm_exit = confirm;
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<Font>()
        .init_resource::<InputFocus>()
        .add_message::<bevy::input::keyboard::KeyboardInput>()
        .insert_non_send(owner)
        .add_systems(Startup, crate::bevy_shell::ui::setup);
    crate::bevy_shell::game_ui::modal::install(&mut app);
    install(&mut app);
    let window = app
        .world_mut()
        .spawn((Window::default(), PrimaryWindow))
        .id();
    app.update();
    Ok((app, window))
}
fn close(app: &mut App, window: Entity) {
    app.world_mut()
        .write_message(WindowCloseRequested { window });
    app.update();
}
fn button(app: &mut App, confirm: bool) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query::<(Entity, &Reply)>()
        .iter(app.world())
        .find(|(_, reply)| reply.confirm == confirm)
        .map(|(entity, _)| entity)
        .ok_or_else(|| "exit reply button absent".into())
}
fn overlay(app: &mut App) -> Result<Entity, Box<dyn std::error::Error>> {
    app.world_mut()
        .query_filtered::<Entity, With<Overlay>>()
        .single(app.world())
        .map_err(|_| "exit overlay absent".into())
}
fn exits(app: &mut App) -> usize {
    app.world_mut()
        .resource_mut::<Messages<AppExit>>()
        .drain()
        .count()
}

#[test]
fn window_close_confirmation_can_be_cancelled_through_its_real_button() -> TestResult {
    let (mut app, window) = app(true)?;
    close(&mut app, window);
    let dialog = overlay(&mut app)?;
    assert_eq!(
        app.world()
            .get::<Node>(dialog)
            .ok_or("overlay node")?
            .display,
        Display::Flex
    );
    assert_eq!(app.world().non_send::<HostOwner>().phase, Phase::Offline);
    assert_eq!(exits(&mut app), 0);
    let no = button(&mut app, false)?;
    assert_eq!(app.world().resource::<InputFocus>().get(), Some(no));
    app.world_mut().trigger(Activate { entity: no });
    app.update();
    assert_eq!(
        app.world()
            .get::<Node>(dialog)
            .ok_or("overlay node")?
            .display,
        Display::None
    );
    assert_eq!(app.world().non_send::<HostOwner>().phase, Phase::Offline);
    assert_eq!(exits(&mut app), 0);
    Ok(())
}
#[test]
fn accepted_no_confirmation_closes_offline_entering_and_playing_once() -> TestResult {
    use crate::play::{PlayView, play_channel};
    for phase in [Phase::Offline, Phase::Entering, Phase::Playing] {
        let (mut app, window) = app(false)?;
        if phase == Phase::Playing {
            let (link, _commands, _outbox) = play_channel();
            link.use_shared_frontend();
            let mut view = PlayView::new(oteryn_renderer::TileCoord::new(0, 0), 0, &[])?;
            view.use_shared_frontend();
            app.world_mut().non_send_mut::<HostOwner>().play =
                Some(crate::bevy_shell::owner::ActivePlay { view, link });
        }
        app.world_mut().non_send_mut::<HostOwner>().phase = phase;
        app.world_mut()
            .write_message(WindowCloseRequested { window });
        app.world_mut()
            .write_message(WindowCloseRequested { window });
        app.update();
        assert_eq!(app.world().non_send::<HostOwner>().phase, Phase::Closed);
        assert!(app.world().non_send::<HostOwner>().play.is_none());
        assert_eq!(exits(&mut app), 1);
    }
    Ok(())
}
#[test]
fn stale_and_duplicate_reply_cannot_close_a_new_confirmation_or_close_twice() -> TestResult {
    let (mut app, window) = app(true)?;
    close(&mut app, window);
    let old_yes = button(&mut app, true)?;
    let no = button(&mut app, false)?;
    let first = app
        .world()
        .resource::<State>()
        .pending
        .ok_or("first request")?;
    // Repeated OS requests retain a single active confirmation.
    close(&mut app, window);
    assert_eq!(app.world().resource::<State>().pending, Some(first));
    app.world_mut().trigger(Activate { entity: no });
    app.world_mut()
        .write_message(WindowCloseRequested { window });
    // Exercise the old published widget before projection replaces it for the new epoch.
    app.world_mut()
        .run_system_once(close_requests)
        .map_err(|error| format!("close request: {error:?}"))?;
    let current = app
        .world()
        .resource::<State>()
        .pending
        .ok_or("second request")?;
    assert_ne!(first, current);
    app.world_mut().trigger(Activate { entity: old_yes });
    assert_eq!(app.world().non_send::<HostOwner>().phase, Phase::Offline);
    assert_eq!(app.world().resource::<State>().pending, Some(current));
    app.update();
    let yes = button(&mut app, true)?;
    app.world_mut().trigger(Activate { entity: yes });
    app.world_mut().trigger(Activate { entity: yes });
    assert_eq!(app.world().non_send::<HostOwner>().phase, Phase::Closed);
    assert_eq!(exits(&mut app), 1);
    Ok(())
}

#[test]
fn only_the_primary_window_requests_application_exit() -> TestResult {
    let (mut app, _primary) = app(true)?;
    let secondary = app.world_mut().spawn(Window::default()).id();
    close(&mut app, secondary);
    let dialog = overlay(&mut app)?;
    assert_eq!(
        app.world()
            .get::<Node>(dialog)
            .ok_or("overlay node")?
            .display,
        Display::None
    );
    assert_eq!(app.world().non_send::<HostOwner>().phase, Phase::Offline);
    assert_eq!(exits(&mut app), 0);
    Ok(())
}

#[derive(Resource, Default)]
struct UnderlyingEscapes(u32);
#[test]
fn one_escape_cancels_quit_without_reaching_the_underlying_modal() -> TestResult {
    use bevy::input::{
        ButtonState,
        keyboard::{Key as LogicalKey, KeyboardInput},
    };
    let (mut app, window) = app(true)?;
    let underlying = app
        .world_mut()
        .spawn((GameModal, GlobalZIndex(200), Node::default()))
        .id();
    app.init_resource::<UnderlyingEscapes>().add_systems(
        Update,
        (move |guard: ModalGuard,
               mut keys: MessageReader<KeyboardInput>,
               mut count: ResMut<UnderlyingEscapes>| {
            if keys
                .read()
                .any(|key| key.key_code == KeyCode::Escape && key.state == ButtonState::Pressed)
                && guard.allows(underlying)
            {
                count.0 += 1;
            }
        })
        .after(widgets::project),
    );
    close(&mut app, window);
    app.world_mut().write_message(KeyboardInput {
        key_code: KeyCode::Escape,
        logical_key: LogicalKey::Escape,
        state: ButtonState::Pressed,
        text: None,
        repeat: false,
        window,
    });
    app.update();
    let dialog = overlay(&mut app)?;
    assert_eq!(
        app.world()
            .get::<Node>(dialog)
            .ok_or("overlay node")?
            .display,
        Display::None
    );
    assert_eq!(app.world().resource::<UnderlyingEscapes>().0, 0);
    assert_eq!(
        app.world()
            .get::<Node>(underlying)
            .ok_or("underlying modal")?
            .display,
        Display::Flex
    );
    assert_eq!(app.world().non_send::<HostOwner>().phase, Phase::Offline);
    assert_eq!(exits(&mut app), 0);
    Ok(())
}

#[test]
fn confirmed_quit_releases_the_pending_selector_callback_without_a_server() -> TestResult {
    use crate::bevy_shell::selection::SelectionBridge;
    use crate::character_selection::SelectionAction;
    use oteryn_foundation::CancellationToken;
    use std::{
        thread,
        time::{Duration, Instant},
    };
    let (mut app, window) = app(true)?;
    let (bridge, callback) = SelectionBridge::new();
    {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        owner.selection = bridge;
        owner.phase = Phase::Entering;
    }
    let cancellation = CancellationToken::new();
    let pending_cancellation = cancellation.clone();
    let worker = thread::spawn(move || callback(&[], "", pending_cancellation));
    let deadline = Instant::now() + Duration::from_secs(3);
    while app
        .world()
        .non_send::<HostOwner>()
        .selection
        .pending()
        .is_none()
        && Instant::now() < deadline
    {
        app.world_mut().non_send_mut::<HostOwner>().selection.poll();
        thread::yield_now();
    }
    if app
        .world()
        .non_send::<HostOwner>()
        .selection
        .pending()
        .is_none()
    {
        cancellation.cancel();
        let _ = worker.join();
        return Err("selector callback did not arrive".into());
    }
    close(&mut app, window);
    let yes = button(&mut app, true)?;
    app.world_mut().trigger(Activate { entity: yes });
    let result = worker.join().map_err(|_| "selector callback thread")?;
    assert_eq!(result, SelectionAction::Cancel);
    assert_eq!(app.world().non_send::<HostOwner>().phase, Phase::Closed);
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .selection
            .pending()
            .is_none()
    );
    assert_eq!(exits(&mut app), 1);
    Ok(())
}
