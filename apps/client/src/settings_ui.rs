use egui::{Align2, RichText, Vec2};
use oteryn_client::settings::ClientSettings;

pub struct SettingsPanel {
    pub current: ClientSettings,
    draft: ClientSettings,
    pub open: bool,
    pub applied: bool,
    tab: usize,
    message: Option<String>,
    browser: crate::preferences_browser::PreferencesBrowser,
}

impl SettingsPanel {
    pub fn replace_current(&mut self, settings: ClientSettings) {
        self.draft = settings.clone();
        self.current = settings;
    }
    pub fn new() -> Self {
        let loaded = ClientSettings::path().and_then(|path| {
            if path.exists() {
                Some(ClientSettings::load(&path))
            } else {
                None
            }
        });
        let (current, message) = match loaded {
            Some(Ok(settings)) => (settings, None),
            Some(Err(_)) => (ClientSettings::default(), Some("Nie można odczytać ustawień. Użyto domyślnych. / Cannot read preferences. Using defaults.".into())),
            None => (ClientSettings::default(), None),
        };
        Self {
            draft: current.clone(),
            current,
            open: false,
            applied: true,
            tab: 0,
            message,
            browser: crate::preferences_browser::PreferencesBrowser::new(),
        }
    }

    pub fn set_language(&mut self, english: bool) {
        self.current.english = english;
        self.draft.english = english;
        self.applied = true;
        if let Some(path) = ClientSettings::path()
            && self.current.save(&path).is_err()
        {
            self.message = Some("Nie zapisano języka. / Language was not saved.".into());
        }
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        portal: Option<&str>,
        gateway: Option<&str>,
    ) -> bool {
        if !self.open {
            return false;
        }
        let en = self.current.english;
        let tr = |pl, english| if en { english } else { pl };
        if self.browser.open {
            use crate::preferences_browser::PreferenceAction;
            match self
                .browser
                .show(ctx, &mut self.draft, self.message.as_deref())
            {
                PreferenceAction::None => {
                    if !self.browser.open {
                        self.open = false;
                        self.browser.open = true;
                        self.draft = self.current.clone();
                    }
                }
                PreferenceAction::Apply => self.save_preferences(),
                PreferenceAction::Cancel => {
                    self.draft = self.current.clone();
                    self.open = false;
                    self.message = None;
                }
                PreferenceAction::Defaults => {
                    self.draft = ClientSettings::default();
                    self.message = None;
                }
                PreferenceAction::QuickSettings => {
                    self.browser.open = false;
                    self.tab = 5;
                }
            }
            return false;
        }
        let mut check_connection = false;
        let available = ctx.content_rect().size();
        egui::Window::new(tr("Ustawienia", "Settings"))
            .id(egui::Id::new("client-settings"))
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .resizable(false).collapsible(false)
            .fixed_size(Vec2::new(available.x.min(730.0) - 35.0, available.y.min(510.0) - 60.0))
            .show(ctx, |ui| {
                ui.label(RichText::new(tr("OTERYN • USTAWIENIA KLIENTA", "OTERYN • CLIENT PREFERENCES")).strong());
                if ui.button(tr("Wszystkie ustawienia", "All preferences")).clicked() { self.browser.open = true; }
                ui.separator();
                ui.horizontal_top(|ui| {
                    ui.vertical(|ui| {
                        ui.set_width(140.0);
                        for (index, label) in [tr("Obraz", "Display"), tr("Interfejs", "Interface"),
                            tr("Sterowanie", "Controls"), tr("Dźwięk", "Audio"),
                            tr("Gra i czat", "Game and chat"), tr("Połączenie", "Connection"),
                            tr("Prywatność", "Privacy")].iter().enumerate() {
                            ui.selectable_value(&mut self.tab, index, *label);
                        }
                        ui.add_space(16.0);
                        ui.small(tr("F10 — ustawienia w grze", "F10 — settings in game"));
                    });
                    ui.separator();
                    egui::ScrollArea::vertical().max_height(290.0).show(ui, |ui| {
                        ui.vertical(|ui| {
                        ui.set_min_width(available.x.min(730.0) - 230.0);
                        match self.tab {
                            0 => {
                                ui.heading(tr("Obraz", "Display"));
                                ui.checkbox(&mut self.draft.fullscreen, tr("Pełny ekran bez ramek", "Borderless fullscreen"));
                                egui::ComboBox::from_label(tr("Rozmiar okna", "Window size"))
                                    .selected_text(format!("{} × {}", self.draft.window_width, self.draft.window_height))
                                    .show_ui(ui, |ui| {
                                        for (width, height) in [(900, 620), (1024, 768), (1280, 720), (1600, 900), (1920, 1080)] {
                                            if ui.selectable_label((self.draft.window_width, self.draft.window_height) == (width, height), format!("{width} × {height}")).clicked() {
                                                self.draft.window_width = width; self.draft.window_height = height;
                                            }
                                        }
                                    });
                                ui.checkbox(&mut self.draft.vsync, tr("Synchronizacja pionowa (VSync)", "Vertical synchronization (VSync)"));
                                egui::ComboBox::from_label(tr("Limit klatek", "Frame limit"))
                                    .selected_text(if self.draft.fps == 0 { tr("Bez limitu", "Unlimited").into() } else { format!("{} FPS", self.draft.fps) })
                                    .show_ui(ui, |ui| {
                                        for fps in [30, 60, 90, 120, 144, 165, 240, 360, 0] {
                                            let text = if fps == 0 { tr("Bez limitu", "Unlimited").into() } else { format!("{fps} FPS") };
                                            ui.selectable_value(&mut self.draft.fps, fps, text);
                                        }
                                    });
                                ui.add(egui::Slider::new(&mut self.draft.background_fps, 5..=60).text(tr("FPS w tle", "Background FPS")));
                                ui.small(tr("VSync może ograniczyć FPS do częstotliwości monitora.", "VSync may cap FPS to the monitor refresh rate."));
                            }
                            1 => {
                                ui.heading(tr("Interfejs i dostępność", "Interface and accessibility"));
                                egui::ComboBox::from_label(tr("Język", "Language"))
                                    .selected_text(if self.draft.english { "English" } else { "Polski" })
                                    .show_ui(ui, |ui| {
                                        ui.selectable_value(&mut self.draft.english, false, "Polski");
                                        ui.selectable_value(&mut self.draft.english, true, "English");
                                    });
                                ui.add(egui::Slider::new(&mut self.draft.ui_scale, 0.8..=1.8).text(tr("Skala interfejsu", "Interface scale")));
                                ui.checkbox(&mut self.draft.high_contrast, tr("Zwiększony kontrast paneli", "High contrast panels"));
                                ui.checkbox(&mut self.draft.reduced_motion, tr("Ograniczenie animacji oczekiwania", "Reduce loading animation"));
                                ui.small(tr("Skala dotyczy ekranów klienta; nie powiększa mapy gry.", "Scale affects client screens; it does not zoom the game map."));
                            }
                            2 => {
                                ui.heading(tr("Sterowanie ruchem", "Movement controls"));
                                ui.horizontal(|ui| {
                                    if ui.button(tr("Strzałki", "Arrow keys")).clicked() { self.draft.movement_keys = [82, 79, 81, 80]; }
                                    if ui.button("WASD").clicked() { self.draft.movement_keys = [26, 7, 22, 4]; }
                                });
                                for (index, label) in [tr("Północ", "North"), tr("Wschód", "East"), tr("Południe", "South"), tr("Zachód", "West")].iter().enumerate() {
                                    egui::ComboBox::from_id_salt(("movement", index))
                                        .selected_text(format!("{}: {}", label, key_label(self.draft.movement_keys[index])))
                                        .show_ui(ui, |ui| {
                                            for key in (4..=29).chain(79..=82) {
                                                ui.selectable_value(&mut self.draft.movement_keys[index], key, key_label(key));
                                            }
                                        });
                                }
                                ui.checkbox(&mut self.draft.click_to_walk, tr("Chodzenie lewym kliknięciem", "Walk with left mouse click"));
                                ui.small(tr("Każdy kierunek wymaga innego klawisza. F10 otwiera ustawienia.", "Each direction requires a different key. F10 opens settings."));
                                ui.separator();
                                ui.heading(tr("Pasek akcji", "Action bar"));
                                ui.checkbox(&mut self.draft.action_bar.visible, tr("Pokaż pasek", "Show bar"));
                                ui.checkbox(&mut self.draft.action_bar.locked, tr("Zablokuj przypisania", "Lock assignments"));
                                for slot in 0..oteryn_client::action_bar::ACTION_BAR_SLOTS {
                                    let shortcut = &mut self.draft.action_bar.shortcuts[slot];
                                    egui::ComboBox::from_id_salt(("action-shortcut", slot))
                                        .selected_text(format!("{}: {}", slot + 1, shortcut.map_or_else(|| tr("Brak", "None").into(), |s| action_key_label(s.key))))
                                        .show_ui(ui, |ui| {
                                            ui.selectable_value(shortcut, None, tr("Brak", "None"));
                                            for key in (4..=39).chain(58..=66).chain(68..=69) {
                                                ui.selectable_value(shortcut, Some(oteryn_client::action_bar::SlotShortcut { key, modifiers: 0 }), action_key_label(key));
                                            }
                                        });
                                }
                                ui.small(tr("Prawy przycisk na polu paska pozwala przypisać akcję. Przypisania przedmiotów i czarów dotyczą bieżącej sesji.", "Right-click an action slot to assign it. Item and spell assignments belong to the current session."));
                            }
                            3 => {
                                ui.heading(tr("Dźwięk", "Audio"));
                                ui.label(tr("Moduł odtwarzania audio nie jest jeszcze zaimplementowany.", "The audio playback module has not been implemented yet."));
                                ui.separator();
                                ui.label(tr("Zakres docelowy: głośność główna, muzyka, efekty, otoczenie, dźwięki UI, urządzenie wyjściowe i wyciszenie w tle.", "Target scope: master volume, music, effects, ambience, UI sounds, output device and mute in background."));
                            }
                            4 => {
                                ui.heading(tr("Gra, HUD i czat", "Game, HUD and chat"));
                                ui.checkbox(&mut self.draft.show_inventory, tr("Ekwipunek i otwarty kontener", "Inventory and open container"));
                                ui.checkbox(&mut self.draft.show_battle, tr("Lista widocznych postaci", "Visible actors list"));
                                ui.checkbox(&mut self.draft.show_minimap, tr("Minimapa", "Minimap"));
                                ui.checkbox(&mut self.draft.show_chat, tr("Konsola czatu", "Chat console"));
                                ui.separator();
                                ui.label(tr("Zakres docelowy: nazwy i paski zdrowia, obrażenia, siatka mapy, minimapa, układ paneli, rozmiar czatu, znaczniki czasu, filtry i powiadomienia.", "Target scope: names and health bars, damage, map grid, minimap, panel layout, chat size, timestamps, filters and notifications."));
                            }
                            5 => {
                                ui.heading(tr("Połączenie", "Connection"));
                                ui.label(tr("Portal konta", "Account portal"));
                                ui.monospace(portal.unwrap_or(tr("Nie skonfigurowano", "Not configured")));
                                ui.label(tr("Brama logowania", "Login gateway"));
                                ui.monospace(gateway.unwrap_or(tr("Nie skonfigurowano", "Not configured")));
                                ui.small(tr("Profil jest wczytywany przy uruchomieniu. Zmiana serwera wymaga ponownego uruchomienia; tokeny nie są przenoszone między serwerami.", "The profile is loaded at startup. Changing servers requires restarting; tokens are not transferred between servers."));
                                check_connection = ui.button(tr("Odśwież status świata", "Refresh world status")).clicked();
                            }
                            _ => {
                                ui.heading(tr("Prywatność i konto", "Privacy and account"));
                                ui.label(tr("Logowanie odbywa się w przeglądarce. Klient nie przechowuje hasła ani tokenów na dysku.", "Sign-in takes place in your browser. The client does not store passwords or tokens on disk."));
                                ui.label(tr("Zmiana konta i wylogowanie są dostępne na ekranie wyboru postaci.", "Switch account and sign out are available on the character screen."));
                                ui.label(tr("Ustawienia są przechowywane lokalnie:", "Preferences are stored locally:"));
                                if let Some(path) = ClientSettings::path() { ui.monospace(path.display().to_string()); }
                                ui.separator();
                                ui.small(format!("Oteryn {} • protocol-oteryn • DX12", env!("CARGO_PKG_VERSION")));
                            }
                        }
                        });
                    });
                });
                ui.separator();
                if let Some(message) = &self.message { ui.label(message); }
                ui.horizontal(|ui| {
                    if ui.button(tr("Domyślne", "Defaults")).clicked() { self.draft = ClientSettings::default(); self.message = None; }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button(tr("Anuluj", "Cancel")).clicked() {
                            self.draft = self.current.clone(); self.open = false; self.message = None;
                        }
                        if ui.button(tr("Zastosuj i zapisz", "Apply and save")).clicked() {
                            let result = ClientSettings::path().ok_or_else(|| std::io::Error::other("Preferences directory unavailable"))
                                .and_then(|path| self.draft.save(&path));
                            match result {
                                Ok(()) => { self.current = self.draft.clone(); self.applied = true; self.message = Some(tr("Zapisano ustawienia.", "Preferences saved.").into()); }
                                Err(_) => self.message = Some(tr("Nie zapisano. Sprawdź różne klawisze kierunków i dostęp do folderu ustawień.", "Not saved. Check distinct movement keys and preferences directory access.").into()),
                            }
                        }
                    });
                });
            });
        check_connection
    }

    fn save_preferences(&mut self) {
        let result = ClientSettings::path()
            .ok_or_else(|| std::io::Error::other("Preferences directory unavailable"))
            .and_then(|path| self.draft.save(&path));
        match result {
            Ok(()) => { self.current = self.draft.clone(); self.applied = true; self.message = Some(if self.current.english { "Preferences saved." } else { "Zapisano ustawienia." }.into()); }
            Err(_) => self.message = Some(if self.current.english { "Not saved. Check distinct movement keys, action shortcuts and preferences directory access." } else { "Nie zapisano. Sprawdź klawisze kierunków, skróty akcji i dostęp do folderu ustawień." }.into()),
        }
    }
}

fn key_label(code: u16) -> String {
    match code {
        79 => "ArrowRight".into(),
        80 => "ArrowLeft".into(),
        81 => "ArrowDown".into(),
        82 => "ArrowUp".into(),
        4..=29 => char::from_u32(u32::from(code) + 61)
            .unwrap_or('?')
            .to_string(),
        _ => "?".into(),
    }
}

fn action_key_label(code: u16) -> String {
    match code {
        30..=38 => (code - 29).to_string(),
        39 => "0".into(),
        58..=69 => format!("F{}", code - 57),
        _ => key_label(code),
    }
}
