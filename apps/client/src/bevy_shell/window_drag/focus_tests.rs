use super::*;
#[test]
fn an_old_focus_burst_does_not_cancel_the_next_fresh_caption_gesture() -> Result {
    let (mut app, window, panel) = app(1., 1.);
    app.add_systems(
        PreUpdate,
        move |mut messages: MessageWriter<WindowFocused>, mut sent: Local<bool>| {
            if !*sent {
                for focused in [false, true, false, true] {
                    messages.write(WindowFocused { window, focused });
                }
                *sent = true;
            }
        },
    );
    let origin = Vec2::new(320., 255.);
    press_at(
        &mut app,
        panel,
        window,
        origin,
        PointerId::Mouse,
        PointerButton::Primary,
    )?;
    move_to(
        &mut app,
        panel,
        window,
        origin,
        Vec2::new(10., 0.),
        PointerId::Mouse,
        PointerButton::Primary,
        true,
    )?;
    app.update();
    assert!(
        app.world()
            .get::<DragState>(panel)
            .ok_or("state")?
            .gesture
            .is_none()
    );
    press_at(
        &mut app,
        panel,
        window,
        origin,
        PointerId::Mouse,
        PointerButton::Primary,
    )?;
    move_to(
        &mut app,
        panel,
        window,
        origin,
        Vec2::new(10., 0.),
        PointerId::Mouse,
        PointerButton::Primary,
        true,
    )?;
    app.update();
    move_to(
        &mut app,
        panel,
        window,
        origin,
        Vec2::new(20., 0.),
        PointerId::Mouse,
        PointerButton::Primary,
        false,
    )?;
    assert_eq!(
        translation(&app, panel)?,
        Vec2::new(30., 0.),
        "consumed focus-loss messages must not cancel a later fresh press"
    );
    Ok(())
}
