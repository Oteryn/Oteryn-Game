//! Native action rows. Assignments use the current session's projections, never saved handles.
use egui::{Rect, RichText};
use oteryn_client::action_bar::{
    ACTION_BAR_ROWS, ACTION_BAR_SLOTS, ACTION_COLUMN_WIDTH, ACTION_ROW_HEIGHT, ActionBar,
    ActionBarCommand, ActionBarEdge, ActionBarPreferences, SlotAssignment, SlotShortcut,
    row_position,
};
use oteryn_client::play::GameState;
use oteryn_client::settings::ClientSettings;
use oteryn_session::{ChatIntent, ChatRoom, EntityDetail, EntityKind, SpellTarget};
use std::num::NonZeroU32;

/// The caller must obtain these indices/targets from the current admitted content generation.
/// An empty slice exposes no fabricated spell-book entries.
pub struct AvailableSpell {
    pub label: String,
    pub spell: NonZeroU32,
    pub target: SpellTarget,
    pub aim_at_target: bool,
}

pub fn show(
    ctx: &egui::Context,
    bar: &mut ActionBar,
    state: &GameState,
    spells: &[AvailableSpell],
    settings: &ClientSettings,
    body: Rect,
    enabled: bool,
) -> Vec<ActionBarCommand> {
    if !body.is_finite() || !body.is_positive() {
        return Vec::new();
    }
    let english = settings.english;
    let [bottom, left, right] = bar.preferences().visible_rows();
    // Unset values preserve the renderer behavior that predates the reference
    // controls; the UI keeps the choice visibly indeterminate until changed.
    let show_shortcuts = settings
        .stored_bool("action_bars.bars.labels")
        .unwrap_or(true);
    let show_item_amounts = settings
        .stored_bool("action_bars.bars.item_amounts")
        .unwrap_or(true);
    let show_tooltips = settings
        .stored_bool("action_bars.bars.tooltips")
        .unwrap_or(true);
    let mut shown = [0_usize; 3];
    let mut commands = Vec::new();
    for row in 0..ACTION_BAR_ROWS {
        let Some(preferences) = bar
            .preferences()
            .row(row)
            .filter(|_| bar.preferences().row_is_visible(row))
        else {
            continue;
        };
        let Some((edge, _)) = row_position(row) else {
            continue;
        };
        let group = row / 3;
        let index = shown[group] as f32;
        shown[group] += 1;
        let vertical = edge != ActionBarEdge::Bottom;
        let top = body.bottom() - f32::from(bottom) * ACTION_ROW_HEIGHT;
        let (pos, size) = match edge {
            ActionBarEdge::Bottom => (
                egui::pos2(
                    body.left() + f32::from(left) * ACTION_COLUMN_WIDTH,
                    top + index * ACTION_ROW_HEIGHT,
                ),
                egui::vec2(
                    (body.width() - f32::from(left + right) * ACTION_COLUMN_WIDTH).max(1.0),
                    ACTION_ROW_HEIGHT,
                ),
            ),
            ActionBarEdge::Left => (
                egui::pos2(body.left() + index * ACTION_COLUMN_WIDTH, body.top()),
                egui::vec2(ACTION_COLUMN_WIDTH, (top - body.top()).max(1.0)),
            ),
            ActionBarEdge::Right => (
                egui::pos2(
                    body.right() - (index + 1.0) * ACTION_COLUMN_WIDTH,
                    body.top(),
                ),
                egui::vec2(ACTION_COLUMN_WIDTH, (top - body.top()).max(1.0)),
            ),
        };
        let bounds = Rect::from_min_size(pos, size);
        let clip = bounds.intersect(body);
        // A partially fitting row is omitted instead of placing an interactive area over
        // the header/chat. The shell separately explains when the game body is too small.
        if !clip.is_positive() || clip != bounds || size.y < ACTION_ROW_HEIGHT {
            continue;
        }
        egui::Area::new(egui::Id::new(("action-row", row)))
            .fixed_pos(pos)
            .default_size(size)
            .constrain_to(body)
            .sense(egui::Sense::hover())
            .enabled(enabled)
            .show(ctx, |ui| {
                ui.set_min_size(size);
                ui.set_max_size(size);
                ui.set_clip_rect(clip.intersect(ui.clip_rect()));
                ui.spacing_mut().item_spacing = egui::vec2(2.0, 2.0);
                egui::Frame::NONE
                    .fill(ui.visuals().panel_fill)
                    .show(ui, |ui| {
                        let name = row_label(row, english);
                        let header = ui.label(
                            RichText::new(if vertical {
                                format!(
                                    "{}{}",
                                    row % 3 + 1,
                                    if preferences.locked { " 🔒" } else { "" }
                                )
                            } else {
                                name
                            })
                            .size(8.0),
                        );
                        if !enabled {
                            egui::Popup::close_id(ctx, egui::Popup::default_response_id(&header));
                        }
                        header.context_menu(|ui| {
                            if ui
                                .add_enabled(
                                    enabled && !preferences.locked,
                                    egui::Button::new(if english {
                                        "Clear row assignments"
                                    } else {
                                        "Wyczyść przypisania rzędu"
                                    }),
                                )
                                .clicked()
                            {
                                let _ = bar.clear_row(row);
                                ui.close();
                            }
                        });
                        let width = if vertical {
                            ACTION_COLUMN_WIDTH - 4.0
                        } else {
                            48.0
                        };
                        let mut slots = |ui: &mut egui::Ui| {
                            for index in 0..ACTION_BAR_SLOTS {
                                let slot = row * ACTION_BAR_SLOTS + index;
                                let (label, available) = bar.assignment(slot).map_or_else(
                                    || ("—".into(), false),
                                    |a| assignment_label(a, state, english, show_item_amounts),
                                );
                                let shortcut = preferences.shortcuts[index]
                                    .map_or_else(|| "—".into(), shortcut_label);
                                let mut response =
                                    ui.add_sized([width, 24.0], egui::Button::new(""));
                                if show_tooltips {
                                    response = response.on_hover_text(format!(
                                        "{} · {}\n{label}\n{shortcut}",
                                        row_label(row, english),
                                        index + 1
                                    ));
                                }
                                response.widget_info(|| {
                                    egui::WidgetInfo::labeled(
                                        egui::WidgetType::Button,
                                        response.enabled(),
                                        format!(
                                            "{} · {}: {label}, {shortcut}",
                                            row_label(row, english),
                                            index + 1
                                        ),
                                    )
                                });
                                paint_slot_text(
                                    ui,
                                    &response,
                                    &label,
                                    if show_shortcuts { &shortcut } else { "" },
                                );
                                if enabled
                                    && available
                                    && response.clicked()
                                    && let Some(command) = bar.activate(slot)
                                {
                                    commands.push(command);
                                }
                                if !enabled {
                                    egui::Popup::close_id(
                                        ctx,
                                        egui::Popup::default_response_id(&response),
                                    );
                                }
                                response.context_menu(|ui| {
                                    ui.add_enabled_ui(enabled && !preferences.locked, |ui| {
                                        assign_menu(ui, bar, slot, state, spells, english)
                                    });
                                });
                            }
                        };
                        if vertical {
                            egui::ScrollArea::vertical()
                                .id_salt(("action-row-scroll", row))
                                .max_height((size.y - 16.0).max(1.0))
                                .show(ui, slots);
                        } else {
                            egui::ScrollArea::horizontal()
                                .id_salt(("action-row-scroll", row))
                                .scroll_bar_visibility(
                                    egui::scroll_area::ScrollBarVisibility::AlwaysVisible,
                                )
                                .show(ui, |ui| {
                                    ui.horizontal(&mut slots);
                                });
                        }
                    });
            });
    }
    commands
}

/// A separate one-line layout for each row keeps shortcut glyphs visible when the
/// caption is elided. Truncating a single multiline Button instead hides its second row.
fn paint_slot_text(ui: &egui::Ui, response: &egui::Response, caption: &str, shortcut: &str) {
    let mut inner = response.rect.shrink(2.0);
    // Scrollbar width can narrow a column. Vertical clipping must not reposition text.
    inner.min.x = inner.min.x.max(ui.clip_rect().left());
    inner.max.x = inner.max.x.min(ui.clip_rect().right());
    if !inner.is_finite() || !inner.is_positive() {
        return;
    }
    let color = if response.enabled() {
        ui.style().interact(response).text_color()
    } else {
        ui.visuals().weak_text_color()
    };
    for (index, text) in [caption, shortcut].into_iter().enumerate() {
        let top = inner.top() + inner.height() * 0.5 * index as f32;
        let line = Rect::from_min_size(
            egui::pos2(inner.left(), top),
            egui::vec2(inner.width(), inner.height() * 0.5),
        );
        let painter = ui.painter().with_clip_rect(line.intersect(ui.clip_rect()));
        let mut job = egui::text::LayoutJob::simple_singleline(
            text.into(),
            egui::FontId::proportional(9.0),
            color,
        );
        job.wrap.max_width = line.width();
        job.wrap.max_rows = 1;
        job.wrap.break_anywhere = true;
        let galley = painter.layout_job(job);
        let pos = line.center() - galley.size() * 0.5;
        painter.galley(pos, galley, color);
    }
}

fn assignment_label(
    assignment: &SlotAssignment,
    state: &GameState,
    english: bool,
    show_item_amounts: bool,
) -> (String, bool) {
    let item = match assignment.command {
        ActionBarCommand::MoveToBackpack(handle) => state
            .inventory
            .iter()
            .flat_map(|i| i.entries.iter().chain(i.equipment.iter().map(|e| &e.item)))
            .chain(state.container.iter().flat_map(|c| c.entries.iter()))
            .find(|item| item.handle == handle)
            .map(|item| {
                if show_item_amounts {
                    format!("#{} ×{}", item.item_definition_ref, item.count)
                } else {
                    format!("#{}", item.item_definition_ref)
                }
            }),
        ActionBarCommand::OpenItem(handle) => state.entities.iter().find_map(|e| {
            if e.kind == EntityKind::Corpse
                && let EntityDetail::Object {
                    item_handle: Some(current),
                    item_definition_ref,
                    quantity,
                } = e.detail
                && current == handle
            {
                Some(if show_item_amounts {
                    format!(
                        "{} #{item_definition_ref} ×{quantity}",
                        if english { "Corpse" } else { "Zwłoki" }
                    )
                } else {
                    format!(
                        "{} #{item_definition_ref}",
                        if english { "Corpse" } else { "Zwłoki" }
                    )
                })
            } else {
                None
            }
        }),
        _ => return (assignment.label.clone(), true),
    };
    item.map_or_else(
        || {
            (
                if english {
                    "Unavailable"
                } else {
                    "Niedostępne"
                }
                .into(),
                false,
            )
        },
        |label| (label, true),
    )
}

fn assign_menu(
    ui: &mut egui::Ui,
    bar: &mut ActionBar,
    slot: usize,
    state: &GameState,
    spells: &[AvailableSpell],
    english: bool,
) {
    let tr = |pl, en| if english { en } else { pl };
    ui.label(tr("Przypisz akcję", "Assign action"));
    let mut choose = |ui: &mut egui::Ui, label: String, command| {
        if ui.button(&label).clicked()
            && bar
                .assign(slot, Some(SlotAssignment { label, command }))
                .is_ok()
        {
            ui.close();
        }
    };
    ui.menu_button(tr("Czary", "Spells"), |ui| {
        if spells.is_empty() {
            ui.label(tr(
                "Brak katalogu czarów bieżącej sesji",
                "No spell catalogue for this session",
            ));
        }
        for spell in spells.iter().take(256) {
            choose(
                ui,
                spell.label.clone(),
                ActionBarCommand::CastSpell {
                    spell: spell.spell,
                    target: spell.target,
                    aim_at_target: spell.aim_at_target,
                },
            );
        }
    });
    ui.menu_button(tr("Przedmioty → plecak", "Items → backpack"), |ui| {
        let inventory = state
            .inventory
            .iter()
            .flat_map(|i| i.entries.iter().chain(i.equipment.iter().map(|e| &e.item)));
        let container = state.container.iter().flat_map(|c| c.entries.iter());
        let mut any = false;
        for item in inventory.chain(container) {
            any = true;
            choose(
                ui,
                format!("#{} ×{}", item.item_definition_ref, item.count),
                ActionBarCommand::MoveToBackpack(item.handle),
            );
        }
        if !any {
            ui.label(tr("Brak dostępnych przedmiotów", "No available items"));
        }
    });
    ui.menu_button(tr("Otwórz widoczne zwłoki", "Open visible corpse"), |ui| {
        let mut any = false;
        for entity in &state.entities {
            if entity.kind == EntityKind::Corpse
                && let EntityDetail::Object {
                    item_handle: Some(handle),
                    item_definition_ref,
                    ..
                } = entity.detail
            {
                any = true;
                choose(
                    ui,
                    format!(
                        "{} #{} ({}, {})",
                        tr("Zwłoki", "Corpse"),
                        item_definition_ref,
                        entity.position.x,
                        entity.position.y
                    ),
                    ActionBarCommand::OpenItem(handle),
                );
            }
        }
        if !any {
            ui.label(tr("Brak widocznych zwłok", "No visible corpse"));
        }
    });
    ui.menu_button(tr("Kanały czatu", "Chat channels"), |ui| {
        for (label, room) in [
            (tr("Pomoc", "Help"), ChatRoom::Help),
            (tr("Świat", "World"), ChatRoom::World),
            ("English", ChatRoom::English),
            (tr("Reklamy", "Advertising"), ChatRoom::Advertising),
        ] {
            choose(
                ui,
                label.into(),
                ActionBarCommand::Chat(ChatIntent::OpenRoom(room)),
            );
        }
    });
    if ui.button(tr("Wyczyść pole", "Clear slot")).clicked() {
        let _ = bar.assign(slot, None);
        ui.close();
    }
}

pub fn preferences_editor(
    ui: &mut egui::Ui,
    preferences: &mut ActionBarPreferences,
    english: bool,
) {
    let tr = |pl, en| if english { en } else { pl };
    ui.small(tr("Ukrycie rzędu nie wyłącza jego skrótów. Przypisania przedmiotów i czarów są czyszczone po zmianie sesji.", "Hidden rows keep shortcuts active. Item and spell assignments are cleared on admission changes."));
    for row in 0..ACTION_BAR_ROWS {
        let Some(mut draft) = preferences.row(row) else {
            continue;
        };
        ui.push_id(("action-row-preferences", row), |ui| {
            ui.collapsing(row_label(row, english), |ui| {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut draft.visible, tr("Widoczny", "Visible"));
                    ui.checkbox(
                        &mut draft.locked,
                        tr("Zablokuj przypisania", "Lock assignments"),
                    );
                });
                for (slot, shortcut) in draft.shortcuts.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(format!("{}", slot + 1));
                        egui::ComboBox::from_id_salt(("key", slot))
                            .selected_text(
                                shortcut.map_or_else(
                                    || tr("Brak", "None").into(),
                                    |s| key_label(s.key),
                                ),
                            )
                            .show_ui(ui, |ui| {
                                ui.selectable_value(shortcut, None, tr("Brak", "None"));
                                for key in (4..=39).chain(42..=57).chain(58..=66).chain(68..=82) {
                                    let choice = Some(SlotShortcut {
                                        key,
                                        modifiers: shortcut.map_or(0, |s| s.modifiers),
                                    });
                                    ui.selectable_value(shortcut, choice, key_label(key));
                                }
                            });
                        if let Some(shortcut) = shortcut {
                            for (label, bit) in
                                [("Shift", 1), ("Ctrl", 2), ("Alt", 4), ("Super", 8)]
                            {
                                let mut active = shortcut.modifiers & bit != 0;
                                if ui.checkbox(&mut active, label).changed() {
                                    if active {
                                        shortcut.modifiers |= bit;
                                    } else {
                                        shortcut.modifiers &= !bit;
                                    }
                                }
                            }
                        }
                    });
                }
                if ui
                    .button(tr("Usuń skróty rzędu", "Clear row shortcuts"))
                    .clicked()
                {
                    draft.shortcuts.fill(None);
                }
            });
        });
        let _ = preferences.set_row(row, draft);
    }
}

pub fn row_label(row: usize, english: bool) -> String {
    let edge = match row_position(row) {
        Some((ActionBarEdge::Bottom, _)) => {
            if english {
                "Bottom"
            } else {
                "Dół"
            }
        }
        Some((ActionBarEdge::Left, _)) => {
            if english {
                "Left"
            } else {
                "Lewo"
            }
        }
        Some((ActionBarEdge::Right, _)) => {
            if english {
                "Right"
            } else {
                "Prawo"
            }
        }
        None => "?",
    };
    format!("{edge} {}", row % 3 + 1)
}

fn key_label(key: u16) -> String {
    match key {
        4..=29 => char::from_u32(u32::from(b'A') + u32::from(key - 4))
            .map_or_else(|| key.to_string(), |c| c.to_string()),
        30..=38 => (key - 29).to_string(),
        39 => "0".into(),
        42 => "Backspace".into(),
        43 => "Tab".into(),
        44 => "Space".into(),
        45..=57 => [
            "-",
            "=",
            "[",
            "]",
            "\\",
            "Non-US #",
            ";",
            "'",
            "`",
            ",",
            ".",
            "/",
            "Caps Lock",
        ][usize::from(key - 45)]
        .into(),
        58..=69 => format!("F{}", key - 57),
        70..=78 => [
            "Print Screen",
            "Scroll Lock",
            "Pause",
            "Insert",
            "Home",
            "Page Up",
            "Delete",
            "End",
            "Page Down",
        ][usize::from(key - 70)]
        .into(),
        79 => "→".into(),
        80 => "←".into(),
        81 => "↓".into(),
        82 => "↑".into(),
        _ => format!("HID {key}"),
    }
}

pub fn shortcut_label(shortcut: SlotShortcut) -> String {
    let mut parts: Vec<String> = [("Shift", 1), ("Ctrl", 2), ("Alt", 4), ("Super", 8)]
        .into_iter()
        .filter(|(_, bit)| shortcut.modifiers & bit != 0)
        .map(|(name, _)| name.into())
        .collect();
    parts.push(key_label(shortcut.key));
    parts.join("+")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vertical_scrolling_clips_text_without_repositioning_the_two_lines()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut positions = Vec::new();
        for clipped_top in [0.0, 10.0] {
            let ctx = egui::Context::default();
            for _ in 0..2 {
                ctx.begin_pass(egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(300.0, 200.0),
                    )),
                    ..Default::default()
                });
                egui::Area::new("text-scroll-test".into())
                    .fixed_pos([40.0, 40.0])
                    .show(&ctx, |ui| {
                        let response = ui.add_sized([40.0, 36.0], egui::Button::new(""));
                        let mut clip = response.rect;
                        clip.min.y += clipped_top;
                        ui.set_clip_rect(clip.intersect(ui.clip_rect()));
                        paint_slot_text(ui, &response, "Caption", "1");
                    });
                let mut output = ctx.end_pass();
                output.textures_delta.clear();
                let rows: Vec<_> = output
                    .shapes
                    .iter()
                    .filter_map(|shape| match &shape.shape {
                        egui::epaint::Shape::Text(text)
                            if matches!(text.galley.job.text.as_str(), "Caption" | "1") =>
                        {
                            Some((text.galley.job.text.clone(), text.pos))
                        }
                        _ => None,
                    })
                    .collect();
                if !rows.is_empty() {
                    positions.push(rows);
                }
            }
        }
        let first = positions.first().ok_or("No text rows painted")?;
        assert_eq!(first.len(), 2);
        assert!(positions.iter().all(|rows| rows == first));
        Ok(())
    }

    #[test]
    fn shortcut_digits_are_rendered_on_the_second_line_including_nine_rows()
    -> Result<(), Box<dyn std::error::Error>> {
        for nine_rows in [false, true] {
            let mut preferences = ActionBarPreferences::default();
            for row in &mut preferences.extra_rows {
                row.visible = nine_rows;
            }
            let mut bar = ActionBar::new(preferences, &[])?;
            let ctx = egui::Context::default();
            let body = Rect::from_min_size(egui::pos2(100.0, 80.0), egui::vec2(640.0, 540.0));
            let mut shapes = Vec::new();
            for _ in 0..2 {
                ctx.begin_pass(egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(900.0, 700.0),
                    )),
                    ..Default::default()
                });
                let _ = show(
                    &ctx,
                    &mut bar,
                    &GameState::default(),
                    &[],
                    &ClientSettings {
                        english: true,
                        ..Default::default()
                    },
                    body,
                    true,
                );
                let mut output = ctx.end_pass();
                output.textures_delta.clear();
                shapes = output.shapes;
            }
            let glyphs = |text: &egui::epaint::TextShape| {
                text.galley
                    .rows
                    .iter()
                    .flat_map(|row| row.row.glyphs.iter())
                    .map(|glyph| glyph.chr)
                    .collect::<String>()
            };
            let (clip, shortcut) = shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text)
                        if text.galley.job.text == "1" && glyphs(text) == "1" =>
                    {
                        Some((shape.clip_rect, text))
                    }
                    _ => None,
                })
                .ok_or("Default digit was not rendered")?;
            assert!(!shortcut.galley.elided);
            assert_eq!(shortcut.galley.rows.len(), 1);
            assert_eq!(
                shortcut.visual_bounding_rect().intersect(clip),
                shortcut.visual_bounding_rect()
            );
            let caption = shapes
                .iter()
                .find_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text)
                        if glyphs(text) == "—"
                            && (shape.clip_rect.center().x - clip.center().x).abs() < 0.1
                            && shape.clip_rect.bottom() == clip.top() =>
                    {
                        Some(text)
                    }
                    _ => None,
                })
                .ok_or("Caption and shortcut were not separate text lines")?;
            assert!(caption.pos.y < shortcut.pos.y);
            assert!(
                caption.visual_bounding_rect().bottom() <= shortcut.visual_bounding_rect().top()
            );
        }
        Ok(())
    }

    #[test]
    fn disabled_rows_reject_clicks_and_assignment_menus() -> Result<(), Box<dyn std::error::Error>>
    {
        let mut bar = ActionBar::new(ActionBarPreferences::default(), &[])?;
        let command = ActionBarCommand::Chat(ChatIntent::OpenRoom(ChatRoom::Help));
        bar.assign(
            0,
            Some(SlotAssignment {
                label: "Help".into(),
                command: command.clone(),
            }),
        )?;
        let ctx = egui::Context::default();
        let body = Rect::from_min_size(egui::pos2(100.0, 80.0), egui::vec2(600.0, 400.0));
        let mut frame = |enabled, events| {
            ctx.begin_pass(egui::RawInput {
                screen_rect: Some(Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(800.0, 600.0),
                )),
                events,
                ..Default::default()
            });
            let commands = show(
                &ctx,
                &mut bar,
                &GameState::default(),
                &[],
                &ClientSettings {
                    english: true,
                    ..Default::default()
                },
                body,
                enabled,
            );
            ctx.end_pass().textures_delta.clear();
            commands
        };
        for _ in 0..2 {
            frame(true, Vec::new());
        }
        let pos = egui::pos2(body.left() + 20.0, body.bottom() - 24.0);
        let pointer = |button, pressed| {
            vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ]
        };
        assert!(frame(false, pointer(egui::PointerButton::Primary, true)).is_empty());
        assert!(frame(false, pointer(egui::PointerButton::Primary, false)).is_empty());
        frame(false, pointer(egui::PointerButton::Secondary, true));
        frame(false, pointer(egui::PointerButton::Secondary, false));
        assert!(!egui::Popup::is_any_open(&ctx));
        frame(true, Vec::new()); // Register enabled hit-test targets before a new physical press.
        frame(true, pointer(egui::PointerButton::Secondary, true));
        frame(true, pointer(egui::PointerButton::Secondary, false));
        assert!(egui::Popup::is_any_open(&ctx));
        frame(false, Vec::new());
        assert!(!egui::Popup::is_any_open(&ctx));
        frame(true, Vec::new());
        frame(true, pointer(egui::PointerButton::Primary, true));
        assert_eq!(
            frame(true, pointer(egui::PointerButton::Primary, false)),
            vec![command]
        );
        Ok(())
    }

    #[test]
    fn item_labels_follow_authoritative_count_and_disappearance()
    -> Result<(), Box<dyn std::error::Error>> {
        let handle = std::num::NonZeroU64::new(1).ok_or("handle")?;
        let assignment = SlotAssignment {
            label: "old count".into(),
            command: ActionBarCommand::MoveToBackpack(handle),
        };
        let mut state = GameState {
            inventory: Some(oteryn_session::CharacterInventory {
                entries: vec![oteryn_session::ItemEntry {
                    handle,
                    item_definition_ref: NonZeroU32::new(7).ok_or("definition")?,
                    count: NonZeroU32::new(3).ok_or("count")?,
                    sub_type: 0,
                }],
                ..Default::default()
            }),
            ..Default::default()
        };
        assert_eq!(
            assignment_label(&assignment, &state, true, true),
            ("#7 ×3".into(), true)
        );
        assert_eq!(
            assignment_label(&assignment, &state, true, false),
            ("#7".into(), true)
        );
        state.inventory = None;
        assert_eq!(
            assignment_label(&assignment, &state, true, true),
            ("Unavailable".into(), false)
        );
        assert_eq!(
            shortcut_label(SlotShortcut {
                key: 4,
                modifiers: 7
            }),
            "Shift+Ctrl+Alt+A"
        );
        Ok(())
    }

    #[test]
    fn all_visible_rows_stay_within_a_tiny_body() -> Result<(), Box<dyn std::error::Error>> {
        for size in [egui::vec2(150.0, 50.0), egui::vec2(300.0, 220.0)] {
            let body = Rect::from_min_size(egui::pos2(120.0, 80.0), size);
            let mut preferences = ActionBarPreferences::default();
            for row in &mut preferences.extra_rows {
                row.visible = true;
            }
            let mut bar = ActionBar::new(preferences, &[])?;
            let ctx = egui::Context::default();
            for _ in 0..2 {
                ctx.begin_pass(egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(800.0, 600.0),
                    )),
                    ..Default::default()
                });
                let _ = show(
                    &ctx,
                    &mut bar,
                    &GameState::default(),
                    &[],
                    &ClientSettings {
                        english: true,
                        ..Default::default()
                    },
                    body,
                    true,
                );
                let mut output = ctx.end_pass();
                output.textures_delta.clear(); // Geometry-only test intentionally has no renderer.
                for shape in output.shapes {
                    if shape.clip_rect.is_positive() {
                        assert_eq!(shape.clip_rect.intersect(body), shape.clip_rect);
                    }
                }
            }
        }
        Ok(())
    }
}
