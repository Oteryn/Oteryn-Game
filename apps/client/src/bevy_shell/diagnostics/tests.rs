use super::{
    super::{game_ui::GameModal, owner::HostOwner},
    overlay::{self, Overlay, OverlayBody, OverlayText},
    *,
};
use bevy::{
    ecs::error::{BevyError, ErrorContext, Severity},
    utils::prelude::DebugName,
    window::{PrimaryWindow, WindowResolution},
};
use std::{
    sync::atomic::{AtomicU64, Ordering},
    time::Duration,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;
static NEXT: AtomicU64 = AtomicU64::new(0);

fn temp_preferences() -> std::path::PathBuf {
    std::env::temp_dir().join(format!(
        "oteryn-diagnostics-{}-{}/client-preferences.json",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}

/// The diagnostics plugin over a real host owner, a primary window and real time, without a
/// renderer or the event loop.
fn app(preferences: Option<std::path::PathBuf>) -> Result<App, Box<dyn std::error::Error>> {
    let mut app = App::new();
    app.insert_non_send(HostOwner::new(preferences)?)
        .insert_resource(Time::<Real>::default())
        .insert_resource(UiScale(1.25))
        .init_resource::<ButtonInput<KeyCode>>();
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

fn press_ctrl_shift_d(app: &mut App) {
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keys.press(KeyCode::ControlLeft);
    keys.press(KeyCode::ShiftRight);
    keys.press(KeyCode::KeyD);
    app.update();
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keys.release_all();
    keys.clear();
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
fn ctrl_shift_d_toggles_the_overlay_and_persists_the_choice() -> TestResult {
    let path = temp_preferences();
    let mut app = app(Some(path.clone()))?;
    app.update();
    assert_eq!(overlay_display(&mut app)?, Display::None);
    // D alone, or Ctrl+D, is not the chord.
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::KeyD);
    app.update();
    assert_eq!(overlay_display(&mut app)?, Display::None);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .release_all();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .clear();

    press_ctrl_shift_d(&mut app);
    assert_eq!(overlay_display(&mut app)?, Display::Flex);
    let saved = crate::play::PreferenceStore::read_transfer(&path)?;
    assert!(saved.diagnostics.overlay);
    // A new owner over the same file starts with the overlay shown.
    let reopened = HostOwner::new(Some(path.clone()))?;
    assert!(reopened.settings.preferences.diagnostics.overlay);
    drop(reopened);

    press_ctrl_shift_d(&mut app);
    assert_eq!(overlay_display(&mut app)?, Display::None);
    assert!(
        !crate::play::PreferenceStore::read_transfer(&path)?
            .diagnostics
            .overlay
    );
    drop(app);
    if let Some(directory) = path.parent() {
        std::fs::remove_dir_all(directory)?;
    }
    Ok(())
}

#[test]
fn overlay_lines_come_from_the_window_scale_owner_modals_and_frame_time() -> TestResult {
    let mut app = app(None)?;
    app.world_mut()
        .spawn((GameModal, Name::new("options"), Node::default()));
    app.world_mut().spawn((
        GameModal,
        Name::new("closed dialog"),
        Node {
            display: Display::None,
            ..default()
        },
    ));
    {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        let mut preferences = owner.settings.preferences.clone();
        preferences.diagnostics.overlay = true;
        preferences.diagnostics.log_level = 2;
        owner.settings.apply(preferences);
    }
    app.insert_resource(LastCrash(Some("crashes/crash-x.log".into())));
    // Two 20 ms frames, then enough time for the next refresh.
    for step in [20, 20, 600] {
        app.world_mut()
            .resource_mut::<Time<Real>>()
            .update_with_duration(Duration::from_millis(step));
        app.update();
    }
    let text = app.world().resource::<OverlayText>().0.clone();
    let lines: Vec<&str> = text.lines().collect();
    assert!(
        text.contains("Window 1024x720  scale 1.00  UI 125%"),
        "{text}"
    );
    assert!(text.contains("Connection offline  (login)"), "{text}");
    assert!(text.contains("Open windows 1"), "{text}");
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
    assert!(
        entities >= 7,
        "panel, caption, body, grip, window and two modals: {entities}"
    );
    assert!(
        lines
            .iter()
            .any(|line| line.starts_with("FPS ") && line.contains("max 600.0 ms")),
        "{text}"
    );
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
fn the_overlay_refreshes_at_most_twice_a_second() -> TestResult {
    let mut app = app(None)?;
    app.update();
    let first = app.world().resource::<OverlayText>().0.clone();
    assert!(!first.is_empty());
    app.world_mut().spawn((GameModal, Node::default()));
    app.world_mut()
        .resource_mut::<Time<Real>>()
        .update_with_duration(Duration::from_millis(100));
    app.update();
    assert_eq!(app.world().resource::<OverlayText>().0, first);
    app.world_mut()
        .resource_mut::<Time<Real>>()
        .update_with_duration(overlay::REFRESH);
    app.update();
    assert!(
        app.world()
            .resource::<OverlayText>()
            .0
            .contains("Open windows 1")
    );
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
    assert!(text.starts_with("Oteryn client diagnostics\nFPS 60\n"));
    assert!(text.contains("Authorization: Bearer [redacted]"));
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
    let directory = std::env::temp_dir().join(format!(
        "oteryn-ecs-error-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let file = std::sync::Arc::new(logging::RollingFile::new(
        directory.clone(),
        logging::KEEP_FILES,
        logging::TOTAL_CAP,
    ));
    let subscriber = logging::test_subscriber(&file);
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
    std::fs::remove_dir_all(directory)?;
    Ok(())
}
