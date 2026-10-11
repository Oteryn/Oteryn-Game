//! Profile-based hotkey pages matching the reference table structure.

use oteryn_client::{
    action_bar::{ACTION_BAR_ROWS, ACTION_BAR_SLOTS, SlotShortcut},
    hotkeys::{
        CustomHotkey, CustomHotkeyAction, HotkeyPair, HotkeyProfile, MAX_HOTKEY_PROFILES,
        ObjectUseMode, general_action_supported, is_movement_action,
    },
    settings::ClientSettings,
    settings_catalog::ShortcutBinding,
};

/// The English labels are exact `label_gameactionidentifier_*` translations from the official
/// Tibia 15.33.b21348 `client.en.qm`. The Polish labels are Oteryn-authored translations: the
/// official package contains no Polish Tibia catalogue.
const GENERAL_ACTIONS: &[(&str, [&str; 2])] = &[
    (
        "action.bottom.1",
        [
            "Pasek akcji: pokaż/ukryj dolny pasek akcji 1",
            "Action Bar: Show/hide Bottom Action Bar 1",
        ],
    ),
    (
        "action.bottom.2",
        [
            "Pasek akcji: pokaż/ukryj dolny pasek akcji 2",
            "Action Bar: Show/hide Bottom Action Bar 2",
        ],
    ),
    (
        "action.bottom.3",
        [
            "Pasek akcji: pokaż/ukryj dolny pasek akcji 3",
            "Action Bar: Show/hide Bottom Action Bar 3",
        ],
    ),
    (
        "action.bottom.all",
        [
            "Pasek akcji: pokaż/ukryj dolne paski akcji",
            "Action Bar: Show/hide Bottom Action Bars",
        ],
    ),
    (
        "action.left.1",
        [
            "Pasek akcji: pokaż/ukryj lewy pasek akcji 1",
            "Action Bar: Show/hide Left Action Bar 1",
        ],
    ),
    (
        "action.left.2",
        [
            "Pasek akcji: pokaż/ukryj lewy pasek akcji 2",
            "Action Bar: Show/hide Left Action Bar 2",
        ],
    ),
    (
        "action.left.3",
        [
            "Pasek akcji: pokaż/ukryj lewy pasek akcji 3",
            "Action Bar: Show/hide Left Action Bar 3",
        ],
    ),
    (
        "action.left.all",
        [
            "Pasek akcji: pokaż/ukryj lewe paski akcji",
            "Action Bar: Show/hide Left Action Bars",
        ],
    ),
    (
        "action.right.1",
        [
            "Pasek akcji: pokaż/ukryj prawy pasek akcji 1",
            "Action Bar: Show/hide Right Action Bar 1",
        ],
    ),
    (
        "action.right.2",
        [
            "Pasek akcji: pokaż/ukryj prawy pasek akcji 2",
            "Action Bar: Show/hide Right Action Bar 2",
        ],
    ),
    (
        "action.right.3",
        [
            "Pasek akcji: pokaż/ukryj prawy pasek akcji 3",
            "Action Bar: Show/hide Right Action Bar 3",
        ],
    ),
    (
        "action.right.all",
        [
            "Pasek akcji: pokaż/ukryj prawe paski akcji",
            "Action Bar: Show/hide Right Action Bars",
        ],
    ),
    (
        "battle.first",
        [
            "Lista walki: zaatakuj pierwszy cel",
            "Battle List: Attack First Target",
        ],
    ),
    (
        "battle.next",
        [
            "Lista walki: zaatakuj następny cel",
            "Battle List: Attack Next Target",
        ],
    ),
    (
        "battle.previous",
        [
            "Lista walki: zaatakuj poprzedni cel",
            "Battle List: Attack Previous Target",
        ],
    ),
    (
        "chat.send",
        [
            "Czat: wyślij bieżący wiersz czatu",
            "Chat: Send current chat line",
        ],
    ),
    (
        "chat.server_messages",
        [
            "Czat: pokaż/ukryj komunikaty serwera w bieżącym kanale",
            "Chat: Show/hide Show Server messages in current channel",
        ],
    ),
    (
        "chat.close",
        [
            "Kanał czatu: zamknij bieżący kanał",
            "Chat Channel: Close Current Channel",
        ],
    ),
    (
        "chat.next",
        ["Kanał czatu: następny kanał", "Chat Channel: Next Channel"],
    ),
    (
        "chat.list",
        [
            "Kanał czatu: otwórz listę kanałów",
            "Chat Channel: Open Channel List",
        ],
    ),
    (
        "chat.help",
        [
            "Kanał czatu: otwórz kanał pomocy",
            "Chat Channel: Open Help Channel",
        ],
    ),
    (
        "chat.loot",
        [
            "Kanał czatu: otwórz kanał łupu",
            "Chat Channel: Open Loot Channel",
        ],
    ),
    (
        "chat.npc",
        [
            "Kanał czatu: otwórz kanał NPC",
            "Chat Channel: Open NPC Channel",
        ],
    ),
    (
        "chat.server",
        [
            "Kanał czatu: otwórz kanał serwera",
            "Chat Channel: Open Server Channel",
        ],
    ),
    (
        "chat.previous",
        [
            "Kanał czatu: poprzedni kanał",
            "Chat Channel: Previous Channel",
        ],
    ),
    (
        "chat.local",
        [
            "Kanał czatu: pokaż kanał domyślny",
            "Chat Channel: Show Default Channel",
        ],
    ),
    (
        "chat.mode.off",
        [
            "Tryb czatu: ustaw „Czat wyłączony”",
            "Chat Mode: Set to Chat Off",
        ],
    ),
    (
        "chat.mode.on",
        [
            "Tryb czatu: ustaw „Czat włączony”",
            "Chat Mode: Set to Chat On",
        ],
    ),
    (
        "chat.mode.temporary",
        [
            "Tryb czatu: ustaw „Czat włączony*”",
            "Chat Mode: Set to Chat On*",
        ],
    ),
    (
        "chat.text.copy",
        [
            "Tekst czatu: kopiuj do schowka",
            "Chat Text: Copy to clipboard",
        ],
    ),
    (
        "chat.text.select_all",
        ["Tekst czatu: zaznacz wszystko", "Chat Text: Select All"],
    ),
    (
        "combat.chase",
        ["Walka: przełącz tryb pościgu", "Combat: Toggle Chase Mode"],
    ),
    (
        "combat.secure",
        [
            "Walka: przełącz tryb bezpieczny",
            "Combat: Toggle Secure Mode",
        ],
    ),
    (
        "container.manual_sort",
        [
            "Kontenery: przełącz tryb ręcznego sortowania",
            "Containers: Toggle Manual Sort Mode",
        ],
    ),
    (
        "client.bug_report",
        ["Okna: otwórz zgłoszenie błędu", "Dialogs: Open Bugreport"],
    ),
    (
        "dialog.compendium",
        ["Okna: otwórz Kompendium", "Dialogs: Open Compendium"],
    ),
    (
        "dialog.cyclopedia.bestiary",
        [
            "Okna: otwórz Cyklopedię — Bestiariusz",
            "Dialogs: Open Cyclopedia - Bestiary",
        ],
    ),
    (
        "dialog.cyclopedia.character",
        [
            "Okna: otwórz Cyklopedię — Postać",
            "Dialogs: Open Cyclopedia - Character",
        ],
    ),
    (
        "dialog.cyclopedia.charms",
        [
            "Okna: otwórz Cyklopedię — Talizmany",
            "Dialogs: Open Cyclopedia - Charms",
        ],
    ),
    (
        "dialog.cyclopedia.items",
        [
            "Okna: otwórz Cyklopedię — Przedmioty",
            "Dialogs: Open Cyclopedia - Items",
        ],
    ),
    (
        "dialog.cyclopedia.magical_archive",
        [
            "Okna: otwórz Cyklopedię — Archiwum magiczne",
            "Dialogs: Open Cyclopedia - Magical Archive",
        ],
    ),
    (
        "dialog.cyclopedia.map",
        [
            "Okna: otwórz Cyklopedię — Mapę",
            "Dialogs: Open Cyclopedia - Map",
        ],
    ),
    (
        "dialog.exaltation_forge",
        [
            "Okna: otwórz Kuźnię egzaltacji",
            "Dialogs: Open Exaltation Forge",
        ],
    ),
    (
        "dialog.exiva_options",
        ["Okna: otwórz opcje Exivy", "Dialogs: Open Exiva Options"],
    ),
    (
        "client.ignore",
        [
            "Okna: otwórz listę ignorowanych",
            "Dialogs: Open Ignore List",
        ],
    ),
    (
        "dialog.manage_containers",
        [
            "Okna: otwórz zarządzanie kontenerami",
            "Dialogs: Open Manage Containers",
        ],
    ),
    (
        "client.options",
        ["Okna: otwórz ustawienia", "Dialogs: Open Options"],
    ),
    (
        "client.hotkeys",
        [
            "Okna: otwórz Ustawienia — Własne skróty",
            "Dialogs: Open Options - Custom Hotkeys",
        ],
    ),
    (
        "dialog.prey",
        ["Okna: otwórz Prey", "Dialogs: Open Prey Dialog"],
    ),
    (
        "dialog.quest_log",
        ["Okna: otwórz dziennik zadań", "Dialogs: Open Questlog"],
    ),
    (
        "dialog.reward_wall",
        ["Okna: otwórz Ścianę nagród", "Dialogs: Open Reward Wall"],
    ),
    (
        "dialog.social.assemble_team",
        [
            "Okna: otwórz Społeczność — Utwórz drużynę",
            "Dialogs: Open Social - Assemble Team",
        ],
    ),
    (
        "dialog.social.badges",
        [
            "Okna: otwórz Społeczność — Odznaki",
            "Dialogs: Open Social - Badges",
        ],
    ),
    (
        "dialog.social.friend_list",
        [
            "Okna: otwórz Społeczność — Lista znajomych",
            "Dialogs: Open Social - Friend List",
        ],
    ),
    (
        "dialog.social.friends_config",
        [
            "Okna: otwórz Społeczność — Ustawienia znajomych",
            "Dialogs: Open Social - Friends Config",
        ],
    ),
    (
        "dialog.social.friend_invitations",
        [
            "Okna: otwórz Społeczność — Zaproszenia znajomych",
            "Dialogs: Open Social - Friends Invitations",
        ],
    ),
    (
        "dialog.social.friend_search",
        [
            "Okna: otwórz Społeczność — Wyszukiwanie znajomych",
            "Dialogs: Open Social - Friends Search",
        ],
    ),
    (
        "dialog.social.join_team",
        [
            "Okna: otwórz Społeczność — Dołącz do drużyny",
            "Dialogs: Open Social - Join Team",
        ],
    ),
    (
        "dialog.task_board",
        ["Okna: otwórz Tablicę zleceń", "Dialogs: Open Task Board"],
    ),
    (
        "dialog.weapon_proficiency",
        [
            "Okna: otwórz Biegłość w broni",
            "Dialogs: Open Weapon Proficiency",
        ],
    ),
    (
        "dialog.wheel_of_destiny",
        [
            "Okna: otwórz Koło przeznaczenia",
            "Dialogs: Open Wheel of Destiny",
        ],
    ),
    (
        "loot.nearby_corpses",
        [
            "Łup: szybko zbierz łup z pobliskich ciał",
            "Loot: Quick Loot Nearby Corpses",
        ],
    ),
    ("minimap.center", ["Minimapa: wyśrodkuj", "Minimap: Center"]),
    (
        "minimap.floor_down",
        ["Minimapa: piętro niżej", "Minimap: One Floor Down"],
    ),
    (
        "minimap.floor_up",
        ["Minimapa: piętro wyżej", "Minimap: One Floor Up"],
    ),
    (
        "minimap.east",
        ["Minimapa: przewiń na wschód", "Minimap: Scroll East"],
    ),
    (
        "minimap.north",
        ["Minimapa: przewiń na północ", "Minimap: Scroll North"],
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
        "minimap.zoom_in",
        ["Minimapa: przybliż", "Minimap: Zoom In"],
    ),
    ("minimap.zoom_out", ["Minimapa: oddal", "Minimap: Zoom Out"]),
    (
        "dialog.lenshelp",
        ["Inne: aktywuj Lenshelp", "Misc.: Activate Lenshelp"],
    ),
    (
        "gameplay.exiva_allow_all",
        [
            "Inne: zezwól/zabroń wszystkim postaciom używać na mnie Exivy",
            "Misc.: Allow/disallow all characters to Exiva me",
        ],
    ),
    (
        "client.change_character",
        ["Inne: zmień postać", "Misc.: Change Character"],
    ),
    (
        "client.clear_message",
        [
            "Inne: usuń najstarszy komunikat z okna gry",
            "Misc.: Clear oldest message from Game Window",
        ],
    ),
    (
        "dialog.customise_character",
        ["Inne: dostosuj postać", "Misc.: Customise Character"],
    ),
    ("client.logout", ["Inne: wyloguj", "Misc.: Logout"]),
    (
        "client.next_profile",
        ["Inne: następny profil skrótów", "Misc.: Next Hotkey Preset"],
    ),
    (
        "client.previous_profile",
        [
            "Inne: poprzedni profil skrótów",
            "Misc.: Previous Hotkey Preset",
        ],
    ),
    (
        "client.screenshot",
        ["Inne: zrób zrzut ekranu", "Misc.: Take Screenshot"],
    ),
    (
        "movement.east",
        ["Ruch: idź na wschód", "Movement: Go East"],
    ),
    (
        "movement.north",
        ["Ruch: idź na północ", "Movement: Go North"],
    ),
    (
        "movement.north_east",
        ["Ruch: idź na północny wschód", "Movement: Go North-East"],
    ),
    (
        "movement.north_west",
        ["Ruch: idź na północny zachód", "Movement: Go North-West"],
    ),
    (
        "movement.south",
        ["Ruch: idź na południe", "Movement: Go South"],
    ),
    (
        "movement.south_east",
        ["Ruch: idź na południowy wschód", "Movement: Go South-East"],
    ),
    (
        "movement.south_west",
        ["Ruch: idź na południowy zachód", "Movement: Go South-West"],
    ),
    (
        "movement.west",
        ["Ruch: idź na zachód", "Movement: Go West"],
    ),
    (
        "client.mount",
        ["Ruch: wsiądź/zsiądź", "Movement: Mount/dismount"],
    ),
    (
        "client.stop",
        [
            "Ruch: zatrzymaj wszystkie akcje",
            "Movement: Stop All Actions",
        ],
    ),
    (
        "pvp.dove",
        ["Tryb PvP: ustaw gołębia", "PvP Mode: Set to Dove"],
    ),
    (
        "pvp.red_fist",
        [
            "Tryb PvP: ustaw czerwoną pięść",
            "PvP Mode: Set to Red Fist",
        ],
    ),
    (
        "pvp.white_hand",
        ["Tryb PvP: ustaw białą dłoń", "PvP Mode: Set to White Hand"],
    ),
    (
        "pvp.yellow_hand",
        ["Tryb PvP: ustaw żółtą dłoń", "PvP Mode: Set to Yellow Hand"],
    ),
    (
        "sound.toggle",
        ["Dźwięk: wycisz/włącz", "Sound: Mute/unmute"],
    ),
    (
        "client.names",
        [
            "Interfejs: pokaż/ukryj nazwy i paski stworzeń",
            "UI: Show/hide Creature Names and Bars",
        ],
    ),
    (
        "client.fps",
        [
            "Interfejs: pokaż/ukryj wskaźnik FPS/opóźnienia",
            "UI: Show/hide FPS / lag indicator",
        ],
    ),
    (
        "client.fullscreen",
        ["Interfejs: przełącz pełny ekran", "UI: Toggle Fullscreen"],
    ),
    (
        "window.battle.secondary",
        [
            "Okna: otwórz dodatkową listę walki",
            "Windows: Open secondary battle list",
        ],
    ),
    (
        "window.kill_tracker",
        [
            "Okna: pokaż/ukryj licznik zabójstw",
            "Windows: Show/hide Kill Tracker window",
        ],
    ),
    (
        "window.vip",
        ["Okna: pokaż/ukryj listę VIP", "Windows: Show/hide VIP list"],
    ),
    (
        "window.xp_analyser",
        [
            "Okna: pokaż/ukryj analizator doświadczenia",
            "Windows: Show/hide XP analyser",
        ],
    ),
    (
        "window.analytics_selector",
        [
            "Okna: pokaż/ukryj wybór analizatorów",
            "Windows: Show/hide analytics selector",
        ],
    ),
    (
        "window.battle",
        [
            "Okna: pokaż/ukryj listę walki",
            "Windows: Show/hide battle list",
        ],
    ),
    (
        "window.bestiary_tracker",
        [
            "Okna: pokaż/ukryj śledzenie bestiariusza",
            "Windows: Show/hide bestiary tracker",
        ],
    ),
    (
        "window.boss_cooldowns",
        [
            "Okna: pokaż/ukryj odnowienia bossów",
            "Windows: Show/hide boss cooldowns",
        ],
    ),
    (
        "window.bosstiary_tracker",
        [
            "Okna: pokaż/ukryj śledzenie bestiariusza bossów",
            "Windows: Show/hide bosstiary tracker",
        ],
    ),
    (
        "window.drop_tracker",
        [
            "Okna: pokaż/ukryj śledzenie zdobyczy",
            "Windows: Show/hide drop tracker",
        ],
    ),
    (
        "window.hunting_analyser",
        [
            "Okna: pokaż/ukryj analizator polowania",
            "Windows: Show/hide hunting analyser",
        ],
    ),
    (
        "window.imbuement_tracker",
        [
            "Okna: pokaż/ukryj śledzenie nasyceń",
            "Windows: Show/hide imbuement tracker",
        ],
    ),
    (
        "window.impact_analyser",
        [
            "Okna: pokaż/ukryj analizator wpływu",
            "Windows: Show/hide impact analyser",
        ],
    ),
    (
        "window.damage_input_analyser",
        [
            "Okna: pokaż/ukryj analizator otrzymanych obrażeń",
            "Windows: Show/hide input analyser",
        ],
    ),
    (
        "window.loot_analyser",
        [
            "Okna: pokaż/ukryj analizator łupu",
            "Windows: Show/hide loot analyser",
        ],
    ),
    (
        "window.party_hunt_analyser",
        [
            "Okna: pokaż/ukryj analizator polowania drużynowego",
            "Windows: Show/hide party hunt analyser",
        ],
    ),
    (
        "window.party",
        [
            "Okna: pokaż/ukryj listę drużyny",
            "Windows: Show/hide party list",
        ],
    ),
    (
        "window.player_guide",
        [
            "Okna: pokaż/ukryj poradnik gracza",
            "Windows: Show/hide player guide",
        ],
    ),
    (
        "window.quest_tracker",
        [
            "Okna: pokaż/ukryj śledzenie zadań",
            "Windows: Show/hide quest tracker",
        ],
    ),
    (
        "window.skills",
        [
            "Okna: pokaż/ukryj umiejętności",
            "Windows: Show/hide skills window",
        ],
    ),
    (
        "window.spell_list",
        [
            "Okna: pokaż/ukryj listę czarów",
            "Windows: Show/hide spell list",
        ],
    ),
    (
        "window.supply_analyser",
        [
            "Okna: pokaż/ukryj analizator zasobów",
            "Windows: Show/hide supply analyser",
        ],
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BindingCandidateStatus {
    Available,
    OverwritesExisting,
    Invalid,
}

const CONFIRM_LABEL: &str = "Ok";
const BINDING_CELL_WIDTH: f32 = 120.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum NewAction {
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
            ui.heading(profile_editor_title(edit.operation, en));
            ui.label(profile_editor_prompt(edit.operation, en));
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
                    .add_enabled(valid_name, egui::Button::new(CONFIRM_LABEL))
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
        let reserved_clear_width = ui.spacing().interact_size.x + ui.spacing().item_spacing.x;
        let response = ui.add(
            egui::TextEdit::singleline(&mut state.search)
                .desired_width((ui.available_width() - reserved_clear_width).max(0.0))
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
                    let label = action_bar_label(row, slot);
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
                    action_bar_binding(ui, primary[slot].map(slot_to_binding), en);
                    action_bar_binding(ui, secondary[slot].map(slot_to_binding), en);
                    ui.end_row();
                }
            }
        });
}

fn action_bar_binding(
    ui: &mut egui::Ui,
    value: Option<ShortcutBinding>,
    en: bool,
) -> egui::Response {
    binding_cell(ui, value, false).on_disabled_hover_text(if en {
        "Editing requires an authoritative live action-bar assignment source."
    } else {
        "Edycja wymaga wiarygodnego źródła przypisań paska akcji z bieżącej sesji."
    })
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
    new_action_menu(ui, en, state);
    if let Some(kind) = state.new_action {
        edit_new_action(ui, profile, en, state, kind);
    }
}

fn new_action_menu(ui: &mut egui::Ui, en: bool, state: &mut State) -> egui::Response {
    ui.menu_button(if en { "New Action" } else { "Nowa czynność" }, |ui| {
        for (kind, pl, english) in [
            (NewAction::Spell, "Przypisz czar", "Assign Spell"),
            (NewAction::Object, "Przypisz przedmiot", "Assign Object"),
            (NewAction::Text, "Przypisz tekst", "Assign Text"),
        ] {
            let available = new_action_available(kind);
            let response = ui
                .add_enabled(available, egui::Button::new(if en { english } else { pl }))
                .on_hover_text(new_action_availability_hint(kind, en));
            if response.clicked() {
                state.new_action = Some(kind);
                ui.close();
            }
        }
    })
    .response
}

fn new_action_available(kind: NewAction) -> bool {
    // The live client currently has neither an admitted spell catalogue nor an item-use
    // consumer. An inventory projection alone does not turn a persisted definition number
    // into an executable object hotkey, so those entry points must remain non-interactive.
    matches!(kind, NewAction::Text)
}

fn new_action_availability_hint(kind: NewAction, en: bool) -> &'static str {
    match (kind, en) {
        (NewAction::Spell, true) => {
            "Requires an authoritative live spell catalogue and spell-action consumer."
        }
        (NewAction::Spell, false) => {
            "Wymaga autorytatywnego katalogu czarów sesji i obsługi użycia czaru."
        }
        (NewAction::Object, true) => {
            "Requires an authoritative live inventory selection and item-use consumer."
        }
        (NewAction::Object, false) => {
            "Wymaga autorytatywnego wyboru z ekwipunku sesji i obsługi użycia przedmiotu."
        }
        (NewAction::Text, true) => "Text actions are routed to the real chat composer.",
        (NewAction::Text, false) => "Akcje tekstowe trafiają do rzeczywistego pola czatu.",
    }
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
                ui.label(new_text_label(en));
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
            if ui.button(if en { "Cancel" } else { "Anuluj" }).clicked() {
                state.new_action = None;
            }
        });
    });
}

fn new_text_label(en: bool) -> &'static str {
    if en { "Text:" } else { "Tekst:" }
}

fn new_action_valid(state: &State, kind: NewAction) -> bool {
    match kind {
        NewAction::Spell => !state.spell_key.trim().is_empty(),
        NewAction::Object => state
            .object_definition
            .parse::<u32>()
            .is_ok_and(|definition| definition != 0),
        NewAction::Text => !state.new_text.trim().is_empty(),
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
    _en: bool,
    editor: &mut Option<BindingEditor>,
) -> egui::Response {
    let response = binding_cell(ui, value, true);
    if response.clicked() {
        *editor = Some(BindingEditor {
            target,
            action: action.into(),
            captured: value,
        });
    }
    response
}

fn binding_editor(ui: &mut egui::Ui, draft: &mut ClientSettings, en: bool, state: &mut State) {
    let Some(mut editor) = state.binding_editor.take() else {
        return;
    };
    let mut close = false;
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
            if editor.target.is_stop_all() {
                editor.captured = Some(ShortcutBinding {
                    key: 41,
                    ctrl: modifiers.ctrl,
                    alt: modifiers.alt,
                    shift: modifiers.shift,
                    meta: modifiers.mac_cmd,
                });
            } else {
                close = true;
            }
        } else if let Some(key) = egui_key_to_hid(physical) {
            editor.captured = Some(ShortcutBinding {
                key,
                ctrl: modifiers.ctrl,
                alt: modifiers.alt,
                shift: modifiers.shift,
                meta: modifiers.mac_cmd,
            });
        }
    }
    let candidate_status = binding_candidate_status(draft, &editor.target, editor.captured);
    egui::Modal::new(egui::Id::new("hotkey-binding-editor")).show(ui.ctx(), |ui| {
        ui.heading(binding_editor_title(&editor.action, en));
        ui.weak(binding_editor_mode(editor.target.chat_on(), en));
        ui.add_space(6.0);
        ui.strong(binding_cell_label(editor.captured));
        ui.label(binding_editor_instruction(&editor.action, en));
        if editor.target.is_movement() {
            ui.label(if en {
                "This action must be bound to a single key."
            } else {
                "Ta czynność musi być przypisana do pojedynczego klawisza."
            });
        }
        match candidate_status {
            BindingCandidateStatus::OverwritesExisting => {
                ui.colored_label(
                    ui.visuals().error_fg_color,
                    if en {
                        "This hotkey is already in use and will be overwritten."
                    } else {
                        "Ten skrót jest już używany i zostanie nadpisany."
                    },
                );
            }
            BindingCandidateStatus::Invalid if editor.captured.is_some() => {
                ui.colored_label(
                    ui.visuals().error_fg_color,
                    if en {
                        "This hotkey is invalid or reserved."
                    } else {
                        "Ten skrót jest nieprawidłowy lub zarezerwowany."
                    },
                );
            }
            BindingCandidateStatus::Available | BindingCandidateStatus::Invalid => {}
        }
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    candidate_status != BindingCandidateStatus::Invalid,
                    egui::Button::new(CONFIRM_LABEL),
                )
                .clicked()
            {
                let _ = apply_binding_candidate(draft, &editor.target, editor.captured);
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

    fn is_stop_all(&self) -> bool {
        matches!(
            self,
            Self::General {
                action: "client.stop",
                ..
            }
        )
    }

    fn is_movement(&self) -> bool {
        matches!(
            self,
            Self::General {
                action,
                ..
            } if is_movement_action(action)
        )
    }
}

fn binding_candidate_status(
    draft: &ClientSettings,
    target: &BindingTarget,
    value: Option<ShortcutBinding>,
) -> BindingCandidateStatus {
    prepare_binding_candidate(draft, target, value).0
}

fn apply_binding_candidate(
    draft: &mut ClientSettings,
    target: &BindingTarget,
    value: Option<ShortcutBinding>,
) -> bool {
    let (_, candidate) = prepare_binding_candidate(draft, target, value);
    let Some(candidate) = candidate else {
        return false;
    };
    *draft = candidate;
    true
}

fn prepare_binding_candidate(
    draft: &ClientSettings,
    target: &BindingTarget,
    value: Option<ShortcutBinding>,
) -> (BindingCandidateStatus, Option<ClientSettings>) {
    let mut candidate = draft.clone();
    if set_binding(&mut candidate, target, value) && candidate.validate_hotkey_draft().is_ok() {
        return (BindingCandidateStatus::Available, Some(candidate));
    }

    let Some(value) = value else {
        return (BindingCandidateStatus::Invalid, None);
    };
    let mut candidate = draft.clone();
    if clear_binding_conflicts(&mut candidate, target.chat_on(), value)
        && set_binding(&mut candidate, target, Some(value))
        && candidate.validate_hotkey_draft().is_ok()
    {
        return (BindingCandidateStatus::OverwritesExisting, Some(candidate));
    }
    (BindingCandidateStatus::Invalid, None)
}

fn clear_binding_conflicts(
    draft: &mut ClientSettings,
    chat_on: bool,
    value: ShortcutBinding,
) -> bool {
    let mut cleared = false;
    {
        let Some(profile) = draft.hotkeys.active_mut() else {
            return false;
        };
        let map = if chat_on {
            &mut profile.general_chat_on
        } else {
            &mut profile.general_chat_off
        };
        let mut newly_empty = Vec::new();
        for (action, pair) in map.iter_mut() {
            if clear_pair_binding(pair, value) {
                cleared = true;
                if *pair == HotkeyPair::default() && !keep_empty_general_pair(action) {
                    newly_empty.push(action.clone());
                }
            }
        }
        for action in newly_empty {
            map.remove(&action);
        }

        if !chat_on && !has_binding_modifiers(value) {
            for (action, key) in [
                "movement.north",
                "movement.east",
                "movement.south",
                "movement.west",
            ]
            .into_iter()
            .zip(draft.movement_keys)
            {
                if key == value.key && !map.contains_key(action) {
                    map.insert(action.into(), HotkeyPair::default());
                    cleared = true;
                }
            }
        }

        for action in &mut profile.custom {
            let pair = if chat_on {
                &mut action.chat_on
            } else {
                &mut action.chat_off
            };
            cleared |= clear_pair_binding(pair, value);
        }
    }

    let slot = binding_to_slot(value);
    for row in 0..ACTION_BAR_ROWS {
        let Some(mut preferences) = draft.action_bar.row(row) else {
            continue;
        };
        let (primary, secondary) = if chat_on {
            (
                &mut preferences.chat_on_shortcuts,
                &mut preferences.chat_on_secondary_shortcuts,
            )
        } else {
            (
                &mut preferences.shortcuts,
                &mut preferences.secondary_shortcuts,
            )
        };
        let mut row_changed = false;
        for shortcut in primary.iter_mut().chain(secondary.iter_mut()) {
            if *shortcut == Some(slot) {
                *shortcut = None;
                row_changed = true;
            }
        }
        if row_changed {
            cleared = true;
            let _ = draft.action_bar.set_row(row, preferences);
        }
    }
    cleared
}

fn clear_pair_binding(pair: &mut HotkeyPair, value: ShortcutBinding) -> bool {
    let mut cleared = false;
    for binding in [&mut pair.first, &mut pair.second] {
        if *binding == Some(value) {
            *binding = None;
            cleared = true;
        }
    }
    cleared
}

fn has_binding_modifiers(value: ShortcutBinding) -> bool {
    value.ctrl || value.alt || value.shift || value.meta
}

fn keep_empty_general_pair(action: &str) -> bool {
    is_movement_action(action) || action == "chat.send"
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
            if pair == HotkeyPair::default() && !keep_empty_general_pair(action) {
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
        Key::Escape => 41,
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

fn unavailable_binding(
    ui: &mut egui::Ui,
    value: Option<ShortcutBinding>,
    en: bool,
) -> egui::Response {
    binding_cell(ui, value, false).on_disabled_hover_text(if en {
        "This action has no client consumer yet."
    } else {
        "Ta czynność nie ma jeszcze obsługi w kliencie."
    })
}

fn binding_cell(
    ui: &mut egui::Ui,
    value: Option<ShortcutBinding>,
    enabled: bool,
) -> egui::Response {
    let size = egui::vec2(BINDING_CELL_WIDTH, ui.spacing().interact_size.y);
    ui.add_enabled_ui(enabled, |ui| {
        ui.add_sized(size, egui::Button::new(binding_cell_label(value)))
    })
    .inner
}

fn binding_cell_label(value: Option<ShortcutBinding>) -> String {
    value.map_or_else(String::new, chord)
}

fn binding_editor_title(action: &str, en: bool) -> String {
    if en {
        format!("Edit Hotkey for \"{action}\"")
    } else {
        format!("Edytuj skrót dla „{action}”")
    }
}

fn binding_editor_mode(chat_on: bool, en: bool) -> &'static str {
    match (en, chat_on) {
        (true, true) => "Mode: \"Chat On\"",
        (true, false) => "Mode: \"Chat Off\"",
        (false, true) => "Tryb: „Czat włączony”",
        (false, false) => "Tryb: „Czat wyłączony”",
    }
}

fn binding_editor_instruction(action: &str, en: bool) -> String {
    if en {
        format!(
            "Click \"Ok\" to assign the hotkey. Click \"Clear\" to remove the hotkey from \"{action}\"."
        )
    } else {
        format!(
            "Kliknij „Ok”, aby przypisać skrót. Kliknij „Wyczyść”, aby usunąć skrót z „{action}”."
        )
    }
}

fn profile_editor_title(operation: ProfileOperation, en: bool) -> &'static str {
    match (en, operation) {
        (true, ProfileOperation::Add) => "Add hotkey preset",
        (true, ProfileOperation::Copy) => "Copy hotkey preset",
        (true, ProfileOperation::Rename) => "Rename hotkey preset",
        (false, ProfileOperation::Add) => "Dodaj profil skrótów",
        (false, ProfileOperation::Copy) => "Kopiuj profil skrótów",
        (false, ProfileOperation::Rename) => "Zmień nazwę profilu skrótów",
    }
}

fn profile_editor_prompt(operation: ProfileOperation, en: bool) -> &'static str {
    match (en, operation) {
        (true, ProfileOperation::Add | ProfileOperation::Copy) => {
            "Enter a name for the new preset:"
        }
        (true, ProfileOperation::Rename) => "Enter a new name for the preset:",
        (false, ProfileOperation::Add | ProfileOperation::Copy) => "Podaj nazwę nowego profilu:",
        (false, ProfileOperation::Rename) => "Podaj nową nazwę profilu:",
    }
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

fn action_bar_label(row: usize, slot: usize) -> String {
    format!(
        "{} Action Bar: Action Button {}.{:02}",
        edge(row),
        row + 1,
        slot + 1
    )
}

fn key_name(key: u16) -> String {
    match key {
        4..=29 => char::from_u32(u32::from(b'A') + u32::from(key - 4))
            .unwrap_or('?')
            .to_string(),
        30..=38 => (key - 29).to_string(),
        39 => "0".into(),
        40 => "Enter".into(),
        41 => "Esc".into(),
        42 => "Backspace".into(),
        43 => "Tab".into(),
        44 => "Space".into(),
        58..=69 => format!("F{}", key - 57),
        73 => "Insert".into(),
        74 => "Home".into(),
        75 => "Page Up".into(),
        76 => "Delete".into(),
        77 => "End".into(),
        78 => "Page Down".into(),
        79 => "→".into(),
        80 => "←".into(),
        81 => "↓".into(),
        82 => "↑".into(),
        88 => "Numpad Enter".into(),
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
    use std::collections::BTreeSet;

    #[test]
    fn general_hotkey_catalog_matches_the_official_120_action_inventory() {
        assert_eq!(GENERAL_ACTIONS.len(), 120);

        let ids = GENERAL_ACTIONS
            .iter()
            .map(|(id, _)| *id)
            .collect::<BTreeSet<_>>();
        let english_labels = GENERAL_ACTIONS
            .iter()
            .map(|(_, labels)| labels[1])
            .collect::<BTreeSet<_>>();
        assert_eq!(ids.len(), GENERAL_ACTIONS.len());
        assert_eq!(english_labels.len(), GENERAL_ACTIONS.len());
        assert!(
            GENERAL_ACTIONS
                .iter()
                .all(|(_, labels)| !labels[0].is_empty())
        );

        for (prefix, expected) in [
            ("Action Bar:", 12),
            ("Battle List:", 3),
            ("Chat:", 2),
            ("Chat Channel:", 9),
            ("Chat Mode:", 3),
            ("Chat Text:", 2),
            ("Combat:", 2),
            ("Containers:", 1),
            ("Dialogs:", 27),
            ("Loot:", 1),
            ("Minimap:", 9),
            ("Misc.:", 9),
            ("Movement:", 10),
            ("PvP Mode:", 4),
            ("Sound:", 1),
            ("UI:", 3),
            ("Windows:", 22),
        ] {
            assert_eq!(
                GENERAL_ACTIONS
                    .iter()
                    .filter(|(_, labels)| labels[1].starts_with(prefix))
                    .count(),
                expected,
                "{prefix}"
            );
        }

        let english = |id: &str| {
            GENERAL_ACTIONS
                .iter()
                .find(|(candidate, _)| *candidate == id)
                .map(|(_, labels)| labels[1])
        };
        assert_eq!(
            english("chat.server_messages"),
            Some("Chat: Show/hide Show Server messages in current channel")
        );
        assert_eq!(
            english("container.manual_sort"),
            Some("Containers: Toggle Manual Sort Mode")
        );
        assert_eq!(english("client.stop"), Some("Movement: Stop All Actions"));
        assert_eq!(
            english("window.kill_tracker"),
            Some("Windows: Show/hide Kill Tracker window")
        );
    }

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
    fn legacy_typed_custom_actions_remain_valid_and_round_trippable()
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
    fn text_action_rejects_whitespace_only_input() {
        let mut state = State {
            new_text: " \t  ".into(),
            ..State::default()
        };
        assert!(!new_action_valid(&state, NewAction::Text));
        state.new_text = " hello ".into();
        assert!(new_action_valid(&state, NewAction::Text));
    }

    #[test]
    fn new_action_is_an_anchored_menu_and_only_text_is_actionable() {
        fn frame(
            context: &egui::Context,
            state: &mut State,
            input: egui::RawInput,
        ) -> (egui::Rect, Vec<String>) {
            let mut button = egui::Rect::NOTHING;
            let mut output = context.run_ui(input, |ui| {
                button = new_action_menu(ui, true, state).rect;
            });
            let text = output
                .shapes
                .iter()
                .filter_map(|shape| match &shape.shape {
                    egui::epaint::Shape::Text(text) => Some(text.galley.job.text.clone()),
                    _ => None,
                })
                .collect();
            output.textures_delta.clear();
            (button, text)
        }

        let context = egui::Context::default();
        let mut state = State::default();
        let (button, _) = frame(&context, &mut state, egui::RawInput::default());
        assert!(button.is_positive());
        let pointer = |pressed| egui::RawInput {
            events: vec![
                egui::Event::PointerMoved(button.center()),
                egui::Event::PointerButton {
                    pos: button.center(),
                    button: egui::PointerButton::Primary,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ],
            ..Default::default()
        };
        frame(&context, &mut state, pointer(true));
        let (_, mut text) = frame(&context, &mut state, pointer(false));
        let (_, next_text) = frame(&context, &mut state, egui::RawInput::default());
        text.extend(next_text);

        assert!(egui::Popup::is_any_open(&context));
        for label in ["Assign Spell", "Assign Object", "Assign Text"] {
            assert!(text.iter().any(|visible| visible == label), "{label}");
        }
        assert!(!new_action_available(NewAction::Spell));
        assert!(!new_action_available(NewAction::Object));
        assert!(new_action_available(NewAction::Text));
        assert!(state.new_action.is_none());
    }

    #[test]
    fn assign_text_editor_uses_the_reference_field_label() {
        assert_eq!(new_text_label(true), "Text:");
        assert_eq!(new_text_label(false), "Tekst:");
        assert_eq!(
            new_action_availability_hint(NewAction::Object, true),
            "Requires an authoritative live inventory selection and item-use consumer."
        );
        assert_eq!(
            new_action_availability_hint(NewAction::Spell, true),
            "Requires an authoritative live spell catalogue and spell-action consumer."
        );
    }

    #[test]
    fn binding_editor_keeps_chat_contexts_and_overwrites_conflicts_atomically()
    -> Result<(), &'static str> {
        let mut settings = ClientSettings::default();
        let original_chat_on = settings
            .hotkeys
            .active()
            .ok_or("default profile")?
            .general_chat_on
            .clone();
        let first = ShortcutBinding {
            key: 30,
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
        settings
            .hotkeys
            .active_mut()
            .ok_or("default profile")?
            .general_chat_off
            .insert("explicit.clear".into(), HotkeyPair::default());
        let profile = settings.hotkeys.active().ok_or("default profile")?;
        assert_eq!(
            profile.general_chat_off["client.options"].first,
            Some(first)
        );
        assert_eq!(profile.general_chat_on, original_chat_on);

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
        assert_eq!(
            binding_candidate_status(&settings, &fullscreen_off, Some(first)),
            BindingCandidateStatus::OverwritesExisting
        );
        assert_eq!(
            binding_candidate_status(&settings, &fullscreen_on, Some(first)),
            BindingCandidateStatus::Available
        );
        assert!(apply_binding_candidate(
            &mut settings,
            &fullscreen_off,
            Some(first)
        ));
        let profile = settings.hotkeys.active().ok_or("default profile")?;
        assert!(!profile.general_chat_off.contains_key("client.options"));
        assert!(profile.general_chat_off.contains_key("explicit.clear"));
        assert_eq!(
            profile.general_chat_off["client.fullscreen"].first,
            Some(first)
        );
        assert_eq!(profile.general_chat_on, original_chat_on);
        assert!(settings.validate_hotkey_draft().is_ok());
        Ok(())
    }

    #[test]
    fn conflict_overwrite_spans_action_bar_and_custom_but_not_chat_contexts()
    -> Result<(), &'static str> {
        let mut settings = ClientSettings::default();
        let binding = ShortcutBinding {
            key: 69,
            ctrl: true,
            alt: true,
            shift: true,
            meta: false,
        };
        let action_bar_off = BindingTarget::ActionBar {
            row: 8,
            slot: 49,
            chat_on: false,
            second: true,
        };
        assert!(set_binding(&mut settings, &action_bar_off, Some(binding)));
        assert!(settings.validate_hotkey_draft().is_ok());

        let general_off = BindingTarget::General {
            action: "client.options",
            chat_on: false,
            second: false,
        };
        assert_eq!(
            binding_candidate_status(&settings, &general_off, Some(binding)),
            BindingCandidateStatus::OverwritesExisting
        );
        assert!(apply_binding_candidate(
            &mut settings,
            &general_off,
            Some(binding)
        ));
        assert_eq!(
            settings
                .action_bar
                .row(8)
                .ok_or("row 9")?
                .secondary_shortcuts[49],
            None
        );

        settings
            .hotkeys
            .active_mut()
            .ok_or("default profile")?
            .custom
            .push(CustomHotkey {
                action: CustomHotkeyAction::Text {
                    text: "hello".into(),
                    send_automatically: false,
                },
                chat_on: HotkeyPair {
                    first: Some(binding),
                    second: None,
                },
                chat_off: HotkeyPair::default(),
            });
        assert!(settings.validate_hotkey_draft().is_ok());

        let action_bar_on = BindingTarget::ActionBar {
            row: 8,
            slot: 49,
            chat_on: true,
            second: false,
        };
        assert_eq!(
            binding_candidate_status(&settings, &action_bar_on, Some(binding)),
            BindingCandidateStatus::OverwritesExisting
        );
        assert!(apply_binding_candidate(
            &mut settings,
            &action_bar_on,
            Some(binding)
        ));
        let profile = settings.hotkeys.active().ok_or("default profile")?;
        assert_eq!(profile.custom[0].chat_on.first, None);
        assert_eq!(
            profile.general_chat_off["client.options"].first,
            Some(binding)
        );
        assert_eq!(
            settings.action_bar.row(8).ok_or("row 9")?.chat_on_shortcuts[49].map(slot_to_binding),
            Some(binding)
        );
        assert!(settings.validate_hotkey_draft().is_ok());
        Ok(())
    }

    #[test]
    fn conflict_overwrite_is_scoped_to_the_active_profile() -> Result<(), &'static str> {
        let mut settings = ClientSettings::default();
        let binding = ShortcutBinding {
            key: 68,
            ctrl: true,
            alt: true,
            shift: false,
            meta: false,
        };
        let mut other = HotkeyProfile {
            name: "Other".into(),
            ..HotkeyProfile::default()
        };
        other.general_chat_off.insert(
            "client.options".into(),
            HotkeyPair {
                first: Some(binding),
                second: None,
            },
        );
        settings.hotkeys.profiles.push(other);
        assert!(settings.validate_hotkey_draft().is_ok());

        let active_target = BindingTarget::General {
            action: "client.previous_profile",
            chat_on: false,
            second: false,
        };
        assert_eq!(
            binding_candidate_status(&settings, &active_target, Some(binding)),
            BindingCandidateStatus::Available
        );
        assert!(apply_binding_candidate(
            &mut settings,
            &active_target,
            Some(binding)
        ));
        assert_eq!(
            settings.hotkeys.profiles[1].general_chat_off["client.options"].first,
            Some(binding)
        );
        assert!(settings.validate_hotkey_draft().is_ok());
        Ok(())
    }

    #[test]
    fn overwrite_materializes_a_clear_for_a_legacy_movement_fallback() -> Result<(), &'static str> {
        let mut settings = ClientSettings::default();
        settings
            .hotkeys
            .active_mut()
            .ok_or("default profile")?
            .general_chat_off
            .clear();
        assert!(settings.validate_hotkey_draft().is_ok());
        let north = ShortcutBinding {
            key: settings.movement_keys[0],
            ctrl: false,
            alt: false,
            shift: false,
            meta: false,
        };
        let target = BindingTarget::General {
            action: "client.options",
            chat_on: false,
            second: false,
        };
        assert_eq!(
            binding_candidate_status(&settings, &target, Some(north)),
            BindingCandidateStatus::OverwritesExisting
        );
        assert!(apply_binding_candidate(&mut settings, &target, Some(north)));
        let profile = settings.hotkeys.active().ok_or("default profile")?;
        assert_eq!(
            profile.general_chat_off["movement.north"],
            HotkeyPair::default()
        );
        assert_eq!(
            profile.general_chat_off["client.options"].first,
            Some(north)
        );
        assert!(settings.validate_hotkey_draft().is_ok());
        Ok(())
    }

    #[test]
    fn empty_table_bindings_are_visually_blank() {
        assert_eq!(binding_cell_label(None), "");
        assert_eq!(
            binding_cell_label(Some(ShortcutBinding {
                key: 4,
                ctrl: true,
                alt: false,
                shift: false,
                meta: false,
            })),
            "Ctrl+A"
        );
    }

    #[test]
    fn blank_and_unavailable_binding_cells_keep_a_full_stable_hit_target() {
        let context = egui::Context::default();
        let mut editor = None;
        let mut editable = egui::Rect::NOTHING;
        let mut unavailable = egui::Rect::NOTHING;
        let mut editable_enabled = false;
        let mut unavailable_enabled = true;
        let mut output = context.run_ui(egui::RawInput::default(), |ui| {
            let response = binding_button(
                ui,
                None,
                BindingTarget::General {
                    action: "client.options",
                    chat_on: false,
                    second: false,
                },
                "Options",
                true,
                &mut editor,
            );
            editable = response.rect;
            editable_enabled = response.enabled();
            let response = unavailable_binding(ui, None, true);
            unavailable = response.rect;
            unavailable_enabled = response.enabled();
        });
        output.textures_delta.clear();

        assert_eq!(editable.width(), BINDING_CELL_WIDTH);
        assert_eq!(unavailable.width(), BINDING_CELL_WIDTH);
        assert_eq!(editable.size(), unavailable.size());
        assert!(editable_enabled);
        assert!(!unavailable_enabled);
        assert!(editor.is_none());
    }

    #[test]
    fn action_bar_inventory_keeps_all_nine_by_fifty_saved_rows() {
        let labels = (0..ACTION_BAR_ROWS)
            .flat_map(|row| (0..ACTION_BAR_SLOTS).map(move |slot| action_bar_label(row, slot)))
            .collect::<BTreeSet<_>>();
        assert_eq!(labels.len(), ACTION_BAR_ROWS * ACTION_BAR_SLOTS);
        assert!(labels.contains("Bottom Action Bar: Action Button 1.01"));
        assert!(labels.contains("Right Action Bar: Action Button 9.50"));
    }

    #[test]
    fn action_bar_page_disables_both_binding_cells_without_changing_saved_values() {
        let context = egui::Context::default();
        let binding = ShortcutBinding {
            key: 69,
            ctrl: true,
            alt: false,
            shift: false,
            meta: false,
        };
        let mut first_enabled = true;
        let mut second_enabled = true;
        let mut rendered = Vec::new();
        let mut output = context.run_ui(egui::RawInput::default(), |ui| {
            let first = action_bar_binding(ui, Some(binding), true);
            let second = action_bar_binding(ui, None, true);
            first_enabled = first.enabled();
            second_enabled = second.enabled();
        });
        rendered.extend(output.shapes.iter().filter_map(|shape| match &shape.shape {
            egui::epaint::Shape::Text(text) => Some(text.galley.job.text.clone()),
            _ => None,
        }));
        output.textures_delta.clear();
        assert!(!first_enabled);
        assert!(!second_enabled);
        assert!(rendered.iter().any(|text| text == "Ctrl+F12"));

        let mut settings = ClientSettings::default();
        settings.action_bar.extra_rows[7].secondary_shortcuts[49] = Some(binding_to_slot(binding));
        let before = settings.clone();
        let mut state = State {
            search_applied: action_bar_label(8, 49),
            ..State::default()
        };
        let mut output = context.run_ui(egui::RawInput::default(), |ui| {
            action_bars(ui, &mut settings, true, &mut state);
        });
        output.textures_delta.clear();
        assert_eq!(settings, before);
        assert!(state.binding_editor.is_none());
    }

    #[test]
    fn binding_editor_uses_the_reference_english_copy() {
        let action = "Movement: Go East";
        assert_eq!(
            binding_editor_title(action, true),
            "Edit Hotkey for \"Movement: Go East\""
        );
        assert_eq!(binding_editor_mode(false, true), "Mode: \"Chat Off\"");
        assert_eq!(
            binding_editor_instruction(action, true),
            "Click \"Ok\" to assign the hotkey. Click \"Clear\" to remove the hotkey from \"Movement: Go East\"."
        );
        assert_eq!(CONFIRM_LABEL, "Ok");
    }

    #[test]
    fn profile_editors_use_the_reference_english_copy() {
        assert_eq!(
            profile_editor_title(ProfileOperation::Add, true),
            "Add hotkey preset"
        );
        assert_eq!(
            profile_editor_prompt(ProfileOperation::Add, true),
            "Enter a name for the new preset:"
        );
        assert_eq!(
            profile_editor_title(ProfileOperation::Copy, true),
            "Copy hotkey preset"
        );
        assert_eq!(
            profile_editor_prompt(ProfileOperation::Copy, true),
            "Enter a name for the new preset:"
        );
        assert_eq!(
            profile_editor_title(ProfileOperation::Rename, true),
            "Rename hotkey preset"
        );
        assert_eq!(
            profile_editor_prompt(ProfileOperation::Rename, true),
            "Enter a new name for the preset:"
        );
        assert_eq!(CONFIRM_LABEL, "Ok");
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
        assert_eq!(egui_key_to_hid(egui::Key::Escape), Some(41));
    }

    #[test]
    fn clearing_chat_send_keeps_an_explicit_empty_pair_distinct_from_legacy_absence() {
        let mut settings = ClientSettings::default();
        for second in [false, true] {
            assert!(set_binding(
                &mut settings,
                &BindingTarget::General {
                    action: "chat.send",
                    chat_on: true,
                    second,
                },
                None,
            ));
        }
        assert_eq!(
            settings
                .hotkeys
                .active()
                .and_then(|profile| profile.general_chat_on.get("chat.send"))
                .copied(),
            Some(HotkeyPair::default())
        );
        assert!(settings.validate_hotkey_draft().is_ok());
    }

    #[test]
    fn reference_default_navigation_and_numpad_keys_have_human_labels() {
        for (key, expected) in [
            (74, "Home"),
            (75, "Page Up"),
            (77, "End"),
            (78, "Page Down"),
            (88, "Numpad Enter"),
        ] {
            assert_eq!(key_name(key), expected);
        }
    }

    #[test]
    fn binding_editor_captures_stop_escape_cancels_other_escape_and_keeps_f10()
    -> Result<(), &'static str> {
        let run_key = |settings: &mut ClientSettings, state: &mut State, key: egui::Key| {
            let context = egui::Context::default();
            let mut output = context.run_ui(
                egui::RawInput {
                    events: vec![egui::Event::Key {
                        key,
                        physical_key: Some(key),
                        pressed: true,
                        repeat: false,
                        modifiers: egui::Modifiers::NONE,
                    }],
                    ..Default::default()
                },
                |ui| binding_editor(ui, settings, true, state),
            );
            output.textures_delta.clear();
        };
        let editor = |action: &'static str| BindingEditor {
            target: BindingTarget::General {
                action,
                chat_on: false,
                second: false,
            },
            action: action.into(),
            captured: None,
        };

        let mut settings = ClientSettings::default();
        let mut state = State {
            binding_editor: Some(editor("client.stop")),
            ..State::default()
        };
        run_key(&mut settings, &mut state, egui::Key::Escape);
        let stop = state.binding_editor.as_ref().ok_or("stop editor closed")?;
        assert_eq!(stop.captured.map(|binding| binding.key), Some(41));

        state.binding_editor = Some(editor("client.options"));
        run_key(&mut settings, &mut state, egui::Key::Escape);
        assert!(state.binding_editor.is_none());
        assert!(
            !settings
                .hotkeys
                .active()
                .ok_or("active profile")?
                .general_chat_off
                .contains_key("client.options")
        );

        state.binding_editor = Some(editor("client.options"));
        run_key(&mut settings, &mut state, egui::Key::F10);
        let f10 = state.binding_editor.as_ref().ok_or("F10 editor closed")?;
        assert_eq!(f10.captured.map(|binding| binding.key), Some(67));
        Ok(())
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

    #[test]
    fn movement_clear_is_retained_and_reserved_keys_follow_action_semantics()
    -> Result<(), &'static str> {
        let mut settings = ClientSettings::default();
        let movement = BindingTarget::General {
            action: "movement.north",
            chat_on: false,
            second: false,
        };
        assert!(set_binding(&mut settings, &movement, None));
        assert!(
            settings
                .hotkeys
                .active()
                .ok_or("default profile")?
                .general_chat_off
                .contains_key("movement.north")
        );
        let modified = ShortcutBinding {
            key: 26,
            ctrl: true,
            alt: false,
            shift: false,
            meta: false,
        };
        let unchanged = settings.clone();
        assert_eq!(
            binding_candidate_status(&settings, &movement, Some(modified)),
            BindingCandidateStatus::Invalid
        );
        assert!(!apply_binding_candidate(
            &mut settings,
            &movement,
            Some(modified)
        ));
        assert_eq!(settings, unchanged);

        let stop = BindingTarget::General {
            action: "client.stop",
            chat_on: false,
            second: true,
        };
        let options = BindingTarget::General {
            action: "client.options",
            chat_on: false,
            second: false,
        };
        let escape = ShortcutBinding {
            key: 41,
            ctrl: false,
            alt: false,
            shift: false,
            meta: false,
        };
        assert_eq!(
            binding_candidate_status(&settings, &stop, Some(escape)),
            BindingCandidateStatus::OverwritesExisting
        );
        assert!(apply_binding_candidate(&mut settings, &stop, Some(escape)));
        let stop_pair = settings
            .hotkeys
            .active()
            .ok_or("default profile")?
            .general_chat_off["client.stop"];
        assert_eq!(stop_pair.first, None);
        assert_eq!(stop_pair.second, Some(escape));

        let unchanged = settings.clone();
        assert_eq!(
            binding_candidate_status(&settings, &options, Some(escape)),
            BindingCandidateStatus::Invalid
        );
        assert!(!apply_binding_candidate(
            &mut settings,
            &options,
            Some(escape)
        ));
        assert_eq!(settings, unchanged);
        Ok(())
    }
}
