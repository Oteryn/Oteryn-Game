use super::tests::{TestResult, app, frame, wait_for_writes};
use super::*;
use crate::play::screenshot::TestDir;

fn press(app: &mut App) {
    let mut keys = app.world_mut().resource_mut::<ButtonInput<KeyCode>>();
    keys.reset_all();
    keys.press(KeyCode::F12);
    app.update();
    app.world_mut()
        .resource_mut::<ButtonInput<KeyCode>>()
        .reset_all();
}

#[test]
fn deferred_game_capture_crops_the_viewport_at_dispatch() -> TestResult {
    let fixture = TestDir::new()?;
    let mut app = app(fixture.path(), true)?;
    let occupied = app.world_mut().spawn(Screenshot::primary_window()).id();
    app.world_mut().resource_mut::<WorldViewport>().size = UVec2::splat(40);
    press(&mut app);
    app.world_mut().resource_mut::<WorldViewport>().size = UVec2::ONE;
    app.world_mut().despawn(occupied);
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
    let path = std::fs::read_dir(fixture.path())?
        .next()
        .ok_or("deferred capture missing")??
        .path();
    let bytes = std::fs::read(path)?;
    assert_eq!(i32::from_le_bytes(bytes[18..22].try_into()?), 1);
    Ok(())
}

#[test]
fn fifth_deferred_capture_reports_that_the_queue_is_full() -> TestResult {
    let fixture = TestDir::new()?;
    let mut app = app(fixture.path(), false)?;
    let _occupied = app.world_mut().spawn(Screenshot::primary_window());
    for _ in 0..5 {
        press(&mut app);
    }
    assert!(
        app.world()
            .non_send::<HostOwner>()
            .status
            .contains("queue is full")
    );
    Ok(())
}

#[test]
fn backlog_manual_capture_survives_crop_preference_change() -> TestResult {
    let fixture = TestDir::new()?;
    let mut app = app(fixture.path(), false)?;
    let (link, _, _) = crate::play::play_channel();
    {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        owner.settings.preferences.screenshots.backlog_frames = 1;
        owner.play = Some(crate::bevy_shell::owner::ActivePlay {
            link,
            view: crate::play::PlayView::new(oteryn_renderer::TileCoord::new(0, 0), 0, &[])?,
        });
    }
    press(&mut app);
    let entity = app
        .world_mut()
        .query_filtered::<Entity, With<Screenshot>>()
        .single(app.world())?;
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .settings
        .preferences
        .screenshots
        .game_only = true;
    app.world_mut().trigger(ScreenshotCaptured {
        entity,
        image: frame(),
    });
    app.world_mut().despawn(entity);
    wait_for_writes(&mut app)?;
    assert_eq!(std::fs::read_dir(fixture.path())?.count(), 1);
    Ok(())
}

#[test]
fn deferred_manual_capture_leaves_death_capture_for_the_current_destination() -> TestResult {
    let first = TestDir::new()?;
    let second = TestDir::new()?;
    let mut app = app(first.path(), false)?;
    let (link, _, outbox) = crate::play::play_channel();
    {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        owner.settings.preferences.screenshots.backlog_frames = 1;
        owner.settings.preferences.screenshots.on_death = true;
        owner.play = Some(crate::bevy_shell::owner::ActivePlay {
            link,
            view: crate::play::PlayView::new(oteryn_renderer::TileCoord::new(0, 0), 0, &[])?,
        });
    }
    let publish_vitals = |revision, health| {
        outbox.push(vec![oteryn_session::SessionEvent::ActorVitals(
            oteryn_session::AppliedDelta {
                server_sequence: revision,
                base_revision: revision - 1,
                new_revision: revision,
                value: oteryn_session::ActorVitals {
                    health,
                    max_health: 100,
                    ..default()
                },
            },
        )]);
    };
    publish_vitals(1, 100);
    let occupied = app.world_mut().spawn(Screenshot::primary_window()).id();
    press(&mut app);
    app.world_mut()
        .non_send_mut::<HostOwner>()
        .settings
        .preferences
        .screenshots
        .directory = second.path().to_string_lossy().into_owned();
    publish_vitals(2, 0);
    app.world_mut().despawn(occupied);
    for destination in [first.path(), second.path()] {
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
        assert_eq!(std::fs::read_dir(destination)?.count(), 1);
    }
    Ok(())
}
