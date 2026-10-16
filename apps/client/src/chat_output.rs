//! Selectable received text, never an editable substitute for server history.
use oteryn_client::settings::ClientSettings;

pub(crate) fn show(
    ui: &mut egui::Ui,
    channel: usize,
    contents: &str,
    settings: &ClientSettings,
) -> egui::text_edit::TextEditOutput {
    // egui's immutable TextBuffer supports selection/copy but refuses editing,
    // including paste, cut and typed text. Do not turn this into a String buffer.
    let mut received = contents;
    let mut edit = egui::TextEdit::multiline(&mut received)
        .id_salt(("received-chat-output", channel))
        .desired_width(ui.available_width())
        .desired_rows(1)
        .frame(egui::Frame::NONE);
    if let Some(size) = settings.stored_int("console.console.font_size") {
        edit = edit.font(egui::FontId::proportional(size.clamp(10, 32) as f32));
    }
    edit.show(ui)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preferences_browser::test_support::frame;

    fn draw(
        ctx: &egui::Context,
        channel: usize,
        contents: &str,
    ) -> egui::text_edit::TextEditOutput {
        egui::Area::new(egui::Id::new("chat-output-test"))
            .show(ctx, |ui| {
                ui.set_width(440.0);
                show(ui, channel, contents, &ClientSettings::default())
            })
            .inner
    }

    #[test]
    fn received_unicode_can_be_selected_and_copied_but_never_edited() -> Result<(), &'static str> {
        let ctx = egui::Context::default();
        let size = egui::vec2(480.0, 120.0);
        let contents = "Zażółć 🦀\nreceived second line";
        let (initial, _) = frame(&ctx, size, Vec::new(), |ctx| draw(ctx, 0, contents));
        let id = initial.response.id;
        let mut state = egui::TextEdit::load_state(&ctx, id).ok_or("output state")?;
        state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::two(
                egui::text::CCursor::new(0),
                egui::text::CCursor::new(contents.chars().count()),
            )));
        state.store(&ctx, id);
        ctx.memory_mut(|memory| memory.request_focus(id));
        let (selected, output) = frame(&ctx, size, vec![egui::Event::Copy], |ctx| {
            draw(ctx, 0, contents)
        });
        assert!(output.platform_output.commands.iter().any(|command| {
            matches!(command, egui::OutputCommand::CopyText(text) if text == contents)
        }));
        assert_eq!(selected.galley.job.text, contents);
        let (unchanged, _) = frame(
            &ctx,
            size,
            vec![
                egui::Event::Text("replacement".into()),
                egui::Event::Paste("paste".into()),
                egui::Event::Cut,
            ],
            |ctx| draw(ctx, 0, contents),
        );
        assert_eq!(unchanged.galley.job.text, contents);
        // A widget change flag may describe an attempted operation; the actual
        // immutable text/rendered galley is the authority, not that flag.
        Ok(())
    }

    #[test]
    fn pointer_drag_selects_received_unicode_across_lines() -> Result<(), &'static str> {
        let ctx = egui::Context::default();
        let size = egui::vec2(480.0, 120.0);
        let contents = "Zażółć 🦀\nsecond line";
        let _ = frame(&ctx, size, Vec::new(), |ctx| draw(ctx, 0, contents));
        let (initial, _) = frame(&ctx, size, Vec::new(), |ctx| draw(ctx, 0, contents));
        let position = |index| {
            initial.galley_pos
                + initial
                    .galley
                    .pos_from_cursor(egui::text::CCursor::new(index))
                    .left_center()
                    .to_vec2()
        };
        let start = position(0);
        let end = position(contents.chars().count());
        let _ = frame(
            &ctx,
            size,
            vec![
                egui::Event::PointerMoved(start),
                egui::Event::PointerButton {
                    pos: start,
                    button: egui::PointerButton::Primary,
                    pressed: true,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            |ctx| draw(ctx, 0, contents),
        );
        let _ = frame(&ctx, size, vec![egui::Event::PointerMoved(end)], |ctx| {
            draw(ctx, 0, contents)
        });
        let (selected, output) = frame(
            &ctx,
            size,
            vec![
                egui::Event::PointerButton {
                    pos: end,
                    button: egui::PointerButton::Primary,
                    pressed: false,
                    modifiers: egui::Modifiers::NONE,
                },
                egui::Event::Copy,
            ],
            |ctx| draw(ctx, 0, contents),
        );
        assert!(!selected.cursor_range.ok_or("pointer selection")?.is_empty());
        assert!(output.platform_output.commands.iter().any(|command| {
            matches!(command, egui::OutputCommand::CopyText(text) if text == contents)
        }));
        Ok(())
    }

    #[test]
    fn channel_selection_is_independent_and_removed_lines_cannot_be_copied()
    -> Result<(), &'static str> {
        let ctx = egui::Context::default();
        let size = egui::vec2(480.0, 120.0);
        let (initial, _) = frame(&ctx, size, Vec::new(), |ctx| draw(ctx, 0, "received 🦀"));
        let first = initial.response.id;
        let mut state = egui::TextEdit::load_state(&ctx, first).ok_or("output state")?;
        state
            .cursor
            .set_char_range(Some(egui::text::CCursorRange::two(
                egui::text::CCursor::new(0),
                egui::text::CCursor::new(10),
            )));
        state.store(&ctx, first);
        let (second, _) = frame(&ctx, size, Vec::new(), |ctx| {
            draw(ctx, 1, "different channel")
        });
        assert_ne!(first, second.response.id);
        ctx.memory_mut(|memory| memory.request_focus(first));
        let (shortened, output) = frame(&ctx, size, vec![egui::Event::Copy], |ctx| {
            draw(ctx, 0, "ż🦀")
        });
        assert_eq!(shortened.galley.job.text, "ż🦀");
        assert!(output.platform_output.commands.iter().any(|command| {
            matches!(command, egui::OutputCommand::CopyText(text) if text == "ż🦀")
        }));
        assert!(output.platform_output.commands.iter().all(|command| {
            !matches!(command, egui::OutputCommand::CopyText(text) if text.contains("received"))
        }));
        assert!(shortened.cursor_range.is_none_or(|range| {
            range
                .sorted_cursors()
                .iter()
                .all(|cursor| cursor.index <= egui::text::CharIndex(2))
        }));
        Ok(())
    }
}
