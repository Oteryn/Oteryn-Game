use super::*;
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
    let root = std::env::temp_dir().join(format!("gap60-captures-{}", std::process::id()));
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
    std::fs::remove_dir_all(root)?;
    Ok(())
}
#[test]
fn game_only_crops_rendered_pixels_and_write_errors_reach_player() -> TestResult {
    let root = std::env::temp_dir().join(format!("gap60-crop-{}", std::process::id()));
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
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn death_event_captures_without_f12_and_deduplicates() -> TestResult {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(format!(".audit-screenshots-auto-{}", std::process::id()));
    capture_death(&root, false)?;
    assert_eq!(std::fs::read_dir(&root)?.count(), 1);
    std::fs::remove_dir_all(root)?;
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

#[cfg(unix)]
#[test]
fn f12_without_config_home_saves_capture() -> TestResult {
    const CHILD: &str = "OTERYN_TEST_F12_NO_HOME";
    if std::env::var_os(CHILD).is_none()
        || crate::play::PreferenceStore::path().is_some()
        || std::env::var_os("LOCALAPPDATA").is_some()
    {
        let root = std::env::temp_dir().join(format!("oteryn-f12-no-home-{}", std::process::id()));
        std::fs::create_dir(&root)?;
        let status = std::process::Command::new("sh")
            .args(["-c", r#"umask 022; exec "$@""#, "sh"])
            .arg(std::env::current_exe()?)
            .args([
                "--exact",
                "bevy_shell::screenshots::tests::f12_without_config_home_saves_capture",
            ])
            .env(CHILD, "1")
            .env("TMPDIR", &root)
            .env_remove("HOME")
            .env_remove("LOCALAPPDATA")
            .env_remove("XDG_CONFIG_HOME")
            .status();
        std::fs::remove_dir_all(root)?;
        assert!(status?.success());
        return Ok(());
    }
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
    assert_eq!(parent.parent(), Some(std::env::temp_dir().as_path()));
    Ok(())
}

#[cfg(unix)]
#[test]
fn death_with_backlog_uses_a_private_fallback() -> TestResult {
    use std::os::unix::fs::PermissionsExt;
    const CHILD: &str = "OTERYN_TEST_DEATH_NO_HOME";
    if std::env::var_os(CHILD).is_some()
        && crate::play::PreferenceStore::path().is_none()
        && std::env::var_os("LOCALAPPDATA").is_none()
    {
        return capture_death(std::path::Path::new(""), true);
    }
    let root = std::env::temp_dir().join(format!("oteryn-death-no-home-{}", std::process::id()));
    std::fs::create_dir(&root)?;
    let status = std::process::Command::new("sh")
        .args(["-c", r#"umask 022; exec "$@""#, "sh"])
        .arg(std::env::current_exe()?)
        .args([
            "--exact",
            "bevy_shell::screenshots::tests::death_with_backlog_uses_a_private_fallback",
        ])
        .env(CHILD, "1")
        .env("TMPDIR", &root)
        .env_remove("HOME")
        .env_remove("LOCALAPPDATA")
        .env_remove("XDG_CONFIG_HOME")
        .status()?;
    assert!(status.success());
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
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn inherited_f12_marker_preserves_existing_captures() -> TestResult {
    use std::os::unix::fs::PermissionsExt;
    let root = std::env::temp_dir().join(format!("oteryn-f12-marker-{}", std::process::id()));
    std::fs::create_dir(&root)?;
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
    let status = std::process::Command::new(std::env::current_exe()?)
        .args([
            "--exact",
            "bevy_shell::screenshots::tests::f12_without_config_home_saves_capture",
        ])
        .env("OTERYN_TEST_F12_NO_HOME", "1")
        .env("HOME", &root)
        .env("TMPDIR", &root)
        .env_remove("LOCALAPPDATA")
        .env_remove("XDG_CONFIG_HOME")
        .status();
    let preserved = std::fs::read(&existing);
    std::fs::remove_dir_all(root)?;
    assert!(status?.success());
    assert_eq!(preserved?, b"existing capture");
    Ok(())
}

#[cfg(unix)]
#[test]
fn inherited_capture_destination_marker_runs_the_matrix() -> TestResult {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "oteryn-matrix-marker-{}-{stamp}",
        std::process::id()
    ));
    use std::os::unix::fs::DirBuilderExt;
    std::fs::DirBuilder::new().mode(0o700).create(&root)?;
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
    std::fs::remove_dir_all(root)?;
    assert_eq!(entries, 1, "matrix runner wrote to the inherited home");
    assert_eq!(preserved?, b"existing capture");
    let output = output?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    Ok(())
}

#[cfg(unix)]
#[test]
fn manual_and_death_captures_share_private_destinations() -> TestResult {
    use std::{os::unix::fs::PermissionsExt, process::Command};
    let root = std::env::temp_dir().join(format!("oteryn-capture-matrix-{}", std::process::id()));
    std::fs::create_dir(&root)?;
    let mut failures = Vec::new();
    let cases = (0..8)
        .flat_map(|mask| [(mask, false, false), (mask, true, false)])
        .chain([(4, false, true)]);
    for (mask, explicit, legacy) in cases {
        let case = root.join(format!("{mask}-{explicit}-{legacy}"));
        std::fs::create_dir(&case)?;
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
            .env(
                CAPTURE_DESTINATION_CASE,
                if explicit { "explicit" } else { "default" },
            )
            .env("TMPDIR", &case);
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
        if !child.status()?.success() {
            failures.push((mask, explicit, legacy));
        }
        if legacy {
            assert_eq!(shared.metadata()?.permissions().mode() & 0o777, 0o700);
            assert_eq!(
                std::fs::read(shared.join("legacy.bmp"))?,
                b"BM legacy capture"
            );
            assert_eq!(std::fs::read_dir(shared)?.count(), 1);
        }
    }
    std::fs::remove_dir_all(root)?;
    assert!(
        failures.is_empty(),
        "failed destination cases: {failures:?}"
    );
    Ok(())
}

#[cfg(unix)]
const CAPTURE_DESTINATION_CASE: &str = "OTERYN_TEST_CAPTURE_MATRIX_CHILD";
#[cfg(unix)]
const CAPTURE_DESTINATION_TEST: &str =
    "bevy_shell::screenshots::tests::capture_destination_matrix_case";

#[cfg(unix)]
#[test]
fn capture_destination_matrix_case() -> TestResult {
    use std::{os::unix::fs::PermissionsExt, path::Path};
    if !std::env::args_os().any(|arg| arg == CAPTURE_DESTINATION_TEST) {
        return Ok(());
    }
    let Some(mode) = std::env::var_os(CAPTURE_DESTINATION_CASE) else {
        return Ok(());
    };
    let explicit = mode == "explicit";
    let root = std::env::temp_dir();
    for name in ["HOME", "XDG_CONFIG_HOME", "LOCALAPPDATA"] {
        if let Some(path) = std::env::var_os(name) {
            assert_eq!(Path::new(&path), root.join(name));
        }
    }
    let chosen = root.join("chosen");
    let destination = if explicit {
        chosen.as_path()
    } else {
        Path::new("")
    };
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
        assert_eq!(folder, chosen);
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
    let root = std::env::temp_dir().join(format!("oteryn-audit-capture-{}", std::process::id()));
    std::fs::create_dir(&root)?;
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
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn f12_restricts_existing_capture_directory() -> TestResult {
    use std::os::unix::fs::PermissionsExt;
    let root = std::env::temp_dir().join(format!("oteryn-legacy-captures-{}", std::process::id()));
    std::fs::create_dir(&root)?;
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o755))?;
    let legacy = root.join("legacy.bmp");
    std::fs::write(&legacy, b"BM legacy capture")?;
    std::fs::set_permissions(&legacy, std::fs::Permissions::from_mode(0o644))?;
    capture(&mut app(&root, false)?)?;
    let mode = root.metadata()?.permissions().mode() & 0o777;
    let data = std::fs::read(legacy);
    let entries = std::fs::read_dir(&root)?.count();
    std::fs::remove_dir_all(root)?;
    assert_eq!(entries, 2);
    assert_eq!(mode, 0o700);
    assert_eq!(data?, b"BM legacy capture");
    Ok(())
}

#[cfg(unix)]
fn audit_capture_after_json(
    fixture: &str,
    basename: &str,
    parent_mode: u32,
    expected_mode: u32,
) -> TestResult {
    use super::super::audit;
    use std::os::unix::fs::PermissionsExt;
    let root = std::env::temp_dir().join(format!("oteryn-audit-{fixture}-{}", std::process::id()));
    std::fs::create_dir(&root)?;
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
    capture(&mut app)?;
    let path = output.with_extension("png");
    let saved = std::fs::read(&path);
    let png_mode = path
        .metadata()
        .map(|entry| entry.permissions().mode() & 0o777);
    let mode = folder.metadata()?.permissions().mode() & 0o1777;
    let entries = std::fs::read_dir(&folder)?.count();
    std::fs::remove_dir_all(root)?;
    assert_eq!(&saved?[..8], b"\x89PNG\r\n\x1a\n");
    assert_eq!(png_mode?, 0o600);
    assert_eq!(mode, expected_mode);
    assert_eq!(entries, 2);
    Ok(())
}

#[cfg(unix)]
#[test]
fn audit_f12_after_json_restricts_capture_directory() -> TestResult {
    audit_capture_after_json("normal", "audit", 0o755, 0o700)
}

#[cfg(unix)]
#[test]
fn audit_f12_accepts_maximum_audit_staging_basename() -> TestResult {
    audit_capture_after_json("long", &"a".repeat(244), 0o755, 0o700)
}

#[cfg(unix)]
#[test]
fn audit_f12_writes_private_png_in_shared_sticky_directory() -> TestResult {
    audit_capture_after_json("shared", "audit", 0o1777, 0o1777)
}

#[cfg(unix)]
#[test]
fn inherited_matrix_fixture_preserves_a_previous_process_directory() -> TestResult {
    let root = std::env::temp_dir().join(format!("oteryn-matrix-collision-{}", std::process::id()));
    std::fs::create_dir(&root)?;
    let config = if cfg!(target_os = "macos") {
        "Library/Application Support"
    } else {
        ".config"
    };
    let output = std::process::Command::new("sh")
        .args([
            "-c",
            r#"
            occupied="$TMPDIR/oteryn-matrix-marker-$$"
            mkdir -p "$occupied/$CONFIG/Oteryn/screenshots" || exit 1
            printf unrelated > "$occupied/$CONFIG/Oteryn/screenshots/existing.bmp" || exit 1
            exec "$@"
        "#,
            "sh",
        ])
        .arg(std::env::current_exe()?)
        .args([
            "--exact",
            "bevy_shell::screenshots::tests::inherited_capture_destination_marker_runs_the_matrix",
        ])
        .env("TMPDIR", &root)
        .env("CONFIG", config)
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
    std::fs::remove_dir_all(root)?;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("running 1 test"));
    assert_eq!(entries.len(), 1);
    assert_eq!(preserved?.as_deref(), Some(b"unrelated".as_slice()));
    Ok(())
}

#[cfg(unix)]
#[test]
fn fallback_captures_from_two_processes_use_distinct_directories() -> TestResult {
    use std::os::unix::fs::PermissionsExt;
    let root = std::env::temp_dir().join(format!("oteryn-shared-tmp-{}", std::process::id()));
    std::fs::create_dir(&root)?;
    for _ in 0..2 {
        let output = std::process::Command::new(std::env::current_exe()?)
            .args(["--exact", CAPTURE_DESTINATION_TEST])
            .env(CAPTURE_DESTINATION_CASE, "default")
            .env("TMPDIR", &root)
            .env_remove("HOME")
            .env_remove("XDG_CONFIG_HOME")
            .env_remove("LOCALAPPDATA")
            .output()?;
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
    }
    let folders = std::fs::read_dir(&root)?.collect::<Result<Vec<_>, _>>()?;
    assert_eq!(folders.len(), 2);
    for folder in folders {
        assert_eq!(folder.metadata()?.permissions().mode() & 0o777, 0o700);
        assert_eq!(std::fs::read_dir(folder.path())?.count(), 3);
    }
    std::fs::remove_dir_all(root)?;
    Ok(())
}
