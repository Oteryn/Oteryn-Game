//! Rows 162 and 163: the Graphics frame-rate limit and FPS indicator reach the Bevy pacer.
use super::*;
use crate::bevy_shell::frame_pacing::{Indicator, pace};
use bevy::ui_widgets::{Activate, ValueChange};
use std::time::{Duration, Instant};
type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn graphics_frame_limit_and_fps_indicator_apply_and_pace_frames() -> TestResult {
    let fixture = tempfile::tempdir()?;
    let path = fixture.path().join("bevy-options-frames.json");
    let result = (|| -> TestResult {
        let mut app = super::tests::app(Page::Graphics, Some(path.clone()))?;
        let slider = app
            .world_mut()
            .spawn(Control::Number("graphics.frame_limit"))
            .observe(slide)
            .id();
        app.world_mut().trigger(ValueChange {
            source: slider,
            value: 10.0f32,
            is_final: true,
        });
        let check = app
            .world_mut()
            .spawn(Control::Check("graphics.show_fps"))
            .observe(toggle)
            .id();
        app.world_mut().trigger(ValueChange {
            source: check,
            value: true,
            is_final: true,
        });
        let owner = app.world().non_send::<HostOwner>();
        assert_eq!(
            owner
                .settings
                .preferences
                .render_performance()
                .foreground_fps,
            60
        );
        let apply = app.world_mut().spawn(Control::Apply).observe(activate).id();
        app.world_mut().trigger(Activate { entity: apply });

        let stored = crate::play::SettingsState::new(Some(path.clone())).preferences;
        let performance = stored.render_performance();
        assert_eq!(performance.foreground_fps, 10);
        assert!(performance.show_fps);
        assert_eq!(stored.frames.foreground, 10);
        assert_eq!(stored.performance.foreground_fps, 10);

        let indicator = app
            .world_mut()
            .spawn((
                Indicator,
                Text::default(),
                Node {
                    display: Display::None,
                    ..default()
                },
            ))
            .id();
        app.init_non_send::<crate::play::PerformancePacer>()
            .add_systems(Update, pace);
        app.update();
        let started = Instant::now();
        app.update();
        // Ten frames a second: the second frame waits out the first one's 100 ms interval.
        assert!(started.elapsed() >= Duration::from_millis(95));
        let text = app.world().get::<Text>(indicator).ok_or("indicator text")?;
        assert!(text.0.starts_with("FPS: "), "indicator: {}", text.0);
        let node = app.world().get::<Node>(indicator).ok_or("indicator node")?;
        assert_eq!(node.display, Display::Flex);
        Ok(())
    })();
    fixture.close()?;
    result
}

#[test]
fn toggling_foreground_limit_restores_selected_rate() -> TestResult {
    let mut app = super::tests::app(Page::Graphics, None)?;
    let slider = app
        .world_mut()
        .spawn(Control::Number("graphics.frame_limit"))
        .observe(slide)
        .id();
    app.world_mut().trigger(ValueChange {
        source: slider,
        value: 37.0f32,
        is_final: true,
    });
    let check = app
        .world_mut()
        .spawn(Control::Check("graphics.limit_enabled"))
        .observe(toggle)
        .id();
    let apply = app.world_mut().spawn(Control::Apply).observe(activate).id();
    for (enabled, expected) in [(false, 0), (true, 37)] {
        app.world_mut().trigger(ValueChange {
            source: check,
            value: enabled,
            is_final: true,
        });
        app.world_mut().trigger(Activate { entity: apply });
        let settings = &app.world().non_send::<HostOwner>().settings.preferences;
        assert_eq!(settings.render_performance().foreground_fps, expected);
    }
    Ok(())
}

#[test]
fn even_scaling_applies_to_scene_tile_size() -> TestResult {
    let mut app = super::tests::app(Page::Graphics, None)?;
    let check = app
        .world_mut()
        .spawn(Control::Check("graphics.even_scaling"))
        .observe(toggle)
        .id();
    app.world_mut().trigger(ValueChange {
        source: check,
        value: true,
        is_final: true,
    });
    let apply = app.world_mut().spawn(Control::Apply).observe(activate).id();
    app.world_mut().trigger(Activate { entity: apply });
    let settings = &app.world().non_send::<HostOwner>().settings.preferences;
    assert_eq!(settings.graphics.tile_pixels(180, [4096; 2]), 64);
    Ok(())
}

#[test]
fn fullscreen_monitor_selection_reaches_the_window() -> TestResult {
    let mut app = super::tests::app(Page::Graphics, None)?;
    let monitor = app
        .world_mut()
        .spawn(Control::Number("graphics.monitor"))
        .observe(slide)
        .id();
    app.world_mut().trigger(ValueChange {
        source: monitor,
        value: 2.0f32,
        is_final: true,
    });
    let fullscreen = app
        .world_mut()
        .spawn(Control::Check("graphics.fullscreen"))
        .observe(toggle)
        .id();
    app.world_mut().trigger(ValueChange {
        source: fullscreen,
        value: true,
        is_final: true,
    });
    let apply = app.world_mut().spawn(Control::Apply).observe(activate).id();
    app.world_mut().trigger(Activate { entity: apply });
    let mut window = Window::default();
    crate::bevy_shell::apply_window(
        &app.world().non_send::<HostOwner>().settings.preferences,
        &mut window,
    );
    assert_eq!(
        window.mode,
        bevy::window::WindowMode::BorderlessFullscreen(bevy::window::MonitorSelection::Index(1))
    );
    Ok(())
}

#[test]
fn graphics_selectors_apply_supported_backend_and_sampling_mode() -> TestResult {
    let mut app = super::tests::app(Page::Graphics, None)?;
    for id in ["graphics.engine", "graphics.antialiasing"] {
        let choice = app
            .world_mut()
            .spawn(Control::Select(id, 1))
            .observe(activate)
            .id();
        app.world_mut().trigger(Activate { entity: choice });
    }
    let apply = app.world_mut().spawn(Control::Apply).observe(activate).id();
    app.world_mut().trigger(Activate { entity: apply });
    let graphics = app
        .world()
        .non_send::<HostOwner>()
        .settings
        .preferences
        .graphics;
    assert_eq!(graphics.engine, oteryn_renderer::GraphicsEngine::Vulkan);
    assert_eq!(graphics.antialiasing.samples(), 4);
    Ok(())
}
