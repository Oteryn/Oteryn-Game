//! Profile-based hotkey pages matching the reference table structure.

use oteryn_client::{
    action_bar::{ACTION_BAR_ROWS, ACTION_BAR_SLOTS, SlotShortcut},
    hotkeys::{
        CustomHotkey, CustomHotkeyAction, HotkeyPair, HotkeyProfile, MAX_HOTKEY_PROFILES,
        ObjectUseMode, general_action_supported,
    },
    settings::ClientSettings,
    settings_catalog::ShortcutBinding,
};

const GENERAL_ACTIONS: &[(&str, [&str; 2])] = &[
    (
        "action.bottom.1",
        [
            "Paski akcji: dolny 1",
            "Action Bar: Show/Hide Bottom Action Bar 1",
        ],
    ),
    (
        "action.bottom.2",
        [
            "Paski akcji: dolny 2",
            "Action Bar: Show/Hide Bottom Action Bar 2",
        ],
    ),
    (
        "action.bottom.3",
        [
            "Paski akcji: dolny 3",
            "Action Bar: Show/Hide Bottom Action Bar 3",
        ],
    ),
    (
        "action.bottom.all",
        [
            "Paski akcji: wszystkie dolne",
            "Action Bar: Show/Hide Bottom Action Bars",
        ],
    ),
    (
        "action.left.1",
        [
            "Paski akcji: lewy 1",
            "Action Bar: Show/Hide Left Action Bar 1",
        ],
    ),
    (
        "action.left.2",
        [
            "Paski akcji: lewy 2",
            "Action Bar: Show/Hide Left Action Bar 2",
        ],
    ),
    (
        "action.left.3",
        [
            "Paski akcji: lewy 3",
            "Action Bar: Show/Hide Left Action Bar 3",
        ],
    ),
    (
        "action.left.all",
        [
            "Paski akcji: wszystkie lewe",
            "Action Bar: Show/Hide Left Action Bars",
        ],
    ),
    (
        "action.right.1",
        [
            "Paski akcji: prawy 1",
            "Action Bar: Show/Hide Right Action Bar 1",
        ],
    ),
    (
        "action.right.2",
        [
            "Paski akcji: prawy 2",
            "Action Bar: Show/Hide Right Action Bar 2",
        ],
    ),
    (
        "action.right.3",
        [
            "Paski akcji: prawy 3",
            "Action Bar: Show/Hide Right Action Bar 3",
        ],
    ),
    (
        "action.right.all",
        [
            "Paski akcji: wszystkie prawe",
            "Action Bar: Show/Hide Right Action Bars",
        ],
    ),
    (
        "battle.first",
        [
            "Lista walki: pierwszy cel",
            "Battle List: Attack First Target",
        ],
    ),
    (
        "battle.next",
        [
            "Lista walki: następny cel",
            "Battle List: Attack Next Target",
        ],
    ),
    (
        "battle.previous",
        [
            "Lista walki: poprzedni cel",
            "Battle List: Attack Previous Target",
        ],
    ),
    (
        "chat.close",
        [
            "Kanał czatu: zamknij",
            "Chat Channel: Close Current Channel",
        ],
    ),
    (
        "chat.next",
        ["Kanał czatu: następny", "Chat Channel: Next Channel"],
    ),
    (
        "chat.list",
        [
            "Kanał czatu: lista kanałów",
            "Chat Channel: Open Channel List",
        ],
    ),
    (
        "chat.help",
        ["Kanał czatu: pomoc", "Chat Channel: Open Help Channel"],
    ),
    (
        "chat.loot",
        ["Kanał czatu: łup", "Chat Channel: Open Loot Channel"],
    ),
    ("movement.north", ["Ruch: północ", "Movement: North"]),
    ("movement.east", ["Ruch: wschód", "Movement: East"]),
    ("movement.south", ["Ruch: południe", "Movement: South"]),
    ("movement.west", ["Ruch: zachód", "Movement: West"]),
    (
        "client.options",
        ["Klient: ustawienia", "Client: Open Options"],
    ),
    (
        "chat.previous",
        ["Kanał czatu: poprzedni", "Chat Channel: Previous Channel"],
    ),
    (
        "chat.local",
        ["Kanał czatu: lokalny", "Chat Channel: Switch to Local Chat"],
    ),
    (
        "chat.server_messages",
        ["Czat: komunikaty serwera", "Chat: Toggle Server Messages"],
    ),
    (
        "client.hotkeys",
        ["Klient: ustawienia skrótów", "Client: Open Hotkeys"],
    ),
    (
        "client.ignore",
        ["Klient: lista ignorowanych", "Client: Open Ignore List"],
    ),
    (
        "client.bug_report",
        ["Klient: zgłoszenie błędu", "Client: Bug Report"],
    ),
    (
        "client.change_character",
        ["Klient: zmień postać", "Client: Change Character"],
    ),
    ("client.logout", ["Klient: wyloguj", "Client: Logout"]),
    (
        "client.next_profile",
        [
            "Klient: następny profil skrótów",
            "Client: Next Hotkey Preset",
        ],
    ),
    (
        "client.stop",
        [
            "Klient: zatrzymaj wszystkie akcje",
            "Client: Stop All Actions",
        ],
    ),
    (
        "client.fullscreen",
        ["Interfejs: pełny ekran", "Interface: Toggle Fullscreen"],
    ),
    (
        "client.fps",
        [
            "Interfejs: FPS i opóźnienie",
            "Interface: Toggle FPS/Lag Indicator",
        ],
    ),
    (
        "client.names",
        [
            "Interfejs: nazwy i paski",
            "Interface: Toggle Creature Names/Bars",
        ],
    ),
    (
        "client.clear_message",
        [
            "Okno gry: usuń najstarszy komunikat",
            "Game Window: Clear Oldest Message",
        ],
    ),
    (
        "client.mount",
        ["Postać: wsiądź/zsiądź", "Character: Mount/Dismount"],
    ),
    ("dialog.prey", ["Okno: Prey", "Dialog: Open Prey"]),
    (
        "dialog.quest_log",
        ["Okno: dziennik zadań", "Dialog: Open Quest Log"],
    ),
    (
        "dialog.lenshelp",
        ["Okno: Lenshelp", "Dialog: Open Lenshelp"],
    ),
    ("window.vip", ["Panel: lista VIP", "Window: VIP List"]),
    (
        "window.battle",
        ["Panel: lista walki", "Window: Battle List"],
    ),
    ("window.skills", ["Panel: umiejętności", "Window: Skills"]),
    ("minimap.zoom_out", ["Minimapa: oddal", "Minimap: Zoom Out"]),
    (
        "minimap.zoom_in",
        ["Minimapa: przybliż", "Minimap: Zoom In"],
    ),
    (
        "minimap.north",
        ["Minimapa: przewiń na północ", "Minimap: Scroll North"],
    ),
    (
        "minimap.east",
        ["Minimapa: przewiń na wschód", "Minimap: Scroll East"],
    ),
    (
        "minimap.south",
        ["Minimapa: przewiń na południe", "Minimap: Scroll South"],
    ),
    (
        "minimap.west",
        ["Minimapa: przewiń na zachód", "Minimap: Scroll West"],
    ),
    (
        "minimap.floor_up",
        ["Minimapa: piętro wyżej", "Minimap: Floor Up"],
    ),
    (
        "minimap.floor_down",
        ["Minimapa: piętro niżej", "Minimap: Floor Down"],
    ),
];

#[derive(Default)]
pub(super) struct State {
    search: String,
    search_applied: String,
    search_changed_at: Option<f64>,
    chat_on: bool,
    profile_edit: Option<ProfileEdit>,
    remove_confirmation: bool,
    new_text: String,
    send_automatically: bool,
    new_action: Option<NewAction>,
    spell_key: String,
    spell_parameter: String,
    learned_spells_only: bool,
    object_definition: String,
    object_use_mode: ObjectUseMode,
    binding_editor: Option<BindingEditor>,
}

#[derive(Clone)]
struct BindingEditor {
    target: BindingTarget,
    action: String,
    captured: Option<ShortcutBinding>,
    listening: bool,
}

#[derive(Clone)]
enum BindingTarget {
    General {
        action: &'static str,
        chat_on: bool,
        second: bool,
    },
    ActionBar {
        row: usize,
        slot: usize,
        chat_on: bool,
        second: bool,
    },
    Custom {
        index: usize,
        chat_on: bool,
        second: bool,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum NewAction {
    Choose,
    Spell,
    Object,
    Text,
}

struct ProfileEdit {
    operation: ProfileOperation,
    name: String,
}

#[derive(Clone, Copy)]
enum ProfileOperation {
    Add,
    Copy,
    Rename,
}

pub(super) fn show(
    ui: &mut egui::Ui,
    draft: &mut ClientSettings,
    section: &str,
    en: bool,
    state: &mut State,
) -> bool {
    if !matches!(
        section,
        "general_hotkeys" | "action_hotkeys" | "custom_hotkeys"
    ) {
        return false;
    }
    header(ui, draft, en, state);
    match section {
        "general_hotkeys" => general(ui, draft, en, state),
        "action_hotkeys" => action_bars(ui, draft, en, state),
        "custom_hotkeys" => custom(ui, draft, en, state),
        _ => unreachable!(),
    }
    binding_editor(ui, draft, en, state);
    if draft.validate_hotkey_draft().is_err() {
        ui.colored_label(
            ui.visuals().error_fg_color,
            if en {
                "Conflicting or invalid shortcut. Clear one binding before applying."
            } else {
                "Skrót jest nieprawidłowy lub użyty ponownie. Wyczyść jedno przypisanie przed zastosowaniem."
            },
        );
    }
    true
}

fn header(ui: &mut egui::Ui, draft: &mut ClientSettings, en: bool, state: &mut State) {
    let selected = draft.hotkeys.selected;
    ui.horizontal(|ui| {
        let name = draft
            .hotkeys
            .active()
            .map_or("—", |profile| profile.name.as_str());
        egui::ComboBox::from_id_salt("hotkey-profile")
            .selected_text(name)
            .show_ui(ui, |ui| {
                let choices: Vec<_> = draft
                    .hotkeys
                    .profiles
                    .iter()
                    .map(|p| p.name.clone())
                    .collect();
                for (index, name) in choices.into_iter().enumerate() {
                    if ui.selectable_label(index == selected, name).clicked() {
                        let _ = draft.select_hotkey_profile(index);
                    }
                }
            });
        for (label, operation) in [
            (("Dodaj", "Add"), ProfileOperation::Add),
            (("Kopiuj", "Copy"), ProfileOperation::Copy),
            (("Zmień nazwę", "Rename"), ProfileOperation::Rename),
        ] {
            let can_create = matches!(operation, ProfileOperation::Rename)
                || draft.hotkeys.profiles.len() < MAX_HOTKEY_PROFILES;
            if ui
                .add_enabled(
                    can_create,
                    egui::Button::new(if en { label.1 } else { label.0 }),
                )
                .clicked()
            {
                let name = if matches!(operation, ProfileOperation::Rename) {
                    draft
                        .hotkeys
                        .active()
                        .map_or(String::new(), |p| p.name.clone())
                } else {
                    String::new()
                };
                state.profile_edit = Some(ProfileEdit { operation, name });
            }
        }
        if ui
            .add_enabled(
                draft.hotkeys.profiles.len() > 1,
                egui::Button::new(if en { "Remove" } else { "Usuń" }),
            )
            .clicked()
        {
            state.remove_confirmation = true;
        }
    });
    if let Some(edit) = &mut state.profile_edit {
        let valid_name = profile_name_available(draft, edit);
        let show_invalid_name = !edit.name.trim().is_empty() && !valid_name;
        let mut close_editor = false;
        egui::Modal::new(egui::Id::new("hotkey-profile-editor")).show(ui.ctx(), |ui| {
            ui.heading(match edit.operation {
                ProfileOperation::Add => {
                    if en {
                        "Add Hotkey Preset"
                    } else {
                        "Dodaj profil skrótów"
                    }
                }
                ProfileOperation::Copy => {
                    if en {
                        "Copy Hotkey Preset"
                    } else {
                        "Kopiuj profil skrótów"
                    }
                }
                ProfileOperation::Rename => {
                    if en {
                        "Rename Hotkey Preset"
                    } else {
                        "Zmień nazwę profilu skrótów"
                    }
                }
            });
            ui.label(if en {
                "Profile name:"
            } else {
                "Nazwa profilu:"
            });
            ui.add(egui::TextEdit::singleline(&mut edit.name).char_limit(48));
            if show_invalid_name {
                ui.colored_label(
                    ui.visuals().error_fg_color,
                    if en {
                        "Profile names must be unique."
                    } else {
                        "Nazwy profili muszą być unikalne."
                    },
                );
            }
            ui.horizontal(|ui| {
                if ui
                    .add_enabled(valid_name, egui::Button::new("OK"))
                    .clicked()
                {
                    apply_profile_edit(draft, edit);
                    close_editor = true;
                }
                if ui.button(if en { "Cancel" } else { "Anuluj" }).clicked() {
                    close_editor = true;
                }
            });
        });
        if close_editor {
            state.profile_edit = None;
        }
    }
    if state.remove_confirmation {
        egui::Modal::new(egui::Id::new("hotkey-profile-remove")).show(ui.ctx(), |ui| {
            ui.heading(if en {
                "Remove Hotkey Preset"
            } else {
                "Usuń profil skrótów"
            });
            ui.label(if en {
                "Remove the selected profile?"
            } else {
                "Usunąć wybrany profil?"
            });
            ui.horizontal(|ui| {
                if ui.button(if en { "Remove" } else { "Usuń" }).clicked() {
                    let selected = draft.hotkeys.selected;
                    draft.hotkeys.profiles.remove(selected);
                    draft.hotkeys.selected = selected.min(draft.hotkeys.profiles.len() - 1);
                    draft.action_bar = draft.hotkeys.profiles[draft.hotkeys.selected]
                        .action_bar
                        .clone();
                    state.remove_confirmation = false;
                }
                if ui.button(if en { "Cancel" } else { "Anuluj" }).clicked() {
                    state.remove_confirmation = false;
                }
            });
        });
    }
    ui.checkbox(
        &mut draft.hotkeys.auto_switch,
        if en {
            "Auto-Switch Hotkey Preset"
        } else {
            "Automatycznie przełączaj profil skrótów"
        },
    );
    ui.horizontal(|ui| {
        ui.radio_value(
            &mut state.chat_on,
            true,
            if en {
                "Chat Mode On"
            } else {
                "Tryb czatu włączony"
            },
        );
        ui.radio_value(
            &mut state.chat_on,
            false,
            if en {
                "Chat Mode Off"
            } else {
                "Tryb czatu wyłączony"
            },
        );
    });
    ui.horizontal(|ui| {
        ui.label(if en {
            "Type to search for a hotkey:"
        } else {
            "Wyszukaj skrót:"
        });
        let response = ui.add(
            egui::TextEdit::singleline(&mut state.search)
                .desired_width(ui.available_width() - 28.0)
                .char_limit(128),
        );
        if response.changed() {
            state.search_changed_at = Some(ui.input(|input| input.time));
        }
        if ui.button("×").clicked() {
            state.search.clear();
            state.search_applied.clear();
            state.search_changed_at = None;
        }
    });
    if let Some(changed_at) = state.search_changed_at {
        let elapsed = ui.input(|input| input.time) - changed_at;
        if elapsed >= 0.25 {
            state.search_applied.clone_from(&state.search);
            state.search_changed_at = None;
        } else {
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_secs_f64(0.25 - elapsed));
        }
    }
}

fn apply_profile_edit(draft: &mut ClientSettings, edit: &ProfileEdit) {
    let name = edit.name.trim().to_owned();
    match edit.operation {
        ProfileOperation::Add => {
            let profile = HotkeyProfile {
                name,
                ..HotkeyProfile::default()
            };
            draft.hotkeys.profiles.push(profile);
            let _ = draft.select_hotkey_profile(draft.hotkeys.profiles.len() - 1);
        }
        ProfileOperation::Copy => {
            if let Some(mut profile) = draft.hotkeys.active().cloned() {
                profile.name = name;
                profile.action_bar = draft.action_bar.clone();
                draft.hotkeys.profiles.push(profile);
                let _ = draft.select_hotkey_profile(draft.hotkeys.profiles.len() - 1);
            }
        }
        ProfileOperation::Rename => {
            if let Some(profile) = draft.hotkeys.active_mut() {
                profile.name = name;
            }
        }
    }
}

fn profile_name_available(draft: &ClientSettings, edit: &ProfileEdit) -> bool {
    let candidate = edit.name.trim();
    !candidate.is_empty()
        && candidate.len() <= 48
        && !candidate.chars().any(char::is_control)
        && !draft
            .hotkeys
            .profiles
            .iter()
            .enumerate()
            .any(|(index, profile)| {
                let editing_current = matches!(edit.operation, ProfileOperation::Rename)
                    && index == draft.hotkeys.selected;
                !editing_current && profile.name.eq_ignore_ascii_case(candidate)
            })
}

fn general(ui: &mut egui::Ui, draft: &mut ClientSettings, en: bool, state: &mut State) {
    let Some(profile) = draft.hotkeys.active_mut() else {
        return;
    };
    let map = if state.chat_on {
        &mut profile.general_chat_on
    } else {
        &mut profile.general_chat_off
    };
    table_header(ui, en);
    egui::Grid::new("general-hotkey-table")
        .striped(true)
        .num_columns(3)
        .show(ui, |ui| {
            for (id, labels) in GENERAL_ACTIONS {
                let label = labels[usize::from(en)];
                if !matches_search(&state.search_applied, label) {
                    continue;
                }
                ui.label(label);
                let pair = map.get(*id).copied().unwrap_or_default();
                if general_action_supported(id) {
                    binding_button(
                        ui,
                        pair.first,
                        BindingTarget::General {
                            action: id,
                            chat_on: state.chat_on,
                            second: false,
                        },
                        label,
                        en,
                        &mut state.binding_editor,
                    );
                    binding_button(
                        ui,
                        pair.second,
                        BindingTarget::General {
                            action: id,
                            chat_on: state.chat_on,
                            second: true,
                        },
                        label,
                        en,
                        &mut state.binding_editor,
                    );
                } else {
                    unavailable_binding(ui, pair.first, en);
                    unavailable_binding(ui, pair.second, en);
                }
                ui.end_row();
            }
        });
}

fn action_bars(ui: &mut egui::Ui, draft: &mut ClientSettings, en: bool, state: &mut State) {
    table_header(ui, en);
    egui::Grid::new("action-hotkey-table")
        .striped(true)
        .num_columns(3)
        .show(ui, |ui| {
            for row in 0..ACTION_BAR_ROWS {
                let Some(preferences) = draft.action_bar.row(row) else {
                    continue;
                };
                for slot in 0..ACTION_BAR_SLOTS {
                    let label = format!(
                        "{} Action Bar: Action Button {}.{:02}",
                        edge(row),
                        row + 1,
                        slot + 1
                    );
                    if !matches_search(&state.search_applied, &label) {
                        continue;
                    }
                    ui.label(&label);
                    let (primary, secondary) = if state.chat_on {
                        (
                            &preferences.chat_on_shortcuts,
                            &preferences.chat_on_secondary_shortcuts,
                        )
                    } else {
                        (&preferences.shortcuts, &preferences.secondary_shortcuts)
                    };
                    binding_button(
                        ui,
                        primary[slot].map(slot_to_binding),
                        BindingTarget::ActionBar {
                            row,
                            slot,
                            chat_on: state.chat_on,
                            second: false,
                        },
                        &label,
                        en,
                        &mut state.binding_editor,
                    );
                    binding_button(
                        ui,
                        secondary[slot].map(slot_to_binding),
                        BindingTarget::ActionBar {
                            row,
                            slot,
                            chat_on: state.chat_on,
                            second: true,
                        },
                        &label,
                        en,
                        &mut state.binding_editor,
                    );
                    ui.end_row();
                }
            }
        });
}

fn custom(ui: &mut egui::Ui, draft: &mut ClientSettings, en: bool, state: &mut State) {
    let Some(profile) = draft.hotkeys.active_mut() else {
        return;
    };
    table_header(ui, en);
    let mut remove = None;
    egui::Grid::new("custom-hotkey-table")
        .striped(true)
        .num_columns(3)
        .show(ui, |ui| {
            for (index, action) in profile.custom.iter_mut().enumerate() {
                let label = custom_label(&action.action, en);
                if !matches_search(&state.search_applied, &label) {
                    continue;
                }
                ui.label(&label).context_menu(|ui| {
                    if ui.button(if en { "Remove" } else { "Usuń" }).clicked() {
                        remove = Some(index);
                        ui.close();
                    }
                });
                let pair = if state.chat_on {
                    &mut action.chat_on
                } else {
                    &mut action.chat_off
                };
                if matches!(&action.action, CustomHotkeyAction::Text { .. }) {
                    binding_button(
                        ui,
                        pair.first,
                        BindingTarget::Custom {
                            index,
                            chat_on: state.chat_on,
                            second: false,
                        },
                        &label,
                        en,
                        &mut state.binding_editor,
                    );
                    binding_button(
                        ui,
                        pair.second,
                        BindingTarget::Custom {
                            index,
                            chat_on: state.chat_on,
                            second: true,
                        },
                        &label,
                        en,
                        &mut state.binding_editor,
                    );
                } else {
                    unavailable_binding(ui, pair.first, en);
                    unavailable_binding(ui, pair.second, en);
                }
                ui.end_row();
            }
        });
    if let Some(index) = remove {
        profile.custom.remove(index);
    }
    if ui
        .button(if en { "New Action" } else { "Nowa czynność" })
        .clicked()
    {
        state.new_action = Some(NewAction::Choose);
    }
    match state.new_action {
        Some(NewAction::Choose) => choose_new_action(ui, en, state),
        Some(kind) => edit_new_action(ui, profile, en, state, kind),
        None => {}
    }
}

fn choose_new_action(ui: &mut egui::Ui, en: bool, state: &mut State) {
    egui::Modal::new(egui::Id::new("custom-action-choose")).show(ui.ctx(), |ui| {
        ui.heading(if en { "New Action" } else { "Nowa czynność" });
        ui.horizontal(|ui| {
            for (kind, pl, english, available) in [
                (NewAction::Spell, "Przypisz czar", "Assign Spell", false),
                (
                    NewAction::Object,
                    "Przypisz przedmiot",
                    "Assign Object",
                    false,
                ),
                (NewAction::Text, "Przypisz tekst", "Assign Text", true),
            ] {
                if ui
                    .add_enabled(available, egui::Button::new(if en { english } else { pl }))
                    .on_hover_text(if available {
                        if en {
                            "Text actions are routed to the real chat composer."
                        } else {
                            "Akcje tekstowe trafiają do rzeczywistego pola czatu."
                        }
                    } else if en {
                        "Requires an authoritative live spell or inventory picker."
                    } else {
                        "Wymaga autorytatywnego katalogu czarów lub ekwipunku sesji."
                    })
                    .clicked()
                {
                    state.new_action = Some(kind);
                }
            }
            if ui.button(if en { "Cancel" } else { "Anuluj" }).clicked() {
                state.new_action = None;
            }
        });
    });
}

fn edit_new_action(
    ui: &mut egui::Ui,
    profile: &mut HotkeyProfile,
    en: bool,
    state: &mut State,
    kind: NewAction,
) {
    let title = match kind {
        NewAction::Spell => ("Przypisz czar", "Assign Spell"),
        NewAction::Object => ("Przypisz przedmiot", "Assign Object"),
        NewAction::Text => ("Przypisz tekst", "Assign Text"),
        NewAction::Choose => return,
    };
    egui::Modal::new(egui::Id::new("custom-action-editor")).show(ui.ctx(), |ui| {
        ui.heading(if en { title.1 } else { title.0 });
        match kind {
            NewAction::Spell => {
                ui.label(if en {
                    "Search spells:"
                } else {
                    "Wyszukaj czar:"
                });
                ui.add(
                    egui::TextEdit::singleline(&mut state.spell_key)
                        .char_limit(128)
                        .hint_text(if en {
                            "Spell name or words"
                        } else {
                            "Nazwa lub słowa czaru"
                        }),
                );
                ui.checkbox(
                    &mut state.learned_spells_only,
                    if en {
                        "Learnt spells only"
                    } else {
                        "Tylko poznane czary"
                    },
                );
                ui.label(if en { "Parameter:" } else { "Parametr:" });
                ui.add(egui::TextEdit::singleline(&mut state.spell_parameter).char_limit(128));
            }
            NewAction::Object => {
                ui.label(if en {
                    "Select an object from the current inventory or enter its object number:"
                } else {
                    "Wybierz przedmiot z bieżącego ekwipunku lub wpisz jego numer:"
                });
                ui.add(
                    egui::TextEdit::singleline(&mut state.object_definition)
                        .char_limit(10)
                        .hint_text(if en {
                            "Object number"
                        } else {
                            "Numer przedmiotu"
                        }),
                );
                ui.add_space(6.0);
                for (mode, pl, english) in [
                    (ObjectUseMode::OnSelf, "Użyj na sobie", "Use on yourself"),
                    (ObjectUseMode::OnTarget, "Użyj na celu", "Use on target"),
                    (
                        ObjectUseMode::Crosshair,
                        "Użyj z celownikiem",
                        "With crosshair",
                    ),
                    (
                        ObjectUseMode::AtCursor,
                        "Użyj w pozycji kursora",
                        "Use at cursor position",
                    ),
                    (
                        ObjectUseMode::EquipUnequip,
                        "Załóż/zdejmij",
                        "Equip/unequip",
                    ),
                    (ObjectUseMode::Use, "Użyj", "Use"),
                ] {
                    ui.radio_value(
                        &mut state.object_use_mode,
                        mode,
                        if en { english } else { pl },
                    );
                }
            }
            NewAction::Text => {
                ui.add(
                    egui::TextEdit::singleline(&mut state.new_text)
                        .char_limit(256)
                        .hint_text(if en { "Text" } else { "Tekst" }),
                );
                ui.checkbox(
                    &mut state.send_automatically,
                    if en {
                        "Send automatically"
                    } else {
                        "Wyślij automatycznie"
                    },
                );
            }
            NewAction::Choose => {}
        }
        let valid = new_action_valid(state, kind);
        ui.horizontal(|ui| {
            if ui.add_enabled(valid, egui::Button::new("OK")).clicked() {
                add_new_action(profile, state, kind);
                state.new_action = None;
            }
            if ui
                .add_enabled(
                    valid,
                    egui::Button::new(if en { "Apply" } else { "Zastosuj" }),
                )
                .clicked()
            {
                add_new_action(profile, state, kind);
            }
            if ui.button(if en { "Back" } else { "Wstecz" }).clicked() {
                state.new_action = Some(NewAction::Choose);
            }
            if ui.button(if en { "Cancel" } else { "Anuluj" }).clicked() {
                state.new_action = None;
            }
        });
    });
}

fn new_action_valid(state: &State, kind: NewAction) -> bool {
    match kind {
        NewAction::Spell => !state.spell_key.trim().is_empty(),
        NewAction::Object => state
            .object_definition
            .parse::<u32>()
            .is_ok_and(|definition| definition != 0),
        NewAction::Text => !state.new_text.is_empty(),
        NewAction::Choose => false,
    }
}

fn add_new_action(profile: &mut HotkeyProfile, state: &mut State, kind: NewAction) {
    let action = match kind {
        NewAction::Spell => CustomHotkeyAction::Spell {
            spell_key: std::mem::take(&mut state.spell_key),
            parameter: std::mem::take(&mut state.spell_parameter),
        },
        NewAction::Object => {
            let Some(item_definition_ref) = state.object_definition.parse::<u32>().ok() else {
                return;
            };
            state.object_definition.clear();
            CustomHotkeyAction::Object {
                item_definition_ref,
                use_mode: state.object_use_mode,
            }
        }
        NewAction::Text => CustomHotkeyAction::Text {
            text: std::mem::take(&mut state.new_text),
            send_automatically: state.send_automatically,
        },
        NewAction::Choose => return,
    };
    profile.custom.push(CustomHotkey {
        action,
        chat_on: HotkeyPair::default(),
        chat_off: HotkeyPair::default(),
    });
}

fn table_header(ui: &mut egui::Ui, en: bool) {
    ui.columns(3, |cols| {
        cols[0].strong(if en { "Action" } else { "Czynność" });
        cols[1].strong(if en { "First Key" } else { "Pierwszy skrót" });
        cols[2].strong(if en { "Second Key" } else { "Drugi skrót" });
    });
}

fn binding_button(
    ui: &mut egui::Ui,
    value: Option<ShortcutBinding>,
    target: BindingTarget,
    action: &str,
    en: bool,
    editor: &mut Option<BindingEditor>,
) {
    let response =
        ui.button(value.map_or_else(|| if en { "None".into() } else { "Brak".into() }, chord));
    if response.clicked() {
        *editor = Some(BindingEditor {
            target,
            action: action.into(),
            captured: value,
            listening: true,
        });
    }
}

fn binding_editor(ui: &mut egui::Ui, draft: &mut ClientSettings, en: bool, state: &mut State) {
    let Some(mut editor) = state.binding_editor.take() else {
        return;
    };
    let mut close = false;
    if editor.listening {
        let event = ui.input(|input| {
            input.events.iter().find_map(|event| {
                let egui::Event::Key {
                    key,
                    physical_key,
                    pressed: true,
                    repeat: false,
                    modifiers,
                } = event
                else {
                    return None;
                };
                Some((*key, physical_key.unwrap_or(*key), *modifiers))
            })
        });
        if let Some((logical, physical, modifiers)) = event {
            if logical == egui::Key::Escape {
                close = true;
            } else if let Some(key) = egui_key_to_hid(physical) {
                editor.captured = Some(ShortcutBinding {
                    key,
                    ctrl: modifiers.ctrl,
                    alt: modifiers.alt,
                    shift: modifiers.shift,
                    meta: modifiers.mac_cmd,
                });
                editor.listening = false;
            }
        }
    }
    let valid = binding_candidate_valid(draft, &editor.target, editor.captured);
    egui::Modal::new(egui::Id::new("hotkey-binding-editor")).show(ui.ctx(), |ui| {
        ui.heading(if en { "Set Hotkey" } else { "Ustaw skrót" });
        ui.label(&editor.action);
        ui.weak(if editor.target.chat_on() {
            if en {
                "Chat Mode On"
            } else {
                "Tryb czatu włączony"
            }
        } else if en {
            "Chat Mode Off"
        } else {
            "Tryb czatu wyłączony"
        });
        ui.add_space(6.0);
        ui.label(if editor.listening {
            if en {
                "Press a key…"
            } else {
                "Naciśnij klawisz…"
            }
        } else {
            ""
        });
        ui.strong(
            editor
                .captured
                .map_or_else(|| if en { "None".into() } else { "Brak".into() }, chord),
        );
        if !valid && editor.captured.is_some() {
            ui.colored_label(
                ui.visuals().error_fg_color,
                if en {
                    "This key conflicts with another action or is reserved."
                } else {
                    "Ten klawisz koliduje z inną czynnością lub jest zarezerwowany."
                },
            );
        }
        ui.horizontal(|ui| {
            if ui
                .button(if en { "Capture" } else { "Przechwyć" })
                .clicked()
            {
                editor.listening = true;
            }
            if ui.add_enabled(valid, egui::Button::new("OK")).clicked() {
                let _ = set_binding(draft, &editor.target, editor.captured);
                close = true;
            }
            if ui.button(if en { "Clear" } else { "Wyczyść" }).clicked() {
                let _ = set_binding(draft, &editor.target, None);
                close = true;
            }
            if ui.button(if en { "Cancel" } else { "Anuluj" }).clicked() {
                close = true;
            }
        });
    });
    if !close {
        state.binding_editor = Some(editor);
    }
}

impl BindingTarget {
    const fn chat_on(&self) -> bool {
        match self {
            Self::General { chat_on, .. }
            | Self::ActionBar { chat_on, .. }
            | Self::Custom { chat_on, .. } => *chat_on,
        }
    }
}

fn binding_candidate_valid(
    draft: &ClientSettings,
    target: &BindingTarget,
    value: Option<ShortcutBinding>,
) -> bool {
    let mut candidate = draft.clone();
    set_binding(&mut candidate, target, value) && candidate.validate_hotkey_draft().is_ok()
}

fn set_binding(
    draft: &mut ClientSettings,
    target: &BindingTarget,
    value: Option<ShortcutBinding>,
) -> bool {
    match target {
        BindingTarget::General {
            action,
            chat_on,
            second,
        } => {
            let Some(profile) = draft.hotkeys.active_mut() else {
                return false;
            };
            let map = if *chat_on {
                &mut profile.general_chat_on
            } else {
                &mut profile.general_chat_off
            };
            let mut pair = map.get(*action).copied().unwrap_or_default();
            if *second {
                pair.second = value;
            } else {
                pair.first = value;
            }
            if pair == HotkeyPair::default() {
                map.remove(*action);
            } else {
                map.insert((*action).into(), pair);
            }
            true
        }
        BindingTarget::ActionBar {
            row,
            slot,
            chat_on,
            second,
        } => {
            let Some(mut preferences) = draft.action_bar.row(*row) else {
                return false;
            };
            let Some(shortcut) = (if *chat_on && *second {
                preferences.chat_on_secondary_shortcuts.get_mut(*slot)
            } else if *chat_on {
                preferences.chat_on_shortcuts.get_mut(*slot)
            } else if *second {
                preferences.secondary_shortcuts.get_mut(*slot)
            } else {
                preferences.shortcuts.get_mut(*slot)
            }) else {
                return false;
            };
            *shortcut = value.map(binding_to_slot);
            draft.action_bar.set_row(*row, preferences).is_ok()
        }
        BindingTarget::Custom {
            index,
            chat_on,
            second,
        } => {
            let Some(action) = draft
                .hotkeys
                .active_mut()
                .and_then(|profile| profile.custom.get_mut(*index))
            else {
                return false;
            };
            let pair = if *chat_on {
                &mut action.chat_on
            } else {
                &mut action.chat_off
            };
            if *second {
                pair.second = value;
            } else {
                pair.first = value;
            }
            true
        }
    }
}

fn slot_to_binding(value: SlotShortcut) -> ShortcutBinding {
    ShortcutBinding {
        key: value.key,
        shift: value.modifiers & 1 != 0,
        ctrl: value.modifiers & 2 != 0,
        alt: value.modifiers & 4 != 0,
        meta: value.modifiers & 8 != 0,
    }
}

fn binding_to_slot(value: ShortcutBinding) -> SlotShortcut {
    SlotShortcut {
        key: value.key,
        modifiers: u8::from(value.shift)
            | (u8::from(value.ctrl) << 1)
            | (u8::from(value.alt) << 2)
            | (u8::from(value.meta) << 3),
    }
}

fn egui_key_to_hid(key: egui::Key) -> Option<u16> {
    use egui::Key;
    Some(match key {
        Key::A => 4,
        Key::B => 5,
        Key::C => 6,
        Key::D => 7,
        Key::E => 8,
        Key::F => 9,
        Key::G => 10,
        Key::H => 11,
        Key::I => 12,
        Key::J => 13,
        Key::K => 14,
        Key::L => 15,
        Key::M => 16,
        Key::N => 17,
        Key::O => 18,
        Key::P => 19,
        Key::Q => 20,
        Key::R => 21,
        Key::S => 22,
        Key::T => 23,
        Key::U => 24,
        Key::V => 25,
        Key::W => 26,
        Key::X => 27,
        Key::Y => 28,
        Key::Z => 29,
        Key::Num1 => 30,
        Key::Num2 => 31,
        Key::Num3 => 32,
        Key::Num4 => 33,
        Key::Num5 => 34,
        Key::Num6 => 35,
        Key::Num7 => 36,
        Key::Num8 => 37,
        Key::Num9 => 38,
        Key::Num0 => 39,
        Key::Enter => 40,
        Key::Backspace => 42,
        Key::Tab => 43,
        Key::Space => 44,
        Key::Insert => 73,
        Key::Home => 74,
        Key::PageUp => 75,
        Key::Delete => 76,
        Key::End => 77,
        Key::PageDown => 78,
        Key::ArrowRight => 79,
        Key::ArrowLeft => 80,
        Key::ArrowDown => 81,
        Key::ArrowUp => 82,
        Key::F1 => 58,
        Key::F2 => 59,
        Key::F3 => 60,
        Key::F4 => 61,
        Key::F5 => 62,
        Key::F6 => 63,
        Key::F7 => 64,
        Key::F8 => 65,
        Key::F9 => 66,
        Key::F10 => 67,
        Key::F11 => 68,
        Key::F12 => 69,
        _ => return None,
    })
}

fn unavailable_binding(ui: &mut egui::Ui, value: Option<ShortcutBinding>, en: bool) {
    ui.add_enabled(
        false,
        egui::Button::new(value.map_or_else(
            || {
                if en {
                    "Unavailable".into()
                } else {
                    "Niedostępne".into()
                }
            },
            chord,
        )),
    )
    .on_hover_text(if en {
        "This action has no client consumer yet."
    } else {
        "Ta czynność nie ma jeszcze obsługi w kliencie."
    });
}

fn matches_search(search: &str, label: &str) -> bool {
    search.trim().is_empty() || label.to_lowercase().contains(&search.trim().to_lowercase())
}

fn edge(row: usize) -> &'static str {
    match row / 3 {
        0 => "Bottom",
        1 => "Left",
        _ => "Right",
    }
}
fn key_name(key: u16) -> String {
    match key {
        4..=29 => char::from_u32(u32::from(b'A') + u32::from(key - 4))
            .unwrap_or('?')
            .to_string(),
        30..=38 => (key - 29).to_string(),
        39 => "0".into(),
        43 => "Tab".into(),
        44 => "Space".into(),
        58..=69 => format!("F{}", key - 57),
        79 => "→".into(),
        80 => "←".into(),
        81 => "↓".into(),
        82 => "↑".into(),
        _ => format!("HID {key}"),
    }
}
fn chord(value: ShortcutBinding) -> String {
    let mut parts = Vec::new();
    if value.ctrl {
        parts.push("Ctrl".into());
    }
    if value.shift {
        parts.push("Shift".into());
    }
    if value.alt {
        parts.push("Alt".into());
    }
    if value.meta {
        parts.push("Super".into());
    }
    parts.push(key_name(value.key));
    parts.join("+")
}
fn custom_label(action: &CustomHotkeyAction, en: bool) -> String {
    match action {
        CustomHotkeyAction::Text { text, .. } => {
            format!("{}: {text}", if en { "Text" } else { "Tekst" })
        }
        CustomHotkeyAction::Spell { spell_key, .. } => {
            format!("{}: {spell_key}", if en { "Spell" } else { "Czar" })
        }
        CustomHotkeyAction::Object {
            item_definition_ref,
            ..
        } => format!(
            "{}: #{item_definition_ref}",
            if en { "Object" } else { "Przedmiot" }
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_hotkey_pages_render_for_both_languages() {
        for english in [false, true] {
            for section in ["general_hotkeys", "action_hotkeys", "custom_hotkeys"] {
                let context = egui::Context::default();
                let mut settings = ClientSettings::default();
                let mut state = State::default();
                let mut output = context.run_ui(egui::RawInput::default(), |ui| {
                    assert!(show(ui, &mut settings, section, english, &mut state));
                });
                output.textures_delta.clear();
            }
        }
    }

    #[test]
    fn profile_copy_keeps_the_complete_action_bar() -> Result<(), Box<dyn std::error::Error>> {
        let mut settings = ClientSettings::default();
        settings.action_bar.extra_rows[7].visible = true;
        settings.action_bar.secondary_shortcuts[49] = Some(SlotShortcut {
            key: 4,
            modifiers: 2,
        });
        apply_profile_edit(
            &mut settings,
            &ProfileEdit {
                operation: ProfileOperation::Copy,
                name: "Knight".into(),
            },
        );
        assert_eq!(settings.hotkeys.profiles.len(), 2);
        assert_eq!(settings.hotkeys.selected, 1);
        assert_eq!(
            settings.hotkeys.active().ok_or("active profile")?.name,
            "Knight"
        );
        assert_eq!(
            settings
                .hotkeys
                .active()
                .ok_or("active profile")?
                .action_bar,
            settings.action_bar
        );
        settings.validate()?;
        Ok(())
    }

    #[test]
    fn every_custom_action_path_creates_a_valid_typed_action()
    -> Result<(), Box<dyn std::error::Error>> {
        let mut profile = HotkeyProfile::default();
        let mut state = State {
            spell_key: "light healing".into(),
            spell_parameter: "friend".into(),
            object_definition: "3003".into(),
            object_use_mode: ObjectUseMode::Crosshair,
            new_text: "hello".into(),
            send_automatically: true,
            ..State::default()
        };
        add_new_action(&mut profile, &mut state, NewAction::Spell);
        state.object_definition = "3003".into();
        add_new_action(&mut profile, &mut state, NewAction::Object);
        state.new_text = "hello".into();
        add_new_action(&mut profile, &mut state, NewAction::Text);
        let preferences = oteryn_client::hotkeys::HotkeyPreferences {
            selected: 0,
            auto_switch: false,
            profiles: vec![profile],
        };
        preferences.validate(&[82, 79, 81, 80])?;
        assert!(matches!(
            &preferences.profiles[0].custom[0].action,
            CustomHotkeyAction::Spell { .. }
        ));
        assert!(matches!(
            &preferences.profiles[0].custom[1].action,
            CustomHotkeyAction::Object {
                use_mode: ObjectUseMode::Crosshair,
                ..
            }
        ));
        assert!(matches!(
            &preferences.profiles[0].custom[2].action,
            CustomHotkeyAction::Text {
                send_automatically: true,
                ..
            }
        ));
        Ok(())
    }

    #[test]
    fn binding_editor_keeps_chat_contexts_and_rejects_conflicts() -> Result<(), &'static str> {
        let mut settings = ClientSettings::default();
        let first = ShortcutBinding {
            key: 9,
            ctrl: true,
            alt: false,
            shift: false,
            meta: false,
        };
        let options_off = BindingTarget::General {
            action: "client.options",
            chat_on: false,
            second: false,
        };
        assert!(set_binding(&mut settings, &options_off, Some(first)));
        let profile = settings.hotkeys.active().ok_or("default profile")?;
        assert_eq!(
            profile.general_chat_off["client.options"].first,
            Some(first)
        );
        assert!(profile.general_chat_on.is_empty());

        let fullscreen_off = BindingTarget::General {
            action: "client.fullscreen",
            chat_on: false,
            second: false,
        };
        let fullscreen_on = BindingTarget::General {
            action: "client.fullscreen",
            chat_on: true,
            second: false,
        };
        assert!(!binding_candidate_valid(
            &settings,
            &fullscreen_off,
            Some(first)
        ));
        assert!(binding_candidate_valid(
            &settings,
            &fullscreen_on,
            Some(first)
        ));
        Ok(())
    }

    #[test]
    fn binding_editor_preserves_action_bar_modifiers_and_key_identity() {
        let mut settings = ClientSettings::default();
        let target = BindingTarget::ActionBar {
            row: 8,
            slot: 49,
            chat_on: false,
            second: true,
        };
        let binding = ShortcutBinding {
            key: 69,
            ctrl: true,
            alt: true,
            shift: true,
            meta: false,
        };
        assert!(set_binding(&mut settings, &target, Some(binding)));
        assert_eq!(
            settings.action_bar.extra_rows[7].secondary_shortcuts[49].map(slot_to_binding),
            Some(binding)
        );
        assert_eq!(egui_key_to_hid(egui::Key::F12), Some(69));
        assert_eq!(egui_key_to_hid(egui::Key::ArrowUp), Some(82));
        assert_eq!(egui_key_to_hid(egui::Key::Escape), None);
    }

    #[test]
    fn action_bar_binding_editor_keeps_chat_contexts_separate() -> Result<(), &'static str> {
        let mut settings = ClientSettings::default();
        let chat_on = BindingTarget::ActionBar {
            row: 8,
            slot: 49,
            chat_on: true,
            second: false,
        };
        let binding = ShortcutBinding {
            key: 4,
            ctrl: false,
            alt: false,
            shift: false,
            meta: false,
        };
        assert!(set_binding(&mut settings, &chat_on, Some(binding)));
        let row = settings.action_bar.row(8).ok_or("row 9")?;
        assert_eq!(
            row.chat_on_shortcuts[49].map(slot_to_binding),
            Some(binding)
        );
        assert_eq!(row.shortcuts[49], None);
        assert!(settings.validate_hotkey_draft().is_ok());
        Ok(())
    }
}
