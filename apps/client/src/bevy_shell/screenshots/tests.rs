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
        .insert_resource(WorldViewport {
            origin: UVec2::ONE,
            size: UVec2::new(2, 1),
        })
        .add_systems(Update, super::super::audit::capture_on_key);
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
    assert!(save(&frame(), &root, Some(WorldViewport::default())).is_err());
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn death_event_captures_without_f12_and_deduplicates() -> TestResult {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(format!(".audit-screenshots-auto-{}", std::process::id()));
    let mut app = app(&root, true)?;
    let (link, _, outbox) = crate::play::play_channel();
    {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        owner.settings.preferences.screenshots.on_death = true;
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
        0
    );
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
    assert_eq!(std::fs::read_dir(&root)?.count(), 1);
    assert_eq!(
        app.world_mut()
            .query::<&Screenshot>()
            .iter(app.world())
            .count(),
        0
    );
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[cfg(unix)]
#[test]
fn f12_without_config_home_saves_capture() -> TestResult {
    const CHILD: &str = "OTERYN_TEST_F12_NO_HOME";
    if std::env::var_os(CHILD).is_none() {
        assert!(
            std::process::Command::new(std::env::current_exe()?)
                .args([
                    "--exact",
                    "bevy_shell::screenshots::tests::f12_without_config_home_saves_capture"
                ])
                .env(CHILD, "1")
                .env_remove("HOME")
                .env_remove("LOCALAPPDATA")
                .env_remove("XDG_CONFIG_HOME")
                .status()?
                .success()
        );
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
    assert!(std::path::Path::new(path).is_file());
    std::fs::remove_file(path)?;
    Ok(())
}
