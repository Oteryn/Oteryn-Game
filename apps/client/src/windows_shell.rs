use oteryn_client::hotkeys::ResolvedHotkey;
use oteryn_client::input::{MouseActions, StepDir, click_tile};
use oteryn_client::play::{PlayLink, PlayView};
use oteryn_client::scene::Scene;
use oteryn_client::spell::SpellHotkeys;
use oteryn_client::world::{START, World};
use oteryn_client::{AdmittedSession, ClientBootstrap, GameplayEntryError};
use oteryn_foundation::ProcessGeneration;
use oteryn_input_platform::InputPlatformAdapter;
use oteryn_platform_client::native_login::PublicClass;
use oteryn_renderer::{SurfacePhase, TileCoord, WindowsRenderer};
use std::fmt::{self, Display, Formatter};
use std::sync::Arc;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{DeviceEvent, DeviceId, WindowEvent};
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowAttributes, WindowId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShellError {
    EventLoopCreation,
    EventLoopRun,
    WindowCreation,
    InputInitialization,
    RendererInitialization,
    RendererResume,
    RendererSuspend,
    RendererResize,
    RendererRender,
    RendererClose,
}

impl Display for ShellError {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::EventLoopCreation => "client event loop creation failed",
            Self::EventLoopRun => "client event loop failed",
            Self::WindowCreation => "client window creation failed",
            Self::InputInitialization => "client input initialization failed",
            Self::RendererInitialization => "client renderer initialization failed",
            Self::RendererResume => "client renderer resume failed",
            Self::RendererSuspend => "client renderer suspend failed",
            Self::RendererResize => "client renderer resize failed",
            Self::RendererRender => "client renderer render failed",
            Self::RendererClose => "client renderer close failed",
        })
    }
}

impl std::error::Error for ShellError {}

struct Application {
    smoke: bool,
    login: Option<crate::login_screen::LoginScreen>,
    ui_state: Option<egui_winit::State>,
    game_ui: Option<crate::login_screen::LoginScreen>,
    hud: crate::game_ui::GameUi,
    preferences: oteryn_client::settings::ClientSettings,
    last_redraw: std::time::Instant,
    focused: bool,
    window: Option<Arc<Window>>,
    renderer: Option<WindowsRenderer<Arc<Window>>>,
    /// The map, loaded once the window exists; the offline scene and the play view share it.
    world: Option<Arc<World>>,
    /// Without a session: the scene around `offline_own`, which the arrow keys move locally.
    scene: Option<Scene>,
    offline_own: TileCoord,
    offline_facing: StepDir,
    client: Option<ClientBootstrap>,
    play: Option<Play>,
    generation: ProcessGeneration,
    input: InputPlatformAdapter,
    actions: MouseActions,
    hotkeys: SpellHotkeys,
    fatal_error: Option<ShellError>,
}

/// The view of an admitted session and the link to its task on the client runtime.
struct Play {
    view: PlayView,
    link: PlayLink,
    /// The spell bar and combat line last shown in the title.
    status: String,
}

impl Application {
    fn new(
        smoke: bool,
        play: Option<(ClientBootstrap, AdmittedSession)>,
    ) -> Result<Self, ShellError> {
        let (client, play) = match play {
            Some((client, admitted)) => {
                let (view, link) = client
                    .start_play(admitted)
                    .map_err(|_error| ShellError::RendererInitialization)?;
                (
                    Some(client),
                    Some(Play {
                        view,
                        link,
                        status: String::new(),
                    }),
                )
            }
            None => (None, None),
        };
        let login = if play.is_none() && !smoke {
            Some(crate::login_screen::LoginScreen::new())
        } else {
            None
        };
        let game_ui = play
            .as_ref()
            .map(|_| crate::login_screen::LoginScreen::new());
        let preferences = login
            .as_ref()
            .map(|ui| ui.settings.current.clone())
            .unwrap_or_default();
        Ok(Self {
            smoke,
            login,
            ui_state: None,
            game_ui,
            hud: crate::game_ui::GameUi::default(),
            preferences,
            last_redraw: std::time::Instant::now(),
            focused: true,
            window: None,
            renderer: None,
            world: None,
            scene: None,
            offline_own: TileCoord::new(START.0, START.1),
            offline_facing: StepDir::South,
            client,
            play,
            generation: ProcessGeneration::new(1),
            input: InputPlatformAdapter::new(),
            actions: MouseActions::new().map_err(|_error| ShellError::InputInitialization)?,
            hotkeys: SpellHotkeys::new().map_err(|_error| ShellError::InputInitialization)?,
            fatal_error: None,
        })
    }

    fn handle_input(
        &mut self,
        event_loop: &ActiveEventLoop,
        events: &[oteryn_input_actions::NormalizedInputEvent],
        consumed: bool,
    ) {
        let modal = self.game_ui.as_ref().is_some_and(|ui| ui.settings.open)
            || self.hud.blocks_game_input();
        let typing = self
            .game_ui
            .as_ref()
            .is_some_and(|ui| ui.context.egui_wants_keyboard_input());
        if let Some(play) = &self.play {
            self.hud
                .route_actions(events, &play.link, &self.preferences, typing, modal);
        }
        let (hotkey_direction, hotkey_matched) = self.route_general_hotkeys(events, typing, modal);
        let Ok(clicks) =
            self.actions
                .route_with_ui(events, typing, modal || self.login.is_some(), consumed)
        else {
            self.fail(event_loop, ShellError::InputInitialization);
            return;
        };
        if consumed && hotkey_direction.is_none() {
            return;
        }
        if self.login.is_some() || modal {
            return;
        }
        if hotkey_direction.is_none()
            && self
                .game_ui
                .as_ref()
                .is_some_and(|ui| ui.context.egui_wants_keyboard_input())
        {
            return;
        }
        let viewport = self.window.as_ref().and_then(|window| {
            let size = window.inner_size();
            oteryn_client::layout::GameViewport::fit_with_action_rows(
                size.width as f32,
                size.height as f32,
                window.scale_factor() as f32 * self.preferences.ui_scale,
                self.preferences.show_chat,
                self.preferences.action_bar.visible_rows(),
            )
        });
        if viewport.is_none() {
            return;
        }
        if let Some(play) = &mut self.play {
            let direction = hotkey_direction.or_else(|| {
                (!hotkey_matched)
                    .then(|| self.preferences.direction(events))
                    .flatten()
            });
            if let Some(direction) = direction {
                play.view.arrow(direction);
            }
            for spell in self.hotkeys.route(events) {
                play.view.cast(spell);
            }
            let mut render_failed = false;
            for click in clicks
                .into_iter()
                .filter(|_| self.preferences.click_to_walk)
            {
                if let Some((x, y)) = viewport.and_then(|view| view.scene_point(click.x, click.y))
                    && let Some(tile) = click_tile(play.view.scene().view(), x, y)
                {
                    render_failed |= play.view.click(tile).is_err();
                }
            }
            if render_failed {
                self.fail(event_loop, ShellError::RendererRender);
            }
            return;
        }
        let direction = hotkey_direction.or_else(|| {
            (!hotkey_matched)
                .then(|| self.preferences.direction(events))
                .flatten()
        });
        if let Some(direction) = direction {
            self.offline_own = direction.from(self.offline_own);
            self.offline_facing = direction;
            if self.rebuild_offline().is_err() {
                self.fail(event_loop, ShellError::RendererRender);
                return;
            }
        }
        for click in clicks {
            let picked = self.scene.as_mut().and_then(|scene| {
                let tile = click_tile(scene.view(), click.x, click.y)?;
                Some((tile, scene.select_tile(tile)))
            });
            // Without a session a click only selects a visible entity or object.
            if let Some((_, Err(_error))) = picked {
                self.fail(event_loop, ShellError::RendererRender);
            }
        }
    }

    fn route_general_hotkeys(
        &mut self,
        events: &[oteryn_input_actions::NormalizedInputEvent],
        chat_on: bool,
        modal: bool,
    ) -> (Option<StepDir>, bool) {
        if self.login.is_some() {
            return (None, false);
        }
        let actions = self.preferences.hotkeys.resolve(events, chat_on);
        let matched = actions.iter().any(|action| {
            matches!(action, ResolvedHotkey::General(_))
                || self.play.is_some() && matches!(action, ResolvedHotkey::Custom(_))
        });
        let mut movement = None;
        let mut settings_changed = false;
        for action in actions {
            let ResolvedHotkey::General(action) = action else {
                continue;
            };
            if action == "client.options" {
                if let Some(ui) = self.game_ui.as_mut() {
                    ui.settings.open = !ui.settings.open;
                }
                continue;
            }
            if modal {
                continue;
            }
            match action.as_str() {
                "movement.north" => movement = Some(StepDir::North),
                "movement.east" => movement = Some(StepDir::East),
                "movement.south" => movement = Some(StepDir::South),
                "movement.west" => movement = Some(StepDir::West),
                "client.fullscreen" => {
                    self.preferences.fullscreen = !self.preferences.fullscreen;
                    settings_changed = true;
                }
                "action.bottom.all" => {
                    self.preferences.action_bar.edge_enabled[0] =
                        !self.preferences.action_bar.edge_enabled[0];
                    settings_changed = true;
                }
                "action.left.all" => {
                    self.preferences.action_bar.edge_enabled[1] =
                        !self.preferences.action_bar.edge_enabled[1];
                    settings_changed = true;
                }
                "action.right.all" => {
                    self.preferences.action_bar.edge_enabled[2] =
                        !self.preferences.action_bar.edge_enabled[2];
                    settings_changed = true;
                }
                action if action.starts_with("action.") => {
                    let row = match action {
                        "action.bottom.1" => Some(0),
                        "action.bottom.2" => Some(1),
                        "action.bottom.3" => Some(2),
                        "action.left.1" => Some(3),
                        "action.left.2" => Some(4),
                        "action.left.3" => Some(5),
                        "action.right.1" => Some(6),
                        "action.right.2" => Some(7),
                        "action.right.3" => Some(8),
                        _ => None,
                    };
                    if let Some(row) = row
                        && let Some(mut preferences) = self.preferences.action_bar.row(row)
                    {
                        preferences.visible = !preferences.visible;
                        if self
                            .preferences
                            .action_bar
                            .set_row(row, preferences)
                            .is_ok()
                        {
                            settings_changed = true;
                        }
                    }
                }
                _ => {}
            }
        }
        if settings_changed {
            if let Some(window) = &self.window {
                window.set_fullscreen(if self.preferences.fullscreen {
                    Some(winit::window::Fullscreen::Borderless(None))
                } else {
                    None
                });
            }
            if let Some(ui) = self.game_ui.as_mut() {
                ui.settings.replace_current(self.preferences.clone());
            }
            let saved = oteryn_client::settings::ClientSettings::path()
                .is_some_and(|path| self.preferences.save(&path).is_ok());
            if !saved {
                self.hud.preferences_save_failed(self.preferences.english);
            }
        }
        (movement, matched)
    }

    /// The offline scene around `offline_own`, over the loaded world.
    fn rebuild_offline(&mut self) -> Result<(), oteryn_renderer::BatchError> {
        if let Some(world) = &self.world {
            let origin = TileCoord::new(0, 0);
            self.scene = Some(Scene::centered_on(
                Arc::clone(world),
                Some(origin),
                self.offline_own,
                self.offline_facing,
                &[],
            )?);
        }
        Ok(())
    }

    /// A session end returns to the pre-admission state: the offline scene, with the public
    /// class in the title and on the console. The session task is gone with its link.
    fn return_to_login(&mut self, class: PublicClass) {
        self.play = None;
        self.game_ui = None;
        self.hud = crate::game_ui::GameUi::default();
        self.login = Some(crate::login_screen::LoginScreen::new());
        if let (Some(login), Some(window)) = (&self.login, &self.window) {
            self.ui_state = Some(egui_winit::State::new(
                login.context.clone(),
                egui::ViewportId::ROOT,
                window.as_ref(),
                Some(window.scale_factor() as f32),
                None,
                None,
            ));
        }
        let message = GameplayEntryError::Rejected(class).to_string();
        println!("Oteryn: {message}");
        if let Some(window) = &self.window {
            window.set_title(&format!("Oteryn — {message}"));
        }
        if self.rebuild_offline().is_err() {
            self.scene = None;
        }
    }

    fn fail(&mut self, event_loop: &ActiveEventLoop, error: ShellError) {
        retain_first_error(&mut self.fatal_error, error);
        event_loop.exit();
    }
}

fn retain_first_error(slot: &mut Option<ShellError>, error: ShellError) {
    if slot.is_none() {
        *slot = Some(error);
    }
}

const fn redraw_eligible(phase: Option<SurfacePhase>) -> bool {
    matches!(phase, Some(SurfacePhase::Configured))
}

impl ApplicationHandler for Application {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if let (Some(window), Some(renderer)) = (&self.window, &mut self.renderer) {
            let size = window.inner_size();
            if renderer
                .resume(self.generation, size.width, size.height)
                .is_err()
            {
                self.fail(event_loop, ShellError::RendererResume);
            }
            return;
        }
        if self.window.is_some() {
            return;
        }
        let title = if self.play.is_some() {
            "Oteryn".to_owned()
        } else {
            "Oteryn — logowanie".to_owned()
        };
        let attributes = WindowAttributes::default()
            .with_title(title)
            .with_inner_size(LogicalSize::new(
                self.preferences.window_width,
                self.preferences.window_height,
            ));
        let Ok(window) = event_loop.create_window(attributes) else {
            self.fail(event_loop, ShellError::WindowCreation);
            return;
        };
        let window = Arc::new(window);
        if self.smoke {
            self.window = Some(window);
            event_loop.exit();
            return;
        }
        let size = window.inner_size();
        // The map is loaded on the first resume and kept across suspends.
        let world = if let Some(world) = &self.world {
            Arc::clone(world)
        } else {
            let (world, reason) = World::from_env();
            if let Some(reason) = reason {
                println!("Oteryn: drawing without map sprites: {reason}");
            }
            let Ok(world) = world.map(Arc::new) else {
                self.fail(event_loop, ShellError::RendererInitialization);
                return;
            };
            self.world = Some(Arc::clone(&world));
            world
        };
        let shown = match &mut self.play {
            Some(play) => play.view.set_world(Arc::clone(&world)),
            None => self.rebuild_offline(),
        };
        if shown.is_err() {
            self.fail(event_loop, ShellError::RendererInitialization);
            return;
        }
        let Ok(mut renderer) = WindowsRenderer::new(
            Arc::clone(&window),
            self.generation,
            size.width,
            size.height,
        ) else {
            self.fail(event_loop, ShellError::RendererInitialization);
            return;
        };
        // One atlas for the offline scene and the play view, uploaded once.
        renderer.set_atlas(world.atlas().clone());
        if let Some(login) = self.login.as_ref().or(self.game_ui.as_ref()) {
            self.ui_state = Some(egui_winit::State::new(
                login.context.clone(),
                egui::ViewportId::ROOT,
                window.as_ref(),
                Some(window.scale_factor() as f32),
                None,
                None,
            ));
        }
        window.request_redraw();
        self.window = Some(window);
        self.renderer = Some(renderer);
    }

    fn suspended(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(renderer) = &mut self.renderer
            && renderer.suspend(self.generation).is_err()
        {
            self.fail(event_loop, ShellError::RendererSuspend);
        }
    }

    fn device_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _device_id: DeviceId,
        event: DeviceEvent,
    ) {
        if let Ok(events) = self.input.process_device_event(&event) {
            self.handle_input(event_loop, &events, false);
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        let consumed = if let (Some(state), Some(window)) = (&mut self.ui_state, &self.window) {
            state.on_window_event(window, &event).consumed
        } else {
            false
        };
        // Every window event goes through the adapter and router first; select or walk is
        // triggered only by the routed gameplay action.
        if let Ok(events) = self.input.process_window_event(&event) {
            self.handle_input(event_loop, &events, consumed);
        }
        match event {
            WindowEvent::Focused(focused) => {
                self.focused = focused;
            }
            WindowEvent::KeyboardInput { event, .. }
                if event.state == winit::event::ElementState::Pressed
                    && !event.repeat
                    && event.physical_key
                        == winit::keyboard::PhysicalKey::Code(winit::keyboard::KeyCode::F10) =>
            {
                if let Some(ui) = self.login.as_mut().or(self.game_ui.as_mut()) {
                    ui.settings.open = !ui.settings.open;
                }
            }
            WindowEvent::CloseRequested => {
                if let Some(renderer) = &mut self.renderer
                    && renderer.close(self.generation).is_err()
                {
                    self.fail(event_loop, ShellError::RendererClose);
                    return;
                }
                event_loop.exit();
            }
            WindowEvent::Resized(size) => {
                if let Some(renderer) = &mut self.renderer
                    && renderer
                        .resize(self.generation, size.width, size.height)
                        .is_err()
                {
                    self.fail(event_loop, ShellError::RendererResize);
                }
            }
            WindowEvent::RedrawRequested => {
                if let (Some(login), Some(state), Some(window), Some(renderer)) = (
                    &mut self.login,
                    &mut self.ui_state,
                    &self.window,
                    &mut self.renderer,
                ) {
                    if !redraw_eligible(Some(renderer.state().phase())) {
                        return;
                    }
                    let _ = renderer.set_scene_viewport(None);
                    let size = window.inner_size();
                    let scale = window.scale_factor() as f32;
                    let zoom = ((size.width as f32 / scale / 900.0)
                        .min(size.height as f32 / scale / 620.0))
                    .clamp(1.0, 1.8);
                    login
                        .context
                        .set_zoom_factor(zoom * self.preferences.ui_scale);
                    let input = state.take_egui_input(window);
                    let context = login.context.clone();
                    let output = context.run_ui(input, |ui| login.show(ui.ctx()));
                    state.handle_platform_output(window, output.platform_output.clone());
                    if renderer
                        .render_ui(self.generation, &context, output)
                        .is_err()
                    {
                        self.fail(event_loop, ShellError::RendererRender);
                    } else if login.quit {
                        event_loop.exit();
                    }
                    return;
                }
                if let (Some(gui), Some(state), Some(window), Some(renderer), Some(play)) = (
                    &mut self.game_ui,
                    &mut self.ui_state,
                    &self.window,
                    &mut self.renderer,
                    &self.play,
                ) {
                    if !redraw_eligible(Some(renderer.state().phase())) {
                        return;
                    }
                    gui.context.set_zoom_factor(self.preferences.ui_scale);
                    let input = state.take_egui_input(window);
                    let context = gui.context.clone();
                    let size = window.inner_size();
                    let viewport = oteryn_client::layout::GameViewport::fit_with_action_rows(
                        size.width as f32,
                        size.height as f32,
                        window.scale_factor() as f32 * self.preferences.ui_scale,
                        self.preferences.show_chat,
                        self.preferences.action_bar.visible_rows(),
                    );
                    if renderer
                        .set_scene_viewport(viewport.map(|viewport| {
                            [viewport.x, viewport.y, viewport.width, viewport.height]
                        }))
                        .is_err()
                    {
                        self.fail(event_loop, ShellError::RendererRender);
                        return;
                    }
                    let output = context.run_ui(input, |ui| {
                        let scene = oteryn_client::layout::GameViewport::fit_with_action_rows(
                            ui.ctx().content_rect().width(),
                            ui.ctx().content_rect().height(),
                            1.0,
                            self.preferences.show_chat,
                            self.preferences.action_bar.visible_rows(),
                        )
                        .map(|view| {
                            egui::Rect::from_min_size(
                                egui::pos2(view.x, view.y),
                                egui::vec2(view.width, view.height),
                            )
                        });
                        crate::client_chrome::backdrop(ui.ctx(), scene);
                        gui.settings.set_action_bar_available(true);
                        if !gui.settings.open
                            && self
                                .hud
                                .show(ui.ctx(), &play.link, &play.view, &self.preferences)
                        {
                            gui.settings.open = true;
                        }
                        gui.settings_window(ui.ctx());
                        while let Some(row) = gui.settings.take_clear_action_row() {
                            self.hud.clear_action_row(row, &self.preferences);
                        }
                    });
                    if let Some(preferences) = self.hud.take_preferences() {
                        let saved = oteryn_client::settings::ClientSettings::path()
                            .is_some_and(|path| preferences.save(&path).is_ok());
                        if saved {
                            self.preferences = preferences.clone();
                            gui.settings.replace_current(preferences);
                        } else {
                            self.hud.preferences_save_failed(preferences.english);
                        }
                    }
                    state.handle_platform_output(window, output.platform_output.clone());
                    let scene = play.view.scene();
                    let rendered = if viewport.is_some() {
                        renderer.render_batches_ui(
                            self.generation,
                            scene.tiles(),
                            scene.sprites(),
                            &context,
                            output,
                        )
                    } else {
                        renderer.render_ui(self.generation, &context, output)
                    };
                    if rendered.is_err() {
                        self.fail(event_loop, ShellError::RendererRender);
                    }
                    return;
                }
                let scene = self
                    .play
                    .as_ref()
                    .map(|play| play.view.scene())
                    .or(self.scene.as_ref());
                if let (Some(renderer), Some(scene)) = (&mut self.renderer, scene)
                    && redraw_eligible(Some(renderer.state().phase()))
                    && renderer
                        .render_batches(self.generation, scene.tiles(), scene.sprites())
                        .is_err()
                {
                    self.fail(event_loop, ShellError::RendererRender);
                }
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Some(ui) = self.login.as_mut().or(self.game_ui.as_mut())
            && ui.settings.applied
        {
            ui.settings.applied = false;
            self.preferences = ui.settings.current.clone();
            if let Some(window) = &self.window {
                window.set_fullscreen(if self.preferences.fullscreen {
                    Some(winit::window::Fullscreen::Borderless(None))
                } else {
                    None
                });
                if !self.preferences.fullscreen {
                    let _ = window.request_inner_size(LogicalSize::new(
                        self.preferences.window_width,
                        self.preferences.window_height,
                    ));
                }
            }
            if let Some(renderer) = &mut self.renderer
                && renderer
                    .set_vsync(self.generation, self.preferences.vsync)
                    .is_err()
            {
                self.fail(event_loop, ShellError::RendererRender);
                return;
            }
            if self.play.is_some() {
                crate::client_chrome::install(&ui.context, self.preferences.high_contrast);
            } else {
                let mut visuals = ui.context.style_of(egui::Theme::Dark).visuals.clone();
                visuals.window_fill = if self.preferences.high_contrast {
                    egui::Color32::BLACK
                } else {
                    egui::Color32::from_rgb(10, 15, 19)
                };
                visuals.override_text_color = Some(if self.preferences.high_contrast {
                    egui::Color32::WHITE
                } else {
                    egui::Color32::from_rgb(231, 235, 240)
                });
                ui.context.set_visuals(visuals);
            }
        }
        if let Some(login) = &mut self.login
            && let Some((client, admitted, character_name)) = login.poll()
        {
            if self
                .preferences
                .auto_switch_hotkey_profile(&character_name)
                .unwrap_or(false)
            {
                let saved = oteryn_client::settings::ClientSettings::path()
                    .is_some_and(|path| self.preferences.save(&path).is_ok());
                login.settings.replace_current(self.preferences.clone());
                if !saved {
                    self.hud.preferences_save_failed(self.preferences.english);
                }
            }
            match client.start_play(admitted) {
                Ok((mut view, link)) => {
                    if let Some(world) = &self.world
                        && view.set_world(Arc::clone(world)).is_err()
                    {
                        self.fail(event_loop, ShellError::RendererInitialization);
                        return;
                    }
                    self.client = Some(client);
                    self.play = Some(Play { view, link });
                    self.game_ui = self.login.take();
                    if let Some(gui) = &self.game_ui {
                        crate::client_chrome::install(&gui.context, self.preferences.high_contrast);
                    }
                    if let Some(window) = &self.window {
                        window.set_title("Oteryn");
                    }
                }
                Err(_) => self.fail(event_loop, ShellError::RendererInitialization),
            }
        }
        if let Some(play) = &mut self.play
            && let Err(class) = play.view.tick(&play.link)
        {
            self.return_to_login(class);
        }
        if let (Some(play), Some(window)) = (&mut self.play, &self.window) {
            let status = play.view.combat_status();
            if status != play.status {
                window.set_title(&format!("Oteryn — {status}"));
                play.status = status;
            }
        }
        if let Some(window) = &self.window
            && redraw_eligible(
                self.renderer
                    .as_ref()
                    .map(|renderer| renderer.state().phase()),
            )
        {
            let fps = if self.focused {
                self.preferences.fps
            } else {
                self.preferences.background_fps
            };
            let interval = if fps == 0 {
                std::time::Duration::ZERO
            } else {
                std::time::Duration::from_secs_f64(1.0 / f64::from(fps))
            };
            let deadline = self.last_redraw + interval;
            if std::time::Instant::now() >= deadline {
                self.last_redraw = std::time::Instant::now();
                window.request_redraw();
                event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(
                    self.last_redraw + interval.max(std::time::Duration::from_millis(1)),
                ));
            } else {
                event_loop.set_control_flow(winit::event_loop::ControlFlow::WaitUntil(deadline));
            }
        }
    }
}

pub fn run(play: Option<(ClientBootstrap, AdmittedSession)>) -> Result<(), ShellError> {
    let event_loop = EventLoop::new().map_err(|_error| ShellError::EventLoopCreation)?;
    let smoke = std::env::args().any(|argument| argument == "--smoke");
    let mut application = Application::new(smoke, play)?;
    let run_result = event_loop
        .run_app(&mut application)
        .map_err(|_error| ShellError::EventLoopRun);
    drop(application.play.take());
    if let Some(client) = application.client.take() {
        client.shutdown();
    }
    if let Some(error) = application.fatal_error {
        return Err(error);
    }
    run_result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fatal_retention_keeps_the_first_error() {
        let mut error = None;
        retain_first_error(&mut error, ShellError::RendererResize);
        retain_first_error(&mut error, ShellError::RendererRender);
        assert_eq!(error, Some(ShellError::RendererResize));
    }

    #[test]
    fn redraw_requires_a_configured_renderer() {
        assert!(redraw_eligible(Some(SurfacePhase::Configured)));
        assert!(!redraw_eligible(None));
        assert!(!redraw_eligible(Some(SurfacePhase::Unconfigured)));
        assert!(!redraw_eligible(Some(SurfacePhase::Suspended)));
        assert!(!redraw_eligible(Some(SurfacePhase::Lost)));
        assert!(!redraw_eligible(Some(SurfacePhase::Closing)));
    }
}
