//! Independently authored adapter for the installed reference's integer sliders.
use egui::{Event, Key, MouseWheelUnit, PointerButton, Response, Ui};
use std::ops::RangeInclusive;

fn step(modifiers: egui::Modifiers) -> i32 {
    (if modifiers.shift { 10 } else { 1 }) * (if modifiers.ctrl { 100 } else { 1 })
}

fn arrow(ui: &mut Ui, increase: bool) -> (i32, f32) {
    let (_, response) = ui.allocate_exact_size(
        egui::vec2(18.0, ui.spacing().interact_size.y),
        egui::Sense::click(),
    );
    ui.painter()
        .rect_filled(response.rect, 2.0, ui.style().interact(&response).bg_fill);
    let center = response.rect.center();
    let sign = if increase { 1.0 } else { -1.0 };
    ui.painter().add(egui::Shape::convex_polygon(
        vec![
            center + egui::vec2(sign * 3.0, 0.0),
            center + egui::vec2(-sign * 3.0, -4.0),
            center + egui::vec2(-sign * 3.0, 4.0),
        ],
        ui.visuals().text_color(),
        egui::Stroke::NONE,
    ));
    response.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::WidgetType::Button,
            response.enabled(),
            if increase { "Increase" } else { "Decrease" },
        )
    });
    #[cfg(test)]
    ui.ctx().data_mut(|data| {
        data.insert_temp(egui::Id::new(("slider-arrow", increase)), response.rect)
    });
    let id = response.id.with("repeat-deadline");
    let now = ui.input(|input| input.time);
    let held = response.enabled()
        && response.is_pointer_button_down_on()
        && ui.input(|input| input.focused && input.pointer.button_down(PointerButton::Primary));
    let mut increment = 0;
    if held {
        let next = ui
            .ctx()
            .data_mut(|data| *data.get_temp_mut_or_insert_with(id, || now + 0.410));
        if now >= next {
            // Qt timers coalesce missed intervals; do not burst after a stalled frame.
            increment = 1;
            ui.ctx().data_mut(|data| data.insert_temp(id, now + 0.060));
        }
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(60));
    } else {
        ui.ctx().data_mut(|data| data.remove::<f64>(id));
    }
    // The original applies click modifiers on release, but repeats use no modifiers.
    if response.clicked() {
        increment += ui.input(|input| step(input.modifiers));
    }
    (
        if increase { increment } else { -increment },
        response.rect.width(),
    )
}

pub(super) fn suppress_wheel_tail(ctx: &egui::Context) {
    // Suppress the rest of a claimed smooth wheel gesture, not just its first frame.
    let wheel_claim = egui::Id::new("reference-slider-wheel-claimed");
    let fresh_wheel = ctx.input(|input| {
        input
            .events
            .iter()
            .any(|event| matches!(event, Event::MouseWheel { .. }))
    });
    if fresh_wheel || !ctx.input(|input| input.is_scrolling()) {
        ctx.data_mut(|data| data.remove::<bool>(wheel_claim));
    } else if ctx
        .data(|data| data.get_temp::<bool>(wheel_claim))
        .unwrap_or(false)
    {
        ctx.input_mut(|input| input.smooth_scroll_delta.y = 0.0);
    }
}

pub(super) fn show(ui: &mut Ui, value: &mut i32, range: RangeInclusive<i32>) -> Response {
    suppress_wheel_tail(ui.ctx());
    let wheel_claim = egui::Id::new("reference-slider-wheel-claimed");
    let before = *value;
    let min = *range.start();
    let max = *range.end();
    let output = ui.horizontal(|ui| {
        let (left, arrow_width) = arrow(ui, false);
        if left != 0 {
            *value = value.saturating_add(left).clamp(min, max);
        }
        ui.spacing_mut().slider_width =
            (ui.spacing().slider_width - 2.0 * arrow_width - 2.0 * ui.spacing().item_spacing.x)
                .max(45.0);
        let before_keys = *value;
        let mut slider = ui.add(
            egui::Slider::new(value, range)
                .clamping(egui::SliderClamping::Edits)
                .step_by(1.0)
                .show_value(false),
        );
        if slider.enabled() && slider.has_focus() {
            let delta = ui.input_mut(|input| {
                let mut delta = 0;
                input.events.retain(|event| {
                    if let Event::Key {
                        key,
                        pressed: true,
                        modifiers,
                        ..
                    } = event
                    {
                        let sign = match key {
                            Key::ArrowLeft => -1,
                            Key::ArrowRight => 1,
                            _ => return true,
                        };
                        delta += sign * step(*modifiers);
                        return false;
                    }
                    true
                });
                delta
            });
            if delta != 0 {
                *value = before_keys.saturating_add(delta).clamp(min, max);
            }
        }
        if slider.enabled() && slider.hovered() {
            let notches = ui.input_mut(|input| {
                let mut notches = 0.0;
                input.events.retain(|event| {
                    if let Event::MouseWheel {
                        unit: MouseWheelUnit::Line,
                        delta,
                        ..
                    } = event
                        && delta.y != 0.0
                    {
                        notches += delta.y;
                        return false;
                    }
                    true
                });
                if notches != 0.0 {
                    input.smooth_scroll_delta.y = 0.0;
                }
                notches
            });
            if notches != 0.0 {
                ui.ctx()
                    .data_mut(|data| data.insert_temp(wheel_claim, true));
                *value = ((*value as f64 + f64::from(max - min) * f64::from(notches) / 10.0)
                    .clamp(f64::from(min), f64::from(max))
                    + 0.5)
                    .floor() as i32;
            }
        }
        let (right, _) = arrow(ui, true);
        if right != 0 {
            *value = value.saturating_add(right).clamp(min, max);
        }
        if *value != before || left != 0 || right != 0 {
            slider.mark_changed();
        }
        slider
    });
    output.inner
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(
        ctx: &egui::Context,
        value: &mut i32,
        enabled: bool,
        time: f64,
        events: Vec<Event>,
    ) -> Response {
        let modifiers = events
            .iter()
            .find_map(|event| match event {
                Event::PointerButton { modifiers, .. } | Event::Key { modifiers, .. } => {
                    Some(*modifiers)
                }
                _ => None,
            })
            .unwrap_or_default();
        let mut response = None;
        let _ = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(500.0, 200.0),
                )),
                time: Some(time),
                events,
                modifiers,
                focused: true,
                ..Default::default()
            },
            |ui| {
                response = Some(
                    ui.add_enabled_ui(enabled, |ui| show(ui, value, 0..=1000))
                        .inner,
                );
            },
        );
        match response {
            Some(response) => response,
            None => unreachable!("slider rendered"),
        }
    }

    #[test]
    fn keys_use_source_modifiers_and_do_not_reach_another_widget() {
        for (modifiers, expected) in [
            (egui::Modifiers::NONE, 1),
            (egui::Modifiers::SHIFT, 10),
            (egui::Modifiers::CTRL, 100),
            (egui::Modifiers::SHIFT | egui::Modifiers::CTRL, 1000),
        ] {
            let ctx = egui::Context::default();
            let mut value = 0;
            let slider = frame(&ctx, &mut value, true, 0.0, vec![]);
            ctx.memory_mut(|memory| memory.request_focus(slider.id));
            let response = frame(
                &ctx,
                &mut value,
                true,
                0.1,
                vec![Event::Key {
                    key: Key::ArrowRight,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers,
                }],
            );
            assert!(response.changed());
            assert_eq!(value, expected);
            assert!(!ctx.input(|input| input.events.iter().any(|event| matches!(
                event,
                Event::Key {
                    key: Key::ArrowRight,
                    ..
                }
            ))));
            let _ = frame(
                &ctx,
                &mut value,
                false,
                0.2,
                vec![Event::Key {
                    key: Key::ArrowLeft,
                    physical_key: None,
                    pressed: true,
                    repeat: false,
                    modifiers,
                }],
            );
            assert_eq!(value, expected, "disabled child must retain intent");
        }
    }

    #[test]
    fn line_wheel_uses_a_tenth_of_range_and_claims_smooth_tail() {
        let ctx = egui::Context::default();
        let mut value = 200;
        let slider = frame(&ctx, &mut value, true, 0.0, vec![]);
        let response = frame(
            &ctx,
            &mut value,
            true,
            0.1,
            vec![
                Event::PointerMoved(slider.rect.center()),
                Event::MouseWheel {
                    phase: egui::TouchPhase::Move,
                    unit: MouseWheelUnit::Line,
                    delta: egui::vec2(0.0, 1.0),
                    modifiers: egui::Modifiers::NONE,
                },
            ],
        );
        assert!(response.changed());
        assert_eq!(value, 300);
        assert_eq!(ctx.input(|input| input.smooth_scroll_delta.y), 0.0);
        let _ = frame(&ctx, &mut value, true, 0.11, vec![]);
        assert_eq!(ctx.input(|input| input.smooth_scroll_delta.y), 0.0);
        let _ = frame(
            &ctx,
            &mut value,
            false,
            0.2,
            vec![Event::MouseWheel {
                phase: egui::TouchPhase::Move,
                unit: MouseWheelUnit::Line,
                delta: egui::vec2(0.0, -1.0),
                modifiers: egui::Modifiers::NONE,
            }],
        );
        assert_eq!(value, 300);
    }

    #[test]
    fn clicks_and_repeat_follow_source_timing_without_modifier_repeats() {
        let ctx = egui::Context::default();
        let mut value = 100;
        let _ = frame(&ctx, &mut value, true, 0.0, vec![]);
        let pos = match ctx
            .data(|data| data.get_temp::<egui::Rect>(egui::Id::new(("slider-arrow", true))))
        {
            Some(rect) => rect.center(),
            None => unreachable!("arrow rendered"),
        };
        let pointer = |pressed| Event::PointerButton {
            pos,
            button: PointerButton::Primary,
            pressed,
            modifiers: egui::Modifiers::SHIFT,
        };
        let _ = frame(
            &ctx,
            &mut value,
            true,
            1.0,
            vec![Event::PointerMoved(pos), pointer(true)],
        );
        assert_eq!(value, 100);
        let _ = frame(&ctx, &mut value, true, 1.02, vec![pointer(false)]);
        assert_eq!(value, 110);
        let _ = frame(&ctx, &mut value, true, 2.0, vec![pointer(true)]);
        let _ = frame(&ctx, &mut value, true, 2.409, vec![]);
        assert_eq!(value, 110);
        let _ = frame(&ctx, &mut value, true, 2.411, vec![]);
        assert_eq!(value, 111);
        let _ = frame(&ctx, &mut value, true, 2.472, vec![]);
        assert_eq!(value, 112);
        let _ = frame(&ctx, &mut value, false, 3.0, vec![]);
        let _ = frame(&ctx, &mut value, false, 3.1, vec![pointer(false)]);
        assert_eq!(value, 112);
    }
}
