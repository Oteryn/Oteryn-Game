use super::{
    super::owner::{ActivePlay, HostOwner, Phase},
    overlay::{self, DiagnosticsText, Overlay, OverlayBody},
    *,
};
use bevy::{
    ecs::error::{BevyError, ErrorContext, Severity},
    utils::prelude::DebugName,
    window::{PrimaryWindow, WindowResolution},
};
use std::time::Duration;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// The diagnostics systems over a real host owner, a primary window and real time, without a
/// renderer or the event loop.
fn app(preferences: Option<std::path::PathBuf>) -> Result<App, Box<dyn std::error::Error>> {
    let mut app = App::new();
    app.insert_non_send(HostOwner::new(preferences)?)
        .insert_resource(Time::<Real>::default())
        .insert_resource(UiScale(1.25));
    app.world_mut().spawn((
        Window {
            resolution: WindowResolution::new(1024, 720),
            ..default()
        },
        PrimaryWindow,
    ));
    install(&mut app);
    Ok(app)
}

fn overlay_display(app: &mut App) -> Result<Display, Box<dyn std::error::Error>> {
    let mut panels = app.world_mut().query_filtered::<&Node, With<Overlay>>();
    Ok(panels.single(app.world())?.display)
}

/// Ctrl+Shift+D is the depot search default, so the overlay action has no default chord; the
/// player assigns one. These tests assign Ctrl+Shift+F9.
fn bind_overlay_hotkey(owner: &mut HostOwner) -> Result<(), Box<dyn std::error::Error>> {
    let mut preferences = owner.settings.preferences.clone();
    preferences.assign_custom_hotkey(
        "CTRL+SHIFT+F9",
        crate::play::CustomAction::Utility {
            action: crate::play::UtilityAction::ToggleDiagnosticsOverlay,
        },
    )?;
    owner.settings.apply(preferences);
    Ok(())
}

/// The assigned chord through the real input route, pressed and released.
fn press_overlay_hotkey(owner: &mut HostOwner) -> Result<(), Box<dyn std::error::Error>> {
    use oteryn_input_actions::{ButtonState, KeyCode, Modifier, Modifiers, NormalizedInputEvent};
    for state in [ButtonState::Pressed, ButtonState::Released] {
        owner.route_input(
            &[NormalizedInputEvent::Key {
                // The USB HID usage of F9.
                code: KeyCode::new(66)?,
                state,
                modifiers: Modifiers::one(Modifier::Control).with(Modifier::Shift),
                repeat: false,
            }],
            false,
        );
    }
    Ok(())
}

fn play_on(owner: &mut HostOwner) -> Result<crate::play::Outbox, Box<dyn std::error::Error>> {
    let (link, _commands, outbox) = crate::play::play_channel();
    let mut view = crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?;
    view.use_shared_frontend();
    link.use_shared_frontend();
    owner.play = Some(ActivePlay { view, link });
    owner.phase = Phase::Playing;
    Ok(outbox)
}

#[test]
fn the_overlay_follows_the_saved_option_and_is_hidden_by_default() -> TestResult {
    let mut app = app(None)?;
    app.update();
    assert_eq!(overlay_display(&mut app)?, Display::None);
    let mut owner = app.world_mut().non_send_mut::<HostOwner>();
    let mut preferences = owner.settings.preferences.clone();
    preferences.diagnostics.overlay = true;
    owner.settings.apply(preferences);
    app.update();
    assert_eq!(overlay_display(&mut app)?, Display::Flex);
    Ok(())
}

#[test]
fn the_overlay_utility_hotkey_toggles_and_persists_the_choice() -> TestResult {
    let fixture = crate::play::screenshot::TestDir::new()?;
    let path = fixture.join("client-preferences.json");
    let mut owner = HostOwner::new(Some(path.clone()))?;
    let _outbox = play_on(&mut owner)?;
    bind_overlay_hotkey(&mut owner)?;
    assert!(!owner.settings.preferences.diagnostics.overlay);
    press_overlay_hotkey(&mut owner)?;
    assert!(owner.settings.preferences.diagnostics.overlay);
    assert!(
        crate::play::PreferenceStore::read_transfer(&path)?
            .diagnostics
            .overlay
    );
    // A new owner over the same file starts with the overlay shown.
    let reopened = HostOwner::new(Some(path.clone()))?;
    assert!(reopened.settings.preferences.diagnostics.overlay);
    drop(reopened);
    press_overlay_hotkey(&mut owner)?;
    assert!(!owner.settings.preferences.diagnostics.overlay);
    assert!(
        !crate::play::PreferenceStore::read_transfer(&path)?
            .diagnostics
            .overlay
    );
    drop(owner);
    Ok(())
}

#[test]
fn the_overlay_hotkey_does_not_fire_while_the_chat_input_has_focus() -> TestResult {
    let mut owner = HostOwner::new(None)?;
    let _outbox = play_on(&mut owner)?;
    bind_overlay_hotkey(&mut owner)?;
    let link = &owner.play.as_ref().ok_or("play")?.link;
    link.open_chat();
    // Cycle the composer to typing mode, the state in which the chat input has focus.
    for _ in 0..3 {
        if link.preference_chat_open() {
            break;
        }
        link.chat_console_mode();
    }
    assert!(link.preference_chat_open());
    press_overlay_hotkey(&mut owner)?;
    assert!(!owner.settings.preferences.diagnostics.overlay);
    Ok(())
}

#[test]
fn the_overlay_panel_follows_the_hotkey() -> TestResult {
    let mut app = app(None)?;
    app.update();
    assert_eq!(overlay_display(&mut app)?, Display::None);
    let _outbox = play_on(&mut app.world_mut().non_send_mut::<HostOwner>())?;
    bind_overlay_hotkey(&mut app.world_mut().non_send_mut::<HostOwner>())?;
    press_overlay_hotkey(&mut app.world_mut().non_send_mut::<HostOwner>())?;
    app.update();
    assert_eq!(overlay_display(&mut app)?, Display::Flex);
    Ok(())
}

#[test]
fn overlay_lines_come_from_the_pacer_window_scale_owner_and_log() -> TestResult {
    let mut app = app(None)?;
    // Five frames 250 ms apart, the last one just now, through the frame pacer.
    let mut pacer = crate::play::PerformancePacer::default();
    let now = std::time::Instant::now();
    for step in (0..=4u64).rev() {
        pacer.presented(
            now.checked_sub(Duration::from_millis(step * 250))
                .ok_or("clock")?,
        );
    }
    app.insert_non_send(pacer);
    {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        let mut preferences = owner.settings.preferences.clone();
        preferences.diagnostics.overlay = true;
        preferences.diagnostics.log_level = 2;
        owner.settings.apply(preferences);
    }
    app.insert_resource(LastCrash(Some("crashes/crash-x.log".into())));
    app.update();
    let text = app.world().resource::<DiagnosticsText>().0.clone();
    let lines: Vec<&str> = text.lines().collect();
    assert!(
        lines.contains(&"FPS 4  frame avg 250.0 ms  max 250.0 ms"),
        "{text}"
    );
    assert!(
        text.contains("Window 1024x720  scale 1.00  UI 125%"),
        "{text}"
    );
    assert!(text.contains("Connection offline  (login)"), "{text}");
    assert!(text.contains("Log debug  warnings "), "{text}");
    assert!(
        text.contains(&format!("Build {}", crate::CLIENT_BUILD)),
        "{text}"
    );
    assert!(
        text.contains("The client crashed last time; log saved at crashes/crash-x.log"),
        "{text}"
    );
    let entities = lines
        .iter()
        .find_map(|line| line.strip_prefix("Entities "))
        .ok_or("entity line")?
        .parse::<u32>()?;
    assert!(entities >= 3, "panel, text and window: {entities}");
    // Without play, a renderer or an adapter, those lines are left out, not invented.
    for absent in [
        "Position",
        "Character",
        "World:",
        "Renderer",
        "Command round trip",
    ] {
        assert!(!text.contains(absent), "{absent} in {text}");
    }
    let mut bodies = app.world_mut().query_filtered::<&Text, With<OverlayBody>>();
    assert_eq!(bodies.single(app.world())?.0, text);
    Ok(())
}

#[test]
fn dock_content_shows_the_same_text_while_the_panel_is_hidden() -> TestResult {
    let mut app = app(None)?;
    app.update();
    let docked = content(&mut app.world_mut().commands());
    app.world_mut().flush();
    app.world_mut()
        .resource_mut::<Time<Real>>()
        .update_with_duration(overlay::REFRESH);
    app.update();
    assert_eq!(overlay_display(&mut app)?, Display::None);
    let text = app.world().resource::<DiagnosticsText>().0.clone();
    assert!(!text.is_empty());
    assert_eq!(
        app.world().get::<Text>(docked).ok_or("docked text")?.0,
        text
    );
    Ok(())
}

#[test]
fn the_text_refreshes_at_most_twice_a_second() -> TestResult {
    let mut app = app(None)?;
    app.update();
    let first = app.world().resource::<DiagnosticsText>().0.clone();
    assert!(first.contains("Connection offline"), "{first}");
    app.world_mut().non_send_mut::<HostOwner>().phase = Phase::Closed;
    app.world_mut()
        .resource_mut::<Time<Real>>()
        .update_with_duration(Duration::from_millis(100));
    app.update();
    assert_eq!(app.world().resource::<DiagnosticsText>().0, first);
    app.world_mut()
        .resource_mut::<Time<Real>>()
        .update_with_duration(overlay::REFRESH);
    app.update();
    let next = &app.world().resource::<DiagnosticsText>().0;
    assert!(next.contains("Connection closed  (closing)"), "{next}");
    Ok(())
}

#[test]
fn copied_diagnostics_hold_the_overlay_and_the_last_fifty_redacted_lines() {
    for index in 0..60 {
        crate::crash_report::record_log_line(
            crate::crash_report::LineLevel::Other,
            &format!("copy test line {index}"),
        );
    }
    let text = overlay::clipboard_text("FPS 60\nAuthorization: Bearer secret-value");
    let clean = overlay::clipboard_text("FPS 60");
    assert!(clean.starts_with("Oteryn client diagnostics\nFPS 60\n"));
    assert!(text.starts_with(
        "Oteryn client diagnostics\n[record redacted: possible credential]\n\nLast log lines:\n"
    ));
    assert!(!text.contains("secret-value"));
    let copied = text
        .split("Last log lines:\n")
        .nth(1)
        .map_or(0, |lines| lines.lines().count());
    assert_eq!(copied, overlay::COPIED_LOG_LINES);
}

#[test]
fn the_log_folder_opens_as_one_argument_without_a_shell() {
    let folder = std::path::Path::new("/tmp/Oteryn logs; rm -rf x");
    let command = overlay::folder_command(folder);
    let arguments: Vec<_> = command.get_args().collect();
    assert_eq!(arguments, [folder.as_os_str()]);
}

#[test]
fn the_crash_flag_names_the_saved_log() {
    assert_eq!(LastCrash::default().message(), None);
    assert_eq!(
        LastCrash(Some("/data/crashes/crash-1.log".into())).message(),
        Some("The client crashed last time; log saved at /data/crashes/crash-1.log".into())
    );
}

#[test]
fn a_panicking_ecs_error_is_logged_with_its_context_and_still_panics() -> TestResult {
    let fixture = crate::play::screenshot::TestDir::new()?;
    let directory = fixture.join("logs");
    let file = std::sync::Arc::new(logging::RollingFile::new(
        directory.clone(),
        logging::KEEP_FILES,
        logging::TOTAL_CAP,
    ));
    let subscriber = logging::test_subscriber(&file, "info");
    let outcome = bevy::log::tracing::subscriber::with_default(subscriber, || {
        // A warning-severity error is logged by Bevy and does not panic.
        ecs_error(
            BevyError::new(Severity::Warning, "soft"),
            ErrorContext::Command {
                name: DebugName::borrowed("Soft"),
            },
        );
        std::panic::catch_unwind(|| {
            ecs_error(
                BevyError::new(Severity::Panic, "Entity despawned"),
                ErrorContext::Command {
                    name: DebugName::borrowed("Despawn"),
                },
            );
        })
    });
    assert!(outcome.is_err(), "the default stays fail-fast");
    let text = std::fs::read_dir(&directory)?
        .filter_map(Result::ok)
        .map(|entry| std::fs::read_to_string(entry.path()))
        .collect::<Result<String, _>>()?;
    assert!(text.contains("ECS command `"), "{text}");
    assert!(text.contains("failed: Entity despawned"), "{text}");
    assert!(text.contains(" ERROR "), "{text}");
    std::fs::remove_dir_all(&directory)?;
    assert!(!directory.with_added_extension("lock").exists());
    Ok(())
}

#[test]
fn the_traffic_line_shows_session_bytes_each_way() {
    let snapshot = overlay::Snapshot {
        traffic: Some((2048, 512)),
        ..Default::default()
    };
    assert!(
        snapshot
            .lines()
            .contains(&"Traffic out 2.0 KiB  in 0.5 KiB".to_owned()),
        "{:?}",
        snapshot.lines()
    );
    let offline = overlay::Snapshot::default().lines();
    assert!(
        !offline.iter().any(|line| line.starts_with("Traffic")),
        "{offline:?}"
    );
}
