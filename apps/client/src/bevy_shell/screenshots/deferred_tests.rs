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
    for release_readback in [false, true] {
        let fixture = TestDir::new()?;
        let mut app = app(fixture.path(), false)?;
        let occupied = app.world_mut().spawn(Screenshot::primary_window()).id();
        for _ in 0..4 {
            press(&mut app);
        }
        if release_readback {
            app.world_mut().despawn(occupied);
        }
        press(&mut app);
        assert!(
            app.world()
                .non_send::<HostOwner>()
                .status
                .contains("queue is full")
        );
    }
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

#[test]
fn disabling_automatic_capture_during_readback_drops_the_save() -> TestResult {
    readback_after_preference_change(|options| options.automatic_disabled = true, false, 0)
}

#[test]
fn only_disabling_the_trigger_of_an_inflight_readback_cancels_it() -> TestResult {
    readback_after_preference_change(|options| options.on_low_health = false, false, 0)?;
    readback_after_preference_change(|options| options.on_death = true, false, 1)
}

fn readback_after_preference_change(
    change: impl FnOnce(&mut crate::play::ScreenshotPreferences),
    handoff: bool,
    expected: usize,
) -> TestResult {
    let folder = TestDir::new()?;
    let mut app = app(folder.path(), false)?;
    let (outbox, entity) = pending_low_health_capture(&mut app)?;
    change(
        &mut app
            .world_mut()
            .non_send_mut::<HostOwner>()
            .settings
            .preferences
            .screenshots,
    );
    if handoff {
        outbox.screenshot_activity_handoff()?;
    }
    app.world_mut().trigger(ScreenshotCaptured {
        entity,
        image: frame(),
    });
    app.world_mut().despawn(entity);
    wait_for_writes(&mut app)?;
    assert_eq!(std::fs::read_dir(folder.path())?.count(), expected);
    Ok(())
}

#[test]
fn activity_handoff_during_readback_drops_the_old_capture() -> TestResult {
    readback_after_preference_change(|_| {}, true, 0)
}

fn publish_health(outbox: &crate::play::Outbox, revision: u64, health: u32) {
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
}

fn pending_low_health_capture(
    app: &mut App,
) -> Result<(crate::play::Outbox, Entity), Box<dyn std::error::Error>> {
    let (link, _, outbox) = crate::play::play_channel();
    {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        owner.settings.preferences.screenshots.on_low_health = true;
        owner.play = Some(crate::bevy_shell::owner::ActivePlay {
            link,
            view: crate::play::PlayView::new(oteryn_renderer::TileCoord::new(0, 0), 0, &[])?,
        });
    }
    for (revision, health) in [(1, 100), (2, 3)] {
        publish_health(&outbox, revision, health);
        app.update();
    }
    let entity = app
        .world_mut()
        .query_filtered::<Entity, With<Screenshot>>()
        .single(app.world())?;
    Ok((outbox, entity))
}

#[test]
fn restored_crop_and_trigger_do_not_reuse_a_cancelled_readback() -> TestResult {
    let folder = TestDir::new()?;
    let mut app = app(folder.path(), false)?;
    let (outbox, old) = pending_low_health_capture(&mut app)?;
    for (revision, enabled) in [(3, false), (4, true)] {
        let mut owner = app.world_mut().non_send_mut::<HostOwner>();
        let mut prefs = owner.settings.preferences.clone();
        prefs.screenshots.game_only = !enabled;
        prefs.screenshots.on_low_health = enabled;
        owner.settings.apply(prefs);
        publish_health(&outbox, revision, 100);
        app.update();
    }
    publish_health(&outbox, 5, 3);
    app.update();
    app.world_mut().trigger(ScreenshotCaptured {
        entity: old,
        image: frame(),
    });
    app.world_mut().despawn(old);
    wait_for_writes(&mut app)?;
    assert_eq!(
        std::fs::read_dir(folder.path())?.count(),
        0,
        "old frame was saved for the new event"
    );
    let fresh = app
        .world_mut()
        .query_filtered::<Entity, With<Screenshot>>()
        .single(app.world())?;
    app.world_mut().trigger(ScreenshotCaptured {
        entity: fresh,
        image: frame(),
    });
    app.world_mut().despawn(fresh);
    wait_for_writes(&mut app)?;
    assert_eq!(std::fs::read_dir(folder.path())?.count(), 1);
    Ok(())
}
