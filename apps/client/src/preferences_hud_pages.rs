//! HUD, confirmation and capture page compositions. Server state is never fabricated.
use super::{boxed, percent, reference_option, row};
use oteryn_client::settings::ClientSettings;

pub(super) fn show(ui: &mut egui::Ui, draft: &mut ClientSettings, section: &str, en: bool) -> bool {
    match section {
        "hud" => hud(ui, draft, en),
        "miscellaneous" => miscellaneous(ui, draft, en),
        "screenshots" => screenshots(ui, draft, en),
        _ => return false,
    }
    true
}

fn hud(ui: &mut egui::Ui, draft: &mut ClientSettings, en: bool) {
    ui.columns(2, |columns| {
        boxed(&mut columns[0], "", |ui| {
            for (id, labels) in [
                (
                    "hud.player_enabled",
                    ["HUD własnej postaci", "Show HUD for Own Character"],
                ),
                ("hud.resource_bars", ["Paski zasobów", "Show Bars"]),
                ("hud.owner_name", ["Nazwa", "Show Name"]),
                ("hud.owner_health", ["Zdrowie", "Show Health"]),
                ("hud.owner_mana", ["Mana", "Show Mana"]),
                ("show_harmony", ["Harmonia", "Show Harmony"]),
            ] {
                reference_option(ui, draft, "hud", id, labels, en);
            }
            let key = "hud.harmony_position";
            let stored = draft.future_preferences.get(key).cloned();
            for (id, labels) in [("health", ["Obok łuku zdrowia", "next to Health Arc"]), ("mana", ["Obok łuku many", "next to Mana Arc"])] {
                let selected = matches!(&stored, Some(oteryn_client::settings_catalog::FutureValue::Choice(value)) if value == id);
                if ui.radio(selected, labels[usize::from(en)]).clicked() {
                    draft.future_preferences.insert(key.into(), oteryn_client::settings_catalog::FutureValue::Choice(id.into()));
                }
            }
            reference_option(
                ui,
                draft,
                "hud",
                "hud.marks",
                ["Znaczniki", "Show Marks"],
                en,
            );
        });
        boxed(&mut columns[1], "", |ui| {
            for (id, labels) in [
                (
                    "hud.creatures_enabled",
                    ["HUD innych stworzeń", "Show HUD for Other Creatures"],
                ),
                ("hud.other_names", ["Nazwa", "Show Name"]),
                ("hud.other_health", ["Zdrowie", "Show Health"]),
                ("other_marks", ["Znaczniki", "Show Marks"]),
                ("hud.npc_icons", ["Ikony NPC", "Show NPC Icons"]),
            ] {
                reference_option(ui, draft, "hud", id, labels, en);
            }
        });
    });
    boxed(ui, "", |ui| {
        reference_option(
            ui,
            draft,
            "hud",
            "hud.arcs",
            ["Pokaż łuki", "Show Arcs"],
            en,
        );
        // Only the displayed Default Size member is verified; other members remain unknown.
        reference_option(
            ui,
            draft,
            "hud",
            "arc_size_preset",
            ["Rozmiar:", "Size:"],
            en,
        );
        percent(
            ui,
            draft,
            "hud",
            "hud.arc_distance",
            ["Odległość:", "Distance:"],
            en,
        );
        percent(
            ui,
            draft,
            "hud",
            "hud.arc_opacity",
            ["Widoczność:", "Opacity:"],
            en,
        );
    });
    egui::Grid::new("hud-special-conditions")
        .striped(true)
        .spacing([5.0, 2.0])
        .show(ui, |ui| {
            ui.label(if en {
                "Special Condition"
            } else {
                "Stan postaci"
            });
            ui.label("HUD");
            ui.label(if en { "Bar" } else { "Pasek" });
            ui.end_row();
            for (id, labels) in [
                ("poison", ["Zatrucie", "Poisoned"]),
                ("burning", ["Płonięcie", "Burning"]),
                ("electric", ["Porażenie", "Electrified"]),
                ("bleeding", ["Krwawienie", "Bleeding"]),
                ("agony", ["Agonia", "Agony"]),
                ("powerless", ["Bezsilność", "Powerless"]),
            ] {
                ui.label(labels[usize::from(en)]);
                for where_ in ["hud", "bar"] {
                    reference_option(
                        ui,
                        draft,
                        "hud",
                        &format!("condition_{id}_{where_}"),
                        ["", ""],
                        en,
                    );
                }
                ui.end_row();
            }
        });
    ui.weak(if en {
        "Condition order and remaining conditions await verified layout and projection."
    } else {
        "Kolejność i pozostałe stany wymagają weryfikacji układu i danych gry."
    });
    for (id, labels) in [
        (
            "custom_status_bars",
            [
                "Konfigurowalne paski stanu",
                "Show Customisable Status Bars",
            ],
        ),
        ("status_bars", ["Paski stanu", "Show Status Bars"]),
    ] {
        row(ui, draft, "hud", id, labels, en);
    }
}

fn miscellaneous(ui: &mut egui::Ui, draft: &mut ClientSettings, en: bool) {
    for (id, labels) in [
        (
            "buy_confirmation",
            [
                "Pytaj przed zakupem produktów",
                "Ask Before Buying Products",
            ],
        ),
        (
            "stow_confirmation",
            [
                "Pytaj przed schowaniem zawartości",
                "Ask Before Stowing Container Content",
            ],
        ),
        (
            "sort_confirmation",
            [
                "Pytaj przed sortowaniem zagnieżdżonych pojemników",
                "Ask Before Sorting Nested Containers",
            ],
        ),
        (
            "move_confirmation",
            [
                "Pytaj przed przenoszeniem zawartości pojemników",
                "Ask Before Moving Contents of Nested Containers",
            ],
        ),
        (
            "stay_logged_in",
            [
                "Pozostań zalogowany w tej sesji",
                "Stay Logged In for Session",
            ],
        ),
        (
            "optimise_connection",
            [
                "Optymalizuj stabilność połączenia",
                "Optimise Connection Stability",
            ],
        ),
        ("misc.quick_login", ["Szybkie logowanie", "Quick Login"]),
    ] {
        row(ui, draft, "miscellaneous", id, labels, en);
    }
    ui.weak(if en {
        "Session selections do not store credentials or enable remembered sign-in."
    } else {
        "Te wybory nie zapisują danych logowania ani nie włączają zapamiętanej sesji."
    });
}

fn screenshots(ui: &mut egui::Ui, draft: &mut ClientSettings, en: bool) {
    for (id, labels) in [
        (
            "capture.game_only",
            ["Tylko okno gry", "Only Capture Game Window"],
        ),
        (
            "capture.backlog",
            ["Bufor ostatnich 5 sekund", "Keep Backlog of Last 5 Seconds"],
        ),
        ("auto", ["Automatyczne zrzuty", "Enable Auto Screenshots"]),
    ] {
        row(ui, draft, "screenshots", id, labels, en);
    }
    boxed(
        ui,
        if en {
            "Capture Events"
        } else {
            "Zdarzenia do zapisu"
        },
        |ui| {
            ui.columns(2, |columns| {
                for (id, labels) in [
                    ("level", ["Awans poziomu", "Level Up"]),
                    ("skill", ["Awans umiejętności", "Skill Up"]),
                    ("capture.achievements", ["Osiągnięcie", "Achievement"]),
                    (
                        "bestiary_unlocked",
                        ["Odblokowany wpis bestiariusza", "Bestiary Entry Unlocked"],
                    ),
                    (
                        "bestiary_completed",
                        ["Ukończony wpis bestiariusza", "Bestiary Entry Completed"],
                    ),
                    ("capture.treasure", ["Znaleziony skarb", "Treasure Found"]),
                    ("capture.loot", ["Cenny łup", "Valuable Loot"]),
                    ("capture.boss", ["Pokonany boss", "Boss Defeated"]),
                ] {
                    reference_option(&mut columns[0], draft, "screenshots", id, labels, en);
                }
                for (id, labels) in [
                    ("death_pve", ["Śmierć PvE", "Death PvE"]),
                    ("death_pvp", ["Śmierć PvP", "Death PvP"]),
                    ("player_kill", ["Zabójstwo gracza", "Player Kill"]),
                    (
                        "player_assist",
                        ["Asysta przy zabójstwie", "Player Kill Assist"],
                    ),
                    ("player_attack", ["Atak gracza", "Player Attacking"]),
                    (
                        "capture.damage",
                        ["Najwyższe obrażenia", "Highest Damage Dealt"],
                    ),
                    (
                        "capture.healing",
                        ["Najwyższe leczenie", "Highest Healing Done"],
                    ),
                    ("capture.low_health", ["Niskie zdrowie", "Low Health"]),
                    ("gift_of_life", ["Dar życia", "Gift of Life Triggered"]),
                ] {
                    reference_option(&mut columns[1], draft, "screenshots", id, labels, en);
                }
            });
        },
    );
    ui.add_enabled(
        false,
        egui::Button::new(if en {
            "Open Screenshot Folder"
        } else {
            "Otwórz folder zrzutów"
        }),
    )
    .on_hover_text(if en {
        "Capture storage is not configured yet."
    } else {
        "Zapis zrzutów nie został jeszcze podłączony."
    });
}
