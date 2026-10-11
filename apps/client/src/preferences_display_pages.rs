//! Display/input arrangements observed in private captures, distinct from Oteryn extras.
use super::{boxed, reference_option, row};
use oteryn_client::{settings::ClientSettings, settings_catalog::FutureValue};

pub(super) fn show(ui: &mut egui::Ui, draft: &mut ClientSettings, section: &str, en: bool) -> bool {
    match section {
        "controls" => controls(ui, draft, en),
        "interface" => interface(ui, draft, en),
        "graphics" => graphics(ui, draft, en),
        "game_window" => game_window(ui, draft, en),
        "gameplay" => gameplay(ui, draft, en),
        _ => return false,
    }
    true
}

fn controls(ui: &mut egui::Ui, draft: &mut ClientSettings, en: bool) {
    ui.horizontal_wrapped(|ui| {
        reference_option(
            ui,
            draft,
            "controls",
            "controls.mouse_preset",
            ["Schemat myszy:", "Mouse Preset:"],
            en,
        );
        if draft.stored_choice("controls.controls.mouse_preset") == Some("classic_controls") {
            reference_option(
                ui,
                draft,
                "controls",
                "controls.loot_button",
                ["Łup:", "Loot:"],
                en,
            );
        }
    });
    boxed(ui, "", |ui| {
        reference_option(
            ui,
            draft,
            "controls",
            "controls.default_key_delay",
            [
                "Domyślne opóźnienie klawiatury",
                "Use Default Keyboard Delay",
            ],
            en,
        );
        // The capture shows 250ms, but does not establish supported bounds/defaults.
        let custom = matches!(
            draft
                .future_preferences
                .get("controls.controls.default_key_delay"),
            Some(FutureValue::Bool(false))
        );
        ui.add_enabled_ui(custom, |ui| {
            reference_option(
                ui,
                draft,
                "controls",
                "controls.key_delay_ms",
                ["Opóźnienie klawiatury (ms):", "Keyboard Delay (ms):"],
                en,
            );
        });
    });
    boxed(ui, "", |ui| {
        ui.label(if en {
            "To Rotate Your Character Hold:"
        } else {
            "Aby obrócić postać, przytrzymaj:"
        });
        ui.horizontal(|ui| {
            for (id, label) in [
                ("controls.turn_ctrl", "Ctrl"),
                ("controls.turn_shift", "Shift"),
                ("controls.turn_alt", "Alt"),
            ] {
                reference_option(ui, draft, "controls", id, [label, label], en);
            }
        });
        reference_option(
            ui,
            draft,
            "controls",
            "controls.face_movement",
            [
                "Zawsze obracaj w kierunku ruchu",
                "Always Turn Towards the Direction of Movement",
            ],
            en,
        );
    });
    row(
        ui,
        draft,
        "controls",
        "controls.complete_stack_ctrl",
        [
            "Ctrl do przeciągania całych stosów",
            "Press CTRL to Drag Complete Stacks",
        ],
        en,
    );
}

fn interface(ui: &mut egui::Ui, draft: &mut ClientSettings, en: bool) {
    for (section, id, labels) in [
        (
            "interface",
            "interface.highlight_mouse",
            ["Wyróżnij cel myszy", "Highlight Mouse Target"],
        ),
        (
            "interface",
            "interface.system_cursor",
            ["Używaj kursora systemowego", "Use Native Mouse Cursor"],
        ),
        (
            "interface",
            "interface.cursor_animation",
            ["Pokaż animowany kursor", "Show Animated Mouse Cursor"],
        ),
        (
            "interface",
            "interface.big_cursor",
            ["Pokaż duży kursor", "Show Big Mouse Cursor"],
        ),
        (
            "hud",
            "hud.cooldown_bar",
            ["Pokaż pasek odnowienia", "Show Cooldown Bar"],
        ),
        (
            "interface",
            "interface.link_copy_warning",
            [
                "Ostrzeżenie przy kopiowaniu linku",
                "Show Link Copy Warning",
            ],
        ),
    ] {
        row(ui, draft, section, id, labels, en);
    }
    boxed(ui, "", |ui| {
        reference_option(
            ui,
            draft,
            "interface",
            "interface.colourise_loot_value",
            ["Kolorowanie wartości łupów:", "Colourise Loot Value:"],
            en,
        );
        for (id, labels) in [
            (
                "interface.expiry_inventory",
                ["Wygaśnięcie w ekwipunku", "Show Expiry in Inventory"],
            ),
            (
                "interface.expiry_containers",
                ["Wygaśnięcie w pojemnikach", "Show Expiry in Containers"],
            ),
            (
                "interface.expiry_unused",
                [
                    "Wygaśnięcie nieużywanych przedmiotów",
                    "Show Expiry on Unused Items",
                ],
            ),
        ] {
            reference_option(ui, draft, "interface", id, labels, en);
        }
    });
}

fn graphics(ui: &mut egui::Ui, draft: &mut ClientSettings, en: bool) {
    reference_option(
        ui,
        draft,
        "graphics",
        "graphics.engine",
        ["Silnik graficzny:", "Graphics Engine:"],
        en,
    );
    reference_option(
        ui,
        draft,
        "graphics",
        "graphics.antialiasing",
        ["Wygładzanie obrazu:", "Antialiasing Mode:"],
        en,
    );
    boxed(ui, "", |ui| {
        ui.checkbox(
            &mut draft.fullscreen,
            if en {
                "Full Screen Mode"
            } else {
                "Pełny ekran"
            },
        );
        reference_option(
            ui,
            draft,
            "graphics",
            "graphics.integer_scale",
            [
                "Skalowanie całkowite",
                "Scale Using Only Integral Multiples",
            ],
            en,
        );
    });
    boxed(ui, "", |ui| {
        ui.checkbox(&mut draft.vsync, "V-Sync");
        let mut unlimited = !draft.frame_limit_enabled();
        if ui
            .checkbox(
                &mut unlimited,
                if en {
                    "No Frame Rate Limit"
                } else {
                    "Bez limitu FPS"
                },
            )
            .changed()
        {
            draft.set_frame_limit_enabled(!unlimited);
        }
        let mut limit = draft.last_frame_limit;
        ui.add_enabled_ui(!unlimited, |ui| {
            if ui
                .add(egui::Slider::new(&mut limit, 1..=200).text(if en {
                    "Frame Rate Limit"
                } else {
                    "Limit FPS"
                }))
                .changed()
            {
                let _ = draft.set_frame_limit(limit);
            }
        });
        let current_fps = ui.ctx().input(|input| {
            if input.stable_dt > 0.0 {
                1.0 / input.stable_dt
            } else {
                0.0
            }
        });
        ui.weak(format!(
            "{} {current_fps:.0}",
            if en {
                "Current Frame Rate:"
            } else {
                "Aktualna liczba FPS:"
            }
        ));
    });
}

fn game_window(ui: &mut egui::Ui, draft: &mut ClientSettings, en: bool) {
    for (id, labels) in [
        (
            "window.textual_effects",
            ["Efekty tekstowe", "Show Textual Effects"],
        ),
        ("window.general_messages", ["Wiadomości", "Show Messages"]),
        (
            "window.private_messages",
            ["Wiadomości prywatne", "Show Private Messages"],
        ),
        (
            "window.potion_sounds",
            ["Efekty dźwiękowe mikstur", "Show Potion Sound Effects"],
        ),
        ("window.own_spells", ["Zaklęcia", "Show Spells"]),
        (
            "window.other_spells",
            ["Zaklęcia innych", "Show Spells of Others"],
        ),
        (
            "window.hotkey_notices",
            [
                "Komunikaty użycia skrótów",
                "Show Hotkey Usage Notifications",
            ],
        ),
        (
            "window.loot_notices",
            ["Komunikaty łupów", "Show Loot Messages"],
        ),
        (
            "window.loot_highlight",
            ["Wyróżnianie łupów", "Show Loot Highlighting"],
        ),
        (
            "window.boosted_creatures",
            ["Wzmocnione stworzenie", "Show Boosted Creature"],
        ),
        (
            "window.training",
            ["Postęp treningu offline", "Show Offline Training Progress"],
        ),
        (
            "window.store_notices",
            [
                "Powiadomienia sklepu w walce",
                "Show Store Notifications in Combat",
            ],
        ),
        ("window.combat_frame", ["Ramki walki", "Show Combat Frames"]),
        ("window.pvp_frame", ["Ramki PvP", "Show PvP Frames"]),
        (
            "window.attack_animation",
            ["Animacja ataku wręcz", "Show Melee Attack Animation"],
        ),
        ("window.banner", ["Baner informacyjny", "Show Info Banner"]),
    ] {
        row(ui, draft, "game_window", id, labels, en);
    }
    boxed(
        ui,
        if en {
            "Target Marking"
        } else {
            "Oznaczanie celu"
        },
        |ui| {
            reference_option(
                ui,
                draft,
                "game_window",
                "window.target_marking",
                ["Oznaczaj cel wizualnie:", "Mark Target Visually:"],
                en,
            );
        },
    );
}

fn gameplay(ui: &mut egui::Ui, draft: &mut ClientSettings, en: bool) {
    for (id, labels) in [
        (
            "gameplay.inspect_permission",
            ["Pozwól wszystkim oglądać postać", "Allow All to Inspect Me"],
        ),
        (
            "gameplay.cancel_chase",
            ["Automatycznie przerywaj pościg", "Auto Chase Off"],
        ),
        (
            "gameplay.nearby_corpses",
            ["Łupienie pobliskich ciał", "Quick Loot Nearby Corpses"],
        ),
    ] {
        row(ui, draft, "gameplay", id, labels, en);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preferences_browser::test_support::{button_position, click, frame};

    #[test]
    fn frame_switch_restores_144_after_save_load_without_altering_committed_settings()
    -> Result<(), Box<dyn std::error::Error>> {
        for en in [false, true] {
            let ctx = egui::Context::default();
            crate::client_chrome::install(&ctx, false);
            let size = egui::vec2(650.0, 650.0);
            let committed = ClientSettings {
                fps: 144,
                last_frame_limit: 144,
                ..Default::default()
            };
            let mut draft = committed.clone();
            let label = if en {
                "No Frame Rate Limit"
            } else {
                "Bez limitu FPS"
            };
            {
                let mut draw = |ctx: &egui::Context| {
                    egui::Window::new("graphics-test")
                        .default_size([500.0, 550.0])
                        .show(ctx, |ui| graphics(ui, &mut draft, en));
                };
                for _ in 0..4 {
                    let _ = frame(&ctx, size, vec![], &mut draw);
                }
                let (_, out) = frame(&ctx, size, vec![], &mut draw);
                let button = button_position(&out, size, label).ok_or("frame switch visible")?;
                click(&ctx, size, button, &mut draw);
            }
            assert_eq!(draft.fps, 0);
            assert_eq!(draft.last_frame_limit, 144);
            assert_eq!(draft.redraw_fps(true), 0);
            assert_eq!(committed.fps, 144);

            let path = std::env::temp_dir().join(format!(
                "oteryn-frame-limit-ui-{}-{en}.json",
                std::process::id()
            ));
            draft.save(&path)?;
            draft = ClientSettings::load(&path)?;
            std::fs::remove_file(path)?;
            assert_eq!(draft.fps, 0);
            assert_eq!(draft.last_frame_limit, 144);

            {
                let mut draw = |ctx: &egui::Context| {
                    egui::Window::new("graphics-test-after-restart")
                        .default_size([500.0, 550.0])
                        .show(ctx, |ui| graphics(ui, &mut draft, en));
                };
                for _ in 0..2 {
                    let _ = frame(&ctx, size, vec![], &mut draw);
                }
                let (_, out) = frame(&ctx, size, vec![], &mut draw);
                let button = button_position(&out, size, label).ok_or("frame switch visible")?;
                click(&ctx, size, button, &mut draw);
            }
            assert_eq!(draft.fps, 144);
            assert_eq!(draft.redraw_fps(true), 144);
            assert_eq!(committed.fps, 144);

            // Cancel discards a later draft-only change exactly like the panel.
            draft.set_frame_limit(165)?;
            assert_eq!(draft.fps, 165);
            draft = committed.clone();
            assert_eq!(draft.fps, 144);
            assert_eq!(draft.last_frame_limit, 144);
        }
        Ok(())
    }

    #[test]
    fn gameplay_uses_the_exact_reference_inspection_label() {
        let ctx = egui::Context::default();
        crate::client_chrome::install(&ctx, false);
        let size = egui::vec2(620.0, 420.0);
        let mut draft = ClientSettings {
            english: true,
            ..Default::default()
        };
        let mut output = egui::FullOutput::default();
        for _ in 0..2 {
            output = frame(&ctx, size, vec![], |ctx| {
                egui::Window::new("gameplay-test")
                    .fixed_size([560.0, 360.0])
                    .show(ctx, |ui| gameplay(ui, &mut draft, true));
            })
            .1;
        }
        assert!(button_position(&output, size, "Allow All to Inspect Me").is_some());
        assert!(button_position(&output, size, "Allow Others to Inspect Your Character").is_none());
    }
}
