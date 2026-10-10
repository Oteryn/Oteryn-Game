use egui::{Align2, RichText};
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
    pub fn set_action_bar_available(&mut self, available: bool) {
        self.browser.set_action_bar_available(available);
    }
    pub fn take_clear_action_row(&mut self) -> Option<usize> {
        self.browser.take_clear_action_row()
    }
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
                PreferenceAction::Ok => self.save_preferences(true),
                PreferenceAction::Apply => self.save_preferences(false),
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
                PreferenceAction::ExportOptions => self.export_options(),
                PreferenceAction::ImportOptions => self.import_options(),
                PreferenceAction::ResetOptions => {
                    self.draft = ClientSettings::default();
                    self.message = Some(if en {
                        "Defaults are ready. Select Apply or OK to save them."
                    } else {
                        "Ustawienia domyślne są gotowe. Wybierz Zastosuj lub OK, aby je zapisać."
                    }
                    .into());
                }
                PreferenceAction::OpenScreenshotFolder => self.open_screenshot_folder(),
            }
            return false;
        }
        let mut check_connection = false;
        let bounds = ctx.content_rect().shrink(4.0);
        let maximum = bounds.size().max(egui::Vec2::splat(1.0));
        let labels = [
            tr("Obraz", "Display"),
            tr("Interfejs", "Interface"),
            tr("Sterowanie", "Controls"),
            tr("Dźwięk", "Audio"),
            tr("Gra i czat", "Game and chat"),
            tr("Połączenie", "Connection"),
            tr("Prywatność", "Privacy"),
        ];
        egui::Window::new(tr("Ustawienia", "Settings"))
            .id(egui::Id::new("client-settings"))
            .anchor(Align2::CENTER_CENTER, [0.0, 0.0])
            .resizable(true).collapsible(false).constrain_to(bounds)
            .min_size(egui::Vec2::ZERO).max_size(maximum)
            .default_size([maximum.x.min(730.0), maximum.y.min(510.0)])
            .show(ctx, |ui| {
                use crate::preferences_browser::PreferenceAction;
                let (footer_action, body_rect) = crate::preferences_browser::primary_footer(ui, "quick-preferences-footer", en);
                match footer_action {
                    PreferenceAction::Ok => self.save_preferences(true),
                    PreferenceAction::Apply => self.save_preferences(false),
                    PreferenceAction::Cancel => { self.draft = self.current.clone(); self.open = false; self.message = None; }
                    PreferenceAction::None
                    | PreferenceAction::Defaults
                    | PreferenceAction::QuickSettings
                    | PreferenceAction::ExportOptions
                    | PreferenceAction::ImportOptions
                    | PreferenceAction::ResetOptions
                    | PreferenceAction::OpenScreenshotFolder => {}
                }
                let body_size = body_rect.size().max(egui::Vec2::splat(1.0));
                let mut body_ui = ui.new_child(egui::UiBuilder::new().max_rect(body_rect));
                body_ui.set_clip_rect(ui.clip_rect().intersect(body_rect));
                let body_scroll = egui::ScrollArea::both().id_salt(("quick-preferences-body", self.tab))
                    .auto_shrink([false, false]).max_width(body_size.x).max_height(body_size.y).show(&mut body_ui, |ui| {
                // A non-floating scrollbar reserves space before this content is laid out.
                let body_width = ui.available_width().max(1.0);
                ui.set_width(body_width);
                let compact = body_width < 520.0;
                ui.label(RichText::new(tr("OTERYN • USTAWIENIA KLIENTA", "OTERYN • CLIENT PREFERENCES")).strong());
                if ui.button(tr("Wszystkie ustawienia", "All preferences")).clicked() { self.browser.open = true; }
                ui.separator();
                if compact {
                    egui::ComboBox::from_id_salt("quick-preferences-category").width(body_width).truncate()
                        .selected_text(labels[self.tab]).show_ui(ui, |ui| {
                            for (index, label) in labels.iter().enumerate() { ui.selectable_value(&mut self.tab, index, *label); }
                        });
                }
                // The footer is outside this viewport. Leave the final Defaults row inside
                // it as well; screen height is unrelated to a resized dialog's body height.
                let column_height = (ui.available_height() - ui.spacing().interact_size.y
                    - 2.0 * ui.spacing().item_spacing.y - 6.0).max(1.0);
                ui.horizontal_top(|ui| {
                    if !compact { ui.set_max_height(column_height); }
                    if !compact {
                    ui.vertical(|ui| {
                        ui.set_width(140.0);
                        for (index, label) in labels.iter().enumerate() {
                            ui.selectable_value(&mut self.tab, index, *label);
                        }
                        ui.add_space(16.0);
                        ui.small(tr("F10 — ustawienia w grze", "F10 — settings in game"));
                    });
                    ui.separator();
                    }
                    egui::ScrollArea::vertical().id_salt(("quick-preferences-options", self.tab)).max_height(if compact { f32::INFINITY } else { column_height }).show(ui, |ui| {
                        ui.vertical(|ui| {
                        ui.set_width(ui.available_width().max(1.0));
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
                                ui.heading(tr("Paski akcji", "Action bars"));
                                crate::action_bar_ui::preferences_editor(ui, &mut self.draft.action_bar, en);
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
                });
                });
                #[cfg(test)]
                crate::preferences_browser::test_support::record_scroll_geometry(
                    ctx, "quick-preferences-body-geometry", &body_scroll,
                );
                #[cfg(not(test))]
                let _ = body_scroll;
            });
        check_connection
    }

    fn save_preferences(&mut self, close_after_save: bool) {
        let result = ClientSettings::path()
            .ok_or_else(|| std::io::Error::other("Preferences directory unavailable"))
            .and_then(|path| self.draft.save(&path));
        self.finish_save(result, close_after_save);
    }

    fn options_export_path() -> Option<std::path::PathBuf> {
        ClientSettings::path().map(|path| path.with_file_name("oteryn-options-export.json"))
    }

    fn screenshot_directory() -> Option<std::path::PathBuf> {
        ClientSettings::path().and_then(|path| Self::screenshot_directory_for(&path))
    }

    fn screenshot_directory_for(settings_path: &std::path::Path) -> Option<std::path::PathBuf> {
        settings_path
            .parent()
            .map(|parent| parent.join("screenshots"))
    }

    fn open_screenshot_folder(&mut self) {
        let result = Self::screenshot_directory()
            .ok_or_else(|| std::io::Error::other("Preferences directory unavailable"))
            .and_then(|path| {
                std::fs::create_dir_all(&path)?;
                open_directory(&path)?;
                Ok(path)
            });
        self.message = Some(match result {
            Ok(path) if self.draft.english => format!("Opened {}", path.display()),
            Ok(path) => format!("Otwarto {}", path.display()),
            Err(_) if self.draft.english => "The screenshot folder could not be opened.".into(),
            Err(_) => "Nie udało się otworzyć folderu zrzutów.".into(),
        });
    }

    fn export_options(&mut self) {
        let result = Self::options_export_path()
            .ok_or_else(|| std::io::Error::other("Preferences directory unavailable"))
            .and_then(|path| self.export_options_to(&path).map(|()| path));
        self.message = Some(match result {
            Ok(path) if self.draft.english => {
                format!("Options exported to {}", path.display())
            }
            Ok(path) => format!("Wyeksportowano ustawienia do {}", path.display()),
            Err(_) if self.draft.english => "Options were not exported.".into(),
            Err(_) => "Nie wyeksportowano ustawień.".into(),
        });
    }

    fn import_options(&mut self) {
        let result = Self::options_export_path()
            .ok_or_else(|| std::io::Error::other("Preferences directory unavailable"))
            .and_then(|path| self.import_options_from(&path));
        match result {
            Ok(()) => {
                self.message = Some(
                    if self.draft.english {
                        "Options imported. Select Apply or OK to activate them."
                    } else {
                        "Zaimportowano ustawienia. Wybierz Zastosuj lub OK, aby je aktywować."
                    }
                    .into(),
                );
            }
            Err(_) => {
                self.message = Some(if self.draft.english {
                    "Options were not imported. The export file is missing or invalid."
                } else {
                    "Nie zaimportowano ustawień. Plik eksportu nie istnieje lub jest nieprawidłowy."
                }
                .into());
            }
        }
    }

    fn export_options_to(&self, path: &std::path::Path) -> std::io::Result<()> {
        self.draft.save(path)
    }

    fn import_options_from(&mut self, path: &std::path::Path) -> std::io::Result<()> {
        self.draft = ClientSettings::load(path)?;
        Ok(())
    }

    fn finish_save(&mut self, result: std::io::Result<()>, close_after_save: bool) {
        match result {
            Ok(()) => { self.current = self.draft.clone(); self.applied = true; self.message = Some(if self.current.english { "Preferences saved." } else { "Zapisano ustawienia." }.into()); if close_after_save { self.open = false; self.message = None; } }
            Err(_) => self.message = Some(if self.current.english { "Not saved. Check distinct movement keys, action shortcuts and preferences directory access." } else { "Nie zapisano. Sprawdź klawisze kierunków, skróty akcji i dostęp do folderu ustawień." }.into()),
        }
    }
}

#[cfg(target_os = "windows")]
fn open_directory(path: &std::path::Path) -> std::io::Result<()> {
    let _child = std::process::Command::new("explorer.exe")
        .arg(path)
        .spawn()?;
    Ok(())
}

#[cfg(target_os = "macos")]
fn open_directory(path: &std::path::Path) -> std::io::Result<()> {
    let _child = std::process::Command::new("open").arg(path).spawn()?;
    Ok(())
}

#[cfg(all(unix, not(target_os = "macos")))]
fn open_directory(path: &std::path::Path) -> std::io::Result<()> {
    let _child = std::process::Command::new("xdg-open").arg(path).spawn()?;
    Ok(())
}

#[cfg(not(any(target_os = "windows", target_os = "macos", unix)))]
fn open_directory(_path: &std::path::Path) -> std::io::Result<()> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "Opening directories is unsupported on this platform",
    ))
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::preferences_browser::{
        PreferencesBrowser,
        test_support::{button_position, click, frame, scroll_geometry},
    };

    #[test]
    fn ok_closes_only_after_save_success_while_apply_remains_open() {
        for close_after_save in [false, true] {
            let current = ClientSettings::default();
            let mut panel = SettingsPanel {
                draft: ClientSettings {
                    fullscreen: true,
                    ..current.clone()
                },
                current: current.clone(),
                open: true,
                applied: false,
                tab: 0,
                message: None,
                browser: PreferencesBrowser::default(),
            };
            // Exercise completion of a failed/successful save without touching global preferences.
            panel.finish_save(Err(std::io::Error::other("write failed")), close_after_save);
            assert!(panel.open && !panel.applied && panel.message.is_some());
            assert_eq!(panel.current, current);
            assert!(panel.draft.fullscreen);
            panel.finish_save(Ok(()), close_after_save);
            assert_eq!(panel.open, !close_after_save);
            assert!(panel.applied && panel.current.fullscreen);
            assert_eq!(panel.current, panel.draft);
        }
    }

    #[test]
    fn quick_settings_columns_fit_the_current_body_without_outer_overflow()
    -> Result<(), &'static str> {
        for english in [false, true] {
            for size in [
                egui::vec2(1143.0, 814.0),
                egui::vec2(1143.0, 814.0) / 1.8,
                egui::vec2(1920.0, 1440.0),
            ] {
                let ctx = egui::Context::default();
                let current = ClientSettings {
                    english,
                    ..Default::default()
                };
                let mut panel = SettingsPanel {
                    draft: current.clone(),
                    current,
                    open: true,
                    applied: false,
                    tab: 0,
                    message: None,
                    browser: PreferencesBrowser::default(),
                };
                // In particular Controls is taller than the dialog. Its own options
                // scrollbar must absorb that height without moving the whole body.
                for tab in 0..7 {
                    panel.tab = tab;
                    for _ in 0..30 {
                        let _ = frame(&ctx, size, Vec::new(), |ctx| panel.show(ctx, None, None));
                    }
                    let (viewport, content) =
                        scroll_geometry(&ctx, "quick-preferences-body-geometry")
                            .ok_or("Missing quick-preferences scroll geometry")?;
                    assert!(
                        content.x <= viewport.width() + 1.0,
                        "Unnecessary horizontal overflow: tab={tab}, size={size:?}, {content:?} in {viewport:?}"
                    );
                    assert!(
                        content.y <= viewport.height() + 1.0,
                        "Column escaped current body height: tab={tab}, size={size:?}, {content:?} in {viewport:?}"
                    );
                }
            }
        }
        Ok(())
    }

    #[test]
    fn options_export_and_import_round_trip_a_validated_draft_without_applying_it()
    -> Result<(), Box<dyn std::error::Error>> {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "oteryn-options-round-trip-{}-{nonce}.json",
            std::process::id()
        ));
        let current = ClientSettings::default();
        let exported = ClientSettings {
            fullscreen: true,
            ui_scale: 1.4,
            ..current.clone()
        };
        let mut panel = SettingsPanel {
            draft: exported.clone(),
            current: current.clone(),
            open: true,
            applied: false,
            tab: 0,
            message: None,
            browser: PreferencesBrowser::default(),
        };
        panel.export_options_to(&path)?;
        panel.draft = current.clone();
        panel.import_options_from(&path)?;
        let _ = std::fs::remove_file(&path);
        assert_eq!(panel.draft, exported);
        assert_eq!(panel.current, current);
        assert!(!panel.applied);
        Ok(())
    }

    #[test]
    fn screenshot_directory_is_a_sibling_of_the_settings_file() {
        let path = std::path::Path::new("/configuration/oteryn/settings.json");
        assert_eq!(
            SettingsPanel::screenshot_directory_for(path),
            Some(std::path::PathBuf::from(
                "/configuration/oteryn/screenshots"
            ))
        );
    }

    #[test]
    fn quick_settings_cancel_recovers_draft_after_tiny_window_resize() -> Result<(), &'static str> {
        for english in [false, true] {
            let ctx = egui::Context::default();
            for size in [
                egui::vec2(900.0, 620.0),
                egui::vec2(300.0, 200.0) / 1.8,
                egui::vec2(800.0, 600.0) / 1.8,
            ] {
                let current = ClientSettings {
                    english,
                    ui_scale: 1.8,
                    ..Default::default()
                };
                let mut panel = SettingsPanel {
                    draft: ClientSettings {
                        fullscreen: true,
                        ..current.clone()
                    },
                    current,
                    open: true,
                    applied: false,
                    tab: 2,
                    message: Some("A long error must not hide Apply or Cancel. ".repeat(40)),
                    browser: PreferencesBrowser::default(),
                };
                let mut output = egui::FullOutput::default();
                for _ in 0..4 {
                    output = frame(&ctx, size, Vec::new(), |ctx| panel.show(ctx, None, None)).1;
                }
                let apply_label = if english { "Apply" } else { "Zastosuj" };
                assert!(
                    button_position(&output, size, apply_label).is_some(),
                    "Apply is clipped or outside viewport"
                );
                let cancel =
                    button_position(&output, size, if english { "Cancel" } else { "Anuluj" })
                        .ok_or("Cancel is clipped or outside viewport")?;
                let _ = click(&ctx, size, cancel, |ctx| panel.show(ctx, None, None));
                assert!(!panel.open);
                assert_eq!(panel.draft, panel.current);
            }
        }
        Ok(())
    }
}
