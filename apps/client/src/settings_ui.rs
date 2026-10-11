use egui::{Align2, RichText};
use oteryn_client::{action_bar::ACTION_BAR_ROWS, settings::ClientSettings};

pub(crate) fn screenshot_directory() -> Option<std::path::PathBuf> {
    ClientSettings::path().and_then(|path| screenshot_directory_for(&path))
}

fn screenshot_directory_for(settings_path: &std::path::Path) -> Option<std::path::PathBuf> {
    settings_path
        .parent()
        .map(|parent| parent.join("screenshots"))
}

/// Missing intent keeps the historical full-client capture behavior; only an
/// explicit enabled value selects the game viewport crop.
pub(crate) fn game_only_capture(settings: &ClientSettings) -> bool {
    settings.stored_bool("screenshots.capture.game_only") == Some(true)
}

fn encode_screenshot(frame: &oteryn_renderer::CapturedFrame) -> std::io::Result<Vec<u8>> {
    let expected = frame
        .width
        .checked_mul(frame.height)
        .and_then(|pixels| pixels.checked_mul(4))
        .and_then(|bytes| usize::try_from(bytes).ok())
        .ok_or_else(|| {
            std::io::Error::new(std::io::ErrorKind::InvalidData, "invalid screenshot")
        })?;
    if frame.width == 0 || frame.height == 0 || frame.rgba.len() != expected {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "invalid screenshot",
        ));
    }
    let mut encoded = Vec::new();
    image::ImageEncoder::write_image(
        image::codecs::png::PngEncoder::new(&mut encoded),
        &frame.rgba,
        frame.width,
        frame.height,
        image::ExtendedColorType::Rgba8,
    )
    .map_err(std::io::Error::other)?;
    Ok(encoded)
}

pub(crate) fn save_screenshot(
    frame: &oteryn_renderer::CapturedFrame,
) -> std::io::Result<std::path::PathBuf> {
    let directory = screenshot_directory()
        .ok_or_else(|| std::io::Error::other("Preferences directory unavailable"))?;
    save_screenshot_to(&directory, frame)
}

fn save_screenshot_to(
    directory: &std::path::Path,
    frame: &oteryn_renderer::CapturedFrame,
) -> std::io::Result<std::path::PathBuf> {
    use std::io::Write;

    std::fs::create_dir_all(directory)?;
    let encoded = encode_screenshot(frame)?;
    for index in 0_u16..=u16::MAX {
        // PNG is reference-observed. This collision sequence stays Oteryn-local
        // until the manual-capture filename convention is independently observed.
        let name = if index == 0 {
            "oteryn-screenshot.png".to_owned()
        } else {
            format!("oteryn-screenshot-{index:04}.png")
        };
        let path = directory.join(name);
        let mut file = match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        };
        if let Err(error) = file.write_all(&encoded).and_then(|()| file.sync_all()) {
            drop(file);
            let _ = std::fs::remove_file(&path);
            return Err(error);
        }
        return Ok(path);
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "screenshot filename space exhausted",
    ))
}

pub(crate) const fn prelogin_settings_hotkey(
    consumed: bool,
    login_present: bool,
    pressed: bool,
    repeat: bool,
    f10: bool,
) -> bool {
    !consumed && login_present && pressed && !repeat && f10
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SettingsRuntimeAction {
    OpenCompendium,
}

pub struct SettingsPanel {
    pub current: ClientSettings,
    draft: ClientSettings,
    pub open: bool,
    pub applied: bool,
    tab: usize,
    message: Option<String>,
    browser: crate::preferences_browser::PreferencesBrowser,
    runtime_action: Option<SettingsRuntimeAction>,
}

impl SettingsPanel {
    /// Open the ordinary Options entry point without retaining a temporary
    /// direct-route navigation mode from an earlier hotkey.
    pub fn open_options(&mut self) {
        self.browser.clear_transient_route();
        self.browser.open = true;
        self.open = true;
    }

    /// Toggle the ordinary Options entry point. Hiding the window is not an
    /// implicit Cancel: the draft remains intact until the user explicitly
    /// applies or discards it.
    pub fn toggle_options(&mut self) {
        self.browser.clear_transient_route();
        self.browser.open = true;
        self.open = !self.open;
    }

    pub fn open_complete_section(&mut self, id: &str) -> bool {
        if !self.browser.open_section(id) {
            return false;
        }
        self.open = true;
        true
    }

    pub fn set_action_bar_rows_available(&mut self, available: [bool; ACTION_BAR_ROWS]) {
        self.browser.set_action_bar_rows_available(available);
    }
    pub fn set_compendium_available(&mut self, available: bool) {
        self.browser.set_compendium_available(available);
    }
    pub fn take_clear_action_row(&mut self) -> Option<usize> {
        self.browser.take_clear_action_row()
    }
    pub(crate) fn take_runtime_action(&mut self) -> Option<SettingsRuntimeAction> {
        self.runtime_action.take()
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
            runtime_action: None,
        }
    }

    /// Creates a settings panel without reading or writing the user preference path.
    pub fn for_settings_audit(current: ClientSettings) -> Self {
        let mut browser = crate::preferences_browser::PreferencesBrowser::new();
        browser.set_compendium_available(true);
        Self {
            draft: current.clone(),
            current,
            open: false,
            applied: false,
            tab: 0,
            message: None,
            browser,
            runtime_action: None,
        }
    }

    pub fn audit_draft(&self) -> &ClientSettings {
        &self.draft
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
                        self.browser.clear_transient_route();
                        self.draft = self.current.clone();
                    }
                }
                PreferenceAction::Ok => self.save_preferences(true),
                PreferenceAction::Apply => self.save_preferences(false),
                PreferenceAction::Cancel => {
                    self.draft = self.current.clone();
                    self.open = false;
                    self.message = None;
                    self.browser.clear_transient_route();
                }
                PreferenceAction::Defaults => {
                    self.draft = ClientSettings::default();
                    self.message = None;
                }
                PreferenceAction::OpenCompendium => {
                    self.open = false;
                    self.browser.clear_transient_route();
                    self.runtime_action = Some(SettingsRuntimeAction::OpenCompendium);
                }
                PreferenceAction::ExportOptions => self.export_options(),
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
                    PreferenceAction::Cancel => {
                        self.draft = self.current.clone();
                        self.open = false;
                        self.message = None;
                        self.browser.clear_transient_route();
                    }
                    PreferenceAction::None
                    | PreferenceAction::Defaults
                    | PreferenceAction::OpenCompendium
                    | PreferenceAction::ExportOptions
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
                if ui.button(tr("Wszystkie ustawienia", "All preferences")).clicked() {
                    self.browser.clear_transient_route();
                    self.browser.open = true;
                }
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
                        ui.small(tr(
                            "F10 — ustawienia przed logowaniem",
                            "F10 — pre-login settings",
                        ));
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
                                let mut selected_fps = self.draft.fps;
                                egui::ComboBox::from_label(tr("Limit klatek", "Frame limit"))
                                    .selected_text(if selected_fps == 0 { tr("Bez limitu", "Unlimited").into() } else { format!("{selected_fps} FPS") })
                                    .show_ui(ui, |ui| {
                                        for fps in [1, 30, 60, 90, 120, 144, 165, 200, 0] {
                                            let text = if fps == 0 { tr("Bez limitu", "Unlimited").into() } else { format!("{fps} FPS") };
                                            ui.selectable_value(&mut selected_fps, fps, text);
                                        }
                                    });
                                if selected_fps != self.draft.fps {
                                    if selected_fps == 0 {
                                        self.draft.set_frame_limit_enabled(false);
                                    } else {
                                        let _ = self.draft.set_frame_limit(selected_fps);
                                        self.draft.set_frame_limit_enabled(true);
                                    }
                                }
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
                                    if ui.button(tr("Strzałki", "Arrow keys")).clicked() {
                                        crate::preferences_browser::remember_movement_change(
                                            ui.ctx(),
                                            "quick-movement-conflict",
                                            self.draft.set_movement_keys([82, 79, 81, 80]),
                                        );
                                    }
                                    if ui.button("WASD").clicked() {
                                        crate::preferences_browser::remember_movement_change(
                                            ui.ctx(),
                                            "quick-movement-conflict",
                                            self.draft.set_movement_keys([26, 7, 22, 4]),
                                        );
                                    }
                                });
                                for (index, label) in [tr("Północ", "North"), tr("Wschód", "East"), tr("Południe", "South"), tr("Zachód", "West")].iter().enumerate() {
                                    let effective = self.draft.effective_movement_keys()[index];
                                    egui::ComboBox::from_id_salt(("movement", index))
                                        .selected_text(format!("{}: {}", label, effective.map_or_else(|| tr("Nieprzypisany", "Unbound").into(), key_label)))
                                        .show_ui(ui, |ui| {
                                            for key in (4..=29).chain(79..=82) {
                                                if ui.selectable_label(effective == Some(key), key_label(key)).clicked() {
                                                    crate::preferences_browser::remember_movement_change(
                                                        ui.ctx(),
                                                        "quick-movement-conflict",
                                                        self.draft.set_movement_key(index, key),
                                                    );
                                                }
                                            }
                                        });
                                }
                                crate::preferences_browser::movement_change_feedback(
                                    ui,
                                    "quick-movement-conflict",
                                    en,
                                );
                                ui.checkbox(&mut self.draft.click_to_walk, tr("Chodzenie lewym kliknięciem", "Walk with left mouse click"));
                                ui.small(tr("Każdy kierunek wymaga innego klawisza bez modyfikatorów.", "Each direction requires a distinct key without modifiers."));
                                ui.separator();
                                ui.heading(tr("Paski akcji", "Action bars"));
                                crate::action_bar_ui::preferences_editor(ui, &mut self.draft.action_bar, en);
                                ui.small(tr("Widoczność rzędów działa. Edycja skrótów i przypisań wymaga wiarygodnego źródła akcji z bieżącej sesji.", "Row visibility works. Editing shortcuts and assignments requires an authoritative live session action source."));
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

    fn open_screenshot_folder(&mut self) {
        let result = screenshot_directory()
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

    fn export_options_to(&self, path: &std::path::Path) -> std::io::Result<()> {
        self.draft.save(path)
    }

    fn import_options_from(&mut self, path: &std::path::Path) -> std::io::Result<()> {
        self.draft = ClientSettings::load(path)?;
        Ok(())
    }

    fn finish_save(&mut self, result: std::io::Result<()>, close_after_save: bool) {
        match result {
            Ok(()) => { self.current = self.draft.clone(); self.applied = true; self.message = Some(if self.current.english { "Preferences saved." } else { "Zapisano ustawienia." }.into()); if close_after_save { self.open = false; self.message = None; self.browser.clear_transient_route(); } }
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
    fn prelogin_f10_yields_to_the_binding_editor() {
        assert!(prelogin_settings_hotkey(false, true, true, false, true));
        assert!(!prelogin_settings_hotkey(true, true, true, false, true));
        assert!(!prelogin_settings_hotkey(false, false, true, false, true));
        assert!(!prelogin_settings_hotkey(false, true, false, false, true));
        assert!(!prelogin_settings_hotkey(false, true, true, true, true));
        assert!(!prelogin_settings_hotkey(false, true, true, false, false));
    }

    #[test]
    fn help_compendium_dispatches_through_settings_without_applying_the_draft()
    -> Result<(), &'static str> {
        let ctx = egui::Context::default();
        let size = egui::vec2(900.0, 620.0);
        let current = ClientSettings {
            english: true,
            ..Default::default()
        };
        let mut panel = SettingsPanel::for_settings_audit(current.clone());
        assert!(panel.open_complete_section("help"));

        let mut output = egui::FullOutput::default();
        for _ in 0..3 {
            output = frame(&ctx, size, Vec::new(), |ctx| panel.show(ctx, None, None)).1;
        }
        for label in ["Client Help", "Import Options/Minimap"] {
            let position =
                button_position(&output, size, label).ok_or("disabled Help action visible")?;
            let _ = click(&ctx, size, position, |ctx| panel.show(ctx, None, None));
            assert_eq!(panel.take_runtime_action(), None, "{label}");
        }

        panel.draft.fullscreen = !current.fullscreen;
        let unsaved = panel.draft.clone();
        panel.message = Some("unsaved draft".into());
        let compendium =
            button_position(&output, size, "Compendium").ok_or("Compendium action visible")?;
        let _ = click(&ctx, size, compendium, |ctx| panel.show(ctx, None, None));
        let action = panel
            .take_runtime_action()
            .ok_or("Compendium runtime action")?;
        assert_eq!(action, SettingsRuntimeAction::OpenCompendium);
        assert_eq!(panel.take_runtime_action(), None);
        assert_eq!(panel.draft, unsaved);
        assert_eq!(panel.current, current);
        assert!(!panel.open);
        assert_eq!(panel.message.as_deref(), Some("unsaved draft"));
        assert!(!panel.applied);

        let mut game = crate::game_ui::GameUi::default();
        assert!(game.route_settings_action(action));
        assert!(game.is_panel_open("compendium"));
        Ok(())
    }

    #[test]
    fn help_compendium_is_disabled_without_an_in_game_destination() -> Result<(), &'static str> {
        let ctx = egui::Context::default();
        let size = egui::vec2(900.0, 620.0);
        let current = ClientSettings {
            english: true,
            ..Default::default()
        };
        let mut panel = SettingsPanel {
            current: current.clone(),
            draft: current,
            open: false,
            applied: true,
            tab: 0,
            message: None,
            browser: PreferencesBrowser::new(),
            runtime_action: None,
        };
        assert!(panel.open_complete_section("help"));

        let mut output = egui::FullOutput::default();
        for _ in 0..3 {
            output = frame(&ctx, size, Vec::new(), |ctx| panel.show(ctx, None, None)).1;
        }
        let (_, _, enabled) = ctx
            .data(|data| {
                data.get_temp::<(egui::Rect, egui::Rect, bool)>(egui::Id::new((
                    "help-action-row",
                    "compendium",
                )))
            })
            .ok_or("Compendium row geometry")?;
        assert!(!enabled);
        let compendium =
            button_position(&output, size, "Compendium").ok_or("Compendium row visible")?;
        let _ = click(&ctx, size, compendium, |ctx| panel.show(ctx, None, None));
        assert!(panel.open);
        assert_eq!(panel.take_runtime_action(), None);
        Ok(())
    }

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
                runtime_action: None,
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
                    runtime_action: None,
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
            runtime_action: None,
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
    fn quick_movement_editor_reports_atomic_conflicts_and_persists_runtime_binding()
    -> Result<(), Box<dyn std::error::Error>> {
        for english in [false, true] {
            let ctx = egui::Context::default();
            ctx.set_theme(egui::Theme::Dark);
            crate::client_chrome::install(&ctx, false);
            let size = egui::vec2(900.0, 620.0);
            let current = ClientSettings {
                english,
                ..Default::default()
            };
            let mut panel = SettingsPanel {
                draft: current.clone(),
                current,
                open: true,
                applied: false,
                tab: 2,
                message: None,
                browser: PreferencesBrowser::default(),
                runtime_action: None,
            };

            // Start from a valid non-default single-key change so persistence must
            // preserve both the existing edit and the next UI edit.
            panel.draft.set_movement_key(1, 27)?;
            let before_conflict = panel.draft.clone();
            {
                let mut draw = |ctx: &egui::Context| panel.show(ctx, None, None);
                let mut output = egui::FullOutput::default();
                for _ in 0..4 {
                    output = frame(&ctx, size, vec![], &mut draw).1;
                }
                let north = button_position(
                    &output,
                    size,
                    if english { "North: W" } else { "Północ: W" },
                )
                .ok_or("north binding visible")?;
                click(&ctx, size, north, &mut draw);
                let popup = frame(&ctx, size, vec![], &mut draw).1;
                let conflicting =
                    button_position(&popup, size, "E").ok_or("conflicting key visible in popup")?;
                click(&ctx, size, conflicting, &mut draw);
                output = frame(&ctx, size, vec![], &mut draw).1;
                assert!(
                    button_position(
                        &output,
                        size,
                        if english {
                            "Shortcut conflict. Movement keys were not changed."
                        } else {
                            "Konflikt skrótów. Klawisze ruchu nie zostały zmienione."
                        }
                    )
                    .is_some(),
                    "quick movement conflict feedback is not visible"
                );
            }
            assert_eq!(panel.draft, before_conflict);

            {
                let mut draw = |ctx: &egui::Context| panel.show(ctx, None, None);
                let output = frame(&ctx, size, vec![], &mut draw).1;
                let north = button_position(
                    &output,
                    size,
                    if english { "North: W" } else { "Północ: W" },
                )
                .ok_or("north binding still visible")?;
                click(&ctx, size, north, &mut draw);
                let popup = frame(&ctx, size, vec![], &mut draw).1;
                let replacement =
                    button_position(&popup, size, "F").ok_or("replacement key visible in popup")?;
                click(&ctx, size, replacement, &mut draw);
                let output = frame(&ctx, size, vec![], &mut draw).1;
                assert!(
                    button_position(
                        &output,
                        size,
                        if english {
                            "Shortcut conflict. Movement keys were not changed."
                        } else {
                            "Konflikt skrótów. Klawisze ruchu nie zostały zmienione."
                        }
                    )
                    .is_none(),
                    "successful movement change did not clear conflict feedback"
                );
            }
            assert_eq!(panel.draft.effective_movement_keys()[0], Some(9));

            let north = oteryn_input_actions::NormalizedInputEvent::Key {
                code: oteryn_input_actions::KeyCode::new(9)?,
                state: oteryn_input_actions::ButtonState::Pressed,
                modifiers: oteryn_input_actions::Modifiers::NONE,
                repeat: false,
            };
            assert_eq!(
                panel
                    .draft
                    .hotkeys
                    .resolve(std::slice::from_ref(&north), false),
                vec![oteryn_client::hotkeys::ResolvedHotkey::General(
                    "movement.north".into()
                )]
            );

            let nonce = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos();
            let path = std::env::temp_dir().join(format!(
                "oteryn-quick-movement-{}-{nonce}-{}.json",
                std::process::id(),
                u8::from(english)
            ));
            panel.draft.save(&path)?;
            let loaded = ClientSettings::load(&path)?;
            std::fs::remove_file(&path)?;
            assert_eq!(
                loaded.effective_movement_keys(),
                [Some(9), Some(27), Some(22), Some(4)]
            );
            assert_eq!(
                loaded.hotkeys.resolve(std::slice::from_ref(&north), false),
                vec![oteryn_client::hotkeys::ResolvedHotkey::General(
                    "movement.north".into()
                )]
            );
        }
        Ok(())
    }

    #[test]
    fn screenshot_directory_is_a_sibling_of_the_settings_file() {
        let path = std::path::Path::new("/configuration/oteryn/settings.json");
        assert_eq!(
            screenshot_directory_for(path),
            Some(std::path::PathBuf::from(
                "/configuration/oteryn/screenshots"
            ))
        );
    }

    #[test]
    fn screenshot_runtime_selects_crop_only_for_explicit_enabled_intent() {
        let mut settings = ClientSettings::default();
        assert!(!game_only_capture(&settings));
        settings.future_preferences.insert(
            "screenshots.capture.game_only".into(),
            oteryn_client::settings_catalog::FutureValue::Bool(false),
        );
        assert!(!game_only_capture(&settings));
        settings.future_preferences.insert(
            "screenshots.capture.game_only".into(),
            oteryn_client::settings_catalog::FutureValue::Bool(true),
        );
        assert!(game_only_capture(&settings));
    }

    #[test]
    fn screenshots_are_lossless_png_and_never_overwrite_collisions()
    -> Result<(), Box<dyn std::error::Error>> {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_nanos();
        let directory = std::env::temp_dir().join(format!(
            "oteryn-screenshot-test-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir(&directory)?;
        let original = directory.join("oteryn-screenshot.png");
        std::fs::write(&original, b"existing")?;
        let frame = oteryn_renderer::CapturedFrame {
            width: 2,
            height: 1,
            rgba: vec![255, 0, 0, 255, 0, 255, 0, 127],
        };
        let first = save_screenshot_to(&directory, &frame)?;
        let second = save_screenshot_to(&directory, &frame)?;
        assert_eq!(std::fs::read(&original)?, b"existing");
        assert_eq!(
            first.file_name().and_then(std::ffi::OsStr::to_str),
            Some("oteryn-screenshot-0001.png")
        );
        assert_eq!(
            second.file_name().and_then(std::ffi::OsStr::to_str),
            Some("oteryn-screenshot-0002.png")
        );
        for path in [&first, &second] {
            let bytes = std::fs::read(path)?;
            assert_eq!(bytes.get(..8), Some(b"\x89PNG\r\n\x1a\n".as_slice()));
            let decoded =
                image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)?.to_rgba8();
            assert_eq!(decoded.dimensions(), (2, 1));
            assert_eq!(decoded.as_raw(), &frame.rgba);
        }
        for path in [first, second, original] {
            std::fs::remove_file(path)?;
        }
        std::fs::remove_dir(directory)?;
        Ok(())
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
                    runtime_action: None,
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
