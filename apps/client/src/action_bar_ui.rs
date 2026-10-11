//! Native action rows. Assignments use the current session's projections, never saved handles.
use egui::{Rect, RichText};
use oteryn_client::action_bar::{
    ACTION_BAR_ROWS, ACTION_BAR_SLOTS, ACTION_COLUMN_WIDTH, ACTION_ROW_HEIGHT, ActionBar,
    ActionBarCommand, ActionBarEdge, ActionBarPreferences, SlotAssignment, SlotShortcut,
    row_position,
};
use oteryn_client::play::GameState;
use oteryn_client::settings::ClientSettings;
use oteryn_session::{EntityDetail, EntityKind, SpellTarget};
use std::num::NonZeroU32;

/// The caller must obtain these indices/targets from the current admitted content generation.
/// An empty slice exposes no fabricated spell-book entries.
pub struct AvailableSpell {
    pub label: String,
    pub spell: NonZeroU32,
    pub target: SpellTarget,
    pub aim_at_target: bool,
}

const ASSIGNMENT_CATEGORY_LABELS: [[&str; 2]; 6] = [
    ["Zaklęcie", "Spell"],
    ["Przedmiot", "Object"],
    ["Wiele akcji", "Multi-Action"],
    ["Tekst", "Text"],
    ["Umiejętność pasywna", "Passive Ability"],
    ["Skrót klawiszowy", "Hotkey"],
];

const ACTION_BARS_UNLOCKED_TOOLTIP: &str = "Action Bars Unlocked: You can assign actions to or switch actions on action buttons by \"drag&drop\".";
const ACTION_BARS_LOCKED_TOOLTIP: &str = "Action Bars Locked: You cannot assign actions to or switch actions on action buttons by \"drag&drop\".";
const ACTION_BARS_UNAVAILABLE_TOOLTIP: [&str; 2] = [
    "Blokada jest niedostępna, dopóki bieżąca sesja nie ma co najmniej jednego przypisania na pasku akcji.",
    "Locking is unavailable until the current session has at least one action-bar assignment.",
];
const SETTINGS_LOCK_UNAVAILABLE_TOOLTIP: [&str; 2] = [
    "Blokada jest niedostępna w ustawieniach bez danych o przypisaniach bieżącej sesji.",
    "Locking is unavailable in Settings without the current session assignment state.",
];
const EMPTY_ACTION_SLOT_UNAVAILABLE_TOOLTIP: [&str; 2] = [
    "Puste pole jest niedostępne, ponieważ bieżąca sesja nie udostępnia czaru do przypisania ani odblokowanej czynności do przeciągnięcia.",
    "This empty slot is unavailable because the current session provides neither an assignable spell nor an unlocked action to drop here.",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ActionSlotDrag(usize);

fn is_trailing_lock_row(preferences: &ActionBarPreferences, row: usize) -> bool {
    if row >= ACTION_BAR_ROWS || !preferences.row_is_visible(row) {
        return false;
    }
    let group = row / 3;
    (row + 1..(group + 1) * 3).all(|candidate| !preferences.row_is_visible(candidate))
}

fn action_slot_enabled(
    row_enabled: bool,
    assigned: bool,
    spells_available: bool,
    assignments_available: bool,
    assignments_locked: bool,
) -> bool {
    row_enabled && (assigned || spells_available || (assignments_available && !assignments_locked))
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
    let primary_released =
        ctx.input(|input| input.pointer.button_released(egui::PointerButton::Primary));
    let lock_available = bar.has_assignments() || bar.assignments_locked();
    let mut shown = [0_usize; 3];
    let mut commands = Vec::new();
    let mut requested_lock = None;
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
        let trailing_lock = is_trailing_lock_row(bar.preferences(), row);
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
                        let row_has_assignments = bar.row_has_assignments(row);
                        let header = ui.label(
                            RichText::new(if vertical {
                                (row % 3 + 1).to_string()
                            } else {
                                name
                            })
                            .size(8.0),
                        );
                        if !enabled {
                            egui::Popup::close_id(ctx, egui::Popup::default_response_id(&header));
                        }
                        if row_has_assignments {
                            header.context_menu(|ui| {
                                if ui
                                    .add_enabled(
                                        enabled,
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
                        }
                        let width = if vertical {
                            ACTION_COLUMN_WIDTH - 4.0
                        } else {
                            48.0
                        };
                        let mut slots = |ui: &mut egui::Ui| {
                            for index in 0..ACTION_BAR_SLOTS {
                                let slot = row * ACTION_BAR_SLOTS + index;
                                let assigned = bar.assignment(slot).is_some();
                                let (label, available) = bar.assignment(slot).map_or_else(
                                    || ("—".into(), false),
                                    |a| assignment_label(a, state, english, show_item_amounts),
                                );
                                let shortcut = visible_shortcut_label(
                                    &preferences,
                                    index,
                                    bar.chat_context_active(),
                                );
                                let slot_enabled = action_slot_enabled(
                                    enabled,
                                    assigned,
                                    !spells.is_empty(),
                                    lock_available,
                                    bar.assignments_locked(),
                                );
                                let drag_enabled = enabled && assigned && !bar.assignments_locked();
                                let drop_enabled = enabled
                                    && lock_available
                                    && !bar.assignments_locked();
                                let sense = if drag_enabled {
                                    egui::Sense::click_and_drag()
                                } else if assigned || !spells.is_empty() {
                                    egui::Sense::click()
                                } else {
                                    // A live empty slot is only a drop surface. It must not
                                    // advertise a direct click that has no consumer.
                                    egui::Sense::hover()
                                };
                                let mut response = ui
                                    .add_enabled_ui(slot_enabled, |ui| {
                                        ui.add_sized(
                                            [width, 24.0],
                                            egui::Button::new("").sense(sense),
                                        )
                                    })
                                    .inner;
                                if enabled && !slot_enabled {
                                    response = response.on_disabled_hover_text(
                                        EMPTY_ACTION_SLOT_UNAVAILABLE_TOOLTIP
                                            [usize::from(english)],
                                    );
                                } else if show_tooltips {
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
                                if drag_enabled
                                    && response.drag_started_by(egui::PointerButton::Primary)
                                {
                                    egui::DragAndDrop::set_payload(ctx, ActionSlotDrag(slot));
                                }
                                let dropped = if drop_enabled && primary_released {
                                    response
                                        .dnd_release_payload::<ActionSlotDrag>()
                                        .map(|source| source.0)
                                } else {
                                    None
                                };
                                if response.dnd_hover_payload::<ActionSlotDrag>().is_some_and(
                                    |source| {
                                        drop_enabled && source.0 != slot
                                    },
                                ) {
                                    ui.painter().rect_stroke(
                                        response.rect.shrink(1.0),
                                        0.0,
                                        ui.visuals().widgets.active.bg_stroke,
                                        egui::StrokeKind::Inside,
                                    );
                                }
                                if let Some(source) = dropped {
                                    let _ = bar.move_or_swap_from_drop(source, slot);
                                }
                                if enabled
                                    && available
                                    && response.clicked()
                                    && let Some(command) = bar.activate(slot)
                                {
                                    commands.push(command);
                                }
                                if !response.enabled() {
                                    egui::Popup::close_id(
                                        ctx,
                                        egui::Popup::default_response_id(&response),
                                    );
                                }
                                if assigned || !spells.is_empty() {
                                    response.context_menu(|ui| {
                                        ui.add_enabled_ui(enabled, |ui| {
                                            assign_menu(ui, bar, slot, spells, english)
                                        });
                                    });
                                }
                            }
                        };
                        if vertical {
                            let lock_height = if trailing_lock {
                                24.0 + ui.spacing().item_spacing.y
                            } else {
                                0.0
                            };
                            egui::ScrollArea::vertical()
                                .id_salt(("action-row-scroll", row))
                                .max_height((ui.available_height() - lock_height).max(1.0))
                                .show(ui, &mut slots);
                            if trailing_lock
                                && edge_lock_control(
                                    ui,
                                    preferences.locked,
                                    width,
                                    enabled,
                                    lock_available,
                                    english,
                                )
                                .clicked()
                            {
                                requested_lock = Some(!preferences.locked);
                            }
                        } else {
                            let lock_width = if trailing_lock {
                                width + ui.spacing().item_spacing.x
                            } else {
                                0.0
                            };
                            let scroll_width = (ui.available_width() - lock_width).max(1.0);
                            ui.horizontal(|ui| {
                                ui.allocate_ui_with_layout(
                                    egui::vec2(scroll_width, ui.available_height()),
                                    egui::Layout::top_down(egui::Align::Min),
                                    |ui| {
                                        ui.set_width(scroll_width);
                                        egui::ScrollArea::horizontal()
                                            .id_salt(("action-row-scroll", row))
                                            .max_width(scroll_width)
                                            .scroll_bar_visibility(
                                                egui::scroll_area::ScrollBarVisibility::AlwaysVisible,
                                            )
                                            .show(ui, |ui| {
                                                ui.horizontal(&mut slots);
                                            });
                                    },
                                );
                                if trailing_lock
                                    && edge_lock_control(
                                        ui,
                                        preferences.locked,
                                        width,
                                        enabled,
                                        lock_available,
                                        english,
                                    )
                                    .clicked()
                                {
                                    requested_lock = Some(!preferences.locked);
                                }
                            });
                        }
                    });
            });
    }
    if let Some(locked) = requested_lock {
        bar.set_assignments_locked(locked);
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
    spells: &[AvailableSpell],
    english: bool,
) {
    let tr = |pl, en| if english { en } else { pl };
    let mut choose = |ui: &mut egui::Ui, label: String, command| {
        if ui.button(&label).clicked()
            && bar
                .assign(slot, Some(SlotAssignment { label, command }))
                .is_ok()
        {
            ui.close();
        }
    };
    if spells.is_empty() {
        unavailable_assignment_category(
            ui,
            ASSIGNMENT_CATEGORY_LABELS[0][usize::from(english)],
            tr(
                "Brak katalogu zaklęć bieżącej sesji.",
                "No authoritative spell catalogue is available for this session.",
            ),
        );
    } else {
        ui.menu_button(ASSIGNMENT_CATEGORY_LABELS[0][usize::from(english)], |ui| {
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
    }
    for labels in &ASSIGNMENT_CATEGORY_LABELS[1..] {
        unavailable_assignment_category(
            ui,
            labels[usize::from(english)],
            tr(
                "Ta kategoria nie ma jeszcze bezpiecznego wykonania w bieżącej sesji.",
                "This category has no safe session-backed execution yet.",
            ),
        );
    }
    ui.separator();
    if ui
        .add_enabled(
            bar.assignment(slot).is_some(),
            egui::Button::new(tr("Wyczyść pole", "Clear slot")),
        )
        .clicked()
    {
        let _ = bar.assign(slot, None);
        ui.close();
    }
}

fn unavailable_assignment_category(ui: &mut egui::Ui, label: &str, reason: &str) {
    ui.add_enabled(false, egui::Button::new(label))
        .on_disabled_hover_text(reason);
}

fn lock_tooltip(locked: bool) -> &'static str {
    if locked {
        ACTION_BARS_LOCKED_TOOLTIP
    } else {
        ACTION_BARS_UNLOCKED_TOOLTIP
    }
}

fn edge_lock_control(
    ui: &mut egui::Ui,
    locked: bool,
    width: f32,
    enabled: bool,
    assignments_available: bool,
    english: bool,
) -> egui::Response {
    let response = ui.add_enabled(
        enabled && assignments_available,
        egui::Button::new(if locked { "🔒" } else { "🔓" }).min_size(egui::vec2(width, 24.0)),
    );
    if assignments_available {
        response.on_hover_text(lock_tooltip(locked))
    } else {
        response.on_disabled_hover_text(ACTION_BARS_UNAVAILABLE_TOOLTIP[usize::from(english)])
    }
}

fn lock_assignment_control(ui: &mut egui::Ui, locked: &mut bool, english: bool) -> egui::Response {
    ui.add_enabled(
        false,
        egui::Checkbox::new(
            locked,
            if english {
                "Lock action bars"
            } else {
                "Zablokuj paski akcji"
            },
        ),
    )
    .on_disabled_hover_text(SETTINGS_LOCK_UNAVAILABLE_TOOLTIP[usize::from(english)])
}

pub fn preferences_editor(
    ui: &mut egui::Ui,
    preferences: &mut ActionBarPreferences,
    english: bool,
) {
    let tr = |pl, en| if english { en } else { pl };
    ui.small(tr("Ukrycie rzędu nie wyłącza jego skrótów. Przypisania przedmiotów i czarów są czyszczone po zmianie sesji.", "Hidden rows keep shortcuts active. Item and spell assignments are cleared on admission changes."));
    let mut locked = preferences.assignments_locked();
    if lock_assignment_control(ui, &mut locked, english).changed() {
        preferences.set_assignments_locked(locked);
    }
    for row in 0..ACTION_BAR_ROWS {
        let Some(mut draft) = preferences.row(row) else {
            continue;
        };
        ui.push_id(("action-row-preferences", row), |ui| {
            ui.collapsing(row_label(row, english), |ui| {
                ui.horizontal(|ui| {
                    ui.checkbox(&mut draft.visible, tr("Widoczny", "Visible"));
                });
                ui.add_enabled_ui(false, |ui| {
                    for (slot, shortcut) in draft.shortcuts.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(format!("{}", slot + 1));
                            egui::ComboBox::from_id_salt(("key", slot))
                                .selected_text(shortcut.map_or_else(
                                    || tr("Brak", "None").into(),
                                    |s| key_label(s.key),
                                ))
                                .show_ui(ui, |ui| {
                                    ui.selectable_value(shortcut, None, tr("Brak", "None"));
                                    for key in (4..=39).chain(42..=57).chain(58..=66).chain(68..=82)
                                    {
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
                })
                .response
                .on_disabled_hover_text(tr(
                    "Edycja wymaga wiarygodnego źródła przypisań paska akcji z bieżącej sesji.",
                    "Editing requires an authoritative live action-bar assignment source.",
                ));
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

fn visible_shortcut_label(
    preferences: &oteryn_client::action_bar::ActionRowPreferences,
    slot: usize,
    chat_on: bool,
) -> String {
    let (primary, secondary) = if chat_on {
        (
            preferences.chat_on_shortcuts.get(slot).copied().flatten(),
            preferences
                .chat_on_secondary_shortcuts
                .get(slot)
                .copied()
                .flatten(),
        )
    } else {
        (
            preferences.shortcuts.get(slot).copied().flatten(),
            preferences.secondary_shortcuts.get(slot).copied().flatten(),
        )
    };
    let label = [primary, secondary]
        .into_iter()
        .flatten()
        .map(shortcut_label)
        .collect::<Vec<_>>()
        .join(" / ");
    if label.is_empty() {
        "—".into()
    } else {
        label
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oteryn_session::{ChatIntent, ChatRoom};

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
    fn shortcut_labels_are_rendered_on_the_second_line_including_nine_rows()
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
                        if text.galley.job.text == "F1" && glyphs(text) == "F1" =>
                    {
                        Some((shape.clip_rect, text))
                    }
                    _ => None,
                })
                .ok_or("Default F1 shortcut was not rendered")?;
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
    fn disabled_rows_reject_clicks_and_assignment_menus_while_lock_still_allows_the_menu()
    -> Result<(), Box<dyn std::error::Error>> {
        let preferences = ActionBarPreferences {
            locked: true,
            ..Default::default()
        };
        let mut bar = ActionBar::new(preferences, &[])?;
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
            let mut output = ctx.end_pass();
            output.textures_delta.clear();
            let text = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) => Some(text.galley.job.text.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>();
            (commands, text)
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
        assert!(
            frame(false, pointer(egui::PointerButton::Primary, true))
                .0
                .is_empty()
        );
        assert!(
            frame(false, pointer(egui::PointerButton::Primary, false))
                .0
                .is_empty()
        );
        frame(false, pointer(egui::PointerButton::Secondary, true));
        frame(false, pointer(egui::PointerButton::Secondary, false));
        assert!(!egui::Popup::is_any_open(&ctx));
        frame(true, Vec::new()); // Register enabled hit-test targets before a new physical press.
        frame(true, pointer(egui::PointerButton::Secondary, true));
        frame(true, pointer(egui::PointerButton::Secondary, false));
        assert!(egui::Popup::is_any_open(&ctx));
        let (_, popup_text) = frame(true, Vec::new());
        for category in ASSIGNMENT_CATEGORY_LABELS.map(|labels| labels[1]) {
            assert!(popup_text.iter().any(|text| text == category));
        }
        frame(false, Vec::new());
        assert!(!egui::Popup::is_any_open(&ctx));
        frame(true, Vec::new());
        frame(true, pointer(egui::PointerButton::Primary, true));
        assert_eq!(
            frame(true, pointer(egui::PointerButton::Primary, false)).0,
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
    fn visible_shortcuts_follow_chat_context_and_include_both_chords() {
        let mut preferences = oteryn_client::action_bar::ActionRowPreferences::default();
        preferences.shortcuts[0] = Some(SlotShortcut {
            key: 4,
            modifiers: 0,
        });
        preferences.secondary_shortcuts[0] = Some(SlotShortcut {
            key: 5,
            modifiers: 2,
        });
        preferences.chat_on_shortcuts[0] = Some(SlotShortcut {
            key: 6,
            modifiers: 1,
        });
        preferences.chat_on_secondary_shortcuts[0] = Some(SlotShortcut {
            key: 7,
            modifiers: 4,
        });
        assert_eq!(visible_shortcut_label(&preferences, 0, false), "A / Ctrl+B");
        assert_eq!(
            visible_shortcut_label(&preferences, 0, true),
            "Shift+C / Alt+D"
        );
        assert_eq!(visible_shortcut_label(&preferences, 49, false), "—");
    }

    #[test]
    fn assignment_menu_uses_the_observed_categories_only() {
        assert_eq!(
            ASSIGNMENT_CATEGORY_LABELS.map(|labels| labels[1]),
            [
                "Spell",
                "Object",
                "Multi-Action",
                "Text",
                "Passive Ability",
                "Hotkey",
            ]
        );
    }

    #[test]
    fn every_nonempty_edge_has_exactly_one_trailing_lock_and_never_more_than_three() {
        for visible_mask in 0_u16..(1 << ACTION_BAR_ROWS) {
            for edge_mask in 0_u8..8 {
                let mut preferences = ActionBarPreferences {
                    visible: visible_mask & 1 != 0,
                    edge_enabled: std::array::from_fn(|edge| edge_mask & (1 << edge) != 0),
                    ..Default::default()
                };
                for row in 1..ACTION_BAR_ROWS {
                    preferences.extra_rows[row - 1].visible = visible_mask & (1 << row) != 0;
                }
                let controls = (0..ACTION_BAR_ROWS)
                    .filter(|row| is_trailing_lock_row(&preferences, *row))
                    .collect::<Vec<_>>();
                let expected = (0..3)
                    .filter(|edge| {
                        (*edge * 3..*edge * 3 + 3).any(|row| preferences.row_is_visible(row))
                    })
                    .count();
                assert_eq!(controls.len(), expected);
                assert!(controls.len() <= 3);
                for row in controls {
                    let group_end = (row / 3 + 1) * 3;
                    assert!(
                        (row + 1..group_end)
                            .all(|candidate| !preferences.row_is_visible(candidate))
                    );
                }
            }
        }
    }

    #[test]
    fn edge_locks_are_visible_without_scrolling_and_emit_one_global_change()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut preferences = ActionBarPreferences::default();
        preferences.extra_rows[2].visible = true;
        preferences.extra_rows[5].visible = true;
        let mut bar = ActionBar::new(preferences, &[])?;
        bar.assign(
            0,
            Some(SlotAssignment {
                label: "Help".into(),
                command: ActionBarCommand::Chat(ChatIntent::OpenRoom(ChatRoom::Help)),
            }),
        )?;
        let ctx = egui::Context::default();
        let screen = egui::vec2(800.0, 600.0);
        let body = Rect::from_min_size(egui::pos2(100.0, 80.0), egui::vec2(600.0, 400.0));
        let settings = ClientSettings {
            english: true,
            ..Default::default()
        };
        let draw = |bar: &mut ActionBar, events| {
            ctx.begin_pass(egui::RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, screen)),
                events,
                ..Default::default()
            });
            let commands = show(&ctx, bar, &GameState::default(), &[], &settings, body, true);
            let mut output = ctx.end_pass();
            output.textures_delta.clear();
            (commands, output)
        };
        let mut output = egui::FullOutput::default();
        for _ in 0..3 {
            output = draw(&mut bar, Vec::new()).1;
        }
        let visible_locks = |output: &egui::FullOutput, glyph: &str| {
            output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) if text.galley.job.text == glyph => {
                        let bounds = text.visual_bounding_rect();
                        (shape.clip_rect.contains_rect(bounds) && body.contains_rect(bounds))
                            .then_some(bounds.center())
                    }
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        let unlocked = visible_locks(&output, "🔓");
        assert_eq!(unlocked.len(), 3);
        let bottom = unlocked
            .into_iter()
            .max_by(|left, right| left.y.total_cmp(&right.y))
            .ok_or("bottom lock was not painted")?;
        let pointer = |pressed| {
            vec![
                egui::Event::PointerMoved(bottom),
                egui::Event::PointerButton {
                    pos: bottom,
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ]
        };
        assert!(draw(&mut bar, pointer(true)).0.is_empty());
        let commands = draw(&mut bar, pointer(false)).0;
        assert!(commands.is_empty());
        assert!(bar.assignments_locked());
        assert_eq!(bar.take_assignments_lock_change(), Some(true));
        let output = draw(&mut bar, Vec::new()).1;
        assert_eq!(visible_locks(&output, "🔒").len(), 3);

        bar.clear_row(0)?;
        assert!(!bar.has_assignments());
        let output = draw(&mut bar, Vec::new()).1;
        assert_eq!(visible_locks(&output, "🔒").len(), 3);
        assert!(draw(&mut bar, pointer(true)).0.is_empty());
        assert!(draw(&mut bar, pointer(false)).0.is_empty());
        assert!(!bar.assignments_locked());
        assert_eq!(bar.take_assignments_lock_change(), Some(false));
        Ok(())
    }

    #[test]
    fn lock_controls_are_disabled_without_runtime_assignments_and_keep_reference_tooltips() {
        assert_eq!(
            lock_tooltip(false),
            "Action Bars Unlocked: You can assign actions to or switch actions on action buttons by \"drag&drop\"."
        );
        assert_eq!(
            lock_tooltip(true),
            "Action Bars Locked: You cannot assign actions to or switch actions on action buttons by \"drag&drop\"."
        );
        for initial in [false, true] {
            let ctx = egui::Context::default();
            let mut locked = initial;
            let mut enabled = false;
            let mut output = ctx.run_ui(Default::default(), |ui| {
                enabled = lock_assignment_control(ui, &mut locked, true).enabled();
            });
            output.textures_delta.clear();
            assert!(!enabled);
            assert_eq!(locked, initial);
        }

        let ctx = egui::Context::default();
        let mut disabled = true;
        let mut enabled = false;
        let mut output = ctx.run_ui(Default::default(), |ui| {
            disabled = edge_lock_control(ui, false, 48.0, true, false, true).enabled();
            enabled = edge_lock_control(ui, false, 48.0, true, true, true).enabled();
        });
        output.textures_delta.clear();
        assert!(!disabled);
        assert!(enabled);
        assert_eq!(
            ACTION_BARS_UNAVAILABLE_TOOLTIP[1],
            "Locking is unavailable until the current session has at least one action-bar assignment."
        );
        assert_eq!(
            SETTINGS_LOCK_UNAVAILABLE_TOOLTIP[1],
            "Locking is unavailable in Settings without the current session assignment state."
        );
    }

    #[test]
    fn empty_live_slots_enable_only_for_a_real_assignment_path()
    -> Result<(), Box<dyn std::error::Error>> {
        assert!(!action_slot_enabled(true, false, false, false, false));
        assert!(action_slot_enabled(true, true, false, true, true));
        assert!(action_slot_enabled(true, false, true, false, true));
        assert!(action_slot_enabled(true, false, false, true, false));
        assert!(!action_slot_enabled(true, false, false, true, true));
        assert!(!action_slot_enabled(false, true, true, true, false));

        let render = |bar: &mut ActionBar, spells: &[AvailableSpell]| {
            let ctx = egui::Context::default();
            ctx.enable_accesskit();
            let screen = egui::vec2(800.0, 600.0);
            let body = Rect::from_min_size(egui::pos2(100.0, 80.0), egui::vec2(600.0, 400.0));
            let settings = ClientSettings {
                english: true,
                ..Default::default()
            };
            let mut output = egui::FullOutput::default();
            for _ in 0..2 {
                ctx.begin_pass(egui::RawInput {
                    screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, screen)),
                    ..Default::default()
                });
                let _ = show(
                    &ctx,
                    bar,
                    &GameState::default(),
                    spells,
                    &settings,
                    body,
                    true,
                );
                output = ctx.end_pass();
                output.textures_delta.clear();
            }
            output
        };
        let slot_disabled = |output: &egui::FullOutput, label: &str| {
            output
                .platform_output
                .accesskit_update
                .as_ref()
                .and_then(|update| {
                    update.nodes.iter().find_map(|(_, node)| {
                        (node.role() == egui::accesskit::Role::Button
                            && node.label() == Some(label))
                        .then(|| node.is_disabled())
                    })
                })
        };

        let mut empty = ActionBar::new(ActionBarPreferences::default(), &[])?;
        let output = render(&mut empty, &[]);
        assert_eq!(slot_disabled(&output, "Bottom 1 · 1: —, F1"), Some(true));

        let spell = AvailableSpell {
            label: "Exura".into(),
            spell: NonZeroU32::new(1).ok_or("spell id must be nonzero")?,
            target: SpellTarget::None,
            aim_at_target: false,
        };
        let output = render(&mut empty, &[spell]);
        assert_eq!(slot_disabled(&output, "Bottom 1 · 1: —, F1"), Some(false));

        let mut assigned = ActionBar::new(ActionBarPreferences::default(), &[])?;
        assigned.assign(
            0,
            Some(SlotAssignment {
                label: "SOURCE".into(),
                command: ActionBarCommand::Chat(ChatIntent::OpenRoom(ChatRoom::Help)),
            }),
        )?;
        let output = render(&mut assigned, &[]);
        assert_eq!(
            slot_disabled(&output, "Bottom 1 · 1: SOURCE, F1"),
            Some(false)
        );
        assert_eq!(
            slot_disabled(&output, "Bottom 1 · 2: —, F2"),
            Some(false),
            "an unlocked assignment makes empty slots real drop targets"
        );

        assigned.set_assignments_locked(true);
        let output = render(&mut assigned, &[]);
        assert_eq!(
            slot_disabled(&output, "Bottom 1 · 1: SOURCE, F1"),
            Some(false),
            "an assigned slot still activates and exposes its context menu"
        );
        assert_eq!(
            slot_disabled(&output, "Bottom 1 · 2: —, F2"),
            Some(true),
            "locking removes the empty slot's only live interaction"
        );
        assert_eq!(
            EMPTY_ACTION_SLOT_UNAVAILABLE_TOOLTIP[1],
            "This empty slot is unavailable because the current session provides neither an assignable spell nor an unlocked action to drop here."
        );
        Ok(())
    }

    #[test]
    fn only_primary_pointer_drag_moves_or_swaps_and_secondary_keeps_the_context_menu()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut bar = ActionBar::new(ActionBarPreferences::default(), &[])?;
        let source = SlotAssignment {
            label: "SOURCE".into(),
            command: ActionBarCommand::Chat(ChatIntent::OpenRoom(ChatRoom::Help)),
        };
        let target = SlotAssignment {
            label: "TARGET".into(),
            command: ActionBarCommand::Chat(ChatIntent::OpenRoom(ChatRoom::World)),
        };
        bar.assign(0, Some(source.clone()))?;
        bar.assign(1, Some(target.clone()))?;
        let ctx = egui::Context::default();
        let screen = egui::vec2(800.0, 600.0);
        let body = Rect::from_min_size(egui::pos2(100.0, 80.0), egui::vec2(600.0, 400.0));
        let settings = ClientSettings {
            english: true,
            ..Default::default()
        };
        let draw = |bar: &mut ActionBar, events| {
            ctx.begin_pass(egui::RawInput {
                screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, screen)),
                events,
                ..Default::default()
            });
            let commands = show(&ctx, bar, &GameState::default(), &[], &settings, body, true);
            let mut output = ctx.end_pass();
            output.textures_delta.clear();
            (commands, output)
        };
        let mut output = egui::FullOutput::default();
        for _ in 0..3 {
            output = draw(&mut bar, Vec::new()).1;
        }
        let text_center = |output: &egui::FullOutput, expected: &str| {
            output.shapes.iter().find_map(|shape| match &shape.shape {
                egui::epaint::Shape::Text(text) if text.galley.job.text == expected => {
                    Some(text.visual_bounding_rect().center())
                }
                _ => None,
            })
        };
        let source_pos = text_center(&output, "SOURCE").ok_or("source slot was not painted")?;
        let target_pos = text_center(&output, "TARGET").ok_or("target slot was not painted")?;
        let pointer = |pos, button, pressed| {
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
        draw(
            &mut bar,
            pointer(source_pos, egui::PointerButton::Primary, true),
        );
        draw(&mut bar, vec![egui::Event::PointerMoved(target_pos)]);
        draw(
            &mut bar,
            pointer(target_pos, egui::PointerButton::Primary, false),
        );
        assert_eq!(bar.assignment(0), Some(&target));
        assert_eq!(bar.assignment(1), Some(&source));

        let before = [bar.assignment(0).cloned(), bar.assignment(1).cloned()];
        draw(&mut bar, Vec::new());
        draw(
            &mut bar,
            pointer(source_pos, egui::PointerButton::Secondary, true),
        );
        draw(&mut bar, vec![egui::Event::PointerMoved(target_pos)]);
        draw(
            &mut bar,
            pointer(target_pos, egui::PointerButton::Secondary, false),
        );
        assert_eq!(
            [bar.assignment(0).cloned(), bar.assignment(1).cloned()],
            before
        );
        assert!(!egui::Popup::is_any_open(&ctx));

        draw(&mut bar, Vec::new());
        draw(
            &mut bar,
            pointer(source_pos, egui::PointerButton::Secondary, true),
        );
        draw(
            &mut bar,
            pointer(source_pos, egui::PointerButton::Secondary, false),
        );
        assert!(egui::Popup::is_any_open(&ctx));
        assert_eq!(
            [bar.assignment(0).cloned(), bar.assignment(1).cloned()],
            before
        );
        egui::Popup::close_all(&ctx);

        bar.set_assignments_locked(true);
        draw(&mut bar, Vec::new());
        draw(
            &mut bar,
            pointer(source_pos, egui::PointerButton::Primary, true),
        );
        draw(&mut bar, vec![egui::Event::PointerMoved(target_pos)]);
        draw(
            &mut bar,
            pointer(target_pos, egui::PointerButton::Primary, false),
        );
        assert_eq!(
            [bar.assignment(0).cloned(), bar.assignment(1).cloned()],
            before
        );

        bar.set_assignments_locked(false);
        let moving = bar.assignment(0).cloned().ok_or("missing drag source")?;
        bar.assign(1, None)?;
        draw(&mut bar, Vec::new());
        draw(
            &mut bar,
            pointer(source_pos, egui::PointerButton::Primary, true),
        );
        draw(&mut bar, vec![egui::Event::PointerMoved(target_pos)]);
        draw(
            &mut bar,
            pointer(target_pos, egui::PointerButton::Primary, false),
        );
        assert_eq!(bar.assignment(0), None);
        assert_eq!(bar.assignment(1), Some(&moving));
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
