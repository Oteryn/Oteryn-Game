mod capture;
#[path = "features/local-reference-preferences/frame_native.rs"]
mod frame_native;
#[path = "features/local-spell-action-bar/native.rs"]
mod hud;
mod input;
#[path = "features/local-native-login/shell.rs"]
mod native_login_shell;
#[path = "features/local-reference-preferences/native.rs"]
mod settings;

use oteryn_client::input::{MouseActions, StepDir};
use oteryn_client::map::{START, World};
use oteryn_client::native_login::{LoginWindow, error_text};
use oteryn_client::play::{PlayEnd, PlayLink, PlayView};
use oteryn_client::reentry::Reentry;
use oteryn_client::scene::Scene;
use oteryn_client::spell::SpellHotkeys;
use oteryn_client::{ClientBootstrap, GameplayEntryError};
use oteryn_foundation::ProcessGeneration;
use oteryn_input_platform::InputPlatformAdapter;
use oteryn_platform_client::native_login::PublicClass;
use oteryn_renderer::{AtlasImage, SurfacePhase, TileCoord, WindowsRenderer};
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
    LoginInitialization,
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
            Self::LoginInitialization => "native login window creation failed",
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
    frame_timing: frame_native::FrameTiming,
    captures: capture::Captures,
    settings: settings::Settings,
    hud: Option<hud::Hud>,
    audio: oteryn_client::play::NativeAudio,
    smoke: bool,
    window: Option<Arc<Window>>,
    renderer: Option<WindowsRenderer<Arc<Window>>>,
    atlas: Option<AtlasImage>,
    /// Local map used by the offline scene.
    world: Option<Arc<World>>,
    /// Without a session: the scene around `offline_own`, which the arrow keys move locally.
    scene: Option<Scene>,
    offline_own: TileCoord,
    offline_facing: StepDir,
    entry: Option<Reentry>,
    login: Option<LoginWindow>,
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
    fn new(smoke: bool, client: Option<ClientBootstrap>) -> Result<Self, ShellError> {
        let entry = client.map(Reentry::new);
        let login = if smoke {
            None
        } else {
            Some(LoginWindow::open().map_err(|_| ShellError::LoginInitialization)?)
        };
        Ok(Self {
            frame_timing: frame_native::FrameTiming::default(),
            captures: Default::default(),
            settings: settings::Settings::new(),
            hud: None,
            audio: oteryn_client::play::NativeAudio::default(),
            smoke,
            window: None,
            renderer: None,
            atlas: None,
            world: None,
            scene: None,
            offline_own: TileCoord::new(START.0, START.1),
            offline_facing: StepDir::South,
            entry,
            login,
            play: None,
            generation: ProcessGeneration::new(1),
            input: InputPlatformAdapter::new(),
            actions: MouseActions::new().map_err(|_error| ShellError::InputInitialization)?,
            hotkeys: SpellHotkeys::new().map_err(|_error| ShellError::InputInitialization)?,
            fatal_error: None,
        })
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
        self.leave_play(error_text(GameplayEntryError::Rejected(class)));
    }

    /// Reauthenticate after the consumed ticket, then show account character selection.
    fn return_to_character_selection(&mut self) {
        self.leave_play("Choose a character after browser sign-in.");
        if let Some(login) = &self.login {
            login.finish(Ok(()));
        }
        if let Some(entry) = &mut self.entry {
            entry.request();
        }
    }

    fn leave_play(&mut self, message: &str) {
        if let Some(renderer) = &mut self.renderer {
            renderer.cancel_capture();
        }
        self.captures = Default::default();
        self.audio.stop();
        self.play = None;
        if let Some(entry) = &mut self.entry {
            entry.leave_play();
        }
        if let Some(login) = &self.login {
            login.show(message.to_owned());
        }
        println!("Oteryn: {message}");
        if let Some(window) = &self.window {
            window.set_title(&format!("Oteryn — {message} — Enter: log in again"));
        }
        if self.rebuild_offline().is_err() {
            self.scene = None;
        }
    }

    fn request_entry(&mut self) {
        if let Some(login) = &self.login {
            login.show("Review connection settings, then choose Save and sign in.".into());
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

#[path = "windows_shell/application_handler.rs"]
mod application_handler;

pub fn run(client: Option<ClientBootstrap>) -> Result<(), ShellError> {
    let event_loop = EventLoop::new().map_err(|_error| ShellError::EventLoopCreation)?;
    let smoke = std::env::args().any(|argument| argument == "--smoke");
    let mut application = Application::new(smoke, client)?;
    let run_result = event_loop
        .run_app(&mut application)
        .map_err(|_error| ShellError::EventLoopRun);
    drop(application.play.take());
    if let Some(entry) = application.entry.take() {
        entry.shutdown();
    }
    if let Some(error) = application.fatal_error {
        return Err(error);
    }
    run_result
}

#[cfg(test)]
mod tests;
