//! The Bevy desktop host over the production client owners; `oteryn-client.exe` on Windows.
mod account_services;
mod audio;
mod audit;
mod controls;
mod creature_bars;
mod diagnostics;
mod exit;
mod frame_pacing;
mod game_ui;
mod gameplay_dialogs;
mod health_circle;
mod hints;
mod input;
mod live_assets;
mod login;
pub(crate) mod maintenance;
mod owner;
mod scene;
mod screenshots;
mod selection;
mod speech_bubbles;
mod text_messages;
mod ui;
mod updater;
mod window_drag;

use bevy::{
    asset::AssetPlugin,
    image::ImagePlugin,
    prelude::*,
    window::{PresentMode, PrimaryWindow, WindowMode},
};
use owner::HostOwner;
use std::{ffi::OsString, path::PathBuf};

pub struct Options {
    pub ui_assets: PathBuf,
    pub preferences: Option<PathBuf>,
    /// Optional diagnostic state; F12 also writes a rendered PNG beside this path.
    pub ui_audit: Option<PathBuf>,
    pub login_settings: Option<PathBuf>,
    /// `--smoke`: open the window, run a few frames and exit, for packaging checks.
    pub smoke: bool,
}
/// Overrides the packaged `ui-assets` directory beside the executable.
const UI_ASSETS_ENV: &str = "OTERYN_UI_ASSETS";
impl Options {
    pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Self, String> {
        let mut args = args.into_iter();
        let mut ui_assets = None;
        let mut preferences = crate::play::PreferenceStore::path();
        let mut ui_audit = None;
        let mut login_settings = crate::native_login::settings_path().ok();
        let mut smoke = false;
        while let Some(argument) = args.next() {
            if argument == "--smoke" {
                smoke = true;
                continue;
            }
            let target = match argument.to_str() {
                Some("--ui-assets") => &mut ui_assets,
                Some("--preferences") => &mut preferences,
                Some("--ui-audit") => &mut ui_audit,
                Some("--login-settings") => &mut login_settings,
                // The launcher passes its own arguments through, and the executable handles
                // --check-packages before the host starts: like the earlier shell, ignore them.
                _ => continue,
            };
            let value = args
                .next()
                .filter(|v| !v.is_empty())
                .ok_or("Missing option path")?;
            *target = Some(PathBuf::from(value));
        }
        let ui_assets = ui_assets
            .or_else(|| std::env::var_os(UI_ASSETS_ENV).map(PathBuf::from))
            .or_else(|| {
                std::env::current_exe()
                    .ok()
                    .and_then(|exe| exe.parent().map(|directory| directory.join("ui-assets")))
            })
            .ok_or("Pass --ui-assets <directory>; source skins/fonts are external")?;
        // A packaging smoke run starts without the external skins when none are installed.
        let ui_assets = if smoke && !ui_assets.is_dir() {
            ui_assets
        } else {
            let ui_assets = ui_assets
                .canonicalize()
                .map_err(|_| "Cannot open --ui-assets directory")?;
            ui::validate_assets(&ui_assets)?;
            ui_assets
        };
        Ok(Self {
            ui_assets,
            preferences,
            ui_audit,
            login_settings,
            smoke,
        })
    }
}

/// Called only by the executable. Headless tests never install WinitPlugin or run this loop.
pub fn run(args: impl IntoIterator<Item = OsString>) -> Result<(), Box<dyn std::error::Error>> {
    diagnostics::start_logging();
    let options = Options::parse(args)?;
    crate::desktop_startup::prepare(options.preferences.clone(), options.smoke);
    let mut owner = HostOwner::new(options.preferences.clone())?;
    owner.configure_login(options.login_settings.clone(), |name| {
        std::env::var(name).ok()
    });
    let last_crash = diagnostics::detect_last_crash();
    let mut app = build_app(options, owner);
    app.insert_resource(last_crash);
    // App::run transfers the app into its runner. HostOwner::drop owns shutdown there.
    match app.run() {
        AppExit::Success => Ok(()),
        AppExit::Error(code) => Err(format!("Bevy event loop exited with {code}").into()),
    }
}
fn build_app(options: Options, owner: HostOwner) -> App {
    let mut app = App::new();
    if options.smoke {
        app.add_systems(Update, smoke_exit);
    }
    // A packaging smoke run leaves the player's layout file alone.
    let layout = game_ui::layout::LayoutStore::load(
        options
            .preferences
            .as_ref()
            .filter(|_| !options.smoke)
            .map(|path| path.with_file_name("client-window-layout.json")),
    );
    let mut window = Window {
        title: "Oteryn".into(),
        resolution: (1024, 720).into(),
        resize_constraints: bevy::window::WindowResizeConstraints {
            min_width: 1020.,
            min_height: 650.,
            ..default()
        },
        ..default()
    };
    if let Some([x, y, width, height]) = layout.window()
        && let (Ok(width), Ok(height)) = (u32::try_from(width), u32::try_from(height))
    {
        window.position = bevy::window::WindowPosition::At(IVec2::new(x, y));
        window.resolution = (width, height).into();
    }
    app.add_plugins(
        DefaultPlugins
            .set(AssetPlugin {
                file_path: options.ui_assets.to_string_lossy().into_owned(),
                ..default()
            })
            .set(bevy::winit::WinitPlugin {
                run_on_any_thread: cfg!(test),
            })
            .set(ImagePlugin::default_nearest())
            .set(WindowPlugin {
                close_when_requested: false,
                primary_window: Some(window),
                ..default()
            })
            // `diagnostics::start_logging` owns the process subscriber: stderr, file and ring.
            .disable::<bevy::log::LogPlugin>(),
    )
    .add_plugins((
        oteryn_client_ui_bevy::GlobalUiPlugin,
        bevy::input_focus::tab_navigation::TabNavigationPlugin,
    ))
    .insert_resource(ClearColor(Color::BLACK))
    .insert_non_send(owner)
    .insert_resource(layout)
    .insert_resource(audit::AuditOutput::new(options.ui_audit))
    .init_resource::<scene::SceneCache>()
    .init_resource::<speech_bubbles::SpeechBubbles>()
    .init_resource::<game_ui::viewport::WorldViewport>()
    .add_systems(
        Startup,
        (
            ui::setup,
            game_ui::setup,
            game_ui::viewport::setup,
            login::setup,
            selection::setup,
        )
            .chain(),
    )
    .add_systems(PreUpdate, selection::poll.before(poll_owner))
    .add_systems(PreUpdate, poll_owner)
    .add_systems(
        Update,
        (
            login::keyboard,
            login::project,
            selection::keyboard,
            selection::project,
            ui::keyboard,
            ui::project,
            ui::project_toolbar,
            ui::project_manager,
            project_window,
            game_ui::project,
            scene::project,
            speech_bubbles::project,
            scene::typing::project,
        )
            .chain(),
    )
    .add_systems(
        PostUpdate,
        game_ui::viewport::project.after(bevy::ui::UiSystems::PostLayout),
    )
    .add_systems(PostUpdate, audit::write.after(bevy::ui::UiSystems::Layout))
    .add_systems(Update, audit::capture_on_key);
    app.init_non_send::<input::NativeInput>().add_systems(
        Update,
        input::route
            .before(login::keyboard)
            .before(selection::keyboard)
            .before(ui::keyboard)
            .before(ui::project),
    );
    account_services::install(&mut app);
    audio::install(&mut app);
    creature_bars::install(&mut app);
    diagnostics::install(&mut app);
    exit::install(&mut app);
    frame_pacing::install(&mut app);
    game_ui::install(&mut app);
    gameplay_dialogs::install(&mut app);
    health_circle::install(&mut app);
    hints::install(&mut app);
    login::install(&mut app);
    text_messages::install(&mut app);
    updater::install(&mut app);
    window_drag::install(&mut app);
    app
}

/// Ends a `--smoke` run once the window has drawn a few frames.
fn smoke_exit(mut frames: Local<u8>, mut exit: MessageWriter<AppExit>) {
    *frames = frames.saturating_add(1);
    if *frames == 3 {
        exit.write(AppExit::Success);
    }
}

fn poll_owner(
    mut owner: NonSendMut<HostOwner>,
    window: Query<&Window, With<PrimaryWindow>>,
    viewport: Res<game_ui::viewport::WorldViewport>,
) {
    if let Ok(window) = window.single() {
        let size = if viewport.size.x > 0 && viewport.size.y > 0 {
            viewport.size.to_array()
        } else {
            [window.physical_width(), window.physical_height()]
        };
        owner.poll(size);
    }
}
fn project_window(
    owner: NonSend<HostOwner>,
    mut window: Query<&mut Window, With<PrimaryWindow>>,
    mut scale: ResMut<UiScale>,
) {
    if let Ok(mut window) = window.single_mut() {
        apply_window(&owner.settings.preferences, &mut window);
    }
    let next = owner.settings.preferences.interface_scale as f32 / 100.;
    if scale.0 != next {
        scale.0 = next;
    }
}
fn apply_window(preferences: &crate::play::ReferencePreferences, window: &mut Window) {
    let mode = if preferences.graphics.fullscreen {
        WindowMode::BorderlessFullscreen(bevy::window::MonitorSelection::Current)
    } else {
        WindowMode::Windowed
    };
    let present = if preferences.graphics.vsync {
        PresentMode::AutoVsync
    } else {
        PresentMode::AutoNoVsync
    };
    if window.mode != mode {
        window.mode = mode;
    }
    if window.present_mode != present {
        window.present_mode = present;
    }
}

#[cfg(test)]
mod layout_tests;
