//! The real shell uses the same pacing state exercised by the portable graphics tests.
use super::*;
use std::time::{Duration, Instant};
pub(super) fn redraw(app: &mut Application, event_loop: &ActiveEventLoop) {
    if !app
        .frame_pacer
        .begin_redraw(Instant::now(), app.settings.performance())
    {
        return;
    }
    let scene = app
        .play
        .as_ref()
        .map(|play| play.view.scene())
        .or(app.scene.as_ref());
    if let (Some(renderer), Some(scene)) = (&mut app.renderer, scene)
        && redraw_eligible(Some(renderer.state().phase()))
    {
        if app.atlas.as_ref() != Some(scene.atlas()) {
            renderer.set_atlas(scene.atlas().clone());
            app.atlas = Some(scene.atlas().clone());
        }
        let before = renderer.state().presented_frames();
        if renderer
            .render_batches(app.generation, scene.tiles(), scene.sprites())
            .is_err()
        {
            app.fail(event_loop, ShellError::RendererRender);
        } else if renderer.state().presented_frames() > before {
            app.frame_pacer.presented(std::time::Instant::now());
        }
    }
}
pub(super) fn wait(app: &mut Application, event_loop: &ActiveEventLoop) {
    let now = Instant::now();
    let settings = app.settings.performance();
    let menu_events = app.settings.take_window_events(app.play.is_some());
    if !menu_events.is_empty() && app.play.is_some() {
        app.handle_input(event_loop, &menu_events);
    }
    app.poll_entry();
    if let Some(play) = &mut app.play {
        app.settings.restore(&mut play.view, &play.link);
    }
    if let Some(play) = &mut app.play
        && let Err(end) = play.view.tick(&play.link)
    {
        match end {
            PlayEnd::Ended(class) => app.return_to_login(class),
            PlayEnd::LoggedOut => app.return_to_character_selection(),
        }
    }
    if let (Some(play), Some(window)) = (&app.play, &app.window) {
        play.link.item_drag_tick(&play.view, window);
    }
    app.update_hud();
    if let Some(window) = &app.window
        && redraw_eligible(
            app.renderer
                .as_ref()
                .map(|renderer| renderer.state().phase()),
        )
        && app.frame_pacer.due(now, settings)
    {
        window.request_redraw();
    }
    let latency = app
        .play
        .as_ref()
        .and_then(|play| play.link.performance_latency());
    let indicator = app.frame_pacer.indicator(now, settings, latency);
    app.settings.performance_indicator(indicator.as_deref());
    // Keep UI/session transitions responsive independently of the render budget.
    let deadline = app.frame_pacer.deadline(now, settings);
    let wake = if deadline > now {
        deadline.min(now + Duration::from_millis(20))
    } else {
        now + Duration::from_millis(1)
    };
    event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(wake));
}
