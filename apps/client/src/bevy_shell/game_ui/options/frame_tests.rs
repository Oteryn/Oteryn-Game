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
    result
}
