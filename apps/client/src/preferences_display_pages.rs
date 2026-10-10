//! Display/input arrangements observed in private captures, distinct from Oteryn extras.
use super::{PageState, boxed, reference_option, row};
use oteryn_client::{settings::ClientSettings, settings_catalog::FutureValue};

pub(super) fn show(
    ui: &mut egui::Ui,
    draft: &mut ClientSettings,
    section: &str,
    en: bool,
    state: &mut PageState,
) -> bool {
    match section {
        "controls" => controls(ui, draft, en),
        "interface" => interface(ui, draft, en),
        "graphics" => graphics(ui, draft, en, state),
        "game_window" => game_window(ui, draft, en),
        "gameplay" => gameplay(ui, draft, en),
        _ => return false,
    }
    true
}

fn controls(ui: &mut egui::Ui, draft: &mut ClientSettings, en: bool) {
    reference_option(
        ui,
        draft,
        "controls",
        "controls.mouse_preset",
        ["Schemat myszy:", "Mouse Preset:"],
        en,
    );
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
    boxed(ui, "OTERYN", |ui| {
        ui.checkbox(
            &mut draft.click_to_walk,
            if en {
                "Click to Walk"
            } else {
                "Chodzenie kliknięciem"
            },
        );
        reference_option(
            ui,
            draft,
            "controls",
            "controls.movement",
            ["Klawisze ruchu", "Movement Keys"],
            en,
        );
    });
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
    boxed(ui, "OTERYN", |ui| {
        ui.checkbox(
            &mut draft.english,
            if en {
                "English Interface"
            } else {
                "Interfejs po angielsku"
            },
        );
        ui.checkbox(
            &mut draft.high_contrast,
            if en {
                "High Contrast"
            } else {
                "Wysoki kontrast"
            },
        );
        ui.checkbox(
            &mut draft.reduced_motion,
            if en {
                "Reduced Motion"
            } else {
                "Ograniczone animacje"
            },
        );
        ui.add(
            egui::Slider::new(&mut draft.ui_scale, 0.8..=1.8).text(if en {
                "Interface Scale"
            } else {
                "Skala interfejsu"
            }),
        );
    });
}

fn graphics(ui: &mut egui::Ui, draft: &mut ClientSettings, en: bool, state: &mut PageState) {
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
        if draft.fps != 0 {
            state.last_frame_limit = Some(draft.fps);
        }
        let mut unlimited = draft.fps == 0;
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
            draft.fps = if unlimited {
                0
            } else {
                state.last_frame_limit.unwrap_or(60)
            };
        }
        let mut limit = state.last_frame_limit.unwrap_or(60);
        ui.add_enabled_ui(!unlimited, |ui| {
            if ui
                .add(egui::Slider::new(&mut limit, 30..=360).text(if en {
                    "Frame Rate Limit"
                } else {
                    "Limit FPS"
                }))
                .changed()
            {
                draft.fps = limit;
                state.last_frame_limit = Some(limit);
            }
        });
        // Renderer diagnostics are not passed into preferences; never display a made-up FPS.
        ui.weak(if en {
            "Current Frame Rate: —"
        } else {
            "Aktualna liczba FPS: —"
        });
    });
    boxed(ui, "OTERYN", |ui| {
        ui.horizontal(|ui| {
            ui.label(if en { "Window Size:" } else { "Rozmiar okna:" });
            ui.add(egui::DragValue::new(&mut draft.window_width).range(800..=3840));
            ui.label("×");
            ui.add(egui::DragValue::new(&mut draft.window_height).range(600..=2160));
        });
        ui.add(
            egui::Slider::new(&mut draft.background_fps, 5..=60).text(if en {
                "Background FPS"
            } else {
                "FPS w tle"
            }),
        );
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
            [
                "Pozwól innym oglądać postać",
                "Allow Others to Inspect Your Character",
            ],
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
    fn unlimited_frame_switch_restores_draft_limit_without_altering_committed_settings()
    -> Result<(), &'static str> {
        for en in [false, true] {
            let ctx = egui::Context::default();
            crate::client_chrome::install(&ctx, false);
            let size = egui::vec2(650.0, 650.0);
            let committed = ClientSettings {
                fps: 144,
                ..Default::default()
            };
            let mut draft = committed.clone();
            let mut state = PageState::default();
            let label = if en {
                "No Frame Rate Limit"
            } else {
                "Bez limitu FPS"
            };
            let mut draw = |ctx: &egui::Context| {
                egui::Window::new("graphics-test")
                    .default_size([500.0, 550.0])
                    .show(ctx, |ui| graphics(ui, &mut draft, en, &mut state));
            };
            for _ in 0..4 {
                let _ = frame(&ctx, size, vec![], &mut draw);
            }
            let (_, out) = frame(&ctx, size, vec![], &mut draw);
            let button = button_position(&out, size, label).ok_or("frame switch visible")?;
            click(&ctx, size, button, &mut draw);
            assert_eq!(draft.fps, 0);
            let mut draw = |ctx: &egui::Context| {
                egui::Window::new("graphics-test")
                    .default_size([500.0, 550.0])
                    .show(ctx, |ui| graphics(ui, &mut draft, en, &mut state));
            };
            let (_, out) = frame(&ctx, size, vec![], &mut draw);
            let button = button_position(&out, size, label).ok_or("frame switch visible")?;
            click(&ctx, size, button, &mut draw);
            assert_eq!(draft.fps, 144);
            assert_eq!(committed.fps, 144);
        }
        Ok(())
    }
}
