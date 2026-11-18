use super::*;
use crate::play::screenshot::TestDir;
#[cfg(unix)]
use crate::play::screenshot::{assert_one_test_passed, one_test_passed};
use bevy::{
    asset::RenderAssetUsages,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn frame() -> Image {
    colored_frame([20, 40, 60, 255])
}
fn colored_frame(color: [u8; 4]) -> Image {
    Image::new_fill(
        Extent3d {
            width: 4,
            height: 3,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &color,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}
fn app(folder: &std::path::Path, game_only: bool) -> Result<App, Box<dyn std::error::Error>> {
    let mut owner = HostOwner::new(None)?;
    owner.settings.preferences.screenshots.directory = folder.to_string_lossy().into_owned();
    owner.settings.preferences.screenshots.game_only = game_only;
    let mut app = App::new();
    app.insert_non_send(owner)
        .insert_resource(super::super::audit::AuditOutput::new(None))
        .init_resource::<ButtonInput<KeyCode>>()
        .init_resource::<Time<Real>>()
        .insert_resource(WorldViewport {
            origin: UVec2::ONE,
            size: UVec2::new(2, 1),
        })
        .add_systems(Update, super::super::audit::capture_on_key);
    app.world_mut()
        .resource_mut::<Time<Real>>()
        .update_with_instant(std::time::Instant::now());
    Ok(app)
}
fn capture(app: &mut App) -> TestResult {
    capture_frame(app, frame())
}
fn capture_frame(app: &mut App, image: Image) -> TestResult {
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::F12);
    app.update();
    let entity = app
        .world_mut()
        .query_filtered::<Entity, With<Screenshot>>()
        .single(app.world())?;
    app.world_mut()
        .trigger(ScreenshotCaptured { entity, image });
    app.world_mut().despawn(entity);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset_all();
    Ok(())
}
#[test]
fn f12_completion_saves_distinct_bmps_in_configured_destination() -> TestResult {
    let fixture = TestDir::new()?;
    let root = fixture.path().to_path_buf();
    let mut app = app(&root, false)?;
    capture(&mut app)?;
    capture(&mut app)?;
    let files = std::fs::read_dir(&root)?.collect::<Result<Vec<_>, _>>()?;
    assert_eq!(files.len(), 2);
    for file in files {
        let bytes = std::fs::read(file.path())?;
        assert_eq!(&bytes[..2], b"BM");
        assert_eq!(i32::from_le_bytes(bytes[18..22].try_into()?), 4);
        assert_eq!(i32::from_le_bytes(bytes[22..26].try_into()?), 3);
        assert_eq!(&bytes[54..57], &[60, 40, 20]);
    }
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .status
            .starts_with("Screenshot saved:")
    );
    Ok(())
}
#[test]
fn game_only_crops_rendered_pixels_and_write_errors_reach_player() -> TestResult {
    let fixture = TestDir::new()?;
    let root = fixture.path().to_path_buf();
    let mut app = app(&root, true)?;
    capture(&mut app)?;
    let path = std::fs::read_dir(&root)?.next().ok_or("capture")??.path();
    let bytes = std::fs::read(path)?;
    assert_eq!(i32::from_le_bytes(bytes[18..22].try_into()?), 2);
    assert_eq!(i32::from_le_bytes(bytes[22..26].try_into()?), 1);
    let blocked = root.join("file");
    std::fs::write(&blocked, b"occupied")?;
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .settings
        .preferences
        .screenshots
        .directory = blocked.to_string_lossy().into_owned();
    capture(&mut app)?;
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .status
            .starts_with("Screenshot failed:")
    );
    assert_eq!(std::fs::read(blocked)?, b"occupied");
    assert!(
        save(
            &frame(),
            root.to_str().ok_or("capture directory")?,
            Some(WorldViewport::default())
        )
        .is_err()
    );
    Ok(())
}

#[test]
fn death_event_captures_without_f12_and_deduplicates() -> TestResult {
    let fixture = TestDir::new()?;
    let root = fixture.join("captures");
    capture_death(&root, false)?;
    assert_eq!(std::fs::read_dir(&root)?.count(), 1);
    Ok(())
}

fn capture_death(destination: &std::path::Path, backlog: bool) -> TestResult {
    let mut app = app(destination, true)?;
    let (link, _, outbox) = crate::play::play_channel();
    {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        owner.settings.preferences.screenshots.on_death = true;
        owner.settings.preferences.screenshots.backlog_frames = u8::from(backlog);
        owner.play = Some(crate::bevy_shell::owner::ActivePlay {
            link,
            view: crate::play::PlayView::new(oteryn_renderer::TileCoord::new(0, 0), 0, &[])?,
        });
    }
    outbox.push(vec![oteryn_session::SessionEvent::ActorVitals(
        oteryn_session::AppliedDelta {
            server_sequence: 1,
            base_revision: 0,
            new_revision: 1,
            value: oteryn_session::ActorVitals {
                health: 100,
                max_health: 100,
                ..default()
            },
        },
    )]);
    app.update();
    assert_eq!(
        app.world_mut()
            .query::<&Screenshot>()
            .iter(app.world())
            .count(),
        usize::from(backlog)
    );
    if backlog {
        let entity = app
            .world_mut()
            .query_filtered::<Entity, With<Screenshot>>()
            .single(app.world())?;
        app.world_mut().trigger(ScreenshotCaptured {
            entity,
            image: frame(),
        });
        app.world_mut().despawn(entity);
    }
    outbox.push(vec![oteryn_session::SessionEvent::ActorVitals(
        oteryn_session::AppliedDelta {
            server_sequence: 2,
            base_revision: 1,
            new_revision: 2,
            value: oteryn_session::ActorVitals {
                health: 0,
                max_health: 100,
                ..default()
            },
        },
    )]);
    app.update();
    let entity = app
        .world_mut()
        .query_filtered::<Entity, With<Screenshot>>()
        .single(app.world())?;
    app.world_mut().trigger(ScreenshotCaptured {
        entity,
        image: frame(),
    });
    app.world_mut().despawn(entity);
    app.update();
    assert_eq!(
        app.world().non_send::<HostOwner>().status,
        format!("Saved {} screenshot(s)", 1 + usize::from(backlog))
    );
    assert_eq!(
        app.world_mut()
            .query::<&Screenshot>()
            .iter(app.world())
            .count(),
        0
    );
    // At the next backlog interval the request is for buffering only.
    if backlog {
        let mut clock = app.world_mut().resource_mut::<Time<Real>>();
        let now = clock.last_update().ok_or("capture clock")?;
        clock.update_with_instant(now + std::time::Duration::from_secs(1));
        app.update();
        let entity = app
            .world_mut()
            .query_filtered::<Entity, With<Screenshot>>()
            .single(app.world())?;
        app.world_mut().trigger(ScreenshotCaptured {
            entity,
            image: frame(),
        });
        assert_eq!(
            app.world().non_send::<HostOwner>().status,
            "Saved 2 screenshot(s)"
        );
    }
    Ok(())
}

// Libtest accepts a skip pattern that matches no test. Only an explicit
// subprocess invocation can select these cases. The parent owns the fixture.
#[cfg(unix)]
fn child_mode(marker: &str) -> bool {
    let mut args = std::env::args_os();
    while let Some(arg) = args.next() {
        if arg == "--skip" && args.next().is_some_and(|arg| arg == marker) {
            return true;
        }
    }
    false
}

#[cfg(unix)]
fn child_root() -> io::Result<PathBuf> {
    let root = std::env::current_dir()?;
    if std::env::temp_dir() != root {
        return Err(io::Error::other(
            "Capture subprocess must use its test fixture",
        ));
    }
    Ok(root)
}

#[cfg(unix)]
#[test]
fn f12_without_config_home_saves_capture() -> TestResult {
    const CHILD: &str = "oteryn-test-f12-child";
    if !child_mode(CHILD)
        || crate::play::PreferenceStore::path().is_some()
        || std::env::var_os("LOCALAPPDATA").is_some()
    {
        let fixture = TestDir::new()?;
        let root = fixture.path().to_path_buf();
        let output = std::process::Command::new("sh")
            .args(["-c", r#"umask 022; exec "$@""#, "sh"])
            .arg(std::env::current_exe()?)
            .args([
                "--exact",
                "bevy_shell::screenshots::tests::f12_without_config_home_saves_capture",
            ])
            .args(["--skip", CHILD])
            .current_dir(&root)
            .env("TMPDIR", &root)
            .env_remove("HOME")
            .env_remove("LOCALAPPDATA")
            .env_remove("XDG_CONFIG_HOME")
            .output()?;
        assert_one_test_passed(&output);
        return Ok(());
    }
    let root = child_root()?;
    let mut app = app(std::path::Path::new(""), false)?;
    capture(&mut app)?;
    let path = app
        .world()
        .non_send::<HostOwner>()
        .status
        .strip_prefix("Screenshot saved: ")
        .ok_or("capture failed")?;
    use std::os::unix::fs::PermissionsExt;
    let path = std::path::Path::new(path);
    assert!(path.is_file());
    let parent = path.parent().ok_or("capture directory")?;
    assert_eq!(parent.metadata()?.permissions().mode() & 0o077, 0);
    assert_eq!(parent.parent(), Some(root.as_path()));
    Ok(())
}

#[cfg(unix)]
#[test]
fn death_with_backlog_uses_a_private_fallback() -> TestResult {
    use std::os::unix::fs::PermissionsExt;
    const CHILD: &str = "oteryn-test-death-child";
    if child_mode(CHILD)
        && crate::play::PreferenceStore::path().is_none()
        && std::env::var_os("LOCALAPPDATA").is_none()
    {
        child_root()?;
        return capture_death(std::path::Path::new(""), true);
    }
    let fixture = TestDir::new()?;
    let root = fixture.path().to_path_buf();
    let output = std::process::Command::new("sh")
        .args(["-c", r#"umask 022; exec "$@""#, "sh"])
        .arg(std::env::current_exe()?)
        .args([
            "--exact",
            "bevy_shell::screenshots::tests::death_with_backlog_uses_a_private_fallback",
        ])
        .args(["--skip", CHILD])
        .current_dir(&root)
        .env("TMPDIR", &root)
        .env_remove("HOME")
        .env_remove("LOCALAPPDATA")
        .env_remove("XDG_CONFIG_HOME")
        .output()?;
    assert_one_test_passed(&output);
    let mut pending = vec![root.clone()];
    let mut captures = 0;
    while let Some(directory) = pending.pop() {
        for entry in std::fs::read_dir(&directory)? {
            let entry = entry?;
            if entry.file_type()?.is_dir() {
                pending.push(entry.path());
            } else if entry.path().extension().is_some_and(|ext| ext == "bmp") {
                assert_eq!(directory.metadata()?.permissions().mode() & 0o077, 0);
                assert_eq!(&std::fs::read(entry.path())?[..2], b"BM");
                captures += 1;
            }
        }
    }
    assert_eq!(captures, 2);
    Ok(())
}

#[cfg(unix)]
#[test]
fn inherited_f12_marker_preserves_existing_captures() -> TestResult {
    use std::os::unix::fs::PermissionsExt;
    let fixture = TestDir::new()?;
    let root = fixture.path().to_path_buf();
    let config = if cfg!(target_os = "macos") {
        root.join("Library/Application Support")
    } else {
        root.join(".config")
    };
    let screenshots = config.join("Oteryn/screenshots");
    std::fs::create_dir_all(&screenshots)?;
    std::fs::set_permissions(&screenshots, std::fs::Permissions::from_mode(0o700))?;
    let existing = screenshots.join("existing.bmp");
    std::fs::write(&existing, b"existing capture")?;
    let output = std::process::Command::new(std::env::current_exe()?)
        .args([
            "--exact",
            "bevy_shell::screenshots::tests::f12_without_config_home_saves_capture",
        ])
        .env("OTERYN_TEST_F12_NO_HOME", "1")
        .env("HOME", &root)
        .env("TMPDIR", &root)
        .env_remove("LOCALAPPDATA")
        .env_remove("XDG_CONFIG_HOME")
        .output();
    let preserved = std::fs::read(&existing);
    assert_one_test_passed(&output?);
    assert_eq!(preserved?, b"existing capture");
    Ok(())
}

#[cfg(unix)]
#[test]
fn inherited_capture_destination_marker_runs_the_matrix() -> TestResult {
    let fixture = TestDir::new()?;
    let root = fixture.path().to_path_buf();
    let config = if cfg!(target_os = "macos") {
        root.join("Library/Application Support")
    } else {
        root.join(".config")
    };
    let screenshots = config.join("Oteryn/screenshots");
    std::fs::create_dir_all(&screenshots)?;
    let existing = screenshots.join("existing.bmp");
    std::fs::write(&existing, b"existing capture")?;
    let output = std::process::Command::new(std::env::current_exe()?)
        .args([
            "--exact",
            "bevy_shell::screenshots::tests::manual_and_death_captures_share_private_destinations",
        ])
        .env("OTERYN_TEST_CAPTURE_DESTINATION", "default")
        .env("HOME", &root)
        .env("TMPDIR", &root)
        .env_remove("LOCALAPPDATA")
        .env_remove("XDG_CONFIG_HOME")
        .output();
    let entries = std::fs::read_dir(&screenshots)?.count();
    let preserved = std::fs::read(existing);
    assert_eq!(entries, 1, "matrix runner wrote to the inherited home");
    assert_eq!(preserved?, b"existing capture");
    let output = output?;
    assert_one_test_passed(&output);
    Ok(())
}

#[cfg(unix)]
#[test]
fn manual_and_death_captures_share_private_destinations() -> TestResult {
    use std::{os::unix::fs::PermissionsExt, process::Command};
    let fixture = TestDir::new()?;
    let root = fixture.path().to_path_buf();
    let mut failures = Vec::new();
    let cases = (0..8)
        .flat_map(|mask| [(mask, false, false), (mask, true, false)])
        .chain([(4, false, true)]);
    for (mask, explicit, legacy) in cases {
        let case_fixture = tempfile::tempdir_in(&root)?;
        let case = case_fixture.path();
        let shared = case.join("LOCALAPPDATA/Oteryn/screenshots");
        if legacy {
            std::fs::create_dir_all(&shared)?;
            std::fs::set_permissions(&shared, std::fs::Permissions::from_mode(0o755))?;
            let previous = shared.join("legacy.bmp");
            std::fs::write(&previous, b"BM legacy capture")?;
            std::fs::set_permissions(previous, std::fs::Permissions::from_mode(0o644))?;
        } else if mask == 4 && !explicit {
            // Block the old shared destination even when the test runs as root.
            std::fs::create_dir_all(case.join("LOCALAPPDATA/Oteryn"))?;
            std::fs::write(&shared, b"occupied")?;
        }
        let mut child = Command::new("sh");
        child
            .args(["-c", r#"umask 022; exec "$@""#, "sh"])
            .arg(std::env::current_exe()?)
            .args(["--exact", CAPTURE_DESTINATION_TEST])
            .args([
                "--skip",
                if explicit {
                    CAPTURE_EXPLICIT
                } else {
                    CAPTURE_DEFAULT
                },
            ])
            .current_dir(case)
            .env("TMPDIR", case)
            .env(CAPTURE_ROOT, case);
        for (bit, name) in ["HOME", "XDG_CONFIG_HOME", "LOCALAPPDATA"]
            .into_iter()
            .enumerate()
        {
            if mask & (1 << bit) == 0 {
                child.env_remove(name);
            } else {
                child.env(name, case.join(name));
            }
        }
        let output = child.output()?;
        if !one_test_passed(&output) {
            failures.push((
                mask,
                explicit,
                legacy,
                String::from_utf8_lossy(&output.stdout).into_owned(),
                String::from_utf8_lossy(&output.stderr).into_owned(),
            ));
        }
        if legacy {
            assert_eq!(shared.metadata()?.permissions().mode() & 0o777, 0o755);
            assert_eq!(
                std::fs::read(shared.join("legacy.bmp"))?,
                b"BM legacy capture"
            );
            assert_eq!(std::fs::read_dir(shared)?.count(), 1);
        }
    }
    assert!(
        failures.is_empty(),
        "failed destination cases: {failures:?}"
    );
    Ok(())
}

#[cfg(unix)]
const CAPTURE_ROOT: &str = "OTERYN_TEST_CAPTURE_ROOT";
#[cfg(unix)]
const CAPTURE_DEFAULT: &str = "oteryn-test-capture-default";
#[cfg(unix)]
const CAPTURE_EXPLICIT: &str = "oteryn-test-capture-explicit";
#[cfg(unix)]
const CAPTURE_DESTINATION_TEST: &str =
    "bevy_shell::screenshots::tests::capture_destination_matrix_case";

#[cfg(unix)]
#[test]
fn direct_capture_destination_case_preserves_existing_chosen_directory() -> TestResult {
    let fixture = TestDir::new()?;
    let chosen = fixture.join("chosen");
    std::fs::create_dir(&chosen)?;
    let sentinel = chosen.join("existing.bmp");
    std::fs::write(&sentinel, b"existing capture")?;
    let output = std::process::Command::new(std::env::current_exe()?)
        .args([
            "--exact",
            CAPTURE_DESTINATION_TEST,
            "--skip",
            CAPTURE_EXPLICIT,
        ])
        .current_dir(fixture.path())
        .env("TMPDIR", fixture.path())
        .env_remove("HOME")
        .env_remove("XDG_CONFIG_HOME")
        .env_remove("LOCALAPPDATA")
        .output()?;
    assert_eq!(std::fs::read_dir(chosen)?.count(), 1);
    assert_eq!(std::fs::read(sentinel)?, b"existing capture");
    assert_one_test_passed(&output);
    Ok(())
}

#[cfg(unix)]
#[test]
fn capture_destination_matrix_case() -> TestResult {
    use std::{os::unix::fs::PermissionsExt, path::Path};
    if !std::env::args_os().any(|arg| arg == CAPTURE_DESTINATION_TEST) {
        return Ok(());
    }
    let explicit = child_mode(CAPTURE_EXPLICIT);
    if !explicit && !child_mode(CAPTURE_DEFAULT) {
        return Ok(());
    }
    let root = child_root()?;
    if !explicit && std::env::var_os(CAPTURE_ROOT).as_deref() != Some(root.as_os_str()) {
        return Err("Default capture case requires a parent fixture".into());
    }
    let explicit_fixture = if explicit {
        Some(TestDir::new()?)
    } else {
        None
    };
    for name in ["HOME", "XDG_CONFIG_HOME", "LOCALAPPDATA"] {
        if let Some(path) = std::env::var_os(name) {
            assert_eq!(Path::new(&path), root.join(name));
        }
    }
    let chosen = explicit_fixture
        .as_ref()
        .map(|fixture| fixture.join("chosen"));
    let destination = chosen.as_deref().unwrap_or_else(|| Path::new(""));
    let mut manual = app(destination, false)?;
    capture(&mut manual)?;
    let status = &manual.world().non_send::<HostOwner>().status;
    let path = Path::new(
        status
            .strip_prefix("Screenshot saved: ")
            .ok_or(status.as_str())?,
    );
    let folder = path.parent().ok_or("capture directory")?;
    if explicit {
        assert_eq!(folder, destination);
    } else if let Some(config) = crate::play::PreferenceStore::path() {
        assert_eq!(folder, config.with_file_name("screenshots"));
    } else {
        assert_eq!(folder.parent(), Some(root.as_path()));
    }
    capture_death(destination, true)?;
    let files = std::fs::read_dir(folder)?.collect::<Result<Vec<_>, _>>()?;
    assert_eq!(
        files.len(),
        3,
        "expected F12, death and backlog in {folder:?}"
    );
    assert_eq!(folder.metadata()?.permissions().mode() & 0o777, 0o700);
    for file in files {
        assert_eq!(file.metadata()?.permissions().mode() & 0o777, 0o600);
        assert_eq!(&std::fs::read(file.path())?[..2], b"BM");
    }
    if !explicit && std::env::var_os("HOME").is_some() {
        use crate::screenshots::{CaptureBacklog, CaptureController, Frame, Rect};
        let mut capture = CaptureController::default();
        capture.manual();
        let paths = CaptureBacklog::default().process(
            &mut capture,
            &crate::play::ScreenshotPreferences::default(),
            &root.join("bgra-fallback"),
            Frame {
                width: 1,
                height: 1,
                bgra: &[60, 40, 20, 255],
                game: Rect {
                    x: 0,
                    y: 0,
                    width: 1,
                    height: 1,
                },
            },
            std::time::Instant::now(),
        )?;
        assert_eq!(paths.len(), 1);
        assert_eq!(paths[0].parent(), Some(folder));
        assert_eq!(paths[0].metadata()?.permissions().mode() & 0o777, 0o600);
        assert_eq!(&std::fs::read(&paths[0])?[..2], b"BM");
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn audit_f12_replaces_png_with_private_capture() -> TestResult {
    use std::os::unix::fs::PermissionsExt;
    let fixture = TestDir::new()?;
    let root = fixture.path().to_path_buf();
    let output = root.join("new").join("audit.json");
    let mut app = app(&root, false)?;
    app.insert_resource(super::super::audit::AuditOutput::new(Some(output.clone())));
    capture(&mut app)?;
    let path = output.with_extension("png");
    let first = std::fs::read(&path)?;
    assert_eq!(&first[..8], b"\x89PNG\r\n\x1a\n");
    let second_frame = colored_frame([80, 100, 120, 255]);
    capture_frame(&mut app, second_frame.clone())?;
    let second = std::fs::read(&path)?;
    assert_eq!(&second[..8], b"\x89PNG\r\n\x1a\n");
    assert_ne!(second, first);
    let mut expected = std::io::Cursor::new(Vec::new());
    let format = bevy::image::ImageFormat::Png
        .as_image_crate_format()
        .ok_or("PNG format")?;
    second_frame
        .try_into_dynamic()?
        .into_rgb8()
        .write_to(&mut expected, format)?;
    assert_eq!(second, expected.into_inner());
    assert_eq!(path.metadata()?.permissions().mode() & 0o777, 0o600);
    let folder = path.parent().ok_or("capture directory")?;
    assert_eq!(folder.metadata()?.permissions().mode() & 0o777, 0o700);
    assert_eq!(std::fs::read_dir(folder)?.count(), 1);
    Ok(())
}

#[cfg(unix)]
#[test]
fn f12_preserves_legacy_capture_in_existing_directory() -> TestResult {
    use std::os::unix::fs::PermissionsExt;
    let fixture = TestDir::new()?;
    let root = fixture.path().to_path_buf();
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o755))?;
    let legacy = root.join("oteryn-1-2.bmp");
    std::fs::write(&legacy, b"BM legacy capture")?;
    std::fs::set_permissions(&legacy, std::fs::Permissions::from_mode(0o644))?;
    let original = root.join("oteryn-3-4.bmp");
    let backup = root.join("hard-linked-backup.bmp");
    std::fs::write(&original, b"BM linked capture")?;
    std::fs::set_permissions(&original, std::fs::Permissions::from_mode(0o644))?;
    std::fs::hard_link(&original, &backup)?;
    capture(&mut app(&root, false)?)?;
    for linked in [&original, &backup] {
        assert_eq!(linked.metadata()?.permissions().mode() & 0o777, 0o644);
        assert_eq!(std::fs::read(linked)?, b"BM linked capture");
    }
    let mode = root.metadata()?.permissions().mode() & 0o777;
    let legacy_mode = legacy.metadata()?.permissions().mode() & 0o777;
    let data = std::fs::read(&legacy);
    let entries = std::fs::read_dir(&root)?.count();
    assert_eq!(entries, 4);
    assert_eq!(mode, 0o755);
    assert_eq!(legacy_mode, 0o644);
    assert_eq!(data?, b"BM legacy capture");
    Ok(())
}

#[cfg(unix)]
fn audit_capture_after_json(basename: &str, parent_mode: u32, expected_mode: u32) -> TestResult {
    use super::super::audit;
    use std::os::unix::fs::PermissionsExt;
    let fixture = TestDir::new()?;
    let root = fixture.path().to_path_buf();
    let folder = root.join("audit");
    let output = folder.join(basename).with_extension("json");
    let mut app = app(&root, false)?;
    app.insert_resource(audit::AuditOutput::new(Some(output.clone())))
        .add_systems(PostUpdate, audit::write);
    app.world_mut()
        .spawn((Window::default(), bevy::window::PrimaryWindow));
    app.update();
    let json = std::fs::read(&output)?;
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&json)?["schema"],
        "oteryn.bevy-ui-audit.v1"
    );
    std::fs::set_permissions(&folder, std::fs::Permissions::from_mode(parent_mode))?;
    let legacy = folder.join("oteryn-1-2-3.bmp");
    let lookalike = folder.join("oteryn-1-2.bmp");
    let unrelated = folder.join("unrelated.bmp");
    if parent_mode & 0o020 != 0 {
        for file in [&legacy, &lookalike, &unrelated] {
            std::fs::write(file, b"BM previous file")?;
            std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o644))?;
        }
    }
    capture(&mut app)?;
    if parent_mode & 0o020 != 0 {
        assert_eq!(legacy.metadata()?.permissions().mode() & 0o777, 0o644);
        assert_eq!(std::fs::read(legacy)?, b"BM previous file");
        for file in [&lookalike, &unrelated] {
            assert_eq!(file.metadata()?.permissions().mode() & 0o777, 0o644);
            assert_eq!(std::fs::read(file)?, b"BM previous file");
        }
    }
    let path = output.with_extension("png");
    let saved = std::fs::read(&path);
    let png_mode = path
        .metadata()
        .map(|entry| entry.permissions().mode() & 0o777);
    let mode = folder.metadata()?.permissions().mode() & 0o1777;
    let entries = std::fs::read_dir(&folder)?.count();
    assert_eq!(&saved?[..8], b"\x89PNG\r\n\x1a\n");
    assert_eq!(png_mode?, 0o600);
    assert_eq!(mode, expected_mode);
    assert_eq!(entries, if parent_mode & 0o020 != 0 { 5 } else { 2 });
    Ok(())
}

#[cfg(unix)]
#[test]
fn audit_f12_after_json_preserves_existing_directory() -> TestResult {
    audit_capture_after_json("audit", 0o755, 0o755)
}

#[cfg(unix)]
#[test]
fn audit_f12_accepts_maximum_audit_staging_basename() -> TestResult {
    audit_capture_after_json(&"a".repeat(244), 0o755, 0o755)
}

#[cfg(unix)]
#[test]
fn audit_f12_writes_private_png_in_shared_sticky_directory() -> TestResult {
    audit_capture_after_json("audit", 0o1777, 0o1777)
}

#[cfg(unix)]
#[test]
fn inherited_matrix_fixture_preserves_a_previous_process_directory() -> TestResult {
    let fixture = TestDir::new()?;
    let root = fixture.path().to_path_buf();
    let config = if cfg!(target_os = "macos") {
        "Library/Application Support"
    } else {
        ".config"
    };
    let occupied = root.join("previous-matrix-run");
    let screenshots = occupied.join(config).join("Oteryn/screenshots");
    std::fs::create_dir_all(&screenshots)?;
    std::fs::write(screenshots.join("existing.bmp"), b"unrelated")?;
    let output = std::process::Command::new(std::env::current_exe()?)
        .args([
            "--exact",
            "bevy_shell::screenshots::tests::inherited_capture_destination_marker_runs_the_matrix",
        ])
        .env("TMPDIR", &root)
        .output()?;
    let entries = std::fs::read_dir(&root)?.collect::<Result<Vec<_>, _>>()?;
    let preserved = entries
        .first()
        .map(|entry| {
            std::fs::read(
                entry
                    .path()
                    .join(config)
                    .join("Oteryn/screenshots/existing.bmp"),
            )
        })
        .transpose();
    assert_one_test_passed(&output);
    assert_eq!(entries.len(), 1);
    assert_eq!(preserved?.as_deref(), Some(b"unrelated".as_slice()));
    Ok(())
}

#[cfg(unix)]
#[test]
fn fallback_captures_from_two_processes_use_distinct_directories() -> TestResult {
    use std::os::unix::fs::PermissionsExt;
    let fixture = TestDir::new()?;
    let root = fixture.path().to_path_buf();
    for _ in 0..2 {
        let output = std::process::Command::new(std::env::current_exe()?)
            .args(["--exact", CAPTURE_DESTINATION_TEST])
            .args(["--skip", CAPTURE_DEFAULT])
            .env(CAPTURE_ROOT, &root)
            .current_dir(&root)
            .env("TMPDIR", &root)
            .env_remove("HOME")
            .env_remove("XDG_CONFIG_HOME")
            .env_remove("LOCALAPPDATA")
            .output()?;
        assert_one_test_passed(&output);
    }
    let folders = std::fs::read_dir(&root)?.collect::<Result<Vec<_>, _>>()?;
    assert_eq!(folders.len(), 2);
    for folder in folders {
        assert_eq!(folder.metadata()?.permissions().mode() & 0o777, 0o700);
        assert_eq!(std::fs::read_dir(folder.path())?.count(), 3);
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn audit_f12_preserves_group_writable_directory() -> TestResult {
    audit_capture_after_json("audit", 0o775, 0o775)
}

#[cfg(unix)]
#[test]
fn configured_bmps_preserve_group_writable_directory() -> TestResult {
    use std::os::unix::fs::PermissionsExt;
    let fixture = TestDir::new()?;
    let root = fixture.path().to_path_buf();
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o775))?;
    capture(&mut app(&root, false)?)?;
    capture_death(&root, false)?;
    let mode = root.metadata()?.permissions().mode() & 0o777;
    let files = std::fs::read_dir(&root)?.collect::<Result<Vec<_>, _>>()?;
    assert_eq!(mode, 0o775);
    assert_eq!(files.len(), 2);
    Ok(())
}

#[cfg(unix)]
#[test]
fn f12_and_death_save_in_write_and_search_only_directory() -> TestResult {
    use std::os::unix::fs::PermissionsExt;
    let fixture = TestDir::new()?;
    let root = fixture.path().to_path_buf();
    let mut manual = app(&root, false)?;
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o300))?;
    let can_read = std::fs::read_dir(&root).is_ok();
    let manual_result = capture(&mut manual);
    let death_result = capture_death(&root, false);
    let mode = root.metadata()?.permissions().mode() & 0o777;
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o700))?;
    let files = std::fs::read_dir(&root)?.collect::<Result<Vec<_>, _>>()?;
    let file_modes = files
        .iter()
        .map(|file| {
            file.metadata()
                .map(|entry| entry.permissions().mode() & 0o777)
        })
        .collect::<Result<Vec<_>, _>>()?;
    manual_result?;
    death_result?;
    assert_eq!(mode, 0o300);
    assert!(
        manual
            .world()
            .non_send::<HostOwner>()
            .status
            .starts_with("Screenshot saved:")
    );
    assert_eq!(
        file_modes,
        [0o600, 0o600],
        "directory listing permitted: {can_read}"
    );
    Ok(())
}

#[cfg(target_os = "linux")]
#[test]
fn f12_and_death_do_not_open_capture_directory_for_scanning() -> TestResult {
    use rustix::fs::inotify::{self, CreateFlags, Reader, WatchFlags};
    use std::mem::MaybeUninit;

    let fixture = TestDir::new()?;
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(fixture.path(), std::fs::Permissions::from_mode(0o755))?;
    let watch = inotify::init(CreateFlags::NONBLOCK | CreateFlags::CLOEXEC)?;
    inotify::add_watch(&watch, fixture.path(), WatchFlags::OPEN)?;
    capture(&mut app(fixture.path(), false)?)?;
    capture_death(fixture.path(), true)?;
    let mut buffer = [MaybeUninit::uninit(); 4096];
    let mut events = Reader::new(&watch, &mut buffer);
    let mut captures = 0;
    loop {
        match events.next() {
            Ok(event) => {
                assert!(event.file_name().is_some(), "capture opened its directory");
                captures += 1;
            }
            Err(rustix::io::Errno::AGAIN) => break,
            Err(error) => return Err(error.into()),
        }
    }
    assert_eq!(captures, 3);
    Ok(())
}

type AuditOpen = Box<dyn FnMut(&std::path::Path) -> io::Result<std::fs::File>>;

thread_local! {
    static AUDIT_OPEN: std::cell::RefCell<Option<AuditOpen>> = const { std::cell::RefCell::new(None) };
}

pub(super) fn open_audit_capture(path: &std::path::Path) -> io::Result<std::fs::File> {
    AUDIT_OPEN.with_borrow_mut(|open| match open {
        Some(open) => open(path),
        None => crate::play::screenshot::create_capture_file(path),
    })
}

fn audit_f12_retries_staging_collision(vanishing: bool) -> TestResult {
    use crate::play::screenshot::create_capture_file;
    use std::{cell::RefCell, rc::Rc};

    let fixture = TestDir::new()?;
    let output = fixture.join("new-audit.json");
    let mut app = app(fixture.path(), false)?;
    app.insert_resource(super::super::audit::AuditOutput::new(Some(output.clone())));
    let attempted = Rc::new(RefCell::new(Vec::new()));
    let recorded = Rc::clone(&attempted);
    AUDIT_OPEN.set(Some(Box::new(move |candidate| {
        let mut recorded = recorded.borrow_mut();
        let collide = recorded.len() < 2;
        recorded.push(candidate.to_path_buf());
        if collide {
            std::fs::File::options()
                .write(true)
                .create_new(true)
                .open(candidate)?;
            std::fs::write(candidate, b"stale")?;
        }
        let result = create_capture_file(candidate);
        if collide && vanishing {
            std::fs::remove_file(candidate)?;
        }
        result
    })));
    let result = capture(&mut app);
    AUDIT_OPEN.set(None);
    result?;

    assert_eq!(
        &std::fs::read(output.with_extension("png"))?[..8],
        b"\x89PNG\r\n\x1a\n"
    );
    let attempted = attempted.borrow();
    assert_eq!(attempted.len(), 3);
    for prior in &attempted[..2] {
        if vanishing {
            assert!(!prior.exists());
        } else {
            assert_eq!(std::fs::read(prior)?, b"stale");
        }
    }
    assert!(!attempted[2].exists());
    assert_eq!(
        std::fs::read_dir(fixture.path())?.count(),
        if vanishing { 1 } else { 3 }
    );
    Ok(())
}

#[test]
fn audit_f12_retries_when_occupied_staging_name_vanishes() -> TestResult {
    audit_f12_retries_staging_collision(true)
}

#[cfg(unix)]
const AUDIT_COLLISION_TEST: &str =
    "bevy_shell::screenshots::tests::audit_f12_skips_stale_staging_files_from_reused_pid";

#[cfg(unix)]
#[test]
fn inherited_audit_staging_fixture_is_untouched() -> TestResult {
    let fixture = TestDir::new()?;
    let existing = fixture.join("new-audit.png");
    std::fs::write(&existing, b"existing PNG")?;
    let child = std::process::Command::new(std::env::current_exe()?)
        .args(["--exact", AUDIT_COLLISION_TEST])
        .env("TMPDIR", fixture.path())
        .env("OTERYN_TEST_AUDIT_STAGING_FIXTURE", fixture.path())
        .output()?;
    assert_eq!(std::fs::read(existing)?, b"existing PNG");
    assert_one_test_passed(&child);
    Ok(())
}

#[test]
fn audit_f12_skips_stale_staging_files_from_reused_pid() -> TestResult {
    audit_f12_retries_staging_collision(false)
}

#[cfg(target_os = "linux")]
#[test]
fn read_only_legacy_destination_does_not_block_fallback_capture() -> TestResult {
    use std::os::unix::fs::PermissionsExt;
    let fixture = TestDir::new()?;
    let root = fixture.path().to_path_buf();
    let legacy = root.join("LOCALAPPDATA/Oteryn/screenshots");
    std::fs::create_dir_all(&legacy)?;
    std::fs::set_permissions(&legacy, std::fs::Permissions::from_mode(0o755))?;
    std::fs::write(legacy.join("oteryn-1-2-3.bmp"), b"BM legacy capture")?;
    // Return EROFS from chmod in the child, as on a read-only mount. The test
    // needs no mount privileges and still uses the real F12 and death callers.
    let output = std::process::Command::new("python3")
        .args([
            "-I",
            "-c",
            r#"
import ctypes
import errno
import os
import sys

lib = ctypes.CDLL("libseccomp.so.2")
lib.seccomp_init.restype = ctypes.c_void_p
lib.seccomp_rule_add.argtypes = [ctypes.c_void_p, ctypes.c_uint32, ctypes.c_int, ctypes.c_uint]
lib.seccomp_syscall_resolve_name.argtypes = [ctypes.c_char_p]
lib.seccomp_load.argtypes = [ctypes.c_void_p]
ctx = lib.seccomp_init(0x7fff0000)
assert ctx
for syscall in (b"chmod", b"fchmod", b"fchmodat", b"fchmodat2"):
    number = lib.seccomp_syscall_resolve_name(syscall)
    if number >= 0:
        assert lib.seccomp_rule_add(ctx, 0x00050000 | errno.EROFS, number, 0) == 0
assert lib.seccomp_load(ctx) == 0
os.execv(sys.argv[1], sys.argv[1:])
"#,
        ])
        .arg(std::env::current_exe()?)
        .args(["--exact", CAPTURE_DESTINATION_TEST])
        .args(["--skip", CAPTURE_DEFAULT])
        .env(CAPTURE_ROOT, &root)
        .current_dir(&root)
        .env("LOCALAPPDATA", root.join("LOCALAPPDATA"))
        .env("TMPDIR", &root)
        .env_remove("HOME")
        .env_remove("XDG_CONFIG_HOME")
        .output();
    let entries = std::fs::read_dir(&root)?.count();
    let legacy_mode = legacy.metadata()?.permissions().mode() & 0o777;
    let output = output?;
    assert_one_test_passed(&output);
    assert_eq!(
        entries, 2,
        "expected the legacy tree and the private fallback"
    );
    assert_eq!(legacy_mode, 0o755);
    Ok(())
}

#[cfg(unix)]
#[test]
fn crop_fixture_preserves_previous_process_directory() -> TestResult {
    let fixture = TestDir::new()?;
    let template =
        Pixels::new(2, 1, vec![0; 8])?.save(fixture.path().to_str().ok_or("fixture path")?)?;
    let child_fixture = tempfile::tempdir_in(fixture.path())?;
    let child_root = child_fixture.path();
    let occupied = child_root.join("previous-crop-run");
    std::fs::create_dir(&occupied)?;
    std::fs::copy(&template, occupied.join("file"))?;
    let child = std::process::Command::new(std::env::current_exe()?)
        .args([
            "--exact",
            "bevy_shell::screenshots::tests::game_only_crops_rendered_pixels_and_write_errors_reach_player",
        ])
        .env("TMPDIR", child_root)
        .output()?;
    let entries = std::fs::read_dir(child_root)?.collect::<Result<Vec<_>, _>>()?;
    assert_eq!(entries.len(), 1);
    assert_eq!(
        std::fs::read(entries[0].path().join("file"))?,
        std::fs::read(template)?
    );
    assert_one_test_passed(&child);
    Ok(())
}
