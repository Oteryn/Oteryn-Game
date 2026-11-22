use super::tests::{TestResult, app, frame, wait_for_writes};
use super::*;
use crate::play::screenshot::TestDir;

#[test]
fn f12_observer_returns_before_reporting_the_write_result() -> TestResult {
    let fixture = TestDir::new()?;
    let mut app = app(fixture.path(), true)?;
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
    assert_eq!(
        app.world().non_send::<HostOwner>().status,
        "Capturing screenshot…"
    );
    app.world_mut().despawn(entity);
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset_all();
    wait_for_writes(&mut app)?;
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .status
            .starts_with("Screenshot saved:")
    );
    Ok(())
}

fn backlog_app(folder: &std::path::Path) -> Result<App, Box<dyn std::error::Error>> {
    let mut app = app(folder, false)?;
    let (link, _, _) = crate::play::play_channel();
    let mut owner = app.world_mut().non_send_mut::<HostOwner>();
    owner.settings.preferences.screenshots.backlog_frames = 2;
    owner.play = Some(crate::bevy_shell::owner::ActivePlay {
        link,
        view: crate::play::PlayView::new(oteryn_renderer::TileCoord::new(100, 100), 7, &[])?,
    });
    Ok(app)
}

fn next_sample(app: &mut App) -> TestResult {
    let mut clock = app.world_mut().resource_mut::<Time<Real>>();
    let now = clock.last_update().ok_or("capture clock")?;
    clock.update_with_instant(now + std::time::Duration::from_secs(1));
    Ok(())
}

#[test]
fn f12_saves_the_buffered_frames_and_the_current_frame() -> TestResult {
    let fixture = TestDir::new()?;
    let mut app = backlog_app(fixture.path())?;
    for sample in 0..2 {
        if sample == 1 {
            next_sample(&mut app)?;
        }
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
        wait_for_writes(&mut app)?;
        assert_eq!(std::fs::read_dir(fixture.path())?.count(), 0);
    }
    super::tests::capture(&mut app)?;
    assert_eq!(std::fs::read_dir(fixture.path())?.count(), 3);
    Ok(())
}

#[test]
fn f12_saves_when_an_automatic_sample_is_due() -> TestResult {
    let fixture = TestDir::new()?;
    let mut app = backlog_app(fixture.path())?;
    super::tests::capture(&mut app)?;
    assert_eq!(std::fs::read_dir(fixture.path())?.count(), 1);
    Ok(())
}

#[test]
fn audit_f12_shares_a_readback_with_the_due_backlog_sample() -> TestResult {
    let fixture = TestDir::new()?;
    let mut app = backlog_app(fixture.path())?;
    let audit = fixture.join("audit.json");
    app.insert_resource(super::super::audit::AuditOutput::new(Some(audit.clone())));
    super::tests::capture(&mut app)?;
    assert!(audit.with_extension("png").is_file());
    app.insert_resource(super::super::audit::AuditOutput::new(None));
    super::tests::capture(&mut app)?;
    assert_eq!(std::fs::read_dir(fixture.path())?.count(), 3);
    Ok(())
}

#[test]
fn f12_waits_for_a_fresh_readback_after_an_older_capture() -> TestResult {
    for (audit, backlog, already_delivered) in [
        (false, false, false),
        (true, false, true),
        (false, true, false),
    ] {
        let fixture = TestDir::new()?;
        let mut app = if backlog {
            backlog_app(fixture.path())?
        } else {
            app(fixture.path(), false)?
        };
        let audit_path = fixture.path().join("audit.json");
        if audit {
            app.insert_resource(super::super::audit::AuditOutput::new(Some(
                audit_path.clone(),
            )));
        }
        let old = if backlog {
            app.update();
            app.world_mut()
                .query_filtered::<Entity, With<Screenshot>>()
                .single(app.world())?
        } else {
            app.world_mut().spawn(Screenshot::primary_window()).id()
        };
        if already_delivered {
            app.world_mut().trigger(ScreenshotCaptured {
                entity: old,
                image: frame(),
            });
        }
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::F12);
        app.update();
        if !already_delivered {
            app.world_mut().trigger(ScreenshotCaptured {
                entity: old,
                image: frame(),
            });
        }
        assert!(
            app.world().non_send::<Writes>().queued.is_empty() || backlog,
            "F12 attached to an older readback"
        );
        app.world_mut().despawn(old);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let fresh = loop {
            app.update();
            if let Ok(entity) = app
                .world_mut()
                .query_filtered::<Entity, With<Screenshot>>()
                .single(app.world())
            {
                break entity;
            }
            assert!(
                std::time::Instant::now() < deadline,
                "F12 did not request a fresh readback"
            );
            std::thread::yield_now();
        };
        assert_ne!(old, fresh);
        assert_eq!(std::fs::read_dir(fixture.path())?.count(), 0);
        app.world_mut().trigger(ScreenshotCaptured {
            entity: fresh,
            image: super::tests::colored_frame([1, 2, 3, 255]),
        });
        app.world_mut().despawn(fresh);
        wait_for_writes(&mut app)?;
        if audit {
            use bevy::image::{CompressedImageFormats, ImageFormat, ImageSampler, ImageType};
            let actual = Image::from_buffer(
                &std::fs::read(audit_path.with_extension("png"))?,
                ImageType::Format(ImageFormat::Png),
                CompressedImageFormats::NONE,
                true,
                ImageSampler::Default,
                bevy::asset::RenderAssetUsages::MAIN_WORLD,
            )?
            .try_into_dynamic()?
            .into_rgb8();
            let expected = super::tests::colored_frame([1, 2, 3, 255])
                .try_into_dynamic()?
                .into_rgb8();
            assert_eq!(actual, expected);
        } else {
            let mut files = std::fs::read_dir(fixture.path())?
                .map(|file| file.map(|file| file.path()))
                .collect::<Result<Vec<_>, _>>()?;
            files.sort_by_key(|file| {
                let name = file
                    .file_stem()
                    .and_then(|name| name.to_str())
                    .unwrap_or_default();
                name.rsplit('-')
                    .next()
                    .and_then(|sequence| sequence.parse::<u64>().ok())
            });
            let colors = files
                .iter()
                .map(|file| -> io::Result<Vec<u8>> { Ok(std::fs::read(file)?[54..57].to_vec()) })
                .collect::<Result<Vec<_>, _>>()?;
            let expected = if backlog {
                vec![vec![60, 40, 20], vec![3, 2, 1]]
            } else {
                vec![vec![3, 2, 1]]
            };
            assert_eq!(colors, expected);
        }
    }
    Ok(())
}

#[test]
fn deferred_f12_drops_on_generation_change() -> TestResult {
    let folder = TestDir::new()?;
    let mut app = app(folder.path(), false)?;
    let old = app.world_mut().spawn(Screenshot::primary_window()).id();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::F12);
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset_all();
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .enter_character(Some([1; 16]));
    app.world_mut().despawn(old);
    app.update();
    assert_eq!(
        app.world_mut()
            .query_filtered::<Entity, With<Screenshot>>()
            .iter(app.world())
            .count(),
        0
    );
    Ok(())
}

#[test]
fn deferred_f12_keeps_each_press_and_its_directory() -> TestResult {
    let first = TestDir::new()?;
    let second = TestDir::new()?;
    let mut app = app(first.path(), false)?;
    let old = app.world_mut().spawn(Screenshot::primary_window()).id();
    for directory in [first.path(), second.path()] {
        app.world_mut()
            .non_send_mut::<HostOwner>()
            .settings
            .preferences
            .screenshots
            .directory = directory.to_str().ok_or("capture directory")?.into();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::F12);
        app.update();
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .reset_all();
    }
    app.world_mut().despawn(old);
    for directory in [first.path(), second.path()] {
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
        wait_for_writes(&mut app)?;
        assert_eq!(std::fs::read_dir(directory)?.count(), 1);
    }
    Ok(())
}

#[test]
fn deferred_backlog_capture_reports_a_stalled_writer() -> TestResult {
    let folder = TestDir::new()?;
    let mut app = backlog_app(folder.path())?;
    app.world_mut().non_send_mut::<Writes>().automatic = true;
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .press(KeyCode::F12);
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset_all();
    let clock = app.world().resource::<Time<Real>>();
    let later = clock.last_update().ok_or("capture clock")? + std::time::Duration::from_secs(2);
    app.world_mut()
        .resource_mut::<Time<Real>>()
        .update_with_instant(later);
    app.update();
    let owner = app.world().non_send::<HostOwner>();
    assert_eq!(
        owner.status,
        "Screenshot failed: timed out waiting for capture"
    );
    Ok(())
}
