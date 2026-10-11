//! Semantic contract for the compiled, production-routed settings pages.

use crate::{
    preferences_browser::{
        PreferenceAction, PreferencesBrowser,
        test_support::{
            button_position, button_position_in_rect, click, frame, reveal_in_rect, scroll_geometry,
        },
    },
    settings_audit::REFERENCE_SETTINGS_PAGES,
};
use egui::{FullOutput, accesskit::Role};
use oteryn_client::{
    settings::ClientSettings,
    settings_catalog::{FutureValue, REFERENCE_SETTINGS_SECTION_IDS},
};

const SIZE: egui::Vec2 = egui::vec2(900.0, 620.0);
type ChoiceState = (bool, String, Vec<String>);

struct PageContract {
    id: &'static str,
    labels: &'static [&'static str],
}

const fn page(id: &'static str, labels: &'static [&'static str]) -> PageContract {
    PageContract { id, labels }
}

#[rustfmt::skip]
const PAGE_CONTRACTS: [PageContract; 20] = [
    page("basic", &["Gameplay", "Mouse Preset:", "Allow All to Inspect Me", "Sound"]),
    page("controls", &["Mouse Preset:", "To Rotate Your Character Hold:", "Always turn towards the direction of movement", "Press CTRL to Drag Complete Stacks"]),
    page("general_hotkeys", &["Auto-Switch Hotkey Preset", "Chat Mode On", "Chat Mode Off", "Type to search for a hotkey:"]),
    page("action_hotkeys", &["Bottom Action Bar: Action Button 1.01", "Bottom Action Bar: Action Button 1.02", "First Key", "Second Key"]),
    page("custom_hotkeys", &["New Action", "Auto-Switch Hotkey Preset", "Chat Mode On", "Action"]),
    page("interface", &["Highlight Mouse Target", "Use Native Mouse Cursor", "Colourise Loot Value:", "Show Expiry in Inventory", "Show Expiry On Unused Items"]),
    page("hud", &["Show HUD for Own Character", "Show HUD for Other Creatures", "Show Arcs", "Distance: —", "Opacity: —", "Special Condition", "Show in HUD", "Show in Bar", "Show Customisable Status Bars", "Show Status Bars"]),
    page("console", &["Show Info Messages", "Open New Tabs When Receiving Private Messages", "Show Timestamps", "Show Levels"]),
    page("game_window", &["Show Textual Effects", "Show Hotkey Usage Notifications", "Show Loot Messages", "Mark Target Visually:"]),
    page("action_bars", &["Show Bottom Action Bars:", "Show Assigned Hotkey for Action Button", "Show Action Button Tooltip", "Clear Bottom Action Bars:"]),
    page("shortcuts", &["Displayed Shortcuts", "Available Shortcuts", "Skills", "Party List"]),
    page("graphics", &["Graphics Engine:", "Antialiasing Mode:", "No Frame Rate Limit", "Current Frame Rate: 60 fps"]),
    page("effects", &["Show Light Effects", "Ambient Light: —", "Opacity of Spell Effects", "Other Players' Effects: —"]),
    page("sound", &["Sound Device:", "Master Volume: —", "Anthem", "Food and Beverages", "Event Volume: —"]),
    page("battle_sounds", &["Own Battle Sounds: —", "Other Players: —", "Creatures: —", "Attacks and Spells"]),
    page("ui_sounds", &["UI Volume: —", "UI interactions", "VIP login/logout", "Console Messages", "System Announcements"]),
    page("miscellaneous", &["Ask Before Buying Products", "Ask Before Stowing Container Content", "Stay Logged In for Session", "Optimise Connection Stability"]),
    page("gameplay", &["Allow All to Inspect Me", "Auto Chase Off", "Quick Loot Nearby Corpses"]),
    page("screenshots", &["Only Capture Game Window", "Keep Backlog of the Screenshots of the Last 5 Seconds", "Enable Auto Screenshots", "Select all events that should trigger auto screenshots:", "Open Screenshot Folder"]),
    page("help", &["Client Help", "Compendium", "Rule Violations", "Manual", "FAQ", "Info", "Export All Options", "Export Minimap", "Import Options/Minimap", "Reset All Options"]),
];

struct ChoiceContract {
    page: &'static str,
    key: &'static str,
    enabled: bool,
    members: &'static [&'static str],
}

const fn choice(
    page: &'static str,
    key: &'static str,
    enabled: bool,
    members: &'static [&'static str],
) -> ChoiceContract {
    ChoiceContract {
        page,
        key,
        enabled,
        members,
    }
}

#[rustfmt::skip]
const CHOICE_CONTRACTS: &[ChoiceContract] = &[
    choice("basic", "controls.controls.mouse_preset", false, &["Classic Controls", "Regular Controls", "Left Smart-Click"]),
    choice("basic", "interface.interface.colourise_loot_value", false, &["None", "Frames", "Corners"]),
    choice("basic", "graphics.graphics.antialiasing", false, &["None", "Antialiasing", "Smooth Retro"]),
    choice("controls", "controls.controls.mouse_preset", false, &["Classic Controls", "Regular Controls", "Left Smart-Click"]),
    choice("interface", "interface.interface.colourise_loot_value", false, &["None", "Frames", "Corners"]),
    choice("hud", "hud.arc_size_preset", false, &["Default Size", "Small Size", "Large Size"]),
    choice("game_window", "game_window.window.target_marking", true, &["Frame & Highlight", "Frame Only", "Highlight Only", "None"]),
    choice("graphics", "graphics.graphics.engine", false, &[]),
    choice("graphics", "graphics.graphics.antialiasing", false, &["None", "Antialiasing", "Smooth Retro"]),
    choice("sound", "sound.sound.device", false, &[]),
];

#[test]
fn integral_scaling_control_changes_the_real_viewport() -> Result<(), &'static str> {
    let context = context();
    let mut browser = PreferencesBrowser::new();
    assert!(browser.open_section("graphics"));
    let mut draft = ClientSettings {
        english: true,
        ..Default::default()
    };
    let before = oteryn_client::layout::GameViewport::for_settings(1125.0, 775.0, 1.25, &draft)
        .ok_or("initial viewport")?;
    let output = settle(&context, &mut browser, &mut draft);
    let label = "Scale Using Only Integral Multiples";
    assert_eq!(checkbox_disabled(&output, label), Some(false));
    let position = button_position(&output, SIZE, label).ok_or("integral scaling checkbox")?;
    click(&context, SIZE, position, |context| {
        browser.show(context, &mut draft, None)
    });
    assert_eq!(
        draft.stored_bool("graphics.graphics.integer_scale"),
        Some(true)
    );
    let after = oteryn_client::layout::GameViewport::for_settings(1125.0, 775.0, 1.25, &draft)
        .ok_or("integral viewport")?;
    assert!(after.width < before.width && after.height < before.height);
    assert_eq!(after.width % 480.0, 0.0);
    assert_eq!(after.height % 352.0, 0.0);
    Ok(())
}

fn context() -> egui::Context {
    let context = egui::Context::default();
    context.set_theme(egui::Theme::Dark);
    context.enable_accesskit();
    crate::client_chrome::install(&context, false);
    context
}

fn settle(
    context: &egui::Context,
    browser: &mut PreferencesBrowser,
    draft: &mut ClientSettings,
) -> FullOutput {
    let mut output = FullOutput::default();
    for _ in 0..4 {
        output = frame(context, SIZE, Vec::new(), |context| {
            browser.show(context, draft, None)
        })
        .1;
    }
    output
}

#[test]
fn all_compiled_pages_match_the_english_semantic_contract_without_render_mutation()
-> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        REFERENCE_SETTINGS_PAGES.map(|page| page.id),
        REFERENCE_SETTINGS_SECTION_IDS
    );
    for (reference, contract) in REFERENCE_SETTINGS_PAGES.iter().zip(PAGE_CONTRACTS.iter()) {
        assert_eq!(reference.id, contract.id);
        assert!(
            contract.labels.contains(&reference.marker),
            "{} contract omits audit marker {:?}",
            contract.id,
            reference.marker
        );
        assert!(contract.labels.len() >= 3);

        let context = context();
        let mut browser = PreferencesBrowser::new();
        assert!(browser.open_section(contract.id));
        let mut draft = ClientSettings {
            english: true,
            ..Default::default()
        };
        let before = serde_json::to_value(&draft)?;
        let mut output = settle(&context, &mut browser, &mut draft);
        let (body, _) = scroll_geometry(&context, "all-preferences-body-geometry")
            .ok_or("missing production settings body")?;
        for label in contract.labels {
            if button_position_in_rect(&output, body, label).is_none() {
                output = reveal_in_rect(&context, SIZE, body, label, body.center(), |context| {
                    browser.show(context, &mut draft, None)
                });
            }
            assert!(
                button_position_in_rect(&output, body, label).is_some(),
                "{} does not expose exact English label {label:?}",
                contract.id
            );
        }
        for choice in CHOICE_CONTRACTS
            .iter()
            .filter(|choice| choice.page == contract.id)
        {
            let state = context
                .data(|data| {
                    data.get_temp::<ChoiceState>(egui::Id::new(("preference-choice", choice.key)))
                })
                .ok_or("missing routed choice contract")?;
            assert_eq!(
                state.0, choice.enabled,
                "wrong enabled state for {}",
                choice.key
            );
            assert_eq!(state.1, "—", "unknown choice must stay visibly unset");
            assert_eq!(
                state.2, choice.members,
                "wrong choice membership for {}",
                choice.key
            );
        }
        assert_eq!(
            serde_json::to_value(&draft)?,
            before,
            "render mutated {}",
            contract.id
        );
    }
    Ok(())
}

#[test]
fn reference_pages_exclude_oteryn_only_controls() -> Result<(), &'static str> {
    for (page, forbidden) in [
        ("controls", &["Click to Walk", "Movement Keys"][..]),
        (
            "interface",
            &[
                "English Interface",
                "High Contrast",
                "Reduced Motion",
                "Interface Scale",
            ][..],
        ),
        ("console", &["Show Chat Panel"][..]),
        ("graphics", &["Window Size:", "Background FPS"][..]),
        ("sound", &["Mute in Background"][..]),
        (
            "miscellaneous",
            &["Session selections do not store credentials or enable remembered sign-in."][..],
        ),
    ] {
        let context = context();
        let mut browser = PreferencesBrowser::new();
        assert!(browser.open_section(page));
        let mut draft = ClientSettings {
            english: true,
            ..Default::default()
        };
        let mut output = settle(&context, &mut browser, &mut draft);
        let (body, _) = scroll_geometry(&context, "all-preferences-body-geometry")
            .ok_or("missing production settings body")?;
        for label in forbidden {
            if button_position_in_rect(&output, body, label).is_none() {
                output = reveal_in_rect(&context, SIZE, body, label, body.center(), |context| {
                    browser.show(context, &mut draft, None)
                });
            }
            assert!(
                button_position_in_rect(&output, body, label).is_none(),
                "{page} leaked Oteryn-only control {label:?}"
            );
        }
    }
    Ok(())
}

#[test]
fn classic_mouse_preset_alone_reveals_the_exact_loot_button_selector() -> Result<(), &'static str> {
    let context = context();
    let mut browser = PreferencesBrowser::new();
    assert!(browser.open_section("controls"));
    let mut draft = ClientSettings {
        english: true,
        ..Default::default()
    };
    draft.future_preferences.insert(
        "controls.controls.mouse_preset".into(),
        FutureValue::Choice("classic_controls".into()),
    );
    let output = settle(&context, &mut browser, &mut draft);
    let (body, _) = scroll_geometry(&context, "all-preferences-body-geometry")
        .ok_or("missing production settings body")?;
    assert!(button_position_in_rect(&output, body, "Loot:").is_some());
    let state = context
        .data(|data| {
            data.get_temp::<ChoiceState>(egui::Id::new((
                "preference-choice",
                "controls.controls.loot_button",
            )))
        })
        .ok_or("missing classic loot selector")?;
    assert_eq!(
        state,
        (
            false,
            "—".into(),
            vec!["Right".into(), "SHIFT+Right".into(), "Left".into()]
        )
    );

    draft.future_preferences.insert(
        "controls.controls.mouse_preset".into(),
        FutureValue::Choice("regular_controls".into()),
    );
    let output = settle(&context, &mut browser, &mut draft);
    assert!(button_position_in_rect(&output, body, "Loot:").is_none());
    Ok(())
}

fn button_disabled(output: &FullOutput, label: &str) -> Option<bool> {
    output
        .platform_output
        .accesskit_update
        .as_ref()?
        .nodes
        .iter()
        .find_map(|(_, node)| {
            (node.role() == Role::Button && node.label() == Some(label)).then(|| node.is_disabled())
        })
}

fn checkbox_disabled(output: &FullOutput, label: &str) -> Option<bool> {
    output
        .platform_output
        .accesskit_update
        .as_ref()?
        .nodes
        .iter()
        .find_map(|(_, node)| {
            (node.role() == Role::CheckBox && node.label() == Some(label))
                .then(|| node.is_disabled())
        })
}

fn button_y(output: &FullOutput, label: &str) -> Option<f64> {
    output
        .platform_output
        .accesskit_update
        .as_ref()?
        .nodes
        .iter()
        .find_map(|(_, node)| {
            (node.role() == Role::Button && node.label() == Some(label))
                .then(|| node.bounds().map(|bounds| (bounds.y0 + bounds.y1) * 0.5))
                .flatten()
        })
}

#[test]
fn help_page_exposes_the_complete_enabled_and_disabled_action_groups() -> Result<(), &'static str> {
    let context = context();
    let mut browser = PreferencesBrowser::new();
    browser.set_compendium_available(true);
    assert!(browser.open_section("help"));
    let mut draft = ClientSettings {
        english: true,
        ..Default::default()
    };
    let output = settle(&context, &mut browser, &mut draft);
    for label in [
        "Client Help",
        "Rule Violations",
        "Manual",
        "FAQ",
        "Export Minimap",
        "Import Options/Minimap",
    ] {
        assert_eq!(button_disabled(&output, label), Some(true), "{label}");
    }
    for label in [
        "Compendium",
        "Info",
        "Export All Options",
        "Reset All Options",
    ] {
        assert_eq!(button_disabled(&output, label), Some(false), "{label}");
    }
    for stale in ["Import Options", "Import Minimap"] {
        assert_eq!(button_disabled(&output, stale), None, "stale {stale}");
    }
    let local_data_order = [
        "Export All Options",
        "Export Minimap",
        "Import Options/Minimap",
        "Reset All Options",
    ]
    .map(|label| button_y(&output, label).ok_or("missing local-data action"))
    .into_iter()
    .collect::<Result<Vec<_>, _>>()?;
    assert!(
        local_data_order.windows(2).all(|pair| pair[0] < pair[1]),
        "help local-data actions differ from the observed reference order"
    );

    let compendium =
        button_position(&output, SIZE, "Compendium").ok_or("missing Compendium action")?;
    let action = click(&context, SIZE, compendium, |context| {
        browser.show(context, &mut draft, None)
    });
    assert!(
        matches!(action, PreferenceAction::OpenCompendium),
        "Compendium must dispatch its explicit local-panel action"
    );

    let output = settle(&context, &mut browser, &mut draft);
    let import = button_position(&output, SIZE, "Import Options/Minimap")
        .ok_or("missing combined import action")?;
    let action = click(&context, SIZE, import, |context| {
        browser.show(context, &mut draft, None)
    });
    assert!(
        matches!(action, PreferenceAction::None),
        "the combined action must remain disabled until options and minimap import both work"
    );
    Ok(())
}

#[test]
fn assignment_dependent_action_bar_tooltips_stay_disabled_without_a_source()
-> Result<(), &'static str> {
    let context = context();
    let mut browser = PreferencesBrowser::new();
    assert!(browser.open_section("action_bars"));
    let mut draft = ClientSettings {
        english: true,
        ..Default::default()
    };
    let output = settle(&context, &mut browser, &mut draft);
    assert_eq!(
        checkbox_disabled(&output, "Show Action Button Tooltip"),
        Some(true)
    );
    assert!(
        !draft
            .future_preferences
            .contains_key("action_bars.bars.tooltips")
    );
    Ok(())
}

struct DependencyContract {
    page: &'static str,
    master: &'static str,
    gate: &'static str,
    children: &'static [&'static str],
}

const DEPENDENCIES: &[DependencyContract] = &[
    DependencyContract {
        page: "console",
        master: "console.console.timestamps",
        gate: "console-timestamp-seconds",
        children: &["console.console.seconds"],
    },
    DependencyContract {
        page: "battle_sounds",
        master: "battle_sounds.sound.own_spells",
        gate: "battle-own-spells",
        children: &[
            "battle_sounds.sound.attack_spells",
            "battle_sounds.sound.healing_spells",
            "battle_sounds.sound.support_spells",
        ],
    },
    DependencyContract {
        page: "battle_sounds",
        master: "battle_sounds.sound.other_spells",
        gate: "battle-other-spells",
        children: &[
            "battle_sounds.sound.other_attack_spells",
            "battle_sounds.sound.other_healing_spells",
            "battle_sounds.sound.other_support_spells",
        ],
    },
    DependencyContract {
        page: "ui_sounds",
        master: "ui_sounds.sound.chat_enabled",
        gate: "ui-console-messages",
        children: &[
            "ui_sounds.sound.party_chat",
            "ui_sounds.sound.guild",
            "ui_sounds.sound.private_without_tab",
            "ui_sounds.sound.private",
            "ui_sounds.sound.npc",
            "ui_sounds.sound.global",
            "ui_sounds.sound.team_finder",
            "ui_sounds.sound.raids",
            "ui_sounds.sound.system",
        ],
    },
    DependencyContract {
        page: "screenshots",
        master: "screenshots.auto",
        gate: "screenshot-automatic-events",
        children: &[
            "screenshots.level",
            "screenshots.skill",
            "screenshots.capture.achievements",
            "screenshots.bestiary_unlocked",
            "screenshots.bestiary_completed",
            "screenshots.capture.treasure",
            "screenshots.capture.loot",
            "screenshots.capture.boss",
            "screenshots.death_pve",
            "screenshots.death_pvp",
            "screenshots.player_kill",
            "screenshots.player_assist",
            "screenshots.player_attack",
            "screenshots.capture.damage",
            "screenshots.capture.healing",
            "screenshots.capture.low_health",
            "screenshots.gift_of_life",
        ],
    },
];

#[test]
fn compiled_dependency_groups_disable_without_erasing_and_restore_all_children() {
    for contract in DEPENDENCIES {
        let context = context();
        let mut browser = PreferencesBrowser::new();
        assert!(browser.open_section(contract.page));
        let mut draft = ClientSettings {
            english: true,
            ..Default::default()
        };
        // These spell/chat masters also depend on a positive group volume in
        // the installed-original QML; this fixture isolates the boolean gate.
        let volume = match contract.gate {
            "battle-own-spells" => Some("battle_sounds.sound.own_combat"),
            "battle-other-spells" => Some("battle_sounds.sound.other_combat"),
            "ui-console-messages" => Some("ui_sounds.sound.ui"),
            _ => None,
        };
        if let Some(volume) = volume {
            draft
                .future_preferences
                .insert(volume.into(), FutureValue::Int(100));
        }
        draft
            .future_preferences
            .insert(contract.master.into(), FutureValue::Bool(false));
        for child in contract.children {
            draft
                .future_preferences
                .insert((*child).into(), FutureValue::Bool(true));
        }
        let _ = settle(&context, &mut browser, &mut draft);
        assert_eq!(
            context
                .data(|data| data
                    .get_temp::<bool>(egui::Id::new(("retained-child-gate", contract.gate)))),
            Some(false),
            "{}",
            contract.gate
        );
        for child in contract.children {
            assert_eq!(
                draft.future_preferences.get(*child),
                Some(&FutureValue::Bool(true)),
                "disabled child {child} lost intent"
            );
        }
        draft
            .future_preferences
            .insert(contract.master.into(), FutureValue::Bool(true));
        let _ = settle(&context, &mut browser, &mut draft);
        assert_eq!(
            context
                .data(|data| data
                    .get_temp::<bool>(egui::Id::new(("retained-child-gate", contract.gate)))),
            Some(true),
            "{}",
            contract.gate
        );
        for child in contract.children {
            assert_eq!(
                draft.future_preferences.get(*child),
                Some(&FutureValue::Bool(true)),
                "reenabled child {child} lost intent"
            );
        }
    }
}
