use super::*;
type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Tibia Global binds Ctrl+M to ToggleShowServermessagesInCurrentChannel
/// (docs/reference/tibia-manual/controls.md:93); the Market opens from the depot locker.
#[test]
fn ctrl_m_toggles_server_messages_only_outside_blocked_input() -> TestResult {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let mut host = HostOwner::new(None)?;
    let (view, link) = admitted_view([9; 16])?;
    host.install_play(view, link);
    let mut shortcut = NormalizedInputEvent::Key {
        code: KeyCode::new(16)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(Modifier::Control),
        repeat: false,
    };
    let shown = |host: &HostOwner| {
        host.play
            .as_ref()
            .is_some_and(|play| play.link.chat_panel().console.show_server_messages)
    };
    let market_open = |host: &HostOwner| {
        host.play
            .as_ref()
            .is_some_and(|play| play.link.market_panel().open)
    };
    let mut press = |host: &mut HostOwner, blocked: bool| {
        if let NormalizedInputEvent::Key { state, .. } = &mut shortcut {
            *state = ButtonState::Pressed;
        }
        host.route_input(std::slice::from_ref(&shortcut), blocked);
        if let NormalizedInputEvent::Key { state, .. } = &mut shortcut {
            *state = ButtonState::Released;
        }
        host.route_input(std::slice::from_ref(&shortcut), blocked);
    };
    let before = shown(&host);
    press(&mut host, true);
    assert_eq!(shown(&host), before);
    host.open_graphics();
    press(&mut host, false);
    assert_eq!(shown(&host), before);
    host.cancel();
    press(&mut host, false);
    assert_eq!(
        shown(&host),
        !before,
        "Ctrl+M must toggle the server messages"
    );
    assert!(!market_open(&host), "Global has no Market chord");
    Ok(())
}

/// Tibia Global binds Ctrl+K to ShowOptionsHotkeys in every hotkey set of the installed client
/// options (%LOCALAPPDATA%/Tibia/packages/Tibia/conf/clientoptions.json).
#[test]
fn ctrl_k_opens_the_hotkey_options_only_outside_blocked_input() -> TestResult {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let mut host = HostOwner::new(None)?;
    let (view, link) = admitted_view([9; 16])?;
    host.install_play(view, link);
    let mut shortcut = NormalizedInputEvent::Key {
        code: KeyCode::new(14)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(Modifier::Control),
        repeat: false,
    };
    let mut press = |host: &mut HostOwner, blocked, editor| {
        crate::bevy_shell::input::route_keyboard(host, shortcut.clone(), blocked, editor);
        if let NormalizedInputEvent::Key { state, .. } = &mut shortcut {
            *state = ButtonState::Released;
        }
        crate::bevy_shell::input::route_keyboard(host, shortcut.clone(), blocked, editor);
        if let NormalizedInputEvent::Key { state, .. } = &mut shortcut {
            *state = ButtonState::Pressed;
        }
    };
    press(&mut host, true, None);
    assert!(host.preferences_draft().is_none());
    assert_eq!(host.hotkey_options, 0);
    press(&mut host, false, Some(false));
    assert!(host.preferences_draft().is_none());
    assert_eq!(host.hotkey_options, 0);
    press(&mut host, false, None);
    assert!(
        host.preferences_draft().is_some(),
        "Ctrl+K opens the options"
    );
    assert_eq!(host.hotkey_options, 1, "on the hotkey page");
    Ok(())
}

#[test]
fn taskboard_shortcut_opens_the_portable_owner_only_outside_blocked_input() -> TestResult {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let mut host = HostOwner::new(None)?;
    let (view, link) = admitted_view([9; 16])?;
    host.install_play(view, link);
    let shortcut = NormalizedInputEvent::Key {
        code: KeyCode::new(23)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(Modifier::Control).with(Modifier::Shift),
        repeat: false,
    };
    let is_open = |host: &HostOwner| {
        host.play
            .as_ref()
            .is_some_and(|play| play.link.task_board_panel().open)
    };
    host.route_input(std::slice::from_ref(&shortcut), true);
    assert!(!is_open(&host));
    host.open_graphics();
    host.route_input(std::slice::from_ref(&shortcut), false);
    assert!(!is_open(&host));
    host.cancel();
    host.route_input(std::slice::from_ref(&shortcut), false);
    assert!(
        is_open(&host),
        "Ctrl+Shift+T must reach the portable Task Board owner"
    );
    Ok(())
}

use crate::play::screenshot::TestDir as Temp;
fn admitted_view(character: [u8; 16]) -> Result<(PlayView, PlayLink), Box<dyn std::error::Error>> {
    // Test-only fixture. Production install_play is reached solely from Reentry::poll.
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.set_preference_character(character);
    let (link, _commands, _events) = crate::play::play_channel();
    Ok((view, link))
}
#[test]
fn apply_refreshes_accepted_draft_and_reloads_existing_store_cancel_discards() -> TestResult {
    let temp = Temp::new()?;
    let path = temp.join("preferences.json");
    let mut host = HostOwner::new(Some(path.clone()))?;
    let before = host.accepted();
    host.open_graphics();
    host.edit(|d| {
        d.fullscreen = !before.fullscreen;
        d.vsync = !before.vsync;
        d.integer_scaling = !before.integer_scaling;
    });
    let edited = host.draft().ok_or("draft")?;
    assert_eq!(host.accepted(), before);
    host.apply(false);
    assert_eq!(host.accepted(), edited);
    assert_eq!(host.draft(), Some(edited));
    assert_eq!(host.settings.feedback, "Applied and saved");
    host.edit(|d| d.vsync = !d.vsync);
    host.cancel();
    assert_eq!(host.accepted(), edited);
    assert_eq!(host.draft(), None);
    let reload = HostOwner::new(Some(path))?;
    assert_eq!(reload.accepted(), edited);
    host.open_graphics();
    host.edit(|d| d.vsync = !d.vsync);
    host.apply(true);
    assert_eq!(host.draft(), None);
    assert_ne!(host.accepted(), edited);
    Ok(())
}
#[test]
fn blocked_save_accepts_live_settings_and_preserves_exact_owner_feedback() -> TestResult {
    let temp = Temp::new()?;
    let path = temp.join("preferences.json");
    let mut host = HostOwner::new(Some(path.clone()))?;
    // A directory at the destination reliably refuses the existing owner's atomic save.
    std::fs::create_dir(&path)?;
    host.open_graphics();
    host.edit(|d| d.fullscreen = !d.fullscreen);
    let expected = host.draft().ok_or("draft")?;
    host.apply(false);
    assert_eq!(host.accepted(), expected);
    assert_eq!(host.draft(), Some(expected));
    assert!(
        host.settings
            .feedback
            .starts_with("Applied for this session; could not save:")
    );
    assert!(path.is_dir());
    Ok(())
}
#[test]
fn admission_and_disconnect_discard_drafts_and_restore_actual_character_scope() -> TestResult {
    let temp = Temp::new()?;
    let mut host = HostOwner::new(Some(temp.join("preferences.json")))?;
    host.open_graphics();
    host.edit(|d| d.fullscreen = !d.fullscreen);
    let old = host.draft.take().ok_or("old draft")?;
    let (view, link) = admitted_view([1; 16])?;
    assert!(host.install_play(view, link));
    assert_eq!(host.phase, Phase::Playing);
    assert_eq!(host.settings.character_scope(), Some([1; 16]));
    assert_eq!(host.draft(), None);
    let accepted = host.accepted();
    host.draft = Some(old);
    host.apply(false);
    assert_eq!(host.accepted(), accepted);
    assert_eq!(host.draft(), None);
    host.open_graphics();
    host.edit(|d| d.vsync = !d.vsync);
    let generation = host.generation();
    host.leave("Disconnected.");
    assert!(host.play.is_none());
    assert_eq!(host.draft(), None);
    assert_eq!(host.settings.character_scope(), None);
    assert!(host.generation() > generation);
    assert_eq!(host.phase, Phase::Offline);
    Ok(())
}
#[test]
fn same_character_new_admission_generation_rejects_old_draft_even_without_storage() -> TestResult {
    let mut host = HostOwner::new(None)?;
    let (view, link) = admitted_view([3; 16])?;
    host.install_play(view, link);
    host.open_graphics();
    host.edit(|d| d.vsync = !d.vsync);
    let stale = host.draft.take().ok_or("stale draft")?;
    let (view, link) = admitted_view([3; 16])?;
    host.install_play(view, link);
    let accepted = host.accepted();
    host.draft = Some(stale);
    host.apply(false);
    assert_eq!(host.accepted(), accepted);
    assert_eq!(host.draft(), None);
    Ok(())
}
#[test]
fn real_restore_projects_graphics_to_play_scene_and_actual_bevy_window() -> TestResult {
    let temp = Temp::new()?;
    let mut host = HostOwner::new(Some(temp.join("preferences.json")))?;
    let (view, link) = admitted_view([4; 16])?;
    host.install_play(view, link);
    let mut p = host.settings.preferences.clone();
    p.ui_scale = 150;
    p.interface_scale = 125;
    host.settings.apply(p);
    host.open_graphics();
    host.edit(|d| {
        d.fullscreen = true;
        d.vsync = false;
        d.integer_scaling = true;
    });
    host.apply(false);
    let play = host.play.as_mut().ok_or("play")?;
    host.settings.restore(&mut play.view, [960, 720])?;
    assert!(play.view.preferences().graphics.integer_scaling);
    assert_eq!(
        play.view.scene().view().tile_px(),
        host.settings
            .preferences
            .graphics
            .tile_pixels(150, [960, 720])
    );
    assert_eq!(
        play.view.scene().view().tile_px() % oteryn_renderer::PAGE_CELL_PX,
        0
    );
    assert_eq!(play.view.preferences().interface_scale, 125);
    assert_eq!(play.view.preferences().ui_scale, 150);
    let mut window = bevy::prelude::Window::default();
    super::super::render_settings::apply_window(&host.settings.preferences, &mut window);
    assert!(matches!(
        window.mode,
        bevy::window::WindowMode::BorderlessFullscreen(_)
    ));
    assert_eq!(window.present_mode, bevy::window::PresentMode::AutoNoVsync);
    host.open_graphics();
    host.edit(|d| {
        d.fullscreen = false;
        d.vsync = true;
    });
    host.apply(true);
    super::super::render_settings::apply_window(&host.settings.preferences, &mut window);
    assert_eq!(window.mode, bevy::window::WindowMode::Windowed);
    assert_eq!(window.present_mode, bevy::window::PresentMode::AutoVsync);
    Ok(())
}
#[test]
fn missing_identity_cannot_install_play_and_close_consumes_runtime_once() -> TestResult {
    let mut host = HostOwner::new(None)?;
    let view = PlayView::new(oteryn_renderer::TileCoord::new(0, 0), 7, &[])?;
    let (link, _commands, _events) = crate::play::play_channel();
    assert!(!host.install_play(view, link));
    assert!(host.play.is_none());
    host.open_graphics();
    host.close();
    host.close();
    assert_eq!(host.phase, Phase::Closed);
    assert!(host.entry.is_none());
    assert!(host.draft().is_none());
    host.open_graphics();
    assert!(host.draft().is_none());
    Ok(())
}
#[test]
fn reentry_poll_is_nonblocking_and_missing_configuration_never_fabricates_play() -> TestResult {
    let mut host = HostOwner::new(None)?;
    assert!(host.entry.as_mut().ok_or("entry")?.request());
    host.phase = Phase::Entering;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while host.phase == Phase::Entering && std::time::Instant::now() < deadline {
        host.poll([960, 720]);
        std::thread::yield_now();
    }
    assert_eq!(host.phase, Phase::Offline);
    assert!(host.play.is_none());
    host.close();
    assert!(host.entry.is_none());
    Ok(())
}

#[test]
fn native_widget_events_reach_owner_apply_cancel_and_public_audit() -> TestResult {
    use super::super::{audit, ui};
    use bevy::{
        asset::AssetPlugin,
        input_focus::InputFocus,
        prelude::*,
        ui_widgets::{Activate, ValueChange},
    };
    let temp = Temp::new()?;
    let mut app = App::new();
    app.add_plugins((MinimalPlugins, AssetPlugin::default()))
        .init_asset::<Image>()
        .init_asset::<Font>()
        .init_resource::<InputFocus>()
        .insert_non_send(HostOwner::new(Some(temp.join("preferences.json")))?)
        .insert_resource(audit::AuditOutput::new(Some(temp.join("ui-audit.json"))))
        .add_systems(Startup, ui::setup)
        .add_systems(Update, ui::project);
    app.update();
    let find = |app: &mut App, id: &str| -> Result<Entity, Box<dyn std::error::Error>> {
        app.world_mut()
            .query::<(Entity, &ui::Control)>()
            .iter(app.world())
            .find(|(_, c)| c.id() == id)
            .map(|(e, _)| e)
            .ok_or_else(|| format!("missing {id}").into())
    };
    let options = find(&mut app, "shell.options")?;
    app.world_mut().trigger(Activate { entity: options });
    app.update();
    let before = app.world().non_send::<HostOwner>().accepted();
    let fullscreen = find(&mut app, "graphics.fullscreen")?;
    app.world_mut().trigger(ValueChange {
        source: fullscreen,
        value: !before.fullscreen,
        is_final: true,
    });
    app.update();
    assert_eq!(app.world().non_send::<HostOwner>().accepted(), before);
    let apply = find(&mut app, "graphics.apply")?;
    app.world_mut().trigger(Activate { entity: apply });
    app.update();
    assert_eq!(
        app.world().non_send::<HostOwner>().accepted().fullscreen,
        !before.fullscreen
    );
    let accepted = app.world().non_send::<HostOwner>().accepted();
    app.world_mut().trigger(ValueChange {
        source: fullscreen,
        value: before.fullscreen,
        is_final: true,
    });
    let cancel = find(&mut app, "graphics.cancel")?;
    app.world_mut().trigger(Activate { entity: cancel });
    app.update();
    assert_eq!(app.world().non_send::<HostOwner>().accepted(), accepted);
    assert_eq!(app.world().non_send::<HostOwner>().draft(), None);
    Ok(())
}

#[test]
fn existing_input_owners_turn_before_walk_and_clear_all_held_mappings_on_modal_reset() -> TestResult
{
    use crate::combat_input::PlayCommand;
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let mut host = HostOwner::new(None)?;
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.set_preference_character([5; 16]);
    let (link, mut commands, _events) = crate::play::play_channel();
    host.install_play(view, link);
    let key = |mods| NormalizedInputEvent::Key {
        code: KeyCode::ARROW_UP,
        state: ButtonState::Pressed,
        modifiers: mods,
        repeat: false,
    };
    host.route_input(&[key(Modifiers::one(Modifier::Control))], false);
    let play = host.play.as_mut().ok_or("play")?;
    play.view
        .tick(&play.link)
        .map_err(|end| format!("play ended unexpectedly: {end:?}"))?;
    // With the target-move capability absent the turn is consumed, never converted to a walk.
    while let Ok(command) = commands.try_recv() {
        assert!(!matches!(command, PlayCommand::Step(_)));
    }
    host.open_graphics();
    host.route_input(&[key(Modifiers::NONE)], true);
    host.cancel();
    host.route_input(&[key(Modifiers::NONE)], false);
    let play = host.play.as_mut().ok_or("play")?;
    play.view
        .tick(&play.link)
        .map_err(|end| format!("play ended unexpectedly: {end:?}"))?;
    let mut walked = false;
    while let Ok(command) = commands.try_recv() {
        if let PlayCommand::Step(direction) = command {
            assert_eq!(direction, crate::input::StepDir::North);
            walked = true;
        }
    }
    assert!(
        walked,
        "a new unmodified arrow must walk after the modal clears held Ctrl mapping"
    );
    Ok(())
}

#[test]
fn full_preferences_draft_cancels_applies_and_preserves_invalid_edits() -> TestResult {
    let temp = Temp::new()?;
    let path = temp.join("preferences.json");
    let mut host = HostOwner::new(Some(path.clone()))?;
    let original = host.settings.preferences.interface_scale;
    host.open_graphics();
    host.edit_preferences(|preferences| preferences.interface_scale = 125);
    assert_eq!(
        host.preferences_draft().ok_or("draft")?.interface_scale,
        125
    );
    assert_eq!(host.settings.preferences.interface_scale, original);
    host.cancel();
    assert_eq!(host.settings.preferences.interface_scale, original);
    host.open_graphics();
    host.edit_preferences(|preferences| preferences.interface_scale = 125);
    host.apply(false);
    assert_eq!(host.settings.preferences.interface_scale, 125);
    assert_eq!(
        HostOwner::new(Some(path))?
            .settings
            .preferences
            .interface_scale,
        125
    );
    host.edit_preferences(|preferences| preferences.interface_scale = 0);
    host.apply(true);
    assert_eq!(host.settings.preferences.interface_scale, 125);
    assert_eq!(
        host.preferences_draft()
            .ok_or("invalid draft discarded")?
            .interface_scale,
        0
    );
    Ok(())
}

#[test]
fn proficiency_shortcut_requests_catalogue_only_outside_blocked_input() -> TestResult {
    use crate::combat_input::PlayCommand;
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let mut owner = HostOwner::new(None)?;
    let (link, mut commands, outbox) = crate::play::play_channel();
    link.use_shared_frontend();
    outbox.bevy_proficiency_initialize();
    let mut view = PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    view.set_preference_character([7; 16]);
    owner.install_play(view, link);
    let key = NormalizedInputEvent::Key {
        code: KeyCode::new(19)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(Modifier::Control).with(Modifier::Shift),
        repeat: false,
    };
    owner.route_input(std::slice::from_ref(&key), true);
    assert!(commands.try_recv().is_err());
    owner.open_graphics();
    owner.route_input(std::slice::from_ref(&key), false);
    assert!(commands.try_recv().is_err());
    owner.cancel();
    owner.route_input(&[key], false);
    let PlayCommand::Feature(command) = commands.try_recv()? else {
        return Err("proficiency catalogue request missing".into());
    };
    assert_eq!((command.capability, command.kind), (195, 660));
    assert_eq!(command.payload, 0u32.to_be_bytes());
    assert!(
        owner
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .weapon_proficiency_panel()
            .open
    );
    Ok(())
}

fn poll_host_until(host: &mut HostOwner, done: impl Fn(&HostOwner) -> bool) -> TestResult {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        host.poll([800, 600]);
        if done(host) {
            return Ok(());
        }
        if std::time::Instant::now() >= deadline {
            return Err(format!("Host did not reach the expected state: {}", host.status).into());
        }
        std::thread::yield_now();
    }
}

#[test]
fn failed_channel_entry_discards_pending_arrival() -> TestResult {
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(|| exercise_failed_channel_entry().map_err(|error| error.to_string()))?
        .join()
        .map_err(|_| "native Channel entry test thread panicked")?
        .map_err(Into::into)
}

fn exercise_failed_channel_entry() -> TestResult {
    use crate::native_entry::test_support::{Stack, channel_node};
    use std::sync::atomic::Ordering;
    let stack = Stack::start_two_channels()?;
    let channel = channel_node::uuid(channel_node::CHANNEL_B).ok_or("Channel B")?;
    let mut host = HostOwner::new(None)?;
    let mut entry = crate::reentry::Reentry::new(
        crate::ClientBootstrap::new()?.with_native_login(stack.config()),
    );
    assert!(entry.request());
    host.entry = Some(entry);
    poll_host_until(&mut host, |host| host.phase == Phase::Playing)?;

    let switch = |host: &mut HostOwner| -> TestResult {
        poll_host_until(host, |host| {
            host.play
                .as_ref()
                .is_some_and(|play| play.link.channel_switch_panel().selected)
        })?;
        host.play
            .as_ref()
            .ok_or("playing")?
            .link
            .channel_switch_open();
        poll_host_until(host, |host| {
            host.play.as_ref().is_some_and(|play| {
                let panel = play.link.channel_switch_panel();
                !panel.pending && panel.status.is_some()
            })
        })?;
        let link = &host.play.as_ref().ok_or("playing")?.link;
        link.channel_switch_select(channel);
        assert!(link.channel_switch_confirm());
        poll_host_until(host, |host| host.phase == Phase::Entering)
    };
    stack.node.refuse_destination.store(true, Ordering::SeqCst);
    switch(&mut host)?;
    assert_eq!(host.arrival.as_ref().map(|row| row.channel), Some(channel));
    poll_host_until(&mut host, |host| host.phase == Phase::Offline)?;
    assert!(host.status.contains("This channel is full."));
    assert!(host.arrival.is_none());

    assert!(host.entry.as_mut().ok_or("entry")?.request());
    poll_host_until(&mut host, |host| host.phase == Phase::Playing)?;
    let play = host.play.as_ref().ok_or("successful login")?;
    assert!(!play.view.notification_text(false).contains("Second"));

    stack.node.refuse_destination.store(false, Ordering::SeqCst);
    stack.node.switches.store(0, Ordering::SeqCst);
    switch(&mut host)?;
    let expected = oteryn_client_ui::channel_switch::arrival_text(
        host.arrival.as_ref().ok_or("pending arrival")?,
    );
    poll_host_until(&mut host, |host| host.phase == Phase::Playing)?;
    let play = host.play.as_ref().ok_or("successful switch")?;
    assert_eq!(
        play.view
            .notification_text(false)
            .matches(&expected)
            .count(),
        1
    );
    host.close();
    stack.finish();
    Ok(())
}

#[test]
fn arrow_chord_observes_modal_resets_and_consumed_releases() -> TestResult {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifiers, NormalizedInputEvent};
    let mut host = HostOwner::new(None)?;
    let (view, _) = admitted_view([9; 16])?;
    let (link, _, outbox) = crate::play::play_channel();
    crate::play::feature_0013_paced_movement::protocol::bevy_diagonal_fixture(&outbox);
    host.install_play(view, link);
    let key = |code, state| NormalizedInputEvent::Key {
        code,
        state,
        modifiers: Modifiers::NONE,
        repeat: false,
    };
    host.route_input(&[key(KeyCode::ARROW_UP, ButtonState::Pressed)], false);
    host.open_graphics();
    host.route_input(&[key(KeyCode::ARROW_UP, ButtonState::Released)], false);
    host.cancel();
    host.route_input(&[key(KeyCode::ARROW_LEFT, ButtonState::Pressed)], false);
    let play = host.play.as_mut().ok_or("play")?;
    assert_eq!(play.view.next_step(), Some(crate::input::StepDir::West));
    Ok(())
}

#[test]
fn arrow_chord_observes_releases_consumed_by_chat() -> TestResult {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifiers, NormalizedInputEvent};
    let mut host = HostOwner::new(None)?;
    let (view, _) = admitted_view([9; 16])?;
    let (link, _, outbox) = crate::play::play_channel();
    crate::play::feature_0013_paced_movement::protocol::bevy_diagonal_fixture(&outbox);
    host.install_play(view, link);
    let key = |code, state| NormalizedInputEvent::Key {
        code,
        state,
        modifiers: Modifiers::NONE,
        repeat: false,
    };
    host.route_input(&[key(KeyCode::ARROW_UP, ButtonState::Pressed)], false);
    host.route_input(
        &[
            key(KeyCode::new(88)?, ButtonState::Pressed),
            key(KeyCode::ARROW_UP, ButtonState::Released),
        ],
        false,
    );
    host.route_input(&[key(KeyCode::ARROW_LEFT, ButtonState::Pressed)], false);
    let play = host.play.as_mut().ok_or("play")?;
    assert_eq!(play.view.next_step(), Some(crate::input::StepDir::West));
    Ok(())
}

#[test]
fn accepted_logout_restarts_admission_and_clears_character_scope() -> TestResult {
    let mut host = HostOwner::new(None)?;
    let (view, link) = admitted_view([9; 16])?;
    host.install_play(view, link);
    host.finish_play(PlayEnd::LoggedOut);
    assert_eq!(host.phase, Phase::Entering);
    assert!(host.entry.as_ref().is_some_and(Reentry::is_entering));
    assert!(host.play.is_none());
    assert_eq!(host.settings.character_scope(), None);
    host.cancel_login();
    Ok(())
}

#[test]
fn admitted_bulletin_is_presented_without_losing_play() -> TestResult {
    let mut host = HostOwner::new(None)?;
    host.finish_admission(
        Ok(admitted_view([9; 16])?),
        Some(
            r#"{"news":[{"title":"Operator notice","body":"Published announcement"}],"events":[]}"#
                .into(),
        ),
    );
    assert_eq!(host.phase, Phase::Playing);
    assert!(host.play.is_some());
    assert!(host.status.contains("Operator notice"));
    assert!(host.status.contains("Published announcement"));
    assert!(host.bulletin.news().contains("Published announcement"));
    Ok(())
}

#[test]
fn cardinal_keypad_routes_through_gameplay_focus_gates() -> TestResult {
    use crate::input::StepDir;
    use oteryn_input_actions::{ButtonState, KeyCode, Modifiers, NormalizedInputEvent};
    for (code, direction) in [
        (96, StepDir::North),
        (94, StepDir::East),
        (90, StepDir::South),
        (92, StepDir::West),
    ] {
        let mut host = HostOwner::new(None)?;
        let (view, _) = admitted_view([9; 16])?;
        let (link, _, _) = crate::play::play_channel();
        host.install_play(view, link);
        let key = NormalizedInputEvent::Key {
            code: KeyCode::new(code)?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::NONE,
            repeat: false,
        };
        host.open_graphics();
        host.route_input(std::slice::from_ref(&key), false);
        assert_eq!(host.play.as_mut().ok_or("play")?.view.next_step(), None);
        host.cancel();
        host.route_input(&[key], false);
        assert_eq!(
            host.play.as_mut().ok_or("play")?.view.next_step(),
            Some(direction)
        );
    }
    Ok(())
}

#[test]
fn nearby_loot_shortcut_dispatches_without_opening_panel() -> TestResult {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    let mut host = HostOwner::new(None)?;
    let (view, _) = admitted_view([9; 16])?;
    let (link, mut commands, outbox) = crate::play::play_channel();
    link.use_shared_frontend();
    outbox.bevy_quick_loot_selected(true);
    host.install_play(view, link);
    let event = NormalizedInputEvent::Key {
        code: KeyCode::new(20)?,
        state: ButtonState::Pressed,
        modifiers: Modifiers::one(Modifier::Alt),
        repeat: false,
    };
    host.open_graphics();
    host.route_input(std::slice::from_ref(&event), false);
    assert!(commands.try_recv().is_err());
    host.cancel();
    host.route_input(&[event], false);
    let crate::combat_input::PlayCommand::ServerMechanic(command) = commands.try_recv()? else {
        return Err("loot command missing".into());
    };
    assert_eq!(command.capability, 98);
    assert!(
        !host
            .play
            .as_ref()
            .ok_or("play")?
            .link
            .discovery_panel()
            .loot
            .open
    );
    Ok(())
}

#[test]
fn holding_a_spell_key_does_not_queue_unacknowledged_casts() -> TestResult {
    use crate::combat_input::{CombatCommand, PlayCommand};
    use oteryn_input_actions::{ButtonState, KeyCode, Modifiers, NormalizedInputEvent};

    let mut host = HostOwner::new(None)?;
    let (view, _) = admitted_view([9; 16])?;
    let (link, mut commands, _) = crate::play::play_channel();
    assert!(link.spell_bar_bind(&["71", "", "", ""].map(str::to_owned)));
    host.install_play(view, link);
    let key = |repeat| -> Result<_, oteryn_input_actions::InputError> {
        Ok(NormalizedInputEvent::Key {
            code: KeyCode::new(30)?,
            state: ButtonState::Pressed,
            modifiers: Modifiers::NONE,
            repeat,
        })
    };
    for repeat in std::iter::once(false).chain([true; 8]) {
        host.route_input(&[key(repeat)?], false);
        let play = host.play.as_mut().ok_or("play")?;
        play.view
            .tick(&play.link)
            .map_err(|end| format!("{end:?}"))?;
    }
    assert!(
        matches!(commands.try_recv()?, PlayCommand::Combat(CombatCommand::Cast(id)) if id.get() == 71)
    );
    assert!(commands.try_recv().is_err());
    Ok(())
}
