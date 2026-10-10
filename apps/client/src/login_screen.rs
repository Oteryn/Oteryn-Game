use egui::{Align2, Color32, FontId, RichText, Vec2};
use oteryn_client::{
    AdmittedSession, ClientBootstrap, GameplayEntryError, NativeLoginConfig, SignedInAccount,
};
use oteryn_foundation::CancellationToken;
use oteryn_platform_client::native_login::PublicClass;
use oteryn_platform_client::{PlatformClient, PlatformClientConfig};
use std::sync::mpsc::{self, Receiver};

type AccountResult = Result<(ClientBootstrap, SignedInAccount), String>;
type EntryResult = Result<(ClientBootstrap, AdmittedSession, String), String>;
const BLUE: Color32 = Color32::from_rgb(10, 77, 157);
const BORDER: Color32 = Color32::from_rgb(54, 64, 74);
const MUTED: Color32 = Color32::from_rgb(167, 178, 189);
const GREEN: Color32 = Color32::from_rgb(86, 190, 99);
const GOLD: Color32 = Color32::from_rgb(208, 174, 83);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Screen {
    Welcome,
    Browser,
    Authorized,
    Characters,
    Channels,
    Connecting,
}

pub struct LoginScreen {
    pub context: egui::Context,
    background: Option<egui::TextureHandle>,
    config: Option<NativeLoginConfig>,
    screen: Screen,
    signing_in: Option<Receiver<AccountResult>>,
    connecting: Option<Receiver<EntryResult>>,
    cancellation: Option<CancellationToken>,
    account: Option<(ClientBootstrap, SignedInAccount)>,
    directory: Option<Receiver<Result<usize, ()>>>,
    world_count: Option<Result<usize, ()>>,
    selected: usize,
    error: Option<String>,
    english: bool,
    pub settings: crate::settings_ui::SettingsPanel,
    choose_account: bool,
    pub quit: bool,
}

impl LoginScreen {
    pub fn new() -> Self {
        let context = egui::Context::default();
        let mut fonts = egui::FontDefinitions::default();
        fonts.font_data.insert(
            "brand".into(),
            egui::FontData::from_static(include_bytes!("../assets/login-brand.ttf")).into(),
        );
        fonts
            .families
            .insert(egui::FontFamily::Name("brand".into()), vec!["brand".into()]);
        context.set_fonts(fonts);
        let mut style = (*context.style_of(egui::Theme::Dark)).clone();
        style.visuals = egui::Visuals::dark();
        style.visuals.override_text_color = Some(Color32::from_rgb(231, 235, 240));
        style.visuals.window_fill = Color32::from_rgba_unmultiplied(10, 15, 19, 248);
        style.visuals.panel_fill = Color32::from_rgb(10, 15, 19);
        style.visuals.widgets.inactive.bg_fill = Color32::from_rgb(23, 29, 35);
        style.visuals.widgets.inactive.weak_bg_fill = Color32::from_rgb(23, 29, 35);
        style.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, BORDER);
        style.visuals.widgets.hovered.bg_fill = Color32::from_rgb(32, 53, 75);
        style.visuals.selection.bg_fill = BLUE;
        style.visuals.window_corner_radius = egui::CornerRadius::same(6);
        style.spacing.item_spacing = egui::vec2(10.0, 10.0);
        style.spacing.button_padding = egui::vec2(14.0, 9.0);
        context.set_theme(egui::Theme::Dark);
        context.set_style_of(egui::Theme::Dark, style);
        let background = image::load_from_memory(include_bytes!("../assets/login-background.webp"))
            .ok()
            .map(|image| {
                let rgba = image.to_rgba8();
                context.load_texture(
                    "citadel",
                    egui::ColorImage::from_rgba_unmultiplied(
                        [rgba.width() as usize, rgba.height() as usize],
                        rgba.as_raw(),
                    ),
                    egui::TextureOptions::LINEAR,
                )
            });
        let config = NativeLoginConfig::from_launcher_env(|name| std::env::var(name).ok())
            .ok()
            .flatten();
        let settings = crate::settings_ui::SettingsPanel::new();
        let mut result = Self {
            context,
            background,
            config,
            screen: Screen::Welcome,
            signing_in: None,
            connecting: None,
            cancellation: None,
            account: None,
            directory: None,
            world_count: None,
            selected: 0,
            error: None,
            english: settings.current.english,
            settings,
            choose_account: false,
            quit: false,
        };
        result.refresh_directory();
        result
    }

    fn text<'a>(&self, pl: &'a str, en: &'a str) -> &'a str {
        if self.english { en } else { pl }
    }

    fn refresh_directory(&mut self) {
        let Some(config) = &self.config else {
            self.world_count = Some(Err(()));
            return;
        };
        let Ok(platform) = PlatformClientConfig::new(&config.platform_url) else {
            self.world_count = Some(Err(()));
            return;
        };
        let (sender, receiver) = mpsc::channel();
        self.directory = Some(receiver);
        self.world_count = None;
        std::thread::spawn(move || {
            let result = oteryn_client_runtime::ClientRuntime::new()
                .map_err(|_| ())
                .and_then(|runtime| {
                    let result = runtime
                        .block_on(async {
                            let api = PlatformClient::new(platform).map_err(|_| ())?;
                            let snapshot = api
                                .fetch_directory(runtime.cancellation())
                                .await
                                .map_err(|_| ())?;
                            Ok(snapshot.worlds.len())
                        })
                        .map_err(|_| ())?;
                    runtime.shutdown(std::time::Duration::from_millis(250));
                    result
                });
            let _ = sender.send(result);
        });
    }

    pub fn poll(&mut self) -> Option<(ClientBootstrap, AdmittedSession, String)> {
        if let Some(receiver) = &self.directory {
            match receiver.try_recv() {
                Ok(result) => {
                    self.world_count = Some(result);
                    self.directory = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.world_count = Some(Err(()));
                    self.directory = None;
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        if let Some(receiver) = &self.signing_in {
            match receiver.try_recv() {
                Ok(result) => {
                    self.signing_in = None;
                    self.cancellation = None;
                    match result {
                        Ok(account) => {
                            self.account = Some(account);
                            self.selected = 0;
                            self.screen = Screen::Authorized;
                        }
                        Err(message) => {
                            self.error = Some(message);
                            self.screen = Screen::Welcome;
                        }
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.signing_in = None;
                    self.screen = Screen::Welcome;
                    self.error = Some(
                        self.text(
                            "Logowanie przerwane. Spróbuj ponownie.",
                            "Sign-in interrupted. Please try again.",
                        )
                        .into(),
                    );
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        if let Some(receiver) = &self.connecting {
            match receiver.try_recv() {
                Ok(result) => {
                    self.connecting = None;
                    self.cancellation = None;
                    match result {
                        Ok(entry) => return Some(entry),
                        Err(message) => {
                            self.error = Some(message);
                            self.screen = Screen::Welcome;
                        }
                    }
                }
                Err(mpsc::TryRecvError::Disconnected) => {
                    self.connecting = None;
                    self.screen = Screen::Welcome;
                    self.error = Some(
                        self.text(
                            "Połączenie przerwane. Zaloguj się ponownie.",
                            "Connection interrupted. Please sign in again.",
                        )
                        .into(),
                    );
                }
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        None
    }

    fn sign_in(&mut self) {
        self.error = None;
        let Some(mut config) = self.config.clone() else {
            self.error = Some(
                self.text(
                    "Nie skonfigurowano portalu. Uruchom klienta przez launch-client-local.ps1.",
                    "Portal is not configured. Start the client using launch-client-local.ps1.",
                )
                .into(),
            );
            return;
        };
        config.choose_account = self.choose_account;
        self.choose_account = false;
        let client = match ClientBootstrap::new() {
            Ok(client) => client.with_native_login(config),
            Err(_) => {
                self.error = Some(
                    self.text(
                        "Nie udało się uruchomić logowania.",
                        "Unable to start sign-in.",
                    )
                    .into(),
                );
                return;
            }
        };
        self.cancellation = Some(client.login_cancellation());
        let (sender, receiver) = mpsc::channel();
        self.signing_in = Some(receiver);
        self.screen = Screen::Browser;
        let english = self.english;
        std::thread::spawn(move || {
            let result = client
                .sign_in()
                .map(|account| (client, account))
                .map_err(|error| public_error(error, english));
            let _ = sender.send(result);
        });
    }

    fn enter(&mut self) {
        let Some((client, account)) = self.account.take() else {
            return;
        };
        let Some(character) = account.characters.get(self.selected) else {
            self.account = Some((client, account));
            return;
        };
        let character_id = character.character_id.clone();
        let character_name = character.name.clone();
        self.cancellation = Some(client.login_cancellation());
        let (sender, receiver) = mpsc::channel();
        self.connecting = Some(receiver);
        self.screen = Screen::Connecting;
        let english = self.english;
        std::thread::spawn(move || {
            let result = client
                .enter_selected(account, &character_id)
                .map(|session| (client, session, character_name))
                .map_err(|error| public_error(error, english));
            let _ = sender.send(result);
        });
    }

    fn cancel(&mut self) {
        if let Some(token) = self.cancellation.take() {
            token.cancel();
        }
        self.signing_in = None;
        self.connecting = None;
        self.account = None;
        self.screen = Screen::Welcome;
    }

    fn open_portal(&self, path: &str) {
        if let Some(config) = &self.config {
            (config.open_browser)(&format!(
                "{}/{}",
                config.platform_url.trim_end_matches('/'),
                path
            ));
        }
    }

    pub fn show(&mut self, ctx: &egui::Context) {
        self.settings.set_action_bar_available(false);
        self.paint_background(ctx);
        self.toolbar(ctx);
        let width = match self.screen {
            Screen::Characters => 690.0,
            Screen::Channels => 480.0,
            _ => 360.0,
        };
        egui::Window::new("Oteryn")
            .id(egui::Id::new("login-panel"))
            .title_bar(false)
            .resizable(false)
            .collapsible(false)
            .anchor(Align2::CENTER_CENTER, [0.0, 35.0])
            .fixed_size(Vec2::new(width, 0.0))
            .frame(
                egui::Frame::window(&ctx.style_of(egui::Theme::Dark))
                    // 90% opacity: only the panel background is translucent.
                    .fill(Color32::from_rgba_unmultiplied(10, 15, 19, 230))
                    .inner_margin(20.0)
                    .stroke(egui::Stroke::new(1.0, BORDER)),
            )
            .show(ctx, |ui| {
                match self.screen {
                    Screen::Welcome => self.welcome(ui),
                    Screen::Browser => self.browser(ui),
                    Screen::Authorized => self.authorized(ui),
                    Screen::Characters => self.characters(ui),
                    Screen::Channels => self.channels(ui),
                    Screen::Connecting => self.connecting(ui),
                }
                if let Some(error) = &self.error {
                    ui.add_space(12.0);
                    ui.colored_label(Color32::from_rgb(237, 149, 128), error);
                }
            });
        if self.settings.open {
            self.settings_window(ctx);
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(33));
    }

    fn paint_background(&self, ctx: &egui::Context) {
        let rect = ctx.viewport_rect();
        let painter = ctx.layer_painter(egui::LayerId::background());
        if let Some(background) = &self.background {
            let size = background.size_vec2();
            let scale = (rect.width() / size.x).max(rect.height() / size.y);
            let visible = rect.size() / (size * scale);
            if self.settings.current.reduced_motion {
                painter.image(
                    background.id(),
                    rect,
                    egui::Rect::from_center_size(egui::pos2(0.5, 0.5), visible),
                    Color32::WHITE,
                );
            } else {
                crate::login_backdrop::paint(&painter, background, rect, ctx.input(|i| i.time));
            }
        }
        let darkness = if matches!(
            self.screen,
            Screen::Characters | Screen::Channels | Screen::Authorized
        ) {
            218
        } else {
            170
        };
        painter.rect_filled(rect, 0.0, Color32::from_black_alpha(darkness));
        let center = rect.center().x;
        let font = FontId::new(52.0, egui::FontFamily::Name("brand".into()));
        painter.text(
            egui::pos2(center + 2.0, 47.0),
            Align2::CENTER_TOP,
            "OTERYN",
            font.clone(),
            Color32::BLACK,
        );
        painter.text(
            egui::pos2(center, 45.0),
            Align2::CENTER_TOP,
            "OTERYN",
            font,
            Color32::from_rgb(231, 235, 240),
        );
        painter.text(
            egui::pos2(center, 108.0),
            Align2::CENTER_TOP,
            self.text(
                "W E J D Ź   D O   Ś W I A T A",
                "E N T E R   T H E   W O R L D",
            ),
            FontId::proportional(11.0),
            Color32::from_rgb(113, 190, 239),
        );
        let y = rect.height() - 24.0;
        painter.line_segment(
            [egui::pos2(0.0, y), egui::pos2(rect.width(), y)],
            egui::Stroke::new(1.0, BORDER),
        );
        painter.text(
            egui::pos2(center, y + 12.0),
            Align2::CENTER_CENTER,
            concat!("Oteryn ", env!("CARGO_PKG_VERSION"), "  |  Native client"),
            FontId::proportional(11.0),
            MUTED,
        );
    }

    fn toolbar(&mut self, ctx: &egui::Context) {
        let previous_language = self.english;
        egui::Area::new(egui::Id::new("language"))
            .anchor(Align2::RIGHT_TOP, [-20.0, 16.0])
            .show(ctx, |ui| {
                egui::ComboBox::from_id_salt("language-choice")
                    .selected_text(if self.english { "English" } else { "Polski" })
                    .width(90.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.english, false, "Polski");
                        ui.selectable_value(&mut self.english, true, "English");
                    });
            });
        if self.english != previous_language {
            self.settings.set_language(self.english);
        }
        egui::Area::new(egui::Id::new("settings-button"))
            .anchor(Align2::LEFT_BOTTOM, [18.0, -36.0])
            .show(ctx, |ui| {
                if ui
                    .add_sized(
                        [110.0, 30.0],
                        egui::Button::new(self.text("Ustawienia", "Settings")),
                    )
                    .clicked()
                {
                    self.settings.open = true;
                }
            });
        egui::Area::new(egui::Id::new("quit-button"))
            .anchor(Align2::RIGHT_BOTTOM, [-18.0, -36.0])
            .show(ctx, |ui| {
                self.quit = ui
                    .add_sized(
                        [82.0, 30.0],
                        egui::Button::new(self.text("Zamknij", "Quit")),
                    )
                    .clicked();
            });
        if self.screen == Screen::Welcome {
            egui::Area::new(egui::Id::new("directory-status"))
                .anchor(Align2::CENTER_BOTTOM, [0.0, -42.0])
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        match self.world_count {
                            Some(Ok(count)) if count > 0 => {
                                status_dot(ui, GREEN);
                                ui.label(self.text("Dostępne światy", "Worlds available"));
                            }
                            Some(Ok(_)) => {
                                status_dot(ui, GOLD);
                                ui.label(
                                    self.text("Brak dostępnych światów", "No worlds available"),
                                );
                            }
                            Some(Err(())) => {
                                status_dot(ui, Color32::from_rgb(211, 109, 88));
                                ui.label(self.text("Portal niedostępny", "Portal unavailable"));
                            }
                            None => {
                                if self.settings.current.reduced_motion {
                                    ui.label("…");
                                } else {
                                    ui.spinner();
                                }
                                ui.label(self.text("Sprawdzanie portalu…", "Checking portal…"));
                            }
                        }
                        if ui
                            .small_button("↻")
                            .on_hover_text(self.text("Odśwież", "Refresh"))
                            .clicked()
                        {
                            self.refresh_directory();
                        }
                    });
                });
        }
    }

    fn welcome(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.heading(RichText::new(self.text("Witaj w Oteryn", "Welcome")).size(23.0));
            ui.label(
                RichText::new(self.text(
                    "Wybierz, jak rozpocząć swoją przygodę.",
                    "Choose how to begin your adventure.",
                ))
                .size(12.0)
                .color(MUTED),
            );
            ui.add_space(12.0);
            if primary_button(
                ui,
                self.text("Zaloguj z Oteryn", "Sign in with Oteryn"),
                320.0,
                BLUE,
            )
            .clicked()
            {
                self.sign_in();
            }
            ui.label(
                RichText::new(self.text(
                    "Bezpieczne logowanie przez konto Oteryn.",
                    "Secure sign-in using your Oteryn account.",
                ))
                .size(11.0)
                .color(MUTED),
            );
            ui.add_space(6.0);
            ui.separator();
            ui.add_space(6.0);
            if ui
                .add_sized(
                    [320.0, 40.0],
                    egui::Button::new(self.text("Utwórz konto", "Create account")),
                )
                .clicked()
            {
                self.open_portal("register");
            }
            ui.label(
                RichText::new(self.text(
                    "Nowy w Oteryn? Dołącz do świata.",
                    "New to Oteryn? Join the world.",
                ))
                .size(11.0)
                .color(MUTED),
            );
            ui.add_space(4.0);
        });
    }

    fn browser(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.heading(self.text("Logowanie w przeglądarce", "Sign in in your browser"));
            ui.add_space(20.0);
            if self.settings.current.reduced_motion {
                ui.label("…");
            } else {
                ui.add(egui::Spinner::new().size(58.0));
            }
            ui.add_space(16.0);
            ui.label(self.text(
                "Zaloguj się w otwartym portalu Oteryn",
                "Sign in on the opened Oteryn portal",
            ));
            ui.label(
                RichText::new(self.text(
                    "i zatwierdź dostęp dla klienta gry.",
                    "and approve access for the game client.",
                ))
                .color(MUTED),
            );
            ui.add_space(20.0);
            ui.label(
                RichText::new(self.text(
                    "Po zatwierdzeniu wrócisz tutaj automatycznie.",
                    "You will return here automatically after approval.",
                ))
                .size(11.0)
                .color(MUTED),
            );
            ui.add_space(12.0);
            if ui
                .add_sized(
                    [150.0, 35.0],
                    egui::Button::new(self.text("Anuluj", "Cancel")),
                )
                .clicked()
            {
                self.cancel();
            }
        });
    }

    fn authorized(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.heading(self.text("Logowanie zakończone", "Authorization complete"));
            ui.add_space(18.0);
            let (rect, _) = ui.allocate_exact_size(Vec2::splat(80.0), egui::Sense::hover());
            let painter = ui.painter();
            painter.circle_stroke(rect.center(), 35.0, egui::Stroke::new(4.0, GREEN));
            painter.line_segment(
                [
                    rect.center() + egui::vec2(-17.0, 0.0),
                    rect.center() + egui::vec2(-4.0, 13.0),
                ],
                egui::Stroke::new(5.0, GREEN),
            );
            painter.line_segment(
                [
                    rect.center() + egui::vec2(-4.0, 13.0),
                    rect.center() + egui::vec2(21.0, -15.0),
                ],
                egui::Stroke::new(5.0, GREEN),
            );
            ui.add_space(12.0);
            ui.label(self.text(
                "Twoje konto Oteryn jest zalogowane.",
                "You are signed in with Oteryn.",
            ));
            ui.label(
                RichText::new(self.text(
                    "Możesz teraz wybrać swoją postać.",
                    "You can now select your character.",
                ))
                .color(MUTED),
            );
            ui.add_space(18.0);
            if primary_button(
                ui,
                self.text("Wybierz postać", "Select character"),
                280.0,
                BLUE,
            )
            .clicked()
            {
                self.screen = Screen::Characters;
            }
        });
    }

    fn characters(&mut self, ui: &mut egui::Ui) {
        let english = self.english;
        let tr = |pl, en| if english { en } else { pl };
        let Some((_, account)) = &self.account else {
            return;
        };
        ui.vertical_centered(|ui| {
            ui.heading(tr("Wybierz postać", "Select character"));
        });
        ui.add_space(8.0);
        ui.columns(2, |columns| {
            egui::ScrollArea::vertical()
                .max_height(252.0)
                .show(&mut columns[0], |ui| {
                    for (index, character) in account.characters.iter().enumerate() {
                        let selected = self.selected == index;
                        let available = character.availability == "AVAILABLE";
                        let name = &character.name;
                        let world = account
                            .worlds
                            .iter()
                            .find(|world| world.world_ref.as_str() == character.world_id)
                            .map(|world| world.display_name.as_str())
                            .unwrap_or(tr("Świat niedostępny", "World unavailable"));
                        let (rect, response) = ui.allocate_exact_size(
                            Vec2::new(ui.available_width(), 68.0),
                            egui::Sense::click(),
                        );
                        let hovered = response.hovered();
                        let fill = if selected {
                            Color32::from_rgb(30, 29, 19)
                        } else if hovered {
                            Color32::from_rgb(27, 36, 46)
                        } else {
                            Color32::from_rgb(14, 19, 24)
                        };
                        ui.painter().rect(
                            rect,
                            4.0,
                            fill,
                            egui::Stroke::new(1.0, if selected { GOLD } else { BORDER }),
                            egui::StrokeKind::Inside,
                        );
                        let icon = egui::pos2(rect.left() + 27.0, rect.center().y);
                        ui.painter().circle_stroke(
                            icon,
                            17.0,
                            egui::Stroke::new(1.0, if selected { GOLD } else { MUTED }),
                        );
                        ui.painter().text(
                            icon,
                            Align2::CENTER_CENTER,
                            name.chars().next().unwrap_or('·'),
                            FontId::proportional(21.0),
                            if selected { GOLD } else { MUTED },
                        );
                        let x = rect.left() + 55.0;
                        ui.painter().text(
                            egui::pos2(x, rect.top() + 12.0),
                            Align2::LEFT_TOP,
                            name,
                            FontId::proportional(16.0),
                            if selected { GOLD } else { Color32::WHITE },
                        );
                        ui.painter().text(
                            egui::pos2(x, rect.top() + 37.0),
                            Align2::LEFT_TOP,
                            if available {
                                world
                            } else {
                                tr("Postać niedostępna", "Character unavailable")
                            },
                            FontId::proportional(11.0),
                            MUTED,
                        );
                        if response.clicked() && available {
                            self.selected = index;
                        }
                    }
                    if account.characters.is_empty() {
                        ui.label(tr(
                            "Nie znaleziono postaci dla tego konta. Możesz zmienić konto poniżej.",
                            "No characters found for this account. You can switch accounts below.",
                        ));
                    }
                });
            let right = &mut columns[1];
            if let Some(character) = account.characters.get(self.selected) {
                let world = account
                    .worlds
                    .iter()
                    .find(|world| world.world_ref.as_str() == character.world_id);
                egui::Frame::new()
                    .fill(Color32::from_rgb(13, 18, 23))
                    .stroke(egui::Stroke::new(1.0, BORDER))
                    .inner_margin(14.0)
                    .show(right, |ui| {
                        ui.set_min_width(260.0);
                        ui.label(RichText::new(tr("ŚWIAT", "WORLD")).size(11.0).color(MUTED));
                        ui.add_space(4.0);
                        ui.label(
                            RichText::new(
                                world
                                    .map(|world| world.display_name.as_str())
                                    .unwrap_or(tr("Niedostępny", "Unavailable")),
                            )
                            .size(19.0),
                        );
                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(tr("KANAŁ", "CHANNEL"))
                                .size(11.0)
                                .color(MUTED),
                        );
                        ui.label(tr("Przydział automatyczny", "Automatic allocation"));
                        if ui
                            .add_sized(
                                [260.0, 32.0],
                                egui::Button::new(tr("Zobacz kanały", "View channels")),
                            )
                            .clicked()
                        {
                            self.screen = Screen::Channels;
                        }
                        ui.add_space(8.0);
                        ui.label(
                            RichText::new(tr(
                                "Serwer przydzieli kanał podczas wejścia do świata.",
                                "The server allocates a channel when you enter the world.",
                            ))
                            .size(11.0)
                            .color(MUTED),
                        );
                    });
            }
        });
        ui.add_space(10.0);
        ui.separator();
        ui.add_space(4.0);
        let available = account
            .characters
            .get(self.selected)
            .is_some_and(|character| {
                character.availability == "AVAILABLE"
                    && account
                        .worlds
                        .iter()
                        .any(|world| world.world_ref.as_str() == character.world_id)
            });
        let mut enter = false;
        let mut logout = false;
        let mut switch_account = false;
        ui.horizontal(|ui| {
            if ui
                .button(tr("Zarządzaj postaciami", "Manage characters"))
                .clicked()
            {
                self.open_portal("account");
            }
            switch_account = ui.button(tr("Zmień konto", "Switch account")).clicked();
            logout = ui.button(tr("Wyloguj", "Logout")).clicked();
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                enter = ui
                    .add_enabled(
                        available,
                        egui::Button::new(tr("Wejdź do gry", "Enter game"))
                            .min_size(Vec2::new(145.0, 40.0))
                            .fill(Color32::from_rgb(39, 83, 38)),
                    )
                    .clicked();
            });
        });
        if logout {
            self.cancel();
        }
        if switch_account {
            self.cancel();
            self.choose_account = true;
            self.sign_in();
        }
        if enter {
            self.enter();
        }
    }

    fn channels(&mut self, ui: &mut egui::Ui) {
        let Some((_, account)) = &self.account else {
            return;
        };
        let Some(character) = account.characters.get(self.selected) else {
            return;
        };
        let world = account
            .worlds
            .iter()
            .find(|world| world.world_ref.as_str() == character.world_id);
        ui.vertical_centered(|ui| {
            ui.heading(self.text("Kanały świata", "World channels"));
            if let Some(world) = world {
                ui.label(RichText::new(world.display_name.as_str()).color(MUTED));
            }
        });
        ui.add_space(10.0);
        if let Some(world) = world {
            for channel in &world.channels {
                egui::Frame::new()
                    .fill(Color32::from_rgb(15, 22, 28))
                    .stroke(egui::Stroke::new(1.0, BORDER))
                    .inner_margin(14.0)
                    .show(ui, |ui| {
                        ui.set_min_width(400.0);
                        ui.label(RichText::new(channel.display_name.as_str()).size(17.0));
                        ui.label(
                            RichText::new(self.text(
                                "Kanał opublikowany przez portal",
                                "Channel listed by the portal",
                            ))
                            .size(11.0)
                            .color(MUTED),
                        );
                    });
            }
            if world.channels.is_empty() {
                ui.label(self.text("Brak opublikowanych kanałów.", "No published channels."));
            }
        }
        ui.add_space(10.0);
        ui.label(
            RichText::new(self.text(
                "Kanał przydziela serwer. Ręczny wybór nie jest jeszcze dostępny.",
                "The server allocates your channel. Manual selection is not available yet.",
            ))
            .size(12.0)
            .color(MUTED),
        );
        ui.add_space(12.0);
        if ui
            .add_sized([130.0, 35.0], egui::Button::new(self.text("Wróć", "Back")))
            .clicked()
        {
            self.screen = Screen::Characters;
        }
    }

    fn connecting(&mut self, ui: &mut egui::Ui) {
        ui.vertical_centered(|ui| {
            ui.heading(self.text("Łączenie ze światem", "Connecting"));
            ui.add_space(24.0);
            if self.settings.current.reduced_motion {
                ui.label("…");
            } else {
                ui.add(egui::Spinner::new().size(76.0));
            }
            ui.add_space(24.0);
            ui.label(self.text(
                "Potwierdzanie wejścia do gry…",
                "Confirming entry to the game…",
            ));
            ui.label(RichText::new(self.text("Proszę czekać.", "Please wait.")).color(MUTED));
            ui.add_space(24.0);
            if ui
                .add_sized(
                    [140.0, 35.0],
                    egui::Button::new(self.text("Anuluj", "Cancel")),
                )
                .clicked()
            {
                self.cancel();
            }
        });
    }

    pub fn settings_window(&mut self, ctx: &egui::Context) {
        let refresh = self.settings.show(
            ctx,
            self.config.as_ref().map(|c| c.platform_url.as_str()),
            self.config.as_ref().map(|c| c.gateway_url.as_str()),
        );
        self.english = self.settings.current.english;
        if refresh {
            self.refresh_directory();
        }
    }
}

fn status_dot(ui: &mut egui::Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(10.0), egui::Sense::hover());
    ui.painter().circle_filled(rect.center(), 4.0, color);
}

fn primary_button(ui: &mut egui::Ui, label: &str, width: f32, fill: Color32) -> egui::Response {
    ui.add_sized(
        [width, 46.0],
        egui::Button::new(RichText::new(label).size(16.0).color(Color32::WHITE))
            .fill(fill)
            .stroke(egui::Stroke::new(1.0, Color32::from_rgb(37, 115, 194))),
    )
}

fn public_error(error: GameplayEntryError, english: bool) -> String {
    let (pl, en) = match error {
        GameplayEntryError::WorldUnavailable => (
            "Świat jest obecnie niedostępny. Spróbuj ponownie za chwilę.",
            "The world is unavailable. Please try again shortly.",
        ),
        GameplayEntryError::InvalidConfiguration => (
            "Konfiguracja połączenia jest niepoprawna.",
            "Connection configuration is invalid.",
        ),
        GameplayEntryError::Rejected(
            PublicClass::RetryLogin | PublicClass::AuthenticationRequired,
        ) => (
            "Zaloguj się ponownie przez portal Oteryn.",
            "Please sign in again through the Oteryn portal.",
        ),
        GameplayEntryError::Rejected(PublicClass::CharacterAlreadyActive) => (
            "Postać ma aktywne połączenie. Zamknij poprzedniego klienta i spróbuj za chwilę.",
            "This character has an active connection. Close the previous client and try again shortly.",
        ),
        GameplayEntryError::Rejected(PublicClass::ClientUpdateRequired) => (
            "Serwer wymaga nowszej wersji klienta.",
            "The server requires a newer client version.",
        ),
        _ => (
            "Nie udało się połączyć. Spróbuj ponownie za chwilę.",
            "Connection failed. Please try again shortly.",
        ),
    };
    if english { en.into() } else { pl.into() }
}

impl Drop for LoginScreen {
    fn drop(&mut self) {
        self.cancel();
    }
}
