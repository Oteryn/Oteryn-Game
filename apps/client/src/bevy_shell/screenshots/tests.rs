use super::*;
use bevy::{
    asset::RenderAssetUsages,
    render::render_resource::{Extent3d, TextureDimension, TextureFormat},
};
type TestResult = Result<(), Box<dyn std::error::Error>>;
fn frame() -> Image {
    Image::new_fill(
        Extent3d {
            width: 4,
            height: 3,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[20, 40, 60, 255],
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
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::F12);
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
fn manual_and_death_captures_share_private_destinations() -> TestResult {
    use std::{os::unix::fs::PermissionsExt, path::Path, process::Command};
    const TEST: &str =
        "bevy_shell::screenshots::tests::manual_and_death_captures_share_private_destinations";
    const CHILD: &str = "OTERYN_TEST_CAPTURE_DESTINATION";
    let root = std::env::temp_dir();
    if let Some(explicit) = std::env::var_os(CHILD) {
        let explicit = explicit == "explicit";
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
        } else if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            assert_eq!(folder, Path::new(&local).join("Oteryn/screenshots"));
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
        return Ok(());
    }

    let root = root.join(format!("oteryn-capture-matrix-{}", std::process::id()));
    std::fs::create_dir(&root)?;
    let mut failures = Vec::new();
    for mask in 0..8 {
        for explicit in [false, true] {
            let case = root.join(format!("{mask}-{explicit}"));
            std::fs::create_dir(&case)?;
            let mut child = Command::new("sh");
            child
                .args(["-c", r#"umask 022; exec "$@""#, "sh"])
                .arg(std::env::current_exe()?)
                .args(["--exact", TEST])
                .env(CHILD, if explicit { "explicit" } else { "default" })
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
                failures.push((mask, explicit));
            }
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
#[test]
fn audit_f12_replaces_png_with_private_capture() -> TestResult {
    use std::os::unix::fs::PermissionsExt;
    let root = std::env::temp_dir().join(format!("oteryn-audit-capture-{}", std::process::id()));
    std::fs::create_dir(&root)?;
    let output = root.join("new").join("audit.json");
    let mut app = app(&root, false)?;
    app.insert_resource(super::super::audit::AuditOutput::new(Some(output.clone())));
    capture(&mut app)?;
    capture(&mut app)?;
    let path = output.with_extension("png");
    assert_eq!(&std::fs::read(&path)?[..8], b"\x89PNG\r\n\x1a\n");
    assert_eq!(path.metadata()?.permissions().mode() & 0o777, 0o600);
    let folder = path.parent().ok_or("capture directory")?;
    assert_eq!(folder.metadata()?.permissions().mode() & 0o777, 0o700);
    assert_eq!(std::fs::read_dir(folder)?.count(), 1);
    std::fs::remove_dir_all(root)?;
    Ok(())
}
